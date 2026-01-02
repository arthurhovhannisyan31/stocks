use anyhow::{Result, anyhow};
use clap::Parser;
use std::ffi::OsStr;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

pub(crate) const EXTENSION_WHITELIST: &[&str] = &["txt"];

#[derive(Debug, Parser)]
#[command(version, about, next_line_help = true)]
pub(crate) struct CliArgs {
  #[arg(short = 'f', long, value_name = "Tickers file", value_parser = path_validation)]
  pub tickers_file: PathBuf,
  #[arg(short = 's', long, value_name = "Server TCP address", value_parser = server_address_validation)]
  pub server_tcp_addr: SocketAddr,
  #[arg(short = 'u', long, value_name = "Server UDP port", value_parser = port_validation)]
  pub server_udp_port: u16,
  #[arg(short = 'c', long, value_name = "Client UDP address", value_parser = server_address_validation)]
  pub client_udp_addr: SocketAddr,
}

fn path_validation(str: &str) -> Result<PathBuf> {
  let path =
    PathBuf::from_str(str).expect("Failed reading provided path value");

  if !path.exists() {
    return Err(anyhow!("Failed reading provided file path: {path:?}"));
  }

  if let Some(extension) = path.extension().and_then(OsStr::to_str) {
    if EXTENSION_WHITELIST.contains(&extension) {
      return Ok(path);
    }
  }

  Err(anyhow!("Invalid source file"))
}

fn server_address_validation(str: &str) -> Result<SocketAddr> {
  let socket_addr = SocketAddr::from_str(str)?;

  Ok(socket_addr)
}

fn port_validation(str: &str) -> Result<u16> {
  let port = str.parse::<u16>()?;

  Ok(port)
}
