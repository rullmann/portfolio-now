//! Query Templates for Portfolio Chatbot
//!
//! Provides safe, predefined SQL query templates that the AI can use
//! to answer questions about transactions, dividends, and holdings.

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
        _ => Err(format!("Unbekanntes Template: {}", request.template_id)),
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
    if let Some(tt) = txn_type {
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
// Helper
// ============================================================================

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
