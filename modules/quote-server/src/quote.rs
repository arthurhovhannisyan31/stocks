use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct StockQuote {
  pub ticker: String,
  pub price: f64,
  pub volume: u32,
  pub timestamp: u64,
}

// Методы для сериализации/десериализации
impl StockQuote {
  pub fn to_string(&self) -> String {
    format!("{}|{}|{}|{}", self.ticker, self.price, self.volume, self.timestamp)
  }

  pub fn from_string(s: &str) -> Option<Self> {
    let parts: Vec<&str> = s.split('|').collect();
    if parts.len() == 4 {
      Some(StockQuote {
        ticker: parts[0].to_string(),
        price: parts[1].parse().ok()?,
        volume: parts[2].parse().ok()?,
        timestamp: parts[3].parse().ok()?,
      })
    } else {
      None
    }
  }

  // Или бинарная сериализация
  pub fn to_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(self.ticker.as_bytes());
    bytes.push(b'|');
    bytes.extend_from_slice(self.price.to_string().as_bytes());
    bytes.push(b'|');
    bytes.extend_from_slice(self.volume.to_string().as_bytes());
    bytes.push(b'|');
    bytes.extend_from_slice(self.timestamp.to_string().as_bytes());
    bytes
  }
}

struct QuoteGenerator;

impl QuoteGenerator {
  pub fn generate_quote(&mut self, ticker: &str) -> Option<StockQuote> {
    // ... логика изменения цены ...

    let volume = match ticker {
      // Популярные акции имеют больший объём
      "AAPL" | "MSFT" | "TSLA" => 1000 + (rand::random::<f64>() * 5000.0) as u32,
      // Обычные акции - средний объём
      _ => 100 + (rand::random::<f64>() * 1000.0) as u32,
    };

    Some(StockQuote {
      ticker: ticker.to_string(),
      // price: *last_price,
      price: 0.0,
      volume,
      timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
    })
  }
}