use std::{
  ffi::OsStr,
  fs::File,
  io,
  io::ErrorKind,
  io::{BufRead, BufReader},
  net::SocketAddr,
  path::PathBuf,
  str::FromStr,
  sync::atomic::AtomicBool,
  sync::Arc,
};

use anyhow::Context;
use signal_hook::{consts::TERM_SIGNALS, flag};

use crate::error::AppError;

/// Read tickers from a file
///
/// # Example
///
/// ```
/// use common::{ utils::{read_tickers}, error::AppError };
/// use std::{path::PathBuf};
///
/// fn main() -> Result<(), AppError>{
///   let tickers: Vec<String> = read_tickers(PathBuf::from("../../mocks/server-tickers.txt"))?;
///
///   Ok(())
/// }
/// ```
pub fn read_tickers(path: PathBuf) -> Result<Vec<String>, AppError> {
  let tickers_file = File::open(path).context("Failed reading tickers file")?;
  let reader = BufReader::new(tickers_file);
  let lines = reader.lines();
  let tickers: Vec<String> = lines
    .filter_map(anyhow::Result::ok)
    .map(|str| str.trim().to_string())
    .collect();

  Ok(tickers)
}

pub fn register_signal_hooks(
  shutdown: &Arc<AtomicBool>,
) -> Result<(), AppError> {
  for sig in TERM_SIGNALS {
    flag::register_conditional_shutdown(*sig, 1, Arc::clone(&shutdown))
      .map_err(|err| AppError::SignalError { err, signal: *sig })?;
    flag::register(*sig, Arc::clone(&shutdown))
      .map_err(|err| AppError::SignalError { err, signal: *sig })?;
  }

  Ok(())
}

pub(crate) const EXTENSION_WHITELIST: &[&str] = &["txt"];

pub fn path_validation(str: &str) -> Result<PathBuf, AppError> {
  let path =
    PathBuf::from_str(str).expect("Failed reading provided path value");

  if !path.exists() {
    return Err(AppError::NotFound {
      err: io::Error::new(
        ErrorKind::NotFound,
        "Failed reading provided file path",
      ),
      source_path: path,
    });
  }

  if let Some(extension) = path.extension().and_then(OsStr::to_str) {
    if EXTENSION_WHITELIST.contains(&extension) {
      return Ok(path);
    }
  }

  Err(AppError::Io(io::Error::new(
    ErrorKind::InvalidFilename,
    "Failed reading file extension",
  )))
}

pub fn server_address_validation(str: &str) -> anyhow::Result<SocketAddr> {
  let socket_addr = SocketAddr::from_str(str)?;

  Ok(socket_addr)
}

pub fn port_validation(str: &str) -> anyhow::Result<u16> {
  let port = str.parse::<u16>()?;

  Ok(port)
}
