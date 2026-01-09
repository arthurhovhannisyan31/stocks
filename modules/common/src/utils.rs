use crate::error::AppError;
use signal_hook::{consts::TERM_SIGNALS, flag};
use std::{
  fs::File,
  io::{BufRead, BufReader},
  path::PathBuf,
  sync::Arc,
  sync::atomic::AtomicBool,
};

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
  let tickers_file = File::open(path)?;
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
