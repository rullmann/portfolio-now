//! Anthropic Claude API provider for chart analysis

use super::{build_analysis_prompt, ChartAnalysisResponse, ChartContext};
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-5-20250514";

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

/// Analyze a chart image using Claude
pub async fn analyze(
    image_base64: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<ChartAnalysisResponse> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|e| anyhow!("Invalid API key: {}", e))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

    let request = MessagesRequest {
        model: MODEL.to_string(),
        max_tokens: 2048,
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
                    text: build_analysis_prompt(context),
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
        return Err(anyhow!("Claude API error {}: {}", status, body));
    }

    let data: MessagesResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

    let analysis = data
        .content
        .first()
        .and_then(|c| c.text.clone())
        .unwrap_or_default();

    Ok(ChartAnalysisResponse {
        analysis,
        provider: "Claude".to_string(),
        model: MODEL.to_string(),
        tokens_used: data.usage.map(|u| u.input_tokens + u.output_tokens),
    })
}
