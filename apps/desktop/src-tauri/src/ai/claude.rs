//! Anthropic Claude API provider for chart analysis

use super::{
    build_analysis_prompt, build_annotation_prompt, parse_annotation_response,
    build_portfolio_insights_prompt, build_chat_system_prompt,
    AiError, AiErrorKind, ChartAnalysisResponse, ChartContext, AnnotationAnalysisResponse,
    PortfolioInsightsContext, PortfolioInsightsResponse, ChatMessage, PortfolioChatResponse,
    get_fallback, parse_retry_delay, calculate_backoff_delay, normalize_markdown_response,
    REQUEST_TIMEOUT_SECS, MAX_RETRIES, MAX_TOKENS, MAX_TOKENS_INSIGHTS, MAX_TOKENS_CHAT,
};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "document")]
    Document { source: DocumentSource },
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Serialize)]
struct DocumentSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ResponseContent>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct ResponseContent {
    text: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Parse Claude API error response
fn parse_error(status: u16, body: &str, model: &str) -> AiError {
    let fallback = get_fallback("claude", model);
    let body_lower = body.to_lowercase();

    match status {
        429 => {
            if body_lower.contains("quota") || body_lower.contains("credit") {
                AiError::quota_exceeded("Claude", model, fallback)
            } else {
                let retry_after = parse_retry_delay(body);
                AiError::rate_limit("Claude", model, retry_after)
            }
        }
        401 => AiError::invalid_api_key("Claude", model),
        403 => {
            if body_lower.contains("permission") || body_lower.contains("access") {
                AiError::invalid_api_key("Claude", model)
            } else {
                AiError::other("Claude", model, "Zugriff verweigert")
            }
        }
        404 => AiError::model_not_found("Claude", model, fallback),
        500..=599 => AiError::server_error("Claude", model, &format!("HTTP {}", status)),
        _ => AiError::other("Claude", model, &format!("HTTP {}: {}",
            status,
            if body.len() > 200 { &body[..200] } else { body }
        )),
    }
}

/// Check if error is retryable
fn is_retryable(err: &AiError) -> bool {
    matches!(err.kind, AiErrorKind::RateLimit | AiErrorKind::ServerError | AiErrorKind::NetworkError)
}

/// Analyze a chart image using Claude with retry logic
pub async fn analyze(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<ChartAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key)
            .map_err(|_| AiError::invalid_api_key("Claude", model))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Create client with timeout and connection pooling
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Claude", model, &e.to_string()))?;

    let request_body = MessagesRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: "image/jpeg".to_string(), // Optimized images are JPEG
                        data: image_base64.to_string(),
                    },
                },
                ContentBlock::Text {
                    text: build_analysis_prompt(context, model),
                },
            ],
        }],
    };

    // Retry loop with exponential backoff
    let mut last_error = AiError::other("Claude", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Claude", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Claude", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Claude", model, &e.to_string())
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
        let data: MessagesResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Claude", model, &format!("JSON parse error: {}", e)))?;

        let raw_analysis = data
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        // Normalize markdown formatting for consistent display
        let analysis = normalize_markdown_response(&raw_analysis);

        return Ok(ChartAnalysisResponse {
            analysis,
            provider: "Claude".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.input_tokens + u.output_tokens),
        });
    }

    Err(last_error)
}

/// Analyze an image using Claude with a custom prompt (e.g., for OCR)
pub async fn analyze_with_custom_prompt(
    image_base64: &str,
    model: &str,
    api_key: &str,
    custom_prompt: &str,
) -> Result<ChartAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key)
            .map_err(|_| AiError::invalid_api_key("Claude", model))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Claude", model, &e.to_string()))?;

    let request_body = MessagesRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: "image/png".to_string(),
                        data: image_base64.to_string(),
                    },
                },
                ContentBlock::Text {
                    text: custom_prompt.to_string(),
                },
            ],
        }],
    };

    let mut last_error = AiError::other("Claude", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Claude", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Claude", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Claude", model, &e.to_string())
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

        let data: MessagesResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Claude", model, &format!("JSON parse error: {}", e)))?;

        let analysis = data
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        return Ok(ChartAnalysisResponse {
            analysis,
            provider: "Claude".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.input_tokens + u.output_tokens),
        });
    }

    Err(last_error)
}

/// Perform OCR on a PDF document using Claude's native PDF support
/// No image conversion needed - PDF is sent directly to the API
pub async fn ocr_pdf(
    pdf_base64: &str,
    model: &str,
    api_key: &str,
    custom_prompt: &str,
) -> Result<ChartAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key)
            .map_err(|_| AiError::invalid_api_key("Claude", model))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    // Enable PDF support beta feature
    headers.insert(
        "anthropic-beta",
        HeaderValue::from_static("pdfs-2024-09-25"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS * 2)) // PDF processing may take longer
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Claude", model, &e.to_string()))?;

    let request_body = MessagesRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS * 2, // PDF OCR may produce more text
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::Document {
                    source: DocumentSource {
                        source_type: "base64".to_string(),
                        media_type: "application/pdf".to_string(),
                        data: pdf_base64.to_string(),
                    },
                },
                ContentBlock::Text {
                    text: custom_prompt.to_string(),
                },
            ],
        }],
    };

    let mut last_error = AiError::other("Claude", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Claude", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Claude", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Claude", model, &e.to_string())
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

        let data: MessagesResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Claude", model, &format!("JSON parse error: {}", e)))?;

        let analysis = data
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        return Ok(ChartAnalysisResponse {
            analysis,
            provider: "Claude".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.input_tokens + u.output_tokens),
        });
    }

    Err(last_error)
}

/// Analyze a chart image using Claude and return structured annotations
pub async fn analyze_with_annotations(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<AnnotationAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key)
            .map_err(|_| AiError::invalid_api_key("Claude", model))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Claude", model, &e.to_string()))?;

    let request_body = MessagesRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: "image/jpeg".to_string(),
                        data: image_base64.to_string(),
                    },
                },
                ContentBlock::Text {
                    text: build_annotation_prompt(context),
                },
            ],
        }],
    };

    let mut last_error = AiError::other("Claude", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Claude", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Claude", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Claude", model, &e.to_string())
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

        let data: MessagesResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Claude", model, &format!("JSON parse error: {}", e)))?;

        let raw_response = data
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        // Parse the JSON response into structured annotations
        let parsed = parse_annotation_response(&raw_response)
            .map_err(|e| AiError::other("Claude", model, &e.to_string()))?;

        return Ok(AnnotationAnalysisResponse {
            analysis: parsed.analysis,
            trend: parsed.trend,
            annotations: parsed.annotations,
            provider: "Claude".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.input_tokens + u.output_tokens),
        });
    }

    Err(last_error)
}

// ============================================================================
// Text-only Analysis (Portfolio Insights & Chat)
// ============================================================================

/// Request body for text-only messages (no images)
#[derive(Serialize)]
struct TextMessagesRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<TextMessage>,
}

#[derive(Serialize)]
struct TextMessage {
    role: String,
    content: String,
}

/// Analyze portfolio with Claude (text-only, no image)
pub async fn analyze_portfolio(
    model: &str,
    api_key: &str,
    context: &PortfolioInsightsContext,
) -> Result<PortfolioInsightsResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key)
            .map_err(|_| AiError::invalid_api_key("Claude", model))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Claude", model, &e.to_string()))?;

    let request_body = TextMessagesRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS_INSIGHTS,
        system: None,
        messages: vec![TextMessage {
            role: "user".to_string(),
            content: build_portfolio_insights_prompt(context),
        }],
    };

    let mut last_error = AiError::other("Claude", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Claude", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Claude", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Claude", model, &e.to_string())
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

        let data: MessagesResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Claude", model, &format!("JSON parse error: {}", e)))?;

        let raw_analysis = data
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        let analysis = normalize_markdown_response(&raw_analysis);

        return Ok(PortfolioInsightsResponse {
            analysis,
            provider: "Claude".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.input_tokens + u.output_tokens),
        });
    }

    Err(last_error)
}

/// Chat with portfolio assistant using Claude
pub async fn chat(
    model: &str,
    api_key: &str,
    messages: &[ChatMessage],
    context: &PortfolioInsightsContext,
) -> Result<PortfolioChatResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key)
            .map_err(|_| AiError::invalid_api_key("Claude", model))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("Claude", model, &e.to_string()))?;

    // Convert messages to Claude format
    let claude_messages: Vec<TextMessage> = messages
        .iter()
        .map(|m| TextMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let request_body = TextMessagesRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS_CHAT,
        system: Some(build_chat_system_prompt(context)),
        messages: claude_messages,
    };

    let mut last_error = AiError::other("Claude", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("Claude", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("Claude", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("Claude", model, &e.to_string())
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

        let data: MessagesResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("Claude", model, &format!("JSON parse error: {}", e)))?;

        let response_text = data
            .content
            .first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        return Ok(PortfolioChatResponse {
            response: response_text,
            provider: "Claude".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.input_tokens + u.output_tokens),
        });
    }

    Err(last_error)
}
