use anyhow::Result;
use serde;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StockQuote {
  pub ticker: String,
  pub price: f64,
  pub volume: u32,
  pub timestamp: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StockRequest {
  pub kind: String,
  pub addr: SocketAddr,
  pub tickers: Vec<String>,
}

pub fn read_tickers(path: PathBuf) -> Result<Vec<String>> {
  let tickers_file = File::open(path)?;
  let reader = BufReader::new(tickers_file);
  let lines = reader.lines();
  let tickers: Vec<String> = lines
    .filter_map(Result::ok)
    .map(|str| str.trim().to_string())
    .collect();

  Ok(tickers)
}
