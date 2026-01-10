//! AI chart analysis command

use crate::ai::{claude, gemini, openai, ChartAnalysisRequest, ChartAnalysisResponse};
use tauri::command;

/// Analyze a chart using AI
#[command]
pub async fn analyze_chart_with_ai(
    request: ChartAnalysisRequest,
) -> Result<ChartAnalysisResponse, String> {
    match request.provider.as_str() {
        "claude" => claude::analyze(&request.image_base64, &request.api_key, &request.context)
            .await
            .map_err(|e| e.to_string()),
        "openai" => openai::analyze(&request.image_base64, &request.api_key, &request.context)
            .await
            .map_err(|e| e.to_string()),
        "gemini" => gemini::analyze(&request.image_base64, &request.api_key, &request.context)
            .await
            .map_err(|e| e.to_string()),
        _ => Err(format!("Unknown AI provider: {}", request.provider)),
    }
}
