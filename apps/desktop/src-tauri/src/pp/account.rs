//! Account model for Portfolio Performance.
//!
//! Accounts represent cash/deposit accounts that hold money (not securities).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::transaction::AccountTransaction;

/// A cash/deposit account
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub uuid: String,
    pub name: String,
    pub currency: String,
    /// Whether the account is retired/inactive
    pub is_retired: bool,
    /// User note
    pub note: Option<String>,
    /// Custom attributes
    pub attributes: HashMap<String, String>,
    /// Last update timestamp
    pub updated_at: Option<String>,
    /// Account transactions
    pub transactions: Vec<AccountTransaction>,
}

impl Account {
    pub fn new(uuid: String, name: String, currency: String) -> Self {
        Self {
            uuid,
            name,
            currency,
            is_retired: false,
            note: None,
            attributes: HashMap::new(),
            updated_at: None,
            transactions: Vec::new(),
        }
    }

    /// Calculate current balance from all transactions
    pub fn balance(&self) -> i64 {
        

        self.transactions.iter().fold(0i64, |acc, tx| {
            if tx.transaction_type.is_credit() {
                acc + tx.amount.amount
            } else {
                acc - tx.amount.amount
            }
        })
    }

    /// Get balance as decimal
    pub fn balance_decimal(&self) -> f64 {
        self.balance() as f64 / super::common::AMOUNT_FACTOR as f64
    }

    /// Sort transactions by date
    pub fn sort_transactions(&mut self) {
        self.transactions.sort_by_key(|tx| tx.date);
    }
}

impl Default for Account {
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
    use crate::pp::{common::Money, transaction::AccountTransactionType};
    use chrono::NaiveDateTime;

    #[test]
    fn test_account_balance_calculation() {
        let mut account = Account::new("test".to_string(), "Test Account".to_string(), "EUR".to_string());

        // Deposit 1000
        account.transactions.push(AccountTransaction::new(
            "tx1".to_string(),
            NaiveDateTime::default(),
            AccountTransactionType::Deposit,
            Money::new(100000, "EUR"), // 1000.00 EUR
        ));

        // Withdraw 300
        account.transactions.push(AccountTransaction::new(
            "tx2".to_string(),
            NaiveDateTime::default(),
            AccountTransactionType::Removal,
            Money::new(30000, "EUR"), // 300.00 EUR
        ));

        // Balance should be 700.00 EUR
        assert_eq!(account.balance(), 70000);
        assert_eq!(account.balance_decimal(), 700.0);
    }
}
