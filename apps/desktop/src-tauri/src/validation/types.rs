//! Symbol Validation Types
//!
//! Types for the symbol validation engine that validates and corrects
//! quote source configurations for securities.

use serde::{Deserialize, Serialize};

/// Validation status for a security's quote configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    /// Not yet validated
    Pending,
    /// Successfully validated with working configuration
    Validated,
    /// AI has suggested a configuration (needs user confirmation)
    AiSuggested,
    /// Validation failed, manual intervention required
    Failed,
    /// Skipped (e.g., retired security, manual feed)
    Skipped,
}

impl ValidationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Validated => "validated",
            Self::AiSuggested => "ai_suggested",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "validated" => Self::Validated,
            "ai_suggested" => Self::AiSuggested,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            _ => Self::Pending,
        }
    }
}

/// Method used for validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationMethod {
    /// Validated through code (provider API search + quote verification)
    Code,
    /// Suggested by AI
    Ai,
    /// Manually set by user
    User,
}

impl ValidationMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Ai => "ai",
            Self::User => "user",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "code" => Some(Self::Code),
            "ai" => Some(Self::Ai),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

/// Validated quote configuration for a security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedConfig {
    /// Provider name (YAHOO, TRADINGVIEW, COINGECKO, etc.)
    pub feed: String,
    /// Feed URL / exchange suffix (e.g., ".DE", "XETR", etc.)
    pub feed_url: Option<String>,
    /// Ticker symbol
    pub ticker: Option<String>,
    /// Exchange name
    pub exchange: Option<String>,
}

/// Search result from a quote provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSearchResult {
    /// Provider name
    pub provider: String,
    /// Symbol/ticker
    pub symbol: String,
    /// Security name
    pub name: Option<String>,
    /// Exchange
    pub exchange: Option<String>,
    /// Security type (stock, ETF, fund, crypto, etc.)
    pub security_type: Option<String>,
    /// Currency
    pub currency: Option<String>,
    /// ISIN if available
    pub isin: Option<String>,
    /// Match confidence (0.0 - 1.0)
    pub confidence: f64,
}

/// AI suggestion for quote configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSuggestion {
    /// Suggested feed
    pub feed: String,
    /// Suggested ticker
    pub ticker: String,
    /// Suggested feed URL / exchange
    pub feed_url: Option<String>,
    /// AI's reasoning
    pub reasoning: String,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
}

/// Symbol mapping entry (stored in database)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolMapping {
    pub id: i64,
    pub security_id: i64,
    pub validated_feed: String,
    pub validated_feed_url: Option<String>,
    pub validated_ticker: Option<String>,
    pub validated_exchange: Option<String>,
    pub provider_results: Option<String>, // JSON
    pub validation_status: ValidationStatus,
    pub confidence: f64,
    pub validation_method: Option<ValidationMethod>,
    pub ai_suggestion_json: Option<String>, // JSON
    pub last_validated_at: Option<String>,
    pub price_check_success: bool,
    pub created_at: String,
}

/// Result of validating a single security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub security_id: i64,
    pub security_name: String,
    pub isin: Option<String>,
    pub original_feed: Option<String>,
    pub original_ticker: Option<String>,
    pub status: ValidationStatus,
    pub validated_config: Option<ValidatedConfig>,
    pub ai_suggestion: Option<AiSuggestion>,
    pub provider_results: Vec<ProviderSearchResult>,
    pub confidence: f64,
    pub error: Option<String>,
}

/// Validation run status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRun {
    pub id: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub total_securities: i32,
    pub validated_count: i32,
    pub failed_count: i32,
    pub ai_suggested_count: i32,
    pub status: String,
}

/// Overall validation status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationStatusSummary {
    pub total_securities: i32,
    pub validated_count: i32,
    pub pending_count: i32,
    pub failed_count: i32,
    pub ai_suggested_count: i32,
    pub skipped_count: i32,
    pub last_run: Option<ValidationRun>,
    pub securities_needing_attention: Vec<ValidationResult>,
}

/// Security data needed for validation
#[derive(Debug, Clone)]
pub struct SecurityForValidation {
    pub id: i64,
    pub name: String,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub ticker: Option<String>,
    pub currency: String,
    pub feed: Option<String>,
    pub feed_url: Option<String>,
    pub is_retired: bool,
}

/// API keys for providers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeys {
    pub coingecko_api_key: Option<String>,
    pub finnhub_api_key: Option<String>,
    pub alpha_vantage_api_key: Option<String>,
    pub twelve_data_api_key: Option<String>,
}

/// AI configuration for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: String, // claude, openai, gemini, perplexity
    pub model: String,
    pub api_key: String,
}
