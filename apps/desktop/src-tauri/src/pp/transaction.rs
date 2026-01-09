//! Transaction types for Portfolio Performance.
//!
//! Transactions can belong to either accounts (AccountTransaction) or
//! portfolios (PortfolioTransaction). They share common fields but have
//! different transaction types.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use super::common::{ForexInfo, Money};

/// Unit type for transaction components (fees, taxes, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UnitType {
    /// Broker/transaction fee
    Fee,
    /// Tax amount
    Tax,
    /// Gross transaction value (before fees/taxes)
    GrossValue,
}

impl UnitType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "FEE" => Some(Self::Fee),
            "TAX" => Some(Self::Tax),
            "GROSS_VALUE" => Some(Self::GrossValue),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fee => "FEE",
            Self::Tax => "TAX",
            Self::GrossValue => "GROSS_VALUE",
        }
    }
}

/// A single unit (component) of a transaction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionUnit {
    pub unit_type: UnitType,
    pub amount: Money,
    /// Forex information if the unit was in a different currency
    pub forex: Option<ForexInfo>,
}

impl TransactionUnit {
    pub fn new(unit_type: UnitType, amount: Money) -> Self {
        Self {
            unit_type,
            amount,
            forex: None,
        }
    }

    pub fn with_forex(mut self, forex: ForexInfo) -> Self {
        self.forex = Some(forex);
        self
    }

    /// Create a fee unit
    pub fn fee(amount: Money) -> Self {
        Self::new(UnitType::Fee, amount)
    }

    /// Create a tax unit
    pub fn tax(amount: Money) -> Self {
        Self::new(UnitType::Tax, amount)
    }

    /// Create a gross value unit
    pub fn gross_value(amount: Money) -> Self {
        Self::new(UnitType::GrossValue, amount)
    }
}

/// Cross-entry type for linked transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CrossEntryType {
    /// Transfer between portfolios
    PortfolioTransfer,
    /// Transfer between accounts
    AccountTransfer,
    /// Buy/sell pair (portfolio transaction + account transaction)
    BuySell,
}

/// Cross-entry linking two related transactions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossEntry {
    pub entry_type: CrossEntryType,
    /// UUID of the source transaction
    pub source_uuid: String,
    /// UUID of the target transaction
    pub target_uuid: String,
}

impl CrossEntry {
    pub fn portfolio_transfer(source_uuid: String, target_uuid: String) -> Self {
        Self {
            entry_type: CrossEntryType::PortfolioTransfer,
            source_uuid,
            target_uuid,
        }
    }

    pub fn account_transfer(source_uuid: String, target_uuid: String) -> Self {
        Self {
            entry_type: CrossEntryType::AccountTransfer,
            source_uuid,
            target_uuid,
        }
    }

    pub fn buy_sell(source_uuid: String, target_uuid: String) -> Self {
        Self {
            entry_type: CrossEntryType::BuySell,
            source_uuid,
            target_uuid,
        }
    }
}

/// Account transaction types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountTransactionType {
    /// Cash deposit
    Deposit,
    /// Cash withdrawal
    Removal,
    /// Interest income
    Interest,
    /// Interest charge (negative)
    InterestCharge,
    /// Dividend payment
    Dividends,
    /// Fee charge
    Fees,
    /// Fee refund
    FeesRefund,
    /// Tax charge
    Taxes,
    /// Tax refund
    TaxRefund,
    /// Buy (debit when paying from cash)
    Buy,
    /// Sell (credit when receiving cash)
    Sell,
    /// Transfer in from another account
    TransferIn,
    /// Transfer out to another account
    TransferOut,
}

impl AccountTransactionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DEPOSIT" => Some(Self::Deposit),
            "REMOVAL" => Some(Self::Removal),
            "INTEREST" => Some(Self::Interest),
            "INTEREST_CHARGE" => Some(Self::InterestCharge),
            "DIVIDENDS" => Some(Self::Dividends),
            "FEES" => Some(Self::Fees),
            "FEES_REFUND" => Some(Self::FeesRefund),
            "TAXES" => Some(Self::Taxes),
            "TAX_REFUND" => Some(Self::TaxRefund),
            "BUY" => Some(Self::Buy),
            "SELL" => Some(Self::Sell),
            "TRANSFER_IN" => Some(Self::TransferIn),
            "TRANSFER_OUT" => Some(Self::TransferOut),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "DEPOSIT",
            Self::Removal => "REMOVAL",
            Self::Interest => "INTEREST",
            Self::InterestCharge => "INTEREST_CHARGE",
            Self::Dividends => "DIVIDENDS",
            Self::Fees => "FEES",
            Self::FeesRefund => "FEES_REFUND",
            Self::Taxes => "TAXES",
            Self::TaxRefund => "TAX_REFUND",
            Self::Buy => "BUY",
            Self::Sell => "SELL",
            Self::TransferIn => "TRANSFER_IN",
            Self::TransferOut => "TRANSFER_OUT",
        }
    }

    /// Is this a credit (money coming in)?
    pub fn is_credit(&self) -> bool {
        matches!(
            self,
            Self::Deposit
                | Self::Interest
                | Self::Dividends
                | Self::FeesRefund
                | Self::TaxRefund
                | Self::Sell
                | Self::TransferIn
        )
    }
}

/// Portfolio transaction types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PortfolioTransactionType {
    /// Purchase of securities
    Buy,
    /// Sale of securities
    Sell,
    /// Transfer in from another portfolio
    TransferIn,
    /// Transfer out to another portfolio
    TransferOut,
    /// Delivery inbound (non-cash inflow)
    DeliveryInbound,
    /// Delivery outbound (non-cash outflow)
    DeliveryOutbound,
}

impl PortfolioTransactionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "BUY" => Some(Self::Buy),
            "SELL" => Some(Self::Sell),
            "TRANSFER_IN" => Some(Self::TransferIn),
            "TRANSFER_OUT" => Some(Self::TransferOut),
            "DELIVERY_INBOUND" => Some(Self::DeliveryInbound),
            "DELIVERY_OUTBOUND" => Some(Self::DeliveryOutbound),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
            Self::TransferIn => "TRANSFER_IN",
            Self::TransferOut => "TRANSFER_OUT",
            Self::DeliveryInbound => "DELIVERY_INBOUND",
            Self::DeliveryOutbound => "DELIVERY_OUTBOUND",
        }
    }

    /// Is this a purchase (shares coming in)?
    pub fn is_purchase(&self) -> bool {
        matches!(
            self,
            Self::Buy | Self::TransferIn | Self::DeliveryInbound
        )
    }
}

/// Account transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransaction {
    pub uuid: String,
    pub date: NaiveDateTime,
    pub transaction_type: AccountTransactionType,
    pub amount: Money,
    /// Number of shares (for dividends) - stored as shares * 10^8
    pub shares: Option<i64>,
    /// Reference to security (for dividends, interest from securities)
    pub security_uuid: Option<String>,
    /// Transaction units (fees, taxes, forex)
    pub units: Vec<TransactionUnit>,
    /// Cross-entry for linked transactions
    pub cross_entry: Option<CrossEntry>,
    /// User note
    pub note: Option<String>,
    /// Source document reference
    pub source: Option<String>,
    /// Last update timestamp
    pub updated_at: Option<String>,
}

impl AccountTransaction {
    pub fn new(
        uuid: String,
        date: NaiveDateTime,
        transaction_type: AccountTransactionType,
        amount: Money,
    ) -> Self {
        Self {
            uuid,
            date,
            transaction_type,
            amount,
            shares: None,
            security_uuid: None,
            units: Vec::new(),
            cross_entry: None,
            note: None,
            source: None,
            updated_at: None,
        }
    }

    /// Calculate total fees from units
    pub fn total_fees(&self) -> i64 {
        self.units
            .iter()
            .filter(|u| u.unit_type == UnitType::Fee)
            .map(|u| u.amount.amount)
            .sum()
    }

    /// Calculate total taxes from units
    pub fn total_taxes(&self) -> i64 {
        self.units
            .iter()
            .filter(|u| u.unit_type == UnitType::Tax)
            .map(|u| u.amount.amount)
            .sum()
    }
}

/// Portfolio transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioTransaction {
    pub uuid: String,
    pub date: NaiveDateTime,
    pub transaction_type: PortfolioTransactionType,
    pub amount: Money,
    /// Number of shares - stored as shares * 10^8
    pub shares: i64,
    /// Reference to security
    pub security_uuid: Option<String>,
    /// Transaction units (fees, taxes, forex)
    pub units: Vec<TransactionUnit>,
    /// Cross-entry for linked transactions
    pub cross_entry: Option<CrossEntry>,
    /// User note
    pub note: Option<String>,
    /// Source document reference
    pub source: Option<String>,
    /// Last update timestamp
    pub updated_at: Option<String>,
}

impl PortfolioTransaction {
    pub fn new(
        uuid: String,
        date: NaiveDateTime,
        transaction_type: PortfolioTransactionType,
        amount: Money,
        shares: i64,
    ) -> Self {
        Self {
            uuid,
            date,
            transaction_type,
            amount,
            shares,
            security_uuid: None,
            units: Vec::new(),
            cross_entry: None,
            note: None,
            source: None,
            updated_at: None,
        }
    }

    /// Calculate total fees from units
    pub fn total_fees(&self) -> i64 {
        self.units
            .iter()
            .filter(|u| u.unit_type == UnitType::Fee)
            .map(|u| u.amount.amount)
            .sum()
    }

    /// Calculate total taxes from units
    pub fn total_taxes(&self) -> i64 {
        self.units
            .iter()
            .filter(|u| u.unit_type == UnitType::Tax)
            .map(|u| u.amount.amount)
            .sum()
    }

    /// Get shares as decimal
    pub fn shares_decimal(&self) -> f64 {
        super::common::shares::to_decimal(self.shares)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_type_parsing() {
        assert_eq!(UnitType::from_str("FEE"), Some(UnitType::Fee));
        assert_eq!(UnitType::from_str("TAX"), Some(UnitType::Tax));
        assert_eq!(UnitType::from_str("GROSS_VALUE"), Some(UnitType::GrossValue));
        assert_eq!(UnitType::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_portfolio_transaction_type_serialization() {
        // Test that serde serializes to SCREAMING_SNAKE_CASE
        let types = vec![
            (PortfolioTransactionType::Buy, "\"BUY\""),
            (PortfolioTransactionType::Sell, "\"SELL\""),
            (PortfolioTransactionType::DeliveryInbound, "\"DELIVERY_INBOUND\""),
            (PortfolioTransactionType::DeliveryOutbound, "\"DELIVERY_OUTBOUND\""),
            (PortfolioTransactionType::TransferIn, "\"TRANSFER_IN\""),
            (PortfolioTransactionType::TransferOut, "\"TRANSFER_OUT\""),
        ];

        for (tx_type, expected) in types {
            let json = serde_json::to_string(&tx_type).unwrap();
            println!("{:?} -> {}", tx_type, json);
            assert_eq!(json, expected, "Serialization of {:?} failed", tx_type);
        }
    }

    #[test]
    fn test_account_transaction_type_parsing() {
        assert_eq!(
            AccountTransactionType::from_str("DIVIDENDS"),
            Some(AccountTransactionType::Dividends)
        );
        assert_eq!(
            AccountTransactionType::from_str("DEPOSIT"),
            Some(AccountTransactionType::Deposit)
        );
        assert!(AccountTransactionType::Deposit.is_credit());
        assert!(!AccountTransactionType::Removal.is_credit());
    }

    #[test]
    fn test_portfolio_transaction_type_parsing() {
        assert_eq!(
            PortfolioTransactionType::from_str("BUY"),
            Some(PortfolioTransactionType::Buy)
        );
        assert_eq!(
            PortfolioTransactionType::from_str("DELIVERY_INBOUND"),
            Some(PortfolioTransactionType::DeliveryInbound)
        );
        assert!(PortfolioTransactionType::Buy.is_purchase());
        assert!(!PortfolioTransactionType::Sell.is_purchase());
    }

    #[test]
    fn test_transaction_fee_calculation() {
        let mut tx = PortfolioTransaction::new(
            "test".to_string(),
            chrono::NaiveDateTime::default(),
            PortfolioTransactionType::Buy,
            Money::new(10000, "EUR"),
            100_000_000,
        );

        tx.units.push(TransactionUnit::fee(Money::new(1000, "EUR")));
        tx.units.push(TransactionUnit::fee(Money::new(500, "EUR")));
        tx.units.push(TransactionUnit::tax(Money::new(200, "EUR")));

        assert_eq!(tx.total_fees(), 1500);
        assert_eq!(tx.total_taxes(), 200);
    }
}
