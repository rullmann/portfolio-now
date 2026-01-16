//! DivvyDiary export functionality
//!
//! Uploads portfolio holdings and transaction history to DivvyDiary
//! API endpoint: https://api.divvydiary.com/portfolios/{id}/import
//!
//! Based on Portfolio Performance's DivvyDiaryUploader.java

use crate::db;
use crate::fifo;
use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::command;

const DIVVYDIARY_API_BASE: &str = "https://api.divvydiary.com";
const USER_AGENT: &str = "PortfolioNow/0.1.0";

/// DivvyDiary portfolio from API (internal - id is numeric)
#[derive(Debug, Clone, Deserialize)]
struct DivvyDiaryPortfolioRaw {
    id: i64,
    name: String,
}

/// DivvyDiary portfolio metadata (for frontend - id as string)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivvyDiaryPortfolio {
    pub id: String,
    pub name: String,
}

impl From<DivvyDiaryPortfolioRaw> for DivvyDiaryPortfolio {
    fn from(raw: DivvyDiaryPortfolioRaw) -> Self {
        Self {
            id: raw.id.to_string(),
            name: raw.name,
        }
    }
}

/// Security holding for DivvyDiary
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DivvyDiarySecurity {
    isin: String,
    quantity: f64,
    buyin: DivvyDiaryBuyin,
}

/// Buy-in price for a security
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DivvyDiaryBuyin {
    price: f64,
    currency: String,
}

/// Activity (transaction) for DivvyDiary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DivvyDiaryActivity {
    #[serde(rename = "type")]
    activity_type: String,
    isin: String,
    datetime: String,
    quantity: f64,
    amount: f64,
    fees: f64,
    taxes: f64,
    currency: String,
    broker: String,
    broker_reference: String,
}

/// Complete upload payload
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DivvyDiaryPayload {
    securities: Vec<DivvyDiarySecurity>,
    activities: Vec<DivvyDiaryActivity>,
}

/// Session response from DivvyDiary
#[derive(Debug, Clone, Deserialize)]
struct SessionResponse {
    portfolios: Vec<DivvyDiaryPortfolioRaw>,
}

/// Export result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DivvyDiaryExportResult {
    pub success: bool,
    pub message: String,
    pub securities_count: i32,
    pub activities_count: i32,
}

/// Get available DivvyDiary portfolios for the user
#[command]
pub async fn get_divvydiary_portfolios(
    api_key: String,
) -> Result<Vec<DivvyDiaryPortfolio>, String> {
    if api_key.is_empty() {
        return Err("DivvyDiary API-Key fehlt".to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/session", DIVVYDIARY_API_BASE))
        .header("X-API-Key", &api_key)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Verbindungsfehler: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "DivvyDiary API Fehler: {} - Bitte API-Key überprüfen",
            response.status()
        ));
    }

    // Get raw response body for debugging
    let body = response.text().await.map_err(|e| format!("Fehler beim Lesen der Antwort: {}", e))?;
    log::info!("DivvyDiary API response: {}", &body[..body.len().min(500)]);

    let session: SessionResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Ungültige API-Antwort: {} - Body: {}", e, &body[..body.len().min(200)]))?;

    Ok(session.portfolios.into_iter().map(|p| p.into()).collect())
}

/// Upload portfolio to DivvyDiary
/// portfolio_id: None = alle Portfolios (Gesamtdepot), Some(id) = einzelnes Portfolio
#[command]
pub async fn upload_to_divvydiary(
    api_key: String,
    divvydiary_portfolio_id: String,
    portfolio_id: Option<i64>,
    include_transactions: bool,
) -> Result<DivvyDiaryExportResult, String> {
    if api_key.is_empty() {
        return Err("DivvyDiary API-Key fehlt".to_string());
    }

    // Collect data from database
    let payload = build_payload(portfolio_id, include_transactions)?;

    let securities_count = payload.securities.len() as i32;
    let activities_count = payload.activities.len() as i32;

    if securities_count == 0 {
        return Err("Keine Wertpapiere mit ISIN zum Exportieren gefunden".to_string());
    }

    // Upload to DivvyDiary
    let client = reqwest::Client::new();
    let url = format!(
        "{}/portfolios/{}/import?splitAdjusted=true",
        DIVVYDIARY_API_BASE, divvydiary_portfolio_id
    );

    let response = client
        .post(&url)
        .header("X-API-Key", &api_key)
        .header("Content-Type", "application/json")
        .header("User-Agent", USER_AGENT)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Verbindungsfehler: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "DivvyDiary Upload fehlgeschlagen: {} - {}",
            status, body
        ));
    }

    Ok(DivvyDiaryExportResult {
        success: true,
        message: format!(
            "Export erfolgreich: {} Wertpapiere, {} Transaktionen",
            securities_count, activities_count
        ),
        securities_count,
        activities_count,
    })
}

/// Build the payload for DivvyDiary from local database
/// portfolio_id: None = alle Portfolios, Some(id) = einzelnes Portfolio
fn build_payload(
    portfolio_id: Option<i64>,
    include_transactions: bool,
) -> Result<DivvyDiaryPayload, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Datenbank nicht initialisiert".to_string())?;

    // Build portfolio filter for SQL
    let portfolio_filter = match portfolio_id {
        Some(id) => format!("AND t.owner_id = {}", id),
        None => String::new(), // Alle Portfolios
    };

    // Get current holdings with ISIN (aggregate by ISIN across portfolios)
    let holdings_sql = format!(
        r#"
        SELECT s.isin, MIN(s.currency) as currency,
               SUM(CASE
                   WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                   WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                   ELSE 0
               END) as total_shares
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        WHERE t.owner_type = 'portfolio'
          {}
          AND s.isin IS NOT NULL
          AND s.isin != ''
        GROUP BY s.isin
        HAVING total_shares > 0
        "#,
        portfolio_filter
    );

    let mut stmt = conn.prepare(&holdings_sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

    let mut securities: Vec<DivvyDiarySecurity> = Vec::new();

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let isin: String = row.get(0).map_err(|e| e.to_string())?;
        let currency: String = row.get(1).map_err(|e| e.to_string())?;
        let total_shares: i64 = row.get(2).map_err(|e| e.to_string())?;
        let quantity = total_shares as f64 / fifo::SHARES_SCALE as f64;

        // Get average cost basis for this security (converted to security's currency)
        let cost_basis = get_average_cost_basis(conn, &isin, &currency, portfolio_id)?;

        securities.push(DivvyDiarySecurity {
            isin,
            quantity,
            buyin: DivvyDiaryBuyin {
                price: cost_basis,
                currency,
            },
        });
    }

    // Get activities (transactions) if requested
    let mut activities: Vec<DivvyDiaryActivity> = Vec::new();

    if include_transactions {
        // Build a map of ISIN -> target currency (the security's currency)
        let mut isin_currency_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for sec in &securities {
            isin_currency_map.insert(sec.isin.clone(), sec.buyin.currency.clone());
        }

        let activities_sql = format!(
            r#"
            SELECT t.uuid, t.txn_type, t.date, t.amount, t.currency, t.shares,
                   s.isin, s.currency as security_currency,
                   COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'FEE'), 0) as fees,
                   COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0) as taxes
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE t.owner_type = 'portfolio'
              {}
              AND t.txn_type IN ('BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND')
              AND s.isin IS NOT NULL
              AND s.isin != ''
            ORDER BY t.date ASC
            "#,
            portfolio_filter
        );

        let mut stmt = conn.prepare(&activities_sql).map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let uuid: String = row.get(0).map_err(|e| e.to_string())?;
            let txn_type: String = row.get(1).map_err(|e| e.to_string())?;
            let date: String = row.get(2).map_err(|e| e.to_string())?;
            let amount: i64 = row.get(3).map_err(|e| e.to_string())?;
            let txn_currency: String = row.get(4).map_err(|e| e.to_string())?;
            let shares: i64 = row.get(5).map_err(|e| e.to_string())?;
            let isin: String = row.get(6).map_err(|e| e.to_string())?;
            let security_currency: String = row.get(7).map_err(|e| e.to_string())?;
            let fees: i64 = row.get(8).map_err(|e| e.to_string())?;
            let taxes: i64 = row.get(9).map_err(|e| e.to_string())?;

            // Map transaction type to DivvyDiary format
            let activity_type = match txn_type.as_str() {
                "BUY" | "DELIVERY_INBOUND" => "BUY",
                "SELL" | "DELIVERY_OUTBOUND" => "SELL",
                _ => continue,
            };

            // Convert date to ISO 8601 format
            let datetime = format_datetime(&date);

            // Get target currency (security's currency) - all transactions for same ISIN must use same currency
            let target_currency = isin_currency_map.get(&isin)
                .cloned()
                .unwrap_or_else(|| security_currency.clone());

            // Convert amounts to target currency if needed
            let (final_amount, final_fees, final_taxes) = if txn_currency == target_currency {
                (
                    amount.abs() as f64 / fifo::AMOUNT_SCALE as f64,
                    fees as f64 / fifo::AMOUNT_SCALE as f64,
                    taxes as f64 / fifo::AMOUNT_SCALE as f64,
                )
            } else {
                // Parse date for currency conversion
                let conv_date = NaiveDate::parse_from_str(&date[..10], "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Utc::now().date_naive());

                let amount_f64 = amount.abs() as f64 / fifo::AMOUNT_SCALE as f64;
                let fees_f64 = fees as f64 / fifo::AMOUNT_SCALE as f64;
                let taxes_f64 = taxes as f64 / fifo::AMOUNT_SCALE as f64;

                (
                    crate::currency::convert(conn, amount_f64, &txn_currency, &target_currency, conv_date)
                        .unwrap_or(amount_f64),
                    crate::currency::convert(conn, fees_f64, &txn_currency, &target_currency, conv_date)
                        .unwrap_or(fees_f64),
                    crate::currency::convert(conn, taxes_f64, &txn_currency, &target_currency, conv_date)
                        .unwrap_or(taxes_f64),
                )
            };

            activities.push(DivvyDiaryActivity {
                activity_type: activity_type.to_string(),
                isin,
                datetime,
                quantity: shares as f64 / fifo::SHARES_SCALE as f64,
                amount: final_amount,
                fees: final_fees,
                taxes: final_taxes,
                currency: target_currency,
                broker: "portfolioperformance".to_string(),
                broker_reference: uuid,
            });
        }
    }

    Ok(DivvyDiaryPayload {
        securities,
        activities,
    })
}

/// Calculate average cost basis per share for a security (converted to target currency)
/// portfolio_id: None = alle Portfolios, Some(id) = einzelnes Portfolio
fn get_average_cost_basis(
    conn: &rusqlite::Connection,
    isin: &str,
    target_currency: &str,
    portfolio_id: Option<i64>,
) -> Result<f64, String> {
    // Build portfolio filter for FIFO lots
    let portfolio_filter = match portfolio_id {
        Some(id) => format!("AND l.portfolio_id = {}", id),
        None => String::new(), // Alle Portfolios
    };

    // Get remaining cost basis from FIFO lots (with currency for conversion)
    let sql = format!(
        r#"
        SELECT l.gross_amount * l.remaining_shares / l.original_shares as lot_cost,
               l.remaining_shares,
               l.currency,
               l.purchase_date
        FROM pp_fifo_lot l
        JOIN pp_security s ON s.id = l.security_id
        WHERE s.isin = ?
          {}
          AND l.remaining_shares > 0
        "#,
        portfolio_filter
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![isin]).map_err(|e| e.to_string())?;

    let mut total_cost_converted = 0.0;
    let mut total_shares: i64 = 0;

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let lot_cost: i64 = row.get(0).map_err(|e| e.to_string())?;
        let shares: i64 = row.get(1).map_err(|e| e.to_string())?;
        let currency: String = row.get(2).map_err(|e| e.to_string())?;
        let purchase_date: String = row.get(3).map_err(|e| e.to_string())?;

        let cost_amount = lot_cost as f64 / fifo::AMOUNT_SCALE as f64;

        // Convert to target currency if needed
        let converted_cost = if currency == target_currency {
            cost_amount
        } else {
            // Parse date for currency conversion
            let date = NaiveDate::parse_from_str(&purchase_date, "%Y-%m-%d")
                .unwrap_or_else(|_| chrono::Utc::now().date_naive());
            crate::currency::convert(conn, cost_amount, &currency, target_currency, date)
                .unwrap_or(cost_amount) // Fallback to original if conversion fails
        };

        total_cost_converted += converted_cost;
        total_shares += shares;
    }

    if total_shares > 0 {
        let cost_per_share = total_cost_converted / (total_shares as f64 / fifo::SHARES_SCALE as f64);
        Ok(cost_per_share)
    } else {
        Ok(0.0)
    }
}

/// Format date string to ISO 8601 datetime
fn format_datetime(date: &str) -> String {
    // Input format: "YYYY-MM-DD" or "YYYY-MM-DD HH:MM:SS"
    if date.len() == 10 {
        // Just date, add time
        format!("{}T12:00:00.000Z", date)
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S") {
        dt.format("%Y-%m-%dT%H:%M:%S.000Z").to_string()
    } else {
        format!("{}T12:00:00.000Z", &date[..10])
    }
}
