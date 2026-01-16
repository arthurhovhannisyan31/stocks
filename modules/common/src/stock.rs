use std::net::SocketAddr;

use serde;

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
/// use common::{stock::{StockRequest}};
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
/// use common::{stock::{StockRequest}};
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
