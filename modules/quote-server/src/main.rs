use anyhow::{Context, Result};
use common::{
  StockQuote, StockRequest, StockResponse, StockResponseStatus, read_tickers,
};
use serde_json::json;
use signal_hook::{consts::TERM_SIGNALS, flag};
use std::{
  collections::HashMap,
  io::{self, Read, Write},
  net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
  path::PathBuf,
  sync::atomic::{AtomicBool, Ordering},
  sync::mpsc::channel,
  sync::{Arc, RwLock, TryLockError, mpsc},
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};

mod configs;
mod quote;

use configs::consts;
use quote::QuoteGenerator;

fn main() -> Result<()> {
  tracing_subscriber::fmt()
    .with_line_number(true)
    .with_thread_ids(true)
    .init();

  info!("Start server");

  let tickers: Vec<String> =
    read_tickers(PathBuf::from("./mocks/server-tickers.txt"))?;

  let shutdown = Arc::new(AtomicBool::new(false));
  for sig in TERM_SIGNALS {
    flag::register_conditional_shutdown(*sig, 1, Arc::clone(&shutdown))?;
    flag::register(*sig, Arc::clone(&shutdown))?;
  }

  let server: Server = Server::new(
    consts::SERVER_TCP_ADDR,
    consts::SERVER_UPD_ADDR,
    tickers,
    shutdown,
  )?;

  info!(addr = %consts::SERVER_TCP_ADDR, "Initialized server");

  server.run()?;

  Ok(())
}

type StockQuoteList = Arc<RwLock<Vec<StockQuote>>>;
type ClientChannelsMap =
  Arc<RwLock<HashMap<SocketAddr, mpsc::Sender<StockQuoteList>>>>;
// Store client address with latest activity timestamp (u64 as secs)
type HealthCheckMap = Arc<RwLock<HashMap<SocketAddr, u64>>>;

#[derive(Debug)]
struct Server {
  tcp: TcpListener,
  udp: UdpSocket,
  tickers: Vec<String>,
  client_channel_map: ClientChannelsMap,
  health_check_map: HealthCheckMap,
  shutdown: Arc<AtomicBool>,
}

impl Server {
  fn new(
    tcp_addr: SocketAddr,
    udp_addr: SocketAddr,
    tickers: Vec<String>,
    shutdown: Arc<AtomicBool>,
  ) -> Result<Self> {
    let tcp_listener = TcpListener::bind(tcp_addr).with_context(|| {
      format!("Bind TCP listener to addr: {}", consts::SERVER_TCP_ADDR)
    })?;
    tcp_listener
      .set_nonblocking(true)
      .context("Failed set_nonblocking for TCP listener")?;
    let udp_socket = UdpSocket::bind(udp_addr).context(format!(
      "Failed binding to UDP socket {:?}",
      consts::SERVER_UPD_ADDR
    ))?;
    udp_socket
      .set_write_timeout(Some(consts::UDP_WRITE_TIMEOUT))
      .context("Failed set_write_timeout for UDP socket")?;

    Ok(Self {
      tcp: tcp_listener,
      udp: udp_socket,
      tickers,
      client_channel_map: Arc::new(RwLock::new(HashMap::new())),
      health_check_map: Arc::new(RwLock::new(HashMap::new())),
      shutdown,
    })
  }
  fn run(&self) -> Result<()> {
    info!("Run server");

    let (tx, rx) = channel::<StockQuoteList>();
    let quotes_broadcasting = self.broadcast_quotes_to_channels(rx);
    let healthcheck_server = self.start_healthcheck_server()?;
    let healthcheck_monitoring = self.start_healthcheck_monitoring()?;
    let quotes_generation_thread = self.start_quotes_generation(tx);
    self.start_tcp_server()?;

    let _ = quotes_generation_thread
      .join()
      .expect("Failed waiting for quotes generation thread");
    let _ = healthcheck_monitoring
      .join()
      .expect("Failed waiting for healthcheck_monitoring thread");
    let _ = healthcheck_server
      .join()
      .expect("Failed waiting for healthcheck thread");
    let _ = quotes_broadcasting
      .join()
      .expect("Failed waiting for quotes broadcasting thread");

    Ok(())
  }
  /* quote list broadcasting to spawned threads */
  fn broadcast_quotes_to_channels(
    &self,
    rx: mpsc::Receiver<StockQuoteList>,
  ) -> thread::JoinHandle<()> {
    info!("Start quotes broadcasting");

    let client_channel_map = Arc::clone(&self.client_channel_map);

    thread::spawn(move || {
      while let Ok(msg) = rx.recv() {
        let client_channel_map = client_channel_map
          .read()
          .expect("Failed obtaining client_channel_map lock");

        for (_, tx) in client_channel_map.iter() {
          match tx.send(Arc::clone(&msg)) {
            Ok(_) => {
              // message sent, continue receiving messages
            }
            Err(e) => {
              error!(err = %e, "Failed sending message to channel");
            }
          }
        }
      }
    })
  }
  fn start_quotes_generation(
    &self,
    tx: mpsc::Sender<StockQuoteList>,
  ) -> thread::JoinHandle<Result<()>> {
    info!("Start quotes generation");

    let mut quote_generator = QuoteGenerator::new(&self.tickers);
    // Share Arc<RwLock> reference to avoid data cloning on message dispatch
    let quotes_list: StockQuoteList = Arc::new(RwLock::new(vec![]));
    let shutdown = Arc::clone(&self.shutdown);

    thread::spawn(move || -> Result<()> {
      while !shutdown.load(Ordering::Acquire) {
        quote_generator.shuffle_prices();
        let new_quotes_list = quote_generator.generate_quote_list();
        {
          let mut mut_quotes_list =
            quotes_list.write().expect("Failed to get mut quotes_list");
          *mut_quotes_list = new_quotes_list
        }

        tx.send(Arc::clone(&quotes_list))
          .with_context(|| "Failed sending list of generated quotes")?;
        thread::sleep(consts::QUOTES_GENERATION_TIMEOUT);
      }

      Ok(())
    })
  }
  fn start_tcp_server(&self) -> Result<()> {
    info!("Start TCP server");
    let shutdown = Arc::clone(&self.shutdown);

    for stream in self.tcp.incoming() {
      if shutdown.load(Ordering::Acquire) {
        return Ok(());
      }

      match stream {
        Ok(stream) => {
          stream
            .set_nodelay(true)
            .context("Failed set_nodelay for TCP stream")?;
          stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .context("Failed set_read_timeout for TCP stream")?;
          stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .context("Failed set_read_timeout for TCP stream")?;

          self.read_tcp_stream(stream)?;
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
          thread::sleep(consts::TCP_STREAM_IDLE_TIMEOUT);
        }
        Err(e) => {
          error!(error = %e, "Connection failed");
        }
      }
    }

    Ok(())
  }
  fn read_tcp_stream(&self, stream: TcpStream) -> Result<()> {
    info!("Read tcp stream!");

    let mut reader = stream.try_clone().context("Failed cloning TcpStream")?;
    let mut writer = stream.try_clone().context("Failed cloning TcpStream")?;

    let mut buf = vec![0u8; 1024];
    let n = reader.read(&mut buf)?;

    let StockRequest {
      kind,
      addr,
      tickers,
    } = serde_json::from_slice::<StockRequest>(&buf[..n])?;

    let response: StockResponse;

    match kind.as_str() {
      "STREAM" => {
        self
          .start_quotes_streaming(addr, tickers)
          .context(format!("Failed start_quotes_streaming for {addr:?}"))?;

        // Add new client to health_check_map
        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .context("Failed reading SystemTime duration since UNIX_EPOCH")?;
        let healthcheck_map = &mut self
          .health_check_map
          .write()
          .expect("Failed locking health_check_map for write");
        healthcheck_map.insert(addr, now.as_secs());

        response = StockResponse {
          status: StockResponseStatus::Ok,
          message: "ok".to_string(),
        };
      }
      _ => {
        warn!(kind = %kind, "Unsupported command");

        response = StockResponse {
          status: StockResponseStatus::Error,
          message: "Unsupported command".to_string(),
        };
      }
    }

    let message = json!(response).to_string();
    writer
      .write_all(message.as_bytes())
      .context("Failed writing to TCP stream")?;
    writer.flush()?;

    Ok(())
  }
  fn start_quotes_streaming(
    &self,
    addr: SocketAddr,
    requested_tickers: Vec<String>,
  ) -> Result<()> {
    info!(addr = %addr, "Start quotes streaming");

    let udp = self.udp.try_clone().context("Failed cloning UDP socket")?;
    let (tx, rx) = channel::<StockQuoteList>();
    {
      let client_channel_map = &mut self
        .client_channel_map
        .write()
        .expect("Failed obtaining client_channel_map lock");
      client_channel_map.insert(addr, tx);
    }

    thread::spawn(move || -> Result<()> {
      while let Ok(quotes) = rx.recv() {
        let quotes = quotes
          .read()
          .expect("Failed reading stock quotes list from RwLock");

        let filtered_quotes: Vec<&StockQuote> = quotes
          .iter()
          .filter(|el| requested_tickers.contains(&el.ticker))
          .collect();
        let message = json!(filtered_quotes).to_string();

        udp
          .send_to(message.as_bytes(), addr)
          .context("Failed sending data to UDP socket")?;
      }

      Ok(())
    });

    Ok(())
  }
  fn start_healthcheck_monitoring(
    &self,
  ) -> Result<thread::JoinHandle<Result<()>>> {
    let health_check_map = Arc::clone(&self.health_check_map);
    let client_channel_map = Arc::clone(&self.client_channel_map);
    let shutdown = Arc::clone(&self.shutdown);

    Ok(thread::spawn(move || {
      while !shutdown.load(Ordering::Acquire) {
        // Avoid deadlock using try_read
        // Read access has lower priority than write hence can be skipped
        match health_check_map.try_read() {
          Ok(healthcheck_map) => {
            let now = SystemTime::now();
            let mut client_channel_map = client_channel_map
              .write()
              .expect("Failed locking client_channel_map with read access");

            for (addr, timestamp) in healthcheck_map.iter() {
              if client_channel_map.contains_key(addr) {
                let latest_timestamp =
                  SystemTime::UNIX_EPOCH + Duration::from_secs(*timestamp);
                let diff = now
                  .duration_since(latest_timestamp)
                  .context("Failed reading timestamp difference")?;

                if diff > consts::HEALTHCHECK_TIMEOUT {
                  client_channel_map.remove(addr);
                  warn!(addr = %addr, "Client is disconnected:");
                }
              }
            }
          }
          Err(TryLockError::WouldBlock) => {
            // Resource is blocked with write access, skip to next iteration
          }
          Err(TryLockError::Poisoned(e)) => {
            return Err(e).expect("Failed reading from healthcheck_map");
          }
        }

        thread::sleep(consts::HEALTH_CHECK_MONITOR_TIMEOUT);
      }

      Ok(())
    }))
  }
  fn start_healthcheck_server(&self) -> Result<thread::JoinHandle<Result<()>>> {
    let udp = self.udp.try_clone().context("Failed cloning UDP socket")?;
    udp
      .set_read_timeout(Some(Duration::from_secs(2)))
      .context("Failed set_read_timeout for udp socket")?;
    let mut buf = vec![0u8; 64];
    let health_check_map = Arc::clone(&self.health_check_map);
    let shutdown = Arc::clone(&self.shutdown);

    Ok(thread::spawn(move || {
      while !shutdown.load(Ordering::Acquire) {
        match udp.recv_from(&mut buf) {
          Ok((_, from)) => {
            // update client activity timestamp
            let mut health_check_map = health_check_map
              .write()
              .expect("Failed reading health_check_map RwLock");
            if let Some(timestamp) = health_check_map.get_mut(&from) {
              *timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("Failed reading SystemTime duration since UNIX_EPOCH")?
                .as_secs();
            }
          }
          Err(e)
            if e.kind() == io::ErrorKind::TimedOut
              || e.kind() == io::ErrorKind::WouldBlock =>
          {
            // skip timeout|blocking read
          }
          Err(e) => return Err(e).context("Failed reading from UDP socket"),
        }
      }

      Ok(())
    }))
  }
}
