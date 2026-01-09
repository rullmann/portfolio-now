//! Protobuf schema definitions for Portfolio Performance binary format.
//!
//! These structures are reverse-engineered from the binary format.
//! Field numbers are based on analysis of real .portfolio files using `protoc --decode_raw`.

#![allow(dead_code)]

use prost::Message;

/// Root client message (OFFICIAL from client.proto)
/// Contains all portfolio data
#[derive(Clone, PartialEq, Message)]
pub struct PClient {
    /// Portfolio file version (e.g., 68)
    #[prost(int32, tag = "1")]
    pub version: i32,

    /// List of securities
    #[prost(message, repeated, tag = "2")]
    pub securities: Vec<PSecurity>,

    /// List of accounts
    #[prost(message, repeated, tag = "3")]
    pub accounts: Vec<PAccount>,

    /// List of portfolios (depots)
    #[prost(message, repeated, tag = "4")]
    pub portfolios: Vec<PPortfolio>,

    /// All transactions (unified flat list)
    #[prost(message, repeated, tag = "5")]
    pub transactions: Vec<PTransaction>,

    /// Investment plans
    #[prost(message, repeated, tag = "6")]
    pub plans: Vec<PInvestmentPlan>,

    /// Watchlists
    #[prost(message, repeated, tag = "7")]
    pub watchlists: Vec<PWatchlist>,

    /// Taxonomies (classification systems)
    #[prost(message, repeated, tag = "8")]
    pub taxonomies: Vec<PTaxonomy>,

    /// Dashboard configurations
    #[prost(message, repeated, tag = "9")]
    pub dashboards: Vec<PDashboard>,

    /// Properties (key-value map, stored as repeated for prost compatibility)
    #[prost(message, repeated, tag = "10")]
    pub properties: Vec<PProperty>,

    /// Settings (bookmarks, attribute types, configuration sets)
    #[prost(message, optional, tag = "11")]
    pub settings: Option<PSettings>,

    /// Base currency (e.g., "EUR")
    #[prost(string, tag = "12")]
    pub base_currency: String,

    // Note: tag 99 is reserved for extensions (google.protobuf.Any)
}

/// Property (key-value setting)
#[derive(Clone, PartialEq, Message)]
pub struct PProperty {
    /// Key
    #[prost(string, tag = "1")]
    pub key: String,

    /// Value
    #[prost(string, tag = "2")]
    pub value: String,
}

/// Transaction (OFFICIAL unified format from client.proto)
/// All transaction types use this single message
#[derive(Clone, PartialEq, Message)]
pub struct PTransaction {
    /// UUID
    #[prost(string, tag = "1")]
    pub uuid: String,

    /// Transaction type (see transaction_type module for values)
    /// PURCHASE=0, SALE=1, INBOUND_DELIVERY=2, OUTBOUND_DELIVERY=3, etc.
    #[prost(int32, tag = "2")]
    pub transaction_type: i32,

    /// Account UUID (for account transactions)
    #[prost(string, optional, tag = "3")]
    pub account: Option<String>,

    /// Portfolio UUID (for portfolio transactions)
    #[prost(string, optional, tag = "4")]
    pub portfolio: Option<String>,

    /// Other account UUID (for transfers: target account)
    #[prost(string, optional, tag = "5")]
    pub other_account: Option<String>,

    /// Other portfolio UUID (for transfers: target portfolio)
    #[prost(string, optional, tag = "6")]
    pub other_portfolio: Option<String>,

    /// Cross-entry UUID (links to counterpart transaction)
    #[prost(string, optional, tag = "7")]
    pub other_uuid: Option<String>,

    /// Cross-entry updated timestamp
    #[prost(message, optional, tag = "8")]
    pub other_updated_at: Option<PTimestamp>,

    /// Transaction date (Timestamp)
    #[prost(message, optional, tag = "9")]
    pub date: Option<PTimestamp>,

    /// Currency code
    #[prost(string, tag = "10")]
    pub currency_code: String,

    /// Amount in smallest units (cents)
    #[prost(int64, tag = "11")]
    pub amount: i64,

    /// Shares * 10^8
    #[prost(int64, optional, tag = "12")]
    pub shares: Option<i64>,

    /// Note/memo
    #[prost(string, optional, tag = "13")]
    pub note: Option<String>,

    /// Security UUID
    #[prost(string, optional, tag = "14")]
    pub security: Option<String>,

    /// Transaction units (fees, taxes, gross value)
    #[prost(message, repeated, tag = "15")]
    pub units: Vec<PTransactionUnit>,

    /// Last update timestamp
    #[prost(message, optional, tag = "16")]
    pub updated_at: Option<PTimestamp>,

    /// Source (import source, etc.)
    #[prost(string, optional, tag = "17")]
    pub source: Option<String>,
}

/// Transaction detail (timestamp + amount)
#[derive(Clone, PartialEq, Message)]
pub struct PTransactionDetail {
    /// Timestamp (Unix seconds)
    #[prost(int64, tag = "1")]
    pub timestamp: i64,

    /// Amount (scaled value)
    #[prost(int64, optional, tag = "2")]
    pub amount: Option<i64>,
}

/// Security definition (OFFICIAL from client.proto)
#[derive(Clone, PartialEq, Message)]
pub struct PSecurity {
    /// UUID
    #[prost(string, tag = "1")]
    pub uuid: String,

    /// Online ID (optional, rarely used)
    #[prost(string, optional, tag = "2")]
    pub online_id: Option<String>,

    /// Name (e.g., "Apple Inc.")
    #[prost(string, tag = "3")]
    pub name: String,

    /// Currency code (e.g., "EUR")
    #[prost(string, optional, tag = "4")]
    pub currency_code: Option<String>,

    /// Target currency code (for currency conversion)
    #[prost(string, optional, tag = "5")]
    pub target_currency_code: Option<String>,

    /// Note/description
    #[prost(string, optional, tag = "6")]
    pub note: Option<String>,

    /// ISIN (e.g., "US0378331005")
    #[prost(string, optional, tag = "7")]
    pub isin: Option<String>,

    /// Ticker symbol (e.g., "APC.DE")
    #[prost(string, optional, tag = "8")]
    pub ticker_symbol: Option<String>,

    /// WKN (e.g., "865985")
    #[prost(string, optional, tag = "9")]
    pub wkn: Option<String>,

    /// Calendar (rarely used)
    #[prost(string, optional, tag = "10")]
    pub calendar: Option<String>,

    /// Quote feed (e.g., "YAHOO")
    #[prost(string, optional, tag = "11")]
    pub feed: Option<String>,

    /// Feed URL
    #[prost(string, optional, tag = "12")]
    pub feed_url: Option<String>,

    /// Historical prices
    #[prost(message, repeated, tag = "13")]
    pub prices: Vec<PPrice>,

    /// Latest quote feed
    #[prost(string, optional, tag = "14")]
    pub latest_feed: Option<String>,

    /// Latest quote feed URL
    #[prost(string, optional, tag = "15")]
    pub latest_feed_url: Option<String>,

    /// Latest price (OHLCV)
    #[prost(message, optional, tag = "16")]
    pub latest: Option<PFullHistoricalPrice>,

    /// Custom attributes
    #[prost(message, repeated, tag = "17")]
    pub attributes: Vec<PKeyValue>,

    /// Security events (dividends, splits, notes)
    #[prost(message, repeated, tag = "18")]
    pub events: Vec<PSecurityEvent>,

    /// Properties
    #[prost(message, repeated, tag = "19")]
    pub properties: Vec<PKeyValue>,

    /// Is security retired/inactive
    #[prost(bool, tag = "20")]
    pub is_retired: bool,

    /// Last update timestamp (seconds since epoch)
    #[prost(message, optional, tag = "21")]
    pub updated_at: Option<PTimestamp>,
}

/// Timestamp (simplified google.protobuf.Timestamp)
#[derive(Clone, PartialEq, Message)]
pub struct PTimestamp {
    #[prost(int64, tag = "1")]
    pub seconds: i64,

    #[prost(int32, tag = "2")]
    pub nanos: i32,
}

/// Full historical price with OHLCV
#[derive(Clone, PartialEq, Message)]
pub struct PFullHistoricalPrice {
    /// Date as epoch day
    #[prost(int64, tag = "1")]
    pub date: i64,

    /// Close price
    #[prost(int64, tag = "2")]
    pub close: i64,

    /// High price
    #[prost(int64, optional, tag = "3")]
    pub high: Option<i64>,

    /// Low price
    #[prost(int64, optional, tag = "4")]
    pub low: Option<i64>,

    /// Volume
    #[prost(int64, optional, tag = "5")]
    pub volume: Option<i64>,
}

/// Historical price entry
#[derive(Clone, PartialEq, Message)]
pub struct PPrice {
    /// Date as days since epoch
    #[prost(int64, tag = "1")]
    pub date: i64,

    /// Price value (scaled by 10^8)
    #[prost(int64, tag = "2")]
    pub value: i64,
}

// PLatestPrice replaced by PFullHistoricalPrice above

// ============================================================================
// OFFICIAL Helper Messages (from client.proto)
// ============================================================================

/// Decimal value with arbitrary precision (for exchange rates)
#[derive(Clone, PartialEq, Message)]
pub struct PDecimalValue {
    #[prost(uint32, tag = "1")]
    pub scale: u32,

    #[prost(uint32, tag = "2")]
    pub precision: u32,

    #[prost(bytes = "vec", tag = "3")]
    pub value: Vec<u8>,
}

/// Flexible value container (oneof)
#[derive(Clone, PartialEq, Message)]
pub struct PAnyValue {
    #[prost(oneof = "PAnyValueKind", tags = "1, 2, 3, 4, 5, 6, 7")]
    pub kind: Option<PAnyValueKind>,
}

/// Oneof variants for PAnyValue
#[derive(Clone, PartialEq, prost::Oneof)]
pub enum PAnyValueKind {
    /// Null value (tag 1)
    #[prost(int32, tag = "1")]
    Null(i32), // google.protobuf.NullValue is just an int enum

    /// String value (tag 2)
    #[prost(string, tag = "2")]
    String(String),

    /// Int32 value (tag 3)
    #[prost(int32, tag = "3")]
    Int32(i32),

    /// Int64 value (tag 4)
    #[prost(int64, tag = "4")]
    Int64(i64),

    /// Double value (tag 5)
    #[prost(double, tag = "5")]
    Double(f64),

    /// Bool value (tag 6)
    #[prost(bool, tag = "6")]
    Bool(bool),

    /// Map value (tag 7)
    #[prost(message, tag = "7")]
    Map(Box<PMap>),
}

/// Key-value pair
#[derive(Clone, PartialEq, Message)]
pub struct PKeyValue {
    #[prost(string, tag = "1")]
    pub key: String,

    #[prost(message, optional, tag = "2")]
    pub value: Option<PAnyValue>,
}

/// Map of key-value pairs
#[derive(Clone, PartialEq, Message)]
pub struct PMap {
    #[prost(message, repeated, tag = "1")]
    pub entries: Vec<PKeyValue>,
}

// ============================================================================
// Security Event (OFFICIAL from client.proto)
// ============================================================================

/// Security event (stock split, note, dividend payment)
#[derive(Clone, PartialEq, Message)]
pub struct PSecurityEvent {
    /// Event type (enum: STOCK_SPLIT=0, NOTE=1, DIVIDEND_PAYMENT=2)
    #[prost(int32, tag = "1")]
    pub event_type: i32,

    /// Event date as epoch day (days since 1970-01-01)
    #[prost(int64, tag = "2")]
    pub date: i64,

    /// Details/description
    #[prost(string, tag = "3")]
    pub details: String,

    /// Additional data (flexible key-value)
    #[prost(message, repeated, tag = "4")]
    pub data: Vec<PAnyValue>,

    /// Source of the event
    #[prost(string, optional, tag = "5")]
    pub source: Option<String>,
}

/// Money amount with currency
#[derive(Clone, PartialEq, Message)]
pub struct PMoney {
    /// Amount in smallest units (cents)
    #[prost(int64, tag = "1")]
    pub amount: i64,

    /// Currency code
    #[prost(string, tag = "2")]
    pub currency_code: String,
}

/// Key-value attribute
#[derive(Clone, PartialEq, Message)]
pub struct PAttribute {
    #[prost(string, tag = "1")]
    pub key: String,

    #[prost(string, tag = "2")]
    pub value: String,
}

/// Account definition (OFFICIAL from client.proto)
#[derive(Clone, PartialEq, Message)]
pub struct PAccount {
    /// UUID
    #[prost(string, tag = "1")]
    pub uuid: String,

    /// Name
    #[prost(string, tag = "2")]
    pub name: String,

    /// Currency code
    #[prost(string, tag = "3")]
    pub currency_code: String,

    /// Note/description
    #[prost(string, optional, tag = "4")]
    pub note: Option<String>,

    /// Is account retired/inactive
    #[prost(bool, tag = "5")]
    pub is_retired: bool,

    /// Custom attributes
    #[prost(message, repeated, tag = "6")]
    pub attributes: Vec<PKeyValue>,

    /// Last update timestamp
    #[prost(message, optional, tag = "7")]
    pub updated_at: Option<PTimestamp>,
}

/// Account transaction
/// Based on real file: very simple structure with just timestamp and amount
#[derive(Clone, PartialEq, Message)]
pub struct PAccountTransaction {
    /// Timestamp (Unix seconds or similar)
    #[prost(int64, tag = "1")]
    pub timestamp: i64,

    /// Amount (scaled value)
    #[prost(int64, tag = "2")]
    pub amount: i64,
}

/// Portfolio (depot) definition (OFFICIAL from client.proto)
#[derive(Clone, PartialEq, Message)]
pub struct PPortfolio {
    /// UUID
    #[prost(string, tag = "1")]
    pub uuid: String,

    /// Name
    #[prost(string, tag = "2")]
    pub name: String,

    /// Note/description
    #[prost(string, optional, tag = "3")]
    pub note: Option<String>,

    /// Is portfolio retired/inactive
    #[prost(bool, tag = "4")]
    pub is_retired: bool,

    /// Reference account UUID
    #[prost(string, optional, tag = "5")]
    pub reference_account: Option<String>,

    /// Custom attributes
    #[prost(message, repeated, tag = "6")]
    pub attributes: Vec<PKeyValue>,

    /// Last update timestamp
    #[prost(message, optional, tag = "7")]
    pub updated_at: Option<PTimestamp>,
}

/// Portfolio transaction
/// Based on real file: same simple structure as account transactions
#[derive(Clone, PartialEq, Message)]
pub struct PPortfolioTransaction {
    /// Timestamp (Unix seconds or similar)
    #[prost(int64, tag = "1")]
    pub timestamp: i64,

    /// Amount (scaled value)
    #[prost(int64, tag = "2")]
    pub amount: i64,
}

/// Transaction unit (OFFICIAL from client.proto)
/// Type enum: GROSS_VALUE=0, TAX=1, FEE=2
#[derive(Clone, PartialEq, Message)]
pub struct PTransactionUnit {
    /// Unit type (enum: GROSS_VALUE=0, TAX=1, FEE=2)
    #[prost(int32, tag = "1")]
    pub unit_type: i32,

    /// Amount in smallest units
    #[prost(int64, tag = "2")]
    pub amount: i64,

    /// Currency code
    #[prost(string, tag = "3")]
    pub currency_code: String,

    /// Forex: Amount in foreign currency
    #[prost(int64, optional, tag = "4")]
    pub fx_amount: Option<i64>,

    /// Forex: Foreign currency code
    #[prost(string, optional, tag = "5")]
    pub fx_currency_code: Option<String>,

    /// Forex: Exchange rate to base currency (as PDecimalValue)
    #[prost(message, optional, tag = "6")]
    pub fx_rate_to_base: Option<PDecimalValue>,
}

/// Cross-entry reference for transfers
#[derive(Clone, PartialEq, Message)]
pub struct PCrossEntry {
    /// Account index
    #[prost(int32, tag = "1")]
    pub account: i32,

    /// Portfolio index
    #[prost(int32, tag = "2")]
    pub portfolio: i32,

    /// Transaction UUID of counterpart
    #[prost(string, tag = "3")]
    pub counterpart_uuid: String,
}

/// Watchlist definition (OFFICIAL from client.proto)
#[derive(Clone, PartialEq, Message)]
pub struct PWatchlist {
    /// Name
    #[prost(string, tag = "1")]
    pub name: String,

    /// Security UUIDs
    #[prost(string, repeated, tag = "2")]
    pub securities: Vec<String>,
}

/// Investment plan definition (OFFICIAL from client.proto)
#[derive(Clone, PartialEq, Message)]
pub struct PInvestmentPlan {
    /// Name
    #[prost(string, tag = "1")]
    pub name: String,

    /// Note/description
    #[prost(string, optional, tag = "2")]
    pub note: Option<String>,

    /// Security UUID
    #[prost(string, optional, tag = "3")]
    pub security: Option<String>,

    /// Portfolio UUID
    #[prost(string, optional, tag = "4")]
    pub portfolio: Option<String>,

    /// Account UUID
    #[prost(string, optional, tag = "5")]
    pub account: Option<String>,

    /// Custom attributes
    #[prost(message, repeated, tag = "6")]
    pub attributes: Vec<PKeyValue>,

    /// Auto-generate transactions
    #[prost(bool, tag = "7")]
    pub auto_generate: bool,

    /// Start date (epoch day)
    #[prost(int64, tag = "8")]
    pub date: i64,

    /// Interval (days between executions)
    #[prost(int32, tag = "9")]
    pub interval: i32,

    /// Amount in smallest units
    #[prost(int64, tag = "10")]
    pub amount: i64,

    /// Fees in smallest units
    #[prost(int64, tag = "11")]
    pub fees: i64,

    /// Transaction UUIDs (generated transactions)
    #[prost(string, repeated, tag = "12")]
    pub transactions: Vec<String>,

    /// Taxes in smallest units
    #[prost(int64, tag = "13")]
    pub taxes: i64,

    /// Plan type (PURCHASE_OR_DELIVERY=0, DEPOSIT=1, REMOVAL=2, INTEREST=3)
    #[prost(int32, tag = "14")]
    pub plan_type: i32,
}

/// Taxonomy (classification system)
/// Based on real file: tags 1, 2, 4, 5 are used
#[derive(Clone, PartialEq, Message)]
pub struct PTaxonomy {
    /// UUID/ID
    #[prost(string, tag = "1")]
    pub id: String,

    /// Name
    #[prost(string, tag = "2")]
    pub name: String,

    /// Dimensions (tag 4, repeated)
    #[prost(string, repeated, tag = "4")]
    pub dimensions: Vec<String>,

    /// Classifications (tag 5, repeated - flat list)
    #[prost(message, repeated, tag = "5")]
    pub classifications: Vec<PClassification>,
}

/// Classification within a taxonomy
/// Based on real file: tags 1, 2, 3, 5, 6, 8, 9 are used
#[derive(Clone, PartialEq, Message)]
pub struct PClassification {
    /// UUID/ID
    #[prost(string, tag = "1")]
    pub id: String,

    /// Parent UUID (optional)
    #[prost(string, optional, tag = "2")]
    pub parent_id: Option<String>,

    /// Name
    #[prost(string, tag = "3")]
    pub name: String,

    /// Color (hex, e.g., "#0000ff")
    #[prost(string, optional, tag = "5")]
    pub color: Option<String>,

    /// Weight (percentage, e.g., 10000 = 100%)
    #[prost(int32, optional, tag = "6")]
    pub weight: Option<i32>,

    /// Data/properties (tag 8, nested message)
    #[prost(message, optional, tag = "8")]
    pub data: Option<PClassificationData>,

    /// Assignments (tag 9, repeated)
    #[prost(message, repeated, tag = "9")]
    pub assignments: Vec<PClassificationAssignment>,
}

/// Classification data/properties
#[derive(Clone, PartialEq, Message)]
pub struct PClassificationData {
    /// Key
    #[prost(string, tag = "1")]
    pub key: String,

    /// Nested value
    #[prost(message, optional, tag = "2")]
    pub value: Option<PClassificationDataValue>,
}

/// Classification data value
#[derive(Clone, PartialEq, Message)]
pub struct PClassificationDataValue {
    /// Value string
    #[prost(string, optional, tag = "2")]
    pub value: Option<String>,
}

/// Classification assignment (security/account to classification)
/// Based on real file: tags 1, 2, 3 are used
#[derive(Clone, PartialEq, Message)]
pub struct PClassificationAssignment {
    /// Vehicle UUID (security or account)
    #[prost(string, tag = "1")]
    pub vehicle_uuid: String,

    /// Weight
    #[prost(int32, optional, tag = "2")]
    pub weight: Option<i32>,

    /// Rank
    #[prost(int32, optional, tag = "3")]
    pub rank: Option<i32>,
}

/// Assignment of a security/account to a classification
#[derive(Clone, PartialEq, Message)]
pub struct PAssignment {
    /// Type (0=SECURITY, 1=ACCOUNT)
    #[prost(int32, tag = "1")]
    pub vehicle_type: i32,

    /// Index of the vehicle
    #[prost(int32, tag = "2")]
    pub index: i32,

    /// Weight
    #[prost(int32, tag = "3")]
    pub weight: i32,

    /// Rank
    #[prost(int32, tag = "4")]
    pub rank: i32,
}

/// Dashboard configuration
#[derive(Clone, PartialEq, Message)]
pub struct PDashboard {
    /// Name
    #[prost(string, tag = "1")]
    pub name: String,

    /// UUID/ID
    #[prost(string, tag = "2")]
    pub id: String,

    /// Columns
    #[prost(message, repeated, tag = "3")]
    pub columns: Vec<PDashboardColumn>,
}

/// Dashboard column
#[derive(Clone, PartialEq, Message)]
pub struct PDashboardColumn {
    /// Weight (percentage)
    #[prost(int32, tag = "1")]
    pub weight: i32,

    /// Widgets
    #[prost(message, repeated, tag = "2")]
    pub widgets: Vec<PDashboardWidget>,
}

/// Dashboard widget
#[derive(Clone, PartialEq, Message)]
pub struct PDashboardWidget {
    /// Widget type
    #[prost(string, tag = "1")]
    pub widget_type: String,

    /// Label
    #[prost(string, tag = "2")]
    pub label: String,

    /// Configuration (serialized)
    #[prost(bytes = "vec", tag = "3")]
    pub configuration: Vec<u8>,
}

/// Settings (OFFICIAL from client.proto)
#[derive(Clone, PartialEq, Message)]
pub struct PSettings {
    /// Bookmarks
    #[prost(message, repeated, tag = "1")]
    pub bookmarks: Vec<PBookmark>,

    /// Attribute types
    #[prost(message, repeated, tag = "2")]
    pub attribute_types: Vec<PAttributeType>,

    /// Configuration sets
    #[prost(message, repeated, tag = "3")]
    pub configuration_sets: Vec<PConfigurationSet>,
}

/// Configuration set (OFFICIAL from client.proto)
#[derive(Clone, PartialEq, Message)]
pub struct PConfigurationSet {
    /// Key
    #[prost(string, tag = "1")]
    pub key: String,

    /// UUID
    #[prost(string, tag = "2")]
    pub uuid: String,

    /// Name
    #[prost(string, tag = "3")]
    pub name: String,

    /// Data (serialized configuration)
    #[prost(string, tag = "4")]
    pub data: String,
}

/// Bookmark
#[derive(Clone, PartialEq, Message)]
pub struct PBookmark {
    #[prost(string, tag = "1")]
    pub label: String,

    #[prost(string, tag = "2")]
    pub pattern: String,
}

/// Attribute type definition
#[derive(Clone, PartialEq, Message)]
pub struct PAttributeType {
    #[prost(string, tag = "1")]
    pub id: String,

    #[prost(string, tag = "2")]
    pub name: String,

    #[prost(string, tag = "3")]
    pub column_label: String,

    #[prost(string, tag = "4")]
    pub source: String,

    #[prost(string, tag = "5")]
    pub target: String,

    #[prost(string, tag = "6")]
    pub attr_type: String,
}

// ============================================================================
// OFFICIAL PP Transaction Types (from client.proto)
// https://github.com/portfolio-performance/portfolio/blob/master/name.abuchen.portfolio/src/name/abuchen/portfolio/model/client.proto
// ============================================================================
pub mod transaction_type {
    /// PURCHASE - Buy securities (portfolio)
    pub const PURCHASE: i32 = 0;
    /// SALE - Sell securities (portfolio)
    pub const SALE: i32 = 1;
    /// INBOUND_DELIVERY - Securities delivered in (e.g., from broker transfer)
    pub const INBOUND_DELIVERY: i32 = 2;
    /// OUTBOUND_DELIVERY - Securities delivered out
    pub const OUTBOUND_DELIVERY: i32 = 3;
    /// SECURITY_TRANSFER - Transfer between own portfolios
    pub const SECURITY_TRANSFER: i32 = 4;
    /// CASH_TRANSFER - Transfer between own accounts
    pub const CASH_TRANSFER: i32 = 5;
    /// DEPOSIT - Cash deposit to account
    pub const DEPOSIT: i32 = 6;
    /// REMOVAL - Cash removal from account
    pub const REMOVAL: i32 = 7;
    /// DIVIDEND - Dividend payment
    pub const DIVIDEND: i32 = 8;
    /// INTEREST - Interest received
    pub const INTEREST: i32 = 9;
    /// INTEREST_CHARGE - Interest paid (negative)
    pub const INTEREST_CHARGE: i32 = 10;
    /// TAX - Tax payment
    pub const TAX: i32 = 11;
    /// TAX_REFUND - Tax refund
    pub const TAX_REFUND: i32 = 12;
    /// FEE - Fee payment
    pub const FEE: i32 = 13;
    /// FEE_REFUND - Fee refund
    pub const FEE_REFUND: i32 = 14;
}

// ============================================================================
// OFFICIAL PP TransactionUnit Types (from client.proto)
// ============================================================================
pub mod unit_type {
    /// GROSS_VALUE - The gross value of the transaction
    pub const GROSS_VALUE: i32 = 0;
    /// TAX - Tax amount
    pub const TAX: i32 = 1;
    /// FEE - Fee amount
    pub const FEE: i32 = 2;
}

// ============================================================================
// OFFICIAL PP SecurityEvent Types (from client.proto)
// ============================================================================
pub mod event_type {
    /// STOCK_SPLIT - Stock split event
    pub const STOCK_SPLIT: i32 = 0;
    /// NOTE - Note/memo event
    pub const NOTE: i32 = 1;
    /// DIVIDEND_PAYMENT - Dividend payment event
    pub const DIVIDEND_PAYMENT: i32 = 2;
}

// ============================================================================
// OFFICIAL PP InvestmentPlan Types (from client.proto)
// ============================================================================
pub mod investment_plan_type {
    pub const PURCHASE_OR_DELIVERY: i32 = 0;
    pub const DEPOSIT: i32 = 1;
    pub const REMOVAL: i32 = 2;
    pub const INTEREST: i32 = 3;
}
