//! OpenAI GPT-4 Vision API provider for chart analysis

use super::{build_analysis_prompt, ChartAnalysisResponse, ChartContext};
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

const API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MODEL: &str = "gpt-4o";

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

/// Analyze a chart image using OpenAI GPT-4 Vision
pub async fn analyze(
    image_base64: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<ChartAnalysisResponse> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| anyhow!("Invalid API key: {}", e))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

    let request = ChatCompletionRequest {
        model: MODEL.to_string(),
        max_tokens: 2048,
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text {
                    text: build_analysis_prompt(context),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: format!("data:image/png;base64,{}", image_base64),
                    },
                },
            ],
        }],
    };

    let response = client
        .post(API_URL)
        .json(&request)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("OpenAI API error {}: {}", status, body));
    }

    let data: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

    let analysis = data
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    Ok(ChartAnalysisResponse {
        analysis,
        provider: "OpenAI".to_string(),
        model: MODEL.to_string(),
        tokens_used: data.usage.map(|u| u.total_tokens),
    })
}
