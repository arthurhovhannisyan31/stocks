use anyhow::{Context, anyhow};
use clap::Parser;
use common::error::AppError;
use common::{
  stock::{StockQuote, StockRequest, StockResponse, StockResponseStatus},
  utils::{read_tickers, register_signal_hooks},
};
use serde_json::json;
use signal_hook::{consts::SIGTERM, low_level::raise};
use std::{
  io::{self, Read, Write},
  net::{SocketAddr, TcpStream, UdpSocket},
  path::PathBuf,
  sync::atomic::Ordering,
  sync::{Arc, atomic::AtomicBool},
  thread,
  thread::JoinHandle,
};
use tracing::{error, info, warn};

mod configs;

use configs::{CliArgs, consts};

fn main() -> Result<(), AppError> {
  tracing_subscriber::fmt()
    .with_line_number(true)
    .with_thread_ids(true)
    .init();

  info!("Start client");

  let cli = CliArgs::parse();
  let CliArgs {
    client_udp_addr,
    server_tcp_addr,
    server_udp_port,
    tickers_file,
  } = cli;

  let tickers: Vec<String> = read_tickers(PathBuf::from(tickers_file))?;

  let shutdown = Arc::new(AtomicBool::new(false));
  register_signal_hooks(&shutdown)?;

  let client = Client::new(
    client_udp_addr,
    server_tcp_addr,
    server_udp_port,
    tickers,
    shutdown,
  )?;

  info!(
    server = %server_tcp_addr,
    server_udp = %server_udp_port,
    client_udp = %client_udp_addr,
    "Initialized client"
  );

  client.run()?;

  Ok(())
}

#[derive(Debug)]
struct Client {
  server_tcp_addr: SocketAddr,
  server_udp_addr: SocketAddr,
  tickers: Vec<String>,
  udp: UdpSocket,
  shutdown: Arc<AtomicBool>,
}

impl Client {
  fn new(
    client_udp_addr: SocketAddr,
    server_tcp_addr: SocketAddr,
    server_udp_port: u16,
    tickers: Vec<String>,
    shutdown: Arc<AtomicBool>,
  ) -> Result<Self, AppError> {
    let mut server_udp_addr = server_tcp_addr.clone();
    server_udp_addr.set_port(server_udp_port);
    let udp_socket = UdpSocket::bind(client_udp_addr).map_err(|err| {
      AppError::AddressBindError {
        addr: client_udp_addr,
        err,
      }
    })?;
    udp_socket
      .set_read_timeout(Some(consts::UDP_READ_TIMEOUT))
      .map_err(|err| AppError::UdpSocketError { err })?;

    Ok(Self {
      tickers,
      server_tcp_addr,
      server_udp_addr,
      udp: udp_socket,
      shutdown,
    })
  }
  fn run(&self) -> Result<(), AppError> {
    info!("Run client");

    let udp_server = self.start_udp_server()?;
    let healthcheck = self.start_healthcheck_streaming()?;
    self.send_stream_request()?;

    let _ = healthcheck.join().map_err(|_| {
      AppError::OtherError(anyhow!("Failed waiting on healthcheck_thread"))
    })?;
    let _ = udp_server.join().map_err(|_| {
      AppError::OtherError(anyhow!("Failed waiting for udp server thread"))
    })?;

    Ok(())
  }
  fn start_udp_server(
    &self,
  ) -> Result<JoinHandle<Result<(), AppError>>, AppError> {
    info!("Start UDP server");

    let shutdown = Arc::clone(&self.shutdown);
    let udp = self
      .udp
      .try_clone()
      .map_err(|err| AppError::UdpSocketError { err })?;

    Ok(thread::spawn(move || {
      let mut buf = vec![0u8; 4 * 1024];

      while !shutdown.load(Ordering::Acquire) {
        match udp.recv(&mut buf) {
          Ok(n) => {
            let stock_quotes: Vec<StockQuote> =
              serde_json::from_slice::<Vec<StockQuote>>(&buf[..n])
                .map_err(|err| AppError::DeserializationError { err })?;

            for stock_quote in stock_quotes {
              info!("Stock data: {stock_quote:?}");
            }
          }
          Err(e)
            if [io::ErrorKind::TimedOut, io::ErrorKind::WouldBlock]
              .contains(&e.kind()) =>
          {
            warn!(err = %e, "Failed reading from UDP: ");
          }
          Err(e) => return Err(e).context("udp_socket.recv failed")?,
        }
      }

      info!("Stop udp server");

      Ok(())
    }))
  }
  fn send_stream_request(&self) -> Result<(), AppError> {
    info!("Send stream request");

    let stream = TcpStream::connect(&self.server_tcp_addr).context(format!(
      "Failed connecting to server {}",
      self.server_tcp_addr
    ))?;
    stream
      .set_nodelay(true)
      .map_err(|err| AppError::TcpStreamError { err })?;
    stream
      .set_read_timeout(Some(consts::TCP_STREAM_READ_TIMEOUT))
      .map_err(|err| AppError::TcpStreamError { err })?;
    stream
      .set_write_timeout(Some(consts::TCP_STREAM_WRITE_TIMEOUT))
      .map_err(|err| AppError::TcpStreamError { err })?;

    let reader = stream
      .try_clone()
      .map_err(|err| AppError::TcpStreamError { err })?;
    let mut writer = stream
      .try_clone()
      .map_err(|err| AppError::TcpStreamError { err })?;

    let addr = self
      .udp
      .local_addr()
      .context("Failed reading local address")?;

    let stock_request = StockRequest {
      kind: "STREAM".to_string(),
      addr,
      tickers: self.tickers.clone(),
    };

    let message = json!(stock_request).to_string();
    writer
      .write_all(message.as_bytes())
      .context("Failed writing to TCP stream")?;
    writer.flush()?;

    info!("Request sent");

    let _ = self.read_tcp_stream(reader);

    Ok(())
  }
  fn read_tcp_stream(&self, mut stream: TcpStream) -> Result<(), AppError> {
    let peer_addr = stream
      .peer_addr()
      .context("Failed reading stream peer address")?;
    info!(peer = %peer_addr, "Read TCP stream");

    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf)?;

    let StockResponse { message, status } =
      serde_json::from_slice::<StockResponse>(&buf[..n])
        .map_err(|err| AppError::DeserializationError { err })?;

    match status {
      StockResponseStatus::Ok => {
        info!(message = %message, "Request success:");
      }
      StockResponseStatus::Error => {
        error!(message = %message, "Request error:");
        raise(SIGTERM).context("Failed raising SIGTERM signal")?;
      }
    }

    Ok(())
  }
  fn start_healthcheck_streaming(
    &self,
  ) -> Result<JoinHandle<Result<(), AppError>>, AppError> {
    info!("Start healthcheck streaming");

    let local_addr = self
      .udp
      .local_addr()
      .context("Failed reading local address")?;
    let udp = self
      .udp
      .try_clone()
      .map_err(|err| AppError::UdpSocketError { err })?;
    udp
      .set_write_timeout(Some(consts::UDP_WRITE_TIMEOUT))
      .map_err(|err| AppError::UdpSocketError { err })?;
    let server_udp_addr = self.server_udp_addr.clone();
    let shutdown = Arc::clone(&self.shutdown);

    Ok(thread::spawn(move || {
      while !shutdown.load(Ordering::Acquire) {
        let message = local_addr.to_string();

        udp
          .send_to(&message.as_bytes(), server_udp_addr)
          .context(format!("Failed sending to UDP {server_udp_addr:?}"))?;

        thread::sleep(consts::HEALTH_CHECK_STREAMING_TIMEOUT);
      }

      info!("Stop healthcheck streaming");

      Ok(())
    }))
  }
}
