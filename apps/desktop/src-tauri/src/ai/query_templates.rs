//! Query Templates for Portfolio Chatbot
//!
//! Provides safe, predefined SQL query templates that the AI can use
//! to answer questions about transactions, dividends, and holdings.

use chrono::Datelike;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTemplate {
    pub id: String,
    pub description: String,
    pub parameters: Vec<QueryParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryParameter {
    pub name: String,
    pub param_type: String, // "string", "date", "txn_type", "year"
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub template_id: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub template_id: String,
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub row_count: usize,
    pub formatted_markdown: String,
}

// ============================================================================
// Template Definitions
// ============================================================================

/// Get all available query templates
pub fn get_all_templates() -> Vec<QueryTemplate> {
    vec![
        QueryTemplate {
            id: "security_transactions".to_string(),
            description: "Alle Transaktionen (Käufe, Verkäufe, Transfers) für ein Wertpapier".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "security".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Name, ISIN oder Ticker des Wertpapiers".to_string(),
                },
                QueryParameter {
                    name: "txn_type".to_string(),
                    param_type: "txn_type".to_string(),
                    required: false,
                    description: "Transaktionstyp: BUY, SELL, DELIVERY_INBOUND, DELIVERY_OUTBOUND".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "dividends_by_security".to_string(),
            description: "Alle Dividendenzahlungen für ein Wertpapier".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "security".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Name, ISIN oder Ticker des Wertpapiers".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "all_dividends".to_string(),
            description: "Alle Dividenden gruppiert nach Wertpapier".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "year".to_string(),
                    param_type: "year".to_string(),
                    required: false,
                    description: "Jahr filtern (z.B. 2024)".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "transactions_by_date".to_string(),
            description: "Alle Transaktionen in einem Zeitraum".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "from_date".to_string(),
                    param_type: "date".to_string(),
                    required: true,
                    description: "Startdatum (YYYY-MM-DD)".to_string(),
                },
                QueryParameter {
                    name: "to_date".to_string(),
                    param_type: "date".to_string(),
                    required: true,
                    description: "Enddatum (YYYY-MM-DD)".to_string(),
                },
                QueryParameter {
                    name: "txn_type".to_string(),
                    param_type: "txn_type".to_string(),
                    required: false,
                    description: "Transaktionstyp filtern".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "security_cost_basis".to_string(),
            description: "Einstandskurse und FIFO-Lots für ein Wertpapier".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "security".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Name, ISIN oder Ticker des Wertpapiers".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "sold_securities".to_string(),
            description: "Alle verkauften/geschlossenen Positionen".to_string(),
            parameters: vec![],
        },
        QueryTemplate {
            id: "holding_period_analysis".to_string(),
            description: "Haltefrist-Analyse für Krypto/Gold (§ 23 EStG - steuerfrei nach 1 Jahr)".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "asset_type".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Filter: 'crypto', 'gold', oder leer für alle".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "fifo_lot_details".to_string(),
            description: "Detaillierte FIFO-Lots mit Haltetagen und Tax-Status".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "security".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Name, ISIN oder Ticker (optional, ohne = alle)".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "account_transactions".to_string(),
            description: "Kontobewegungen (Einzahlungen, Auszahlungen, Zinsen, Gebühren)".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "account".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Kontoname (optional)".to_string(),
                },
                QueryParameter {
                    name: "year".to_string(),
                    param_type: "year".to_string(),
                    required: false,
                    description: "Jahr filtern (z.B. 2024)".to_string(),
                },
                QueryParameter {
                    name: "amount".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Betrag suchen (z.B. 0.25 für 25 Cent)".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "investment_plans".to_string(),
            description: "Alle Sparpläne mit Details".to_string(),
            parameters: vec![],
        },
        QueryTemplate {
            id: "portfolio_accounts".to_string(),
            description: "Alle Konten mit aktuellen Salden".to_string(),
            parameters: vec![],
        },
        QueryTemplate {
            id: "tax_relevant_sales".to_string(),
            description: "Verkäufe mit Haltefrist und Steuerstatus".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "year".to_string(),
                    param_type: "year".to_string(),
                    required: false,
                    description: "Jahr filtern (z.B. 2024)".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "account_balance_analysis".to_string(),
            description: "Analysiert Kontostand: Woher kommt das aktuelle Guthaben? Running Balance Berechnung.".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "account".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                    description: "Kontoname (z.B. 'Referenz')".to_string(),
                },
            ],
        },
        // ============================================================================
        // NEW: Performance & Allocation Templates (Phase 1)
        // ============================================================================
        QueryTemplate {
            id: "portfolio_performance_summary".to_string(),
            description: "Rendite-Übersicht: TTWROR, IRR, Gewinn/Verlust für einen Zeitraum".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "period".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Zeitraum: 'ytd' (Jahr bis heute), '1y' (1 Jahr), '3y', '5y', 'all' (seit Beginn)".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "current_holdings".to_string(),
            description: "Aktuelle Bestände mit Stückzahlen, Wert, Gewinn/Verlust".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "security".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Name, ISIN oder Ticker (optional, ohne = alle)".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "unrealized_gains_losses".to_string(),
            description: "Nicht realisierte Gewinne/Verluste aller offenen Positionen".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "filter".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Filter: 'gains' (nur Gewinne), 'losses' (nur Verluste), oder leer für alle".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "realized_gains_by_year".to_string(),
            description: "Realisierte Gewinne/Verluste pro Jahr aus Verkäufen".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "year".to_string(),
                    param_type: "year".to_string(),
                    required: false,
                    description: "Jahr filtern (z.B. 2024), oder leer für alle Jahre".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "portfolio_allocation".to_string(),
            description: "Gewichtung/Allokation des Portfolios nach Währung oder Sektor".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "by".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                    description: "Gruppierung: 'currency' (Währung), 'type' (Assetklasse), oder leer für beides".to_string(),
                },
            ],
        },
        QueryTemplate {
            id: "securities_in_multiple_portfolios".to_string(),
            description: "Wertpapiere die in mehreren Depots gehalten werden".to_string(),
            parameters: vec![
                QueryParameter {
                    name: "min_portfolios".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                    description: "Mindestanzahl Depots (Standard: 2)".to_string(),
                },
            ],
        },
    ]
}

// ============================================================================
// Query Execution
// ============================================================================

/// Execute a query template with the given parameters
pub fn execute_template(
    conn: &Connection,
    request: &QueryRequest,
) -> Result<QueryResult, String> {
    match request.template_id.as_str() {
        "security_transactions" => execute_security_transactions(conn, &request.parameters),
        "dividends_by_security" => execute_dividends_by_security(conn, &request.parameters),
        "all_dividends" => execute_all_dividends(conn, &request.parameters),
        "transactions_by_date" => execute_transactions_by_date(conn, &request.parameters),
        "security_cost_basis" => execute_security_cost_basis(conn, &request.parameters),
        "sold_securities" => execute_sold_securities(conn, &request.parameters),
        "holding_period_analysis" => execute_holding_period_analysis(conn, &request.parameters),
        "fifo_lot_details" => execute_fifo_lot_details(conn, &request.parameters),
        "account_transactions" => execute_account_transactions(conn, &request.parameters),
        "investment_plans" => execute_investment_plans(conn, &request.parameters),
        "portfolio_accounts" => execute_portfolio_accounts(conn, &request.parameters),
        "tax_relevant_sales" => execute_tax_relevant_sales(conn, &request.parameters),
        "account_balance_analysis" => execute_account_balance_analysis(conn, &request.parameters),
        // NEW: Performance & Allocation Templates
        "portfolio_performance_summary" => execute_portfolio_performance_summary(conn, &request.parameters),
        "current_holdings" => execute_current_holdings(conn, &request.parameters),
        "unrealized_gains_losses" => execute_unrealized_gains_losses(conn, &request.parameters),
        "realized_gains_by_year" => execute_realized_gains_by_year(conn, &request.parameters),
        "portfolio_allocation" => execute_portfolio_allocation(conn, &request.parameters),
        "securities_in_multiple_portfolios" => execute_securities_in_multiple_portfolios(conn, &request.parameters),
        // Check for user-defined templates
        _ => {
            // Try to find a user template with this ID
            if super::user_templates::is_user_template(&request.template_id) {
                match super::user_templates::get_user_template_by_id(conn, &request.template_id) {
                    Ok(user_template) => {
                        if !user_template.enabled {
                            return Err(format!("Template '{}' ist deaktiviert", request.template_id));
                        }
                        super::user_templates::execute_user_template(conn, &user_template, &request.parameters)
                    }
                    Err(_) => Err(format!("Unbekanntes Template: {}", request.template_id)),
                }
            } else {
                Err(format!("Unbekanntes Template: {}", request.template_id))
            }
        }
    }
}

/// Security transactions (buys, sells, transfers)
fn execute_security_transactions(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let security = params.get("security")
        .ok_or("Parameter 'security' ist erforderlich")?;
    let txn_type = params.get("txn_type");

    let search_pattern = format!("%{}%", security);

    // Build SQL with optional txn_type filter
    let result = if let Some(tt) = txn_type {
        let sql = r#"
            SELECT
                t.date,
                t.txn_type,
                s.name as security_name,
                t.shares / 100000000.0 as shares,
                t.amount / 100.0 as amount,
                t.currency,
                t.note
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE (s.name LIKE ?1 OR s.isin LIKE ?1 OR s.ticker LIKE ?1)
                AND t.txn_type = ?2
                AND t.owner_type = 'portfolio'
            ORDER BY t.date DESC
            LIMIT 50
        "#;
        execute_query(conn, sql, &[&search_pattern, tt], "security_transactions")
    } else {
        let sql = r#"
            SELECT
                t.date,
                t.txn_type,
                s.name as security_name,
                t.shares / 100000000.0 as shares,
                t.amount / 100.0 as amount,
                t.currency,
                t.note
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE (s.name LIKE ?1 OR s.isin LIKE ?1 OR s.ticker LIKE ?1)
                AND t.owner_type = 'portfolio'
                AND t.txn_type IN ('BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND', 'TRANSFER_IN', 'TRANSFER_OUT')
            ORDER BY t.date DESC
            LIMIT 50
        "#;
        execute_query(conn, sql, &[&search_pattern], "security_transactions")
    };

    // If no results, provide helpful error with suggestions
    match result {
        Ok(qr) if qr.row_count == 0 => {
            let mut enhanced = qr;
            enhanced.formatted_markdown = security_not_found_error(conn, security);
            Ok(enhanced)
        }
        other => other,
    }
}

/// Dividends for a specific security
fn execute_dividends_by_security(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let security = params.get("security")
        .ok_or("Parameter 'security' ist erforderlich")?;

    let search_pattern = format!("%{}%", security);

    let sql = r#"
        SELECT
            t.date,
            s.name as security_name,
            t.amount / 100.0 as gross_amount,
            t.currency,
            COALESCE(
                (SELECT SUM(u.amount) / 100.0 FROM pp_txn_unit u WHERE u.txn_id = t.id AND u.unit_type = 'TAX'),
                0
            ) as taxes,
            t.note
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        WHERE (s.name LIKE ?1 OR s.isin LIKE ?1 OR s.ticker LIKE ?1)
            AND t.txn_type = 'DIVIDENDS'
        ORDER BY t.date DESC
        LIMIT 50
    "#;

    execute_query(conn, sql, &[&search_pattern], "dividends_by_security")
}

/// All dividends grouped by security
fn execute_all_dividends(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let year = params.get("year");

    let sql = if year.is_some() {
        r#"
            SELECT
                s.name as security_name,
                COUNT(*) as dividend_count,
                SUM(t.amount) / 100.0 as total_gross,
                MIN(t.date) as first_dividend,
                MAX(t.date) as last_dividend
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
                AND strftime('%Y', t.date) = ?1
            GROUP BY s.id
            ORDER BY total_gross DESC
            LIMIT 30
        "#
    } else {
        r#"
            SELECT
                s.name as security_name,
                COUNT(*) as dividend_count,
                SUM(t.amount) / 100.0 as total_gross,
                MIN(t.date) as first_dividend,
                MAX(t.date) as last_dividend
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
            GROUP BY s.id
            ORDER BY total_gross DESC
            LIMIT 30
        "#
    };

    if let Some(y) = year {
        execute_query(conn, sql, &[&y.as_str()], "all_dividends")
    } else {
        execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "all_dividends")
    }
}

/// Transactions in a date range
fn execute_transactions_by_date(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let from_date = params.get("from_date")
        .ok_or("Parameter 'from_date' ist erforderlich")?;
    let to_date = params.get("to_date")
        .ok_or("Parameter 'to_date' ist erforderlich")?;
    let txn_type = params.get("txn_type");

    // Build SQL with optional txn_type filter
    if let Some(tt) = txn_type {
        let sql = r#"
            SELECT
                t.date,
                t.txn_type,
                s.name as security_name,
                t.shares / 100000000.0 as shares,
                t.amount / 100.0 as amount,
                t.currency
            FROM pp_txn t
            LEFT JOIN pp_security s ON s.id = t.security_id
            WHERE t.date >= ?1
                AND t.date <= ?2
                AND t.txn_type = ?3
            ORDER BY t.date DESC
            LIMIT 100
        "#;
        execute_query(conn, sql, &[from_date, to_date, tt], "transactions_by_date")
    } else {
        let sql = r#"
            SELECT
                t.date,
                t.txn_type,
                s.name as security_name,
                t.shares / 100000000.0 as shares,
                t.amount / 100.0 as amount,
                t.currency
            FROM pp_txn t
            LEFT JOIN pp_security s ON s.id = t.security_id
            WHERE t.date >= ?1
                AND t.date <= ?2
            ORDER BY t.date DESC
            LIMIT 100
        "#;
        execute_query(conn, sql, &[from_date, to_date], "transactions_by_date")
    }
}

/// Cost basis for a security (FIFO lots)
fn execute_security_cost_basis(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let security = params.get("security")
        .ok_or("Parameter 'security' ist erforderlich")?;

    let search_pattern = format!("%{}%", security);

    let sql = r#"
        SELECT
            s.name as security_name,
            l.purchase_date,
            l.original_shares / 100000000.0 as original_shares,
            l.remaining_shares / 100000000.0 as remaining_shares,
            l.gross_amount / 100.0 as cost_basis,
            CASE
                WHEN l.original_shares > 0 THEN (l.gross_amount * 100000000.0 / l.original_shares) / 100.0
                ELSE 0
            END as cost_per_share,
            l.currency
        FROM pp_fifo_lot l
        JOIN pp_security s ON s.id = l.security_id
        WHERE (s.name LIKE ?1 OR s.isin LIKE ?1 OR s.ticker LIKE ?1)
            AND l.remaining_shares > 0
        ORDER BY l.purchase_date DESC
    "#;

    execute_query(conn, sql, &[&search_pattern], "security_cost_basis")
}

/// Sold/closed positions
fn execute_sold_securities(
    conn: &Connection,
    _params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let sql = r#"
        SELECT
            s.name as security_name,
            s.isin,
            MAX(t.date) as last_sale_date,
            SUM(CASE WHEN t.txn_type IN ('SELL', 'DELIVERY_OUTBOUND') THEN t.shares ELSE 0 END) / 100000000.0 as total_sold,
            SUM(CASE WHEN t.txn_type IN ('SELL', 'DELIVERY_OUTBOUND') THEN t.amount ELSE 0 END) / 100.0 as total_amount
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        WHERE t.txn_type IN ('SELL', 'DELIVERY_OUTBOUND')
            AND t.owner_type = 'portfolio'
        GROUP BY s.id
        HAVING total_sold > 0
        ORDER BY last_sale_date DESC
        LIMIT 30
    "#;

    execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "sold_securities")
}

/// Holding period analysis for crypto/gold (tax-free after 1 year in Germany)
fn execute_holding_period_analysis(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let asset_type = params.get("asset_type").map(|s| s.to_lowercase());

    // Build filter based on asset type
    let type_filter = match asset_type.as_deref() {
        Some("crypto") | Some("krypto") => {
            // Common crypto identifiers
            "AND (LOWER(s.name) LIKE '%bitcoin%' OR LOWER(s.name) LIKE '%ethereum%'
                  OR LOWER(s.name) LIKE '%crypto%' OR LOWER(s.name) LIKE '%krypto%'
                  OR LOWER(s.ticker) LIKE '%btc%' OR LOWER(s.ticker) LIKE '%eth%'
                  OR LOWER(s.ticker) LIKE '%sol%' OR LOWER(s.ticker) LIKE '%xrp%'
                  OR s.feed = 'COINGECKO' OR s.feed = 'KRAKEN')"
        }
        Some("gold") => {
            "AND (LOWER(s.name) LIKE '%gold%' OR LOWER(s.ticker) LIKE '%xau%'
                  OR LOWER(s.ticker) = 'gc=f' OR LOWER(s.name) LIKE '%edelmetall%')"
        }
        _ => "", // All assets
    };

    let sql = format!(r#"
        SELECT
            s.name as security_name,
            s.ticker,
            l.purchase_date,
            julianday('now') - julianday(l.purchase_date) as holding_days,
            l.remaining_shares / 100000000.0 as shares,
            l.gross_amount / 100.0 as cost_basis,
            l.currency,
            CASE
                WHEN julianday('now') - julianday(l.purchase_date) >= 365 THEN 'STEUERFREI'
                ELSE 'STEUERPFLICHTIG'
            END as tax_status,
            CASE
                WHEN julianday('now') - julianday(l.purchase_date) < 365
                THEN date(l.purchase_date, '+1 year')
                ELSE NULL
            END as tax_free_date
        FROM pp_fifo_lot l
        JOIN pp_security s ON s.id = l.security_id
        WHERE l.remaining_shares > 0
            {}
        ORDER BY holding_days DESC
    "#, type_filter);

    execute_query(conn, &sql, &[] as &[&dyn rusqlite::ToSql], "holding_period_analysis")
}

/// Detailed FIFO lots with holding days and tax status
fn execute_fifo_lot_details(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let security = params.get("security");

    if let Some(sec) = security {
        let search_pattern = format!("%{}%", sec);
        let sql = r#"
            SELECT
                s.name as security_name,
                s.ticker,
                s.isin,
                l.purchase_date,
                julianday('now') - julianday(l.purchase_date) as holding_days,
                l.original_shares / 100000000.0 as original_shares,
                l.remaining_shares / 100000000.0 as remaining_shares,
                l.gross_amount / 100.0 as cost_basis,
                CASE
                    WHEN l.original_shares > 0 THEN (l.gross_amount * 100000000.0 / l.original_shares) / 100.0
                    ELSE 0
                END as cost_per_share,
                l.currency,
                CASE
                    WHEN julianday('now') - julianday(l.purchase_date) >= 365 THEN 'STEUERFREI'
                    ELSE 'STEUERPFLICHTIG'
                END as tax_status
            FROM pp_fifo_lot l
            JOIN pp_security s ON s.id = l.security_id
            WHERE l.remaining_shares > 0
                AND (s.name LIKE ?1 OR s.isin LIKE ?1 OR s.ticker LIKE ?1)
            ORDER BY l.purchase_date ASC
        "#;
        execute_query(conn, sql, &[&search_pattern], "fifo_lot_details")
    } else {
        let sql = r#"
            SELECT
                s.name as security_name,
                s.ticker,
                s.isin,
                l.purchase_date,
                julianday('now') - julianday(l.purchase_date) as holding_days,
                l.original_shares / 100000000.0 as original_shares,
                l.remaining_shares / 100000000.0 as remaining_shares,
                l.gross_amount / 100.0 as cost_basis,
                CASE
                    WHEN l.original_shares > 0 THEN (l.gross_amount * 100000000.0 / l.original_shares) / 100.0
                    ELSE 0
                END as cost_per_share,
                l.currency,
                CASE
                    WHEN julianday('now') - julianday(l.purchase_date) >= 365 THEN 'STEUERFREI'
                    ELSE 'STEUERPFLICHTIG'
                END as tax_status
            FROM pp_fifo_lot l
            JOIN pp_security s ON s.id = l.security_id
            WHERE l.remaining_shares > 0
            ORDER BY s.name, l.purchase_date ASC
            LIMIT 100
        "#;
        execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "fifo_lot_details")
    }
}

/// Account transactions (deposits, withdrawals, interest, fees)
fn execute_account_transactions(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let account = params.get("account");
    let year = params.get("year");
    let amount = params.get("amount");

    let mut conditions = vec!["t.owner_type = 'account'".to_string()];
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(acc) = account {
        conditions.push(format!("a.name LIKE ?{}", bind_values.len() + 1));
        bind_values.push(format!("%{}%", acc));
    }

    if let Some(y) = year {
        conditions.push(format!("strftime('%Y', t.date) = ?{}", bind_values.len() + 1));
        bind_values.push(y.clone());
    }

    // Amount filter: search for exact amount in both directions (Soll + Haben)
    // Uses ABS to find both positive and negative amounts with same absolute value
    if let Some(amt) = amount {
        if let Ok(amt_float) = amt.parse::<f64>() {
            // Convert to stored format (amount * 100) and allow small tolerance
            let amt_cents = (amt_float * 100.0).abs().round() as i64;
            // Search for both +X and -X (Soll und Haben)
            conditions.push(format!("ABS(ABS(t.amount) - {}) <= 1", amt_cents));
        }
    }

    let where_clause = conditions.join(" AND ");

    let sql = format!(r#"
        SELECT
            t.date,
            a.name as account_name,
            t.txn_type,
            t.amount / 100.0 as amount,
            t.currency,
            t.note,
            s.name as security_name,
            s.ticker
        FROM pp_txn t
        JOIN pp_account a ON a.id = t.owner_id
        LEFT JOIN pp_security s ON s.id = t.security_id
        WHERE {}
        ORDER BY t.date DESC
        LIMIT 100
    "#, where_clause);

    // Convert bind values to references for rusqlite
    let bind_refs: Vec<&dyn rusqlite::ToSql> = bind_values.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

    execute_query(conn, &sql, bind_refs.as_slice(), "account_transactions")
}

/// Investment plans overview
fn execute_investment_plans(
    conn: &Connection,
    _params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let sql = r#"
        SELECT
            ip.name as plan_name,
            s.name as security_name,
            s.ticker,
            p.name as portfolio_name,
            ip.amount / 100.0 as amount,
            ip.interval,
            ip.start_date,
            ip.auto_generate,
            ip.note
        FROM pp_investment_plan ip
        JOIN pp_security s ON s.id = ip.security_id
        LEFT JOIN pp_portfolio p ON p.id = ip.portfolio_id
        ORDER BY ip.name
    "#;

    execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "investment_plans")
}

/// Portfolio accounts with current balances
fn execute_portfolio_accounts(
    conn: &Connection,
    _params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let sql = r#"
        SELECT
            a.name as account_name,
            a.currency,
            COALESCE(
                (SELECT SUM(
                    CASE
                        WHEN t.txn_type IN ('DEPOSIT', 'INTEREST', 'FEES_REFUND', 'TAX_REFUND', 'DIVIDENDS', 'SELL') THEN t.amount
                        WHEN t.txn_type IN ('REMOVAL', 'INTEREST_CHARGE', 'FEES', 'TAXES', 'BUY') THEN -t.amount
                        ELSE 0
                    END
                ) FROM pp_txn t WHERE t.owner_type = 'account' AND t.owner_id = a.id),
                0
            ) / 100.0 as balance,
            (SELECT COUNT(*) FROM pp_txn t WHERE t.owner_type = 'account' AND t.owner_id = a.id) as transaction_count,
            (SELECT MAX(t.date) FROM pp_txn t WHERE t.owner_type = 'account' AND t.owner_id = a.id) as last_transaction
        FROM pp_account a
        WHERE a.is_retired = 0
        ORDER BY a.name
    "#;

    execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "portfolio_accounts")
}

/// Tax-relevant sales with holding period
fn execute_tax_relevant_sales(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let year = params.get("year");

    let year_filter = if let Some(y) = year {
        format!("AND strftime('%Y', t.date) = '{}'", y)
    } else {
        String::new()
    };

    let sql = format!(r#"
        SELECT
            t.date as sale_date,
            s.name as security_name,
            s.ticker,
            t.shares / 100000000.0 as shares_sold,
            t.amount / 100.0 as sale_amount,
            t.currency,
            c.shares_consumed / 100000000.0 as lot_shares,
            l.purchase_date,
            julianday(t.date) - julianday(l.purchase_date) as holding_days,
            CASE
                WHEN julianday(t.date) - julianday(l.purchase_date) >= 365 THEN 'STEUERFREI'
                ELSE 'STEUERPFLICHTIG'
            END as tax_status,
            c.gross_amount / 100.0 as cost_basis
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        JOIN pp_fifo_consumption c ON c.sale_txn_id = t.id
        JOIN pp_fifo_lot l ON l.id = c.lot_id
        WHERE t.txn_type IN ('SELL', 'DELIVERY_OUTBOUND')
            AND t.owner_type = 'portfolio'
            {}
        ORDER BY t.date DESC, l.purchase_date ASC
        LIMIT 100
    "#, year_filter);

    execute_query(conn, &sql, &[] as &[&dyn rusqlite::ToSql], "tax_relevant_sales")
}

/// Account balance analysis - finds which transactions explain the current balance
fn execute_account_balance_analysis(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let account = params.get("account")
        .ok_or("Parameter 'account' ist erforderlich")?;

    let search_pattern = format!("%{}%", account);

    // Strategy: Calculate running balance FORWARD through time, identify origin transactions
    // using ROW_NUMBER for correct chronological ordering
    let sql = r#"
        WITH account_info AS (
            SELECT a.id, a.name, a.currency
            FROM pp_account a
            WHERE a.name LIKE ?1
            LIMIT 1
        ),
        -- All transactions with signed amounts (+ = money in, - = money out)
        all_txns AS (
            SELECT
                t.id,
                t.date,
                t.txn_type,
                t.amount,
                CASE
                    WHEN t.txn_type IN ('DEPOSIT', 'INTEREST', 'DIVIDENDS', 'SELL', 'FEES_REFUND', 'TAX_REFUND', 'TRANSFER_IN') THEN t.amount
                    ELSE -t.amount
                END as signed_amount,
                t.note,
                t.security_id,
                ai.name as account_name,
                ai.currency as account_currency
            FROM pp_txn t
            JOIN account_info ai ON t.owner_id = ai.id
            WHERE t.owner_type = 'account'
        ),
        -- Calculate running balance forward through time with row numbers
        -- IMPORTANT: On same day, process INFLOWS before OUTFLOWS (logical order)
        with_running AS (
            SELECT
                at.*,
                ROW_NUMBER() OVER (
                    ORDER BY at.date ASC,
                             CASE WHEN at.txn_type IN ('DIVIDENDS', 'DEPOSIT', 'INTEREST', 'SELL', 'FEES_REFUND', 'TAX_REFUND', 'TRANSFER_IN') THEN 0 ELSE 1 END ASC,
                             at.id ASC
                ) as row_num,
                SUM(at.signed_amount) OVER (
                    ORDER BY at.date ASC,
                             CASE WHEN at.txn_type IN ('DIVIDENDS', 'DEPOSIT', 'INTEREST', 'SELL', 'FEES_REFUND', 'TAX_REFUND', 'TRANSFER_IN') THEN 0 ELSE 1 END ASC,
                             at.id ASC
                ) as running_balance,
                s.name as security_name,
                s.ticker
            FROM all_txns at
            LEFT JOIN pp_security s ON s.id = at.security_id
        ),
        -- Get current balance (final running balance)
        current AS (
            SELECT running_balance as current_balance
            FROM with_running
            ORDER BY row_num DESC
            LIMIT 1
        ),
        -- Find the last row where running balance was <= 0 (using row_num for correct ordering)
        last_zero_point AS (
            SELECT COALESCE(MAX(row_num), 0) as zero_row
            FROM with_running
            WHERE running_balance <= 0
        ),
        -- Get transactions AFTER the last zero point (these explain current balance)
        origin_txns AS (
            SELECT wr.*
            FROM with_running wr, last_zero_point lzp
            WHERE wr.row_num > lzp.zero_row
              AND wr.signed_amount > 0  -- Only inflows
        )
        SELECT
            wr.account_name,
            wr.account_currency,
            wr.date,
            wr.txn_type,
            wr.amount / 100.0 as amount,
            wr.signed_amount / 100.0 as signed_amount,
            wr.running_balance / 100.0 as running_balance,
            wr.note,
            wr.security_name,
            wr.ticker,
            c.current_balance / 100.0 as current_balance,
            -- Mark if this transaction is in the origin set (contributed to current balance)
            CASE WHEN ot.id IS NOT NULL THEN 1 ELSE 0 END as is_origin
        FROM with_running wr
        CROSS JOIN current c
        LEFT JOIN origin_txns ot ON ot.id = wr.id
        ORDER BY wr.row_num DESC
        LIMIT 20
    "#;

    execute_query(conn, sql, &[&search_pattern], "account_balance_analysis")
}

// ============================================================================
// NEW: Performance & Allocation Templates (Phase 1)
// ============================================================================

/// Portfolio performance summary (TTWROR, gains/losses)
fn execute_portfolio_performance_summary(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let period = params.get("period").map(|s| s.to_lowercase());

    // Calculate date range based on period
    let today = chrono::Local::now().date_naive();
    let (start_date, period_label) = match period.as_deref() {
        Some("ytd") => {
            let year_start = chrono::NaiveDate::from_ymd_opt(today.year(), 1, 1)
                .ok_or("Ungültiges Datum")?;
            (year_start, "Jahr bis heute (YTD)".to_string())
        }
        Some("1y") => {
            let one_year_ago = today - chrono::Duration::days(365);
            (one_year_ago, "Letzte 12 Monate".to_string())
        }
        Some("3y") => {
            let three_years_ago = today - chrono::Duration::days(3 * 365);
            (three_years_ago, "Letzte 3 Jahre".to_string())
        }
        Some("5y") => {
            let five_years_ago = today - chrono::Duration::days(5 * 365);
            (five_years_ago, "Letzte 5 Jahre".to_string())
        }
        _ => {
            // "all" or default - find first transaction date
            let first_date: String = conn
                .query_row(
                    "SELECT MIN(date) FROM pp_txn WHERE owner_type IN ('portfolio', 'account')",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| today.to_string());
            let parsed = chrono::NaiveDate::parse_from_str(&first_date, "%Y-%m-%d")
                .unwrap_or(today - chrono::Duration::days(365));
            (parsed, "Seit Beginn".to_string())
        }
    };

    // Query aggregated portfolio data
    let sql = r#"
        WITH portfolio_stats AS (
            -- Total cost basis from FIFO lots
            SELECT
                COALESCE(SUM(l.gross_amount), 0) / 100.0 as total_cost_basis,
                COALESCE(SUM(l.remaining_shares), 0) / 100000000.0 as total_shares
            FROM pp_fifo_lot l
            WHERE l.remaining_shares > 0
        ),
        current_values AS (
            -- Current portfolio value based on holdings × latest price
            SELECT
                COALESCE(SUM(
                    (l.remaining_shares / 100000000.0) * (lp.value / 100000000.0)
                ), 0) as total_value
            FROM pp_fifo_lot l
            JOIN pp_latest_price lp ON lp.security_id = l.security_id
            WHERE l.remaining_shares > 0
        ),
        period_transactions AS (
            -- Count transactions in period
            SELECT
                COUNT(*) as txn_count,
                SUM(CASE WHEN txn_type IN ('BUY', 'DELIVERY_INBOUND') THEN 1 ELSE 0 END) as buy_count,
                SUM(CASE WHEN txn_type IN ('SELL', 'DELIVERY_OUTBOUND') THEN 1 ELSE 0 END) as sell_count
            FROM pp_txn
            WHERE date >= ?1 AND date <= ?2
        ),
        dividends AS (
            -- Total dividends in period
            SELECT COALESCE(SUM(amount), 0) / 100.0 as total_dividends
            FROM pp_txn
            WHERE txn_type = 'DIVIDENDS'
              AND date >= ?1 AND date <= ?2
        ),
        realized AS (
            -- Realized gains from FIFO consumptions
            SELECT
                COALESCE(SUM(
                    (t.amount / 100.0) - (c.gross_amount / 100.0)
                ), 0) as realized_gains
            FROM pp_fifo_consumption c
            JOIN pp_txn t ON t.id = c.sale_txn_id
            WHERE t.date >= ?1 AND t.date <= ?2
        )
        SELECT
            ps.total_cost_basis,
            cv.total_value,
            CASE
                WHEN ps.total_cost_basis > 0
                THEN ((cv.total_value - ps.total_cost_basis) / ps.total_cost_basis) * 100
                ELSE 0
            END as unrealized_return_pct,
            cv.total_value - ps.total_cost_basis as unrealized_gain_loss,
            pt.txn_count,
            pt.buy_count,
            pt.sell_count,
            d.total_dividends,
            r.realized_gains,
            ?3 as period_label,
            ?1 as start_date,
            ?2 as end_date
        FROM portfolio_stats ps, current_values cv, period_transactions pt, dividends d, realized r
    "#;

    let start_str = start_date.to_string();
    let end_str = today.to_string();

    execute_query(conn, sql, &[&start_str, &end_str, &period_label], "portfolio_performance_summary")
}

/// Current holdings with shares, value, and gain/loss
fn execute_current_holdings(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let security = params.get("security");

    // Use SSOT HOLDINGS_SUM_SQL pattern from pp/common.rs
    if let Some(sec) = security {
        let search_pattern = format!("%{}%", sec);
        let sql = r#"
            SELECT
                s.name as security_name,
                s.ticker,
                s.isin,
                s.currency,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) / 100000000.0 as shares,
                COALESCE(lp.value, 0) / 100000000.0 as current_price,
                (SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0) as current_value,
                COALESCE(l.cost_basis, 0) as cost_basis,
                CASE
                    WHEN COALESCE(l.cost_basis, 0) > 0
                    THEN (((SUM(CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0)) - COALESCE(l.cost_basis, 0)) / COALESCE(l.cost_basis, 0) * 100
                    ELSE 0
                END as gain_loss_pct
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
            LEFT JOIN (
                SELECT security_id, SUM(gross_amount) / 100.0 as cost_basis
                FROM pp_fifo_lot
                WHERE remaining_shares > 0
                GROUP BY security_id
            ) l ON l.security_id = s.id
            WHERE t.owner_type = 'portfolio'
              AND t.shares IS NOT NULL
              AND (s.name LIKE ?1 OR s.isin LIKE ?1 OR s.ticker LIKE ?1)
            GROUP BY s.id
            HAVING shares > 0.0001
            ORDER BY current_value DESC
        "#;
        let result = execute_query(conn, sql, &[&search_pattern], "current_holdings");

        // If no results for specific security search, provide helpful error
        match result {
            Ok(qr) if qr.row_count == 0 => {
                let mut enhanced = qr;
                enhanced.formatted_markdown = security_not_found_error(conn, sec);
                Ok(enhanced)
            }
            other => other,
        }
    } else {
        let sql = r#"
            SELECT
                s.name as security_name,
                s.ticker,
                s.isin,
                s.currency,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) / 100000000.0 as shares,
                COALESCE(lp.value, 0) / 100000000.0 as current_price,
                (SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0) as current_value,
                COALESCE(l.cost_basis, 0) as cost_basis,
                CASE
                    WHEN COALESCE(l.cost_basis, 0) > 0
                    THEN (((SUM(CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0)) - COALESCE(l.cost_basis, 0)) / COALESCE(l.cost_basis, 0) * 100
                    ELSE 0
                END as gain_loss_pct
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
            LEFT JOIN (
                SELECT security_id, SUM(gross_amount) / 100.0 as cost_basis
                FROM pp_fifo_lot
                WHERE remaining_shares > 0
                GROUP BY security_id
            ) l ON l.security_id = s.id
            WHERE t.owner_type = 'portfolio'
              AND t.shares IS NOT NULL
            GROUP BY s.id
            HAVING shares > 0.0001
            ORDER BY current_value DESC
            LIMIT 50
        "#;
        execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "current_holdings")
    }
}

/// Unrealized gains/losses for all open positions
fn execute_unrealized_gains_losses(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let filter = params.get("filter").map(|s| s.to_lowercase());

    let having_clause = match filter.as_deref() {
        Some("gains") | Some("gewinne") => "HAVING gain_loss > 0",
        Some("losses") | Some("verluste") => "HAVING gain_loss < 0",
        _ => "",
    };

    let sql = format!(r#"
        SELECT
            s.name as security_name,
            s.ticker,
            s.currency,
            SUM(l.remaining_shares) / 100000000.0 as shares,
            SUM(l.gross_amount) / 100.0 as cost_basis,
            COALESCE(lp.value, 0) / 100000000.0 as current_price,
            (SUM(l.remaining_shares) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0) as current_value,
            ((SUM(l.remaining_shares) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0)) - (SUM(l.gross_amount) / 100.0) as gain_loss,
            CASE
                WHEN SUM(l.gross_amount) > 0
                THEN ((((SUM(l.remaining_shares) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0)) - (SUM(l.gross_amount) / 100.0)) / (SUM(l.gross_amount) / 100.0)) * 100
                ELSE 0
            END as gain_loss_pct
        FROM pp_fifo_lot l
        JOIN pp_security s ON s.id = l.security_id
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        WHERE l.remaining_shares > 0
        GROUP BY s.id
        {}
        ORDER BY gain_loss DESC
    "#, having_clause);

    execute_query(conn, &sql, &[] as &[&dyn rusqlite::ToSql], "unrealized_gains_losses")
}

/// Realized gains by year from FIFO consumptions
fn execute_realized_gains_by_year(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let year = params.get("year");

    if let Some(y) = year {
        let sql = r#"
            SELECT
                strftime('%Y', t.date) as year,
                s.name as security_name,
                s.ticker,
                t.date as sale_date,
                c.shares_consumed / 100000000.0 as shares_sold,
                c.gross_amount / 100.0 as cost_basis,
                t.amount / 100.0 as sale_amount,
                (t.amount / 100.0) - (c.gross_amount / 100.0) as realized_gain,
                CASE
                    WHEN c.gross_amount > 0
                    THEN (((t.amount / 100.0) - (c.gross_amount / 100.0)) / (c.gross_amount / 100.0)) * 100
                    ELSE 0
                END as realized_gain_pct,
                l.purchase_date,
                julianday(t.date) - julianday(l.purchase_date) as holding_days
            FROM pp_fifo_consumption c
            JOIN pp_txn t ON t.id = c.sale_txn_id
            JOIN pp_fifo_lot l ON l.id = c.lot_id
            JOIN pp_security s ON s.id = l.security_id
            WHERE strftime('%Y', t.date) = ?1
            ORDER BY t.date DESC
            LIMIT 100
        "#;
        let result = execute_query(conn, sql, &[&y.as_str()], "realized_gains_by_year")?;

        if result.rows.is_empty() {
            return Err(no_transactions_for_year_error("Realisierte Gewinne/Verluste", y));
        }
        Ok(result)
    } else {
        // Summary by year
        let sql = r#"
            SELECT
                strftime('%Y', t.date) as year,
                COUNT(DISTINCT t.id) as sale_count,
                SUM(c.shares_consumed) / 100000000.0 as total_shares_sold,
                SUM(c.gross_amount) / 100.0 as total_cost_basis,
                SUM(t.amount) / 100.0 as total_sale_amount,
                SUM((t.amount / 100.0) - (c.gross_amount / 100.0)) as total_realized_gain
            FROM pp_fifo_consumption c
            JOIN pp_txn t ON t.id = c.sale_txn_id
            JOIN pp_fifo_lot l ON l.id = c.lot_id
            GROUP BY strftime('%Y', t.date)
            ORDER BY year DESC
        "#;
        execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "realized_gains_by_year")
    }
}

/// Portfolio allocation by currency or asset type
fn execute_portfolio_allocation(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let group_by = params.get("by").map(|s| s.to_lowercase());

    match group_by.as_deref() {
        Some("currency") | Some("währung") => {
            let sql = r#"
                WITH holdings AS (
                    SELECT
                        s.id,
                        s.currency,
                        (SUM(CASE
                            WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                            WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                            ELSE 0
                        END) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0) as value
                    FROM pp_txn t
                    JOIN pp_security s ON s.id = t.security_id
                    LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
                    WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
                    GROUP BY s.id
                    HAVING SUM(CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END) > 0
                ),
                totals AS (
                    SELECT SUM(value) as total_value FROM holdings
                )
                SELECT
                    h.currency,
                    COUNT(*) as position_count,
                    SUM(h.value) as total_value,
                    (SUM(h.value) / t.total_value) * 100 as allocation_pct
                FROM holdings h, totals t
                WHERE t.total_value > 0
                GROUP BY h.currency
                ORDER BY allocation_pct DESC
            "#;
            execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "portfolio_allocation")
        }
        Some("type") | Some("typ") | Some("asset") => {
            // Asset type based on security feed/characteristics
            let sql = r#"
                WITH holdings AS (
                    SELECT
                        s.id,
                        s.feed,
                        s.name,
                        (SUM(CASE
                            WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                            WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                            ELSE 0
                        END) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0) as value
                    FROM pp_txn t
                    JOIN pp_security s ON s.id = t.security_id
                    LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
                    WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
                    GROUP BY s.id
                    HAVING SUM(CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END) > 0
                ),
                classified AS (
                    SELECT
                        CASE
                            WHEN h.feed IN ('COINGECKO', 'KRAKEN') THEN 'Krypto'
                            WHEN LOWER(h.name) LIKE '%etf%' OR LOWER(h.name) LIKE '%index%' THEN 'ETF/Fonds'
                            WHEN LOWER(h.name) LIKE '%gold%' OR LOWER(h.name) LIKE '%silber%' THEN 'Edelmetalle'
                            WHEN LOWER(h.name) LIKE '%bond%' OR LOWER(h.name) LIKE '%anleihe%' THEN 'Anleihen'
                            ELSE 'Aktien'
                        END as asset_type,
                        h.value
                    FROM holdings h
                ),
                totals AS (
                    SELECT SUM(value) as total_value FROM holdings
                )
                SELECT
                    c.asset_type,
                    COUNT(*) as position_count,
                    SUM(c.value) as total_value,
                    (SUM(c.value) / t.total_value) * 100 as allocation_pct
                FROM classified c, totals t
                WHERE t.total_value > 0
                GROUP BY c.asset_type
                ORDER BY allocation_pct DESC
            "#;
            execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "portfolio_allocation")
        }
        _ => {
            // Both: show currency and position count
            let sql = r#"
                WITH holdings AS (
                    SELECT
                        s.id,
                        s.name,
                        s.currency,
                        s.feed,
                        (SUM(CASE
                            WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                            WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                            ELSE 0
                        END) / 100000000.0) * (COALESCE(lp.value, 0) / 100000000.0) as value
                    FROM pp_txn t
                    JOIN pp_security s ON s.id = t.security_id
                    LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
                    WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
                    GROUP BY s.id
                    HAVING SUM(CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END) > 0
                ),
                totals AS (
                    SELECT SUM(value) as total_value FROM holdings
                )
                SELECT
                    'Gesamt' as category,
                    COUNT(*) as position_count,
                    SUM(h.value) as total_value,
                    100.0 as allocation_pct,
                    GROUP_CONCAT(DISTINCT h.currency) as currencies
                FROM holdings h, totals t
                UNION ALL
                SELECT
                    h.currency as category,
                    COUNT(*) as position_count,
                    SUM(h.value) as total_value,
                    (SUM(h.value) / t.total_value) * 100 as allocation_pct,
                    h.currency as currencies
                FROM holdings h, totals t
                WHERE t.total_value > 0
                GROUP BY h.currency
                ORDER BY CASE WHEN category = 'Gesamt' THEN 0 ELSE 1 END, allocation_pct DESC
            "#;
            execute_query(conn, sql, &[] as &[&dyn rusqlite::ToSql], "portfolio_allocation")
        }
    }
}

/// Securities held in multiple portfolios
fn execute_securities_in_multiple_portfolios(
    conn: &Connection,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    let min_portfolios = params
        .get("min_portfolios")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(2);

    let sql = r#"
        WITH current_holdings AS (
            -- Step 1: Calculate current holdings per security per portfolio
            SELECT
                t.security_id,
                t.owner_id as portfolio_id,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) / 100000000.0 as shares
            FROM pp_txn t
            WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
            GROUP BY t.security_id, t.owner_id
        ),
        positive_holdings AS (
            -- Step 2: Only keep holdings with actual positive shares (> 0.01)
            SELECT
                h.security_id,
                h.portfolio_id,
                h.shares,
                s.name as security_name,
                s.ticker,
                s.isin,
                p.name as portfolio_name
            FROM current_holdings h
            JOIN pp_security s ON s.id = h.security_id
            JOIN pp_portfolio p ON p.id = h.portfolio_id
            WHERE h.shares > 0.01
        )
        -- Step 3: Aggregate by ISIN (same underlying security across different listings)
        SELECT
            MIN(security_name) as security_name,
            GROUP_CONCAT(DISTINCT ticker) as ticker,
            isin,
            COUNT(*) as depot_count,
            GROUP_CONCAT(portfolio_name || ': ' || PRINTF('%.2f', shares) || ' Stk.', ' | ') as in_depots,
            ROUND(SUM(shares), 2) as gesamt
        FROM positive_holdings
        WHERE isin IS NOT NULL AND LENGTH(isin) > 0
        GROUP BY isin
        HAVING COUNT(*) >= ?1
        ORDER BY depot_count DESC, gesamt DESC
    "#;

    execute_query(conn, sql, &[&min_portfolios], "securities_in_multiple_portfolios")
}

// ============================================================================
// Helper - Improved Error Handling with Suggestions
// ============================================================================

/// Find similar securities for error messages with suggestions
fn find_similar_securities(conn: &Connection, search: &str) -> Vec<String> {
    let search_lower = search.to_lowercase();

    // Try to find securities with similar names
    let sql = r#"
        SELECT name, ticker, isin
        FROM pp_security
        WHERE name IS NOT NULL
        ORDER BY
            CASE
                WHEN LOWER(name) LIKE ? THEN 1
                WHEN LOWER(ticker) LIKE ? THEN 2
                WHEN LOWER(name) LIKE ? THEN 3
                ELSE 4
            END,
            name
        LIMIT 5
    "#;

    let exact_pattern = format!("{}%", search_lower);
    let contains_pattern = format!("%{}%", search_lower);

    conn.prepare(sql)
        .and_then(|mut stmt| {
            stmt.query_map([&exact_pattern, &exact_pattern, &contains_pattern], |row| {
                let name: String = row.get(0)?;
                let ticker: Option<String> = row.get(1)?;
                let isin: Option<String> = row.get(2)?;

                let mut result = name.clone();
                if let Some(t) = ticker {
                    result = format!("{} ({})", name, t);
                } else if let Some(i) = isin {
                    result = format!("{} [{}]", name, i);
                }
                Ok(result)
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        })
        .unwrap_or_default()
}

/// Generate an error message when security is not found, with suggestions
fn security_not_found_error(conn: &Connection, search: &str) -> String {
    let suggestions = find_similar_securities(conn, search);

    if suggestions.is_empty() {
        format!("Wertpapier '{}' nicht gefunden. Keine ähnlichen Wertpapiere in der Datenbank.", search)
    } else {
        format!(
            "Wertpapier '{}' nicht gefunden. Meinten Sie: {}?",
            search,
            suggestions.join(", ")
        )
    }
}

/// Generate an error message when no transactions are found for a year
fn no_transactions_for_year_error(search_type: &str, year: &str) -> String {
    let current_year = chrono::Local::now().format("%Y").to_string();
    let prev_year = (year.parse::<i32>().unwrap_or(2024) - 1).to_string();

    if year == current_year {
        format!(
            "Keine {} für {} gefunden. Versuche das Vorjahr ({}) oder 'all' für alle Jahre.",
            search_type, year, prev_year
        )
    } else {
        format!(
            "Keine {} für {} gefunden. Versuche ein anderes Jahr oder 'all' für alle Jahre.",
            search_type, year
        )
    }
}

/// Translate transaction type to user-friendly German
fn translate_txn_type(txn_type: &str) -> &str {
    match txn_type {
        "BUY" => "Kauf",
        "SELL" => "Verkauf",
        "DELIVERY_INBOUND" => "Einlieferung",
        "DELIVERY_OUTBOUND" => "Auslieferung",
        "TRANSFER_IN" => "Transfer ein",
        "TRANSFER_OUT" => "Transfer aus",
        "DIVIDENDS" => "Dividende",
        "DEPOSIT" => "Einzahlung",
        "REMOVAL" => "Auszahlung",
        "INTEREST" => "Zinsen",
        "FEES" => "Gebühren",
        "TAXES" => "Steuern",
        _ => txn_type,
    }
}

/// Format a date from YYYY-MM-DD (or YYYY-MM-DD HH:MM:SS) to DD.MM.YYYY
fn format_date_german(date: &str) -> String {
    // Handle datetime format "YYYY-MM-DD HH:MM:SS" - take only date part
    let date_only = date.split_whitespace().next().unwrap_or(date);

    if date_only.len() == 10 && date_only.contains('-') {
        let parts: Vec<&str> = date_only.split('-').collect();
        if parts.len() == 3 {
            return format!("{}.{}.{}", parts[2], parts[1], parts[0]);
        }
    }
    date.to_string()
}

/// Format a number with German locale (comma as decimal separator)
fn format_number_german(value: f64, decimals: usize) -> String {
    let formatted = format!("{:.1$}", value, decimals);
    formatted.replace('.', ",")
}

/// Special formatting for account balance analysis - provides a SHORT, DIRECT explanation
fn format_account_balance_analysis(rows: &[HashMap<String, serde_json::Value>]) -> String {
    if rows.is_empty() {
        return "Keine Buchungen gefunden.".to_string();
    }

    // Extract summary info from first row
    let first_row = &rows[0];
    let account_name = first_row.get("account_name").and_then(|v| v.as_str()).unwrap_or("Konto");
    let currency = first_row.get("account_currency").and_then(|v| v.as_str()).unwrap_or("EUR");
    let current_balance = first_row.get("current_balance").and_then(|v| v.as_f64()).unwrap_or(0.0);

    // Find origin transactions (those marked with is_origin = 1)
    let origin_txns: Vec<&HashMap<String, serde_json::Value>> = rows.iter()
        .filter(|r| r.get("is_origin").and_then(|v| v.as_i64()).unwrap_or(0) == 1)
        .collect();

    // Generate SHORT, DIRECT answer
    if current_balance.abs() < 0.01 {
        return format!("{}: Saldo 0,00 {} - alle Eingänge wurden ausgegeben.", account_name, currency);
    }

    if current_balance < 0.0 {
        return format!("{}: Konto ist {} {} im Minus.", account_name, format_number_german(current_balance.abs(), 2), currency);
    }

    // Positive balance - explain origin
    if origin_txns.len() == 1 {
        let origin = &origin_txns[0];
        let date = origin.get("date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
        let txn_type = origin.get("txn_type").and_then(|v| v.as_str()).map(translate_txn_type).unwrap_or("-");
        let security = origin.get("security_name").and_then(|v| v.as_str());

        let source = if let Some(sec) = security {
            format!("{} von {}", txn_type, sec)
        } else {
            txn_type.to_string()
        };

        format!("{}: Die {} {} stammen aus der {} am {}.",
            account_name, format_number_german(current_balance, 2), currency, source, date)
    } else if origin_txns.is_empty() {
        format!("{}: Saldo {} {} (Summe aller Buchungen).",
            account_name, format_number_german(current_balance, 2), currency)
    } else {
        // Multiple origins - list them briefly
        let mut output = format!("{}: Die {} {} stammen aus:\n",
            account_name, format_number_german(current_balance, 2), currency);

        for origin in &origin_txns {
            let date = origin.get("date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
            let txn_type = origin.get("txn_type").and_then(|v| v.as_str()).map(translate_txn_type).unwrap_or("-");
            let amount = origin.get("signed_amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let security = origin.get("security_name").and_then(|v| v.as_str());

            let sec_str = security.map(|s| format!(" ({})", s)).unwrap_or_default();
            output.push_str(&format!("• {} {} +{} {}{}\n", date, txn_type, format_number_german(amount, 2), currency, sec_str));
        }
        output
    }
}

/// Format query results as a simple list (no table)
fn format_as_markdown(template_id: &str, _columns: &[String], rows: &[HashMap<String, serde_json::Value>]) -> String {
    if rows.is_empty() {
        return "Keine Ergebnisse gefunden.".to_string();
    }

    // Special handling for account_balance_analysis - generate summary explanation
    if template_id == "account_balance_analysis" {
        return format_account_balance_analysis(rows);
    }

    let mut lines: Vec<String> = Vec::new();

    for row in rows {
        let line = match template_id {
            "security_transactions" => {
                let date = row.get("date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let txn_type = row.get("txn_type").and_then(|v| v.as_str()).map(translate_txn_type).unwrap_or("-");
                let shares = row.get("shares").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let amount = row.get("amount").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                format!("• {} – {} {} Stück für {} {}", date, txn_type, shares, amount, currency)
            }
            "dividends_by_security" => {
                let date = row.get("date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let gross = row.get("gross_amount").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let taxes = row.get("taxes").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or("0,00".to_string());
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                format!("• {} – {} {} brutto, {} {} Steuern", date, gross, currency, taxes, currency)
            }
            "all_dividends" => {
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let count = row.get("dividend_count").and_then(|v| v.as_i64()).unwrap_or(0);
                let total = row.get("total_gross").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                format!("• {} – {} Zahlungen, gesamt {} EUR", name, count, total)
            }
            "transactions_by_date" => {
                let date = row.get("date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let txn_type = row.get("txn_type").and_then(|v| v.as_str()).map(translate_txn_type).unwrap_or("-");
                let security = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let amount = row.get("amount").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                format!("• {} – {} {} für {} {}", date, txn_type, security, amount, currency)
            }
            "security_cost_basis" => {
                let date = row.get("purchase_date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let shares = row.get("remaining_shares").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let cost_per = row.get("cost_per_share").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                format!("• {} – {} Stück @ {} {}/Stück", date, shares, cost_per, currency)
            }
            "sold_securities" => {
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let date = row.get("last_sale_date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let shares = row.get("total_sold").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                format!("• {} – {} Stück verkauft (letzter: {})", name, shares, date)
            }
            "holding_period_analysis" => {
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let ticker = row.get("ticker").and_then(|v| v.as_str()).map(|t| format!(" ({})", t)).unwrap_or_default();
                let purchase_date = row.get("purchase_date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let holding_days = row.get("holding_days").and_then(|v| v.as_f64()).map(|d| d as i64).unwrap_or(0);
                let shares = row.get("shares").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 4)).unwrap_or_default();
                let tax_status = row.get("tax_status").and_then(|v| v.as_str()).unwrap_or("-");
                let tax_free_date = row.get("tax_free_date").and_then(|v| v.as_str()).map(format_date_german);

                let status_icon = if tax_status == "STEUERFREI" { "✅" } else { "⏳" };
                let date_info = if let Some(date) = tax_free_date {
                    format!(" → steuerfrei ab {}", date)
                } else {
                    String::new()
                };

                format!("• {}{}{} – Kauf: {}, {} Stück, {} Tage gehalten, {}{}",
                    status_icon, name, ticker, purchase_date, shares, holding_days, tax_status, date_info)
            }
            "fifo_lot_details" => {
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let purchase_date = row.get("purchase_date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let holding_days = row.get("holding_days").and_then(|v| v.as_f64()).map(|d| d as i64).unwrap_or(0);
                let remaining = row.get("remaining_shares").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 4)).unwrap_or_default();
                let cost_per = row.get("cost_per_share").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                let tax_status = row.get("tax_status").and_then(|v| v.as_str()).unwrap_or("-");

                let status_icon = if tax_status == "STEUERFREI" { "✅" } else { "⏳" };

                format!("• {} {} – Kauf: {}, {} Stück @ {} {}, {} Tage ({})",
                    status_icon, name, purchase_date, remaining, cost_per, currency, holding_days, tax_status)
            }
            "account_transactions" => {
                let date = row.get("date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let account = row.get("account_name").and_then(|v| v.as_str()).unwrap_or("-");
                let txn_type = row.get("txn_type").and_then(|v| v.as_str()).map(translate_txn_type).unwrap_or("-");
                let amount = row.get("amount").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                let note = row.get("note").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
                let security = row.get("security_name").and_then(|v| v.as_str());
                let ticker = row.get("ticker").and_then(|v| v.as_str());

                let mut details = Vec::new();
                if let Some(sec) = security {
                    let ticker_str = ticker.map(|t| format!(" ({})", t)).unwrap_or_default();
                    details.push(format!("Wertpapier: {}{}", sec, ticker_str));
                }
                if let Some(n) = note {
                    details.push(format!("Notiz: {}", n));
                }

                let details_str = if details.is_empty() {
                    String::new()
                } else {
                    format!(" | {}", details.join(", "))
                };

                format!("• {} – {} @ {}: {} {}{}", date, txn_type, account, amount, currency, details_str)
            }
            "investment_plans" => {
                let plan_name = row.get("plan_name").and_then(|v| v.as_str()).unwrap_or("-");
                let security = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let ticker = row.get("ticker").and_then(|v| v.as_str()).map(|t| format!(" ({})", t)).unwrap_or_default();
                let amount = row.get("amount").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let interval = row.get("interval").and_then(|v| v.as_str()).unwrap_or("-");
                let start_date = row.get("start_date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();

                let interval_de = match interval {
                    "MONTHLY" => "monatlich",
                    "QUARTERLY" => "quartalsweise",
                    "YEARLY" => "jährlich",
                    "WEEKLY" => "wöchentlich",
                    _ => interval,
                };

                format!("• {} – {}{}: {} EUR {}, Start: {}", plan_name, security, ticker, amount, interval_de, start_date)
            }
            "portfolio_accounts" => {
                let name = row.get("account_name").and_then(|v| v.as_str()).unwrap_or("-");
                let balance = row.get("balance").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                let txn_count = row.get("transaction_count").and_then(|v| v.as_i64()).unwrap_or(0);
                let last_txn = row.get("last_transaction").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_else(|| "-".to_string());
                format!("• {} – Saldo: {} {}, {} Buchungen, letzte: {}", name, balance, currency, txn_count, last_txn)
            }
            "tax_relevant_sales" => {
                let sale_date = row.get("sale_date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let shares = row.get("shares_sold").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 4)).unwrap_or_default();
                let sale_amount = row.get("sale_amount").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                let holding_days = row.get("holding_days").and_then(|v| v.as_f64()).map(|d| d as i64).unwrap_or(0);
                let tax_status = row.get("tax_status").and_then(|v| v.as_str()).unwrap_or("-");
                let cost_basis = row.get("cost_basis").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();

                let status_icon = if tax_status == "STEUERFREI" { "✅" } else { "⚠️" };

                format!("• {} {} – {} {} Stück verkauft für {} {}, {} Tage gehalten, Einstand: {} {} ({})",
                    status_icon, sale_date, name, shares, sale_amount, currency, holding_days, cost_basis, currency, tax_status)
            }
            // NEW: Performance & Allocation Templates (Phase 1)
            "portfolio_performance_summary" => {
                let period = row.get("period_label").and_then(|v| v.as_str()).unwrap_or("Gesamt");
                let cost_basis = row.get("total_cost_basis").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let value = row.get("total_value").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let unrealized_pct = row.get("unrealized_return_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let unrealized_gl = row.get("unrealized_gain_loss").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let dividends = row.get("total_dividends").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let realized = row.get("realized_gains").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();

                let gain_icon = if unrealized_pct >= 0.0 { "📈" } else { "📉" };
                format!(
                    "**{}**\n• Einstandswert: {} EUR\n• Aktueller Wert: {} EUR\n• {} Unrealisiert: {} EUR ({:+.2}%)\n• Dividenden: {} EUR\n• Realisierte G/V: {} EUR",
                    period, cost_basis, value, gain_icon, unrealized_gl, unrealized_pct, dividends, realized
                )
            }
            "current_holdings" => {
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let ticker = row.get("ticker").and_then(|v| v.as_str()).map(|t| format!(" ({})", t)).unwrap_or_default();
                let shares = row.get("shares").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 4)).unwrap_or_default();
                let value = row.get("current_value").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");
                let gain_pct = row.get("gain_loss_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let gain_icon = if gain_pct >= 0.0 { "🟢" } else { "🔴" };
                format!("• {}{}{} – {} Stück = {} {} ({:+.2}%)", gain_icon, name, ticker, shares, value, currency, gain_pct)
            }
            "unrealized_gains_losses" => {
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let ticker = row.get("ticker").and_then(|v| v.as_str()).map(|t| format!(" ({})", t)).unwrap_or_default();
                let gain = row.get("gain_loss").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let gain_pct = row.get("gain_loss_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let currency = row.get("currency").and_then(|v| v.as_str()).unwrap_or("EUR");

                let gain_icon = if gain >= 0.0 { "🟢" } else { "🔴" };
                format!("• {}{}{} – {:+.2} {} ({:+.2}%)", gain_icon, name, ticker, gain, currency, gain_pct)
            }
            "realized_gains_by_year" => {
                // Check if it's summary (has year, sale_count) or detail (has sale_date)
                if row.contains_key("sale_count") {
                    // Summary view
                    let year = row.get("year").and_then(|v| v.as_str()).unwrap_or("-");
                    let count = row.get("sale_count").and_then(|v| v.as_i64()).unwrap_or(0);
                    let total_gain = row.get("total_realized_gain").and_then(|v| v.as_f64()).unwrap_or(0.0);

                    let gain_icon = if total_gain >= 0.0 { "📈" } else { "📉" };
                    format!("• {} {} – {} Verkäufe, Gesamt: {:+.2} EUR", gain_icon, year, count, total_gain)
                } else {
                    // Detail view
                    let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                    let sale_date = row.get("sale_date").and_then(|v| v.as_str()).map(format_date_german).unwrap_or_default();
                    let gain = row.get("realized_gain").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let holding_days = row.get("holding_days").and_then(|v| v.as_f64()).map(|d| d as i64).unwrap_or(0);

                    let gain_icon = if gain >= 0.0 { "🟢" } else { "🔴" };
                    format!("• {}{} – {} – {:+.2} EUR ({} Tage gehalten)", gain_icon, sale_date, name, gain, holding_days)
                }
            }
            "portfolio_allocation" => {
                let category = row.get("category").or(row.get("currency")).or(row.get("asset_type")).and_then(|v| v.as_str()).unwrap_or("-");
                let count = row.get("position_count").and_then(|v| v.as_i64()).unwrap_or(0);
                let value = row.get("total_value").and_then(|v| v.as_f64()).map(|f| format_number_german(f, 2)).unwrap_or_default();
                let pct = row.get("allocation_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);

                if category == "Gesamt" {
                    format!("**{}**: {} Positionen, {} EUR", category, count, value)
                } else {
                    format!("• {} – {} Positionen, {} EUR ({:.1}%)", category, count, value, pct)
                }
            }
            "securities_in_multiple_portfolios" => {
                let name = row.get("security_name").and_then(|v| v.as_str()).unwrap_or("-");
                let ticker = row.get("ticker").and_then(|v| v.as_str()).map(|t| format!(" ({})", t)).unwrap_or_default();
                let depot_count = row.get("depot_count").and_then(|v| v.as_i64()).unwrap_or(0);
                let in_depots = row.get("in_depots").and_then(|v| v.as_str()).unwrap_or("-");
                let gesamt = row.get("gesamt").and_then(|v| v.as_f64()).unwrap_or(0.0);

                format!("• **{}{}** ({} Depots, {:.2} Stk. gesamt): {}", name, ticker, depot_count, gesamt, in_depots)
            }
            // Note: account_balance_analysis is handled by format_account_balance_analysis() above
            _ => format!("{:?}", row),
        };
        lines.push(line);
    }

    lines.join("\n")
}

/// Execute a SQL query and return results as QueryResult
fn execute_query<P>(
    conn: &Connection,
    sql: &str,
    params: P,
    template_id: &str,
) -> Result<QueryResult, String>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare(sql).map_err(|e| format!("SQL Fehler: {}", e))?;

    let column_names: Vec<String> = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let rows_iter = stmt
        .query_map(params, |row| {
            let mut row_map = HashMap::new();
            for (i, col_name) in column_names.iter().enumerate() {
                let value: serde_json::Value = match row.get_ref(i) {
                    Ok(rusqlite::types::ValueRef::Null) => serde_json::Value::Null,
                    Ok(rusqlite::types::ValueRef::Integer(i)) => serde_json::json!(i),
                    Ok(rusqlite::types::ValueRef::Real(f)) => {
                        // Round to 2 decimal places for amounts, 4 for shares
                        let rounded = if col_name.contains("shares") {
                            (f * 10000.0).round() / 10000.0
                        } else {
                            (f * 100.0).round() / 100.0
                        };
                        serde_json::json!(rounded)
                    }
                    Ok(rusqlite::types::ValueRef::Text(t)) => {
                        serde_json::json!(String::from_utf8_lossy(t))
                    }
                    Ok(rusqlite::types::ValueRef::Blob(b)) => {
                        serde_json::json!(format!("<blob:{} bytes>", b.len()))
                    }
                    Err(_) => serde_json::Value::Null,
                };
                row_map.insert(col_name.clone(), value);
            }
            Ok(row_map)
        })
        .map_err(|e| format!("Query Fehler: {}", e))?;

    let rows: Vec<HashMap<String, serde_json::Value>> = rows_iter
        .filter_map(|r| r.ok())
        .collect();

    let row_count = rows.len();
    let formatted_markdown = format_as_markdown(template_id, &column_names, &rows);

    Ok(QueryResult {
        template_id: template_id.to_string(),
        columns: column_names,
        rows,
        row_count,
        formatted_markdown,
    })
}

// ============================================================================
// System Prompt Helper
// ============================================================================

/// Generate the query templates section for the chat system prompt
pub fn get_templates_for_prompt() -> String {
    r#"=== DATENBANK-ABFRAGEN ===
Du kannst gezielte Abfragen auf der Portfolio-Datenbank ausführen. Wenn eine Frage nach spezifischen Transaktionen, Dividenden oder historischen Daten fragt, nutze eine Abfrage.

TRANSAKTIONSTYPEN (für Benutzer vereinfachen!):
- BUY, DELIVERY_INBOUND = Kauf/Eingang (als "gekauft" bezeichnen)
- SELL, DELIVERY_OUTBOUND = Verkauf/Ausgang (als "verkauft" bezeichnen)

Verfügbare Abfragen:

1. security_transactions - ALLE Transaktionen für ein Wertpapier
   Parameter: security (Name/ISIN/Ticker), txn_type (optional: BUY, SELL, DELIVERY_INBOUND, DELIVERY_OUTBOUND)
   Beispiel: "Wann habe ich Apple gekauft?" oder "Welche Einlieferungen hatte ich bei Tesla?"

2. dividends_by_security - Dividenden für ein Wertpapier
   Parameter: security (Name/ISIN/Ticker)
   Beispiel: "Welche Dividenden habe ich von Microsoft erhalten?"

3. all_dividends - Alle Dividenden (gruppiert)
   Parameter: year (optional, z.B. "2024")
   Beispiel: "Zeige alle Dividenden von 2024"

4. transactions_by_date - Transaktionen in Zeitraum
   Parameter: from_date, to_date (YYYY-MM-DD), txn_type (optional)
   Beispiel: "Welche Käufe habe ich im Januar 2024 gemacht?"

5. security_cost_basis - Einstandskurse (FIFO-Lots)
   Parameter: security (Name/ISIN/Ticker)
   Beispiel: "Was ist mein Einstandskurs bei Tesla?"

6. sold_securities - Verkaufte Positionen
   Keine Parameter
   Beispiel: "Welche Aktien habe ich verkauft?"

7. holding_period_analysis - HALTEFRIST-ANALYSE (§ 23 EStG)
   Parameter: asset_type (optional: "crypto", "gold", oder leer für alle)
   Beispiel: "Welche Krypto-Positionen sind schon steuerfrei?" oder "Haltefrist meines Goldes?"
   WICHTIG: Krypto und Gold sind in Deutschland nach 1 Jahr Haltefrist steuerfrei!
   ✅ = steuerfrei (>365 Tage), ⏳ = noch steuerpflichtig (mit Datum wann steuerfrei)

8. fifo_lot_details - Detaillierte FIFO-Lots mit Haltefrist
   Parameter: security (optional, Name/ISIN/Ticker)
   Beispiel: "Zeige alle FIFO-Lots für Bitcoin" oder "Alle meine Kaufpositionen"

9. account_transactions - Kontobewegungen
   Parameter: account (optional, Kontoname), year (optional)
   Beispiel: "Zeige alle Einzahlungen 2024" oder "Kontobewegungen von Depot 1"

10. investment_plans - Alle Sparpläne
    Keine Parameter
    Beispiel: "Welche Sparpläne habe ich?"

11. portfolio_accounts - Konten mit Salden
    Keine Parameter
    Beispiel: "Wie hoch sind meine Kontostände?" oder "Zeige alle Konten"

12. tax_relevant_sales - Verkäufe mit Steuerinfo
    Parameter: year (optional)
    Beispiel: "Welche Verkäufe 2024 waren steuerpflichtig?" oder "Steuerrelevante Verkäufe"
    ✅ = steuerfrei (>365 Tage gehalten), ⚠️ = steuerpflichtig

WICHTIG: Wenn du eine Abfrage benötigst, antworte NUR mit diesem JSON-Format:
```json
{"query": "template_id", "params": {"key": "value"}}
```

Das System führt die Abfrage aus und sendet dir die Ergebnisse. Formuliere dann eine hilfreiche Antwort.

=== HALTEFRIST-REGELUNG (§ 23 EStG) ===
Private Veräußerungsgeschäfte (Krypto, Gold, andere Rohstoffe) sind nach 1 Jahr Haltefrist STEUERFREI!
- Bitcoin, Ethereum, andere Kryptowährungen: Nach 365 Tagen steuerfrei
- Physisches Gold (und Goldmünzen): Nach 365 Tagen steuerfrei
- Silber, Platin, andere Rohstoffe: Nach 365 Tagen steuerfrei
- ACHTUNG: Aktien, ETFs, Fonds unterliegen der Abgeltungssteuer (25%) - keine Haltefrist!

Nutze Abfragen für:
- Konkrete Fragen zu Transaktionen ("Wann habe ich X gekauft/verkauft?")
- Dividenden-Fragen ("Welche Dividenden von X?")
- Historische Daten ("Was war mein Einstandskurs?")
- Zeitraum-Fragen ("Transaktionen im Jahr 2024")
- HALTEFRIST-Fragen ("Ist mein Bitcoin steuerfrei?", "Wann kann ich Gold steuerfrei verkaufen?")
- Konto-Fragen ("Meine Einzahlungen", "Kontostände")
- Sparplan-Fragen ("Welche Sparpläne habe ich?")

Nutze KEINE Abfrage für:
- Allgemeine Portfolio-Übersicht (diese Daten hast du bereits im Kontext)
- Aktuelle Holdings und Werte
- Performance-Berechnungen"#.to_string()
}

/// Generate the query templates section including user-defined templates
pub fn get_templates_for_prompt_with_user_templates(conn: &Connection) -> String {
    let mut prompt = get_templates_for_prompt();

    // Append user-defined templates if any exist
    if let Ok(user_templates) = super::user_templates::get_enabled_user_templates(conn) {
        if !user_templates.is_empty() {
            prompt.push_str("\n\n=== BENUTZERDEFINIERTE ABFRAGEN ===\n");
            prompt.push_str("Der Benutzer hat eigene Abfragen definiert:\n\n");

            for (i, template) in user_templates.iter().enumerate() {
                prompt.push_str(&format!(
                    "{}. {} - {}\n",
                    i + 13, // Continue numbering from built-in templates
                    template.template_id,
                    template.description
                ));

                if !template.parameters.is_empty() {
                    prompt.push_str("   Parameter: ");
                    let param_strs: Vec<String> = template
                        .parameters
                        .iter()
                        .map(|p| {
                            if p.required {
                                format!("{} ({})", p.param_name, p.param_type)
                            } else {
                                format!("{} (optional, {})", p.param_name, p.param_type)
                            }
                        })
                        .collect();
                    prompt.push_str(&param_strs.join(", "));
                    prompt.push('\n');
                }
            }
        }
    }

    prompt
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Create an in-memory test database with sample data
    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // Create tables (simplified schema matching the real one)
        conn.execute_batch(
            r#"
            CREATE TABLE pp_security (
                id INTEGER PRIMARY KEY,
                uuid TEXT NOT NULL,
                name TEXT NOT NULL,
                currency TEXT NOT NULL,
                isin TEXT,
                wkn TEXT,
                ticker TEXT
            );

            CREATE TABLE pp_portfolio (
                id INTEGER PRIMARY KEY,
                uuid TEXT NOT NULL,
                name TEXT NOT NULL
            );

            CREATE TABLE pp_account (
                id INTEGER PRIMARY KEY,
                uuid TEXT NOT NULL,
                name TEXT NOT NULL,
                currency TEXT NOT NULL
            );

            CREATE TABLE pp_txn (
                id INTEGER PRIMARY KEY,
                uuid TEXT NOT NULL,
                owner_type TEXT NOT NULL,
                owner_id INTEGER NOT NULL,
                security_id INTEGER,
                txn_type TEXT NOT NULL,
                date TEXT NOT NULL,
                amount INTEGER,
                currency TEXT,
                shares INTEGER,
                note TEXT
            );

            CREATE TABLE pp_txn_unit (
                id INTEGER PRIMARY KEY,
                txn_id INTEGER NOT NULL,
                unit_type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                currency TEXT NOT NULL
            );

            CREATE TABLE pp_fifo_lot (
                id INTEGER PRIMARY KEY,
                security_id INTEGER NOT NULL,
                portfolio_id INTEGER NOT NULL,
                purchase_txn_id INTEGER NOT NULL,
                purchase_date TEXT NOT NULL,
                original_shares INTEGER NOT NULL,
                remaining_shares INTEGER NOT NULL,
                gross_amount INTEGER NOT NULL,
                net_amount INTEGER NOT NULL,
                currency TEXT NOT NULL
            );
            "#,
        )
        .unwrap();

        // Insert test data
        // Securities
        conn.execute(
            "INSERT INTO pp_security (id, uuid, name, currency, isin, ticker) VALUES (1, 'sec-1', 'Palantir Technologies Inc.', 'USD', 'US69608A1082', 'PLTR')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_security (id, uuid, name, currency, isin, ticker) VALUES (2, 'sec-2', 'Apple Inc.', 'USD', 'US0378331005', 'AAPL')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_security (id, uuid, name, currency, isin, ticker) VALUES (3, 'sec-3', 'Microsoft Corporation', 'USD', 'US5949181045', 'MSFT')",
            [],
        ).unwrap();

        // Portfolio
        conn.execute(
            "INSERT INTO pp_portfolio (id, uuid, name) VALUES (1, 'pf-1', 'Main Portfolio')",
            [],
        ).unwrap();

        // Account
        conn.execute(
            "INSERT INTO pp_account (id, uuid, name, currency) VALUES (1, 'acc-1', 'Depot Account', 'EUR')",
            [],
        ).unwrap();

        // Transactions - Palantir
        // BUY: 100 shares @ 25 EUR = 2500 EUR (amount in cents: 250000, shares in 10^8: 10000000000)
        conn.execute(
            "INSERT INTO pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (1, 'txn-1', 'portfolio', 1, 1, 'BUY', '2024-03-15', 250000, 'EUR', 10000000000, 'First Palantir purchase')",
            [],
        ).unwrap();
        // BUY: 50 shares @ 27 EUR = 1350 EUR
        conn.execute(
            "INSERT INTO pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (2, 'txn-2', 'portfolio', 1, 1, 'BUY', '2024-05-22', 135000, 'EUR', 5000000000, 'Second Palantir purchase')",
            [],
        ).unwrap();
        // SELL: 75 shares @ 30 EUR = 2250 EUR
        conn.execute(
            "INSERT INTO pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (3, 'txn-3', 'portfolio', 1, 1, 'SELL', '2024-08-10', 225000, 'EUR', 7500000000, 'Partial Palantir sale')",
            [],
        ).unwrap();

        // Transactions - Apple (Dividends)
        conn.execute(
            "INSERT INTO pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (4, 'txn-4', 'account', 1, 2, 'DIVIDENDS', '2024-02-15', 2400, 'USD', NULL, 'Q4 2023 Dividend')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (5, 'txn-5', 'account', 1, 2, 'DIVIDENDS', '2024-05-16', 2500, 'USD', NULL, 'Q1 2024 Dividend')",
            [],
        ).unwrap();

        // Tax units for dividends
        conn.execute(
            "INSERT INTO pp_txn_unit (id, txn_id, unit_type, amount, currency) VALUES (1, 4, 'TAX', 360, 'USD')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_txn_unit (id, txn_id, unit_type, amount, currency) VALUES (2, 5, 'TAX', 375, 'USD')",
            [],
        ).unwrap();

        // Transactions - Microsoft BUY
        conn.execute(
            "INSERT INTO pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (6, 'txn-6', 'portfolio', 1, 3, 'BUY', '2023-11-01', 500000, 'EUR', 1500000000, 'Microsoft purchase')",
            [],
        ).unwrap();

        // FIFO Lots
        conn.execute(
            "INSERT INTO pp_fifo_lot (id, security_id, portfolio_id, purchase_txn_id, purchase_date, original_shares, remaining_shares, gross_amount, net_amount, currency)
             VALUES (1, 1, 1, 1, '2024-03-15', 10000000000, 2500000000, 250000, 245000, 'EUR')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_fifo_lot (id, security_id, portfolio_id, purchase_txn_id, purchase_date, original_shares, remaining_shares, gross_amount, net_amount, currency)
             VALUES (2, 1, 1, 2, '2024-05-22', 5000000000, 5000000000, 135000, 132000, 'EUR')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_fifo_lot (id, security_id, portfolio_id, purchase_txn_id, purchase_date, original_shares, remaining_shares, gross_amount, net_amount, currency)
             VALUES (3, 3, 1, 6, '2023-11-01', 1500000000, 1500000000, 500000, 495000, 'EUR')",
            [],
        ).unwrap();

        conn
    }

    #[test]
    fn test_get_all_templates() {
        let templates = get_all_templates();
        assert_eq!(templates.len(), 13);

        // Check template IDs
        let ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"security_transactions"));
        assert!(ids.contains(&"dividends_by_security"));
        assert!(ids.contains(&"all_dividends"));
        assert!(ids.contains(&"transactions_by_date"));
        assert!(ids.contains(&"security_cost_basis"));
        assert!(ids.contains(&"sold_securities"));
        assert!(ids.contains(&"holding_period_analysis"));
        assert!(ids.contains(&"fifo_lot_details"));
        assert!(ids.contains(&"account_transactions"));
        assert!(ids.contains(&"investment_plans"));
        assert!(ids.contains(&"portfolio_accounts"));
        assert!(ids.contains(&"tax_relevant_sales"));
        assert!(ids.contains(&"account_balance_analysis"));
    }

    #[test]
    fn test_security_transactions_by_name() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "Palantir".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();

        assert_eq!(result.template_id, "security_transactions");
        assert_eq!(result.row_count, 3); // 2 BUYs + 1 SELL
        assert!(result.columns.contains(&"date".to_string()));
        assert!(result.columns.contains(&"txn_type".to_string()));
        assert!(result.columns.contains(&"shares".to_string()));
    }

    #[test]
    fn test_security_transactions_by_isin() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "US69608A1082".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 3);
    }

    #[test]
    fn test_security_transactions_by_ticker() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "PLTR".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 3);
    }

    #[test]
    fn test_security_transactions_filter_buy_only() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "Palantir".to_string());
        params.insert("txn_type".to_string(), "BUY".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 2); // Only 2 BUYs

        // Verify all are BUY
        for row in &result.rows {
            assert_eq!(row.get("txn_type").unwrap(), "BUY");
        }
    }

    #[test]
    fn test_security_transactions_filter_sell_only() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "Palantir".to_string());
        params.insert("txn_type".to_string(), "SELL".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 1); // Only 1 SELL
    }

    #[test]
    fn test_security_transactions_not_found() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "NonExistent".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 0);
    }

    #[test]
    fn test_dividends_by_security() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "Apple".to_string());

        let request = QueryRequest {
            template_id: "dividends_by_security".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 2); // 2 dividends

        // Check columns
        assert!(result.columns.contains(&"gross_amount".to_string()));
        assert!(result.columns.contains(&"taxes".to_string()));
    }

    #[test]
    fn test_all_dividends() {
        let conn = create_test_db();
        let params = HashMap::new();

        let request = QueryRequest {
            template_id: "all_dividends".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 1); // Only Apple has dividends

        // Check grouping
        let row = &result.rows[0];
        assert_eq!(row.get("security_name").unwrap(), "Apple Inc.");
        assert_eq!(row.get("dividend_count").unwrap(), &serde_json::json!(2));
    }

    #[test]
    fn test_all_dividends_filter_by_year() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("year".to_string(), "2024".to_string());

        let request = QueryRequest {
            template_id: "all_dividends".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 1);
    }

    #[test]
    fn test_transactions_by_date() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("from_date".to_string(), "2024-01-01".to_string());
        params.insert("to_date".to_string(), "2024-06-30".to_string());

        let request = QueryRequest {
            template_id: "transactions_by_date".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        // Should include: Palantir BUY (Mar), Apple Dividend (Feb), Palantir BUY (May), Apple Dividend (May)
        assert!(result.row_count >= 4);
    }

    #[test]
    fn test_transactions_by_date_with_type_filter() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("from_date".to_string(), "2024-01-01".to_string());
        params.insert("to_date".to_string(), "2024-12-31".to_string());
        params.insert("txn_type".to_string(), "SELL".to_string());

        let request = QueryRequest {
            template_id: "transactions_by_date".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 1); // Only 1 SELL in 2024
    }

    #[test]
    fn test_security_cost_basis() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "Palantir".to_string());

        let request = QueryRequest {
            template_id: "security_cost_basis".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 2); // 2 FIFO lots with remaining shares

        // Check columns
        assert!(result.columns.contains(&"cost_per_share".to_string()));
        assert!(result.columns.contains(&"remaining_shares".to_string()));
    }

    #[test]
    fn test_sold_securities() {
        let conn = create_test_db();
        let params = HashMap::new();

        let request = QueryRequest {
            template_id: "sold_securities".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();
        assert_eq!(result.row_count, 1); // Only Palantir was sold

        let row = &result.rows[0];
        assert_eq!(row.get("security_name").unwrap(), "Palantir Technologies Inc.");
    }

    #[test]
    fn test_unknown_template() {
        let conn = create_test_db();
        let params = HashMap::new();

        let request = QueryRequest {
            template_id: "unknown_template".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unbekanntes Template"));
    }

    #[test]
    fn test_missing_required_parameter() {
        let conn = create_test_db();
        let params = HashMap::new(); // Missing "security" parameter

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("security"));
    }

    #[test]
    fn test_shares_scaling() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "Palantir".to_string());
        params.insert("txn_type".to_string(), "BUY".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();

        // First BUY should be 100 shares
        let first_buy = result.rows.iter().find(|r| r.get("date").unwrap() == "2024-03-15").unwrap();
        let shares = first_buy.get("shares").unwrap().as_f64().unwrap();
        assert!((shares - 100.0).abs() < 0.01);

        // Second BUY should be 50 shares
        let second_buy = result.rows.iter().find(|r| r.get("date").unwrap() == "2024-05-22").unwrap();
        let shares = second_buy.get("shares").unwrap().as_f64().unwrap();
        assert!((shares - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_amount_scaling() {
        let conn = create_test_db();
        let mut params = HashMap::new();
        params.insert("security".to_string(), "Palantir".to_string());
        params.insert("txn_type".to_string(), "BUY".to_string());

        let request = QueryRequest {
            template_id: "security_transactions".to_string(),
            parameters: params,
        };

        let result = execute_template(&conn, &request).unwrap();

        // First BUY should be 2500 EUR
        let first_buy = result.rows.iter().find(|r| r.get("date").unwrap() == "2024-03-15").unwrap();
        let amount = first_buy.get("amount").unwrap().as_f64().unwrap();
        assert!((amount - 2500.0).abs() < 0.01);
    }

    #[test]
    fn test_get_templates_for_prompt() {
        let prompt = get_templates_for_prompt();

        // Should contain all template descriptions
        assert!(prompt.contains("security_transactions"));
        assert!(prompt.contains("dividends_by_security"));
        assert!(prompt.contains("all_dividends"));
        assert!(prompt.contains("transactions_by_date"));
        assert!(prompt.contains("security_cost_basis"));
        assert!(prompt.contains("sold_securities"));

        // Should contain JSON format instruction
        assert!(prompt.contains(r#"{"query":"#));
    }
}
