use anyhow::Context;
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
/// use common::{ utils::{read_tickers} };
/// use anyhow::{Result};
/// use std::{path::PathBuf};
///
/// fn main() -> Result<()>{
///   let tickers: Vec<String> = read_tickers(PathBuf::from("../../mocks/server-tickers.txt"))?;
///
///   Ok(())
/// }
/// ```
pub fn read_tickers(path: PathBuf) -> anyhow::Result<Vec<String>> {
  let tickers_file = File::open(path).context("Failed reading tickers file")?;
  let reader = BufReader::new(tickers_file);
  let lines = reader.lines();
  let tickers: Vec<String> = lines
    .filter_map(anyhow::Result::ok)
    .map(|str| str.trim().to_string())
    .collect();

  Ok(tickers)
}

// "Failed registering conditional_shutdown for signal {sig:?}"
// format!("Failed registering signal {sig:?}"))?;

pub fn register_signal_hooks(shutdown: &Arc<AtomicBool>) -> anyhow::Result<()> {
  for sig in TERM_SIGNALS {
    flag::register_conditional_shutdown(*sig, 1, Arc::clone(&shutdown))
      .context(format!(
        "Failed registering conditional_shutdown for signal {sig:?}"
      ))?;
    flag::register(*sig, Arc::clone(&shutdown))
      .context(format!("Failed registering signal {sig:?}"))?;
  }

  Ok(())
}
