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

/// Format a date from YYYY-MM-DD to DD.MM.YYYY
fn format_date_german(date: &str) -> String {
    if date.len() == 10 && date.contains('-') {
        let parts: Vec<&str> = date.split('-').collect();
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

/// Format query results as a simple list (no table)
fn format_as_markdown(template_id: &str, _columns: &[String], rows: &[HashMap<String, serde_json::Value>]) -> String {
    if rows.is_empty() {
        return "Keine Ergebnisse gefunden.".to_string();
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

WICHTIG: Wenn du eine Abfrage benötigst, antworte NUR mit diesem JSON-Format:
```json
{"query": "template_id", "params": {"key": "value"}}
```

Das System führt die Abfrage aus und sendet dir die Ergebnisse. Formuliere dann eine hilfreiche Antwort.

Nutze Abfragen für:
- Konkrete Fragen zu Transaktionen ("Wann habe ich X gekauft/verkauft?")
- Dividenden-Fragen ("Welche Dividenden von X?")
- Historische Daten ("Was war mein Einstandskurs?")
- Zeitraum-Fragen ("Transaktionen im Jahr 2024")

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
        assert_eq!(templates.len(), 6);

        // Check template IDs
        let ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"security_transactions"));
        assert!(ids.contains(&"dividends_by_security"));
        assert!(ids.contains(&"all_dividends"));
        assert!(ids.contains(&"transactions_by_date"));
        assert!(ids.contains(&"security_cost_basis"));
        assert!(ids.contains(&"sold_securities"));
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
