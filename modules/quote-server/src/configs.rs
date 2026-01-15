use std::path::PathBuf;

use clap::Parser;

use common::utils::path_validation;

#[derive(Debug, Parser)]
#[command(version, about, next_line_help = true)]
pub(crate) struct CliArgs {
  #[arg(short = 'f', long, value_name = "Tickers file", value_parser = path_validation)]
  pub tickers_file: PathBuf,
}

pub(crate) mod consts {
  use std::net::{IpAddr, Ipv4Addr, SocketAddr};
  use std::time::Duration;

  const SERVER_IP_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
  pub const SERVER_TCP_ADDR: SocketAddr = SocketAddr::new(SERVER_IP_ADDR, 8000);
  pub const SERVER_UPD_ADDR: SocketAddr = SocketAddr::new(SERVER_IP_ADDR, 8001);
  pub const QUOTES_GENERATION_TIMEOUT: Duration = Duration::from_secs(1);
  pub const HEALTHCHECK_TIMEOUT: Duration = Duration::from_secs(5);
  pub const UDP_WRITE_TIMEOUT: Duration = Duration::from_secs(5);
  pub const TCP_STREAM_IDLE_TIMEOUT: Duration = Duration::from_millis(50);
  pub const HEALTH_CHECK_MONITOR_TIMEOUT: Duration = Duration::from_millis(50);
  pub const QUOTE_DEFAULT_PRICE: f64 = 1.0;
}
