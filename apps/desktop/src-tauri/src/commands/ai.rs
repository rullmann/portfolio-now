//! AI chart analysis commands
//!
//! This module provides Tauri commands for AI-powered analysis features:
//! - Chart analysis with annotations (support/resistance, patterns, signals)
//! - Portfolio insights generation
//! - Portfolio chat assistant with action commands

use crate::ai::{
    claude, gemini, openai, perplexity,
    list_claude_models, list_openai_models, list_gemini_models, list_perplexity_models,
    get_model_upgrade, get_models_for_provider, has_vision_support, ModelInfo,
    AiModelInfo, AiError, ChartAnalysisRequest, ChartAnalysisResponse, AnnotationAnalysisResponse,
    EnhancedChartAnalysisRequest, EnhancedAnnotationAnalysisResponse,
    PortfolioInsightsResponse, ChatMessage, PortfolioChatResponse, ChatSuggestedAction,
    // Context loading from ai/context.rs
    load_portfolio_context,
    // Command parsing from ai/command_parser.rs
    parse_response_with_suggestions,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
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

/// Check if a model supports vision/image input.
///
/// Returns true if the model can process images, false otherwise.
/// This is used by the frontend to show/hide image upload UI.
#[command]
pub fn check_vision_support(model: String) -> bool {
    has_vision_support(&model)
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
/// Supports image attachments if the model has vision capability.
#[command]
pub async fn chat_with_portfolio_assistant(
    _app: AppHandle,
    request: PortfolioChatRequest,
) -> Result<PortfolioChatResponse, String> {
    // Check if any message has image attachments
    let has_images = request.messages.iter().any(|m| !m.attachments.is_empty());

    // Auto-upgrade deprecated models
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    // SECURITY: Check if model supports vision when images are attached
    if has_images && !has_vision_support(&model) {
        return Err(format!(
            "Das Modell '{}' unterstützt keine Bilder. Bitte wähle ein Vision-fähiges Modell wie Claude Sonnet, GPT-4o oder Gemini.",
            model
        ));
    }

    // Load portfolio context from database with user name
    // For chat, we always include technical signals (no progress events needed)
    let context = load_portfolio_context(&request.base_currency, request.user_name.clone(), true, None)?;

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

use crate::ai::types::{TransactionCreateCommand, PortfolioTransferCommand, TransactionDeleteCommand};
use crate::commands::crud::{create_transaction, delete_transaction, CreateTransactionRequest, TransactionUnitData};
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

/// Execute a confirmed transaction deletion
///
/// SECURITY: This command should only be called after explicit user confirmation.
#[command]
pub fn execute_confirmed_transaction_delete(
    app: AppHandle,
    payload: String,
) -> Result<String, String> {
    let cmd: TransactionDeleteCommand = serde_json::from_str(&payload)
        .map_err(|e| format!("Invalid delete payload: {}", e))?;

    // Delete the transaction
    delete_transaction(app.clone(), cmd.transaction_id)?;

    Ok(format!(
        "Transaktion #{} erfolgreich gelöscht",
        cmd.transaction_id
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

// ============================================================================
// Extracted Transactions Import (from image analysis)
// ============================================================================

/// Input for a single extracted transaction from image analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExtractedTransactionInput {
    pub date: String,
    pub txn_type: String,
    pub security_name: Option<String>,
    pub isin: Option<String>,
    pub shares: Option<f64>,
    pub gross_amount: Option<f64>,
    pub gross_currency: Option<String>,
    pub amount: Option<f64>,
    pub currency: String,
    pub fees: Option<f64>,
    pub fees_foreign: Option<f64>,
    pub fees_foreign_currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub taxes: Option<f64>,
    pub note: Option<String>,
}

/// Result of importing extracted transactions
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTransactionsResult {
    pub imported_count: usize,
    pub duplicates: Vec<String>,
    pub errors: Vec<String>,
}

/// Import error type to distinguish duplicates from other errors
enum ImportSingleError {
    Duplicate(String),
    Other(String),
}

/// Import extracted transactions from image analysis
///
/// SECURITY: This command should only be called after explicit user confirmation.
/// The transactions were extracted by AI from images and need user review before import.
///
/// portfolio_id: The portfolio to import transactions into (required for BUY/SELL/DELIVERY)
/// delivery_mode: When true, BUY becomes DELIVERY_INBOUND, SELL becomes DELIVERY_OUTBOUND
/// (same logic as PDF import - SSOT)
#[command]
pub fn import_extracted_transactions(
    app: AppHandle,
    transactions: Vec<ExtractedTransactionInput>,
    portfolio_id: Option<i64>,
    delivery_mode: bool,
) -> Result<ImportTransactionsResult, String> {
    if transactions.is_empty() {
        return Err("Keine Transaktionen zum Importieren".to_string());
    }

    let mut imported_count = 0;
    let mut duplicates: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for txn in &transactions {
        // Each transaction is imported separately to avoid holding DB connection
        // while calling create_transaction (which also needs the connection)
        match import_single_extracted_transaction(&app, txn, portfolio_id, delivery_mode) {
            Ok(_) => imported_count += 1,
            Err(ImportSingleError::Duplicate(msg)) => duplicates.push(msg),
            Err(ImportSingleError::Other(msg)) => errors.push(format!("{}: {}", txn.date, msg)),
        }
    }

    // NEVER throw Err() - always return a result with imported_count, duplicates, and errors
    // The frontend will handle displaying appropriate messages for each case
    // This allows duplicates and errors to be shown as friendly chat messages
    Ok(ImportTransactionsResult {
        imported_count,
        duplicates,
        errors,
    })
}

/// Get possible DB transaction types for duplicate detection.
/// Same logic as in pdf_import.rs - SSOT for duplicate check types.
fn get_duplicate_check_types_for_string(txn_type: &str) -> Vec<&'static str> {
    match txn_type.to_uppercase().as_str() {
        "BUY" => vec!["BUY", "DELIVERY_INBOUND"],
        "SELL" => vec!["SELL", "DELIVERY_OUTBOUND"],
        "DELIVERY_INBOUND" | "TRANSFER_IN" => vec!["DELIVERY_INBOUND", "BUY"],
        "DELIVERY_OUTBOUND" | "TRANSFER_OUT" => vec!["DELIVERY_OUTBOUND", "SELL"],
        "DIVIDENDS" | "DIVIDEND" => vec!["DIVIDENDS"],
        "INTEREST" => vec!["INTEREST"],
        "DEPOSIT" => vec!["DEPOSIT"],
        "REMOVAL" => vec!["REMOVAL"],
        _ => vec![],
    }
}

fn normalize_extracted_txn_type(raw: &str) -> String {
    let normalized = raw.trim().to_uppercase();
    let normalized = normalized.replace(' ', "_").replace('-', "_");

    match normalized.as_str() {
        "DIVIDEND" | "DIVIDENDS" | "DIVIDENDE" | "DIVIDENDEN" |
        "AUSSCHÜTTUNG" | "AUSSCHUETTUNG" | "ERTRAG" | "ERTRAGSGUTSCHRIFT" |
        "DIVIDENDENGUTSCHRIFT" => "DIVIDENDS".to_string(),
        "BUY" | "KAUF" => "BUY".to_string(),
        "SELL" | "VERKAUF" => "SELL".to_string(),
        "DELIVERY_INBOUND" | "EINLIEFERUNG" => "DELIVERY_INBOUND".to_string(),
        "DELIVERY_OUTBOUND" | "AUSLIEFERUNG" => "DELIVERY_OUTBOUND".to_string(),
        "TRANSFER_IN" | "UMBUCHUNG_EIN" | "UMBUCHUNG_EINGANG" => "TRANSFER_IN".to_string(),
        "TRANSFER_OUT" | "UMBUCHUNG_AUS" | "UMBUCHUNG_AUSGANG" => "TRANSFER_OUT".to_string(),
        "DEPOSIT" | "EINZAHLUNG" | "EINLAGE" => "DEPOSIT".to_string(),
        "REMOVAL" | "AUSZAHLUNG" | "ENTNAHME" => "REMOVAL".to_string(),
        "INTEREST" | "ZINS" | "ZINSEN" => "INTEREST".to_string(),
        "FEES" | "FEE" | "GEBUEHREN" | "GEBÜHREN" => "FEES".to_string(),
        "TAXES" | "TAX" | "STEUERN" => "TAXES".to_string(),
        _ => normalized,
    }
}

/// Convert transaction type based on delivery mode (same logic as PDF import - SSOT)
/// Always normalizes to uppercase for consistency.
fn apply_delivery_mode(txn_type: &str, delivery_mode: bool) -> String {
    let normalized = normalize_extracted_txn_type(txn_type);
    if !delivery_mode {
        return normalized;
    }
    match normalized.as_str() {
        "BUY" => "DELIVERY_INBOUND".to_string(),
        "SELL" => "DELIVERY_OUTBOUND".to_string(),
        _ => normalized,
    }
}

/// Import a single extracted transaction
fn import_single_extracted_transaction(
    app: &AppHandle,
    txn: &ExtractedTransactionInput,
    portfolio_id: Option<i64>,
    delivery_mode: bool,
) -> Result<i64, ImportSingleError> {
    use crate::db;

    let normalized_date = normalize_extracted_date(&txn.date, &txn.currency);

    // Apply delivery mode conversion (BUY → DELIVERY_INBOUND, SELL → DELIVERY_OUTBOUND)
    let effective_txn_type = apply_delivery_mode(&txn.txn_type, delivery_mode);

    // First phase: gather all needed IDs from DB, then drop connection
    let (security_id, owner_type, owner_id) = {
        let conn_guard = db::get_connection()
            .map_err(|e| ImportSingleError::Other(format!("Datenbankfehler: {}", e)))?;
        let conn = conn_guard.as_ref()
            .ok_or_else(|| ImportSingleError::Other("Datenbank nicht initialisiert".to_string()))?;

        // Check if this transaction type requires a security
        let requires_security = is_portfolio_transaction(&effective_txn_type)
            || effective_txn_type == "DIVIDENDS";

        // Find security using the improved fuzzy-matching function
        // This handles: ISIN match, exact name, partial name, accent-normalized name,
        // multi-word fuzzy match (e.g., "LVMH" + "Vuitton" -> "LVMH Moët Henn. L. Vuitton")
        let security_id = find_security_id(conn, &txn.isin, &txn.security_name);

        // Report error if security required but not found
        if security_id.is_none() && requires_security {
            let identifier = txn.isin.as_ref()
                .filter(|i| i.len() >= 10)
                .map(|i| format!("ISIN {}", i))
                .or_else(|| txn.security_name.as_ref().map(|n| format!("'{}'", n)))
                .unwrap_or_else(|| "ohne Kennung".to_string());

            return Err(ImportSingleError::Other(format!(
                "Wertpapier {} nicht gefunden. Bitte zuerst anlegen.",
                identifier
            )));
        }

        // Determine owner (portfolio or account)
        let (owner_type, owner_id) = if is_portfolio_transaction(&effective_txn_type) {
            // Use provided portfolio_id or fall back to first portfolio
            let final_portfolio_id = if let Some(pid) = portfolio_id {
                // Verify the portfolio exists and is not retired
                let mut stmt = conn.prepare(
                    "SELECT id FROM pp_portfolio WHERE id = ?1 AND is_retired = 0"
                ).map_err(|e| ImportSingleError::Other(format!("SQL Fehler: {}", e)))?;

                stmt.query_row([pid], |row| row.get::<_, i64>(0))
                    .map_err(|_| ImportSingleError::Other(format!("Depot mit ID {} nicht gefunden oder inaktiv", pid)))?
            } else {
                // Fallback: Get first portfolio
                let mut stmt = conn.prepare(
                    "SELECT id FROM pp_portfolio WHERE is_retired = 0 LIMIT 1"
                ).map_err(|e| ImportSingleError::Other(format!("SQL Fehler: {}", e)))?;

                stmt.query_row([], |row| row.get(0))
                    .map_err(|_| ImportSingleError::Other("Kein aktives Depot gefunden".to_string()))?
            };

            ("portfolio".to_string(), final_portfolio_id)
        } else {
            // Get first account matching currency
            let mut stmt = conn.prepare(
                "SELECT id FROM pp_account WHERE currency = ?1 AND is_retired = 0 LIMIT 1"
            ).map_err(|e| ImportSingleError::Other(format!("SQL Fehler: {}", e)))?;

            let account_id: Result<i64, _> = stmt.query_row([&txn.currency], |row| row.get(0));

            if let Ok(id) = account_id {
                ("account".to_string(), id)
            } else {
                // Fallback to any account
                let mut stmt = conn.prepare(
                    "SELECT id FROM pp_account WHERE is_retired = 0 LIMIT 1"
                ).map_err(|e| ImportSingleError::Other(format!("SQL Fehler: {}", e)))?;

                let account_id: i64 = stmt.query_row([], |row| row.get(0))
                    .map_err(|_| ImportSingleError::Other("Kein aktives Konto gefunden".to_string()))?;

                ("account".to_string(), account_id)
            }
        };

        // Duplicate detection (same logic as pdf_import.rs - SSOT)
        if let Some(sec_id) = security_id {
            let effective_amount = if effective_txn_type == "DIVIDENDS" {
                let gross = compute_dividend_gross_amount(txn);
                log::debug!(
                    "Dividend duplicate check: gross_amount={:?}, taxes={:?}, net_amount={:?}, computed_gross={:?}, using={:?}",
                    txn.gross_amount, txn.taxes, txn.amount, gross, gross.or(txn.amount)
                );
                gross.or(txn.amount)
            } else {
                txn.amount
            };
            let amount_cents = effective_amount.map(|a| (a * 100.0).round() as i64).unwrap_or(0);
            let txn_types = get_duplicate_check_types_for_string(&effective_txn_type);

            if !txn_types.is_empty() {
                // Use date prefix for LIKE comparison (handles time differences)
                let date_pattern = format!("{}%", &normalized_date);
                let type_placeholders = txn_types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 4))
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    r#"
                    SELECT 1 FROM pp_txn
                    WHERE security_id = ?1
                      AND date LIKE ?2
                      AND ABS(amount - ?3) <= 1
                      AND txn_type IN ({})
                    LIMIT 1
                    "#,
                    type_placeholders
                );

                let mut params: Vec<&dyn rusqlite::ToSql> =
                    vec![&sec_id, &date_pattern, &amount_cents];
                for t in &txn_types {
                    params.push(t);
                }

                let is_duplicate: bool = conn
                    .query_row(&sql, params.as_slice(), |_| Ok(true))
                    .unwrap_or(false);

                if is_duplicate {
                    return Err(ImportSingleError::Duplicate(format!(
                        "{} {} vom {} bereits vorhanden",
                        txn.txn_type,
                        txn.security_name.as_deref().unwrap_or("Unbekannt"),
                        normalized_date
                    )));
                }
            }
        }

        (security_id, owner_type, owner_id)
    }; // conn_guard is dropped here

    // Scale amounts (amount and fees are in decimal, need to convert to scaled integers)
    let amount_scaled = if effective_txn_type == "DIVIDENDS" {
        compute_dividend_gross_amount(txn)
            .or(txn.amount)
            .map(|a| (a * 100.0).round() as i64)
            .unwrap_or(0)
    } else {
        txn.amount.map(|a| (a * 100.0).round() as i64).unwrap_or(0)
    };

    // For DIVIDENDS without shares: calculate shares from current holdings
    // Formula: gross_amount / shares = dividend_per_share
    // So we need to look up current holdings and store them as shares
    let shares_input = txn.shares.filter(|s| *s > 0.0);
    let shares_scaled = if effective_txn_type == "DIVIDENDS" && shares_input.is_none() {
        // Look up current holdings for this security
        if let Some(sec_id) = security_id {
            let holdings_shares = {
                let conn_guard = db::get_connection()
                    .map_err(|e| ImportSingleError::Other(format!("Datenbankfehler: {}", e)))?;
                let conn = conn_guard.as_ref()
                    .ok_or_else(|| ImportSingleError::Other("Datenbank nicht initialisiert".to_string()))?;

                // Get total holdings for this security across all portfolios (scaled by 10^8)
                let sql = r#"
                    SELECT COALESCE(SUM(CASE
                        WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                        WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                        ELSE 0
                    END), 0) as total_shares
                    FROM pp_txn
                    WHERE security_id = ?1
                      AND owner_type = 'portfolio'
                      AND date <= ?2
                "#;

                conn.query_row(sql, rusqlite::params![sec_id, &normalized_date], |row| row.get::<_, i64>(0))
                    .unwrap_or(0)
            };

            if holdings_shares > 0 {
                log::info!(
                    "DIVIDENDS without shares: using current holdings {} for security_id {}",
                    holdings_shares as f64 / 100_000_000.0,
                    sec_id
                );
                Some(holdings_shares)
            } else {
                log::warn!(
                    "DIVIDENDS without shares: no holdings found for security_id {} at date {}",
                    sec_id,
                    normalized_date
                );
                None
            }
        } else {
            None
        }
    } else {
        shares_input.map(|s| (s * 100_000_000.0).round() as i64)
    };

    // Build units for fees and taxes
    let mut units: Vec<TransactionUnitData> = Vec::new();
    let total_fees = normalize_extracted_fees(txn);
    if let Some(fees) = total_fees {
        if fees > 0.0 {
            units.push(TransactionUnitData {
                unit_type: "FEE".to_string(),
                amount: (fees * 100.0).round() as i64,
                currency: txn.currency.clone(),
                forex_amount: None,
                forex_currency: None,
                exchange_rate: None,
            });
        }
    }
    if let Some(taxes) = txn.taxes {
        if taxes > 0.0 {
            units.push(TransactionUnitData {
                unit_type: "TAX".to_string(),
                amount: (taxes * 100.0).round() as i64,
                currency: txn.currency.clone(),
                forex_amount: None,
                forex_currency: None,
                exchange_rate: None,
            });
        }
    }

    // Build note
    let mut note_parts: Vec<String> = Vec::new();
    if let Some(original_note) = &txn.note {
        note_parts.push(original_note.clone());
    }
    note_parts.push("Aus Bild-Erkennung importiert".to_string());
    let note = Some(note_parts.join(" | "));

    // Second phase: create the transaction (this will get its own DB connection)
    let request = CreateTransactionRequest {
        owner_type,
        owner_id,
        txn_type: effective_txn_type.clone(),
        date: normalized_date,
        amount: amount_scaled,
        currency: txn.currency.clone(),
        shares: shares_scaled,
        security_id,
        note,
        units: if units.is_empty() { None } else { Some(units) },
        reference_account_id: None,
    };

    let result = create_transaction(app.clone(), request)
        .map_err(|e| ImportSingleError::Other(format!("Fehler: {}", e)))?;

    Ok(result.id)
}

fn normalize_extracted_fees(txn: &ExtractedTransactionInput) -> Option<f64> {
    let mut total = txn.fees.unwrap_or(0.0);

    if let Some(foreign) = txn.fees_foreign {
        if foreign > 0.0 {
            if let Some(foreign_currency) = txn.fees_foreign_currency.as_ref() {
                // Currency explicitly specified
                if foreign_currency.eq_ignore_ascii_case(&txn.currency) {
                    // Same currency -> no conversion needed
                    total += foreign;
                } else if let Some(rate) = txn.exchange_rate {
                    // Different currency -> convert with exchange rate
                    total += foreign * rate;
                }
            } else {
                // NO fees_foreign_currency specified
                // Check if transaction itself has foreign currency
                if let Some(gross_currency) = txn.gross_currency.as_ref() {
                    if !gross_currency.eq_ignore_ascii_case(&txn.currency) {
                        // Transaction has foreign currency -> fees_foreign is in that currency
                        if let Some(rate) = txn.exchange_rate {
                            total += foreign * rate;
                        }
                    } else {
                        // gross_currency == txn.currency -> fees_foreign is already in base currency
                        total += foreign;
                    }
                } else {
                    // No gross_currency -> assume fees_foreign is already in txn.currency
                    // This handles AutoFX fees from Trade Republic (already in EUR)
                    total += foreign;
                }
            }
        }
    }

    if total > 0.0 {
        Some(total)
    } else {
        None
    }
}

fn compute_dividend_gross_amount(txn: &ExtractedTransactionInput) -> Option<f64> {
    if let Some(gross) = txn.gross_amount {
        if let Some(gross_currency) = txn.gross_currency.as_ref() {
            if gross_currency.eq_ignore_ascii_case(&txn.currency) {
                return Some(gross);
            }
            if let Some(rate) = txn.exchange_rate {
                return Some(gross * rate);
            }
        } else {
            return Some(gross);
        }
    }

    if let (Some(net), Some(taxes)) = (txn.amount, txn.taxes) {
        if net > 0.0 && taxes >= 0.0 {
            return Some(net + taxes);
        }
    }

    None
}

fn normalize_extracted_date(raw: &str, currency: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return raw.to_string();
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return date.format("%Y-%m-%d").to_string();
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y/%m/%d") {
        return date.format("%Y-%m-%d").to_string();
    }
    if let Ok(date) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S%.f") {
        return date.date().format("%Y-%m-%d").to_string();
    }
    if let Ok(date) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return date.date().format("%Y-%m-%d").to_string();
    }

    let prefer_mdy = currency.eq_ignore_ascii_case("USD");
    if let Some(date) = parse_numeric_date_with_sep(trimmed, '.', prefer_mdy) {
        return date.format("%Y-%m-%d").to_string();
    }
    if let Some(date) = parse_numeric_date_with_sep(trimmed, '/', prefer_mdy) {
        return date.format("%Y-%m-%d").to_string();
    }
    if let Some(date) = parse_numeric_date_with_sep(trimmed, '-', prefer_mdy) {
        return date.format("%Y-%m-%d").to_string();
    }
    if let Some(date) = parse_month_name_date(trimmed) {
        return date.format("%Y-%m-%d").to_string();
    }

    raw.to_string()
}

fn parse_numeric_date_with_sep(s: &str, sep: char, prefer_mdy: bool) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split(sep).map(|p| p.trim()).collect();
    if parts.len() != 3 {
        return None;
    }

    let (p0, p1, p2) = (parts[0], parts[1], parts[2]);
    if p0.len() == 4 {
        let year = p0.parse::<i32>().ok()?;
        let month = p1.parse::<u32>().ok()?;
        let day = p2.parse::<u32>().ok()?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }
    if p2.len() == 4 {
        let year = p2.parse::<i32>().ok()?;
        let a = p0.parse::<u32>().ok()?;
        let b = p1.parse::<u32>().ok()?;
        let (month, day) = if a > 12 && b <= 12 {
            (b, a)
        } else if b > 12 && a <= 12 {
            (a, b)
        } else if prefer_mdy {
            (a, b)
        } else {
            (b, a)
        };
        return NaiveDate::from_ymd_opt(year, month, day);
    }

    None
}

fn parse_month_name_date(s: &str) -> Option<NaiveDate> {
    let cleaned = s
        .to_lowercase()
        .replace(',', " ")
        .replace('.', " ");
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }

    if let (Ok(day), Some(month), Ok(year)) = (
        tokens[0].parse::<u32>(),
        month_name_to_number(tokens[1]),
        tokens[2].parse::<i32>(),
    ) {
        return NaiveDate::from_ymd_opt(year, month, day);
    }

    if let (Some(month), Ok(day), Ok(year)) = (
        month_name_to_number(tokens[0]),
        tokens[1].parse::<u32>(),
        tokens[2].parse::<i32>(),
    ) {
        return NaiveDate::from_ymd_opt(year, month, day);
    }

    None
}

fn month_name_to_number(s: &str) -> Option<u32> {
    match s {
        "jan" | "januar" | "january" => Some(1),
        "feb" | "februar" | "february" => Some(2),
        "mar" | "mär" | "maerz" | "märz" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" | "mai" => Some(5),
        "jun" | "juni" | "june" => Some(6),
        "jul" | "juli" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "okt" | "oktober" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "dez" | "dezember" | "december" => Some(12),
        _ => None,
    }
}

// ============================================================================
// Transaction Enrichment (for preview with holdings data)
// ============================================================================

/// Output for enriched transaction (with shares from holdings for dividends)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EnrichedTransactionOutput {
    pub date: String,
    pub txn_type: String,
    pub security_name: Option<String>,
    pub isin: Option<String>,
    pub shares: Option<f64>,
    pub shares_from_holdings: bool,
    pub gross_amount: Option<f64>,
    pub gross_currency: Option<String>,
    pub amount: Option<f64>,
    pub currency: String,
    pub fees: Option<f64>,
    pub fees_foreign: Option<f64>,
    pub fees_foreign_currency: Option<String>,
    pub exchange_rate: Option<f64>,
    pub taxes: Option<f64>,
    pub note: Option<String>,
}

/// Enrich extracted transactions with holdings data
///
/// For DIVIDEND transactions without shares, this looks up current holdings
/// and calculates the shares. This allows the preview to show the shares
/// BEFORE the actual import happens.
#[command]
pub fn enrich_extracted_transactions(
    transactions: Vec<ExtractedTransactionInput>,
) -> Result<Vec<EnrichedTransactionOutput>, String> {
    use crate::db;

    let conn_guard = db::get_connection()
        .map_err(|e| format!("Datenbankfehler: {}", e))?;
    let conn = conn_guard.as_ref()
        .ok_or_else(|| "Datenbank nicht initialisiert".to_string())?;

    let mut enriched: Vec<EnrichedTransactionOutput> = Vec::new();

    for txn in &transactions {
        let normalized_date = normalize_extracted_date(&txn.date, &txn.currency);
        let effective_txn_type = normalize_extracted_txn_type(&txn.txn_type);
        let shares_input = txn.shares.filter(|s| *s > 0.0);

        // Check if this is a DIVIDEND without shares
        let is_dividend_without_shares = effective_txn_type == "DIVIDENDS" && shares_input.is_none();

        let (shares, shares_from_holdings) = if is_dividend_without_shares {
            // Try to find security and look up holdings
            let security_id = find_security_id(conn, &txn.isin, &txn.security_name);

            if let Some(sec_id) = security_id {
                // Get total holdings for this security at the dividend date
                let sql = r#"
                    SELECT COALESCE(SUM(CASE
                        WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                        WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                        ELSE 0
                    END), 0) as total_shares
                    FROM pp_txn
                    WHERE security_id = ?1
                      AND owner_type = 'portfolio'
                      AND date <= ?2
                "#;

                let holdings_shares: i64 = conn
                    .query_row(sql, rusqlite::params![sec_id, &normalized_date], |row| row.get(0))
                    .unwrap_or(0);

                if holdings_shares > 0 {
                    // Convert from scaled (10^8) to decimal
                    let shares_decimal = holdings_shares as f64 / 100_000_000.0;
                    log::info!(
                        "Enriched DIVIDEND with holdings: {} shares for security_id {} at {}",
                        shares_decimal,
                        sec_id,
                        normalized_date
                    );
                    (Some(shares_decimal), true)
                } else {
                    log::warn!(
                        "No holdings found for DIVIDEND security_id {} at {}",
                        sec_id,
                        normalized_date
                    );
                    (None, false)
                }
            } else {
                log::info!(
                    "Security lookup for DIVIDEND enrichment failed: isin={:?}, name={:?} -> not found in database",
                    txn.isin,
                    txn.security_name
                );
                (None, false)
            }
        } else {
            (shares_input, false)
        };

        enriched.push(EnrichedTransactionOutput {
            date: txn.date.clone(),
            txn_type: txn.txn_type.clone(),
            security_name: txn.security_name.clone(),
            isin: txn.isin.clone(),
            shares,
            shares_from_holdings,
            gross_amount: txn.gross_amount,
            gross_currency: txn.gross_currency.clone(),
            amount: txn.amount,
            currency: txn.currency.clone(),
            fees: txn.fees,
            fees_foreign: txn.fees_foreign,
            fees_foreign_currency: txn.fees_foreign_currency.clone(),
            exchange_rate: txn.exchange_rate,
            taxes: txn.taxes,
            note: txn.note.clone(),
        });
    }

    Ok(enriched)
}

// ============================================================================
// Duplicate Detection (for preview - before import)
// ============================================================================

/// Result of checking a single transaction for duplicates
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCheckResult {
    pub index: usize,
    pub is_duplicate: bool,
    pub message: Option<String>,
}

/// Result of checking all transactions for duplicates
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCheckResponse {
    pub results: Vec<DuplicateCheckResult>,
    pub all_duplicates: bool,
    pub duplicate_count: usize,
}

/// Check extracted transactions for duplicates BEFORE showing preview
///
/// This allows the frontend to filter out duplicates before displaying
/// the transaction preview, or show a message that all are duplicates.
#[command]
pub fn check_extracted_transactions_for_duplicates(
    transactions: Vec<ExtractedTransactionInput>,
) -> Result<DuplicateCheckResponse, String> {
    use crate::db;

    let conn_guard = db::get_connection()
        .map_err(|e| format!("Datenbankfehler: {}", e))?;
    let conn = conn_guard.as_ref()
        .ok_or_else(|| "Datenbank nicht initialisiert".to_string())?;

    let mut results: Vec<DuplicateCheckResult> = Vec::new();
    let mut duplicate_count = 0;

    for (index, txn) in transactions.iter().enumerate() {
        let normalized_date = normalize_extracted_date(&txn.date, &txn.currency);
        let effective_txn_type = normalize_extracted_txn_type(&txn.txn_type);

        // Find security ID
        let security_id = find_security_id(conn, &txn.isin, &txn.security_name);

        // Check for duplicate if security found
        let (is_duplicate, message) = if let Some(sec_id) = security_id {
            let effective_amount = if effective_txn_type == "DIVIDENDS" {
                compute_dividend_gross_amount(txn).or(txn.amount)
            } else {
                txn.amount
            };
            let amount_cents = effective_amount.map(|a| (a * 100.0).round() as i64).unwrap_or(0);
            let txn_types = get_duplicate_check_types_for_string(&effective_txn_type);

            if !txn_types.is_empty() {
                let date_pattern = format!("{}%", &normalized_date);
                let type_placeholders = txn_types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 4))
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    r#"
                    SELECT 1 FROM pp_txn
                    WHERE security_id = ?1
                      AND date LIKE ?2
                      AND ABS(amount - ?3) <= 1
                      AND txn_type IN ({})
                    LIMIT 1
                    "#,
                    type_placeholders
                );

                let mut params: Vec<&dyn rusqlite::ToSql> =
                    vec![&sec_id, &date_pattern, &amount_cents];
                for t in &txn_types {
                    params.push(t);
                }

                let is_dup: bool = conn
                    .query_row(&sql, params.as_slice(), |_| Ok(true))
                    .unwrap_or(false);

                if is_dup {
                    let msg = format!(
                        "{} {} vom {} bereits vorhanden",
                        txn.txn_type,
                        txn.security_name.as_deref().unwrap_or("Unbekannt"),
                        normalized_date
                    );
                    (true, Some(msg))
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        } else {
            // No security found - not a duplicate (will fail during import with different error)
            (false, None)
        };

        if is_duplicate {
            duplicate_count += 1;
        }

        results.push(DuplicateCheckResult {
            index,
            is_duplicate,
            message,
        });
    }

    let all_duplicates = duplicate_count == transactions.len() && !transactions.is_empty();

    Ok(DuplicateCheckResponse {
        results,
        all_duplicates,
        duplicate_count,
    })
}

/// Normalize string for fuzzy matching: lowercase, remove accents, remove punctuation
fn normalize_for_search(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ë' | 'é' | 'è' | 'ê' | 'ē' => 'e',
            'ä' | 'á' | 'à' | 'â' | 'ā' => 'a',
            'ö' | 'ó' | 'ò' | 'ô' | 'ō' => 'o',
            'ü' | 'ú' | 'ù' | 'û' | 'ū' => 'u',
            'ï' | 'í' | 'ì' | 'î' | 'ī' => 'i',
            'ß' => 's',
            'ñ' => 'n',
            '.' | ',' | '-' | '&' | '\'' => ' ', // Replace punctuation with space
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

/// Extract significant words from name (skip common words, keep brand names)
fn extract_significant_words(name: &str) -> Vec<String> {
    let skip_words = ["inc", "corp", "ag", "sa", "se", "plc", "ltd", "gmbh", "co", "kg", "the", "and", "of"];
    normalize_for_search(name)
        .split_whitespace()
        .filter(|w| w.len() >= 3 && !skip_words.contains(&w.to_lowercase().as_str()))
        .map(|s| s.to_string())
        .collect()
}

/// Helper to find security ID by ISIN, WKN, ticker, or name
fn find_security_id(conn: &rusqlite::Connection, isin: &Option<String>, name: &Option<String>) -> Option<i64> {
    // First try by ISIN (most reliable)
    if let Some(isin_val) = isin {
        if isin_val.len() >= 10 && !isin_val.to_lowercase().contains("nicht") {
            let result: Option<i64> = conn
                .query_row(
                    "SELECT id FROM pp_security WHERE isin = ?1 LIMIT 1",
                    [isin_val],
                    |row| row.get(0),
                )
                .ok();

            if result.is_some() {
                log::debug!("find_security_id: Found by ISIN {}", isin_val);
                return result;
            }
        }
    }

    // Then try by name with multiple strategies
    if let Some(name_val) = name {
        // Strategy 1: Exact match (case-insensitive)
        let name_lower = name_val.to_lowercase();
        let result: Option<i64> = conn
            .query_row(
                "SELECT id FROM pp_security WHERE LOWER(name) = ?1 LIMIT 1",
                [&name_lower],
                |row| row.get(0),
            )
            .ok();
        if result.is_some() {
            log::debug!("find_security_id: Found by exact name match '{}'", name_val);
            return result;
        }

        // Strategy 2: Contains the full name
        let search_name = format!("%{}%", name_val);
        let result: Option<i64> = conn
            .query_row(
                "SELECT id FROM pp_security WHERE name LIKE ?1 LIMIT 1",
                [&search_name],
                |row| row.get(0),
            )
            .ok();
        if result.is_some() {
            log::debug!("find_security_id: Found by name contains '{}'", name_val);
            return result;
        }

        // Strategy 3: First word match (e.g., "LVMH" from "LVMH Moët Hennessy...")
        let first_word = name_val.split_whitespace().next().unwrap_or(name_val);
        if first_word.len() >= 3 {
            let search_first = format!("{}%", first_word);
            let result: Option<i64> = conn
                .query_row(
                    "SELECT id FROM pp_security WHERE name LIKE ?1 LIMIT 1",
                    [&search_first],
                    |row| row.get(0),
                )
                .ok();
            if result.is_some() {
                log::debug!("find_security_id: Found by first word '{}' of '{}'", first_word, name_val);
                return result;
            }
        }

        // Strategy 4: Normalized first word (without accents)
        let normalized_first = normalize_for_search(first_word);
        if normalized_first.len() >= 3 && normalized_first != first_word.to_lowercase() {
            // Load all securities and check normalized names
            let mut stmt = conn
                .prepare("SELECT id, name FROM pp_security WHERE is_retired = 0")
                .ok()?;
            let mut rows = stmt.query([]).ok()?;
            while let Some(row) = rows.next().ok()? {
                let id: i64 = row.get(0).ok()?;
                let db_name: String = row.get(1).ok()?;
                let db_normalized = normalize_for_search(&db_name);
                if db_normalized.starts_with(&normalized_first) {
                    log::debug!("find_security_id: Found by normalized first word '{}' -> '{}'", first_word, db_name);
                    return Some(id);
                }
            }
        }

        // Strategy 5: Multi-word fuzzy match (LVMH + Vuitton both in name)
        let significant_words = extract_significant_words(name_val);
        if significant_words.len() >= 2 {
            // Use first and last significant words
            let first = &significant_words[0];
            let last = &significant_words[significant_words.len() - 1];

            let mut stmt = conn
                .prepare("SELECT id, name FROM pp_security WHERE is_retired = 0")
                .ok()?;
            let mut rows = stmt.query([]).ok()?;
            while let Some(row) = rows.next().ok()? {
                let id: i64 = row.get(0).ok()?;
                let db_name: String = row.get(1).ok()?;
                let db_normalized = normalize_for_search(&db_name);

                // Check if both first and last words appear in DB name
                if db_normalized.contains(first) && db_normalized.contains(last) {
                    log::debug!(
                        "find_security_id: Found by multi-word match '{}' + '{}' in '{}'",
                        first, last, db_name
                    );
                    return Some(id);
                }
            }
        }

        // Strategy 6: Any significant word with high confidence (brand names)
        // "LVMH" alone should match "LVMH Moët Henn. L. Vuitton"
        for word in &significant_words {
            if word.len() >= 4 {
                let search_pattern = format!("%{}%", word);
                let result: Option<i64> = conn
                    .query_row(
                        "SELECT id FROM pp_security WHERE LOWER(name) LIKE LOWER(?1) LIMIT 1",
                        [&search_pattern],
                        |row| row.get(0),
                    )
                    .ok();
                if result.is_some() {
                    log::debug!("find_security_id: Found by significant word '{}' from '{}'", word, name_val);
                    return result;
                }
            }
        }

        log::debug!("find_security_id: No match found for name '{}' (words: {:?})", name_val, significant_words);
    }

    log::debug!("find_security_id: No security found for isin={:?}, name={:?}", isin, name);
    None
}

// ============================================================================
// Speech-to-Text (Whisper API)
// ============================================================================

/// Response from Whisper API
#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
}

/// Transcribe audio using OpenAI Whisper API
///
/// Takes base64-encoded audio data (webm, mp3, wav, etc.) and returns transcribed text.
#[command]
pub async fn transcribe_audio(
    audio_base64: String,
    api_key: String,
    language: Option<String>,
) -> Result<String, String> {
    use base64::Engine;
    use reqwest::multipart;

    // Decode base64 audio
    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(&audio_base64)
        .map_err(|e| format!("Failed to decode audio: {}", e))?;

    // Create multipart form with audio file
    let audio_part = multipart::Part::bytes(audio_bytes)
        .file_name("audio.webm")
        .mime_str("audio/webm")
        .map_err(|e| format!("Failed to create audio part: {}", e))?;

    let mut form = multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", audio_part);

    // Add language hint if provided (improves accuracy)
    if let Some(lang) = language {
        form = form.text("language", lang);
    }

    // Call OpenAI Whisper API
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Whisper API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Whisper API error ({}): {}", status, error_text));
    }

    let whisper_response: WhisperResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Whisper response: {}", e))?;

    Ok(whisper_response.text)
}
