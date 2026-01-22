//! OpenAI GPT-5 Vision API provider for chart analysis

use super::{
    build_analysis_prompt, build_annotation_prompt, parse_annotation_response,
    build_enhanced_annotation_prompt, parse_enhanced_annotation_response,
    build_portfolio_insights_prompt, build_opportunities_prompt, build_chat_system_prompt,
    AiError, AiErrorKind, ChartAnalysisResponse, ChartContext, AnnotationAnalysisResponse,
    EnhancedChartContext, EnhancedAnnotationAnalysisResponse,
    PortfolioInsightsContext, PortfolioInsightsResponse,
    ChatMessage as AiChatMessage, PortfolioChatResponse,
    get_fallback, parse_retry_delay, calculate_backoff_delay, normalize_markdown_response,
    REQUEST_TIMEOUT_SECS, MAX_RETRIES, MAX_TOKENS, MAX_TOKENS_INSIGHTS, MAX_TOKENS_CHAT,
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const CHAT_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const RESPONSES_API_URL: &str = "https://api.openai.com/v1/responses";

/// Check if model uses the new Responses API (GPT-5 models)
fn uses_responses_api(model: &str) -> bool {
    model.starts_with("gpt-5")
}

/// Extract text from Responses API response
fn extract_responses_text(response: &ResponsesApiResponse) -> String {
    response
        .output
        .iter()
        .filter(|item| item.output_type == "message")
        .flat_map(|item| item.content.iter())
        // OpenAI Responses API uses "text" as content_type, not "output_text"
        .filter(|c| c.content_type == "text" || c.content_type == "output_text")
        .map(|c| c.text.clone())
        .collect::<Vec<_>>()
        .join("")
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    /// Chat Completions API uses max_tokens (max_completion_tokens is for newer APIs)
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

// ============================================================================
// Responses API Types (GPT-5 models)
// ============================================================================

/// Request body for Responses API (GPT-5)
#[derive(Serialize)]
struct ResponsesApiRequest {
    model: String,
    input: ResponsesInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

/// Input can be a simple string or array of messages
#[derive(Serialize)]
#[serde(untagged)]
enum ResponsesInput {
    Text(String),
    Messages(Vec<ResponsesMessage>),
}

/// Message format for Responses API
#[derive(Serialize)]
struct ResponsesMessage {
    #[serde(rename = "type")]
    msg_type: String,
    role: String,
    content: String,
}

/// Response from Responses API
#[derive(Deserialize)]
struct ResponsesApiResponse {
    output: Vec<ResponsesOutputItem>,
    usage: Option<ResponsesUsage>,
}

/// Output item in Responses API response
#[derive(Deserialize)]
struct ResponsesOutputItem {
    #[serde(rename = "type")]
    output_type: String,
    #[serde(default)]
    content: Vec<ResponsesContent>,
}

/// Content in output
#[derive(Deserialize)]
struct ResponsesContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// Usage stats from Responses API
#[derive(Deserialize)]
struct ResponsesUsage {
    total_tokens: u32,
}

/// Parse OpenAI API error response
fn parse_error(status: u16, body: &str, model: &str) -> AiError {
    let fallback = get_fallback("openai", model);
    let body_lower = body.to_lowercase();

    match status {
        429 => {
            // OpenAI uses 429 for rate limit and quota
            if body_lower.contains("quota") || body_lower.contains("billing") ||
               body_lower.contains("exceeded") {
                AiError::quota_exceeded("OpenAI", model, fallback)
            } else {
                let retry_after = parse_retry_delay(body);
                AiError::rate_limit("OpenAI", model, retry_after)
            }
        }
        401 => AiError::invalid_api_key("OpenAI", model),
        403 => {
            if body_lower.contains("permission") || body_lower.contains("access") {
                AiError::invalid_api_key("OpenAI", model)
            } else {
                AiError::other("OpenAI", model, "Zugriff verweigert")
            }
        }
        404 => AiError::model_not_found("OpenAI", model, fallback),
        500..=599 => AiError::server_error("OpenAI", model, &format!("HTTP {}", status)),
        _ => AiError::other("OpenAI", model, &format!("HTTP {}: {}",
            status,
            if body.len() > 200 { &body[..200] } else { body }
        )),
    }
}

/// Check if error is retryable
fn is_retryable(err: &AiError) -> bool {
    matches!(err.kind, AiErrorKind::RateLimit | AiErrorKind::ServerError | AiErrorKind::NetworkError)
}

/// Analyze a chart image using OpenAI GPT-4 Vision with retry logic
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
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Create client with timeout and connection pooling
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

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
                        // OpenAI accepts JPEG for optimized images
                        url: format!("data:image/jpeg;base64,{}", image_base64),
                    },
                },
            ],
        }],
    };

    // Retry loop with exponential backoff
    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(CHAT_API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        let raw_analysis = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        // Normalize markdown formatting for consistent display
        let analysis = normalize_markdown_response(&raw_analysis);

        return Ok(ChartAnalysisResponse {
            analysis,
            provider: "OpenAI".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
        });
    }

    Err(last_error)
}

/// Analyze an image using OpenAI with a custom prompt (e.g., for OCR)
pub async fn analyze_with_custom_prompt(
    image_base64: &str,
    model: &str,
    api_key: &str,
    custom_prompt: &str,
) -> Result<ChartAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text {
                    text: custom_prompt.to_string(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: format!("data:image/png;base64,{}", image_base64),
                    },
                },
            ],
        }],
    };

    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(CHAT_API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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

        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        let analysis = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        return Ok(ChartAnalysisResponse {
            analysis,
            provider: "OpenAI".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
        });
    }

    Err(last_error)
}

/// Analyze a chart image using OpenAI GPT-4 Vision and return structured annotations
pub async fn analyze_with_annotations(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<AnnotationAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text {
                    text: build_annotation_prompt(context),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: format!("data:image/jpeg;base64,{}", image_base64),
                    },
                },
            ],
        }],
    };

    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(CHAT_API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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

        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        let raw_response = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let parsed = parse_annotation_response(&raw_response)
            .map_err(|e| AiError::other("OpenAI", model, &e.to_string()))?;

        return Ok(AnnotationAnalysisResponse {
            analysis: parsed.analysis,
            trend: parsed.trend,
            annotations: parsed.annotations,
            provider: "OpenAI".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
        });
    }

    Err(last_error)
}

/// Analyze a chart image using OpenAI with enhanced context and return structured annotations
/// with alerts and risk/reward analysis
pub async fn analyze_enhanced(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &EnhancedChartContext,
) -> Result<EnhancedAnnotationAnalysisResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        max_tokens: MAX_TOKENS,
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text {
                    text: build_enhanced_annotation_prompt(context),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: format!("data:image/jpeg;base64,{}", image_base64),
                    },
                },
            ],
        }],
    };

    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(CHAT_API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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

        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        let raw_response = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let parsed = parse_enhanced_annotation_response(&raw_response)
            .map_err(|e| AiError::other("OpenAI", model, &e.to_string()))?;

        return Ok(EnhancedAnnotationAnalysisResponse {
            analysis: parsed.analysis,
            trend: parsed.trend,
            annotations: parsed.annotations,
            alerts: parsed.alerts,
            risk_reward: parsed.risk_reward,
            provider: "OpenAI".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
        });
    }

    Err(last_error)
}

// ============================================================================
// Text-only Analysis (Portfolio Insights & Chat)
// ============================================================================

/// Request body for text-only chat completion
#[derive(Serialize)]
struct TextChatCompletionRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<TextChatMessage>,
}

/// Request body for chat completion with web search tool (o3, o4-mini)
#[derive(Serialize)]
struct TextChatWithToolsRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<TextChatMessage>,
    tools: Vec<WebSearchTool>,
}

/// Web search tool configuration
#[derive(Serialize)]
struct WebSearchTool {
    #[serde(rename = "type")]
    tool_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_context_size: Option<String>,
}

#[derive(Serialize, Clone)]
struct TextChatMessage {
    role: String,
    content: String,
}

/// Check if model supports web search
fn supports_web_search(model: &str) -> bool {
    model.starts_with("o3") || model.starts_with("o4")
}

/// Analyze portfolio with OpenAI (text-only, no image)
pub async fn analyze_portfolio(
    model: &str,
    api_key: &str,
    context: &PortfolioInsightsContext,
) -> Result<PortfolioInsightsResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

    let prompt = build_portfolio_insights_prompt(context);
    let use_responses_api = uses_responses_api(model);
    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        // GPT-5 uses Responses API
        if use_responses_api {
            let request_body = ResponsesApiRequest {
                model: model.to_string(),
                input: ResponsesInput::Text(prompt.clone()),
                instructions: None,
                max_output_tokens: Some(MAX_TOKENS_INSIGHTS),
            };

            let response = match client.post(RESPONSES_API_URL).json(&request_body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = if e.is_timeout() {
                        AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                    } else if e.is_connect() {
                        AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                    } else {
                        AiError::network_error("OpenAI", model, &e.to_string())
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

            let data: ResponsesApiResponse = response
                .json()
                .await
                .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

            let raw_analysis = extract_responses_text(&data);
            let analysis = normalize_markdown_response(&raw_analysis);

            return Ok(PortfolioInsightsResponse {
                analysis,
                provider: "OpenAI".to_string(),
                model: model.to_string(),
                tokens_used: data.usage.map(|u| u.total_tokens),
            });
        }

        // GPT-4.x and older use Chat Completions API
        let request_body = TextChatCompletionRequest {
            model: model.to_string(),
            max_tokens: MAX_TOKENS_INSIGHTS,
            messages: vec![TextChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            }],
        };

        let response = match client.post(CHAT_API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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

        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        let raw_analysis = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let analysis = normalize_markdown_response(&raw_analysis);

        return Ok(PortfolioInsightsResponse {
            analysis,
            provider: "OpenAI".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
        });
    }

    Err(last_error)
}

/// Analyze portfolio for buy opportunities with OpenAI (text-only, no image)
pub async fn analyze_opportunities(
    model: &str,
    api_key: &str,
    context: &PortfolioInsightsContext,
) -> Result<PortfolioInsightsResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

    let prompt = build_opportunities_prompt(context);
    let use_responses_api = uses_responses_api(model);
    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        // GPT-5 uses Responses API
        if use_responses_api {
            let request_body = ResponsesApiRequest {
                model: model.to_string(),
                input: ResponsesInput::Text(prompt.clone()),
                instructions: None,
                max_output_tokens: Some(MAX_TOKENS_INSIGHTS),
            };

            let response = match client.post(RESPONSES_API_URL).json(&request_body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = if e.is_timeout() {
                        AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                    } else if e.is_connect() {
                        AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                    } else {
                        AiError::network_error("OpenAI", model, &e.to_string())
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

            let data: ResponsesApiResponse = response
                .json()
                .await
                .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

            let raw_analysis = extract_responses_text(&data);
            let analysis = normalize_markdown_response(&raw_analysis);

            return Ok(PortfolioInsightsResponse {
                analysis,
                provider: "OpenAI".to_string(),
                model: model.to_string(),
                tokens_used: data.usage.map(|u| u.total_tokens),
            });
        }

        // GPT-4.x and older use Chat Completions API
        let request_body = TextChatCompletionRequest {
            model: model.to_string(),
            max_tokens: MAX_TOKENS_INSIGHTS,
            messages: vec![TextChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            }],
        };

        let response = match client.post(CHAT_API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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

        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        let raw_analysis = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let analysis = normalize_markdown_response(&raw_analysis);

        return Ok(PortfolioInsightsResponse {
            analysis,
            provider: "OpenAI".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
        });
    }

    Err(last_error)
}

/// Multimodal chat message (with images)
#[derive(Serialize)]
struct MultimodalChatMessage {
    role: String,
    content: Vec<ContentPart>,
}

/// Multimodal chat completion request
#[derive(Serialize)]
struct MultimodalChatCompletionRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<MultimodalChatMessage>,
}

/// Chat with portfolio assistant using OpenAI
/// Supports both text-only and multimodal (with images) messages
pub async fn chat(
    model: &str,
    api_key: &str,
    messages: &[AiChatMessage],
    context: &PortfolioInsightsContext,
) -> Result<PortfolioChatResponse, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

    let system_prompt = build_chat_system_prompt(context);
    let use_responses_api = uses_responses_api(model);
    let use_web_search = supports_web_search(model);

    // Check if any message has image attachments
    let has_images = messages.iter().any(|m| !m.attachments.is_empty());

    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        // GPT-5 uses Responses API (text-only for now)
        if use_responses_api {
            // Build messages for Responses API
            let responses_messages: Vec<ResponsesMessage> = messages
                .iter()
                .map(|m| ResponsesMessage {
                    msg_type: "message".to_string(),
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect();

            let request_body = ResponsesApiRequest {
                model: model.to_string(),
                input: ResponsesInput::Messages(responses_messages),
                instructions: Some(system_prompt.clone()),
                max_output_tokens: Some(MAX_TOKENS_CHAT),
            };

            let response = match client.post(RESPONSES_API_URL).json(&request_body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = if e.is_timeout() {
                        AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                    } else if e.is_connect() {
                        AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                    } else {
                        AiError::network_error("OpenAI", model, &e.to_string())
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

            let data: ResponsesApiResponse = response
                .json()
                .await
                .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

            let response_text = extract_responses_text(&data);

            return Ok(PortfolioChatResponse {
                response: response_text,
                provider: "OpenAI".to_string(),
                model: model.to_string(),
                tokens_used: data.usage.map(|u| u.total_tokens),
                suggestions: Vec::new(),
            });
        }

        // GPT-4.x and older use Chat Completions API
        let response = if has_images {
            // Build multimodal messages with images
            let mut openai_messages: Vec<MultimodalChatMessage> = vec![
                MultimodalChatMessage {
                    role: "system".to_string(),
                    content: vec![ContentPart::Text { text: system_prompt.clone() }],
                }
            ];

            for m in messages {
                let mut content: Vec<ContentPart> = Vec::new();

                // Add images first
                for attachment in &m.attachments {
                    content.push(ContentPart::ImageUrl {
                        image_url: ImageUrl {
                            url: format!("data:{};base64,{}", attachment.mime_type, attachment.data),
                        },
                    });
                }

                // Add text content
                if !m.content.is_empty() {
                    content.push(ContentPart::Text { text: m.content.clone() });
                }

                openai_messages.push(MultimodalChatMessage {
                    role: m.role.clone(),
                    content,
                });
            }

            let request_body = MultimodalChatCompletionRequest {
                model: model.to_string(),
                max_tokens: MAX_TOKENS_CHAT,
                messages: openai_messages,
            };

            client.post(CHAT_API_URL).json(&request_body).send().await
        } else {
            // Text-only messages
            let mut openai_messages = vec![TextChatMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            }];

            for m in messages {
                openai_messages.push(TextChatMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                });
            }

            // Send request with or without web search tool
            if use_web_search {
                let request_body = TextChatWithToolsRequest {
                    model: model.to_string(),
                    max_tokens: MAX_TOKENS_CHAT,
                    messages: openai_messages.clone(),
                    tools: vec![WebSearchTool {
                        tool_type: "web_search_preview".to_string(),
                        search_context_size: Some("medium".to_string()),
                    }],
                };
                client.post(CHAT_API_URL).json(&request_body).send().await
            } else {
                let request_body = TextChatCompletionRequest {
                    model: model.to_string(),
                    max_tokens: MAX_TOKENS_CHAT,
                    messages: openai_messages.clone(),
                };
                client.post(CHAT_API_URL).json(&request_body).send().await
            }
        };

        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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

        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        let response_text = data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        return Ok(PortfolioChatResponse {
            response: response_text,
            provider: "OpenAI".to_string(),
            model: model.to_string(),
            tokens_used: data.usage.map(|u| u.total_tokens),
            suggestions: Vec::new(),
        });
    }

    Err(last_error)
}

/// Simple text completion with OpenAI (returns raw response)
pub async fn complete_text(
    model: &str,
    api_key: &str,
    prompt: &str,
) -> Result<String, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key("OpenAI", model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error("OpenAI", model, &e.to_string()))?;

    let use_responses_api = uses_responses_api(model);
    let mut last_error = AiError::other("OpenAI", model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        // GPT-5 uses Responses API
        if use_responses_api {
            let request_body = ResponsesApiRequest {
                model: model.to_string(),
                input: ResponsesInput::Text(prompt.to_string()),
                instructions: None,
                max_output_tokens: Some(MAX_TOKENS_INSIGHTS),
            };

            let response = match client.post(RESPONSES_API_URL).json(&request_body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = if e.is_timeout() {
                        AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                    } else if e.is_connect() {
                        AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                    } else {
                        AiError::network_error("OpenAI", model, &e.to_string())
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

            let data: ResponsesApiResponse = response
                .json()
                .await
                .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

            return Ok(extract_responses_text(&data));
        }

        // GPT-4.x and older use Chat Completions API
        let request_body = TextChatCompletionRequest {
            model: model.to_string(),
            max_tokens: MAX_TOKENS_INSIGHTS,
            messages: vec![TextChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = match client.post(CHAT_API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error("OpenAI", model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error("OpenAI", model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error("OpenAI", model, &e.to_string())
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

        let data: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| AiError::other("OpenAI", model, &format!("JSON parse error: {}", e)))?;

        return Ok(data
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default());
    }

    Err(last_error)
}
