use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub base_currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub note: Option<String>,
}

impl Portfolio {
    pub fn new(name: String, base_currency: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            base_currency,
            created_at: now,
            updated_at: now,
            note: None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Security {
    pub id: String,
    pub name: String,
    pub security_type: SecurityType,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub ticker: Option<String>,
    pub currency: String,
    pub feed: Option<String>,
    pub feed_url: Option<String>,
    pub latest_price: Option<f64>,
    pub latest_price_date: Option<NaiveDate>,
    pub note: Option<String>,
    pub is_retired: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityType {
    Stock,
    Etf,
    Fund,
    Bond,
    Crypto,
    Commodity,
    Other,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub portfolio_id: String,
    pub name: String,
    pub account_type: AccountType,
    pub currency: String,
    pub is_retired: bool,
    pub note: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    Depot,
    Cash,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    pub transaction_type: TransactionType,
    pub date: NaiveDate,
    pub security_id: Option<String>,
    pub shares: Option<f64>,
    pub amount: f64,
    pub currency_gross_amount: Option<f64>,
    pub exchange_rate: Option<f64>,
    pub fees: f64,
    pub taxes: f64,
    pub note: Option<String>,
    pub source: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    Buy,
    Sell,
    Dividend,
    Interest,
    Deposit,
    Withdrawal,
    TransferIn,
    TransferOut,
    Fees,
    Taxes,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceEntry {
    pub security_id: String,
    pub date: NaiveDate,
    pub close: f64,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub volume: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taxonomy {
    pub id: String,
    pub portfolio_id: String,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxonomyClassification {
    pub id: String,
    pub taxonomy_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub weight: Option<f64>,
}
