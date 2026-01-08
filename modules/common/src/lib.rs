//! This is a common crate, which contains structures, types and functions used in workspace crates.

use anyhow::{Context, Result};
use serde;
use signal_hook::{consts::TERM_SIGNALS, flag};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::{
  fs::File,
  io::{BufRead, BufReader},
  net::SocketAddr,
  path::PathBuf,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StockQuote {
  pub ticker: String,
  pub price: f64,
  pub volume: u32,
  pub timestamp: u64,
}

/// # StockRequest serialization and deserialization
///
/// # Serialization
/// ```
/// use std::io::Write;
/// use common::{StockRequest};
/// use anyhow::{Result, Context};
/// use serde_json::json;
///
///
/// fn read_stream(writer: &mut impl Write) -> Result<()>{
///   let stock_request = StockRequest {
///     kind: "STREAM".to_string(),
///     addr: "127.0.0.1:8080".parse()?,
///     tickers: vec![],
///   };
///
///   let message = json!(stock_request).to_string();
///   writer
///     .write_all(message.as_bytes())
///     .context("Failed writing to TCP stream")?;
///   writer.flush()?;
///
///   Ok(())
/// }
/// ```
/// # Deserialization
/// ```
/// use std::io::Read;
/// use common::{StockRequest};
/// use anyhow::{Result};
/// use serde_json::json;
///
/// fn read_stream(reader: &mut impl Read) -> Result<()>{
///   let mut buf = vec![0u8; 1024];
///   let n = reader.read(&mut buf)?;
///
///   let StockRequest {
///     kind,
///     addr,
///     tickers,
///   } = serde_json::from_slice::<StockRequest>(&buf[..n])?;
///
///   Ok(())
/// }
/// ```
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StockRequest {
  pub kind: String,
  pub addr: SocketAddr,
  pub tickers: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum StockResponseStatus {
  Ok,
  Error,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StockResponse {
  pub status: StockResponseStatus,
  pub message: String,
}

/// Read tickers from a file
///
/// # Example
///
/// ```
/// use common::{ read_tickers };
/// use anyhow::{Result};
/// use std::{path::PathBuf};
///
/// fn main() -> Result<()>{
///   let tickers: Vec<String> = read_tickers(PathBuf::from("../../mocks/server-tickers.txt"))?;
///
///   Ok(())
/// }
/// ```
pub fn read_tickers(path: PathBuf) -> Result<Vec<String>> {
  let tickers_file = File::open(path).context("Failed reading tickers file")?;
  let reader = BufReader::new(tickers_file);
  let lines = reader.lines();
  let tickers: Vec<String> = lines
    .filter_map(Result::ok)
    .map(|str| str.trim().to_string())
    .collect();

  Ok(tickers)
}

pub fn register_signal_hooks(shutdown: &Arc<AtomicBool>) -> Result<()> {
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
