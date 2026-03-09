//! OpenRouter API provider — OpenAI-compatible chat completions
//!
//! OpenRouter provides access to hundreds of models from all providers
//! through a single API endpoint. Uses the same format as OpenAI Chat Completions.
//!
//! API docs: https://openrouter.ai/docs/api-reference

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

const API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const PROVIDER_NAME: &str = "OpenRouter";

// ============================================================================
// Request/Response types (OpenAI Chat Completions compatible)
// ============================================================================

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
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

#[derive(Serialize)]
struct TextChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct TextChatCompletionRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    messages: Vec<TextChatMessage>,
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

#[derive(Deserialize, Clone, Copy)]
struct Usage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: u32,
}

// ============================================================================
// Helpers
// ============================================================================

fn build_client(api_key: &str, model: &str) -> Result<reqwest::Client, AiError> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|_| AiError::invalid_api_key(PROVIDER_NAME, model))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    // OpenRouter recommended headers
    headers.insert(
        "HTTP-Referer",
        HeaderValue::from_static("https://portfolio-now.app"),
    );
    headers.insert("X-Title", HeaderValue::from_static("Portfolio Now"));

    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .pool_max_idle_per_host(2)
        .build()
        .map_err(|e| AiError::network_error(PROVIDER_NAME, model, &e.to_string()))
}

fn parse_error(status: u16, body: &str, model: &str) -> AiError {
    let fallback = get_fallback("openrouter", model);
    let body_lower = body.to_lowercase();

    match status {
        429 => {
            if body_lower.contains("quota") || body_lower.contains("billing") || body_lower.contains("credits") {
                AiError::quota_exceeded(PROVIDER_NAME, model, fallback)
            } else {
                let retry_after = parse_retry_delay(body);
                AiError::rate_limit(PROVIDER_NAME, model, retry_after)
            }
        }
        401 => AiError::invalid_api_key(PROVIDER_NAME, model),
        402 => AiError::quota_exceeded(PROVIDER_NAME, model, fallback),
        404 => AiError::model_not_found(PROVIDER_NAME, model, fallback),
        500..=599 => AiError::server_error(PROVIDER_NAME, model, &format!("HTTP {}", status)),
        _ => AiError::other(PROVIDER_NAME, model, &format!("HTTP {}: {}",
            status,
            if body.len() > 200 { &body[..body.char_indices().nth(200).map(|(i, _)| i).unwrap_or(body.len())] } else { body }
        )),
    }
}

fn is_retryable(err: &AiError) -> bool {
    matches!(err.kind, AiErrorKind::RateLimit | AiErrorKind::ServerError | AiErrorKind::NetworkError)
}

/// Generic retry loop for Chat Completions requests with vision
async fn send_vision_request(
    client: &reqwest::Client,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    image_base64: &str,
    max_tokens: u32,
) -> Result<(String, Option<u32>), AiError> {
    let mut messages = Vec::new();

    if !system_prompt.is_empty() {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: vec![ContentPart::Text { text: system_prompt.to_string() }],
        });
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: vec![
            ContentPart::Text { text: user_prompt.to_string() },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: format!("data:image/jpeg;base64,{}", image_base64),
                },
            },
        ],
    });

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        max_tokens: Some(max_tokens),
        messages,
    };

    let mut last_error = AiError::other(PROVIDER_NAME, model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error(PROVIDER_NAME, model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error(PROVIDER_NAME, model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error(PROVIDER_NAME, model, &e.to_string())
                };

                if attempt < MAX_RETRIES && is_retryable(&last_error) { continue; }
                return Err(last_error);
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            last_error = parse_error(status.as_u16(), &body, model);
            if attempt < MAX_RETRIES && is_retryable(&last_error) { continue; }
            return Err(last_error);
        }

        let data: ChatCompletionResponse = response.json().await
            .map_err(|e| AiError::other(PROVIDER_NAME, model, &format!("JSON parse error: {}", e)))?;

        let text = data.choices.first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        return Ok((text, data.usage.map(|u| u.total_tokens)));
    }

    Err(last_error)
}

/// Generic retry loop for text-only Chat Completions requests
async fn send_text_request(
    client: &reqwest::Client,
    model: &str,
    messages: Vec<TextChatMessage>,
    max_tokens: u32,
) -> Result<(String, Option<u32>), AiError> {
    let request_body = TextChatCompletionRequest {
        model: model.to_string(),
        max_tokens: Some(max_tokens),
        messages,
    };

    let mut last_error = AiError::other(PROVIDER_NAME, model, "No attempts made");

    for attempt in 0..=MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
        }

        let response = match client.post(API_URL).json(&request_body).send().await {
            Ok(resp) => resp,
            Err(e) => {
                last_error = if e.is_timeout() {
                    AiError::network_error(PROVIDER_NAME, model, "Zeitüberschreitung")
                } else if e.is_connect() {
                    AiError::network_error(PROVIDER_NAME, model, "Verbindung fehlgeschlagen")
                } else {
                    AiError::network_error(PROVIDER_NAME, model, &e.to_string())
                };

                if attempt < MAX_RETRIES && is_retryable(&last_error) { continue; }
                return Err(last_error);
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            last_error = parse_error(status.as_u16(), &body, model);
            if attempt < MAX_RETRIES && is_retryable(&last_error) { continue; }
            return Err(last_error);
        }

        let data: ChatCompletionResponse = response.json().await
            .map_err(|e| AiError::other(PROVIDER_NAME, model, &format!("JSON parse error: {}", e)))?;

        let text = data.choices.first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        return Ok((text, data.usage.map(|u| u.total_tokens)));
    }

    Err(last_error)
}

// ============================================================================
// Public API — matches the same interface as other providers
// ============================================================================

pub async fn analyze(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<ChartAnalysisResponse, AiError> {
    let client = build_client(api_key, model)?;
    let prompt = build_analysis_prompt(context, model);
    let (raw, tokens) = send_vision_request(&client, model, "", &prompt, image_base64, MAX_TOKENS).await?;

    Ok(ChartAnalysisResponse {
        analysis: normalize_markdown_response(&raw),
        provider: PROVIDER_NAME.to_string(),
        model: model.to_string(),
        tokens_used: tokens,
        input_tokens: None,
        output_tokens: None,
    })
}

pub async fn analyze_with_annotations(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<AnnotationAnalysisResponse, AiError> {
    let client = build_client(api_key, model)?;
    let prompt = build_annotation_prompt(context);
    let (raw, tokens) = send_vision_request(&client, model, "", &prompt, image_base64, MAX_TOKENS).await?;

    let parsed = parse_annotation_response(&raw)
        .map_err(|e| AiError::other(PROVIDER_NAME, model, &e.to_string()))?;

    Ok(AnnotationAnalysisResponse {
        analysis: parsed.analysis,
        trend: parsed.trend,
        annotations: parsed.annotations,
        provider: PROVIDER_NAME.to_string(),
        model: model.to_string(),
        tokens_used: tokens,
        input_tokens: None,
        output_tokens: None,
    })
}

pub async fn analyze_enhanced(
    image_base64: &str,
    model: &str,
    api_key: &str,
    context: &EnhancedChartContext,
) -> Result<EnhancedAnnotationAnalysisResponse, AiError> {
    let client = build_client(api_key, model)?;
    let prompt = build_enhanced_annotation_prompt(context);
    let (raw, tokens) = send_vision_request(&client, model, "", &prompt, image_base64, MAX_TOKENS).await?;

    let parsed = parse_enhanced_annotation_response(&raw)
        .map_err(|e| AiError::other(PROVIDER_NAME, model, &e.to_string()))?;

    Ok(EnhancedAnnotationAnalysisResponse {
        analysis: parsed.analysis,
        trend: parsed.trend,
        annotations: parsed.annotations,
        alerts: parsed.alerts,
        risk_reward: parsed.risk_reward,
        provider: PROVIDER_NAME.to_string(),
        model: model.to_string(),
        tokens_used: tokens,
        input_tokens: None,
        output_tokens: None,
    })
}

pub async fn analyze_portfolio(
    model: &str,
    api_key: &str,
    context: &PortfolioInsightsContext,
) -> Result<PortfolioInsightsResponse, AiError> {
    let client = build_client(api_key, model)?;
    let prompt = build_portfolio_insights_prompt(context);
    let (raw, tokens) = send_text_request(
        &client, model,
        vec![TextChatMessage { role: "user".to_string(), content: prompt }],
        MAX_TOKENS_INSIGHTS,
    ).await?;

    Ok(PortfolioInsightsResponse {
        analysis: normalize_markdown_response(&raw),
        provider: PROVIDER_NAME.to_string(),
        model: model.to_string(),
        tokens_used: tokens,
        input_tokens: None,
        output_tokens: None,
    })
}

pub async fn analyze_opportunities(
    model: &str,
    api_key: &str,
    context: &PortfolioInsightsContext,
) -> Result<PortfolioInsightsResponse, AiError> {
    let client = build_client(api_key, model)?;
    let prompt = build_opportunities_prompt(context);
    let (raw, tokens) = send_text_request(
        &client, model,
        vec![TextChatMessage { role: "user".to_string(), content: prompt }],
        MAX_TOKENS_INSIGHTS,
    ).await?;

    Ok(PortfolioInsightsResponse {
        analysis: normalize_markdown_response(&raw),
        provider: PROVIDER_NAME.to_string(),
        model: model.to_string(),
        tokens_used: tokens,
        input_tokens: None,
        output_tokens: None,
    })
}

pub async fn chat(
    model: &str,
    api_key: &str,
    messages: &[AiChatMessage],
    context: &PortfolioInsightsContext,
) -> Result<PortfolioChatResponse, AiError> {
    let client = build_client(api_key, model)?;
    let system_prompt = build_chat_system_prompt(context);
    let has_images = messages.iter().any(|m| !m.attachments.is_empty());

    if has_images {
        // Use vision request format for messages with image attachments
        let mut vision_messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: vec![ContentPart::Text { text: system_prompt }],
            },
        ];

        for msg in messages {
            let mut parts = vec![ContentPart::Text { text: msg.content.clone() }];
            for attachment in &msg.attachments {
                let media_type = super::detect_image_media_type(&attachment.data);
                parts.push(ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: format!("data:{};base64,{}", media_type, attachment.data),
                    },
                });
            }
            vision_messages.push(ChatMessage {
                role: msg.role.clone(),
                content: parts,
            });
        }

        let request_body = ChatCompletionRequest {
            model: model.to_string(),
            max_tokens: Some(MAX_TOKENS_CHAT),
            messages: vision_messages,
        };

        let mut last_error = AiError::other(PROVIDER_NAME, model, "No attempts made");
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(calculate_backoff_delay(attempt - 1)).await;
            }
            let response = match client.post(API_URL).json(&request_body).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    last_error = AiError::network_error(PROVIDER_NAME, model, &e.to_string());
                    if attempt < MAX_RETRIES && is_retryable(&last_error) { continue; }
                    return Err(last_error);
                }
            };
            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                last_error = parse_error(status.as_u16(), &body, model);
                if attempt < MAX_RETRIES && is_retryable(&last_error) { continue; }
                return Err(last_error);
            }
            let data: ChatCompletionResponse = response.json().await
                .map_err(|e| AiError::other(PROVIDER_NAME, model, &format!("JSON parse error: {}", e)))?;
            let text = data.choices.first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            return Ok(PortfolioChatResponse {
                response: normalize_markdown_response(&text),
                provider: PROVIDER_NAME.to_string(),
                model: model.to_string(),
                tokens_used: data.usage.map(|u| u.total_tokens),
                input_tokens: data.usage.and_then(|u| u.prompt_tokens),
                output_tokens: data.usage.and_then(|u| u.completion_tokens),
                suggestions: vec![],
                pending_queries: vec![],
            });
        }
        Err(last_error)
    } else {
        // Text-only chat
        let mut chat_messages = vec![
            TextChatMessage { role: "system".to_string(), content: system_prompt },
        ];
        for msg in messages {
            chat_messages.push(TextChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }
        let (raw, tokens) = send_text_request(&client, model, chat_messages, MAX_TOKENS_CHAT).await?;

        Ok(PortfolioChatResponse {
            response: normalize_markdown_response(&raw),
            provider: PROVIDER_NAME.to_string(),
            model: model.to_string(),
            tokens_used: tokens,
            input_tokens: None,
            output_tokens: None,
            suggestions: vec![],
            pending_queries: vec![],
        })
    }
}

pub async fn complete_text(
    model: &str,
    api_key: &str,
    prompt: &str,
) -> Result<String, AiError> {
    let client = build_client(api_key, model)?;
    let (text, _) = send_text_request(
        &client, model,
        vec![TextChatMessage { role: "user".to_string(), content: prompt.to_string() }],
        MAX_TOKENS_INSIGHTS,
    ).await?;
    Ok(text)
}

// ============================================================================
// Model Listing — fetch live from OpenRouter API
// ============================================================================

use super::AiModelInfo;

/// Fetch all available models from OpenRouter API
pub async fn list_models(api_key: &str) -> anyhow::Result<Vec<AiModelInfo>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| anyhow::anyhow!("Invalid API key: {}", e))?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(15))
        .build()?;

    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("OpenRouter API error: {}", body));
    }

    #[derive(Deserialize)]
    struct OpenRouterModel {
        id: String,
        name: String,
        #[serde(default)]
        description: Option<String>,
        architecture: Option<Architecture>,
        #[serde(default, rename = "context_length")]
        context_length: Option<u32>,
        #[serde(default)]
        pricing: Option<Pricing>,
    }

    #[derive(Deserialize)]
    struct Architecture {
        modality: Option<String>,
    }

    #[derive(Deserialize)]
    struct Pricing {
        #[serde(default)]
        prompt: Option<String>,
        #[serde(default)]
        completion: Option<String>,
    }

    #[derive(Deserialize)]
    struct ModelsResponse {
        data: Vec<OpenRouterModel>,
    }

    let data: ModelsResponse = response.json().await?;

    let models: Vec<AiModelInfo> = data
        .data
        .into_iter()
        .filter(|m| {
            // Only include models that can do text generation (not just embedding/moderation)
            let modality = m.architecture.as_ref()
                .and_then(|a| a.modality.as_deref())
                .unwrap_or("text->text");
            // Accept text->text and multimodal models
            modality.contains("text") &&
            // Exclude utility models
            !m.id.contains("embedding") &&
            !m.id.contains("moderation") &&
            !m.id.contains("tts") &&
            !m.id.contains("whisper") &&
            !m.id.contains("dall-e")
        })
        .map(|m| {
            let modality = m.architecture.as_ref()
                .and_then(|a| a.modality.as_deref())
                .unwrap_or("text->text");
            let supports_vision = modality.contains("image") || modality.contains("multi");
            // OpenRouter models with search capability
            let supports_web_search = m.id.contains("online") || m.id.starts_with("perplexity/");

            let short_desc = m.description
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(80)
                .collect::<String>();

            // OpenRouter pricing is per-token (string), convert to per-1M-tokens (f64)
            let pricing_input = m.pricing.as_ref()
                .and_then(|p| p.prompt.as_deref())
                .and_then(|s| s.parse::<f64>().ok())
                .map(|v| v * 1_000_000.0);
            let pricing_output = m.pricing.as_ref()
                .and_then(|p| p.completion.as_deref())
                .and_then(|s| s.parse::<f64>().ok())
                .map(|v| v * 1_000_000.0);

            AiModelInfo {
                id: m.id,
                name: m.name,
                description: if short_desc.is_empty() { "via OpenRouter".to_string() } else { short_desc },
                supports_vision,
                supports_web_search,
                max_output_tokens: None, // OpenRouter doesn't expose this directly
                context_window: m.context_length,
                pricing_input,
                pricing_output,
                deprecation_date: None,
            }
        })
        .collect();

    Ok(models)
}
