//! AI chart analysis commands

use crate::ai::{
    claude, gemini, openai, perplexity,
    list_claude_models, list_openai_models, list_gemini_models, list_perplexity_models,
    get_model_upgrade,
    AiModelInfo, AiError, ChartAnalysisRequest, ChartAnalysisResponse,
};
use tauri::command;

/// Analyze a chart using AI
///
/// Returns ChartAnalysisResponse on success, or a JSON-serialized AiError on failure.
/// Automatically upgrades deprecated models to their replacements.
#[command]
pub async fn analyze_chart_with_ai(
    request: ChartAnalysisRequest,
) -> Result<ChartAnalysisResponse, String> {
    // Check if the model is deprecated and auto-upgrade
    let model = if let Some(upgraded) = get_model_upgrade(&request.provider, &request.model) {
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
