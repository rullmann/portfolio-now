//! Central normalization for AI responses
//!
//! SINGLE SOURCE OF TRUTH for handling LLM formatting quirks.
//! All AI response parsing should use `normalize_ai_response()` ONCE at the start,
//! then all parsers work with the normalized string.
//!
//! Common LLM formatting issues handled:
//! - `] ]` instead of `]]`
//! - `[[ QUERY_DB` instead of `[[QUERY_DB`
//! - Line breaks inside command markers
//! - Unicode whitespace instead of ASCII space

use once_cell::sync::Lazy;
use regex::Regex;

// Pre-compiled regexes for better performance
static RE_CLOSE_BRACKET: Lazy<Regex> = Lazy::new(|| Regex::new(r"\]\s+\]").unwrap());
static RE_OPEN_BRACKET: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\s+\[").unwrap());
static RE_COMMAND_MARKERS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\[\[\s*(QUERY_DB|WATCHLIST_ADD|WATCHLIST_REMOVE|TRANSACTION_CREATE|TRANSACTION_DELETE|PORTFOLIO_TRANSFER|QUERY_TRANSACTIONS|QUERY_PORTFOLIO_VALUE|EXTRACTED_TRANSACTIONS)\s*:"
    ).unwrap()
});

/// Normalize AI response for consistent command parsing
///
/// This function should be called ONCE at the start of response processing.
/// It handles common LLM formatting quirks that would otherwise break parsing.
///
/// # Example
/// ```ignore
/// let normalized = normalize_ai_response(&raw_response);
/// let (commands, cleaned) = parse_db_queries(&normalized);
/// ```
pub fn normalize_ai_response(response: &str) -> String {
    let mut result = response.to_string();

    // 1. Normalize line endings (CRLF → LF)
    result = result.replace("\r\n", "\n");

    // 2. Normalize Unicode whitespace to ASCII space (except newlines)
    result = normalize_unicode_whitespace(&result);

    // 3. Fix bracket spacing: "] ]" → "]]", "[ [" → "[["
    //    Also handles "]\n]" and similar patterns
    result = normalize_brackets(&result);

    // 4. Fix command marker spacing: "[[ QUERY_DB :" → "[[QUERY_DB:"
    result = normalize_command_markers(&result);

    result
}

/// Replace Unicode whitespace characters with ASCII space
/// Preserves newlines and regular spaces
fn normalize_unicode_whitespace(s: &str) -> String {
    s.chars()
        .map(|c| {
            // Keep newlines and regular spaces, convert other whitespace to space
            if c.is_whitespace() && c != '\n' && c != ' ' {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// Normalize bracket patterns
/// Handles: "] ]", "]\n]", "] \n]", "[ [", "[\n[" etc.
fn normalize_brackets(s: &str) -> String {
    let result = RE_CLOSE_BRACKET.replace_all(s, "]]");
    RE_OPEN_BRACKET.replace_all(&result, "[[").to_string()
}

/// Normalize command markers
/// Handles: "[[ QUERY_DB :" → "[[QUERY_DB:", "[[QUERY_DB :" → "[[QUERY_DB:" etc.
fn normalize_command_markers(s: &str) -> String {
    RE_COMMAND_MARKERS.replace_all(s, "[[$1:").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_spacing_close() {
        assert_eq!(normalize_ai_response("[[QUERY_DB:{}] ]"), "[[QUERY_DB:{}]]");
    }

    #[test]
    fn test_bracket_spacing_open() {
        assert_eq!(normalize_ai_response("[ [QUERY_DB:{}]]"), "[[QUERY_DB:{}]]");
    }

    #[test]
    fn test_bracket_newline() {
        assert_eq!(normalize_ai_response("[[QUERY_DB:{}]\n]"), "[[QUERY_DB:{}]]");
    }

    #[test]
    fn test_bracket_space_newline() {
        assert_eq!(normalize_ai_response("[[QUERY_DB:{}] \n]"), "[[QUERY_DB:{}]]");
    }

    #[test]
    fn test_marker_spacing() {
        assert_eq!(
            normalize_ai_response("[[ QUERY_DB :{}]]"),
            "[[QUERY_DB:{}]]"
        );
    }

    #[test]
    fn test_marker_spacing_watchlist() {
        assert_eq!(
            normalize_ai_response("[[ WATCHLIST_ADD :{\"watchlist\":\"Test\"}]]"),
            "[[WATCHLIST_ADD:{\"watchlist\":\"Test\"}]]"
        );
    }

    #[test]
    fn test_marker_spacing_transaction() {
        assert_eq!(
            normalize_ai_response("[[ TRANSACTION_CREATE :{\"type\":\"BUY\"}]]"),
            "[[TRANSACTION_CREATE:{\"type\":\"BUY\"}]]"
        );
    }

    #[test]
    fn test_preserves_json_content() {
        let input = r#"[[QUERY_DB:{"params":{"key":"value with spaces"}}]]"#;
        assert_eq!(normalize_ai_response(input), input);
    }

    #[test]
    fn test_preserves_response_text() {
        let input = "Ich habe die Daten abgefragt.\n\n[[QUERY_DB:{}]]\n\nDas war's!";
        let expected = "Ich habe die Daten abgefragt.\n\n[[QUERY_DB:{}]]\n\nDas war's!";
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_multiple_commands() {
        let input = "[[ QUERY_DB :{}] ]\n[[ WATCHLIST_ADD :{\"w\":\"x\"}] ]";
        let expected = "[[QUERY_DB:{}]]\n[[WATCHLIST_ADD:{\"w\":\"x\"}]]";
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_crlf_normalization() {
        let input = "Line1\r\nLine2\r\n[[QUERY_DB:{}]]";
        let expected = "Line1\nLine2\n[[QUERY_DB:{}]]";
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_unicode_whitespace() {
        // Non-breaking space (U+00A0) and other Unicode whitespace
        let input = "[[QUERY_DB:\u{00A0}{}]]"; // NBSP before {}
        let expected = "[[QUERY_DB: {}]]"; // Converted to regular space
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_complex_real_world() {
        // Simulates real AI output with multiple formatting issues
        let input = r#"Ich frage die Daten ab.

[[ QUERY_DB :{"template":"securities_in_multiple_portfolios","params":{"min_portfolios":2}}] ]

Hier sind die Ergebnisse."#;
        let expected = r#"Ich frage die Daten ab.

[[QUERY_DB:{"template":"securities_in_multiple_portfolios","params":{"min_portfolios":2}}]]

Hier sind die Ergebnisse."#;
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_extracted_transactions_marker() {
        assert_eq!(
            normalize_ai_response("[[ EXTRACTED_TRANSACTIONS :{\"transactions\":[]}]]"),
            "[[EXTRACTED_TRANSACTIONS:{\"transactions\":[]}]]"
        );
    }

    #[test]
    fn test_query_transactions_marker() {
        assert_eq!(
            normalize_ai_response("[[ QUERY_TRANSACTIONS :{\"security\":\"Apple\"}]]"),
            "[[QUERY_TRANSACTIONS:{\"security\":\"Apple\"}]]"
        );
    }

    #[test]
    fn test_query_portfolio_value_marker() {
        assert_eq!(
            normalize_ai_response("[[ QUERY_PORTFOLIO_VALUE :{\"date\":\"2024-01-01\"}]]"),
            "[[QUERY_PORTFOLIO_VALUE:{\"date\":\"2024-01-01\"}]]"
        );
    }

    #[test]
    fn test_empty_response() {
        assert_eq!(normalize_ai_response(""), "");
    }

    #[test]
    fn test_no_commands() {
        let input = "Das ist eine normale Antwort ohne Commands.";
        assert_eq!(normalize_ai_response(input), input);
    }
}
