//! ChatBot command parsing and execution
//!
//! This module handles parsing and executing commands embedded in AI responses,
//! such as watchlist modifications and transaction queries.
//!
//! SECURITY: Commands are parsed and returned as suggestions. Execution requires
//! explicit user confirmation via separate Tauri commands. This prevents prompt
//! injection attacks where malicious data could trigger unwanted actions.

use crate::commands::ai_helpers;
use regex::Regex;
use serde::{Deserialize, Serialize};

// ============================================================================
// Watchlist Commands
// ============================================================================

/// Watchlist command parsed from AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
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
// Database Query Commands (using query_templates)
// ============================================================================

use crate::ai::query_templates::{execute_template, QueryRequest};
use crate::db::get_connection;
use std::collections::HashMap;

/// Database query parsed from AI response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbQuery {
    pub template: String,
    pub params: HashMap<String, String>,
}

/// Parse database query commands from AI response
///
/// Extracts `[[QUERY_DB:...]]` commands
pub fn parse_db_queries(response: &str) -> (Vec<DbQuery>, String) {
    let mut queries = Vec::new();
    let mut cleaned_response = response.to_string();

    // Match [[QUERY_DB:{"template":"...", "params":{...}}]]
    let query_re = Regex::new(r#"\[\[QUERY_DB:\s*\{([^]]+)\}\]\]"#).unwrap();

    for cap in query_re.captures_iter(response) {
        let json_content = &cap[1];

        // Extract template name
        let template = Regex::new(r#""template"\s*:\s*"([^"]+)""#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .map(|c| c[1].to_string());

        if let Some(template) = template {
            // Extract params object
            let mut params = HashMap::new();

            // Find params section and extract key-value pairs
            if let Some(params_re) = Regex::new(r#""params"\s*:\s*\{([^}]*)\}"#).ok() {
                if let Some(params_cap) = params_re.captures(json_content) {
                    let params_content = &params_cap[1];

                    // Extract individual parameters
                    if let Some(param_re) = Regex::new(r#""([^"]+)"\s*:\s*"([^"]+)""#).ok() {
                        for pcap in param_re.captures_iter(params_content) {
                            params.insert(pcap[1].to_string(), pcap[2].to_string());
                        }
                    }
                }
            }

            queries.push(DbQuery { template, params });
        }
    }

    let clean_re = Regex::new(r#"\[\[QUERY_DB:[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim_start().to_string();

    (queries, cleaned_response)
}

/// Execute database queries and return formatted results
pub fn execute_db_queries(queries: &[DbQuery]) -> Vec<String> {
    let mut results = Vec::new();

    let guard = match get_connection() {
        Ok(g) => g,
        Err(e) => {
            results.push(format!("Fehler beim Zugriff auf Datenbank: {}", e));
            return results;
        }
    };

    let conn = match guard.as_ref() {
        Some(c) => c,
        None => {
            results.push("Datenbank nicht initialisiert.".to_string());
            return results;
        }
    };

    for query in queries {
        let request = QueryRequest {
            template_id: query.template.clone(),
            parameters: query.params.clone(),
        };

        match execute_template(conn, &request) {
            Ok(result) => {
                if result.row_count == 0 {
                    results.push(format!("**{}**: Keine Ergebnisse gefunden.", query.template));
                } else {
                    // Some templates (like account_balance_analysis) return a complete answer
                    // - no need for a header
                    if query.template == "account_balance_analysis" {
                        results.push(result.formatted_markdown);
                    } else {
                        results.push(format!(
                            "**{} ({} Ergebnisse)**:\n\n{}",
                            query.template, result.row_count, result.formatted_markdown
                        ));
                    }
                }
            }
            Err(e) => {
                results.push(format!("Fehler bei Abfrage '{}': {}", query.template, e));
            }
        }
    }

    results
}

// ============================================================================
// Transaction Create Commands
// ============================================================================

use crate::ai::types::{TransactionCreateCommand, PortfolioTransferCommand};

/// Parse transaction create commands from AI response
///
/// Extracts `[[TRANSACTION_CREATE:...]]` commands.
/// SECURITY: These are returned as SUGGESTIONS, never auto-executed.
pub fn parse_transaction_create_commands(response: &str) -> (Vec<TransactionCreateCommand>, String) {
    let mut commands = Vec::new();
    let mut cleaned_response = response.to_string();

    // Match [[TRANSACTION_CREATE:{...}]]
    let cmd_re = Regex::new(r#"\[\[TRANSACTION_CREATE:\s*(\{[^]]+\})\]\]"#).unwrap();

    for cap in cmd_re.captures_iter(response) {
        let json_str = &cap[1];

        // Try to parse as JSON
        if let Ok(cmd) = serde_json::from_str::<TransactionCreateCommand>(json_str) {
            commands.push(cmd);
        } else {
            log::warn!("Failed to parse TRANSACTION_CREATE command: {}", json_str);
        }
    }

    // Remove command tags from response
    let clean_re = Regex::new(r#"\[\[TRANSACTION_CREATE:[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim().to_string();

    (commands, cleaned_response)
}

/// Parse portfolio transfer commands from AI response
///
/// Extracts `[[PORTFOLIO_TRANSFER:...]]` commands.
/// SECURITY: These are returned as SUGGESTIONS, never auto-executed.
pub fn parse_portfolio_transfer_commands(response: &str) -> (Vec<PortfolioTransferCommand>, String) {
    let mut commands = Vec::new();
    let mut cleaned_response = response.to_string();

    // Match [[PORTFOLIO_TRANSFER:{...}]]
    let cmd_re = Regex::new(r#"\[\[PORTFOLIO_TRANSFER:\s*(\{[^]]+\})\]\]"#).unwrap();

    for cap in cmd_re.captures_iter(response) {
        let json_str = &cap[1];

        // Try to parse as JSON
        if let Ok(cmd) = serde_json::from_str::<PortfolioTransferCommand>(json_str) {
            commands.push(cmd);
        } else {
            log::warn!("Failed to parse PORTFOLIO_TRANSFER command: {}", json_str);
        }
    }

    // Remove command tags from response
    let clean_re = Regex::new(r#"\[\[PORTFOLIO_TRANSFER:[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim().to_string();

    (commands, cleaned_response)
}

/// Format transaction for user confirmation display
fn format_transaction_description(cmd: &TransactionCreateCommand) -> String {
    let type_label = match cmd.txn_type.as_str() {
        "BUY" => "Kauf",
        "SELL" => "Verkauf",
        "DELIVERY_INBOUND" => "Einlieferung",
        "DELIVERY_OUTBOUND" => "Auslieferung",
        "DIVIDENDS" => "Dividende",
        "DEPOSIT" => "Einlage",
        "REMOVAL" => "Entnahme",
        "INTEREST" => "Zinsen",
        "FEES" => "Gebühren",
        "TAXES" => "Steuern",
        "TRANSFER_IN" => "Umbuchung (Eingang)",
        "TRANSFER_OUT" => "Umbuchung (Ausgang)",
        _ => &cmd.txn_type,
    };

    let security_str = cmd.security_name.as_ref()
        .map(|n| format!(" - {}", n))
        .unwrap_or_default();

    let shares_str = cmd.shares
        .map(|s| format!(", {:.2} Stk.", s as f64 / 100_000_000.0))
        .unwrap_or_default();

    let amount_str = cmd.amount
        .map(|a| format!(", {:.2} {}", a as f64 / 100.0, cmd.currency))
        .unwrap_or_default();

    let fees_str = cmd.fees.filter(|&f| f > 0)
        .map(|f| format!(", Gebühren: {:.2} {}", f as f64 / 100.0, cmd.currency))
        .unwrap_or_default();

    format!(
        "{}{}: {}{}{}{}",
        type_label, security_str, cmd.date, shares_str, amount_str, fees_str
    )
}

/// Format portfolio transfer for user confirmation display
fn format_transfer_description(cmd: &PortfolioTransferCommand) -> String {
    let shares = cmd.shares as f64 / 100_000_000.0;
    format!(
        "Depotwechsel: {:.2} Stk. am {} (Depot {} → Depot {})",
        shares, cmd.date, cmd.from_portfolio_id, cmd.to_portfolio_id
    )
}

// ============================================================================
// Combined Response Processing
// ============================================================================

/// Suggested action from AI response that requires user confirmation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedAction {
    /// Type of action: "watchlist_add", "watchlist_remove", "transaction_create", "portfolio_transfer"
    pub action_type: String,
    /// Human-readable description of the action
    pub description: String,
    /// JSON payload for the action
    pub payload: String,
}

/// Result of parsing AI response for commands
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedResponseWithSuggestions {
    /// Cleaned response text without command tags
    pub cleaned_response: String,
    /// Suggested actions that require user confirmation
    pub suggestions: Vec<SuggestedAction>,
    /// Results from read-only queries (transactions, portfolio value)
    pub query_results: Vec<String>,
}

/// Parse AI response and extract suggestions without executing anything dangerous
///
/// SECURITY: This is the secure parsing function that:
/// - Parses all command types from AI response
/// - Returns watchlist commands as SUGGESTIONS (not executed)
/// - Returns transaction commands as SUGGESTIONS (not executed)
/// - Executes ONLY read-only queries (transaction queries, portfolio value queries)
/// - Returns structured result for frontend to handle
pub fn parse_response_with_suggestions(response: String) -> ParsedResponseWithSuggestions {
    let mut current_response = response;
    let mut suggestions: Vec<SuggestedAction> = Vec::new();
    let mut query_results: Vec<String> = Vec::new();

    // Parse watchlist commands - DO NOT EXECUTE, return as suggestions
    let (wl_commands, cleaned) = parse_watchlist_commands(&current_response);
    current_response = cleaned;

    for cmd in wl_commands {
        let (action_type, description) = match cmd.action.as_str() {
            "add" => (
                "watchlist_add".to_string(),
                format!("\"{}\" zur Watchlist \"{}\" hinzufügen", cmd.security, cmd.watchlist),
            ),
            "remove" => (
                "watchlist_remove".to_string(),
                format!("\"{}\" von Watchlist \"{}\" entfernen", cmd.security, cmd.watchlist),
            ),
            _ => continue,
        };

        suggestions.push(SuggestedAction {
            action_type,
            description,
            payload: serde_json::to_string(&cmd).unwrap_or_default(),
        });
    }

    // Parse transaction create commands - DO NOT EXECUTE, return as suggestions
    let (txn_create_commands, cleaned) = parse_transaction_create_commands(&current_response);
    current_response = cleaned;

    for cmd in txn_create_commands {
        suggestions.push(SuggestedAction {
            action_type: "transaction_create".to_string(),
            description: format_transaction_description(&cmd),
            payload: serde_json::to_string(&cmd).unwrap_or_default(),
        });
    }

    // Parse portfolio transfer commands - DO NOT EXECUTE, return as suggestions
    let (transfer_commands, cleaned) = parse_portfolio_transfer_commands(&current_response);
    current_response = cleaned;

    for cmd in transfer_commands {
        suggestions.push(SuggestedAction {
            action_type: "portfolio_transfer".to_string(),
            description: format_transfer_description(&cmd),
            payload: serde_json::to_string(&cmd).unwrap_or_default(),
        });
    }

    // Parse and execute transaction queries (READ-ONLY, safe to execute)
    let (txn_queries, cleaned) = parse_transaction_queries(&current_response);
    current_response = cleaned;

    if !txn_queries.is_empty() {
        let results = execute_transaction_queries(&txn_queries);
        query_results.extend(results);
    }

    // Parse and execute portfolio value queries (READ-ONLY, safe to execute)
    let (pv_queries, cleaned) = parse_portfolio_value_queries(&current_response);
    current_response = cleaned;

    if !pv_queries.is_empty() {
        let results = execute_portfolio_value_queries(&pv_queries);
        query_results.extend(results);
    }

    // Parse and execute database queries (READ-ONLY, safe to execute)
    let (db_queries, cleaned) = parse_db_queries(&current_response);
    current_response = cleaned;

    if !db_queries.is_empty() {
        let results = execute_db_queries(&db_queries);
        query_results.extend(results);
    }

    ParsedResponseWithSuggestions {
        cleaned_response: current_response,
        suggestions,
        query_results,
    }
}

/// Execute a confirmed watchlist action
///
/// SECURITY: This should only be called after explicit user confirmation
pub async fn execute_confirmed_watchlist_action(
    action_type: &str,
    payload: &str,
    alpha_vantage_api_key: Option<String>,
) -> Result<String, String> {
    let cmd: WatchlistCommand = serde_json::from_str(payload)
        .map_err(|e| format!("Invalid payload: {}", e))?;

    match action_type {
        "watchlist_add" => {
            ai_helpers::ai_add_to_watchlist(
                cmd.watchlist,
                cmd.security,
                alpha_vantage_api_key,
            )
            .await
            .map(|r| r.message)
            .map_err(|e| e.to_string())
        }
        "watchlist_remove" => {
            ai_helpers::ai_remove_from_watchlist(cmd.watchlist, cmd.security)
                .map(|r| r.message)
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("Unknown action type: {}", action_type)),
    }
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

    #[test]
    fn test_parse_transaction_create_buy() {
        let response = r#"Ich erstelle die Transaktion für dich.

[[TRANSACTION_CREATE:{"preview":true,"type":"BUY","portfolioId":1,"securityId":42,"securityName":"Apple Inc.","shares":1000000000,"amount":180000,"currency":"EUR","date":"2026-01-15","fees":100}]]

Bitte bestätige die Transaktion."#;

        let (commands, cleaned) = parse_transaction_create_commands(response);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].txn_type, "BUY");
        assert_eq!(commands[0].portfolio_id, Some(1));
        assert_eq!(commands[0].security_id, Some(42));
        assert_eq!(commands[0].security_name, Some("Apple Inc.".to_string()));
        assert_eq!(commands[0].shares, Some(1000000000));
        assert_eq!(commands[0].amount, Some(180000));
        assert_eq!(commands[0].currency, "EUR");
        assert_eq!(commands[0].date, "2026-01-15");
        assert_eq!(commands[0].fees, Some(100));
        assert!(!cleaned.contains("TRANSACTION_CREATE"));
        assert!(cleaned.contains("Bitte bestätige"));
    }

    #[test]
    fn test_parse_transaction_create_dividend() {
        let response = r#"[[TRANSACTION_CREATE:{"type":"DIVIDENDS","accountId":1,"securityId":42,"securityName":"Microsoft","amount":5000,"currency":"EUR","date":"2026-01-20"}]]"#;

        let (commands, _) = parse_transaction_create_commands(response);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].txn_type, "DIVIDENDS");
        assert_eq!(commands[0].account_id, Some(1));
        assert_eq!(commands[0].amount, Some(5000));
    }

    #[test]
    fn test_parse_transaction_create_deposit() {
        let response = r#"[[TRANSACTION_CREATE:{"type":"DEPOSIT","accountId":1,"amount":100000,"currency":"EUR","date":"2026-01-15"}]]"#;

        let (commands, _) = parse_transaction_create_commands(response);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].txn_type, "DEPOSIT");
        assert_eq!(commands[0].account_id, Some(1));
        assert_eq!(commands[0].amount, Some(100000));
        assert_eq!(commands[0].security_id, None);
    }

    #[test]
    fn test_parse_portfolio_transfer() {
        let response = r#"Ich übertrage die Aktien für dich.

[[PORTFOLIO_TRANSFER:{"securityId":42,"shares":1000000000,"date":"2026-01-15","fromPortfolioId":1,"toPortfolioId":2,"note":"Depotwechsel zu Broker B"}]]

Bitte bestätige."#;

        let (commands, cleaned) = parse_portfolio_transfer_commands(response);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].security_id, 42);
        assert_eq!(commands[0].shares, 1000000000);
        assert_eq!(commands[0].date, "2026-01-15");
        assert_eq!(commands[0].from_portfolio_id, 1);
        assert_eq!(commands[0].to_portfolio_id, 2);
        assert_eq!(commands[0].note, Some("Depotwechsel zu Broker B".to_string()));
        assert!(!cleaned.contains("PORTFOLIO_TRANSFER"));
    }

    #[test]
    fn test_format_transaction_description() {
        let cmd = TransactionCreateCommand {
            preview: true,
            txn_type: "BUY".to_string(),
            portfolio_id: Some(1),
            account_id: None,
            security_id: Some(42),
            security_name: Some("Apple Inc.".to_string()),
            shares: Some(1000000000), // 10 shares
            amount: Some(180000),      // 1800 EUR
            currency: "EUR".to_string(),
            date: "2026-01-15".to_string(),
            fees: Some(100),           // 1 EUR
            taxes: None,
            note: None,
            other_portfolio_id: None,
            other_account_id: None,
        };

        let desc = format_transaction_description(&cmd);
        assert!(desc.contains("Kauf"));
        assert!(desc.contains("Apple"));
        assert!(desc.contains("10.00 Stk."));
        assert!(desc.contains("1800.00 EUR"));
        assert!(desc.contains("Gebühren: 1.00 EUR"));
    }

    #[test]
    fn test_parse_transaction_no_commands() {
        let response = "Das ist eine normale Antwort ohne Transaction-Commands.";

        let (txn_cmds, cleaned) = parse_transaction_create_commands(response);
        let (transfer_cmds, _) = parse_portfolio_transfer_commands(&cleaned);

        assert!(txn_cmds.is_empty());
        assert!(transfer_cmds.is_empty());
    }
}
