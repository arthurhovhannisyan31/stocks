use anyhow::{anyhow, Context};
use clap::Parser;
use serde_json::json;
use std::{
  collections::HashMap,
  io::{self, Read, Write},
  net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
  path::PathBuf,
  sync::atomic::{AtomicBool, Ordering},
  sync::{mpsc, Arc, RwLock, TryLockError},
  thread,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tracing::{error, info, warn};

use common::{
  error::AppError,
  stock::{StockQuote, StockRequest, StockResponse, StockResponseStatus},
  utils::{read_tickers, register_signal_hooks},
};

mod configs;
mod quote;

use configs::{consts, CliArgs};
use quote::QuoteGenerator;

fn main() -> Result<(), AppError> {
  tracing_subscriber::fmt()
    .with_line_number(true)
    .with_thread_ids(true)
    .init();

  info!("Start server");

  let cli = CliArgs::parse();
  let CliArgs { tickers_file } = cli;

  let tickers: Vec<String> = read_tickers(PathBuf::from(tickers_file))?;

  let shutdown = Arc::new(AtomicBool::new(false));
  register_signal_hooks(&shutdown)?;

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
  Arc<RwLock<HashMap<SocketAddr, mpsc::SyncSender<StockQuoteList>>>>;
type HealthCheckMap = Arc<RwLock<HashMap<SocketAddr, Instant>>>;

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
  ) -> Result<Self, AppError> {
    let tcp_listener = TcpListener::bind(tcp_addr).map_err(|err| {
      AppError::AddressBindError {
        err,
        addr: consts::SERVER_TCP_ADDR,
      }
    })?;
    tcp_listener
      .set_nonblocking(true)
      .map_err(|err| AppError::TcpListenerError { err })?;
    let udp_socket =
      UdpSocket::bind(udp_addr).map_err(|err| AppError::AddressBindError {
        err,
        addr: consts::SERVER_UPD_ADDR,
      })?;
    udp_socket
      .set_write_timeout(Some(consts::UDP_WRITE_TIMEOUT))
      .map_err(|err| AppError::UdpSocketError { err })?;

    Ok(Self {
      tcp: tcp_listener,
      udp: udp_socket,
      tickers,
      client_channel_map: Arc::new(RwLock::new(HashMap::new())),
      health_check_map: Arc::new(RwLock::new(HashMap::new())),
      shutdown,
    })
  }
  fn run(&self) -> Result<(), AppError> {
    info!("Run server");

    let (tx, rx) = mpsc::sync_channel::<StockQuoteList>(1);
    let quotes_broadcasting = self.broadcast_quotes_to_channels(rx);
    let healthcheck_server = self.start_healthcheck_server()?;
    let healthcheck_monitoring = self.start_healthcheck_monitoring()?;
    let quotes_generation_thread = self.start_quotes_generation(tx);
    self.start_tcp_server()?;

    let _ = quotes_generation_thread.join().map_err(|_| {
      AppError::OtherError(anyhow!(
        "Failed waiting for quotes generation thread"
      ))
    })?;
    let _ = healthcheck_monitoring.join().map_err(|_| {
      AppError::OtherError(anyhow!(
        "Failed waiting for healthcheck_monitoring thread"
      ))
    })?;
    let _ = healthcheck_server.join().map_err(|_| {
      AppError::OtherError(anyhow!("Failed waiting for healthcheck thread"))
    })?;
    let _ = quotes_broadcasting.join().map_err(|_| {
      AppError::OtherError(anyhow!(
        "Failed waiting for quotes broadcasting thread"
      ))
    })?;

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
    tx: mpsc::SyncSender<StockQuoteList>,
  ) -> thread::JoinHandle<Result<(), AppError>> {
    info!("Start quotes generation");

    let mut quote_generator = QuoteGenerator::new(&self.tickers);
    // Share Arc<RwLock> reference to avoid data cloning on message dispatch
    let quotes_list: StockQuoteList = Arc::new(RwLock::new(vec![]));
    let shutdown = Arc::clone(&self.shutdown);

    thread::spawn(move || -> Result<(), AppError> {
      while !shutdown.load(Ordering::Acquire) {
        quote_generator.shuffle_prices();
        let new_quotes_list = quote_generator.generate_quote_list();
        {
          let mut mut_quotes_list =
            quotes_list.write().expect("Failed to get mut quotes_list");
          *mut_quotes_list = new_quotes_list
        }

        tx.send(Arc::clone(&quotes_list))
          .context("Failed sending list of generated quotes")?;
        thread::sleep(consts::QUOTES_GENERATION_TIMEOUT);
      }

      Ok(())
    })
  }
  fn start_tcp_server(&self) -> Result<(), AppError> {
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
            .map_err(|err| AppError::TcpStreamError { err })?;
          stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|err| AppError::TcpStreamError { err })?;
          stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|err| AppError::TcpStreamError { err })?;

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
  fn read_tcp_stream(&self, stream: TcpStream) -> Result<(), AppError> {
    info!("Read tcp stream!");

    let mut reader = stream
      .try_clone()
      .map_err(|err| AppError::TcpStreamError { err })?;
    let mut writer = stream
      .try_clone()
      .map_err(|err| AppError::TcpStreamError { err })?;

    let mut buf = vec![0u8; 1024];
    let n = reader.read(&mut buf)?;

    let StockRequest {
      kind,
      addr,
      tickers,
    } = serde_json::from_slice::<StockRequest>(&buf[..n])
      .map_err(|err| AppError::DeserializationError { err })?;

    let response: StockResponse;

    match kind.as_str() {
      "STREAM" => {
        self.start_quotes_streaming(addr, tickers)?;

        // Add new client to health_check_map
        let now = SystemTime::now()
          .duration_since(UNIX_EPOCH)
          .context("Failed reading SystemTime duration since UNIX_EPOCH")?;
        let healthcheck_map = &mut self
          .health_check_map
          .write()
          .expect("Failed locking health_check_map for write");
        healthcheck_map.insert(addr, Instant::now());

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
    writer.flush().context("Failed writing data to stream")?;

    Ok(())
  }
  fn start_quotes_streaming(
    &self,
    addr: SocketAddr,
    requested_tickers: Vec<String>,
  ) -> Result<(), AppError> {
    info!(addr = %addr, "Start quotes streaming");

    let udp = self
      .udp
      .try_clone()
      .map_err(|err| AppError::UdpSocketError { err })?;
    let (tx, rx) = mpsc::sync_channel::<StockQuoteList>(1);
    {
      let client_channel_map = &mut self
        .client_channel_map
        .write()
        .expect("Failed obtaining client_channel_map lock");
      client_channel_map.insert(addr, tx);
    }

    thread::spawn(move || -> Result<(), AppError> {
      while let Ok(quotes) = rx.recv() {
        let filtered_quotes: Vec<StockQuote> = {
          let quotes = quotes
            .read()
            .expect("Failed reading stock quotes list from RwLock");

          quotes
            .iter()
            .filter_map(|el| {
              if requested_tickers.contains(&el.ticker) {
                return Some(el.clone());
              }
              None
            })
            .collect()
        };

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
  ) -> Result<thread::JoinHandle<Result<(), AppError>>, AppError> {
    let health_check_map = Arc::clone(&self.health_check_map);
    let client_channel_map = Arc::clone(&self.client_channel_map);
    let shutdown = Arc::clone(&self.shutdown);

    Ok(thread::spawn(move || {
      while !shutdown.load(Ordering::Acquire) {
        // Health check monitoring has low priority, hence can be skipped sometimes
        // Health check map should be blocked as less as possible to give access to other writers
        match health_check_map.try_write() {
          Ok(mut health_check_map) => {
            if health_check_map.is_empty() {
              continue;
            }

            let current = Instant::now();
            let remove_list: Vec<SocketAddr> = health_check_map
              .iter()
              .filter_map(|(addr, instant)| {
                let diff = current.duration_since(*instant);

                if diff > consts::HEALTHCHECK_TIMEOUT {
                  warn!(addr = %addr, "Client is disconnected:");

                  return Some(*addr);
                }

                None
              })
              .collect();

            let mut client_channel_map = client_channel_map
              .write()
              .expect("Failed locking client_channel_map with read access");

            for addr in remove_list {
              health_check_map.remove(&addr);
              client_channel_map.remove(&addr);
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
  fn start_healthcheck_server(
    &self,
  ) -> Result<thread::JoinHandle<Result<(), AppError>>, AppError> {
    let udp = self
      .udp
      .try_clone()
      .map_err(|err| AppError::UdpSocketError { err })?;
    udp
      .set_read_timeout(Some(Duration::from_secs(2)))
      .map_err(|err| AppError::UdpSocketError { err })?;
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
            if let Some(instant) = health_check_map.get_mut(&from) {
              *instant = Instant::now();
            }
          }
          Err(e)
            if e.kind() == io::ErrorKind::TimedOut
              || e.kind() == io::ErrorKind::WouldBlock =>
          {
            // skip timeout|blocking read
          }
          Err(e) => return Err(e).context("Failed reading from UDP socket")?,
        }
      }

      Ok(())
    }))
  }
}
