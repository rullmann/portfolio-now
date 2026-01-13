//! AI-powered chart analysis module.
//!
//! Supports multiple providers: Claude (Anthropic), GPT-5 (OpenAI), Gemini (Google), Perplexity (Sonar)

pub mod claude;
pub mod gemini;
pub mod models;
pub mod openai;
pub mod perplexity;

// Re-export model registry functions
pub use models::{
    get_default, get_fallback, get_model, get_model_upgrade, get_models_for_provider,
    is_valid_model, ModelInfo, VisionModel, DEPRECATED_MODELS, VISION_MODELS,
};

use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

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

/// Parse retry delay from error response (supports "4s", "4.5s", seconds as number)
pub fn parse_retry_delay(text: &str) -> Option<u32> {
    // Try to find "retryDelay": "Xs" pattern
    if let Some(idx) = text.find("retryDelay") {
        let after = &text[idx..];
        // Look for number followed by 's'
        for word in after.split_whitespace().take(5) {
            let clean = word.trim_matches(|c: char| !c.is_numeric() && c != '.');
            if let Ok(secs) = clean.parse::<f64>() {
                return Some(secs.ceil() as u32);
            }
        }
    }
    // Try to find "retry in X" pattern
    if let Some(idx) = text.find("retry in") {
        let after = &text[idx + 8..];
        for word in after.split_whitespace().take(3) {
            let clean = word.trim_matches(|c: char| !c.is_numeric() && c != '.');
            if let Ok(secs) = clean.parse::<f64>() {
                return Some(secs.ceil() as u32);
            }
        }
    }
    None
}

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

/// Extended annotation response with alerts and risk/reward
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

/// Structured response from AI with annotations
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

/// Response from portfolio chat
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioChatResponse {
    pub response: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

// ============================================================================
// Prompt Templates (Tiered by Model Capability)
// ============================================================================

/// Determine if a model is a "fast" tier (haiku, mini, flash, sonar base)
pub fn is_fast_model(model: &str) -> bool {
    model.contains("haiku") ||
    model.contains("mini") ||
    model.contains("flash") ||
    // Perplexity: base sonar is fast, pro/reasoning are not
    (model == "sonar" || model.ends_with("sonar"))
}

/// Build the analysis prompt with chart context.
/// Uses a shorter prompt for fast/cheap models to reduce token usage.
pub fn build_analysis_prompt(ctx: &ChartContext, model: &str) -> String {
    let indicators_str = if ctx.indicators.is_empty() {
        "Keine".to_string()
    } else {
        ctx.indicators.join(", ")
    };

    if is_fast_model(model) {
        // Compact prompt for fast/cheap models (~40% fewer tokens)
        format!(
            r#"Technische Chart-Analyse für {} ({}).
Kurs: {:.2} {} | Zeitraum: {} | Indikatoren: {}

WICHTIG: Verwende EXAKT dieses Markdown-Format mit ## für Überschriften:

## Trend
[Aufwärts/Abwärts/Seitwärts + Stärke]

## Support/Widerstand
**S:** [Levels] | **R:** [Levels]

## Muster
[Formation oder "Keine"]

## Signal
[Bullisch/Bärisch/Neutral] - [Begründung]

## Risiko
[1 Hauptrisiko]"#,
            ctx.security_name,
            ctx.ticker.as_deref().unwrap_or("-"),
            ctx.current_price,
            ctx.currency,
            ctx.timeframe,
            indicators_str
        )
    } else {
        // Full prompt for pro/standard models
        format!(
            r#"Du bist ein erfahrener technischer Analyst. Analysiere den beigefügten Chart.

**Wertpapier:** {} ({})
**Zeitraum:** {}
**Aktueller Kurs:** {:.2} {}
**Aktive Indikatoren:** {}

WICHTIG: Antworte in Markdown-Format mit Überschriften im Format: ## Überschrift

## Trend
[1-2 Sätze: Primärer Trend (Aufwärts/Abwärts/Seitwärts), Trendstärke]

## Unterstützung & Widerstand
- **Unterstützung:** [Preisniveau(s)]
- **Widerstand:** [Preisniveau(s)]

## Chartmuster
[1-2 Sätze: Erkennbare Formationen oder Keine eindeutigen Muster erkennbar]

## Indikatoren
[1-2 Sätze zur Interpretation der aktiven Indikatoren, oder Keine Indikatoren aktiv]

## Einschätzung
- **Kurzfristig:** [Bullisch/Bärisch/Neutral] - [1 Satz Begründung]
- **Mittelfristig:** [Bullisch/Bärisch/Neutral] - [1 Satz Begründung]

## Risiken
[1-2 konkrete Risikofaktoren]

Beginne direkt mit der Trend-Überschrift. Keine Einleitung, keine zusätzlichen Abschnitte."#,
            ctx.security_name,
            ctx.ticker.as_deref().unwrap_or("-"),
            ctx.timeframe,
            ctx.current_price,
            ctx.currency,
            indicators_str
        )
    }
}

/// Build a prompt that requests structured JSON output with chart annotations.
/// The AI will return support/resistance levels, patterns, and signals as JSON.
pub fn build_annotation_prompt(ctx: &ChartContext) -> String {
    let indicators_str = if ctx.indicators.is_empty() {
        "Keine".to_string()
    } else {
        ctx.indicators.join(", ")
    };

    format!(
        r##"Du bist ein erfahrener technischer Analyst. Analysiere den Chart und gib strukturierte Annotations zurück.

**Wertpapier:** {} ({})
**Zeitraum:** {}
**Aktueller Kurs:** {:.2} {}
**Aktive Indikatoren:** {}

Antworte AUSSCHLIESSLICH mit validem JSON (keine Markdown-Formatierung, kein Text davor oder danach) in diesem Format:
{{
  "analysis": "2-3 Sätze Gesamteinschätzung des Charts",
  "trend": {{
    "direction": "bullish" oder "bearish" oder "neutral",
    "strength": "strong" oder "moderate" oder "weak"
  }},
  "annotations": [
    {{
      "type": "support" oder "resistance" oder "pattern" oder "signal" oder "target" oder "stoploss" oder "note",
      "price": 123.45,
      "time": "2024-01-15" oder null,
      "time_end": null,
      "title": "Kurzer Titel (max 20 Zeichen)",
      "description": "Ausführliche Erklärung warum dieses Level wichtig ist",
      "confidence": 0.85,
      "signal": "bullish" oder "bearish" oder "neutral" oder null
    }}
  ]
}}

WICHTIGE REGELN:
1. Identifiziere 2-5 relevante Annotations (Support, Resistance, Patterns, Signale)
2. Preise müssen exakt aus dem Chart abgelesen werden - schätze realistische Werte
3. Für Support/Resistance: time ist null (horizontale Linien)
4. Für Patterns/Signale: time ist das Datum wo das Pattern auftritt
5. Confidence: 0.5 (unsicher) bis 1.0 (sehr sicher)
6. Signal: Bei Support="bullish", bei Resistance="bearish", bei neutralen Zonen="neutral"
7. Gib NUR valides JSON zurück, keine Erklärungen außerhalb des JSON"##,
        ctx.security_name,
        ctx.ticker.as_deref().unwrap_or("N/A"),
        ctx.timeframe,
        ctx.current_price,
        ctx.currency,
        indicators_str
    )
}

/// Parse JSON response from AI into structured annotations.
/// Handles common AI quirks like markdown code blocks around JSON.
pub fn parse_annotation_response(raw: &str) -> Result<AnnotationAnalysisJson> {
    // Remove markdown code blocks if present
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str(cleaned)
        .map_err(|e| anyhow!("Failed to parse AI JSON response: {}. Raw: {}", e, &raw[..raw.len().min(200)]))
}

/// Build enhanced annotation prompt with indicator values, OHLC data, volume analysis,
/// and requests for alerts and risk/reward analysis.
pub fn build_enhanced_annotation_prompt(ctx: &EnhancedChartContext) -> String {
    // Format indicator values with signals
    let indicators_str = if ctx.indicator_values.is_empty() {
        "Keine aktiven Indikatoren".to_string()
    } else {
        ctx.indicator_values
            .iter()
            .map(|i| {
                let signal_str = i.signal.as_ref()
                    .map(|s| format!(" [{}]", s))
                    .unwrap_or_default();
                let prev_str = i.previous_value
                    .map(|p| format!(" (vorher: {:.2})", p))
                    .unwrap_or_default();
                format!("- {}({}): {:.2}{}{}", i.name, i.params, i.current_value, signal_str, prev_str)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format volume analysis
    let volume_str = ctx.volume_analysis.as_ref()
        .map(|v| format!(
            "Aktuelles Volumen: {} | 20-Tage-Ø: {:.0} | Ratio: {:.2}x | Trend: {}",
            v.current_volume, v.avg_volume_20d, v.volume_ratio, v.volume_trend
        ))
        .unwrap_or_else(|| "Keine Volumendaten verfügbar".to_string());

    // Format price statistics
    let price_stats = format!(
        "Aktueller Kurs: {:.2} {} | Veränderung: {:+.2}%",
        ctx.current_price,
        ctx.currency,
        ctx.price_change_percent.unwrap_or(0.0)
    );

    let high_low_str = match (ctx.high_52_week, ctx.low_52_week) {
        (Some(high), Some(low)) => {
            let dist = ctx.distance_from_high_percent.unwrap_or(0.0);
            format!("52W-Hoch: {:.2} | 52W-Tief: {:.2} | Abstand vom Hoch: {:.1}%", high, low, dist)
        }
        _ => String::new(),
    };

    // Format candles summary
    let candles_summary = ctx.candles.as_ref()
        .map(|candles| {
            if candles.is_empty() {
                return "Keine Kerzendaten".to_string();
            }
            let last_10: Vec<_> = candles.iter().rev().take(10).collect();
            let bullish_count = last_10.iter().filter(|c| c.close > c.open).count();
            let bearish_count = last_10.len() - bullish_count;
            let avg_range: f64 = if !last_10.is_empty() {
                last_10.iter()
                    .map(|c| if c.close > 0.0 { (c.high - c.low) / c.close * 100.0 } else { 0.0 })
                    .sum::<f64>() / last_10.len() as f64
            } else {
                0.0
            };
            format!(
                "Letzte 10 Kerzen: {} bullish, {} bearish | Ø-Range: {:.2}%",
                bullish_count, bearish_count, avg_range
            )
        })
        .unwrap_or_else(|| "Keine Kerzendaten".to_string());

    // Format last 5 candles as table for precise data
    let candles_table = ctx.candles.as_ref()
        .map(|candles| {
            let last_5: Vec<_> = candles.iter().rev().take(5).rev().collect();
            if last_5.is_empty() {
                return String::new();
            }
            let rows: Vec<String> = last_5.iter()
                .map(|c| {
                    let vol_str = c.volume.map(|v| format!("{}", v)).unwrap_or_else(|| "-".to_string());
                    format!("{}: O={:.2} H={:.2} L={:.2} C={:.2} V={}", c.date, c.open, c.high, c.low, c.close, vol_str)
                })
                .collect();
            format!("\n**Letzte 5 Kerzen (OHLCV):**\n{}", rows.join("\n"))
        })
        .unwrap_or_default();

    // Build web context instructions if enabled
    let web_context_str = if ctx.include_web_context {
        format!(
            r##"

=== WEB-RECHERCHE (AKTIV) ===
Recherchiere im Web nach aktuellen Informationen zu {} und integriere sie in deine Analyse:
1. **Aktuelle Nachrichten**: Suche nach relevanten News der letzten 7 Tage
2. **Earnings-Termine**: Prüfe bevorstehende oder kürzliche Quartalsberichte
3. **Analysteneinschätzungen**: Aktuelle Ratings und Kursziele
4. **Sektor-Entwicklung**: Relevante Branchennews

Füge einen "news_summary" Abschnitt zur Analyse hinzu mit den wichtigsten Erkenntnissen."##,
            ctx.security_name
        )
    } else {
        String::new()
    };

    format!(
        r##"Du bist ein erfahrener technischer Analyst. Analysiere den Chart und gib strukturierte Annotations zurück.{}

**Wertpapier:** {} ({})
**Zeitraum:** {}
{}
{}

**TECHNISCHE INDIKATOREN (BERECHNETE WERTE):**
{}

**VOLUMEN-ANALYSE:**
{}

**KERZEN-STATISTIK:**
{}{}

WICHTIG: Die Indikatorwerte oben sind BERECHNET - nutze sie für präzise Analyse!
- RSI > 70 = überkauft, RSI < 30 = überverkauft
- MACD Histogramm > 0 = bullisches Momentum
- Volumen-Ratio > 1.5 = erhöhtes Interesse, < 0.5 = geringes Interesse

Antworte AUSSCHLIESSLICH mit validem JSON (keine Markdown-Formatierung, kein Text davor oder danach):
{{
  "analysis": "2-3 Sätze Gesamteinschätzung mit Bezug auf die konkreten Indikatorwerte",
  "trend": {{
    "direction": "bullish" | "bearish" | "neutral",
    "strength": "strong" | "moderate" | "weak"
  }},
  "annotations": [
    {{
      "type": "support" | "resistance" | "pattern" | "signal" | "target" | "stoploss",
      "price": 123.45,
      "time": "2024-01-15" | null,
      "time_end": null,
      "title": "Kurzer Titel",
      "description": "Ausführliche Erklärung",
      "confidence": 0.85,
      "signal": "bullish" | "bearish" | "neutral" | null
    }}
  ],
  "alerts": [
    {{
      "price": 150.00,
      "condition": "above" | "below" | "crosses_up" | "crosses_down",
      "reason": "Wichtiger Widerstand - Ausbruch wäre bullisch",
      "priority": "high" | "medium" | "low"
    }}
  ],
  "risk_reward": {{
    "entry_price": 145.50,
    "stop_loss": 140.00,
    "take_profit": 160.00,
    "risk_reward_ratio": 2.64,
    "rationale": "Entry bei Support, SL unter letztem Tief, TP bei Widerstand"
  }} | null
}}

WICHTIGE REGELN:
1. Identifiziere 2-5 relevante Annotations basierend auf Chart UND Indikatoren
2. Schlage 1-3 sinnvolle Preisalarme vor (z.B. bei Support/Resistance-Durchbruch)
3. Berechne ein Risk/Reward-Setup wenn ein klares Setup erkennbar ist (sonst null)
4. Preise müssen exakt aus dem Chart abgelesen werden
5. Confidence: 0.5 (unsicher) bis 1.0 (sehr sicher)
6. Gib NUR valides JSON zurück"##,
        web_context_str,
        ctx.security_name,
        ctx.ticker.as_deref().unwrap_or("N/A"),
        ctx.timeframe,
        price_stats,
        high_low_str,
        indicators_str,
        volume_str,
        candles_summary,
        candles_table
    )
}

/// Parse enhanced JSON response from AI into structured annotations with alerts and risk/reward.
pub fn parse_enhanced_annotation_response(raw: &str) -> Result<EnhancedAnnotationAnalysisJson> {
    // Remove markdown code blocks if present
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str(cleaned)
        .map_err(|e| anyhow!("Failed to parse enhanced AI JSON response: {}. Raw: {}", e, &raw[..raw.len().min(200)]))
}

/// Calculate exponential backoff delay
pub fn calculate_backoff_delay(attempt: u32) -> std::time::Duration {
    let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
    std::time::Duration::from_millis(delay_ms.min(10_000)) // Max 10 seconds
}

// ============================================================================
// Portfolio Insights Prompt
// ============================================================================

/// Build the portfolio insights prompt for AI analysis
pub fn build_portfolio_insights_prompt(ctx: &PortfolioInsightsContext) -> String {
    // Format top positions
    let top_positions_str = ctx
        .top_positions
        .iter()
        .take(5)
        .map(|(name, weight)| format!("- {} ({:.1}%)", name, weight))
        .collect::<Vec<_>>()
        .join("\n");

    // Format currency allocation
    let currency_str = ctx
        .currency_allocation
        .iter()
        .map(|(currency, weight)| format!("- {}: {:.1}%", currency, weight))
        .collect::<Vec<_>>()
        .join("\n");

    // Format holdings summary (top 10 for context)
    let holdings_str = ctx
        .holdings
        .iter()
        .take(10)
        .map(|h| {
            let gl_str = h
                .gain_loss_percent
                .map(|g| format!("{:+.1}%", g))
                .unwrap_or_else(|| "-".to_string());
            format!(
                "- {} | {:.2} {} | {:.1}% | G/V: {}",
                h.name, h.current_value, ctx.base_currency, h.weight_percent, gl_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Performance info
    let perf_str = if let Some(ttwror) = ctx.ttwror {
        let ann_str = ctx
            .ttwror_annualized
            .map(|a| format!(" (p.a. {:.1}%)", a))
            .unwrap_or_default();
        format!("TTWROR: {:.1}%{}", ttwror, ann_str)
    } else {
        "Keine Performance-Daten".to_string()
    };

    let irr_str = ctx
        .irr
        .map(|i| format!("- IRR: {:.1}%", i))
        .unwrap_or_default();

    format!(
        r#"Du bist ein erfahrener Finanzberater. Analysiere dieses Portfolio und gib eine Einschätzung.

**Portfolio-Übersicht** (Stand: {})
- Gesamtwert: {:.2} {}
- Einstandswert: {:.2} {}
- Gesamtrendite: {:+.1}%
- {}
{}

**Top-Positionen:**
{}

**Holdings (Top 10 von {}):**
{}

**Währungsverteilung:**
{}

**Dividenden:**
- Jährliche Dividenden: {:.2} {}
{}

**Anlagehorizont:** {} Tage

Antworte in Markdown mit diesen Abschnitten:

## Zusammenfassung
[2-3 Sätze Gesamtbewertung des Portfolios]

## Stärken
[2-3 konkrete Stärken mit Zahlen]

## Risiken
[2-3 konkrete Risiken/Schwächen mit Zahlen, z.B. Klumpenrisiko, Währungsrisiko]

## Empfehlungen
[2-3 konkrete, umsetzbare Vorschläge zur Optimierung]

WICHTIG:
- Sei direkt und konkret. Keine allgemeinen Floskeln.
- Beziehe dich auf die konkreten Zahlen im Portfolio.
- Gib KEINE Kaufempfehlungen für einzelne Aktien.
- Beginne direkt mit ## Zusammenfassung"#,
        ctx.analysis_date,
        ctx.total_value,
        ctx.base_currency,
        ctx.total_cost_basis,
        ctx.base_currency,
        ctx.total_gain_loss_percent,
        perf_str,
        irr_str,
        top_positions_str,
        ctx.holdings.len(),
        holdings_str,
        currency_str,
        ctx.annual_dividends,
        ctx.base_currency,
        ctx.dividend_yield
            .map(|y| format!("- Dividendenrendite: {:.2}%", y))
            .unwrap_or_default(),
        ctx.portfolio_age_days,
    )
}

// ============================================================================
// Chat System Prompt
// ============================================================================

/// Build the system prompt for portfolio chat
pub fn build_chat_system_prompt(ctx: &PortfolioInsightsContext) -> String {
    // Format ALL holdings for context (with extended details)
    let holdings_str = ctx
        .holdings
        .iter()
        .map(|h| {
            let gl_str = h
                .gain_loss_percent
                .map(|g| format!("{:+.1}%", g))
                .unwrap_or_else(|| "-".to_string());
            let ticker_str = h.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
            let price_str = h.current_price.map(|p| format!(", Kurs: {:.2}", p)).unwrap_or_default();
            let avg_cost_str = h.avg_cost_per_share.map(|a| format!(", Ø-Kurs: {:.2}", a)).unwrap_or_default();
            let first_buy_str = h.first_buy_date.as_ref().map(|d| format!(", Erstkauf: {}", d)).unwrap_or_default();
            format!(
                "- {}{}: {:.4} Stk., Wert: {:.2} {} ({:.1}%), Einstand: {:.2} {}, G/V: {}{}{}{}",
                h.name, ticker_str, h.shares, h.current_value, ctx.base_currency,
                h.weight_percent, h.cost_basis, ctx.base_currency, gl_str, price_str, avg_cost_str, first_buy_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Format recent transactions
    let txn_str = if ctx.recent_transactions.is_empty() {
        "Keine aktuellen Transaktionen".to_string()
    } else {
        ctx.recent_transactions
            .iter()
            .take(20)
            .map(|t| {
                let sec_str = t.security_name.as_ref().map(|s| format!(" - {}", s)).unwrap_or_default();
                let shares_str = t.shares.map(|s| format!(", {:.4} Stk.", s)).unwrap_or_default();
                format!("- {}: {}{}, {:.2} {}{}", t.date, t.txn_type, sec_str, t.amount, t.currency, shares_str)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format recent dividends
    let div_str = if ctx.recent_dividends.is_empty() {
        "Keine Dividenden im letzten Jahr".to_string()
    } else {
        ctx.recent_dividends
            .iter()
            .take(15)
            .map(|d| {
                format!("- {}: {} - Brutto: {:.2} {}, Netto: {:.2} {}",
                    d.date, d.security_name, d.gross_amount, d.currency, d.net_amount, d.currency)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format watchlist
    let watchlist_str = if ctx.watchlist.is_empty() {
        "Keine Watchlist-Einträge".to_string()
    } else {
        ctx.watchlist
            .iter()
            .map(|w| {
                let ticker_str = w.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                let price_str = w.current_price.map(|p| format!(", Kurs: {:.2} {}", p, w.currency)).unwrap_or_default();
                format!("- {}{}{}", w.name, ticker_str, price_str)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format sold positions (historical holdings that are now fully sold)
    let sold_positions_str = if ctx.sold_positions.is_empty() {
        "Keine verkauften Positionen".to_string()
    } else {
        ctx.sold_positions
            .iter()
            .map(|s| {
                let ticker_str = s.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                let gain_str = if s.realized_gain_loss >= 0.0 {
                    format!("+{:.2}", s.realized_gain_loss)
                } else {
                    format!("{:.2}", s.realized_gain_loss)
                };
                format!(
                    "- {}{}: Gekauft: {:.4} Stk., Verkauft: {:.4} Stk., Realisiert: {} {}, Letzte Txn: {}",
                    s.name, ticker_str, s.total_bought_shares, s.total_sold_shares,
                    gain_str, ctx.base_currency, s.last_transaction_date
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format yearly overview
    let yearly_str = if ctx.yearly_overview.is_empty() {
        "Keine Jahresübersicht verfügbar".to_string()
    } else {
        ctx.yearly_overview
            .iter()
            .map(|y| {
                let gain_str = if y.realized_gains >= 0.0 {
                    format!("+{:.2}", y.realized_gains)
                } else {
                    format!("{:.2}", y.realized_gains)
                };
                format!(
                    "- {}: Realisierte Gewinne: {} {}, Dividenden: {:.2} {}, Transaktionen: {}",
                    y.year, gain_str, ctx.base_currency, y.dividends, ctx.base_currency, y.transaction_count
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let perf_str = match (ctx.ttwror, ctx.ttwror_annualized) {
        (Some(t), Some(a)) => format!("TTWROR: {:.1}% (p.a. {:.1}%)", t, a),
        (Some(t), None) => format!("TTWROR: {:.1}%", t),
        _ => "Keine Performance-Daten".to_string(),
    };

    // Currency allocation
    let currency_str = ctx.currency_allocation
        .iter()
        .map(|(c, p)| format!("{}: {:.1}%", c, p))
        .collect::<Vec<_>>()
        .join(", ");

    // Fees and taxes summary
    let fees_taxes_str = {
        let ft = &ctx.fees_and_taxes;
        let current_year = chrono::Utc::now().format("%Y").to_string();
        format!(
            "Gesamt Gebühren: {:.2} {}, Gesamt Steuern: {:.2} {}\n{} Gebühren: {:.2} {}, {} Steuern: {:.2} {}",
            ft.total_fees, ctx.base_currency, ft.total_taxes, ctx.base_currency,
            current_year, ft.fees_this_year, ctx.base_currency, current_year, ft.taxes_this_year, ctx.base_currency
        )
    };

    // Investment summary
    let investment_str = {
        let inv = &ctx.investment_summary;
        let first_date_str = inv.first_investment_date.as_ref()
            .map(|d| format!(", Erste Investition: {}", d))
            .unwrap_or_default();
        format!(
            "Investiert: {:.2} {}, Entnommen: {:.2} {}, Netto: {:.2} {}, Einzahlungen: {:.2} {}, Auszahlungen: {:.2} {}{}",
            inv.total_invested, ctx.base_currency,
            inv.total_withdrawn, ctx.base_currency,
            inv.net_invested, ctx.base_currency,
            inv.total_deposits, ctx.base_currency,
            inv.total_removals, ctx.base_currency,
            first_date_str
        )
    };

    // Sector/Taxonomy allocation
    let sector_str = if ctx.sector_allocation.is_empty() {
        "Keine Taxonomie-Zuordnungen".to_string()
    } else {
        ctx.sector_allocation
            .iter()
            .map(|s| {
                let allocs = s.allocations
                    .iter()
                    .take(5)
                    .map(|(name, pct)| format!("{}: {:.1}%", name, pct))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: {}", s.taxonomy_name, allocs)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Portfolio extremes
    let extremes_str = match &ctx.portfolio_extremes {
        Some(e) => format!(
            "Allzeithoch: {:.2} {} ({}), Allzeittief: {:.2} {} ({})\nJahreshoch {}: {:.2} {} ({}), Jahrestief: {:.2} {} ({})",
            e.all_time_high, ctx.base_currency, e.all_time_high_date,
            e.all_time_low, ctx.base_currency, e.all_time_low_date,
            chrono::Utc::now().format("%Y"),
            e.year_high, ctx.base_currency, e.year_high_date,
            e.year_low, ctx.base_currency, e.year_low_date
        ),
        None => "Keine historischen Daten verfügbar".to_string(),
    };

    // User greeting
    let user_greeting = match &ctx.user_name {
        Some(name) if !name.is_empty() => format!("Der Benutzer heißt {}. Sprich ihn gelegentlich mit Namen an, aber nicht in jeder Nachricht.", name),
        _ => "Der Benutzer hat keinen Namen angegeben.".to_string(),
    };

    // Provider status and quote sync info
    let provider_status_str = match &ctx.provider_status {
        Some(status) => {
            let mut sections: Vec<String> = Vec::new();

            // Quote sync status (always show)
            let sync = &status.quote_sync;
            let sync_str = if sync.synced_today_count == sync.held_count {
                format!(
                    "=== KURS-STATUS ({}) ===\nAlle {} Wertpapiere haben aktuelle Kurse von heute.",
                    sync.today, sync.held_count
                )
            } else {
                let outdated_str = sync.outdated.iter().take(10).cloned().collect::<Vec<_>>().join("\n- ");
                let more_str = if sync.outdated.len() > 10 {
                    format!("\n- ... und {} weitere", sync.outdated.len() - 10)
                } else {
                    String::new()
                };
                format!(
                    "=== KURS-STATUS ({}) ===\n{} von {} Wertpapieren haben KEINEN aktuellen Kurs von heute:\n- {}{}",
                    sync.today, sync.outdated_count, sync.held_count, outdated_str, more_str
                )
            };
            sections.push(sync_str);

            // Provider issues (only if any)
            if status.cannot_sync_count > 0 {
                let issues_str = status.issues.iter().take(5).cloned().collect::<Vec<_>>().join("\n- ");
                let more_str = if status.issues.len() > 5 {
                    format!("\n- ... und {} weitere", status.issues.len() - 5)
                } else {
                    String::new()
                };
                let api_key_hint = if !status.missing_api_keys.is_empty() {
                    format!("\nFehlende API-Keys: {}", status.missing_api_keys.join(", "))
                } else {
                    String::new()
                };
                sections.push(format!(
                    "=== PROVIDER-PROBLEME ===\n{} Wertpapiere können generell keine Kurse abrufen:\n- {}{}{}",
                    status.cannot_sync_count, issues_str, more_str, api_key_hint
                ));
            }

            format!("\n\n{}", sections.join("\n\n"))
        }
        None => String::new(),
    };

    format!(
        r##"Du bist ein Portfolio-Assistent für die App "Portfolio Now".

=== BENUTZER ===
{}

=== PORTFOLIO-ÜBERSICHT ===
- Gesamtwert: {:.2} {}
- Einstandswert: {:.2} {}
- Gesamtrendite: {:+.1}%
- {}
- Jährliche Dividenden: {:.2} {}
- Dividendenrendite: {:.2}%
- Währungsverteilung: {}
- Portfolio-Alter: {} Tage
- Stand: {}{}

=== ALLE HOLDINGS ({} Positionen) ===
{}

=== LETZTE TRANSAKTIONEN ===
{}

=== LETZTE DIVIDENDEN (12 Monate) ===
{}

=== WATCHLIST ===
{}

=== VERKAUFTE POSITIONEN (Historisch) ===
{}

=== JAHRESÜBERSICHT ===
{}

=== GEBÜHREN & STEUERN ===
{}

=== INVESTITIONSÜBERSICHT ===
{}

=== SEKTOR-ALLOKATION ===
{}

=== PORTFOLIO EXTREMWERTE ===
{}

=== DEINE FÄHIGKEITEN ===
Du kannst:
1. Alle Fragen zum Portfolio beantworten (Holdings, Performance, Dividenden, Transaktionen)
2. Aktien analysieren und LIVE im Web recherchieren (aktuelle Kurse, News, DAX-Stand etc.)
3. Finanzkonzepte erklären (TTWROR, IRR, FIFO, etc.)
4. Rebalancing-Vorschläge machen
5. Steuerliche Aspekte erläutern
6. WATCHLIST VERWALTEN - Du kannst Aktien zur Watchlist hinzufügen oder entfernen!

=== WEB-SUCHE ===
Bei Fragen zu AKTUELLEN Kursen, Indizes (DAX, S&P 500, etc.) oder News: Recherchiere SOFORT im Web!
Beispiele für Web-Suche: "Wie steht der DAX?", "Apple Kurs heute", "Aktuelle Nvidia News"

=== WATCHLIST-BEFEHLE ===
Wenn der Benutzer dich bittet, eine Aktie zur Watchlist hinzuzufügen oder zu entfernen, gib einen speziellen Befehl im JSON-Format aus.

WICHTIG: Gib den Befehl am ANFANG deiner Antwort aus, gefolgt von einer Bestätigung.

Zum HINZUFÜGEN (auch für Aktien die nicht im Bestand sind):
[[WATCHLIST_ADD:{{"watchlist":"Name der Watchlist","security":"Aktienname oder Ticker"}}]]

Zum ENTFERNEN:
[[WATCHLIST_REMOVE:{{"watchlist":"Name der Watchlist","security":"Aktienname oder Ticker"}}]]

Beispiele:
- "Füge Apple zu meiner Watchlist hinzu" → [[WATCHLIST_ADD:{{"watchlist":"Standard","security":"Apple"}}]]
- "Setze Tesla auf die Tech-Watchlist" → [[WATCHLIST_ADD:{{"watchlist":"Tech","security":"Tesla"}}]]
- "Entferne Microsoft von der Watchlist" → [[WATCHLIST_REMOVE:{{"watchlist":"Standard","security":"Microsoft"}}]]

Wenn keine Watchlist genannt wird, verwende "Standard" als Namen.
Du kannst auch Aktien hinzufügen, die nicht im Portfolio sind - sie werden automatisch gesucht und angelegt.

=== TRANSAKTIONS-ABFRAGEN ===
Du kannst ALLE Transaktionen abfragen - nicht nur die letzten 20 im Kontext oben.
Nutze diesen Befehl, wenn der Benutzer nach spezifischen oder allen Transaktionen fragt:

[[QUERY_TRANSACTIONS:{{"security":"Name oder Ticker","year":2024,"type":"BUY","limit":50}}]]

Parameter (alle optional):
- security: Name, Ticker oder ISIN des Wertpapiers
- year: Jahr der Transaktionen (z.B. 2024)
- type: BUY (inkl. Einlieferungen), SELL (inkl. Auslieferungen), DIVIDENDS
- limit: Maximale Anzahl (Standard: 100, Max: 500)

Beispiele:
- "Zeige alle Apple-Transaktionen" → [[QUERY_TRANSACTIONS:{{"security":"Apple"}}]]
- "Welche Käufe hatte ich 2024?" → [[QUERY_TRANSACTIONS:{{"year":2024,"type":"BUY"}}]]
- "Alle Transaktionen von Microsoft 2023" → [[QUERY_TRANSACTIONS:{{"security":"Microsoft","year":2023}}]]
- "Zeige alle meine Verkäufe" → [[QUERY_TRANSACTIONS:{{"type":"SELL"}}]]

WICHTIG: Einlieferungen werden als "BUY (Einlieferung)" angezeigt, Auslieferungen als "SELL (Auslieferung)".

=== PORTFOLIO-WERT ABFRAGEN ===
Du kannst den historischen Depotwert zu einem bestimmten Datum abfragen:

[[QUERY_PORTFOLIO_VALUE:{{"date":"2025-04-04"}}]]

Parameter:
- date: Datum im Format YYYY-MM-DD

Beispiele:
- "Wie hoch stand das Depot am 04.04.2025?" → [[QUERY_PORTFOLIO_VALUE:{{"date":"2025-04-04"}}]]
- "Depotwert Ende letztes Jahr" → [[QUERY_PORTFOLIO_VALUE:{{"date":"2024-12-31"}}]]
- "Wert am 1. Januar" → [[QUERY_PORTFOLIO_VALUE:{{"date":"2025-01-01"}}]]

WICHTIG: Gib den Befehl am ANFANG deiner Antwort aus!

=== ANTWORT-STIL ===
- KURZ und PRÄGNANT antworten - keine langen Einleitungen oder Zusammenfassungen
- Bullet Points nutzen, keine Fließtexte
- Bei Kurs-Fragen: Nur den Wert + kurze Info (max 2-3 Sätze)
- Portfolio-Zahlen konkret nennen wenn relevant
- Sprache: Deutsch"##,
        user_greeting,
        ctx.total_value,
        ctx.base_currency,
        ctx.total_cost_basis,
        ctx.base_currency,
        ctx.total_gain_loss_percent,
        perf_str,
        ctx.annual_dividends,
        ctx.base_currency,
        ctx.dividend_yield.unwrap_or(0.0),
        currency_str,
        ctx.portfolio_age_days,
        ctx.analysis_date,
        provider_status_str,
        ctx.holdings.len(),
        holdings_str,
        txn_str,
        div_str,
        watchlist_str,
        sold_positions_str,
        yearly_str,
        fees_taxes_str,
        investment_str,
        sector_str,
        extremes_str,
    )
}

/// Normalize AI response to ensure consistent markdown formatting
/// Fixes common issues where AI returns plain text instead of markdown
pub fn normalize_markdown_response(text: &str) -> String {
    let mut result = text.to_string();

    // Common headings that should be ## formatted
    let headings = [
        "Trend",
        "Support/Widerstand",
        "Support & Widerstand",
        "Unterstützung & Widerstand",
        "Unterstützung und Widerstand",
        "Muster",
        "Chartmuster",
        "Signal",
        "Indikatoren",
        "Einschätzung",
        "Risiko",
        "Risiken",
    ];

    for heading in headings {
        // Replace "Heading:" or "Heading\n" at start of line with "## Heading\n"
        // But only if not already prefixed with ##
        let patterns = [
            format!("\n{}:", heading),
            format!("\n{}\n", heading),
            format!("\n{}  \n", heading), // With trailing spaces
        ];

        for pattern in patterns {
            if result.contains(&pattern) && !result.contains(&format!("\n## {}", heading)) {
                result = result.replace(&pattern, &format!("\n## {}\n", heading));
            }
        }

        // Handle start of string
        if result.starts_with(&format!("{}:", heading)) || result.starts_with(&format!("{}\n", heading)) {
            if !result.starts_with("## ") {
                result = format!("## {}\n{}", heading, &result[heading.len()..].trim_start_matches(':').trim_start());
            }
        }
    }

    // Ensure there's a newline before ## if not at start
    result = result.replace("\n##", "\n\n##");
    result = result.replace("\n\n\n##", "\n\n##"); // Remove triple newlines

    // Remove any citations like [1], [2], etc. that Perplexity adds
    let citation_regex = regex::Regex::new(r"\[\d+\]").unwrap();
    result = citation_regex.replace_all(&result, "").to_string();

    result.trim().to_string()
}

// ============================================================================
// Model Listing API
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

/// Fetch available models from Claude API
pub async fn list_claude_models(api_key: &str) -> Result<Vec<AiModelInfo>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|e| anyhow!("Invalid API key: {}", e))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client
        .get("https://api.anthropic.com/v1/models")
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Claude API error: {}", body));
    }

    #[derive(Deserialize)]
    struct ClaudeModel {
        id: String,
        display_name: Option<String>,
    }

    #[derive(Deserialize)]
    struct ClaudeModelsResponse {
        data: Vec<ClaudeModel>,
    }

    let data: ClaudeModelsResponse = response.json().await?;

    // Filter to vision-capable models (claude-3 and newer support vision)
    let models: Vec<AiModelInfo> = data
        .data
        .into_iter()
        .filter(|m| {
            m.id.contains("claude-3") ||
            m.id.contains("claude-sonnet-4") ||
            m.id.contains("claude-opus-4") ||
            m.id.contains("claude-haiku-4")
        })
        .map(|m| {
            let description = if m.id.contains("opus") {
                "Beste Qualität"
            } else if m.id.contains("sonnet") {
                "Ausgewogen"
            } else if m.id.contains("haiku") {
                "Schnell & günstig"
            } else {
                "Vision-fähig"
            };
            AiModelInfo {
                name: m.display_name.unwrap_or_else(|| m.id.clone()),
                id: m.id,
                description: description.to_string(),
                supports_vision: true,
            }
        })
        .collect();

    Ok(models)
}

/// Fetch available models from OpenAI API
pub async fn list_openai_models(api_key: &str) -> Result<Vec<AiModelInfo>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| anyhow!("Invalid API key: {}", e))?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client
        .get("https://api.openai.com/v1/models")
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("OpenAI API error: {}", body));
    }

    #[derive(Deserialize)]
    struct OpenAIModel {
        id: String,
    }

    #[derive(Deserialize)]
    struct OpenAIModelsResponse {
        data: Vec<OpenAIModel>,
    }

    let data: OpenAIModelsResponse = response.json().await?;

    // Filter to vision-capable chat models (o3/o4 are reasoning-only, no vision)
    let vision_models = ["gpt-4o", "gpt-4-turbo", "gpt-4.1"];
    let models: Vec<AiModelInfo> = data
        .data
        .into_iter()
        .filter(|m| {
            vision_models.iter().any(|v| m.id.starts_with(v)) &&
            !m.id.contains("audio") &&
            !m.id.contains("realtime")
        })
        .map(|m| {
            let description = if m.id.contains("mini") {
                "Schnell & günstig"
            } else if m.id.starts_with("o3") || m.id.starts_with("o4") {
                "Reasoning-Modell"
            } else if m.id.contains("4.1") {
                "Neuestes, 1M Kontext"
            } else {
                "Multimodal"
            };
            AiModelInfo {
                name: m.id.clone(),
                id: m.id,
                description: description.to_string(),
                supports_vision: true,
            }
        })
        .collect();

    Ok(models)
}

/// Fetch available models from Gemini API
pub async fn list_gemini_models(api_key: &str) -> Result<Vec<AiModelInfo>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini API error: {}", body));
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GeminiModel {
        name: String,
        display_name: Option<String>,
        supported_generation_methods: Option<Vec<String>>,
    }

    #[derive(Deserialize)]
    struct GeminiModelsResponse {
        models: Vec<GeminiModel>,
    }

    let data: GeminiModelsResponse = response.json().await?;

    // Filter to models that support generateContent (i.e., can process images)
    let models: Vec<AiModelInfo> = data
        .models
        .into_iter()
        .filter(|m| {
            // Only include gemini models that support generateContent
            m.supported_generation_methods
                .as_ref()
                .map(|methods| methods.contains(&"generateContent".to_string()))
                .unwrap_or(false)
                && (m.name.contains("gemini-2") || m.name.contains("gemini-3"))
                && !m.name.contains("aqa")
                && !m.name.contains("embedding")
        })
        .map(|m| {
            // Extract model ID from "models/gemini-xxx" format
            let id = m.name.replace("models/", "");
            let description = if id.contains("pro") {
                if id.contains("preview") { "Beste Qualität (Preview)" } else { "Beste Qualität" }
            } else if id.contains("flash") {
                if id.contains("lite") { "Ultra-schnell" } else { "Schnell" }
            } else {
                "Vision-fähig"
            };
            AiModelInfo {
                name: m.display_name.unwrap_or_else(|| id.clone()),
                id,
                description: description.to_string(),
                supports_vision: true,
            }
        })
        .collect();

    Ok(models)
}

/// Fetch available models from Perplexity API
/// Note: Perplexity doesn't have a models endpoint, so we return hardcoded models
pub async fn list_perplexity_models(_api_key: &str) -> Result<Vec<AiModelInfo>> {
    // Perplexity doesn't expose a models list API, so we use known Sonar models
    // Only sonar and sonar-pro support vision input (reasoning/research models don't)
    Ok(vec![
        AiModelInfo {
            id: "sonar-pro".to_string(),
            name: "Sonar Pro".to_string(),
            description: "Beste Qualität + Web-Suche".to_string(),
            supports_vision: true,
        },
        AiModelInfo {
            id: "sonar".to_string(),
            name: "Sonar".to_string(),
            description: "Schnell + Web-Suche".to_string(),
            supports_vision: true,
        },
    ])
}
