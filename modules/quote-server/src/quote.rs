use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use common::StockQuote;
use rand::Rng;

use crate::configs::consts;

pub struct QuoteGenerator {
  price_map: HashMap<String, f64>,
}

impl QuoteGenerator {
  pub fn new(tickers: &Vec<String>) -> Self {
    Self {
      price_map: tickers.iter().map(|val| (val.clone(), 1.0)).collect(),
    }
  }
  pub fn generate_quote(&self, ticker: &str) -> StockQuote {
    let last_price = self
      .price_map
      .get(ticker)
      .unwrap_or(&consts::QUOTE_DEFAULT_PRICE);

    let volume = match ticker {
      "AAPL" | "GOOGL" | "MSFT" | "TSLA" | "NVDA" => {
        1000 + (rand::random::<f64>() * 5000.0) as u32
      }
      _ => 100 + (rand::random::<f64>() * 1000.0) as u32,
    };

    StockQuote {
      ticker: ticker.to_string(),
      price: *last_price,
      volume,
      timestamp: SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64,
    }
  }
  pub fn shuffle_prices(&mut self) {
    let mut rng = rand::rng();
    let ratio = rng.random_range(0.5..1.5);

    for (_, val) in self.price_map.iter_mut() {
      *val *= ratio;
    }
  }
  pub fn generate_quote_list(&self) -> Vec<StockQuote> {
    self
      .price_map
      .iter()
      .map(|(key, _)| self.generate_quote(key))
      .collect()
  }
}
