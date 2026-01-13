//! ChatBot command parsing and execution
//!
//! This module handles parsing and executing commands embedded in AI responses,
//! such as watchlist modifications and transaction queries.

use crate::commands::ai_helpers;
use regex::Regex;
use serde::Serialize;

// ============================================================================
// Watchlist Commands
// ============================================================================

/// Watchlist command parsed from AI response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistCommand {
    pub action: String, // "add" or "remove"
    pub watchlist: String,
    pub security: String,
}

/// Parse watchlist commands from AI response
///
/// Extracts `[[WATCHLIST_ADD:...]]` and `[[WATCHLIST_REMOVE:...]]` commands
///
/// # Returns
/// Tuple of (commands, cleaned_response_text)
pub fn parse_watchlist_commands(response: &str) -> (Vec<WatchlistCommand>, String) {
    let mut commands = Vec::new();
    let mut cleaned_response = response.to_string();

    // Parse WATCHLIST_ADD commands: [[WATCHLIST_ADD:{"watchlist":"...","security":"..."}]]
    let add_re = Regex::new(r#"\[\[WATCHLIST_ADD:\s*\{[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*"security"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in add_re.captures_iter(response) {
        commands.push(WatchlistCommand {
            action: "add".to_string(),
            watchlist: cap[1].to_string(),
            security: cap[2].to_string(),
        });
    }
    // Also try reversed order
    let add_re2 = Regex::new(r#"\[\[WATCHLIST_ADD:\s*\{[^}]*"security"\s*:\s*"([^"]+)"[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in add_re2.captures_iter(response) {
        let cmd = WatchlistCommand {
            action: "add".to_string(),
            watchlist: cap[2].to_string(),
            security: cap[1].to_string(),
        };
        if !commands.iter().any(|c| c.watchlist == cmd.watchlist && c.security == cmd.security) {
            commands.push(cmd);
        }
    }

    // Parse WATCHLIST_REMOVE commands
    let remove_re = Regex::new(r#"\[\[WATCHLIST_REMOVE:\s*\{[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*"security"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in remove_re.captures_iter(response) {
        commands.push(WatchlistCommand {
            action: "remove".to_string(),
            watchlist: cap[1].to_string(),
            security: cap[2].to_string(),
        });
    }
    let remove_re2 = Regex::new(r#"\[\[WATCHLIST_REMOVE:\s*\{[^}]*"security"\s*:\s*"([^"]+)"[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in remove_re2.captures_iter(response) {
        let cmd = WatchlistCommand {
            action: "remove".to_string(),
            watchlist: cap[2].to_string(),
            security: cap[1].to_string(),
        };
        if !commands.iter().any(|c| c.watchlist == cmd.watchlist && c.security == cmd.security && c.action == cmd.action) {
            commands.push(cmd);
        }
    }

    // Remove all command tags from response
    let clean_re = Regex::new(r#"\[\[WATCHLIST_(ADD|REMOVE):[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim_start().to_string();

    (commands, cleaned_response)
}

/// Execute watchlist commands
pub async fn execute_watchlist_commands(
    commands: &[WatchlistCommand],
    alpha_vantage_api_key: Option<String>,
) -> Vec<String> {
    let mut results = Vec::new();

    for cmd in commands {
        let result = match cmd.action.as_str() {
            "add" => {
                match ai_helpers::ai_add_to_watchlist(
                    cmd.watchlist.clone(),
                    cmd.security.clone(),
                    alpha_vantage_api_key.clone(),
                ).await {
                    Ok(r) => r.message,
                    Err(e) => format!("Fehler: {}", e),
                }
            }
            "remove" => {
                match ai_helpers::ai_remove_from_watchlist(
                    cmd.watchlist.clone(),
                    cmd.security.clone(),
                ) {
                    Ok(r) => r.message,
                    Err(e) => format!("Fehler: {}", e),
                }
            }
            _ => continue,
        };
        results.push(result);
    }

    results
}

// ============================================================================
// Transaction Queries
// ============================================================================

/// Transaction query parsed from AI response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionQuery {
    pub security: Option<String>,
    pub year: Option<i32>,
    pub txn_type: Option<String>,
    pub limit: Option<i32>,
}

/// Parse transaction query commands from AI response
///
/// Extracts `[[QUERY_TRANSACTIONS:...]]` commands
pub fn parse_transaction_queries(response: &str) -> (Vec<TransactionQuery>, String) {
    let mut queries = Vec::new();
    let mut cleaned_response = response.to_string();

    let query_re = Regex::new(r#"\[\[QUERY_TRANSACTIONS:\s*\{([^}]*)\}\]\]"#).unwrap();

    for cap in query_re.captures_iter(response) {
        let json_content = &cap[1];

        let security = Regex::new(r#""security"\s*:\s*"([^"]+)""#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .map(|c| c[1].to_string());

        let year = Regex::new(r#""year"\s*:\s*(\d{4})"#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .and_then(|c| c[1].parse::<i32>().ok());

        let txn_type = Regex::new(r#""type"\s*:\s*"([^"]+)""#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .map(|c| c[1].to_string());

        let limit = Regex::new(r#""limit"\s*:\s*(\d+)"#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .and_then(|c| c[1].parse::<i32>().ok());

        queries.push(TransactionQuery {
            security,
            year,
            txn_type,
            limit,
        });
    }

    let clean_re = Regex::new(r#"\[\[QUERY_TRANSACTIONS:[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim_start().to_string();

    (queries, cleaned_response)
}

/// Execute transaction queries and return formatted results
pub fn execute_transaction_queries(queries: &[TransactionQuery]) -> Vec<String> {
    let mut results = Vec::new();

    for query in queries {
        match ai_helpers::ai_query_transactions(
            query.security.clone(),
            query.year,
            query.txn_type.clone(),
            query.limit,
        ) {
            Ok(result) => {
                if result.transactions.is_empty() {
                    results.push("Keine Transaktionen gefunden.".to_string());
                } else {
                    let mut output = format!("**{}**\n\n", result.message);
                    for txn in &result.transactions {
                        let sec_str = txn.security_name.as_ref()
                            .map(|s| {
                                let ticker = txn.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                                format!(" - {}{}", s, ticker)
                            })
                            .unwrap_or_default();
                        let shares_str = txn.shares.map(|s| format!(", {:.4} Stk.", s)).unwrap_or_default();
                        output.push_str(&format!(
                            "- {}: {}{}, {:.2} {}{}\n",
                            txn.date, txn.txn_type, sec_str, txn.amount, txn.currency, shares_str
                        ));
                    }
                    results.push(output);
                }
            }
            Err(e) => {
                results.push(format!("Fehler bei Transaktionsabfrage: {}", e));
            }
        }
    }

    results
}

// ============================================================================
// Portfolio Value Queries
// ============================================================================

/// Portfolio value query parsed from AI response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioValueQuery {
    pub date: String,
}

/// Parse portfolio value query commands from AI response
///
/// Extracts `[[QUERY_PORTFOLIO_VALUE:...]]` commands
pub fn parse_portfolio_value_queries(response: &str) -> (Vec<PortfolioValueQuery>, String) {
    let mut queries = Vec::new();
    let mut cleaned_response = response.to_string();

    let query_re = Regex::new(r#"\[\[QUERY_PORTFOLIO_VALUE:\s*\{([^}]*)\}\]\]"#).unwrap();

    for cap in query_re.captures_iter(response) {
        let json_content = &cap[1];

        let date = Regex::new(r#""date"\s*:\s*"([^"]+)""#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .map(|c| c[1].to_string());

        if let Some(date) = date {
            queries.push(PortfolioValueQuery { date });
        }
    }

    let clean_re = Regex::new(r#"\[\[QUERY_PORTFOLIO_VALUE:[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim_start().to_string();

    (queries, cleaned_response)
}

/// Execute portfolio value queries and return formatted results
pub fn execute_portfolio_value_queries(queries: &[PortfolioValueQuery]) -> Vec<String> {
    let mut results = Vec::new();

    for query in queries {
        match ai_helpers::ai_query_portfolio_value(query.date.clone()) {
            Ok(result) => {
                if result.found {
                    results.push(format!("**Depotwert am {}:** {:.2} {}", result.date, result.value, result.currency));
                } else {
                    results.push(result.message);
                }
            }
            Err(e) => {
                results.push(format!("Fehler bei Depotwert-Abfrage: {}", e));
            }
        }
    }

    results
}

// ============================================================================
// Combined Response Processing
// ============================================================================

/// Process all embedded commands in AI response
///
/// Parses and executes watchlist, transaction, and portfolio value commands.
/// Returns the cleaned response text with command results appended.
pub async fn process_response_commands(
    response: String,
    alpha_vantage_api_key: Option<String>,
    emit_watchlist_update: impl Fn(),
) -> (String, Vec<String>) {
    let mut current_response = response;
    let mut additional_results: Vec<String> = Vec::new();

    // Parse and execute watchlist commands
    let (wl_commands, cleaned) = parse_watchlist_commands(&current_response);
    current_response = cleaned;

    if !wl_commands.is_empty() {
        let results = execute_watchlist_commands(&wl_commands, alpha_vantage_api_key).await;
        emit_watchlist_update();
        additional_results.extend(results);
    }

    // Parse and execute transaction queries
    let (txn_queries, cleaned) = parse_transaction_queries(&current_response);
    current_response = cleaned;

    if !txn_queries.is_empty() {
        let results = execute_transaction_queries(&txn_queries);
        additional_results.extend(results);
    }

    // Parse and execute portfolio value queries
    let (pv_queries, cleaned) = parse_portfolio_value_queries(&current_response);
    current_response = cleaned;

    if !pv_queries.is_empty() {
        let results = execute_portfolio_value_queries(&pv_queries);
        additional_results.extend(results);
    }

    (current_response, additional_results)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_watchlist_add_standard_format() {
        let response = r#"Ich füge Apple zur Watchlist hinzu.
[[WATCHLIST_ADD:{"watchlist":"Standard","security":"Apple"}]]
Erledigt!"#;

        let (commands, cleaned) = parse_watchlist_commands(response);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "add");
        assert_eq!(commands[0].watchlist, "Standard");
        assert_eq!(commands[0].security, "Apple");
        assert!(!cleaned.contains("WATCHLIST_ADD"));
        assert!(cleaned.contains("Ich füge Apple"));
    }

    #[test]
    fn test_parse_watchlist_add_reversed_order() {
        let response = r#"[[WATCHLIST_ADD:{"security":"Tesla","watchlist":"Tech"}]]"#;

        let (commands, _) = parse_watchlist_commands(response);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].security, "Tesla");
        assert_eq!(commands[0].watchlist, "Tech");
    }

    #[test]
    fn test_parse_watchlist_remove() {
        let response = r#"[[WATCHLIST_REMOVE:{"watchlist":"Standard","security":"Microsoft"}]]"#;

        let (commands, _) = parse_watchlist_commands(response);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, "remove");
        assert_eq!(commands[0].security, "Microsoft");
    }

    #[test]
    fn test_parse_multiple_watchlist_commands() {
        let response = r#"
[[WATCHLIST_ADD:{"watchlist":"Tech","security":"NVIDIA"}]]
[[WATCHLIST_ADD:{"watchlist":"Dividenden","security":"Coca-Cola"}]]
[[WATCHLIST_REMOVE:{"watchlist":"Standard","security":"Intel"}]]
"#;

        let (commands, _) = parse_watchlist_commands(response);

        assert_eq!(commands.len(), 3);
        assert_eq!(commands[0].action, "add");
        assert_eq!(commands[1].action, "add");
        assert_eq!(commands[2].action, "remove");
    }

    #[test]
    fn test_parse_watchlist_no_commands() {
        let response = "Das ist eine normale Antwort ohne Commands.";

        let (commands, cleaned) = parse_watchlist_commands(response);

        assert_eq!(commands.len(), 0);
        assert_eq!(cleaned, response);
    }

    #[test]
    fn test_parse_transaction_query_security_only() {
        let response = r#"[[QUERY_TRANSACTIONS:{"security":"Apple"}]]"#;

        let (queries, _) = parse_transaction_queries(response);

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].security, Some("Apple".to_string()));
        assert_eq!(queries[0].year, None);
        assert_eq!(queries[0].txn_type, None);
    }

    #[test]
    fn test_parse_transaction_query_with_year() {
        let response = r#"[[QUERY_TRANSACTIONS:{"year":2024,"type":"BUY"}]]"#;

        let (queries, _) = parse_transaction_queries(response);

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].year, Some(2024));
        assert_eq!(queries[0].txn_type, Some("BUY".to_string()));
    }

    #[test]
    fn test_parse_transaction_query_all_params() {
        let response = r#"[[QUERY_TRANSACTIONS:{"security":"Microsoft","year":2023,"type":"SELL","limit":50}]]"#;

        let (queries, _) = parse_transaction_queries(response);

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].security, Some("Microsoft".to_string()));
        assert_eq!(queries[0].year, Some(2023));
        assert_eq!(queries[0].txn_type, Some("SELL".to_string()));
        assert_eq!(queries[0].limit, Some(50));
    }

    #[test]
    fn test_parse_portfolio_value_query() {
        let response = r#"[[QUERY_PORTFOLIO_VALUE:{"date":"2025-04-04"}]]"#;

        let (queries, _) = parse_portfolio_value_queries(response);

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].date, "2025-04-04");
    }

    #[test]
    fn test_parse_empty_response() {
        let (wl_cmds, _) = parse_watchlist_commands("");
        let (txn_queries, _) = parse_transaction_queries("");
        let (pv_queries, _) = parse_portfolio_value_queries("");

        assert!(wl_cmds.is_empty());
        assert!(txn_queries.is_empty());
        assert!(pv_queries.is_empty());
    }

    #[test]
    fn test_combined_commands_in_response() {
        let response = r#"Ich habe die Transaktionen abgefragt und füge Apple zur Watchlist hinzu.

[[QUERY_TRANSACTIONS:{"security":"Tesla","year":2024}]]
[[WATCHLIST_ADD:{"watchlist":"Tech","security":"Apple"}]]
[[QUERY_PORTFOLIO_VALUE:{"date":"2024-12-31"}]]

Das war's!"#;

        let (wl_cmds, r1) = parse_watchlist_commands(response);
        let (txn_queries, r2) = parse_transaction_queries(&r1);
        let (pv_queries, final_cleaned) = parse_portfolio_value_queries(&r2);

        assert_eq!(wl_cmds.len(), 1);
        assert_eq!(txn_queries.len(), 1);
        assert_eq!(pv_queries.len(), 1);

        assert!(!final_cleaned.contains("WATCHLIST"));
        assert!(!final_cleaned.contains("QUERY_TRANSACTIONS"));
        assert!(!final_cleaned.contains("QUERY_PORTFOLIO_VALUE"));
        assert!(final_cleaned.contains("Das war's!"));
    }
}
