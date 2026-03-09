//! AI-powered chart analysis module.
//!
//! Supports multiple providers: Claude (Anthropic), GPT-5 (OpenAI), Gemini (Google), Perplexity (Sonar)
//!
//! # Module Structure
//!
//! - `types`: All type definitions (requests, responses, errors, etc.)
//! - `prompts`: Prompt building functions for different analysis types
//! - `parsing`: Response parsing and utility functions
//! - `context`: Portfolio context loading for AI analysis
//! - `command_parser`: ChatBot command parsing and execution
//! - `models`: Vision model registry and metadata
//! - Provider implementations: `claude`, `openai`, `gemini`, `perplexity`

use std::collections::HashSet;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Provider implementations
pub mod claude;
pub mod gemini;
pub mod openai;
pub mod openrouter;
pub mod perplexity;

// Core modules
pub mod models;
pub mod types;
pub mod prompts;
pub mod parsing;

// Portfolio context and command parsing
pub mod command_parser;
pub mod context;
pub mod normalizer;

// Dynamic SQL execution (replaces query_templates, structured_query, user_templates)
pub mod sql_executor;

// Re-export SQL executor functions
pub use sql_executor::{
    extract_sql_from_response, remove_sql_blocks, validate_sql, execute_sql,
    format_as_markdown, is_sql_pattern_approved, approve_sql_pattern,
    clear_sql_approvals, SqlQuery, SqlResult, SqlValidationError,
};

// ============================================================================
// Session State for Query Approvals
// ============================================================================

/// Session-scoped approved query types
/// SECURITY: Approvals are cleared on app restart
static APPROVED_QUERY_TYPES: Lazy<Mutex<HashSet<command_parser::QueryType>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

/// Check if a query type has been approved for this session
pub fn is_query_type_approved(qt: &command_parser::QueryType) -> bool {
    APPROVED_QUERY_TYPES
        .lock()
        .map(|guard| guard.contains(qt))
        .unwrap_or(false)
}

/// Approve a query type for this session
pub fn approve_query_type(qt: command_parser::QueryType) {
    if let Ok(mut guard) = APPROVED_QUERY_TYPES.lock() {
        guard.insert(qt);
    }
}

/// Get all currently approved query types
pub fn get_approved_query_types() -> Vec<command_parser::QueryType> {
    APPROVED_QUERY_TYPES
        .lock()
        .map(|guard| guard.iter().cloned().collect())
        .unwrap_or_default()
}

/// Revoke all query type approvals (e.g., on logout or session end)
pub fn revoke_all_approvals() {
    if let Ok(mut guard) = APPROVED_QUERY_TYPES.lock() {
        guard.clear();
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Detect image media type from base64-encoded data by looking at magic bytes
pub fn detect_image_media_type(base64_data: &str) -> String {
    // Check first few characters of base64 for known magic byte patterns
    if base64_data.starts_with("/9j/") {
        "image/jpeg".to_string()
    } else if base64_data.starts_with("iVBOR") {
        "image/png".to_string()
    } else if base64_data.starts_with("R0lGO") {
        "image/gif".to_string()
    } else if base64_data.starts_with("UklGR") {
        "image/webp".to_string()
    } else {
        // Default to JPEG for optimized chart images
        "image/jpeg".to_string()
    }
}

// ============================================================================
// Re-exports from types module
// ============================================================================

pub use types::{
    // Constants
    REQUEST_TIMEOUT_SECS, MAX_RETRIES, RETRY_BASE_DELAY_MS,
    MAX_TOKENS, MAX_TOKENS_INSIGHTS, MAX_TOKENS_CHAT,
    // Error types
    AiError, AiErrorKind,
    // News research types
    NewsResearchResponse,
    // Chart analysis types
    ChartAnalysisRequest, ChartAnalysisResponse, ChartContext,
    IndicatorValue, CandleData, VolumeAnalysis,
    EnhancedChartContext, EnhancedChartAnalysisRequest, EnhancedAnnotationAnalysisResponse,
    AlertSuggestion, RiskRewardAnalysis, EnhancedAnnotationAnalysisJson,
    // Annotation types
    AnnotationType, SignalDirection, TrendDirection, TrendStrength, TrendInfo,
    ChartAnnotation, AnnotationAnalysisJson, AnnotationAnalysisResponse,
    // Portfolio types
    PortfolioSummary, HoldingSummary, RecentTransaction, DividendPayment, WatchlistItem,
    SoldPosition, YearlyOverview, PortfolioInsightsContext,
    FeesAndTaxesSummary, YearlyFeesAndTaxes, InvestmentSummary,
    SectorAllocation, PortfolioExtremes,
    QuoteProviderStatusSummary, QuoteSyncInfo,
    PortfolioInsightsResponse,
    // Chat types
    ChatMessage, ChatImageAttachment, PortfolioChatResponse, ChatSuggestedAction,
    // Transaction command types
    TransactionCreateCommand, PortfolioTransferCommand, TransactionValidationResult,
    // Model listing
    AiModelInfo,
};

// ============================================================================
// Re-exports from prompts module
// ============================================================================

pub use prompts::{
    is_fast_model,
    build_analysis_prompt,
    build_annotation_prompt,
    build_enhanced_annotation_prompt,
    build_portfolio_insights_prompt,
    build_opportunities_prompt,
    build_chat_system_prompt,
    build_news_research_prompt,
    build_trading_analysis_prompt,
};

// ============================================================================
// Re-exports from parsing module
// ============================================================================

pub use parsing::{
    parse_retry_delay,
    parse_annotation_response,
    parse_enhanced_annotation_response,
    calculate_backoff_delay,
    normalize_markdown_response,
};

// ============================================================================
// Re-exports from models module
// ============================================================================

pub use models::{
    get_default, get_fallback, get_model, get_model_upgrade, get_models_for_provider,
    get_model_provider, has_vision_support, is_valid_model,
    ModelInfo, VisionModel, DEPRECATED_MODELS, VISION_MODELS,
};

// ============================================================================
// Re-exports from context module
// ============================================================================

pub use context::load_portfolio_context;

// ============================================================================
// Re-exports from normalizer module
// ============================================================================

pub use normalizer::normalize_ai_response;

// ============================================================================
// Re-exports from command_parser module
// ============================================================================

pub use command_parser::{
    // Parsing functions (read-only)
    parse_watchlist_commands,
    // Transaction command parsing (returns suggestions, no auto-execution)
    parse_transaction_create_commands, parse_portfolio_transfer_commands,
    // Security: Suggestion-based execution (replaces auto-execution)
    parse_response_with_suggestions, execute_confirmed_watchlist_action,
    // Types
    WatchlistCommand,
    SuggestedAction, ParsedResponseWithSuggestions,
    // Transaction type normalization (SSOT – used by commands/ai.rs too)
    normalize_extracted_txn_type,
    // Security: Query approval types
    QueryType, PendingQuery,
    // Extracted transactions from images (returns suggestions, no auto-execution)
    ExtractedTransaction, ExtractedTransactionsPayload,
};

// ============================================================================
// Model Listing API Functions
// ============================================================================

use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

/// Check if a model ID is deprecated (exists in DEPRECATED_MODELS)
fn is_deprecated_model(model_id: &str) -> bool {
    models::DEPRECATED_MODELS.iter().any(|(old, _)| *old == model_id)
}
use serde::Deserialize;

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
        #[serde(default)]
        input_token_limit: Option<u32>,
        #[serde(default)]
        output_token_limit: Option<u32>,
        #[serde(default)]
        deprecation_date: Option<String>,
    }

    #[derive(Deserialize)]
    struct ClaudeModelsResponse {
        data: Vec<ClaudeModel>,
    }

    let data: ClaudeModelsResponse = response.json().await?;

    // Claude pricing per 1M tokens (USD) — pattern-based for date-variant IDs
    fn claude_pricing(id: &str) -> (Option<f64>, Option<f64>) {
        if id.contains("opus") { return (Some(5.0), Some(25.0)); }
        if id.contains("sonnet") { return (Some(3.0), Some(15.0)); }
        if id.contains("haiku") { return (Some(1.0), Some(5.0)); }
        (None, None)
    }

    // Include all claude models (claude-3+, claude-sonnet-4+, claude-opus-4+, claude-haiku-4+, future versions)
    // Filter out deprecated models that have been superseded
    let models: Vec<AiModelInfo> = data
        .data
        .into_iter()
        .filter(|m| {
            m.id.starts_with("claude-") &&
            !m.id.contains("claude-2") &&
            !m.id.contains("claude-1") &&
            !m.id.contains("instant") &&
            !is_deprecated_model(&m.id)
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
            let (pi, po) = claude_pricing(&m.id);
            AiModelInfo {
                name: m.display_name.unwrap_or_else(|| m.id.clone()),
                id: m.id,
                description: description.to_string(),
                supports_vision: true,
                supports_web_search: false,
                max_output_tokens: m.output_token_limit,
                context_window: m.input_token_limit,
                pricing_input: pi,
                pricing_output: po,
                deprecation_date: m.deprecation_date,
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

    // OpenAI /v1/models doesn't return token limits or pricing, so we use hardcoded lookups
    fn openai_limits(id: &str) -> (Option<u32>, Option<u32>) {
        // (max_output_tokens, context_window)
        if id.starts_with("o3") { return (Some(100_000), Some(200_000)); }
        if id.starts_with("o4") { return (Some(100_000), Some(200_000)); }
        if id.starts_with("gpt-5") { return (Some(128_000), Some(400_000)); }
        if id.starts_with("gpt-4.1") { return (Some(32_768), Some(1_000_000)); }
        if id.starts_with("gpt-4o-mini") { return (Some(16_384), Some(128_000)); }
        if id.starts_with("gpt-4o") { return (Some(16_384), Some(128_000)); }
        (None, None)
    }

    fn openai_pricing(id: &str) -> (Option<f64>, Option<f64>) {
        // (input $/1M, output $/1M)
        if id.starts_with("o3") { return (Some(2.0), Some(8.0)); }
        if id.starts_with("o4") { return (Some(1.10), Some(4.40)); }
        if id.starts_with("gpt-5") && id.contains("mini") { return (Some(0.25), Some(2.0)); }
        if id.starts_with("gpt-5") { return (Some(2.0), Some(8.0)); }
        if id.starts_with("gpt-4.1") { return (Some(2.0), Some(8.0)); }
        if id.starts_with("gpt-4o-mini") { return (Some(0.15), Some(0.60)); }
        if id.starts_with("gpt-4o") { return (Some(2.50), Some(10.0)); }
        (None, None)
    }

    // Include all relevant chat/reasoning models, exclude deprecated
    let allowed_prefixes = ["gpt-4o", "gpt-4.1", "gpt-5", "o3", "o4-mini"];
    let models: Vec<AiModelInfo> = data
        .data
        .into_iter()
        .filter(|m| {
            allowed_prefixes.iter().any(|v| m.id.starts_with(v)) &&
            !m.id.contains("audio") &&
            !m.id.contains("realtime") &&
            !m.id.contains("transcribe") &&
            !m.id.contains("tts") &&
            !is_deprecated_model(&m.id)
        })
        .map(|m| {
            let is_reasoning = m.id.starts_with("o3") || m.id.starts_with("o4");
            let has_web_search = is_reasoning; // o3/o4-mini support web search via tool
            let description = if m.id.contains("mini") && m.id.starts_with("gpt-5") {
                "Neuestes GPT-5, schnell & günstig"
            } else if m.id.contains("mini") {
                "Schnell & günstig"
            } else if m.id.starts_with("o3") {
                "Smartest, Vision + Web-Suche"
            } else if m.id.starts_with("o4") {
                "Schnell, Vision + Web-Suche"
            } else if m.id.contains("4.1") {
                "1M Kontext"
            } else {
                "Multimodal"
            };
            let (max_output, ctx) = openai_limits(&m.id);
            let (pi, po) = openai_pricing(&m.id);
            let supports_vision = !m.id.contains("4.1"); // GPT-4.1 is text-only
            AiModelInfo {
                name: m.id.clone(),
                id: m.id,
                description: description.to_string(),
                supports_vision,
                supports_web_search: has_web_search,
                max_output_tokens: max_output,
                context_window: ctx,
                pricing_input: pi,
                pricing_output: po,
                deprecation_date: None,
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
        #[serde(default)]
        input_token_limit: Option<u32>,
        #[serde(default)]
        output_token_limit: Option<u32>,
    }

    #[derive(Deserialize)]
    struct GeminiModelsResponse {
        models: Vec<GeminiModel>,
    }

    let data: GeminiModelsResponse = response.json().await?;

    // Gemini pricing per 1M tokens (USD) — pattern-based
    fn gemini_pricing(id: &str) -> (Option<f64>, Option<f64>) {
        if id.contains("flash") && id.contains("lite") { return (Some(0.075), Some(0.30)); }
        if id.contains("flash") { return (Some(0.30), Some(2.50)); }
        if id.contains("pro") { return (Some(1.25), Some(10.0)); }
        (None, None)
    }

    // Filter to models that support generateContent — no hardcoded version filter
    let models: Vec<AiModelInfo> = data
        .models
        .into_iter()
        .filter(|m| {
            m.supported_generation_methods
                .as_ref()
                .map(|methods| methods.contains(&"generateContent".to_string()))
                .unwrap_or(false)
                && m.name.contains("gemini")
                && !m.name.contains("aqa")
                && !m.name.contains("embedding")
                && !m.name.contains("bisection")
                // Exclude very old models (1.0, 1.5)
                && !m.name.contains("gemini-1.0")
                && !m.name.contains("gemini-1.5")
                && !is_deprecated_model(&m.name.replace("models/", ""))
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
            let (pi, po) = gemini_pricing(&id);
            AiModelInfo {
                name: m.display_name.unwrap_or_else(|| id.clone()),
                id,
                description: description.to_string(),
                supports_vision: true,
                supports_web_search: false,
                max_output_tokens: m.output_token_limit,
                context_window: m.input_token_limit,
                pricing_input: pi,
                pricing_output: po,
                deprecation_date: None,
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
            supports_web_search: true,
            max_output_tokens: Some(128_000),
            context_window: Some(200_000),
            pricing_input: Some(3.0),
            pricing_output: Some(15.0),
            deprecation_date: None,
        },
        AiModelInfo {
            id: "sonar".to_string(),
            name: "Sonar".to_string(),
            description: "Schnell + Web-Suche".to_string(),
            supports_vision: true,
            supports_web_search: true,
            max_output_tokens: Some(128_000),
            context_window: Some(200_000),
            pricing_input: Some(1.0),
            pricing_output: Some(1.0),
            deprecation_date: None,
        },
    ])
}

/// Fetch available models from OpenRouter API
pub async fn list_openrouter_models(api_key: &str) -> Result<Vec<AiModelInfo>> {
    openrouter::list_models(api_key).await
}
