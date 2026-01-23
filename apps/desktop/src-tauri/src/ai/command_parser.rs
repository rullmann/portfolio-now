//! ChatBot command parsing and execution
//!
//! This module handles parsing and executing commands embedded in AI responses,
//! such as watchlist modifications and transaction queries.
//!
//! SECURITY: Commands are parsed and returned as suggestions. Execution requires
//! explicit user confirmation via separate Tauri commands. This prevents prompt
//! injection attacks where malicious data could trigger unwanted actions.

use crate::ai::normalizer::normalize_ai_response;
use crate::commands::ai_helpers;
use chrono::{NaiveDate, Local};
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

/// Database query parsed from AI response (internal - accepts any JSON value for params)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DbQueryRaw {
    pub template: String,
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

/// Database query parsed from AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbQuery {
    pub template: String,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

impl From<DbQueryRaw> for DbQuery {
    fn from(raw: DbQueryRaw) -> Self {
        let params = raw.params.into_iter()
            .map(|(k, v)| {
                let str_val = match v {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    other => other.to_string(),
                };
                (k, str_val)
            })
            .collect();
        DbQuery {
            template: raw.template,
            params,
        }
    }
}

/// Parse database query commands from AI response
///
/// Extracts `[[QUERY_DB:...]]` commands using brace-counting for robust JSON extraction.
/// This approach correctly handles nested objects and special characters in values.
///
/// NOTE: AI response formatting quirks (like `] ]` or `[[ QUERY_DB :`) are handled
/// centrally by `normalize_ai_response()` in `parse_response_with_suggestions()`.
pub fn parse_db_queries(response: &str) -> (Vec<DbQuery>, String) {
    let mut queries = Vec::new();
    let mut cleaned_response = response.to_string();
    let marker = "[[QUERY_DB:";
    let end_marker = "]]";

    // Find all occurrences of the marker and extract JSON by counting braces
    let mut search_start = 0;
    while let Some(start_idx) = cleaned_response[search_start..].find(marker) {
        let abs_start = search_start + start_idx;
        let json_start = abs_start + marker.len();

        // Find matching closing brace by counting (using char_indices for correct byte positions)
        let mut brace_count = 0;
        let mut json_end = None;

        for (byte_offset, c) in cleaned_response[json_start..].char_indices() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        // byte_offset is the start of '}', add 1 for the byte length of '}'
                        json_end = Some(json_start + byte_offset + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end_idx) = json_end {
            let json_str = &cleaned_response[json_start..end_idx];

            // Verify it ends with ]]
            let after_json = &cleaned_response[end_idx..];
            if after_json.starts_with(end_marker) {
                // Try to parse as JSON using serde (via DbQueryRaw to handle int/string params)
                match serde_json::from_str::<DbQueryRaw>(json_str) {
                    Ok(raw_query) => {
                        queries.push(raw_query.into());
                    }
                    Err(e) => {
                        log::warn!("Failed to parse QUERY_DB: {} - JSON: {}", e, json_str);
                    }
                }

                // Always remove this command from the response
                let full_end = end_idx + end_marker.len();
                cleaned_response = format!(
                    "{}{}",
                    &cleaned_response[..abs_start],
                    &cleaned_response[full_end..]
                );
                // Don't advance search_start since we removed content
            } else {
                // No closing ]], try to find and remove partial command anyway
                if let Some(fallback_end) = cleaned_response[abs_start..].find("]]") {
                    let full_end = abs_start + fallback_end + 2;
                    log::warn!("Removing malformed QUERY_DB command");
                    cleaned_response = format!(
                        "{}{}",
                        &cleaned_response[..abs_start],
                        &cleaned_response[full_end..]
                    );
                } else {
                    search_start = end_idx;
                }
            }
        } else {
            // No matching brace found, skip this marker
            search_start = json_start;
        }
    }

    // Final cleanup: Remove any remaining [[QUERY_DB:...]] patterns that might have been missed
    // This is a safety net for edge cases like malformed JSON or unexpected formatting
    // NOTE: Whitespace issues like "] ]" are handled by normalize_ai_response() upstream
    let re_fallback = regex::Regex::new(r"(?s)\[\[QUERY_DB:.*?\]\]").unwrap();
    cleaned_response = re_fallback.replace_all(&cleaned_response, "").to_string();

    cleaned_response = cleaned_response.trim().to_string();
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

use crate::ai::types::{TransactionCreateCommand, PortfolioTransferCommand, TransactionDeleteCommand};

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

/// Parse transaction delete commands from AI response
///
/// Extracts `[[TRANSACTION_DELETE:...]]` commands.
/// SECURITY: These are returned as SUGGESTIONS, never auto-executed.
pub fn parse_transaction_delete_commands(response: &str) -> (Vec<TransactionDeleteCommand>, String) {
    let mut commands = Vec::new();
    let mut cleaned_response = response.to_string();

    // Match [[TRANSACTION_DELETE:{...}]]
    let cmd_re = Regex::new(r#"\[\[TRANSACTION_DELETE:\s*(\{[^]]+\})\]\]"#).unwrap();

    for cap in cmd_re.captures_iter(response) {
        let json_str = &cap[1];

        // Try to parse as JSON
        if let Ok(cmd) = serde_json::from_str::<TransactionDeleteCommand>(json_str) {
            commands.push(cmd);
        } else {
            log::warn!("Failed to parse TRANSACTION_DELETE command: {}", json_str);
        }
    }

    // Remove command tags from response
    let clean_re = Regex::new(r#"\[\[TRANSACTION_DELETE:[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim().to_string();

    (commands, cleaned_response)
}

// ============================================================================
// Date Validation and Correction
// ============================================================================

/// Validate and potentially correct an ISO date string (YYYY-MM-DD)
///
/// Returns the corrected date string if valid, or None if the date is invalid.
/// This function attempts to fix common AI misreadings:
/// - Swapped month/day (if day > 12 and month <= 12)
/// - Non-ISO formats that can be converted
fn validate_and_correct_date(date_str: &str) -> Option<String> {
    let trimmed = date_str.trim();

    // Try parsing as ISO format first
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return validate_date_plausibility(date);
    }

    // Try DD.MM.YYYY (German format) and convert to ISO
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%d.%m.%Y") {
        return validate_date_plausibility(date);
    }

    // Try DD/MM/YYYY (EU format) and convert to ISO
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%d/%m/%Y") {
        return validate_date_plausibility(date);
    }

    // Try MM/DD/YYYY (US format) - only if it makes sense
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%m/%d/%Y") {
        // US format is less common for EU brokers, validate carefully
        return validate_date_plausibility(date);
    }

    // If we have something like "YYYY-MM-DD" with invalid values, try to fix
    let parts: Vec<&str> = trimmed.split('-').collect();
    if parts.len() == 3 {
        if let (Ok(year), Ok(month), Ok(day)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            // Check if month and day might be swapped
            if month > 12 && day <= 12 {
                // Month and day are likely swapped
                if let Some(date) = NaiveDate::from_ymd_opt(year, day, month) {
                    log::warn!("Date correction: {} looks like swapped MM/DD, correcting to {}", trimmed, date);
                    return validate_date_plausibility(date);
                }
            }
        }
    }

    log::warn!("Could not validate date: {}", trimmed);
    None
}

/// Check if a date is plausible (not too far in future, not too old)
fn validate_date_plausibility(date: NaiveDate) -> Option<String> {
    let today = Local::now().date_naive();
    let min_date = NaiveDate::from_ymd_opt(2000, 1, 1)?;
    let max_date = today + chrono::Duration::days(30); // Allow up to 30 days in future for settlements

    if date < min_date {
        log::warn!("Date {} is too old (before 2000), likely AI misread", date);
        return None;
    }

    if date > max_date {
        log::warn!("Date {} is too far in the future, likely AI misread", date);
        return None;
    }

    Some(date.format("%Y-%m-%d").to_string())
}

/// Validate extracted transaction and fix common issues
fn normalize_extracted_txn_type(raw: &str) -> String {
    let normalized = raw.trim().to_uppercase();
    let normalized = normalized.replace(' ', "_").replace('-', "_");

    match normalized.as_str() {
        // Dividends
        "DIVIDEND" | "DIVIDENDS" | "DIVIDENDE" | "DIVIDENDEN" |
        "AUSSCHÜTTUNG" | "AUSSCHUETTUNG" | "ERTRAG" | "ERTRAGSGUTSCHRIFT" |
        "DIVIDENDENGUTSCHRIFT" => "DIVIDENDS".to_string(),
        // Buys / Sells
        "BUY" | "KAUF" => "BUY".to_string(),
        "SELL" | "VERKAUF" => "SELL".to_string(),
        // Transfers / Deliveries
        "DELIVERY_INBOUND" | "EINLIEFERUNG" => "DELIVERY_INBOUND".to_string(),
        "DELIVERY_OUTBOUND" | "AUSLIEFERUNG" => "DELIVERY_OUTBOUND".to_string(),
        "TRANSFER_IN" | "UMBUCHUNG_EIN" | "UMBUCHUNG_EINGANG" => "TRANSFER_IN".to_string(),
        "TRANSFER_OUT" | "UMBUCHUNG_AUS" | "UMBUCHUNG_AUSGANG" => "TRANSFER_OUT".to_string(),
        // Cash
        "DEPOSIT" | "EINZAHLUNG" | "EINLAGE" => "DEPOSIT".to_string(),
        "REMOVAL" | "AUSZAHLUNG" | "ENTNAHME" => "REMOVAL".to_string(),
        "INTEREST" | "ZINS" | "ZINSEN" => "INTEREST".to_string(),
        "FEES" | "FEE" | "GEBUEHREN" | "GEBÜHREN" => "FEES".to_string(),
        "TAXES" | "TAX" | "STEUERN" => "TAXES".to_string(),
        _ => normalized,
    }
}

fn validate_extracted_transaction(txn: &mut ExtractedTransaction) {
    let normalized_type = normalize_extracted_txn_type(&txn.txn_type);
    if normalized_type != txn.txn_type {
        log::info!(
            "Normalized txn_type from '{}' to '{}'",
            txn.txn_type,
            normalized_type
        );
        txn.txn_type = normalized_type;
    }

    // Validate and correct the date
    if let Some(corrected) = validate_and_correct_date(&txn.date) {
        if corrected != txn.date {
            log::info!("Corrected transaction date from '{}' to '{}'", txn.date, corrected);
            txn.date = corrected;
        }
    } else {
        log::warn!("Invalid date '{}' in extracted transaction for '{}'",
            txn.date,
            txn.security_name.as_deref().unwrap_or("unknown"));
    }

    // Validate value_date if present
    if let Some(ref vd) = txn.value_date {
        if let Some(corrected) = validate_and_correct_date(vd) {
            if corrected != *vd {
                log::info!("Corrected value date from '{}' to '{}'", vd, corrected);
                txn.value_date = Some(corrected);
            }
        }
    }

    // Normalize shares (AI sometimes returns 0 or negative for buys/dividends)
    if let Some(shares) = txn.shares {
        if (txn.txn_type == "BUY" || txn.txn_type == "DELIVERY_INBOUND") && shares < 0.0 {
            txn.shares = Some(shares.abs());
        } else if shares <= 0.0 {
            txn.shares = None;
        }
    }

    // Ensure amount is positive for buys, negative values might indicate wrong sign
    if let Some(amount) = txn.amount {
        if amount < 0.0 && (txn.txn_type == "BUY" || txn.txn_type == "DELIVERY_INBOUND") {
            txn.amount = Some(amount.abs());
        }
    }
}

// ============================================================================
// Extracted Transactions from Images
// ============================================================================

/// A single transaction extracted from an image (e.g., bank statement screenshot)
///
/// Supports currency conversion details for foreign currency transactions:
/// - `gross_amount` / `gross_currency`: Original transaction amount in foreign currency
/// - `amount` / `currency`: Final amount in account/portfolio currency (after conversion)
/// - `exchange_rate`: The conversion rate used (foreign → local)
/// - `price_per_share`: Price per share in the transaction currency
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedTransaction {
    pub date: String,
    #[serde(alias = "type")]
    pub txn_type: String, // BUY, SELL, DIVIDENDS, DEPOSIT, REMOVAL, etc.
    pub security_name: Option<String>,
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub ticker: Option<String>,
    pub shares: Option<f64>, // Unscaled shares (e.g., 10.5)

    // Primary amount (in account/portfolio currency after conversion)
    pub amount: Option<f64>, // Unscaled amount (e.g., 1500.00 EUR)
    pub currency: String,    // Account/portfolio currency (e.g., "EUR")

    // Original foreign currency details (if different from account currency)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_amount: Option<f64>,   // Original amount in foreign currency (e.g., 1650.00 USD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_currency: Option<String>, // Foreign currency (e.g., "USD")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<f64>,  // Exchange rate (foreign → local, e.g., 0.91 for USD→EUR)

    // Price per share
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_per_share: Option<f64>,         // Price in transaction currency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_per_share_currency: Option<String>, // Currency of price_per_share (usually same as gross_currency or currency)

    // Fees and taxes (can also have foreign currency equivalents)
    pub fees: Option<f64>,  // Unscaled fees in account currency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fees_foreign: Option<f64>,     // Fees in foreign currency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fees_foreign_currency: Option<String>,

    pub taxes: Option<f64>, // Unscaled taxes in account currency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxes_foreign: Option<f64>,    // Taxes in foreign currency
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxes_foreign_currency: Option<String>,

    // Additional info
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_date: Option<String>,    // Valuta/settlement date if different from trade date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,      // Broker order/reference number
}

/// Container for multiple extracted transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedTransactionsPayload {
    pub transactions: Vec<ExtractedTransaction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_description: Option<String>, // e.g., "Kontoauszug Januar 2024"
}

/// Parse extracted transactions from AI response (from image analysis)
///
/// Extracts `[[EXTRACTED_TRANSACTIONS:...]]` commands.
/// SECURITY: These are returned as SUGGESTIONS, never auto-executed.
pub fn parse_extracted_transactions(response: &str) -> (Vec<ExtractedTransactionsPayload>, String) {
    let mut payloads = Vec::new();
    let mut cleaned_response = response.to_string();
    let marker = "[[EXTRACTED_TRANSACTIONS:";
    let end_marker = "]]";

    // Find all occurrences of the marker and extract JSON by counting braces
    let mut search_start = 0;
    while let Some(start_idx) = cleaned_response[search_start..].find(marker) {
        let abs_start = search_start + start_idx;
        let json_start = abs_start + marker.len();

        // Find matching closing braces by counting (using char_indices for correct byte positions)
        let mut brace_count = 0;
        let mut json_end = None;

        for (byte_offset, c) in cleaned_response[json_start..].char_indices() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        // byte_offset is the start of '}', add 1 for the byte length of '}'
                        json_end = Some(json_start + byte_offset + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end_idx) = json_end {
            let json_str = &cleaned_response[json_start..end_idx];

            // Verify it ends with ]]
            let after_json = &cleaned_response[end_idx..];
            if after_json.starts_with(end_marker) {
                // Try to parse as JSON
                match serde_json::from_str::<ExtractedTransactionsPayload>(json_str) {
                    Ok(mut payload) => {
                        if !payload.transactions.is_empty() {
                            // Validate and correct each transaction
                            for txn in &mut payload.transactions {
                                validate_extracted_transaction(txn);
                            }
                            payloads.push(payload);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to parse EXTRACTED_TRANSACTIONS: {} - JSON: {}", e, json_str);
                        // JSON parsing failed, but we still need to remove the command
                        // to avoid showing raw JSON to the user
                    }
                }

                // ALWAYS remove this command from the response (even if parsing failed)
                let full_end = end_idx + end_marker.len();
                cleaned_response = format!(
                    "{}{}",
                    &cleaned_response[..abs_start],
                    &cleaned_response[full_end..]
                );
                // Don't advance search_start since we removed content
            } else {
                // No closing ]], try to find and remove partial command anyway
                // Find the next ]] after the marker to clean up malformed commands
                if let Some(fallback_end) = cleaned_response[abs_start..].find("]]") {
                    let full_end = abs_start + fallback_end + 2;
                    log::warn!("Removing malformed EXTRACTED_TRANSACTIONS command");
                    cleaned_response = format!(
                        "{}{}",
                        &cleaned_response[..abs_start],
                        &cleaned_response[full_end..]
                    );
                } else {
                    search_start = end_idx;
                }
            }
        } else {
            // No matching brace found, try to remove partial command anyway
            if let Some(fallback_end) = cleaned_response[abs_start..].find("]]") {
                let full_end = abs_start + fallback_end + 2;
                log::warn!("Removing incomplete EXTRACTED_TRANSACTIONS command (no matching braces)");
                cleaned_response = format!(
                    "{}{}",
                    &cleaned_response[..abs_start],
                    &cleaned_response[full_end..]
                );
            } else {
                // Skip this marker entirely
                search_start = json_start;
            }
        }
    }

    cleaned_response = cleaned_response.trim().to_string();
    (payloads, cleaned_response)
}

/// Format extracted transactions for user preview
fn format_extracted_transactions_description(payload: &ExtractedTransactionsPayload) -> String {
    let count = payload.transactions.len();
    let source = payload.source_description.as_ref()
        .map(|s| format!(" aus \"{}\"", s))
        .unwrap_or_default();

    if count == 1 {
        let txn = &payload.transactions[0];
        let type_label = get_transaction_type_label_de(&txn.txn_type);
        let security = txn.security_name.as_ref()
            .map(|s| format!(" - {}", s))
            .unwrap_or_default();

        // Format amount with optional foreign currency conversion
        let amount_str = match (txn.gross_amount, txn.gross_currency.as_ref(), txn.amount) {
            (Some(gross), Some(gross_curr), Some(net)) if gross_curr != &txn.currency => {
                // Show both foreign and local amounts
                let rate_str = txn.exchange_rate
                    .map(|r| format!(" @ {:.4}", r))
                    .unwrap_or_default();
                format!(", {:.2} {} → {:.2} {}{}", gross, gross_curr, net, txn.currency, rate_str)
            }
            (_, _, Some(amt)) => format!(", {:.2} {}", amt, txn.currency),
            _ => String::new(),
        };

        let shares = txn.shares
            .map(|s| format!(", {:.4} Stk.", s))
            .unwrap_or_default();

        let price_str = txn.price_per_share
            .map(|p| {
                let curr = txn.price_per_share_currency.as_ref()
                    .or(txn.gross_currency.as_ref())
                    .unwrap_or(&txn.currency);
                format!(" @ {:.2} {}", p, curr)
            })
            .unwrap_or_default();

        format!(
            "1 Transaktion extrahiert{}: {}{} am {}{}{}{}",
            source, type_label, security, txn.date, shares, price_str, amount_str
        )
    } else {
        // Summarize multiple transactions
        let mut types: Vec<&str> = Vec::new();
        let mut total_amount = 0.0f64;
        let mut currency = "EUR".to_string();
        let mut has_fx = false;

        for txn in &payload.transactions {
            let type_label = get_transaction_type_label_de(&txn.txn_type);
            if !types.contains(&type_label) {
                types.push(type_label);
            }
            if let Some(amt) = txn.amount {
                total_amount += amt;
            }
            currency = txn.currency.clone();
            if txn.gross_currency.as_ref().is_some_and(|gc| gc != &txn.currency) {
                has_fx = true;
            }
        }

        let fx_note = if has_fx { " (mit Währungsumrechnung)" } else { "" };

        format!(
            "{} Transaktionen extrahiert{}: {} (Gesamt: {:.2} {}){}",
            count,
            source,
            types.join(", "),
            total_amount,
            currency,
            fx_note
        )
    }
}

/// Get German label for transaction type
fn get_transaction_type_label_de(txn_type: &str) -> &'static str {
    match txn_type {
        "BUY" => "Kauf",
        "SELL" => "Verkauf",
        "DELIVERY_INBOUND" => "Einlieferung",
        "DELIVERY_OUTBOUND" => "Auslieferung",
        "DIVIDENDS" => "Dividende",
        "DEPOSIT" => "Einlage",
        "REMOVAL" => "Entnahme",
        "INTEREST" => "Zinsen",
        "INTEREST_CHARGE" => "Zinsbelastung",
        "FEES" => "Gebühren",
        "FEES_REFUND" => "Gebührenerstattung",
        "TAXES" => "Steuern",
        "TAX_REFUND" => "Steuererstattung",
        "TRANSFER_IN" => "Umbuchung (Eingang)",
        "TRANSFER_OUT" => "Umbuchung (Ausgang)",
        _ => "Transaktion",
    }
}

/// Format transaction delete for user confirmation display
fn format_delete_description(cmd: &TransactionDeleteCommand) -> String {
    cmd.description.clone().unwrap_or_else(|| format!("Transaktion #{} löschen", cmd.transaction_id))
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
    // CENTRAL: Normalize once at the start, all parsers benefit
    let normalized = normalize_ai_response(&response);
    let mut current_response = normalized;
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

    // Parse transaction delete commands - DO NOT EXECUTE, return as suggestions
    let (delete_commands, cleaned) = parse_transaction_delete_commands(&current_response);
    current_response = cleaned;

    for cmd in delete_commands {
        suggestions.push(SuggestedAction {
            action_type: "transaction_delete".to_string(),
            description: format_delete_description(&cmd),
            payload: serde_json::to_string(&cmd).unwrap_or_default(),
        });
    }

    // Parse extracted transactions from images - DO NOT EXECUTE, return as suggestions
    let (extracted_payloads, cleaned) = parse_extracted_transactions(&current_response);
    current_response = cleaned;

    for payload in extracted_payloads {
        suggestions.push(SuggestedAction {
            action_type: "extracted_transactions".to_string(),
            description: format_extracted_transactions_description(&payload),
            payload: serde_json::to_string(&payload).unwrap_or_default(),
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

    #[test]
    fn test_parse_extracted_transactions_simple() {
        let response = r#"[[EXTRACTED_TRANSACTIONS:{"transactions":[{"date":"2026-01-15","txnType":"BUY","securityName":"Apple","currency":"EUR","amount":100.0}],"sourceDescription":"Test"}]]
Some text after."#;

        let (payloads, cleaned) = parse_extracted_transactions(response);

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].transactions.len(), 1);
        assert_eq!(payloads[0].transactions[0].txn_type, "BUY");
        assert_eq!(payloads[0].transactions[0].security_name, Some("Apple".to_string()));
        assert!(!cleaned.contains("EXTRACTED_TRANSACTIONS"));
        assert!(cleaned.contains("Some text after."));
    }

    #[test]
    fn test_parse_extracted_transactions_complex() {
        let response = r#"[[EXTRACTED_TRANSACTIONS:{ "transactions": [ { "date": "2025-12-24", "txnType": "DIVIDENDS", "securityName": "BlackRock Inc.", "isin": "nicht verfügbar", "ticker": "BLK", "shares": 0.0, "pricePerShare": 0.0, "pricePerShareCurrency": "USD", "grossAmount": 62.52, "grossCurrency": "USD", "exchangeRate": 1.1808, "amount": 53.14, "currency": "EUR", "fees": 0.0, "feesForeign": 0.0, "feesForeignCurrency": "", "taxes": 9.38, "valueDate": "2025-12-24", "orderId": "nicht verfügbar", "note": "Dividende von BlackRock" }, { "date": "2025-12-24", "txnType": "DIVIDENDS", "securityName": "BlackRock Inc.", "isin": "nicht verfügbar", "ticker": "BLK", "shares": 0.0, "pricePerShare": 0.0, "grossAmount": -9.38, "grossCurrency": "USD", "amount": -7.95, "currency": "EUR", "fees": 0.0, "feesForeign": 0.0, "feesForeignCurrency": "", "taxes": 0.0, "valueDate": "2025-12-24", "orderId": "nicht verfügbar", "note": "Dividendensteuer von BlackRock" } ], "sourceDescription": "Kontoauszug" }]]
Ich habe 2 Transaktionen erkannt."#;

        let (payloads, cleaned) = parse_extracted_transactions(response);

        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].transactions.len(), 2);
        assert_eq!(payloads[0].transactions[0].txn_type, "DIVIDENDS");
        assert_eq!(payloads[0].transactions[0].security_name, Some("BlackRock Inc.".to_string()));
        assert_eq!(payloads[0].transactions[0].gross_amount, Some(62.52));
        assert_eq!(payloads[0].transactions[0].exchange_rate, Some(1.1808));
        assert_eq!(payloads[0].source_description, Some("Kontoauszug".to_string()));
        assert!(!cleaned.contains("EXTRACTED_TRANSACTIONS"));
        assert!(cleaned.contains("Ich habe 2 Transaktionen erkannt."));
    }

    #[test]
    fn test_parse_db_query_simple() {
        let response = r#"[[QUERY_DB:{"template":"all_dividends","params":{"year":"2024"}}]]"#;

        let (queries, cleaned) = parse_db_queries(response);

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].template, "all_dividends");
        assert_eq!(queries[0].params.get("year"), Some(&"2024".to_string()));
        assert!(!cleaned.contains("QUERY_DB"));
    }

    #[test]
    fn test_parse_db_query_nested_params() {
        let response = r#"Ich frage die Daten ab.
[[QUERY_DB:{"template":"account_balance_analysis","params":{"account":"Referenzkonto"}}]]
Hier sind die Ergebnisse."#;

        let (queries, cleaned) = parse_db_queries(response);

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].template, "account_balance_analysis");
        assert_eq!(queries[0].params.get("account"), Some(&"Referenzkonto".to_string()));
        assert!(cleaned.contains("Ich frage die Daten ab."));
        assert!(cleaned.contains("Hier sind die Ergebnisse."));
        assert!(!cleaned.contains("QUERY_DB"));
    }

    #[test]
    fn test_parse_db_query_empty_params() {
        let response = r#"[[QUERY_DB:{"template":"sold_securities","params":{}}]]"#;

        let (queries, cleaned) = parse_db_queries(response);

        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].template, "sold_securities");
        assert!(queries[0].params.is_empty());
        assert!(cleaned.is_empty());
    }

    #[test]
    fn test_parse_db_query_multiple() {
        let response = r#"[[QUERY_DB:{"template":"all_dividends","params":{"year":"2024"}}]]
[[QUERY_DB:{"template":"security_transactions","params":{"security":"Apple"}}]]"#;

        let (queries, cleaned) = parse_db_queries(response);

        assert_eq!(queries.len(), 2);
        assert_eq!(queries[0].template, "all_dividends");
        assert_eq!(queries[1].template, "security_transactions");
        assert!(!cleaned.contains("QUERY_DB"));
    }

    #[test]
    fn test_parse_db_query_integer_param() {
        // Test with integer parameter (not string) - exact format from AI
        let response = r#"[[QUERY_DB:{"template":"securities_in_multiple_portfolios","params":{"min_portfolios":2}}]]"#;

        let (queries, cleaned) = parse_db_queries(response);

        assert_eq!(queries.len(), 1, "Should find 1 query");
        assert_eq!(queries[0].template, "securities_in_multiple_portfolios");
        assert_eq!(queries[0].params.get("min_portfolios"), Some(&"2".to_string()));
        assert!(!cleaned.contains("QUERY_DB"), "Command should be removed from response");
        assert!(cleaned.is_empty(), "Cleaned response should be empty");
    }

    #[test]
    fn test_parse_db_query_whitespace_in_brackets() {
        // Test with space before closing bracket (AI formatting issue)
        // NOTE: Whitespace issues are now handled by normalize_ai_response() centrally.
        // This test verifies that normalize + parse_db_queries works correctly.
        let response = r#"[[QUERY_DB:{"template":"securities_in_multiple_portfolios","params":{"min_portfolios":2}}] ]"#;

        // First normalize (as done in parse_response_with_suggestions)
        let normalized = normalize_ai_response(response);
        let (queries, cleaned) = parse_db_queries(&normalized);

        assert_eq!(queries.len(), 1, "Should find 1 query after normalization");
        assert_eq!(queries[0].template, "securities_in_multiple_portfolios");
        assert!(!cleaned.contains("QUERY_DB"), "Command should be removed");
        assert!(!cleaned.contains("] ]"), "Malformed brackets should be removed");
        assert!(cleaned.is_empty(), "Cleaned response should be empty");
    }

    #[test]
    fn test_parse_response_with_suggestions_normalizes_whitespace() {
        // Integration test: verify that parse_response_with_suggestions handles
        // AI formatting quirks correctly through central normalization
        let response = r#"Text before

[[ QUERY_DB :{"template":"all_dividends","params":{}}] ]

Text after"#.to_string();

        let result = parse_response_with_suggestions(response);

        // Query results are executed, so we check the cleaned response
        assert!(!result.cleaned_response.contains("QUERY_DB"), "Command should be removed");
        assert!(!result.cleaned_response.contains("] ]"), "Malformed brackets should not remain");
        assert!(result.cleaned_response.contains("Text before"), "Regular text preserved");
        assert!(result.cleaned_response.contains("Text after"), "Regular text preserved");
    }
}
