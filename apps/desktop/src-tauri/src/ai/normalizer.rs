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
        r"(?i)\[\[\s*(QUERY[\s_]*DB|WATCHLIST[\s_]*ADD|WATCHLIST[\s_]*REMOVE|TRANSACTION[\s_]*CREATE|TRANSACTION[\s_]*DELETE|PORTFOLIO[\s_]*TRANSFER|QUERY[\s_]*TRANSACTIONS|QUERY[\s_]*PORTFOLIO[\s_]*VALUE|EXTRACTED[\s_]*TRANSACTIONS|STRUCTURED[\s_]*QUERY|SCREENER[\s_]*RUN)\s*:"
    ).unwrap()
});
// Fixes LLM bug: single closing bracket after JSON (e.g., [[COMMAND:{...}] instead of [[COMMAND:{...}]])

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

    // 3. Normalize Unicode brackets to ASCII (e.g., 【】 → [[]])
    result = normalize_unicode_brackets(&result);

    // 4. Fix bracket spacing: "] ]" → "]]", "[ [" → "[["
    //    Also handles "]\n]" and similar patterns
    result = normalize_brackets(&result);

    // 5. Fix command marker spacing: "[[ QUERY_DB :" → "[[QUERY_DB:"
    result = normalize_command_markers(&result);

    // 6. Fix single closing bracket: [[COMMAND:{...}] → [[COMMAND:{...}]]
    result = fix_single_close_bracket(&result);

    result
}

/// Fixes LLM bug where command tags have only one closing bracket
/// Example: [[COMMAND:{...}] → [[COMMAND:{...}]]
///
/// Uses brace counting to only fix `}]` where `}` closes the outermost JSON object,
/// avoiding false positives on nested JSON arrays like `{"filters":[{...}],"mode":"..."}`.
fn fix_single_close_bracket(s: &str) -> String {
    static RE_COMMAND_END: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"\}\]").unwrap()
    });

    let mut result = String::with_capacity(s.len() + 10);
    let mut last_end = 0;

    for mat in RE_COMMAND_END.find_iter(s) {
        let match_end = mat.end();
        // Check if next char is NOT a ]
        let next_char = s.get(match_end..match_end + 1);
        let needs_fix = next_char != Some("]");

        // Verify we're inside a command tag and this } closes the outermost JSON
        let before = &s[..mat.start()];
        let in_command_and_outermost = {
            if let Some(bracket_pos) = before.rfind("[[") {
                let between = &before[bracket_pos..];
                if between.contains(":") && !between.contains("]]") {
                    // Found [[COMMAND:... — now count braces from the colon to see if
                    // this } closes the outermost object
                    if let Some(colon_offset) = between.find(':') {
                        let json_region = &between[colon_offset + 1..];
                        let mut depth = 0;
                        for ch in json_region.chars() {
                            match ch {
                                '{' => depth += 1,
                                '}' => depth -= 1,
                                _ => {}
                            }
                        }
                        // json_region goes up to mat.start() and doesn't include the
                        // matched }. So depth==1 means the matched } closes the outermost.
                        depth == 1
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        if needs_fix && in_command_and_outermost {
            result.push_str(&s[last_end..match_end]);
            result.push(']'); // Add the missing ]
            last_end = match_end;
        }
    }

    result.push_str(&s[last_end..]);
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

/// Normalize Unicode bracket characters to ASCII equivalents
/// Handles CJK brackets that some LLMs may output
fn normalize_unicode_brackets(s: &str) -> String {
    s.replace('【', "[")   // U+3010 Left Black Lenticular Bracket
     .replace('】', "]")   // U+3011 Right Black Lenticular Bracket
     .replace('［', "[")   // U+FF3B Fullwidth Left Square Bracket
     .replace('］', "]")   // U+FF3D Fullwidth Right Square Bracket
     .replace('「', "[")   // U+300C Left Corner Bracket
     .replace('」', "]")   // U+300D Right Corner Bracket
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
    RE_COMMAND_MARKERS
        .replace_all(s, |caps: &regex::Captures| {
            let raw = &caps[1];
            let canonical = canonicalize_command_marker(raw);
            format!("[[{}:", canonical)
        })
        .to_string()
}

/// Normalize command marker variants to canonical UPPER_SNAKE_CASE
fn canonicalize_command_marker(raw: &str) -> String {
    let mut out = String::new();
    let mut last_underscore = false;

    for ch in raw.chars() {
        if ch.is_ascii_whitespace() || ch == '_' || ch == '-' {
            if !last_underscore {
                out.push('_');
                last_underscore = true;
            }
        } else {
            out.push(ch.to_ascii_uppercase());
            last_underscore = false;
        }
    }

    out
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

    #[test]
    fn test_unicode_brackets_cjk() {
        // Test CJK lenticular brackets (U+3010, U+3011)
        let input = "【【QUERY_DB:{}】】";
        let expected = "[[QUERY_DB:{}]]";
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_unicode_brackets_fullwidth() {
        // Test fullwidth brackets (U+FF3B, U+FF3D)
        let input = "［［STRUCTURED_QUERY:{}］］";
        let expected = "[[STRUCTURED_QUERY:{}]]";
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_unicode_brackets_corner() {
        // Test corner brackets (U+300C, U+300D)
        let input = "「「QUERY_DB:{}」」";
        let expected = "[[QUERY_DB:{}]]";
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_structured_query_marker() {
        assert_eq!(
            normalize_ai_response("[[ STRUCTURED_QUERY :{\"from\":\"transactions\"}]]"),
            "[[STRUCTURED_QUERY:{\"from\":\"transactions\"}]]"
        );
    }

    #[test]
    fn test_structured_query_marker_lowercase() {
        assert_eq!(
            normalize_ai_response("[[ structured_query :{\"from\":\"transactions\"}]]"),
            "[[STRUCTURED_QUERY:{\"from\":\"transactions\"}]]"
        );
    }

    #[test]
    fn test_structured_query_marker_with_space() {
        assert_eq!(
            normalize_ai_response("[[ STRUCTURED QUERY :{\"from\":\"transactions\"}]]"),
            "[[STRUCTURED_QUERY:{\"from\":\"transactions\"}]]"
        );
    }

    #[test]
    fn test_single_close_bracket() {
        // LLM sometimes generates only one closing bracket
        let input = r#"[[STRUCTURED_QUERY:{"from":"transactions"}]"#;
        let expected = r#"[[STRUCTURED_QUERY:{"from":"transactions"}]]"#;
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_single_close_bracket_query_db() {
        let input = r#"[[QUERY_DB:{"query":"all_dividends"}]"#;
        let expected = r#"[[QUERY_DB:{"query":"all_dividends"}]]"#;
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_single_close_bracket_with_text() {
        let input = r#"Hier sind die Ergebnisse:

[[STRUCTURED_QUERY:{"from":"transactions","where":{"type":"BUY"}}]

Das sind alle Käufe."#;
        let expected = r#"Hier sind die Ergebnisse:

[[STRUCTURED_QUERY:{"from":"transactions","where":{"type":"BUY"}}]]

Das sind alle Käufe."#;
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_double_close_bracket_unchanged() {
        // Already correct - should not add extra bracket
        let input = r#"[[STRUCTURED_QUERY:{"from":"transactions"}]]"#;
        assert_eq!(normalize_ai_response(input), input);
    }

    #[test]
    fn test_screener_run_with_nested_arrays() {
        // SCREENER_RUN contains JSON arrays - must NOT add extra ] inside JSON
        let input = r#"[[SCREENER_RUN:{"filters":[{"type":"rsi","operator":"lt","value":[50,70]}],"mode":"market","universe":"DAX"}]]"#;
        assert_eq!(normalize_ai_response(input), input);
    }

    #[test]
    fn test_screener_run_single_bracket_fix() {
        // Single closing bracket should be fixed
        let input = r#"[[SCREENER_RUN:{"filters":[],"mode":"market"}]"#;
        let expected = r#"[[SCREENER_RUN:{"filters":[],"mode":"market"}]]"#;
        assert_eq!(normalize_ai_response(input), expected);
    }

    #[test]
    fn test_screener_run_already_correct() {
        let input = r#"[[SCREENER_RUN:{"filters":[{"type":"rsi","operator":"lt","value":30}],"mode":"market","universe":"DAX"}]]"#;
        assert_eq!(normalize_ai_response(input), input);
    }
}
