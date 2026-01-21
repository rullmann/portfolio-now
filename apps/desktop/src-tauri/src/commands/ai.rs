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

// ============================================================================
// Transaction Execution Commands
// ============================================================================

use crate::ai::types::{TransactionCreateCommand, PortfolioTransferCommand};
use crate::commands::crud::{create_transaction, CreateTransactionRequest};
use crate::events::{emit_data_changed, DataChangedPayload};

/// Execute a confirmed transaction creation
///
/// SECURITY: This command should only be called after explicit user confirmation.
/// The frontend MUST display a transaction preview and get user approval before
/// calling this command. This prevents prompt injection attacks.
#[command]
pub fn execute_confirmed_transaction(
    app: AppHandle,
    payload: String,
) -> Result<String, String> {
    let cmd: TransactionCreateCommand = serde_json::from_str(&payload)
        .map_err(|e| format!("Invalid transaction payload: {}", e))?;

    // Determine owner type and ID
    let (owner_type, owner_id) = match (&cmd.portfolio_id, &cmd.account_id) {
        (Some(pid), _) if is_portfolio_transaction(&cmd.txn_type) => ("portfolio".to_string(), *pid),
        (_, Some(aid)) if is_account_transaction(&cmd.txn_type) => ("account".to_string(), *aid),
        (Some(pid), _) => ("portfolio".to_string(), *pid),
        (_, Some(aid)) => ("account".to_string(), *aid),
        _ => return Err("Kein Depot oder Konto angegeben".to_string()),
    };

    // Validate required fields based on transaction type
    validate_transaction_fields(&cmd)?;

    // Build CreateTransactionRequest
    let request = CreateTransactionRequest {
        owner_type: owner_type.clone(),
        owner_id,
        txn_type: cmd.txn_type.clone(),
        date: cmd.date.clone(),
        amount: cmd.amount.unwrap_or(0),
        currency: cmd.currency.clone(),
        shares: cmd.shares,
        security_id: cmd.security_id,
        note: cmd.note.clone(),
        units: None, // No units from AI commands
        reference_account_id: None, // Could be extended later
    };

    // Create the transaction
    let result = create_transaction(app.clone(), request)
        .map_err(|e| format!("Fehler beim Erstellen der Transaktion: {}", e))?;

    // Emit data_changed event
    emit_data_changed(&app, DataChangedPayload::transaction("created", cmd.security_id));

    // Format success message
    let type_label = get_transaction_type_label(&cmd.txn_type);
    let security_str = cmd.security_name.as_ref()
        .map(|n| format!(" für {}", n))
        .unwrap_or_default();
    let amount_str = cmd.amount
        .map(|a| format!(" über {:.2} {}", a as f64 / 100.0, cmd.currency))
        .unwrap_or_default();

    Ok(format!(
        "{}{}{} am {} erstellt (ID: {})",
        type_label, security_str, amount_str, cmd.date, result.id
    ))
}

/// Execute a confirmed portfolio transfer (Depotwechsel)
///
/// SECURITY: This command should only be called after explicit user confirmation.
#[command]
pub fn execute_confirmed_portfolio_transfer(
    app: AppHandle,
    payload: String,
) -> Result<String, String> {
    let cmd: PortfolioTransferCommand = serde_json::from_str(&payload)
        .map_err(|e| format!("Invalid transfer payload: {}", e))?;

    // Create DELIVERY_OUTBOUND from source portfolio
    let outbound_request = CreateTransactionRequest {
        owner_type: "portfolio".to_string(),
        owner_id: cmd.from_portfolio_id,
        txn_type: "DELIVERY_OUTBOUND".to_string(),
        date: cmd.date.clone(),
        amount: 0, // Deliveries don't have an amount
        currency: "EUR".to_string(), // Will be updated by the system
        shares: Some(cmd.shares),
        security_id: Some(cmd.security_id),
        note: cmd.note.clone(),
        units: None,
        reference_account_id: None,
    };

    let outbound_result = create_transaction(app.clone(), outbound_request)
        .map_err(|e| format!("Fehler bei Auslieferung: {}", e))?;

    // Create DELIVERY_INBOUND to target portfolio
    let inbound_request = CreateTransactionRequest {
        owner_type: "portfolio".to_string(),
        owner_id: cmd.to_portfolio_id,
        txn_type: "DELIVERY_INBOUND".to_string(),
        date: cmd.date.clone(),
        amount: 0,
        currency: "EUR".to_string(),
        shares: Some(cmd.shares),
        security_id: Some(cmd.security_id),
        note: cmd.note.clone(),
        units: None,
        reference_account_id: None,
    };

    let inbound_result = create_transaction(app.clone(), inbound_request)
        .map_err(|e| format!("Fehler bei Einlieferung: {}", e))?;

    // Emit data_changed event
    emit_data_changed(&app, DataChangedPayload::transaction("created", Some(cmd.security_id)));

    let shares_display = cmd.shares as f64 / 100_000_000.0;
    Ok(format!(
        "Depotwechsel erfolgreich: {:.4} Stück am {} übertragen (Auslieferung ID: {}, Einlieferung ID: {})",
        shares_display, cmd.date, outbound_result.id, inbound_result.id
    ))
}

/// Check if transaction type is for portfolios
fn is_portfolio_transaction(txn_type: &str) -> bool {
    matches!(
        txn_type,
        "BUY" | "SELL" | "TRANSFER_IN" | "TRANSFER_OUT" | "DELIVERY_INBOUND" | "DELIVERY_OUTBOUND"
    )
}

/// Check if transaction type is for accounts
fn is_account_transaction(txn_type: &str) -> bool {
    matches!(
        txn_type,
        "DEPOSIT" | "REMOVAL" | "INTEREST" | "INTEREST_CHARGE" | "DIVIDENDS" |
        "FEES" | "FEES_REFUND" | "TAXES" | "TAX_REFUND"
    )
}

/// Validate that required fields are present for the transaction type
fn validate_transaction_fields(cmd: &TransactionCreateCommand) -> Result<(), String> {
    match cmd.txn_type.as_str() {
        "BUY" | "SELL" => {
            if cmd.portfolio_id.is_none() {
                return Err("Kein Depot angegeben".to_string());
            }
            if cmd.security_id.is_none() {
                return Err("Kein Wertpapier angegeben".to_string());
            }
            if cmd.shares.is_none() || cmd.shares == Some(0) {
                return Err("Keine Stückzahl angegeben".to_string());
            }
            if cmd.amount.is_none() || cmd.amount == Some(0) {
                return Err("Kein Betrag angegeben".to_string());
            }
        }
        "DELIVERY_INBOUND" | "DELIVERY_OUTBOUND" => {
            if cmd.portfolio_id.is_none() {
                return Err("Kein Depot angegeben".to_string());
            }
            if cmd.security_id.is_none() {
                return Err("Kein Wertpapier angegeben".to_string());
            }
            if cmd.shares.is_none() || cmd.shares == Some(0) {
                return Err("Keine Stückzahl angegeben".to_string());
            }
        }
        "DIVIDENDS" => {
            if cmd.account_id.is_none() {
                return Err("Kein Konto angegeben".to_string());
            }
            if cmd.security_id.is_none() {
                return Err("Kein Wertpapier angegeben".to_string());
            }
            if cmd.amount.is_none() || cmd.amount == Some(0) {
                return Err("Kein Betrag angegeben".to_string());
            }
        }
        "DEPOSIT" | "REMOVAL" | "INTEREST" | "FEES" | "TAXES" => {
            if cmd.account_id.is_none() {
                return Err("Kein Konto angegeben".to_string());
            }
            if cmd.amount.is_none() || cmd.amount == Some(0) {
                return Err("Kein Betrag angegeben".to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

/// Get German label for transaction type
fn get_transaction_type_label(txn_type: &str) -> &'static str {
    match txn_type {
        "BUY" => "Kauf",
        "SELL" => "Verkauf",
        "DELIVERY_INBOUND" => "Einlieferung",
        "DELIVERY_OUTBOUND" => "Auslieferung",
        "DIVIDENDS" => "Dividende",
        "DEPOSIT" => "Einlage",
        "REMOVAL" => "Entnahme",
        "INTEREST" => "Zinsen",
        "INTEREST_CHARGE" => "Zinsbelastung",
        "FEES" => "Gebühren",
        "FEES_REFUND" => "Gebührenerstattung",
        "TAXES" => "Steuern",
        "TAX_REFUND" => "Steuererstattung",
        "TRANSFER_IN" => "Umbuchung (Eingang)",
        "TRANSFER_OUT" => "Umbuchung (Ausgang)",
        _ => "Transaktion",
    }
}
