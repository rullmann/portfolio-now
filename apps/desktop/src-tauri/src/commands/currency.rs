//! Currency conversion commands for Tauri

use crate::currency;
use crate::db;
use chrono::NaiveDate;
use serde::Serialize;
use tauri::command;

/// Exchange rate result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeRateResult {
    pub base: String,
    pub target: String,
    pub rate: f64,
    pub date: String,
}

/// Conversion result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResult {
    pub original_amount: f64,
    pub original_currency: String,
    pub converted_amount: f64,
    pub target_currency: String,
    pub rate: f64,
    pub date: String,
}

/// Get exchange rate for a currency pair
#[command]
pub fn get_exchange_rate(
    base: String,
    target: String,
    date: Option<String>,
) -> Result<ExchangeRateResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let rate_date = date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let rate = currency::get_exchange_rate(conn, &base, &target, rate_date)
        .map_err(|e| e.to_string())?;

    Ok(ExchangeRateResult {
        base,
        target,
        rate,
        date: rate_date.to_string(),
    })
}

/// Convert an amount between currencies
#[command]
pub fn convert_currency(
    amount: f64,
    from: String,
    to: String,
    date: Option<String>,
) -> Result<ConversionResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let conv_date = date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let rate = currency::get_exchange_rate(conn, &from, &to, conv_date)
        .map_err(|e| e.to_string())?;

    let converted = amount * rate;

    Ok(ConversionResult {
        original_amount: amount,
        original_currency: from,
        converted_amount: converted,
        target_currency: to,
        rate,
        date: conv_date.to_string(),
    })
}

/// Get the latest exchange rate for a currency pair
#[command]
pub fn get_latest_exchange_rate(base: String, target: String) -> Result<ExchangeRateResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (date, rate) = currency::get_latest_rate(conn, &base, &target)
        .map_err(|e| e.to_string())?;

    Ok(ExchangeRateResult {
        base,
        target,
        rate,
        date: date.to_string(),
    })
}

/// Get the configured base currency
#[command]
pub fn get_base_currency() -> Result<String, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    currency::get_base_currency(conn).map_err(|e| e.to_string())
}

/// Get all holdings converted to base currency
#[command]
pub fn get_holdings_in_base_currency() -> Result<f64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());
    let today = chrono::Utc::now().date_naive();

    // Get holdings with their currencies
    let holdings_sql = r#"
        SELECT
            t.security_id,
            s.currency,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        WHERE t.owner_type = 'portfolio'
          AND t.shares IS NOT NULL
        GROUP BY t.security_id
        HAVING net_shares > 0
    "#;

    let mut total_value = 0.0;

    {
        let mut stmt = conn.prepare(holdings_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows.flatten() {
            let (security_id, security_currency, share_count) = row;

            // Get latest price
            let price_sql = "SELECT value FROM pp_latest_price WHERE security_id = ?";
            let price: Option<i64> = conn
                .query_row(price_sql, [security_id], |row| row.get(0))
                .ok();

            if let Some(p) = price {
                let shares_f = share_count as f64 / 100_000_000.0;
                let price_f = p as f64 / 100_000_000.0;
                let value_in_security_currency = shares_f * price_f;

                // Convert to base currency
                let value_in_base = if security_currency == base_currency {
                    value_in_security_currency
                } else {
                    currency::convert(conn, value_in_security_currency, &security_currency, &base_currency, today)
                        .unwrap_or(value_in_security_currency)
                };

                total_value += value_in_base;
            }
        }
    }

    Ok(total_value)
}
