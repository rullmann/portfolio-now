//! AI Fallback for Symbol Validation
//!
//! Uses AI to suggest quote source configurations when code-based validation fails.

use super::types::{AiConfig, AiSuggestion, ProviderSearchResult, SecurityForValidation};
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Get AI suggestion for a security's quote configuration
pub async fn get_ai_suggestion(
    security: &SecurityForValidation,
    provider_results: &[ProviderSearchResult],
    ai_config: &AiConfig,
) -> Result<AiSuggestion> {
    let prompt = build_validation_prompt(security, provider_results);

    let response_text = match ai_config.provider.as_str() {
        "claude" => call_claude(&prompt, &ai_config.model, &ai_config.api_key).await?,
        "openai" => call_openai(&prompt, &ai_config.model, &ai_config.api_key).await?,
        "gemini" => call_gemini(&prompt, &ai_config.model, &ai_config.api_key).await?,
        "perplexity" => call_perplexity(&prompt, &ai_config.model, &ai_config.api_key).await?,
        _ => return Err(anyhow!("Unknown AI provider: {}", ai_config.provider)),
    };

    parse_ai_suggestion(&response_text)
}

/// Build the prompt for AI validation
fn build_validation_prompt(security: &SecurityForValidation, provider_results: &[ProviderSearchResult]) -> String {
    let provider_results_json = serde_json::to_string_pretty(provider_results).unwrap_or_default();

    format!(
        r#"Du bist ein Experte für Finanzdaten-Provider und Börsensymbole.

Analysiere dieses Wertpapier und schlage die beste Kursquellen-Konfiguration vor.

## Wertpapier:
- Name: {name}
- ISIN: {isin}
- WKN: {wkn}
- Aktueller Ticker: {ticker}
- Währung: {currency}

## Provider-Suchergebnisse:
{provider_results}

## Verfügbare Provider:
- YAHOO: Für Aktien, ETFs. Internationale Börsen mit Suffix (z.B. .DE für Deutschland, .SW für Schweiz, .L für London). US-Aktien ohne Suffix.
- TRADINGVIEW: Format EXCHANGE:SYMBOL (z.B. XETR:SAP). Gut für exotische Märkte.
- COINGECKO: Für Kryptowährungen. Symbol ist die CoinGecko coin_id (z.B. "bitcoin", "ethereum").
- PORTFOLIO_REPORT: Für deutsche Fonds/ETFs per UUID. Verwendet ISIN zur Suche.

## Spezialfälle:
- "Gold physisch", "Goldbarren", "Xetra-Gold" → YAHOO mit Ticker "GC=F" (Gold Futures) oder TRADINGVIEW mit "TVC:GOLD" oder "OANDA:XAUUSD"
- Kryptowährungen → COINGECKO mit coin_id
- Deutsche Aktien → YAHOO mit .DE Suffix oder TRADINGVIEW mit XETR: Präfix
- Schweizer Aktien → YAHOO mit .SW Suffix oder TRADINGVIEW mit SIX: Präfix

## Antworte NUR im JSON-Format:
{{
  "feed": "YAHOO|TRADINGVIEW|COINGECKO|PORTFOLIO_REPORT",
  "ticker": "Das korrekte Symbol für den gewählten Provider",
  "feed_url": "Exchange-Suffix (.DE) oder Präfix (XETR) oder CoinGecko Währung (EUR)",
  "reasoning": "Kurze Begründung deiner Wahl",
  "confidence": 0.0-1.0
}}

Wichtig:
- Wähle den Provider, der am zuverlässigsten für dieses Wertpapier funktioniert
- Bei Krypto: feed_url ist die Zielwährung (EUR, USD, CHF)
- Bei TradingView: feed_url ist der Exchange-Präfix ohne Doppelpunkt
- Confidence sollte niedrig sein, wenn du unsicher bist"#,
        name = security.name,
        isin = security.isin.as_deref().unwrap_or("Nicht vorhanden"),
        wkn = security.wkn.as_deref().unwrap_or("Nicht vorhanden"),
        ticker = security.ticker.as_deref().unwrap_or("Nicht vorhanden"),
        currency = security.currency,
        provider_results = provider_results_json,
    )
}

/// Parse AI response into AiSuggestion
fn parse_ai_suggestion(response: &str) -> Result<AiSuggestion> {
    // Try to extract JSON from response
    let json_str = extract_json(response).ok_or_else(|| anyhow!("No JSON found in AI response"))?;

    #[derive(Deserialize)]
    struct RawSuggestion {
        feed: String,
        ticker: String,
        feed_url: Option<String>,
        reasoning: Option<String>,
        confidence: Option<f64>,
    }

    let raw: RawSuggestion = serde_json::from_str(&json_str)
        .map_err(|e| anyhow!("Failed to parse AI suggestion JSON: {}", e))?;

    Ok(AiSuggestion {
        feed: raw.feed.to_uppercase(),
        ticker: raw.ticker,
        feed_url: raw.feed_url,
        reasoning: raw.reasoning.unwrap_or_else(|| "No reasoning provided".to_string()),
        confidence: raw.confidence.unwrap_or(0.5),
    })
}

/// Extract JSON from a response that might contain other text
fn extract_json(text: &str) -> Option<String> {
    // Look for JSON object
    if let Some(start) = text.find('{') {
        let mut depth = 0;
        let mut end = start;
        for (i, c) in text[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth == 0 {
            return Some(text[start..end].to_string());
        }
    }
    None
}

// =============================================================================
// Provider implementations
// =============================================================================

/// Call Claude API
async fn call_claude(prompt: &str, model: &str, api_key: &str) -> Result<String> {
    #[derive(Serialize)]
    struct Request {
        model: String,
        max_tokens: u32,
        messages: Vec<Message>,
    }

    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Deserialize)]
    struct Response {
        content: Vec<Content>,
    }

    #[derive(Deserialize)]
    struct Content {
        text: Option<String>,
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|_| anyhow!("Invalid API key"))?,
    );
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;

    let request = Request {
        model: model.to_string(),
        max_tokens: 1024,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Claude API error: {}", body));
    }

    let data: Response = response.json().await?;
    data.content
        .first()
        .and_then(|c| c.text.clone())
        .ok_or_else(|| anyhow!("Empty response from Claude"))
}

/// Call OpenAI API
async fn call_openai(prompt: &str, model: &str, api_key: &str) -> Result<String> {
    #[derive(Serialize)]
    struct Request {
        model: String,
        messages: Vec<Message>,
        max_tokens: u32,
    }

    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Deserialize)]
    struct Response {
        choices: Vec<Choice>,
    }

    #[derive(Deserialize)]
    struct Choice {
        message: ResponseMessage,
    }

    #[derive(Deserialize)]
    struct ResponseMessage {
        content: Option<String>,
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(|_| anyhow!("Invalid API key"))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;

    let request = Request {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: 1024,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("OpenAI API error: {}", body));
    }

    let data: Response = response.json().await?;
    data.choices
        .first()
        .and_then(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("Empty response from OpenAI"))
}

/// Call Gemini API
async fn call_gemini(prompt: &str, model: &str, api_key: &str) -> Result<String> {
    #[derive(Serialize)]
    struct Request {
        contents: Vec<Content>,
    }

    #[derive(Serialize)]
    struct Content {
        parts: Vec<Part>,
    }

    #[derive(Serialize)]
    struct Part {
        text: String,
    }

    #[derive(Deserialize)]
    struct Response {
        candidates: Vec<Candidate>,
    }

    #[derive(Deserialize)]
    struct Candidate {
        content: ResponseContent,
    }

    #[derive(Deserialize)]
    struct ResponseContent {
        parts: Vec<ResponsePart>,
    }

    #[derive(Deserialize)]
    struct ResponsePart {
        text: Option<String>,
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;

    let request = Request {
        contents: vec![Content {
            parts: vec![Part {
                text: prompt.to_string(),
            }],
        }],
    };

    let response = client
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Gemini API error: {}", body));
    }

    let data: Response = response.json().await?;
    data.candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .and_then(|p| p.text.clone())
        .ok_or_else(|| anyhow!("Empty response from Gemini"))
}

/// Call Perplexity API
async fn call_perplexity(prompt: &str, model: &str, api_key: &str) -> Result<String> {
    #[derive(Serialize)]
    struct Request {
        model: String,
        messages: Vec<Message>,
    }

    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Deserialize)]
    struct Response {
        choices: Vec<Choice>,
    }

    #[derive(Deserialize)]
    struct Choice {
        message: ResponseMessage,
    }

    #[derive(Deserialize)]
    struct ResponseMessage {
        content: Option<String>,
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(|_| anyhow!("Invalid API key"))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;

    let request = Request {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post("https://api.perplexity.ai/chat/completions")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Perplexity API error: {}", body));
    }

    let data: Response = response.json().await?;
    data.choices
        .first()
        .and_then(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("Empty response from Perplexity"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json() {
        let text = r#"Here is my analysis:
{
  "feed": "YAHOO",
  "ticker": "AAPL",
  "feed_url": null,
  "reasoning": "Apple is a US stock",
  "confidence": 0.95
}
That's my suggestion."#;

        let json = extract_json(text).unwrap();
        assert!(json.contains("\"feed\": \"YAHOO\""));
    }

    #[test]
    fn test_parse_ai_suggestion() {
        let response = r#"{
  "feed": "YAHOO",
  "ticker": "SAP.DE",
  "feed_url": ".DE",
  "reasoning": "German stock on XETRA",
  "confidence": 0.9
}"#;

        let suggestion = parse_ai_suggestion(response).unwrap();
        assert_eq!(suggestion.feed, "YAHOO");
        assert_eq!(suggestion.ticker, "SAP.DE");
        assert_eq!(suggestion.feed_url, Some(".DE".to_string()));
        assert_eq!(suggestion.confidence, 0.9);
    }
}
