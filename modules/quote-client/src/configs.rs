use clap::Parser;
use common::utils::{
  path_validation, port_validation, server_address_validation,
};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Parser)]
#[command(version, about, next_line_help = true)]
pub(crate) struct CliArgs {
  #[arg(short = 'f', long, value_name = "Tickers file", value_parser = path_validation)]
  pub tickers_file: PathBuf,
  #[arg(short = 's',long, value_name = "Server TCP address", value_parser = server_address_validation)]
  pub server_tcp_addr: SocketAddr,
  #[arg(short = 'S',long, value_name = "Server UDP port", value_parser = port_validation)]
  pub server_udp_port: u16,
  #[arg(short = 'c',long, value_name = "Client UDP address", value_parser = server_address_validation)]
  pub client_udp_addr: SocketAddr,
}

pub(crate) mod consts {
  use std::time::Duration;

  pub const UDP_READ_TIMEOUT: Duration = Duration::from_secs(5);
  pub const UDP_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
  pub const TCP_STREAM_READ_TIMEOUT: Duration = Duration::from_secs(2);
  pub const TCP_STREAM_WRITE_TIMEOUT: Duration = Duration::from_secs(2);
  pub const HEALTH_CHECK_STREAMING_TIMEOUT: Duration =
    Duration::from_millis(50);
}
