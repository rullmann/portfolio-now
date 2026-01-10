//! AI-powered chart analysis module.
//!
//! Supports multiple providers: Claude (Anthropic), GPT-4 (OpenAI), Gemini (Google), Perplexity (Sonar)

pub mod claude;
pub mod openai;
pub mod gemini;
pub mod perplexity;

use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

// ============================================================================
// Structured AI Errors
// ============================================================================

/// Types of AI API errors
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AiErrorKind {
    /// Rate limit exceeded - too many requests, retry after delay
    RateLimit,
    /// Quota/credits exhausted - need to upgrade plan or switch provider
    QuotaExceeded,
    /// Invalid or expired API key
    InvalidApiKey,
    /// Model not found or not available
    ModelNotFound,
    /// Server error on provider side
    ServerError,
    /// Network/connection error
    NetworkError,
    /// Other/unknown error
    Other,
}

/// Structured AI error with details
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiError {
    pub kind: AiErrorKind,
    pub message: String,
    pub provider: String,
    pub model: String,
    /// Suggested retry delay in seconds (for rate limit errors)
    pub retry_after_secs: Option<u32>,
    /// Suggested fallback model (for quota/model errors)
    pub fallback_model: Option<String>,
}

impl AiError {
    pub fn rate_limit(provider: &str, model: &str, retry_after: Option<u32>) -> Self {
        Self {
            kind: AiErrorKind::RateLimit,
            message: "Zu viele Anfragen. Bitte warte einen Moment.".to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: retry_after,
            fallback_model: None,
        }
    }

    pub fn quota_exceeded(provider: &str, model: &str, fallback: Option<&str>) -> Self {
        Self {
            kind: AiErrorKind::QuotaExceeded,
            message: "Kontingent erschöpft. Bitte wechsle das Modell oder den Anbieter.".to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: fallback.map(String::from),
        }
    }

    pub fn invalid_api_key(provider: &str, model: &str) -> Self {
        Self {
            kind: AiErrorKind::InvalidApiKey,
            message: "Ungültiger API Key. Bitte überprüfe deine Einstellungen.".to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: None,
        }
    }

    pub fn model_not_found(provider: &str, model: &str, fallback: Option<&str>) -> Self {
        Self {
            kind: AiErrorKind::ModelNotFound,
            message: format!("Modell '{}' nicht verfügbar.", model),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: fallback.map(String::from),
        }
    }

    pub fn server_error(provider: &str, model: &str, details: &str) -> Self {
        Self {
            kind: AiErrorKind::ServerError,
            message: format!("Server-Fehler bei {}: {}", provider, details),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: Some(5),
            fallback_model: None,
        }
    }

    pub fn network_error(provider: &str, model: &str, details: &str) -> Self {
        Self {
            kind: AiErrorKind::NetworkError,
            message: format!("Netzwerkfehler: {}", details),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: Some(3),
            fallback_model: None,
        }
    }

    pub fn other(provider: &str, model: &str, message: &str) -> Self {
        Self {
            kind: AiErrorKind::Other,
            message: message.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            retry_after_secs: None,
            fallback_model: None,
        }
    }
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AiError {}

/// Parse retry delay from error response (supports "4s", "4.5s", seconds as number)
pub fn parse_retry_delay(text: &str) -> Option<u32> {
    // Try to find "retryDelay": "Xs" pattern
    if let Some(idx) = text.find("retryDelay") {
        let after = &text[idx..];
        // Look for number followed by 's'
        for word in after.split_whitespace().take(5) {
            let clean = word.trim_matches(|c: char| !c.is_numeric() && c != '.');
            if let Ok(secs) = clean.parse::<f64>() {
                return Some(secs.ceil() as u32);
            }
        }
    }
    // Try to find "retry in X" pattern
    if let Some(idx) = text.find("retry in") {
        let after = &text[idx + 8..];
        for word in after.split_whitespace().take(3) {
            let clean = word.trim_matches(|c: char| !c.is_numeric() && c != '.');
            if let Ok(secs) = clean.parse::<f64>() {
                return Some(secs.ceil() as u32);
            }
        }
    }
    None
}

/// Get fallback model for a given provider and model
pub fn get_fallback_model(provider: &str, current_model: &str) -> Option<&'static str> {
    match provider {
        "gemini" | "Gemini" => {
            // Gemini fallback chain: pro -> flash -> 2.5-flash
            if current_model.contains("pro") {
                Some("gemini-2.0-flash")
            } else if current_model.contains("2.5") || current_model.contains("3") {
                Some("gemini-2.0-flash")
            } else {
                None
            }
        }
        "claude" | "Claude" => {
            // Claude fallback: opus -> sonnet -> haiku
            if current_model.contains("opus") {
                Some("claude-sonnet-4-5-20250514")
            } else if current_model.contains("sonnet") {
                Some("claude-haiku-4-5-20251015")
            } else {
                None
            }
        }
        "openai" | "OpenAI" => {
            // OpenAI fallback: 4.1 -> 4o -> 4o-mini
            if current_model.contains("4.1") && !current_model.contains("mini") {
                Some("gpt-4.1-mini")
            } else if current_model.contains("4o") && !current_model.contains("mini") {
                Some("gpt-4o-mini")
            } else {
                None
            }
        }
        "perplexity" | "Perplexity" => {
            // Perplexity fallback: reasoning -> pro -> sonar
            if current_model.contains("reasoning") {
                Some("sonar-pro")
            } else if current_model.contains("pro") {
                Some("sonar")
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Request for chart analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartAnalysisRequest {
    pub image_base64: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub context: ChartContext,
}

/// Context about the chart being analyzed
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartContext {
    pub security_name: String,
    pub ticker: Option<String>,
    pub currency: String,
    pub current_price: f64,
    pub timeframe: String,
    pub indicators: Vec<String>,
}

/// Response from AI analysis
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartAnalysisResponse {
    pub analysis: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

// ============================================================================
// Request Configuration Constants
// ============================================================================

/// Request timeout in seconds
pub const REQUEST_TIMEOUT_SECS: u64 = 60;

/// Maximum retries for transient errors
pub const MAX_RETRIES: u32 = 2;

/// Base delay for exponential backoff (milliseconds)
pub const RETRY_BASE_DELAY_MS: u64 = 1000;

/// Maximum tokens for response
pub const MAX_TOKENS: u32 = 1500;

// ============================================================================
// Prompt Templates (Tiered by Model Capability)
// ============================================================================

/// Determine if a model is a "fast" tier (haiku, mini, flash, sonar base)
pub fn is_fast_model(model: &str) -> bool {
    model.contains("haiku") ||
    model.contains("mini") ||
    model.contains("flash") ||
    // Perplexity: base sonar is fast, pro/reasoning are not
    (model == "sonar" || model.ends_with("sonar"))
}

/// Build the analysis prompt with chart context.
/// Uses a shorter prompt for fast/cheap models to reduce token usage.
pub fn build_analysis_prompt(ctx: &ChartContext, model: &str) -> String {
    let indicators_str = if ctx.indicators.is_empty() {
        "Keine".to_string()
    } else {
        ctx.indicators.join(", ")
    };

    if is_fast_model(model) {
        // Compact prompt for fast/cheap models (~40% fewer tokens)
        format!(
            r#"Technische Chart-Analyse für {} ({}).
Kurs: {:.2} {} | Zeitraum: {} | Indikatoren: {}

WICHTIG: Verwende EXAKT dieses Markdown-Format mit ## für Überschriften:

## Trend
[Aufwärts/Abwärts/Seitwärts + Stärke]

## Support/Widerstand
**S:** [Levels] | **R:** [Levels]

## Muster
[Formation oder "Keine"]

## Signal
[Bullisch/Bärisch/Neutral] - [Begründung]

## Risiko
[1 Hauptrisiko]"#,
            ctx.security_name,
            ctx.ticker.as_deref().unwrap_or("-"),
            ctx.current_price,
            ctx.currency,
            ctx.timeframe,
            indicators_str
        )
    } else {
        // Full prompt for pro/standard models
        format!(
            r#"Du bist ein erfahrener technischer Analyst. Analysiere den beigefügten Chart.

**Wertpapier:** {} ({})
**Zeitraum:** {}
**Aktueller Kurs:** {:.2} {}
**Aktive Indikatoren:** {}

WICHTIG: Antworte in Markdown-Format mit Überschriften im Format: ## Überschrift

## Trend
[1-2 Sätze: Primärer Trend (Aufwärts/Abwärts/Seitwärts), Trendstärke]

## Unterstützung & Widerstand
- **Unterstützung:** [Preisniveau(s)]
- **Widerstand:** [Preisniveau(s)]

## Chartmuster
[1-2 Sätze: Erkennbare Formationen oder Keine eindeutigen Muster erkennbar]

## Indikatoren
[1-2 Sätze zur Interpretation der aktiven Indikatoren, oder Keine Indikatoren aktiv]

## Einschätzung
- **Kurzfristig:** [Bullisch/Bärisch/Neutral] - [1 Satz Begründung]
- **Mittelfristig:** [Bullisch/Bärisch/Neutral] - [1 Satz Begründung]

## Risiken
[1-2 konkrete Risikofaktoren]

Beginne direkt mit der Trend-Überschrift. Keine Einleitung, keine zusätzlichen Abschnitte."#,
            ctx.security_name,
            ctx.ticker.as_deref().unwrap_or("-"),
            ctx.timeframe,
            ctx.current_price,
            ctx.currency,
            indicators_str
        )
    }
}

/// Calculate exponential backoff delay
pub fn calculate_backoff_delay(attempt: u32) -> std::time::Duration {
    let delay_ms = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
    std::time::Duration::from_millis(delay_ms.min(10_000)) // Max 10 seconds
}

/// Normalize AI response to ensure consistent markdown formatting
/// Fixes common issues where AI returns plain text instead of markdown
pub fn normalize_markdown_response(text: &str) -> String {
    let mut result = text.to_string();

    // Common headings that should be ## formatted
    let headings = [
        "Trend",
        "Support/Widerstand",
        "Support & Widerstand",
        "Unterstützung & Widerstand",
        "Unterstützung und Widerstand",
        "Muster",
        "Chartmuster",
        "Signal",
        "Indikatoren",
        "Einschätzung",
        "Risiko",
        "Risiken",
    ];

    for heading in headings {
        // Replace "Heading:" or "Heading\n" at start of line with "## Heading\n"
        // But only if not already prefixed with ##
        let patterns = [
            format!("\n{}:", heading),
            format!("\n{}\n", heading),
            format!("\n{}  \n", heading), // With trailing spaces
        ];

        for pattern in patterns {
            if result.contains(&pattern) && !result.contains(&format!("\n## {}", heading)) {
                result = result.replace(&pattern, &format!("\n## {}\n", heading));
            }
        }

        // Handle start of string
        if result.starts_with(&format!("{}:", heading)) || result.starts_with(&format!("{}\n", heading)) {
            if !result.starts_with("## ") {
                result = format!("## {}\n{}", heading, &result[heading.len()..].trim_start_matches(':').trim_start());
            }
        }
    }

    // Ensure there's a newline before ## if not at start
    result = result.replace("\n##", "\n\n##");
    result = result.replace("\n\n\n##", "\n\n##"); // Remove triple newlines

    // Remove any citations like [1], [2], etc. that Perplexity adds
    let citation_regex = regex::Regex::new(r"\[\d+\]").unwrap();
    result = citation_regex.replace_all(&result, "").to_string();

    result.trim().to_string()
}

// ============================================================================
// Model Listing API
// ============================================================================

/// Available AI model info
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub supports_vision: bool,
}

/// Fetch available models from Claude API
pub async fn list_claude_models(api_key: &str) -> Result<Vec<AiModelInfo>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|e| anyhow!("Invalid API key: {}", e))?,
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_static("2023-06-01"),
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client
        .get("https://api.anthropic.com/v1/models")
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Claude API error: {}", body));
    }

    #[derive(Deserialize)]
    struct ClaudeModel {
        id: String,
        display_name: Option<String>,
    }

    #[derive(Deserialize)]
    struct ClaudeModelsResponse {
        data: Vec<ClaudeModel>,
    }

    let data: ClaudeModelsResponse = response.json().await?;

    // Filter to vision-capable models (claude-3 and newer support vision)
    let models: Vec<AiModelInfo> = data
        .data
        .into_iter()
        .filter(|m| {
            m.id.contains("claude-3") ||
            m.id.contains("claude-sonnet-4") ||
            m.id.contains("claude-opus-4") ||
            m.id.contains("claude-haiku-4")
        })
        .map(|m| {
            let description = if m.id.contains("opus") {
                "Beste Qualität"
            } else if m.id.contains("sonnet") {
                "Ausgewogen"
            } else if m.id.contains("haiku") {
                "Schnell & günstig"
            } else {
                "Vision-fähig"
            };
            AiModelInfo {
                name: m.display_name.unwrap_or_else(|| m.id.clone()),
                id: m.id,
                description: description.to_string(),
                supports_vision: true,
            }
        })
        .collect();

    Ok(models)
}

/// Fetch available models from OpenAI API
pub async fn list_openai_models(api_key: &str) -> Result<Vec<AiModelInfo>> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))
            .map_err(|e| anyhow!("Invalid API key: {}", e))?,
    );

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client
        .get("https://api.openai.com/v1/models")
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("OpenAI API error: {}", body));
    }

    #[derive(Deserialize)]
    struct OpenAIModel {
        id: String,
    }

    #[derive(Deserialize)]
    struct OpenAIModelsResponse {
        data: Vec<OpenAIModel>,
    }

    let data: OpenAIModelsResponse = response.json().await?;

    // Filter to vision-capable chat models
    let vision_models = ["gpt-4o", "gpt-4-turbo", "gpt-4.1", "o3", "o4"];
    let models: Vec<AiModelInfo> = data
        .data
        .into_iter()
        .filter(|m| {
            vision_models.iter().any(|v| m.id.starts_with(v)) &&
            !m.id.contains("audio") &&
            !m.id.contains("realtime")
        })
        .map(|m| {
            let description = if m.id.contains("mini") {
                "Schnell & günstig"
            } else if m.id.starts_with("o3") || m.id.starts_with("o4") {
                "Reasoning-Modell"
            } else if m.id.contains("4.1") {
                "Neuestes, 1M Kontext"
            } else {
                "Multimodal"
            };
            AiModelInfo {
                name: m.id.clone(),
                id: m.id,
                description: description.to_string(),
                supports_vision: true,
            }
        })
        .collect();

    Ok(models)
}

/// Fetch available models from Gemini API
pub async fn list_gemini_models(api_key: &str) -> Result<Vec<AiModelInfo>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini API error: {}", body));
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GeminiModel {
        name: String,
        display_name: Option<String>,
        supported_generation_methods: Option<Vec<String>>,
    }

    #[derive(Deserialize)]
    struct GeminiModelsResponse {
        models: Vec<GeminiModel>,
    }

    let data: GeminiModelsResponse = response.json().await?;

    // Filter to models that support generateContent (i.e., can process images)
    let models: Vec<AiModelInfo> = data
        .models
        .into_iter()
        .filter(|m| {
            // Only include gemini models that support generateContent
            m.supported_generation_methods
                .as_ref()
                .map(|methods| methods.contains(&"generateContent".to_string()))
                .unwrap_or(false)
                && (m.name.contains("gemini-2") || m.name.contains("gemini-3"))
                && !m.name.contains("aqa")
                && !m.name.contains("embedding")
        })
        .map(|m| {
            // Extract model ID from "models/gemini-xxx" format
            let id = m.name.replace("models/", "");
            let description = if id.contains("pro") {
                if id.contains("preview") { "Beste Qualität (Preview)" } else { "Beste Qualität" }
            } else if id.contains("flash") {
                if id.contains("lite") { "Ultra-schnell" } else { "Schnell" }
            } else {
                "Vision-fähig"
            };
            AiModelInfo {
                name: m.display_name.unwrap_or_else(|| id.clone()),
                id,
                description: description.to_string(),
                supports_vision: true,
            }
        })
        .collect();

    Ok(models)
}

/// Fetch available models from Perplexity API
/// Note: Perplexity doesn't have a models endpoint, so we return hardcoded models
pub async fn list_perplexity_models(_api_key: &str) -> Result<Vec<AiModelInfo>> {
    // Perplexity doesn't expose a models list API, so we use known Sonar models
    // Updated January 2025 - sonar-reasoning was deprecated
    Ok(vec![
        AiModelInfo {
            id: "sonar-pro".to_string(),
            name: "Sonar Pro".to_string(),
            description: "Beste Qualität + Web-Suche".to_string(),
            supports_vision: true,
        },
        AiModelInfo {
            id: "sonar".to_string(),
            name: "Sonar".to_string(),
            description: "Schnell + Web-Suche".to_string(),
            supports_vision: true,
        },
        AiModelInfo {
            id: "sonar-reasoning-pro".to_string(),
            name: "Sonar Reasoning Pro".to_string(),
            description: "Reasoning mit CoT".to_string(),
            supports_vision: true,
        },
        AiModelInfo {
            id: "sonar-deep-research".to_string(),
            name: "Sonar Deep Research".to_string(),
            description: "Experten-Recherche".to_string(),
            supports_vision: true,
        },
    ])
}

/// Map of deprecated models to their replacements
pub fn get_model_upgrade(provider: &str, model: &str) -> Option<&'static str> {
    match (provider, model) {
        // Perplexity deprecated models
        ("perplexity", "sonar-reasoning") => Some("sonar-reasoning-pro"),
        // Claude deprecated models
        ("claude", m) if m.contains("claude-3-") => Some("claude-sonnet-4-5-20250514"),
        ("claude", m) if m.contains("claude-2") => Some("claude-sonnet-4-5-20250514"),
        // OpenAI deprecated models
        ("openai", "gpt-4-vision-preview") => Some("gpt-4o"),
        ("openai", "gpt-4-turbo") => Some("gpt-4.1"),
        // Gemini deprecated models
        ("gemini", m) if m.contains("gemini-1") => Some("gemini-2.0-flash"),
        ("gemini", "gemini-pro-vision") => Some("gemini-2.0-flash"),
        _ => None,
    }
}
