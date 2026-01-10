//! Security model for Portfolio Performance.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::common::{LatestPrice, Money, PriceEntry};

/// Security type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityType {
    #[default]
    Stock,
    Etf,
    Fund,
    Bond,
    Crypto,
    Commodity,
    Other,
}

/// Security event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityEventType {
    StockSplit,
    DividendPayment,
    Note,
}

impl SecurityEventType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "STOCK_SPLIT" => Some(Self::StockSplit),
            "DIVIDEND_PAYMENT" => Some(Self::DividendPayment),
            "NOTE" => Some(Self::Note),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StockSplit => "STOCK_SPLIT",
            Self::DividendPayment => "DIVIDEND_PAYMENT",
            Self::Note => "NOTE",
        }
    }
}

/// Security event (stock split, dividend announcement, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityEvent {
    pub date: NaiveDate,
    pub event_type: SecurityEventType,
    /// Details (e.g., "4:1" for stock split)
    pub details: Option<String>,
}

/// Dividend event with payment information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendEvent {
    /// Declaration/ex-dividend date
    pub date: NaiveDate,
    /// Data source
    pub source: Option<String>,
    /// Actual payment date
    pub payment_date: Option<NaiveDate>,
    /// Dividend amount per share
    pub amount: Option<Money>,
}

/// Unified security event enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityEventKind {
    Event(SecurityEvent),
    Dividend(DividendEvent),
}

/// A security (stock, ETF, fund, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Security {
    pub uuid: String,
    pub name: String,
    pub currency: String,
    /// Target currency for currency conversion (e.g., for exchange rate securities)
    pub target_currency: Option<String>,
    /// Online ID for data providers
    pub online_id: Option<String>,
    /// ISIN (International Securities Identification Number)
    pub isin: Option<String>,
    /// WKN (German security identification)
    pub wkn: Option<String>,
    /// Ticker symbol
    pub ticker: Option<String>,
    /// Trading calendar
    pub calendar: Option<String>,
    /// Historical price feed identifier
    pub feed: Option<String>,
    /// Historical price feed URL
    pub feed_url: Option<String>,
    /// Latest/current price feed identifier
    pub latest_feed: Option<String>,
    /// Latest/current price feed URL
    pub latest_feed_url: Option<String>,
    /// Historical prices
    pub prices: Vec<PriceEntry>,
    /// Latest price with market data
    pub latest: Option<LatestPrice>,
    /// Security events (splits, dividends, etc.)
    pub events: Vec<SecurityEventKind>,
    /// Custom attributes (user-defined key-value pairs)
    pub attributes: std::collections::HashMap<String, String>,
    /// Properties (system key-value pairs, separate from attributes)
    pub properties: std::collections::HashMap<String, String>,
    /// Whether the security is retired/inactive
    pub is_retired: bool,
    /// User note
    pub note: Option<String>,
    /// Last update timestamp
    pub updated_at: Option<String>,
}

impl Security {
    pub fn new(uuid: String, name: String, currency: String) -> Self {
        Self {
            uuid,
            name,
            currency,
            target_currency: None,
            online_id: None,
            isin: None,
            wkn: None,
            ticker: None,
            calendar: None,
            feed: None,
            feed_url: None,
            latest_feed: None,
            latest_feed_url: None,
            prices: Vec::new(),
            latest: None,
            events: Vec::new(),
            attributes: std::collections::HashMap::new(),
            properties: std::collections::HashMap::new(),
            is_retired: false,
            note: None,
            updated_at: None,
        }
    }

    /// Get the latest price as decimal
    pub fn latest_price_decimal(&self) -> Option<f64> {
        self.latest
            .as_ref()
            .and_then(|l| l.value)
            .map(|v| super::common::prices::to_decimal(v))
    }

    /// Get the most recent price from history
    pub fn most_recent_price(&self) -> Option<&PriceEntry> {
        self.prices.last()
    }

    /// Add a price entry (maintains sorted order by date)
    pub fn add_price(&mut self, entry: PriceEntry) {
        match self.prices.binary_search_by_key(&entry.date, |e| e.date) {
            Ok(pos) => self.prices[pos] = entry, // Update existing
            Err(pos) => self.prices.insert(pos, entry), // Insert at correct position
        }
    }
}

impl Default for Security {
    fn default() -> Self {
        Self::new(
            uuid::Uuid::new_v4().to_string(),
            String::new(),
            "EUR".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_creation() {
        let sec = Security::new("test-uuid".to_string(), "Apple Inc.".to_string(), "USD".to_string());
        assert_eq!(sec.uuid, "test-uuid");
        assert_eq!(sec.name, "Apple Inc.");
        assert_eq!(sec.currency, "USD");
        assert!(!sec.is_retired);
    }

    #[test]
    fn test_add_price_sorted() {
        let mut sec = Security::default();

        // Add prices out of order
        sec.add_price(PriceEntry::new(
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            100,
        ));
        sec.add_price(PriceEntry::new(
            NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
            90,
        ));
        sec.add_price(PriceEntry::new(
            NaiveDate::from_ymd_opt(2024, 1, 20).unwrap(),
            110,
        ));

        // Should be sorted
        assert_eq!(sec.prices[0].date, NaiveDate::from_ymd_opt(2024, 1, 10).unwrap());
        assert_eq!(sec.prices[1].date, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert_eq!(sec.prices[2].date, NaiveDate::from_ymd_opt(2024, 1, 20).unwrap());
    }

    #[test]
    fn test_security_event_type_parsing() {
        assert_eq!(
            SecurityEventType::from_str("STOCK_SPLIT"),
            Some(SecurityEventType::StockSplit)
        );
        assert_eq!(
            SecurityEventType::from_str("DIVIDEND_PAYMENT"),
            Some(SecurityEventType::DividendPayment)
        );
    }
}
