//! Perplexity Sonar API provider for chart analysis
//!
//! Perplexity's Sonar models combine real-time web search with vision capabilities.
//! API format is OpenAI-compatible.

use super::{
    build_analysis_prompt, AiError, AiErrorKind, ChartAnalysisResponse, ChartContext,
    get_fallback_model, parse_retry_delay, calculate_backoff_delay, normalize_markdown_response,
    REQUEST_TIMEOUT_SECS, MAX_RETRIES, MAX_TOKENS,
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_URL: &str = "https://api.perplexity.ai/chat/completions";

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: Vec<ContentPart>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    total_tokens: u32,
}

/// Parse Perplexity API error response
fn parse_error(status: u16, body: &str, model: &str) -> AiError {
    let fallback = get_fallback_model("perplexity", model);
    let body_lower = body.to_lowercase();

    match status {
        429 => {
            if body_lower.contains("quota") || body_lower.contains("billing") ||
               body_lower.contains("exceeded") || body_lower.contains("limit") {
                AiError::quota_exceeded("Perplexity", model, fallback)
            } else {
                let retry_after = parse_retry_delay(body);
                AiError::rate_limit("Perplexity", model, retry_after)
            }
        }
        401 => AiError::invalid_api_key("Perplexity", model),
        403 => {
            if body_lower.contains("permission") || body_lower.contains("access") {
                AiError::invalid_api_key("Perplexity", model)
            } else {
                AiError::other("Perplexity", model, "Zugriff verweigert")
            }
        }
        404 => AiError::model_not_found("Perplexity", model, fallback),
        500..=599 => AiError::server_error("Perplexity", model, &format!("HTTP {}", status)),
        _ => AiError::other("Perplexity", model, &format!("HTTP {}: {}",
            status,
            if body.len() > 200 { &body[..200] } else { body }
        )),
    }
}

/// Check if error is retryable
fn is_retryable(err: &AiError) -> bool {
    matches!(err.kind, AiErrorKind::RateLimit | AiErrorKind::ServerError | AiErrorKind::NetworkError)
}

/// Analyze a chart image using Perplexity Sonar with retry logic
pub async fn analyze(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<ChartAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("Perplexity", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Create client with timeout and connection pooling
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Perplexity", model, &e.to_string()))?;

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text {
                    text: build_analysis_prompt(context, model),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        // Perplexity accepts base64 data URLs like OpenAI
                        url: format!("data:image/jpeg;base64,{}", image_base64),
                    },
                },
            ],
        }],
    };

    // Retry loop with exponential backoff
    let mut last_error = AiError::other("Perplexity", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Perplexity", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Perplexity", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Perplexity", model, &e.to_string())
                };

                if attempt < MAX_RETRIES && is_retryable(&last_error) {
                    continue;
                }
                return Err(last_error);
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            last_error = parse_error(status.as_u16(), &body, model);

            if attempt < MAX_RETRIES && is_retryable(&last_error) {
                continue;
            }
            return Err(last_error);
        }

        // Success - parse response
        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Perplexity", model, &format!("JSON parse error: {}", e)))?;

        let raw_analysis = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        // Normalize markdown formatting for consistent display
        let analysis = normalize_markdown_response(&raw_analysis);

        return Ok(ChartAnalysisResponse {
            analysis,
            provider: "Perplexity".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
        });
    }

    Err(last_error)
}
