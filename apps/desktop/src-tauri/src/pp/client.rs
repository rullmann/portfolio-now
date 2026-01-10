//! Client model - the root element of a Portfolio Performance file.

use serde::{Deserialize, Serialize};

use super::account::Account;
use super::portfolio::Portfolio;
use super::security::Security;
use super::taxonomy::Taxonomy;

/// Current Portfolio Performance file format version
pub const CURRENT_VERSION: i32 = 68;

/// The root client object representing a complete Portfolio Performance file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    /// File format version
    pub version: i32,
    /// Base currency (default: EUR)
    pub base_currency: String,
    /// All securities defined in the file
    pub securities: Vec<Security>,
    /// All cash accounts
    pub accounts: Vec<Account>,
    /// All portfolios (depots)
    pub portfolios: Vec<Portfolio>,
    /// Taxonomies for classification
    pub taxonomies: Vec<Taxonomy>,
    /// Watchlists
    pub watchlists: Vec<Watchlist>,
    /// Investment plans
    pub plans: Vec<InvestmentPlan>,
    /// Dashboards configuration
    pub dashboards: Vec<Dashboard>,
    /// Settings and properties (raw JSON for now)
    pub properties: serde_json::Value,
    pub settings: serde_json::Value,
}

impl Client {
    pub fn new(base_currency: impl Into<String>) -> Self {
        Self {
            version: CURRENT_VERSION,
            base_currency: base_currency.into(),
            securities: Vec::new(),
            accounts: Vec::new(),
            portfolios: Vec::new(),
            taxonomies: Vec::new(),
            watchlists: Vec::new(),
            plans: Vec::new(),
            dashboards: Vec::new(),
            properties: serde_json::Value::Null,
            settings: serde_json::Value::Null,
        }
    }

    /// Find a security by UUID
    pub fn find_security(&self, uuid: &str) -> Option<&Security> {
        self.securities.iter().find(|s| s.uuid == uuid)
    }

    /// Find a security by ISIN
    pub fn find_security_by_isin(&self, isin: &str) -> Option<&Security> {
        self.securities.iter().find(|s| s.isin.as_deref() == Some(isin))
    }

    /// Find an account by UUID
    pub fn find_account(&self, uuid: &str) -> Option<&Account> {
        self.accounts.iter().find(|a| a.uuid == uuid)
    }

    /// Find a portfolio by UUID
    pub fn find_portfolio(&self, uuid: &str) -> Option<&Portfolio> {
        self.portfolios.iter().find(|p| p.uuid == uuid)
    }

    /// Get all active (non-retired) securities
    pub fn active_securities(&self) -> Vec<&Security> {
        self.securities.iter().filter(|s| !s.is_retired).collect()
    }

    /// Get all active accounts
    pub fn active_accounts(&self) -> Vec<&Account> {
        self.accounts.iter().filter(|a| !a.is_retired).collect()
    }

    /// Get all active portfolios
    pub fn active_portfolios(&self) -> Vec<&Portfolio> {
        self.portfolios.iter().filter(|p| !p.is_retired).collect()
    }

    /// Calculate total holdings across all portfolios
    pub fn total_holdings(&self) -> std::collections::HashMap<String, i64> {
        let mut total = std::collections::HashMap::new();
        for portfolio in &self.portfolios {
            if portfolio.is_retired {
                continue;
            }
            for (sec_uuid, shares) in portfolio.holdings() {
                *total.entry(sec_uuid).or_insert(0) += shares;
            }
        }
        total
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new("EUR")
    }
}

/// A watchlist grouping securities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Watchlist {
    pub name: String,
    /// UUIDs of securities in this watchlist
    pub security_uuids: Vec<String>,
}

impl Watchlist {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            security_uuids: Vec::new(),
        }
    }
}

/// An investment plan for recurring transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestmentPlan {
    pub name: String,
    /// Security UUID (optional)
    pub security_uuid: Option<String>,
    /// Portfolio UUID
    pub portfolio_uuid: Option<String>,
    /// Account UUID
    pub account_uuid: Option<String>,
    /// Amount in smallest currency units
    pub amount: i64,
    /// Fees in smallest currency units
    pub fees: i64,
    /// Taxes in smallest currency units
    pub taxes: i64,
    /// Interval (<100 = months, >100 = weeks)
    pub interval: i32,
    /// Start date
    pub start: Option<String>,
    /// Auto-generate transactions
    pub auto_generate: bool,
    /// Plan type (PURCHASE_OR_DELIVERY=0, DEPOSIT=1, REMOVAL=2, INTEREST=3)
    pub plan_type: i32,
    /// Note/description
    pub note: Option<String>,
    /// Custom attributes
    pub attributes: std::collections::HashMap<String, String>,
    /// Generated transaction UUIDs (for tracking executed plans)
    pub transactions: Vec<String>,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub name: String,
    pub id: Option<String>,
    pub columns: Vec<DashboardColumn>,
    pub configuration: serde_json::Value,
}

/// Dashboard column
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardColumn {
    pub weight: Option<i32>,
    pub widgets: Vec<DashboardWidget>,
}

/// Dashboard widget
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardWidget {
    pub widget_type: String,
    pub label: Option<String>,
    pub configuration: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = Client::new("USD");
        assert_eq!(client.version, CURRENT_VERSION);
        assert_eq!(client.base_currency, "USD");
    }

    #[test]
    fn test_find_security() {
        let mut client = Client::default();
        client.securities.push(Security::new(
            "sec-1".to_string(),
            "Apple".to_string(),
            "USD".to_string(),
        ));

        assert!(client.find_security("sec-1").is_some());
        assert!(client.find_security("unknown").is_none());
    }
}
