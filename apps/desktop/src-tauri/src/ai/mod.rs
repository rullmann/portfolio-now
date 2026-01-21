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

// Provider implementations
pub mod claude;
pub mod gemini;
pub mod openai;
pub mod perplexity;

// Core modules
pub mod models;
pub mod types;
pub mod prompts;
pub mod parsing;

// Portfolio context and command parsing
pub mod command_parser;
pub mod context;
pub mod query_templates;

// ============================================================================
// Re-exports from types module
// ============================================================================

pub use types::{
    // Constants
    REQUEST_TIMEOUT_SECS, MAX_RETRIES, RETRY_BASE_DELAY_MS,
    MAX_TOKENS, MAX_TOKENS_INSIGHTS, MAX_TOKENS_CHAT,
    // Error types
    AiError, AiErrorKind,
    // Chart analysis types
    ChartAnalysisRequest, ChartAnalysisResponse, ChartContext,
    IndicatorValue, CandleData, VolumeAnalysis,
    EnhancedChartContext, EnhancedChartAnalysisRequest, EnhancedAnnotationAnalysisResponse,
    AlertSuggestion, RiskRewardAnalysis, EnhancedAnnotationAnalysisJson,
    // Annotation types
    AnnotationType, SignalDirection, TrendDirection, TrendStrength, TrendInfo,
    ChartAnnotation, AnnotationAnalysisJson, AnnotationAnalysisResponse,
    // Portfolio types
    HoldingSummary, RecentTransaction, DividendPayment, WatchlistItem,
    SoldPosition, YearlyOverview, PortfolioInsightsContext,
    FeesAndTaxesSummary, YearlyFeesAndTaxes, InvestmentSummary,
    SectorAllocation, PortfolioExtremes,
    QuoteProviderStatusSummary, QuoteSyncInfo,
    PortfolioInsightsResponse,
    // Chat types
    ChatMessage, PortfolioChatResponse, ChatSuggestedAction,
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
    is_valid_model, ModelInfo, VisionModel, DEPRECATED_MODELS, VISION_MODELS,
};

// ============================================================================
// Re-exports from context module
// ============================================================================

pub use context::load_portfolio_context;

// ============================================================================
// Re-exports from command_parser module
// ============================================================================

pub use command_parser::{
    // Parsing functions (read-only)
    parse_watchlist_commands,
    parse_transaction_queries, execute_transaction_queries,
    parse_portfolio_value_queries, execute_portfolio_value_queries,
    // Transaction command parsing (returns suggestions, no auto-execution)
    parse_transaction_create_commands, parse_portfolio_transfer_commands,
    // Security: Suggestion-based execution (replaces auto-execution)
    parse_response_with_suggestions, execute_confirmed_watchlist_action,
    // Types
    WatchlistCommand, TransactionQuery, PortfolioValueQuery,
    SuggestedAction, ParsedResponseWithSuggestions,
};

// ============================================================================
// Model Listing API Functions
// ============================================================================

use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
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
