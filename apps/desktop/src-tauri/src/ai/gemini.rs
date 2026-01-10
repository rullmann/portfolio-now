//! Google Gemini API provider for chart analysis

use super::{
    build_analysis_prompt, AiError, AiErrorKind, ChartAnalysisResponse, ChartContext,
    get_fallback_model, parse_retry_delay, calculate_backoff_delay, normalize_markdown_response,
    REQUEST_TIMEOUT_SECS, MAX_RETRIES,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

fn api_url(model: &str, api_key: &str) -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    )
}

#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Option<Vec<ResponsePart>>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct UsageMetadata {
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
}

/// Parse Gemini API error response
fn parse_error(status: u16, body: &str, model: &str) -> AiError {
    let fallback = get_fallback_model("gemini", model);

    match status {
        429 => {
            // Check if it's quota exceeded vs rate limit
            let body_lower = body.to_lowercase();
            if body_lower.contains("quota") || body_lower.contains("exceeded") ||
               body_lower.contains("resource_exhausted") {
                AiError::quota_exceeded("Gemini", model, fallback)
            } else {
                let retry_after = parse_retry_delay(body);
                AiError::rate_limit("Gemini", model, retry_after)
            }
        }
        401 | 403 => AiError::invalid_api_key("Gemini", model),
        404 => AiError::model_not_found("Gemini", model, fallback),
        500..=599 => AiError::server_error("Gemini", model, &format!("HTTP {}", status)),
        _ => AiError::other("Gemini", model, &format!("HTTP {}: {}", status,
            if body.len() > 200 { &body[..200] } else { body }
        )),
    }
}

/// Check if error is retryable
fn is_retryable(err: &AiError) -> bool {
    matches!(err.kind, AiErrorKind::RateLimit | AiErrorKind::ServerError | AiErrorKind::NetworkError)
}

/// Analyze a chart image using Google Gemini with retry logic
pub async fn analyze(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<ChartAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Create client with timeout and connection pooling
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Gemini", model, &e.to_string()))?;

    let request_body = GenerateContentRequest {
        contents: vec![Content {
            role: "user".to_string(),
            parts: vec![
                Part::Text {
                    text: build_analysis_prompt(context, model),
                },
                Part::InlineData {
                    inline_data: InlineData {
                        // Gemini accepts JPEG for optimized images
                        mime_type: "image/jpeg".to_string(),
                        data: image_base64.to_string(),
                    },
                },
            ],
        }],
    };

    let api_endpoint = api_url(model, api_key);

    // Retry loop with exponential backoff
    let mut last_error = AiError::other("Gemini", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(&api_endpoint).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Gemini", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Gemini", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Gemini", model, &e.to_string())
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
        let data: GenerateContentResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Gemini", model, &format!("JSON parse error: {}", e)))?;

        let raw_analysis = data
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .unwrap_or_default();

        // Normalize markdown formatting for consistent display
        let analysis = normalize_markdown_response(&raw_analysis);

        return Ok(ChartAnalysisResponse {
            analysis,
            provider: "Gemini".to_string(),
            model: model.to_string(),
            tokens_used: data.usage_metadata.and_then(|u| u.total_token_count),
        });
    }

    Err(last_error)
}
