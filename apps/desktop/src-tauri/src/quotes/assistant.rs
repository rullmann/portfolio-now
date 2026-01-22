//! Quote Source Assistant
//!
//! AI-powered quote source finder with validation.
//! Parses AI suggestions and validates them by fetching a test quote.

use crate::ai::types::{AiQuoteSuggestion, ValidatedQuoteSuggestion};
use crate::quotes::{self, ProviderType, SecurityQuoteRequest};
use anyhow::{anyhow, Result};
use regex::Regex;

/// Parse AI suggestion from response text
///
/// The AI response should contain a JSON block with the suggestion.
/// We extract it using regex to be robust against surrounding text.
pub fn parse_ai_suggestion(response: &str) -> Result<AiQuoteSuggestion> {
    // Try to find JSON in code block first
    let json_regex = Regex::new(r"```(?:json)?\s*\n?([\s\S]*?)\n?```")
        .map_err(|e| anyhow!("Regex error: {}", e))?;

    let json_str = if let Some(caps) = json_regex.captures(response) {
        caps.get(1).map(|m| m.as_str().trim()).unwrap_or("")
    } else {
        // Try to find raw JSON object by looking for { followed by "provider"
        // We need to find balanced braces
        if let Some(start) = response.find('{') {
            let rest = &response[start..];
            // Find matching closing brace
            let mut depth = 0;
            let mut end = 0;
            for (i, c) in rest.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if end > 0 && rest[..end].contains("\"provider\"") {
                &response[start..start + end]
            } else {
                return Err(anyhow!("No JSON suggestion found in AI response"));
            }
        } else {
            return Err(anyhow!("No JSON suggestion found in AI response"));
        }
    };

    // Parse the JSON
    let suggestion: AiQuoteSuggestion = serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Failed to parse JSON suggestion: {} - JSON: {}", e, json_str))?;

    // Validate required fields
    if suggestion.provider.is_empty() {
        return Err(anyhow!("Provider is empty"));
    }
    if suggestion.ticker.is_empty() {
        return Err(anyhow!("Ticker is empty"));
    }

    Ok(suggestion)
}

/// Validate a quote suggestion by fetching a test quote
///
/// This attempts to fetch a quote using the suggested configuration.
/// If successful, returns validated=true with the test price.
/// If failed, returns validated=false with the error.
pub async fn validate_suggestion(
    suggestion: &AiQuoteSuggestion,
    api_keys: Option<&crate::commands::quotes::ApiKeys>,
) -> ValidatedQuoteSuggestion {
    // Map provider string to ProviderType
    let provider = match ProviderType::from_str(&suggestion.provider) {
        Some(p) => p,
        None => {
            return ValidatedQuoteSuggestion {
                suggestion: suggestion.clone(),
                validated: false,
                test_price: None,
                test_date: None,
                test_currency: None,
                validation_error: Some(format!("Unknown provider: {}", suggestion.provider)),
            };
        }
    };

    // Skip validation for providers that require API keys we don't have
    let keys = api_keys.cloned().unwrap_or_default();
    let api_key = match provider {
        ProviderType::Finnhub => {
            if keys.finnhub.is_none() {
                return ValidatedQuoteSuggestion {
                    suggestion: suggestion.clone(),
                    validated: false,
                    test_price: None,
                    test_date: None,
                    test_currency: None,
                    validation_error: Some("Finnhub API key required".to_string()),
                };
            }
            keys.finnhub.clone()
        }
        ProviderType::AlphaVantage => {
            if keys.alpha_vantage.is_none() {
                return ValidatedQuoteSuggestion {
                    suggestion: suggestion.clone(),
                    validated: false,
                    test_price: None,
                    test_date: None,
                    test_currency: None,
                    validation_error: Some("Alpha Vantage API key required".to_string()),
                };
            }
            keys.alpha_vantage.clone()
        }
        ProviderType::TwelveData => {
            if keys.twelve_data.is_none() {
                return ValidatedQuoteSuggestion {
                    suggestion: suggestion.clone(),
                    validated: false,
                    test_price: None,
                    test_date: None,
                    test_currency: None,
                    validation_error: Some("Twelve Data API key required".to_string()),
                };
            }
            keys.twelve_data.clone()
        }
        ProviderType::CoinGecko => keys.coingecko.clone(),
        _ => None,
    };

    // Build the request
    let request = SecurityQuoteRequest {
        id: 0, // Dummy ID for test
        symbol: suggestion.ticker.clone(),
        provider,
        feed_url: suggestion.feed_url.clone(),
        api_key,
        currency: None,
    };

    // Fetch the quote
    let results = quotes::fetch_all_quotes(vec![request]).await;

    if let Some(result) = results.into_iter().next() {
        if result.success {
            if let Some(latest) = result.latest {
                return ValidatedQuoteSuggestion {
                    suggestion: suggestion.clone(),
                    validated: true,
                    test_price: Some(latest.quote.close),
                    test_date: Some(latest.quote.date.to_string()),
                    test_currency: latest.currency,
                    validation_error: None,
                };
            }
        } else {
            return ValidatedQuoteSuggestion {
                suggestion: suggestion.clone(),
                validated: false,
                test_price: None,
                test_date: None,
                test_currency: None,
                validation_error: result.error,
            };
        }
    }

    ValidatedQuoteSuggestion {
        suggestion: suggestion.clone(),
        validated: false,
        test_price: None,
        test_date: None,
        test_currency: None,
        validation_error: Some("No result from quote fetch".to_string()),
    }
}

/// Get problematic securities (no provider, fetch error, or stale quotes)
pub fn get_problematic_securities(
    conn: &rusqlite::Connection,
    stale_days: i32,
) -> Result<Vec<crate::ai::types::ProblematicSecurity>> {
    use chrono::{Duration, Utc};

    let today = Utc::now().date_naive();
    let stale_threshold = today - Duration::days(stale_days as i64);

    let mut stmt = conn.prepare(
        r#"
        SELECT
            s.id,
            s.name,
            s.isin,
            s.ticker,
            s.currency,
            s.feed,
            s.feed_url,
            lp.date as last_quote_date
        FROM pp_security s
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        WHERE s.is_retired = 0
          AND (
            -- No provider configured
            (s.feed IS NULL OR s.feed = '' OR s.feed = 'MANUAL')
            -- Or has holdings (is relevant)
            OR EXISTS (
                SELECT 1 FROM pp_txn t
                WHERE t.security_id = s.id
                  AND t.owner_type = 'portfolio'
            )
          )
        ORDER BY s.name
        "#,
    )?;

    let mut securities = Vec::new();

    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let isin: Option<String> = row.get(2)?;
        let ticker: Option<String> = row.get(3)?;
        let currency: String = row.get(4)?;
        let feed: Option<String> = row.get(5)?;
        let feed_url: Option<String> = row.get(6)?;
        let last_quote_date: Option<String> = row.get(7)?;

        Ok((id, name, isin, ticker, currency, feed, feed_url, last_quote_date))
    })?;

    for row in rows {
        let (id, name, isin, ticker, currency, feed, feed_url, last_quote_date) = row?;

        // Determine problem type
        let (problem_type, problem_description) = if feed.is_none() || feed.as_deref() == Some("") || feed.as_deref() == Some("MANUAL") {
            ("no_provider".to_string(), "Kein Kursanbieter konfiguriert".to_string())
        } else if last_quote_date.is_none() {
            ("fetch_error".to_string(), "Noch nie Kurse abgerufen".to_string())
        } else {
            // Check if stale
            let quote_date = chrono::NaiveDate::parse_from_str(
                last_quote_date.as_deref().unwrap_or(""),
                "%Y-%m-%d"
            ).ok();

            if let Some(qd) = quote_date {
                if qd < stale_threshold {
                    let days = (today - qd).num_days();
                    ("stale".to_string(), format!("Kurs {} Tage alt", days))
                } else {
                    continue; // Not problematic
                }
            } else {
                continue; // Can't determine, skip
            }
        };

        securities.push(crate::ai::types::ProblematicSecurity {
            id,
            name,
            isin,
            ticker,
            currency,
            feed,
            feed_url,
            problem_type,
            problem_description,
            last_quote_date,
        });
    }

    Ok(securities)
}

/// Apply a validated suggestion to a security
pub fn apply_suggestion(
    conn: &rusqlite::Connection,
    security_id: i64,
    suggestion: &AiQuoteSuggestion,
) -> Result<()> {
    // Update both latest_feed/latest_feed_url (for current quotes)
    // and feed/feed_url (for historical)
    conn.execute(
        r#"
        UPDATE pp_security
        SET feed = ?1,
            feed_url = ?2,
            latest_feed = ?1,
            latest_feed_url = ?2,
            ticker = COALESCE(ticker, ?3)
        WHERE id = ?4
        "#,
        rusqlite::params![
            &suggestion.provider,
            &suggestion.feed_url,
            &suggestion.ticker,
            security_id
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ai_suggestion_with_code_block() {
        let response = r#"
Für Nestlé mit Schweizer ISIN verwende ich Yahoo Finance mit dem SIX-Suffix.

```json
{
  "provider": "YAHOO",
  "ticker": "NESN",
  "feed_url": ".SW",
  "confidence": 0.95,
  "reason": "CH-ISIN → SIX Swiss Exchange (.SW)"
}
```
"#;

        let suggestion = parse_ai_suggestion(response).unwrap();
        assert_eq!(suggestion.provider, "YAHOO");
        assert_eq!(suggestion.ticker, "NESN");
        assert_eq!(suggestion.feed_url, Some(".SW".to_string()));
        assert_eq!(suggestion.confidence, 0.95);
    }

    #[test]
    fn test_parse_ai_suggestion_without_code_block() {
        let response = r#"
Hier ist mein Vorschlag:
{
  "provider": "COINGECKO",
  "ticker": "bitcoin",
  "feed_url": "EUR",
  "confidence": 0.9,
  "reason": "Bitcoin cryptocurrency"
}
"#;

        let suggestion = parse_ai_suggestion(response).unwrap();
        assert_eq!(suggestion.provider, "COINGECKO");
        assert_eq!(suggestion.ticker, "bitcoin");
    }

    #[test]
    fn test_parse_ai_suggestion_no_json() {
        let response = "I don't know what to suggest.";
        assert!(parse_ai_suggestion(response).is_err());
    }
}
