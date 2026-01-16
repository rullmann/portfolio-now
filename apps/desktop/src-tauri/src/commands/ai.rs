//! AI chart analysis commands
//!
//! This module provides Tauri commands for AI-powered analysis features:
//! - Chart analysis with annotations (support/resistance, patterns, signals)
//! - Portfolio insights generation
//! - Portfolio chat assistant with action commands

use crate::ai::{
    claude, gemini, openai, perplexity,
    list_claude_models, list_openai_models, list_gemini_models, list_perplexity_models,
    get_model_upgrade, get_models_for_provider, ModelInfo,
    AiModelInfo, AiError, ChartAnalysisRequest, ChartAnalysisResponse, AnnotationAnalysisResponse,
    EnhancedChartAnalysisRequest, EnhancedAnnotationAnalysisResponse,
    PortfolioInsightsResponse, ChatMessage, PortfolioChatResponse, ChatSuggestedAction,
    // Context loading from ai/context.rs
    load_portfolio_context,
    // Command parsing from ai/command_parser.rs
    parse_response_with_suggestions,
};
use serde::Deserialize;
use tauri::{command, AppHandle, Emitter};

/// Analyze a chart using AI
///
/// Returns ChartAnalysisResponse on success, or a JSON-serialized AiError on failure.
/// Automatically upgrades deprecated models to their replacements.
#[command]
pub async fn analyze_chart_with_ai(
    request: ChartAnalysisRequest,
) -> Result<ChartAnalysisResponse, String> {
    // Check if the model is deprecated and auto-upgrade
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    let result = match request.provider.as_str() {
        "claude" => claude::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        "openai" => openai::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        "gemini" => gemini::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        "perplexity" => perplexity::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    // Convert AiError to JSON string for frontend parsing
    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}

/// Fetch available models for a given AI provider
#[command]
pub async fn get_ai_models(
    provider: String,
    api_key: String,
) -> Result<Vec<AiModelInfo>, String> {
    match provider.as_str() {
        "claude" => list_claude_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        "openai" => list_openai_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        "gemini" => list_gemini_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        "perplexity" => list_perplexity_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        _ => Err(format!("Unknown AI provider: {}", provider)),
    }
}

/// Analyze a chart using AI and return structured annotations
///
/// Returns AnnotationAnalysisResponse on success with support/resistance levels,
/// patterns, and signals as structured JSON instead of markdown text.
#[command]
pub async fn analyze_chart_with_annotations(
    request: ChartAnalysisRequest,
) -> Result<AnnotationAnalysisResponse, String> {
    // Check if the model is deprecated and auto-upgrade
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    let result = match request.provider.as_str() {
        "claude" => claude::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        "openai" => openai::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        "gemini" => gemini::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        "perplexity" => perplexity::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    // Convert AiError to JSON string for frontend parsing
    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}

/// Analyze a chart using AI with enhanced context (indicator values, OHLC data, volume)
/// and return structured annotations with alerts and risk/reward analysis.
///
/// This is the enhanced version that provides indicator values to the AI
/// instead of just names, enabling more precise analysis.
#[command]
pub async fn analyze_chart_enhanced(
    request: EnhancedChartAnalysisRequest,
) -> Result<EnhancedAnnotationAnalysisResponse, String> {
    // Check if the model is deprecated and auto-upgrade
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    let result = match request.provider.as_str() {
        "claude" => claude::analyze_enhanced(&request.image_base64, &model, &request.api_key, &request.context).await,
        "openai" => openai::analyze_enhanced(&request.image_base64, &model, &request.api_key, &request.context).await,
        "gemini" => gemini::analyze_enhanced(&request.image_base64, &model, &request.api_key, &request.context).await,
        "perplexity" => perplexity::analyze_enhanced(&request.image_base64, &model, &request.api_key, &request.context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    // Convert AiError to JSON string for frontend parsing
    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}

/// Get vision-capable models for a provider from the centralized registry.
///
/// Returns the list of vision models from the static registry.
/// No API key required - this uses the internal model definitions.
#[command]
pub fn get_vision_models(provider: String) -> Vec<ModelInfo> {
    get_models_for_provider(&provider)
        .into_iter()
        .map(ModelInfo::from)
        .collect()
}

// ============================================================================
// Portfolio Insights Commands
// ============================================================================

/// Request for portfolio insights analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioInsightsRequest {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_currency: String,
    /// Analysis type: "insights" (portfolio evaluation) or "opportunities" (buy recommendations)
    #[serde(default = "default_insights")]
    pub analysis_type: String,
}

fn default_insights() -> String {
    "insights".to_string()
}


/// Analyze portfolio with AI to get insights
///
/// Returns a markdown-formatted analysis with strengths, weaknesses, and recommendations.
/// The `analysis_type` parameter determines whether to analyze for general insights or buying opportunities.
#[command]
pub async fn analyze_portfolio_with_ai(
    _app: AppHandle,
    request: PortfolioInsightsRequest,
) -> Result<PortfolioInsightsResponse, String> {
    // Load portfolio context without technical signals (AI does the analysis now)
    let context = load_portfolio_context(
        &request.base_currency,
        None,
        false, // No technical signals needed - AI analyzes directly
        None,
    )?;

    // Check if portfolio has holdings
    if context.holdings.is_empty() {
        return Err("Keine Holdings im Portfolio gefunden. Bitte importiere zuerst Transaktionen.".to_string());
    }

    // Auto-upgrade deprecated models
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    // Call the appropriate provider based on analysis type
    let result = match request.analysis_type.as_str() {
        "opportunities" => {
            // Buy opportunity analysis
            match request.provider.as_str() {
                "claude" => claude::analyze_opportunities(&model, &request.api_key, &context).await,
                "openai" => openai::analyze_opportunities(&model, &request.api_key, &context).await,
                "gemini" => gemini::analyze_opportunities(&model, &request.api_key, &context).await,
                "perplexity" => perplexity::analyze_opportunities(&model, &request.api_key, &context).await,
                _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
            }
        }
        _ => {
            // Default: portfolio insights
            match request.provider.as_str() {
                "claude" => claude::analyze_portfolio(&model, &request.api_key, &context).await,
                "openai" => openai::analyze_portfolio(&model, &request.api_key, &context).await,
                "gemini" => gemini::analyze_portfolio(&model, &request.api_key, &context).await,
                "perplexity" => perplexity::analyze_portfolio(&model, &request.api_key, &context).await,
                _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
            }
        }
    };

    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}

// ============================================================================
// Portfolio Chat Commands
// ============================================================================

/// Request for portfolio chat
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioChatRequest {
    pub messages: Vec<ChatMessage>,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_currency: String,
    pub user_name: Option<String>,
}

/// Chat with portfolio assistant
///
/// Sends user messages to AI with portfolio context injected.
/// The AI can execute embedded commands for watchlist management and data queries.
#[command]
pub async fn chat_with_portfolio_assistant(
    _app: AppHandle,
    request: PortfolioChatRequest,
) -> Result<PortfolioChatResponse, String> {
    // Load portfolio context from database with user name
    // For chat, we always include technical signals (no progress events needed)
    let context = load_portfolio_context(&request.base_currency, request.user_name.clone(), true, None)?;

    // Auto-upgrade deprecated models
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    // Call the appropriate provider
    let result = match request.provider.as_str() {
        "claude" => claude::chat(&model, &request.api_key, &request.messages, &context).await,
        "openai" => openai::chat(&model, &request.api_key, &request.messages, &context).await,
        "gemini" => gemini::chat(&model, &request.api_key, &request.messages, &context).await,
        "perplexity" => perplexity::chat(&model, &request.api_key, &request.messages, &context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    // Process the result using the secure suggestion-based command parser
    // SECURITY: This uses parse_response_with_suggestions which:
    // - Returns watchlist modifications as SUGGESTIONS (not executed)
    // - Only executes read-only queries (transactions, portfolio value)
    match result {
        Ok(mut response) => {
            // Parse response and extract suggestions (watchlist commands NOT executed)
            let parsed = parse_response_with_suggestions(response.response.clone());

            // Update response with cleaned text
            response.response = parsed.cleaned_response;

            // Append query results if any (read-only queries are safe to execute)
            if !parsed.query_results.is_empty() {
                if response.response.trim().is_empty() || response.response.len() < 10 {
                    response.response = parsed.query_results.join("\n\n");
                } else {
                    response.response = format!("{}\n\n{}", response.response, parsed.query_results.join("\n\n"));
                }
            }

            // Convert suggestions to response format
            // Frontend must display these and get user confirmation before executing
            response.suggestions = parsed.suggestions
                .into_iter()
                .map(|s| ChatSuggestedAction {
                    action_type: s.action_type,
                    description: s.description,
                    payload: s.payload,
                })
                .collect();

            Ok(response)
        }
        Err(e) => Err(serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())),
    }
}

// ============================================================================
// Confirmed Action Execution (Security)
// ============================================================================

/// Execute a confirmed AI-suggested watchlist action
///
/// SECURITY: This command should only be called after explicit user confirmation.
/// The frontend must display the suggested action and get user approval before
/// calling this command. This prevents prompt injection attacks.
#[command]
pub async fn execute_confirmed_ai_action(
    app: AppHandle,
    action_type: String,
    payload: String,
    alpha_vantage_api_key: Option<String>,
) -> Result<String, String> {
    use crate::ai::execute_confirmed_watchlist_action;

    log::info!("Executing confirmed AI action: {} with payload: {}", action_type, payload);

    let result = execute_confirmed_watchlist_action(
        &action_type,
        &payload,
        alpha_vantage_api_key,
    ).await?;

    // Emit watchlist update event if successful
    if action_type.starts_with("watchlist") {
        let _ = app.emit("watchlist-updated", ());
    }

    Ok(result)
}
