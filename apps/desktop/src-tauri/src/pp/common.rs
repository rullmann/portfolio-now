//! Common types shared across the Portfolio Performance data model.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

// =============================================================================
// Date Parsing (SINGLE SOURCE OF TRUTH)
// =============================================================================

/// Parse date string flexibly - handles multiple date formats
///
/// Supported formats:
/// - "YYYY-MM-DD" (e.g., "2024-01-15")
/// - "YYYY-MM-DD HH:MM:SS" (e.g., "2024-01-15 00:00:00")
/// - "YYYY-MM-DDTHH:MM:SS" (ISO8601, e.g., "2024-01-15T00:00:00")
///
/// # Example
/// ```
/// use crate::pp::common::parse_date_flexible;
/// let date = parse_date_flexible("2024-01-15").unwrap();
/// ```
pub fn parse_date_flexible(date_str: &str) -> Option<NaiveDate> {
    // Try date-only format first: "2024-01-15"
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        // Then try with time: "2024-01-15 00:00:00"
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
        // Then try ISO8601: "2024-01-15T00:00:00"
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
}

// =============================================================================
// Holdings SQL (SINGLE SOURCE OF TRUTH)
// =============================================================================

/// SQL fragment for calculating net shares from portfolio transactions.
///
/// Use this constant in SQL queries to ensure consistent holdings calculation.
/// BUY/TRANSFER_IN/DELIVERY_INBOUND add shares, SELL/TRANSFER_OUT/DELIVERY_OUTBOUND subtract.
///
/// # Usage
/// ```sql
/// SELECT security_id, {HOLDINGS_SUM_SQL} as net_shares
/// FROM pp_txn t
/// WHERE t.owner_type = 'portfolio'
/// GROUP BY security_id
/// ```
pub const HOLDINGS_SUM_SQL: &str = r#"SUM(CASE
    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
    ELSE 0
END)"#;

/// List of transaction types that ADD shares to holdings
pub const HOLDINGS_ADD_TYPES: &[&str] = &["BUY", "TRANSFER_IN", "DELIVERY_INBOUND"];

/// List of transaction types that REMOVE shares from holdings
pub const HOLDINGS_REMOVE_TYPES: &[&str] = &["SELL", "TRANSFER_OUT", "DELIVERY_OUTBOUND"];

/// Factor for converting shares (PP stores shares * 10^8)
pub const SHARES_FACTOR: i64 = 100_000_000;

/// Factor for converting amounts (PP stores amounts in cents)
pub const AMOUNT_FACTOR: i64 = 100;

/// Factor for converting prices (PP stores prices * 10^8)
pub const PRICE_FACTOR: i64 = 100_000_000;

/// Monetary amount with currency
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    /// Amount in smallest currency units (e.g., cents for EUR)
    pub amount: i64,
    /// ISO 4217 currency code (e.g., "EUR", "USD")
    pub currency: String,
}

impl Money {
    pub fn new(amount: i64, currency: impl Into<String>) -> Self {
        Self {
            amount,
            currency: currency.into(),
        }
    }

    /// Create a zero-value Money in the given currency
    pub fn zero(currency: impl Into<String>) -> Self {
        Self {
            amount: 0,
            currency: currency.into(),
        }
    }

    /// Convert to decimal representation (e.g., cents to euros)
    pub fn to_decimal(&self) -> f64 {
        self.amount as f64 / AMOUNT_FACTOR as f64
    }

    /// Create from decimal representation
    pub fn from_decimal(value: f64, currency: impl Into<String>) -> Self {
        Self {
            amount: (value * AMOUNT_FACTOR as f64).round() as i64,
            currency: currency.into(),
        }
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.amount == 0
    }

    /// Add another Money (must be same currency)
    pub fn add(&self, other: &Money) -> Option<Money> {
        if self.currency != other.currency {
            return None;
        }
        Some(Money {
            amount: self.amount + other.amount,
            currency: self.currency.clone(),
        })
    }
}

impl Default for Money {
    fn default() -> Self {
        Self {
            amount: 0,
            currency: "EUR".to_string(),
        }
    }
}

/// Forex conversion information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForexInfo {
    /// Amount in foreign currency (smallest units)
    pub amount: Money,
    /// Exchange rate used for conversion
    pub exchange_rate: f64,
}

impl ForexInfo {
    pub fn new(amount: Money, exchange_rate: f64) -> Self {
        Self {
            amount,
            exchange_rate,
        }
    }
}

/// Price entry for a security at a specific date
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceEntry {
    pub date: NaiveDate,
    /// Price in smallest currency units * 10^8
    pub value: i64,
}

impl PriceEntry {
    pub fn new(date: NaiveDate, value: i64) -> Self {
        Self { date, value }
    }

    /// Convert price value to decimal (e.g., 22552500000 → 225.525)
    pub fn to_decimal(&self) -> f64 {
        self.value as f64 / PRICE_FACTOR as f64
    }

    /// Create from decimal price
    pub fn from_decimal(date: NaiveDate, price: f64) -> Self {
        Self {
            date,
            value: (price * PRICE_FACTOR as f64).round() as i64,
        }
    }
}

/// Latest price with additional market data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestPrice {
    pub date: Option<NaiveDate>,
    /// Price in smallest currency units * 10^8
    pub value: Option<i64>,
    /// Day's high price
    pub high: Option<i64>,
    /// Day's low price
    pub low: Option<i64>,
    /// Trading volume
    pub volume: Option<i64>,
}

impl Default for LatestPrice {
    fn default() -> Self {
        Self {
            date: None,
            value: None,
            high: None,
            low: None,
            volume: None,
        }
    }
}

/// Timestamp for tracking entity updates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedAt(pub NaiveDateTime);

impl UpdatedAt {
    pub fn now() -> Self {
        Self(chrono::Utc::now().naive_utc())
    }

    pub fn parse(s: &str) -> Option<Self> {
        // PP format: "2021-04-19T17:27:17.175575Z" or "2025-10-17T07:49:53.771798Z"
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| Self(dt.naive_utc()))
            .or_else(|| {
                NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                    .ok()
                    .map(Self)
            })
    }
}

/// Helper functions for share conversions
pub mod shares {
    use super::SHARES_FACTOR;

    /// Convert from PP internal format (shares * 10^8) to decimal
    pub fn to_decimal(shares: i64) -> f64 {
        shares as f64 / SHARES_FACTOR as f64
    }

    /// Convert from decimal to PP internal format (shares * 10^8)
    pub fn from_decimal(shares: f64) -> i64 {
        (shares * SHARES_FACTOR as f64).round() as i64
    }
}

/// Helper functions for price conversions
pub mod prices {
    use super::PRICE_FACTOR;

    /// Convert from PP internal format (price * 10^8) to decimal
    pub fn to_decimal(price: i64) -> f64 {
        price as f64 / PRICE_FACTOR as f64
    }

    /// Convert from decimal to PP internal format (price * 10^8)
    pub fn from_decimal(price: f64) -> i64 {
        (price * PRICE_FACTOR as f64).round() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_money_operations() {
        let m1 = Money::new(10000, "EUR"); // 100.00 EUR
        assert_eq!(m1.to_decimal(), 100.0);

        let m2 = Money::from_decimal(50.0, "EUR");
        assert_eq!(m2.amount, 5000);

        let sum = m1.add(&m2).unwrap();
        assert_eq!(sum.amount, 15000);
        assert_eq!(sum.to_decimal(), 150.0);
    }

    #[test]
    fn test_money_different_currencies() {
        let eur = Money::new(100, "EUR");
        let usd = Money::new(100, "USD");
        assert!(eur.add(&usd).is_none());
    }

    #[test]
    fn test_price_conversion() {
        let price = PriceEntry::from_decimal(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(), 225.525);
        assert_eq!(price.value, 22552500000);
        assert!((price.to_decimal() - 225.525).abs() < 0.0001);
    }

    #[test]
    fn test_shares_conversion() {
        assert_eq!(shares::to_decimal(100_000_000), 1.0);
        assert_eq!(shares::to_decimal(150_000_000), 1.5);
        assert_eq!(shares::from_decimal(1.0), 100_000_000);
        assert_eq!(shares::from_decimal(2.5), 250_000_000);
    }

    #[test]
    fn test_updated_at_parsing() {
        let ts = UpdatedAt::parse("2021-04-19T17:27:17.175575Z").unwrap();
        assert_eq!(ts.0.date().year(), 2021);
        assert_eq!(ts.0.date().month(), 4);
    }
}
