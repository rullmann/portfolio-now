//! Portfolio model for Portfolio Performance.
//!
//! Portfolios (Depots) hold securities and track buy/sell transactions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::transaction::PortfolioTransaction;

/// A securities portfolio (Depot)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Portfolio {
    pub uuid: String,
    pub name: String,
    /// Reference to the linked cash account (for settlements)
    pub reference_account_uuid: Option<String>,
    /// Whether the portfolio is retired/inactive
    pub is_retired: bool,
    /// User note
    pub note: Option<String>,
    /// Custom attributes
    pub attributes: HashMap<String, String>,
    /// Last update timestamp
    pub updated_at: Option<String>,
    /// Portfolio transactions
    pub transactions: Vec<PortfolioTransaction>,
}

impl Portfolio {
    pub fn new(uuid: String, name: String) -> Self {
        Self {
            uuid,
            name,
            reference_account_uuid: None,
            is_retired: false,
            note: None,
            attributes: HashMap::new(),
            updated_at: None,
            transactions: Vec::new(),
        }
    }

    /// Calculate holdings (security_uuid → shares)
    pub fn holdings(&self) -> HashMap<String, i64> {
        

        let mut holdings: HashMap<String, i64> = HashMap::new();

        for tx in &self.transactions {
            if let Some(ref sec_uuid) = tx.security_uuid {
                let entry = holdings.entry(sec_uuid.clone()).or_insert(0);
                if tx.transaction_type.is_purchase() {
                    *entry += tx.shares;
                } else {
                    *entry -= tx.shares;
                }
            }
        }

        // Remove zero holdings
        holdings.retain(|_, &mut shares| shares > 0);
        holdings
    }

    /// Sort transactions by date
    pub fn sort_transactions(&mut self) {
        self.transactions.sort_by_key(|tx| tx.date);
    }

    /// Get all transactions for a specific security
    pub fn transactions_for_security(&self, security_uuid: &str) -> Vec<&PortfolioTransaction> {
        self.transactions
            .iter()
            .filter(|tx| tx.security_uuid.as_deref() == Some(security_uuid))
            .collect()
    }
}

impl Default for Portfolio {
    fn default() -> Self {
        Self::new(uuid::Uuid::new_v4().to_string(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pp::common::Money;
    use crate::pp::transaction::PortfolioTransactionType;
    use chrono::NaiveDateTime;

    #[test]
    fn test_portfolio_holdings_calculation() {
        let mut portfolio = Portfolio::new("test".to_string(), "Test Portfolio".to_string());

        // Buy 10 shares of security A
        let mut tx1 = PortfolioTransaction::new(
            "tx1".to_string(),
            NaiveDateTime::default(),
            PortfolioTransactionType::Buy,
            Money::new(100000, "EUR"),
            1_000_000_000, // 10 shares
        );
        tx1.security_uuid = Some("sec-a".to_string());
        portfolio.transactions.push(tx1);

        // Buy 5 more shares of security A
        let mut tx2 = PortfolioTransaction::new(
            "tx2".to_string(),
            NaiveDateTime::default(),
            PortfolioTransactionType::DeliveryInbound,
            Money::new(50000, "EUR"),
            500_000_000, // 5 shares
        );
        tx2.security_uuid = Some("sec-a".to_string());
        portfolio.transactions.push(tx2);

        // Sell 3 shares of security A
        let mut tx3 = PortfolioTransaction::new(
            "tx3".to_string(),
            NaiveDateTime::default(),
            PortfolioTransactionType::Sell,
            Money::new(30000, "EUR"),
            300_000_000, // 3 shares
        );
        tx3.security_uuid = Some("sec-a".to_string());
        portfolio.transactions.push(tx3);

        // Buy 20 shares of security B
        let mut tx4 = PortfolioTransaction::new(
            "tx4".to_string(),
            NaiveDateTime::default(),
            PortfolioTransactionType::Buy,
            Money::new(200000, "EUR"),
            2_000_000_000, // 20 shares
        );
        tx4.security_uuid = Some("sec-b".to_string());
        portfolio.transactions.push(tx4);

        let holdings = portfolio.holdings();

        // Security A: 10 + 5 - 3 = 12 shares
        assert_eq!(holdings.get("sec-a"), Some(&1_200_000_000));
        // Security B: 20 shares
        assert_eq!(holdings.get("sec-b"), Some(&2_000_000_000));
    }
}
