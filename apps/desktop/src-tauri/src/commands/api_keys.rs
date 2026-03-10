use serde::Serialize;

use crate::security;

#[derive(Debug, Serialize)]
pub struct ApiKeyValidationResult {
    pub valid: bool,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn validate_api_key(
    provider: String,
    api_key: String,
) -> Result<ApiKeyValidationResult, String> {
    // Rate limit: max 30 validations per minute
    security::check_rate_limit(
        "validate_api_key",
        &security::limits::api_key_validation(),
    )?;

    if api_key.trim().is_empty() {
        return Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("API Key ist leer".to_string()),
        });
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP Client Fehler: {}", e))?;

    match provider.as_str() {
        "brandfetch" => validate_brandfetch(&client, &api_key).await,
        "finnhub" => validate_finnhub(&client, &api_key).await,
        "alphaVantage" => validate_alpha_vantage(&client, &api_key).await,
        "coingecko" => validate_coingecko(&client, &api_key).await,
        "twelveData" => validate_twelve_data(&client, &api_key).await,
        "anthropic" => validate_anthropic(&client, &api_key).await,
        "openai" => validate_openai(&client, &api_key).await,
        "gemini" => validate_gemini(&client, &api_key).await,
        "perplexity" => validate_perplexity(&client, &api_key).await,
        "openrouter" => validate_openrouter(&client, &api_key).await,
        "divvyDiary" => validate_divvydiary(&client, &api_key).await,
        _ => Err(format!("Unbekannter Provider: {}", provider)),
    }
}

async fn validate_brandfetch(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let url = format!(
        "https://cdn.brandfetch.io/google.com/w/30/h/30/icon?c={}",
        api_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    if resp.status().is_success() {
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if content_type.contains("image") {
            Ok(ApiKeyValidationResult {
                valid: true,
                error: None,
            })
        } else {
            Ok(ApiKeyValidationResult {
                valid: false,
                error: Some("Ungültige Client ID".to_string()),
            })
        }
    } else {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        })
    }
}

async fn validate_finnhub(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let resp = client
        .get("https://finnhub.io/api/v1/quote?symbol=AAPL")
        .header("X-Finnhub-Token", api_key)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    if !resp.status().is_success() {
        return Ok(ApiKeyValidationResult {
            valid: false,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        });
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON-Fehler: {}", e))?;

    if json.get("c").is_some() {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    }
}

async fn validate_alpha_vantage(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let url = format!(
        "https://www.alphavantage.co/query?function=GLOBAL_QUOTE&symbol=AAPL&apikey={}",
        api_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    if !resp.status().is_success() {
        return Ok(ApiKeyValidationResult {
            valid: false,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        });
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON-Fehler: {}", e))?;

    if json.get("Error Message").is_some() {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    } else if json.get("Note").is_some() {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Rate Limit erreicht".to_string()),
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    }
}

async fn validate_coingecko(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let resp = client
        .get("https://pro-api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd")
        .header("x-cg-pro-api-key", api_key)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    if !resp.status().is_success() {
        return Ok(ApiKeyValidationResult {
            valid: false,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        });
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON-Fehler: {}", e))?;

    if json.get("bitcoin").is_some() {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    }
}

async fn validate_twelve_data(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let url = format!(
        "https://api.twelvedata.com/quote?symbol=AAPL&apikey={}",
        api_key
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    if !resp.status().is_success() {
        return Ok(ApiKeyValidationResult {
            valid: false,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        });
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON-Fehler: {}", e))?;

    if json.get("code").is_some() {
        let message = json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Ungültiger API Key");
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some(message.to_string()),
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    }
}

async fn validate_anthropic(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251015",
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "hi"}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    let status = resp.status().as_u16();
    if status == 401 || status == 403 {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    }
}

async fn validate_openai(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let body = serde_json::json!({
        "model": "gpt-4o-mini",
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "hi"}]
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    let status = resp.status().as_u16();
    if status == 401 {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    }
}

async fn validate_gemini(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );
    let body = serde_json::json!({
        "contents": [{"parts": [{"text": "hi"}]}],
        "generationConfig": {"maxOutputTokens": 1}
    });

    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    let status = resp.status().as_u16();
    if status == 400 || status == 403 {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    }
}

async fn validate_perplexity(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let body = serde_json::json!({
        "model": "sonar",
        "max_tokens": 1,
        "messages": [{"role": "user", "content": "hi"}]
    });

    let resp = client
        .post("https://api.perplexity.ai/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    let status = resp.status().as_u16();
    if status == 401 {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    }
}

async fn validate_openrouter(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let resp = client
        .get("https://openrouter.ai/api/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    let status = resp.status().as_u16();
    if status == 401 || status == 403 {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some("Ungültiger API Key".to_string()),
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    }
}

async fn validate_divvydiary(
    client: &reqwest::Client,
    api_key: &str,
) -> Result<ApiKeyValidationResult, String> {
    let resp = client
        .get("https://api.divvydiary.com/session")
        .header("X-API-Key", api_key)
        .send()
        .await
        .map_err(|e| format!("Netzwerkfehler: {}", e))?;

    if resp.status().is_success() {
        Ok(ApiKeyValidationResult {
            valid: true,
            error: None,
        })
    } else {
        Ok(ApiKeyValidationResult {
            valid: false,
            error: Some(format!("HTTP {}", resp.status().as_u16())),
        })
    }
}
