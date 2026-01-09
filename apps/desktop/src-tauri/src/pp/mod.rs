//! Portfolio Performance data models.
//!
//! This module contains the domain models for Portfolio Performance XML files.
//! These are separate from the database models used for local storage.

pub mod account;
pub mod client;
pub mod common;
pub mod portfolio;
pub mod security;
pub mod taxonomy;
pub mod transaction;

// Re-export main types for convenience
pub use account::Account;
pub use client::{Client, Dashboard, DashboardColumn, DashboardWidget, InvestmentPlan, Watchlist, CURRENT_VERSION};
pub use common::{ForexInfo, LatestPrice, Money, PriceEntry, UpdatedAt, AMOUNT_FACTOR, PRICE_FACTOR, SHARES_FACTOR};
pub use portfolio::Portfolio;
pub use security::{DividendEvent, Security, SecurityEvent, SecurityEventKind, SecurityEventType, SecurityType};
pub use taxonomy::{Classification, ClassificationAssignment, Taxonomy};
pub use transaction::{
    AccountTransaction, AccountTransactionType, CrossEntry, CrossEntryType,
    PortfolioTransaction, PortfolioTransactionType, TransactionUnit, UnitType,
};
