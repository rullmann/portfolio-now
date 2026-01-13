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
    PortfolioInsightsResponse, ChatMessage, PortfolioChatResponse,
    // Context loading from ai/context.rs
    load_portfolio_context,
    // Command parsing from ai/command_parser.rs
    process_response_commands,
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
}


/// Analyze portfolio with AI to get insights
///
/// Returns a markdown-formatted analysis with strengths, weaknesses, and recommendations.
#[command]
pub async fn analyze_portfolio_with_ai(
    request: PortfolioInsightsRequest,
) -> Result<PortfolioInsightsResponse, String> {
    // Load portfolio context from database (no user name for insights)
    let context = load_portfolio_context(&request.base_currency, None)?;

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

    // Call the appropriate provider
    let result = match request.provider.as_str() {
        "claude" => claude::analyze_portfolio(&model, &request.api_key, &context).await,
        "openai" => openai::analyze_portfolio(&model, &request.api_key, &context).await,
        "gemini" => gemini::analyze_portfolio(&model, &request.api_key, &context).await,
        "perplexity" => perplexity::analyze_portfolio(&model, &request.api_key, &context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
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
    pub alpha_vantage_api_key: Option<String>,
    pub user_name: Option<String>,
}

/// Chat with portfolio assistant
///
/// Sends user messages to AI with portfolio context injected.
/// The AI can execute embedded commands for watchlist management and data queries.
#[command]
pub async fn chat_with_portfolio_assistant(
    app: AppHandle,
    request: PortfolioChatRequest,
) -> Result<PortfolioChatResponse, String> {
    // Load portfolio context from database with user name
    let context = load_portfolio_context(&request.base_currency, request.user_name.clone())?;

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

    // Process the result using the command parser module
    match result {
        Ok(mut response) => {
            // Process all embedded commands (watchlist, transactions, portfolio value queries)
            // The emit callback is called when watchlist commands are executed
            let app_handle = app.clone();
            let (cleaned_response, additional_results) = process_response_commands(
                response.response.clone(),
                request.alpha_vantage_api_key,
                move || { let _ = app_handle.emit("watchlist-updated", ()); },
            ).await;

            // Update response with cleaned text
            response.response = cleaned_response;

            // If there are command results, append or replace
            if !additional_results.is_empty() {
                if response.response.trim().is_empty() || response.response.len() < 10 {
                    response.response = additional_results.join("\n\n");
                } else {
                    response.response = format!("{}\n\n{}", response.response, additional_results.join("\n\n"));
                }
            }

            Ok(response)
        }
        Err(e) => Err(serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())),
    }
}
