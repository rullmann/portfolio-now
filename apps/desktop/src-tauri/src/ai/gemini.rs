//! Google Gemini API provider for chart analysis

use super::{build_analysis_prompt, ChartAnalysisResponse, ChartContext};
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

const MODEL: &str = "gemini-2.0-flash";

fn api_url(api_key: &str) -> String {
    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        MODEL, api_key
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

/// Analyze a chart image using Google Gemini
pub async fn analyze(
    image_base64: &str,
    api_key: &str,
    context: &ChartContext,
) -> Result<ChartAnalysisResponse> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

    let request = GenerateContentRequest {
        contents: vec![Content {
            role: "user".to_string(),
            parts: vec![
                Part::Text {
                    text: build_analysis_prompt(context),
                },
                Part::InlineData {
                    inline_data: InlineData {
                        mime_type: "image/png".to_string(),
                        data: image_base64.to_string(),
                    },
                },
            ],
        }],
    };

    let response = client
        .post(&api_url(api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| anyhow!("Request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini API error {}: {}", status, body));
    }

    let data: GenerateContentResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

    let analysis = data
        .candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.content)
        .and_then(|c| c.parts)
        .and_then(|p| p.into_iter().next())
        .and_then(|p| p.text)
        .unwrap_or_default();

    Ok(ChartAnalysisResponse {
        analysis,
        provider: "Gemini".to_string(),
        model: MODEL.to_string(),
        tokens_used: data.usage_metadata.and_then(|u| u.total_token_count),
    })
}
