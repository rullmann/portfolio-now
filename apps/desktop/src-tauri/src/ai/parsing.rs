//! AI response parsing and utility functions
//!
//! This module contains functions for parsing AI responses and utility functions
//! for retry logic and markdown normalization.

use anyhow::{anyhow, Result};

use crate::ai::types::{AnnotationAnalysisJson, EnhancedAnnotationAnalysisJson, RETRY_BASE_DELAY_MS};

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

/// Parse JSON response from AI into structured annotations.
/// Handles common AI quirks like markdown code blocks around JSON.
pub fn parse_annotation_response(raw: &str) -> Result<AnnotationAnalysisJson> {
    // Remove markdown code blocks if present
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str(cleaned)
        .map_err(|e| anyhow!("Failed to parse AI JSON response: {}. Raw: {}", e, &raw[..raw.len().min(200)]))
}

/// Parse enhanced JSON response from AI into structured annotations with alerts and risk/reward.
pub fn parse_enhanced_annotation_response(raw: &str) -> Result<EnhancedAnnotationAnalysisJson> {
    // Remove markdown code blocks if present
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    serde_json::from_str(cleaned)
        .map_err(|e| anyhow!("Failed to parse enhanced AI JSON response: {}. Raw: {}", e, &raw[..raw.len().min(200)]))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_retry_delay_retry_delay_format() {
        let text = r#"{"error": {"retryDelay": "4s"}}"#;
        assert_eq!(parse_retry_delay(text), Some(4));
    }

    #[test]
    fn test_parse_retry_delay_decimal() {
        let text = r#"retryDelay: 2.5s"#;
        assert_eq!(parse_retry_delay(text), Some(3)); // Ceiled
    }

    #[test]
    fn test_parse_retry_delay_retry_in_format() {
        let text = "Please retry in 10 seconds";
        assert_eq!(parse_retry_delay(text), Some(10));
    }

    #[test]
    fn test_parse_retry_delay_none() {
        let text = "Some error without delay info";
        assert_eq!(parse_retry_delay(text), None);
    }

    #[test]
    fn test_calculate_backoff_delay() {
        assert_eq!(calculate_backoff_delay(0), std::time::Duration::from_millis(1000));
        assert_eq!(calculate_backoff_delay(1), std::time::Duration::from_millis(2000));
        assert_eq!(calculate_backoff_delay(2), std::time::Duration::from_millis(4000));
        assert_eq!(calculate_backoff_delay(10), std::time::Duration::from_millis(10000)); // Capped at 10s
    }

    #[test]
    fn test_normalize_markdown_removes_citations() {
        let text = "This is analysis [1] with citations [2] in it.";
        let result = normalize_markdown_response(text);
        assert_eq!(result, "This is analysis  with citations  in it.");
    }

    #[test]
    fn test_normalize_markdown_adds_heading_format() {
        let text = "Trend:\nBullish market";
        let result = normalize_markdown_response(text);
        assert!(result.contains("## Trend"));
    }

    #[test]
    fn test_parse_annotation_response_removes_code_blocks() {
        let raw = r#"```json
{"analysis": "test", "trend": {"direction": "bullish", "strength": "strong"}, "annotations": []}
```"#;
        let result = parse_annotation_response(raw);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().analysis, "test");
    }

    #[test]
    fn test_parse_annotation_response_handles_plain_json() {
        let raw = r#"{"analysis": "test", "trend": {"direction": "neutral", "strength": "weak"}, "annotations": []}"#;
        let result = parse_annotation_response(raw);
        assert!(result.is_ok());
    }
}
