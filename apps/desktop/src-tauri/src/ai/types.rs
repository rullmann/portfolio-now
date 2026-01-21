//! AI module type definitions
//!
//! This module contains all the type definitions used across the AI module,
//! including request/response types, chart annotations, portfolio context,
//! and error handling types.

use serde::{Deserialize, Serialize};

// ============================================================================
// Request Configuration Constants
// ============================================================================

/// Request timeout in seconds
pub const REQUEST_TIMEOUT_SECS: u64 = 60;

/// Maximum retries for transient errors
pub const MAX_RETRIES: u32 = 2;

/// Base delay for exponential backoff (milliseconds)
pub const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Maximum tokens for response
pub const MAX_TOKENS: u32 = 1500;

/// Maximum tokens for portfolio insights (longer response needed)
pub const MAX_TOKENS_INSIGHTS: u32 = 2000;

/// Maximum tokens for chat responses
pub const MAX_TOKENS_CHAT: u32 = 1000;

// ============================================================================
// Structured AI Errors
// ============================================================================

/// Types of AI API errors
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AiErrorKind {
    /// Rate limit exceeded - too many requests, retry after delay
    RateLimit,
    /// Quota/credits exhausted - need to upgrade plan or switch provider
    QuotaExceeded,
    /// Invalid or expired API key
    InvalidApiKey,
    /// Model not found or not available
    ModelNotFound,
    /// Server error on provider side
    ServerError,
    /// Network/connection error
    NetworkError,
    /// Other/unknown error
    Other,
}

/// Structured AI error with details
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub kind: AiErrorKind,
    pub message: String,
    pub provider: String,
    pub model: String,
    /// Suggested retry delay in seconds (for rate limit errors)
    pub retry_after_secs: Option<u32>,
    /// Suggested fallback model (for quota/model errors)
    pub fallback_model: Option<String>,
}

impl AiError {
    pub fn rate_limit(provider: &str, model: &str, retry_after: Option<u32>) -> Self {
        Self {
            kind: AiErrorKind::RateLimit,
            message: "Zu viele Anfragen. Bitte warte einen Moment.".to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: retry_after,
            fallback_model: None,
        }
    }

    pub fn quota_exceeded(provider: &str, model: &str, fallback: Option<&str>) -> Self {
        Self {
            kind: AiErrorKind::QuotaExceeded,
            message: "Kontingent erschöpft. Bitte wechsle das Modell oder den Anbieter.".to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: fallback.map(String::from),
        }
    }

    pub fn invalid_api_key(provider: &str, model: &str) -> Self {
        Self {
            kind: AiErrorKind::InvalidApiKey,
            message: "Ungültiger API Key. Bitte überprüfe deine Einstellungen.".to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: None,
        }
    }

    pub fn model_not_found(provider: &str, model: &str, fallback: Option<&str>) -> Self {
        Self {
            kind: AiErrorKind::ModelNotFound,
            message: format!("Modell '{}' nicht verfügbar.", model),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: fallback.map(String::from),
        }
    }

    pub fn server_error(provider: &str, model: &str, details: &str) -> Self {
        Self {
            kind: AiErrorKind::ServerError,
            message: format!("Server-Fehler bei {}: {}", provider, details),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: Some(5),
            fallback_model: None,
        }
    }

    pub fn network_error(provider: &str, model: &str, details: &str) -> Self {
        Self {
            kind: AiErrorKind::NetworkError,
            message: format!("Netzwerkfehler: {}", details),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: Some(3),
            fallback_model: None,
        }
    }

    pub fn other(provider: &str, model: &str, message: &str) -> Self {
        Self {
            kind: AiErrorKind::Other,
            message: message.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: None,
        }
    }
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AiError {}

// ============================================================================
// Chart Analysis Types
// ============================================================================

/// Request for chart analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartAnalysisRequest {
    pub image_base64: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub context: ChartContext,
}

/// Context about the chart being analyzed
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartContext {
    pub security_name: String,
    pub ticker: Option<String>,
    pub currency: String,
    pub current_price: f64,
    pub timeframe: String,
    pub indicators: Vec<String>,
}

/// Response from AI analysis
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartAnalysisResponse {
    pub analysis: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

// ============================================================================
// Enhanced Chart Analysis Types
// ============================================================================

/// A single indicator reading with current value and signal
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorValue {
    pub name: String,                    // e.g., "RSI"
    pub params: String,                  // e.g., "14"
    pub current_value: f64,              // e.g., 72.5
    pub previous_value: Option<f64>,     // For trend detection
    pub signal: Option<String>,          // "overbought", "oversold", "bullish_crossover", etc.
}

/// OHLC candlestick data for a single period
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandleData {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<i64>,
}

/// Volume analysis context
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeAnalysis {
    pub current_volume: i64,
    pub avg_volume_20d: f64,
    pub volume_ratio: f64,               // current / avg
    pub volume_trend: String,            // "increasing", "decreasing", "stable"
}

/// Enhanced chart context with indicator values, OHLC data, and volume analysis
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedChartContext {
    // Basic info
    pub security_name: String,
    pub ticker: Option<String>,
    pub currency: String,
    pub current_price: f64,
    pub timeframe: String,

    // Enhanced indicators with actual values
    pub indicator_values: Vec<IndicatorValue>,

    // OHLC data (last N candles)
    pub candles: Option<Vec<CandleData>>,

    // Volume analysis
    pub volume_analysis: Option<VolumeAnalysis>,

    // Price statistics
    pub price_change_percent: Option<f64>,
    pub high_52_week: Option<f64>,
    pub low_52_week: Option<f64>,
    pub distance_from_high_percent: Option<f64>,

    // Web context (news, earnings, analyst ratings)
    // When true, AI should search for recent news and incorporate into analysis
    #[serde(default)]
    pub include_web_context: bool,
}

/// AI-suggested price alert
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertSuggestion {
    pub price: f64,
    pub condition: String,               // "above", "below", "crosses_up", "crosses_down"
    pub reason: String,
    pub priority: String,                // "high", "medium", "low"
}

/// Risk/Reward analysis from AI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskRewardAnalysis {
    #[serde(default)]
    pub entry_price: Option<f64>,
    #[serde(default)]
    pub stop_loss: Option<f64>,
    #[serde(default)]
    pub take_profit: Option<f64>,
    #[serde(default)]
    pub risk_reward_ratio: Option<f64>,
    #[serde(default)]
    pub rationale: Option<String>,
}

/// Request for enhanced chart analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedChartAnalysisRequest {
    pub image_base64: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub context: EnhancedChartContext,
}

/// Extended annotation response with alerts and risk/reward (JSON parsing intermediate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAnnotationAnalysisJson {
    pub analysis: String,
    pub trend: TrendInfo,
    pub annotations: Vec<ChartAnnotation>,
    pub alerts: Vec<AlertSuggestion>,
    pub risk_reward: Option<RiskRewardAnalysis>,
}

/// Response from enhanced AI analysis with annotations
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedAnnotationAnalysisResponse {
    pub analysis: String,
    pub trend: TrendInfo,
    pub annotations: Vec<ChartAnnotation>,
    pub alerts: Vec<AlertSuggestion>,
    pub risk_reward: Option<RiskRewardAnalysis>,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

// ============================================================================
// Chart Annotations (Structured AI Output)
// ============================================================================

/// Type of chart annotation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationType {
    Support,
    Resistance,
    Trendline,
    Pattern,
    Signal,
    Target,
    Stoploss,
    Note,
}

/// Signal direction for annotations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SignalDirection {
    Bullish,
    Bearish,
    Neutral,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Bullish,
    Bearish,
    Neutral,
}

/// Trend strength
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TrendStrength {
    Strong,
    Moderate,
    Weak,
}

/// Trend information from AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendInfo {
    pub direction: TrendDirection,
    pub strength: TrendStrength,
}

/// A single chart annotation from AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartAnnotation {
    #[serde(rename = "type")]
    pub annotation_type: AnnotationType,
    pub price: f64,
    pub time: Option<String>,
    pub time_end: Option<String>,
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub signal: Option<SignalDirection>,
}

/// Structured response from AI with annotations (JSON parsing intermediate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationAnalysisJson {
    pub analysis: String,
    pub trend: TrendInfo,
    pub annotations: Vec<ChartAnnotation>,
}

/// Response from AI analysis with annotations
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationAnalysisResponse {
    pub analysis: String,
    pub trend: TrendInfo,
    pub annotations: Vec<ChartAnnotation>,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

// ============================================================================
// Portfolio Insights Types
// ============================================================================

/// Summary of a single holding for AI context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldingSummary {
    pub name: String,
    pub isin: Option<String>,
    pub ticker: Option<String>,
    pub shares: f64,
    pub current_value: f64,
    pub current_price: Option<f64>,
    pub cost_basis: f64,
    pub weight_percent: f64,
    pub gain_loss_percent: Option<f64>,
    pub currency: String,
    /// Average cost per share (Einstandskurs)
    pub avg_cost_per_share: Option<f64>,
    /// First purchase date
    pub first_buy_date: Option<String>,
    /// Total fees paid for this position
    pub total_fees: f64,
    /// Total taxes paid for this position
    pub total_taxes: f64,
}

/// Recent transaction for context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTransaction {
    pub date: String,
    pub txn_type: String,
    pub security_name: Option<String>,
    pub shares: Option<f64>,
    pub amount: f64,
    pub currency: String,
}

/// Dividend payment info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendPayment {
    pub date: String,
    pub security_name: String,
    pub gross_amount: f64,
    pub net_amount: f64,
    pub currency: String,
}

/// Watchlist item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistItem {
    pub name: String,
    pub isin: Option<String>,
    pub ticker: Option<String>,
    pub current_price: Option<f64>,
    pub currency: String,
}

/// Sold/closed position (no longer held but has historical transactions)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoldPosition {
    pub name: String,
    pub ticker: Option<String>,
    pub isin: Option<String>,
    pub total_bought_shares: f64,
    pub total_sold_shares: f64,
    pub realized_gain_loss: f64,
    pub last_transaction_date: String,
}

/// Yearly overview with realized gains and dividends
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YearlyOverview {
    pub year: i32,
    pub realized_gains: f64,
    pub dividends: f64,
    pub transaction_count: i32,
}

/// Summary of fees and taxes paid
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeesAndTaxesSummary {
    pub total_fees: f64,
    pub total_taxes: f64,
    pub fees_this_year: f64,
    pub taxes_this_year: f64,
    pub by_year: Vec<YearlyFeesAndTaxes>,
}

/// Fees and taxes for a specific year
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YearlyFeesAndTaxes {
    pub year: i32,
    pub fees: f64,
    pub taxes: f64,
}

/// Investment summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvestmentSummary {
    /// Total amount invested (sum of all buys)
    pub total_invested: f64,
    /// Total amount withdrawn (sum of all sells)
    pub total_withdrawn: f64,
    /// Net invested (invested - withdrawn)
    pub net_invested: f64,
    /// Total deposits to accounts
    pub total_deposits: f64,
    /// Total removals from accounts
    pub total_removals: f64,
    /// First investment date
    pub first_investment_date: Option<String>,
}

/// Sector/Taxonomy allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorAllocation {
    pub taxonomy_name: String,
    pub allocations: Vec<(String, f64)>, // (category name, percentage)
}

/// Portfolio historical extremes (high/low)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioExtremes {
    pub all_time_high: f64,
    pub all_time_high_date: String,
    pub all_time_low: f64,
    pub all_time_low_date: String,
    pub year_high: f64,
    pub year_high_date: String,
    pub year_low: f64,
    pub year_low_date: String,
}

/// Summary of quote provider status for AI context
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteProviderStatusSummary {
    /// Securities that can sync prices
    pub can_sync_count: usize,
    /// Securities that cannot sync (missing API key or no provider)
    pub cannot_sync_count: usize,
    /// List of providers that need API keys but don't have them configured
    pub missing_api_keys: Vec<String>,
    /// Securities that cannot sync with reasons
    pub issues: Vec<String>,
    /// Quote sync status
    pub quote_sync: QuoteSyncInfo,
}

/// Info about quote synchronization status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteSyncInfo {
    /// Total held securities
    pub held_count: usize,
    /// Securities with quotes from today
    pub synced_today_count: usize,
    /// Securities with outdated or no quotes
    pub outdated_count: usize,
    /// Today's date
    pub today: String,
    /// Securities with outdated quotes (name, days old)
    pub outdated: Vec<String>,
}

/// Portfolio context for AI analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioInsightsContext {
    // Holdings (ALL of them)
    pub holdings: Vec<HoldingSummary>,
    pub total_value: f64,
    pub total_cost_basis: f64,
    pub total_gain_loss_percent: f64,

    // Performance
    pub ttwror: Option<f64>,
    pub ttwror_annualized: Option<f64>,
    pub irr: Option<f64>,

    // Diversification
    pub currency_allocation: Vec<(String, f64)>,
    pub top_positions: Vec<(String, f64)>,

    // Dividends
    pub dividend_yield: Option<f64>,
    pub annual_dividends: f64,
    pub recent_dividends: Vec<DividendPayment>,

    // Recent transactions (last 20)
    pub recent_transactions: Vec<RecentTransaction>,

    // Watchlist
    pub watchlist: Vec<WatchlistItem>,

    // Historical data
    pub sold_positions: Vec<SoldPosition>,
    pub yearly_overview: Vec<YearlyOverview>,

    // Period
    pub portfolio_age_days: u32,
    pub analysis_date: String,
    pub base_currency: String,

    // User profile
    pub user_name: Option<String>,

    // Quote provider status (for AI to know about sync issues)
    pub provider_status: Option<QuoteProviderStatusSummary>,

    // Fees & Taxes
    pub fees_and_taxes: FeesAndTaxesSummary,

    // Investment summary
    pub investment_summary: InvestmentSummary,

    // Taxonomy/Sector allocation
    pub sector_allocation: Vec<SectorAllocation>,

    // Portfolio historical extremes
    pub portfolio_extremes: Option<PortfolioExtremes>,
}

/// Response from portfolio insights AI analysis
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioInsightsResponse {
    pub analysis: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

// ============================================================================
// Chat Types
// ============================================================================

/// A single chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Suggested action from AI that requires user confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSuggestedAction {
    /// Type of action: "watchlist_add", "watchlist_remove"
    pub action_type: String,
    /// Human-readable description of the action
    pub description: String,
    /// Serialized payload for execution
    pub payload: String,
}

/// Response from portfolio chat
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioChatResponse {
    pub response: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
    /// Suggested actions that require user confirmation (watchlist modifications)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggestions: Vec<ChatSuggestedAction>,
}

// ============================================================================
// Transaction Create Command Types
// ============================================================================

/// Transaction create command parsed from AI response.
/// SECURITY: This is returned as a SUGGESTION that requires user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionCreateCommand {
    /// Always true - transactions require preview/confirmation
    #[serde(default = "default_true")]
    pub preview: bool,
    /// Transaction type: BUY, SELL, DELIVERY_INBOUND, DELIVERY_OUTBOUND, DIVIDENDS, DEPOSIT, REMOVAL, etc.
    #[serde(rename = "type")]
    pub txn_type: String,
    /// Portfolio ID (for portfolio transactions)
    pub portfolio_id: Option<i64>,
    /// Account ID (for account transactions like DIVIDENDS, DEPOSIT, REMOVAL)
    pub account_id: Option<i64>,
    /// Security ID (required for BUY/SELL/DELIVERY/DIVIDENDS)
    pub security_id: Option<i64>,
    /// Security name for display
    pub security_name: Option<String>,
    /// Number of shares × 10^8 (e.g., 10 shares = 1_000_000_000)
    pub shares: Option<i64>,
    /// Amount in cents × 10^2 (e.g., 180.00 EUR = 18000)
    pub amount: Option<i64>,
    /// Currency code (EUR, USD, etc.)
    #[serde(default = "default_eur")]
    pub currency: String,
    /// Transaction date in ISO format (YYYY-MM-DD)
    pub date: String,
    /// Fees in cents × 10^2
    #[serde(default)]
    pub fees: Option<i64>,
    /// Taxes in cents × 10^2
    #[serde(default)]
    pub taxes: Option<i64>,
    /// Optional note
    pub note: Option<String>,
    /// For transfers: other portfolio ID
    pub other_portfolio_id: Option<i64>,
    /// For transfers: other account ID
    pub other_account_id: Option<i64>,
}

fn default_true() -> bool {
    true
}

fn default_eur() -> String {
    "EUR".to_string()
}

/// Portfolio transfer command (Depotwechsel) parsed from AI response.
/// Creates paired DELIVERY_OUTBOUND and DELIVERY_INBOUND transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioTransferCommand {
    /// Security ID to transfer
    pub security_id: i64,
    /// Number of shares × 10^8
    pub shares: i64,
    /// Transfer date in ISO format
    pub date: String,
    /// Source portfolio ID
    pub from_portfolio_id: i64,
    /// Target portfolio ID
    pub to_portfolio_id: i64,
    /// Optional note
    pub note: Option<String>,
}

/// Result of transaction validation before execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    /// Resolved names for display
    pub portfolio_name: Option<String>,
    pub account_name: Option<String>,
    pub security_name: Option<String>,
    /// Current holdings for SELL validation
    pub current_holdings: Option<f64>,
}

// ============================================================================
// Model Listing API Types
// ============================================================================

/// Available AI model info
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub supports_vision: bool,
}
