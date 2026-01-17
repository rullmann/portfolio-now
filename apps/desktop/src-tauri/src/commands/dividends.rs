//! Dividend calendar and forecast commands
//!
//! Provides dividend calendar view, ex-dividend tracking, and future dividend projections
//! based on historical payment patterns.

use crate::db;
use crate::pp::common::shares;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::command;

// ============================================================================
// Types
// ============================================================================

/// A single dividend event for the calendar
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDividend {
    pub date: String,
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub amount: f64,
    pub currency: String,
    pub is_estimated: bool,
}

/// Calendar data for a month
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthCalendarData {
    pub year: i32,
    pub month: u32,
    pub total_amount: f64,
    pub currency: String,
    pub dividends: Vec<CalendarDividend>,
}

/// Dividend payment pattern for a security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendPattern {
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    /// Payment frequency: MONTHLY, QUARTERLY, SEMI_ANNUAL, ANNUAL, IRREGULAR
    pub frequency: String,
    /// Typical payment months (1-12)
    pub payment_months: Vec<u32>,
    /// Average dividend per share
    pub avg_per_share: f64,
    /// Last 4 dividends per share for trend
    pub recent_amounts: Vec<f64>,
    /// Growth rate (year over year)
    pub growth_rate: Option<f64>,
    pub currency: String,
}

/// Annual dividend forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendForecast {
    pub year: i32,
    pub currency: String,
    /// Total estimated dividends for the year
    pub total_estimated: f64,
    /// Total already received
    pub total_received: f64,
    /// Total remaining expected
    pub total_remaining: f64,
    /// Monthly breakdown
    pub by_month: Vec<MonthForecast>,
    /// Per security forecast
    pub by_security: Vec<SecurityForecast>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthForecast {
    pub month: u32,
    pub month_name: String,
    pub estimated: f64,
    pub received: f64,
    pub is_past: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityForecast {
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub pattern: DividendPattern,
    /// Current shares held
    pub shares_held: f64,
    /// Estimated annual dividends
    pub estimated_annual: f64,
    /// Expected payments this year
    pub expected_payments: Vec<ExpectedPayment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedPayment {
    pub month: u32,
    pub estimated_amount: f64,
    pub is_received: bool,
    pub actual_amount: Option<f64>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Detect dividend payment frequency based on historical data
fn detect_frequency(payment_months: &[u32], payment_count: usize) -> String {
    if payment_count == 0 {
        return "NONE".to_string();
    }

    let unique_months: std::collections::HashSet<_> = payment_months.iter().collect();

    // Check for monthly (11-12 unique months over multiple years)
    if unique_months.len() >= 11 {
        return "MONTHLY".to_string();
    }

    // Check for quarterly (3-4 unique months)
    if unique_months.len() >= 3 && unique_months.len() <= 5 {
        // Verify it's roughly quarterly spacing
        let mut months: Vec<_> = unique_months.iter().copied().copied().collect();
        months.sort();
        if months.len() >= 4 {
            return "QUARTERLY".to_string();
        }
    }

    // Check for semi-annual (2 unique months)
    if unique_months.len() == 2 {
        return "SEMI_ANNUAL".to_string();
    }

    // Check for annual (1 unique month)
    if unique_months.len() == 1 {
        return "ANNUAL".to_string();
    }

    "IRREGULAR".to_string()
}

/// Get month name in German
fn get_month_name(month: u32) -> String {
    match month {
        1 => "Januar",
        2 => "Februar",
        3 => "März",
        4 => "April",
        5 => "Mai",
        6 => "Juni",
        7 => "Juli",
        8 => "August",
        9 => "September",
        10 => "Oktober",
        11 => "November",
        12 => "Dezember",
        _ => "Unbekannt",
    }
    .to_string()
}

// ============================================================================
// Commands
// ============================================================================

/// Get dividend calendar for a specific month
#[command]
pub fn get_dividend_calendar(year: i32, month: Option<u32>) -> Result<Vec<MonthCalendarData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let base_currency: String = conn
        .query_row(
            "SELECT base_currency FROM pp_import ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "EUR".to_string());

    // Determine date range
    let (start_date, end_date) = match month {
        Some(m) => {
            let start = format!("{}-{:02}-01", year, m);
            let days_in_month = if m == 12 {
                31
            } else {
                NaiveDate::from_ymd_opt(year, m + 1, 1)
                    .and_then(|d| d.pred_opt())
                    .map(|d| d.day())
                    .unwrap_or(28)
            };
            let end = format!("{}-{:02}-{:02}", year, m, days_in_month);
            (start, end)
        }
        None => (format!("{}-01-01", year), format!("{}-12-31", year)),
    };

    // Get actual dividends
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                t.date,
                s.id,
                s.name,
                s.isin,
                (t.amount - COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0)) / 100.0 as net_amount,
                t.currency
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
              AND t.date >= ?1 AND t.date <= ?2
            ORDER BY t.date
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([&start_date, &end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    // Group by month
    let mut month_data: HashMap<u32, MonthCalendarData> = HashMap::new();

    for row in rows.flatten() {
        let (date, security_id, security_name, isin, amount, currency) = row;
        let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok();
        let m = parsed_date.map(|d| d.month()).unwrap_or(1);

        let entry = month_data.entry(m).or_insert_with(|| MonthCalendarData {
            year,
            month: m,
            total_amount: 0.0,
            currency: base_currency.clone(),
            dividends: Vec::new(),
        });

        entry.total_amount += amount;
        entry.dividends.push(CalendarDividend {
            date,
            security_id,
            security_name,
            security_isin: isin,
            amount,
            currency,
            is_estimated: false,
        });
    }

    // Sort by month
    let mut result: Vec<MonthCalendarData> = month_data.into_values().collect();
    result.sort_by_key(|m| m.month);

    Ok(result)
}

/// Analyze dividend patterns for securities
#[command]
pub fn get_dividend_patterns() -> Result<Vec<DividendPattern>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get securities with dividend history (last 3 years)
    let three_years_ago = chrono::Utc::now()
        .date_naive()
        .checked_sub_months(chrono::Months::new(36))
        .map(|d| d.to_string())
        .unwrap_or_else(|| "2020-01-01".to_string());

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                s.id,
                s.name,
                s.isin,
                t.date,
                t.amount / 100.0 as amount,
                t.currency,
                t.shares
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
              AND t.date >= ?1
            ORDER BY s.id, t.date DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([&three_years_ago], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<i64>>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    // Group by security
    let mut security_dividends: HashMap<i64, Vec<(String, String, Option<String>, String, f64, String, Option<i64>)>> =
        HashMap::new();

    for row in rows.flatten() {
        let (sec_id, name, isin, date, amount, currency, shares_raw) = row;
        security_dividends
            .entry(sec_id)
            .or_default()
            .push((date, name, isin, currency.clone(), amount, currency, shares_raw));
    }

    let mut patterns: Vec<DividendPattern> = Vec::new();

    for (sec_id, dividends) in security_dividends {
        if dividends.is_empty() {
            continue;
        }

        let (_, name, isin, currency, _, _, _) = &dividends[0];

        // Extract payment months
        let payment_months: Vec<u32> = dividends
            .iter()
            .filter_map(|(date, ..)| {
                NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .ok()
                    .map(|d| d.month())
            })
            .collect();

        // Calculate per-share amounts
        let per_share_amounts: Vec<f64> = dividends
            .iter()
            .filter_map(|(_, _, _, _, amount, _, shares_raw)| {
                shares_raw
                    .map(|s| shares::to_decimal(s))
                    .filter(|&s| s > 0.0)
                    .map(|s| amount / s)
            })
            .collect();

        let avg_per_share = if !per_share_amounts.is_empty() {
            per_share_amounts.iter().sum::<f64>() / per_share_amounts.len() as f64
        } else {
            0.0
        };

        // Recent 4 amounts for trend
        let recent_amounts: Vec<f64> = per_share_amounts.iter().take(4).copied().collect();

        // Calculate YoY growth if we have enough data
        let growth_rate = if per_share_amounts.len() >= 8 {
            let recent_avg: f64 = per_share_amounts.iter().take(4).sum::<f64>() / 4.0;
            let old_avg: f64 = per_share_amounts.iter().skip(4).take(4).sum::<f64>() / 4.0;
            if old_avg > 0.0 {
                Some(((recent_avg - old_avg) / old_avg) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        // Get unique payment months
        let unique_months: Vec<u32> = {
            let mut months: std::collections::HashSet<u32> = payment_months.iter().copied().collect();
            let mut sorted: Vec<u32> = months.drain().collect();
            sorted.sort();
            sorted
        };

        let frequency = detect_frequency(&payment_months, dividends.len());

        patterns.push(DividendPattern {
            security_id: sec_id,
            security_name: name.clone(),
            security_isin: isin.clone(),
            frequency,
            payment_months: unique_months,
            avg_per_share,
            recent_amounts,
            growth_rate,
            currency: currency.clone(),
        });
    }

    // Sort by avg_per_share descending
    patterns.sort_by(|a, b| {
        b.avg_per_share
            .partial_cmp(&a.avg_per_share)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(patterns)
}

/// Generate annual dividend forecast
#[command]
pub fn estimate_annual_dividends(year: Option<i32>) -> Result<DividendForecast, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let forecast_year = year.unwrap_or_else(|| chrono::Utc::now().date_naive().year());
    let current_month = chrono::Utc::now().date_naive().month();

    let base_currency: String = conn
        .query_row(
            "SELECT base_currency FROM pp_import ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "EUR".to_string());

    // Get patterns
    let patterns = get_dividend_patterns()?;

    // Get current holdings
    let mut holdings_stmt = conn
        .prepare(
            r#"
            SELECT security_id, SUM(CASE
                WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                ELSE 0
            END) as total_shares
            FROM pp_txn
            WHERE owner_type = 'portfolio'
            GROUP BY security_id
            HAVING total_shares > 0
            "#,
        )
        .map_err(|e| e.to_string())?;

    let holdings: HashMap<i64, f64> = holdings_stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, shares::to_decimal(row.get::<_, i64>(1)?))))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get already received dividends this year
    let year_start = format!("{}-01-01", forecast_year);
    let year_end = format!("{}-12-31", forecast_year);

    let mut received_stmt = conn
        .prepare(
            r#"
            SELECT security_id, strftime('%m', date) as month, SUM(amount) / 100.0 as total
            FROM pp_txn
            WHERE txn_type = 'DIVIDENDS'
              AND date >= ?1 AND date <= ?2
            GROUP BY security_id, month
            "#,
        )
        .map_err(|e| e.to_string())?;

    let received: Vec<(i64, u32, f64)> = received_stmt
        .query_map([&year_start, &year_end], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?.parse::<u32>().unwrap_or(1),
                row.get::<_, f64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Build received map: security_id -> month -> amount
    let mut received_map: HashMap<i64, HashMap<u32, f64>> = HashMap::new();
    for (sec_id, month, amount) in &received {
        received_map
            .entry(*sec_id)
            .or_default()
            .insert(*month, *amount);
    }

    // Build forecasts
    let mut total_estimated = 0.0;
    let mut total_received = 0.0;
    let mut month_estimates: HashMap<u32, (f64, f64)> = HashMap::new(); // (estimated, received)
    let mut security_forecasts: Vec<SecurityForecast> = Vec::new();

    for pattern in patterns {
        let shares_held = holdings.get(&pattern.security_id).copied().unwrap_or(0.0);
        if shares_held <= 0.0 {
            continue;
        }

        let sec_received = received_map.get(&pattern.security_id);

        // Estimate payments per month based on pattern
        let mut expected_payments: Vec<ExpectedPayment> = Vec::new();
        let mut sec_estimated_annual = 0.0;

        for &month in &pattern.payment_months {
            let estimated_amount = pattern.avg_per_share * shares_held;
            let actual = sec_received.and_then(|m| m.get(&month)).copied();
            let is_received = actual.is_some();

            expected_payments.push(ExpectedPayment {
                month,
                estimated_amount,
                is_received,
                actual_amount: actual,
            });

            if is_received {
                total_received += actual.unwrap_or(0.0);
                let entry = month_estimates.entry(month).or_insert((0.0, 0.0));
                entry.1 += actual.unwrap_or(0.0);
            } else {
                sec_estimated_annual += estimated_amount;
                let entry = month_estimates.entry(month).or_insert((0.0, 0.0));
                entry.0 += estimated_amount;
            }
        }

        total_estimated += sec_estimated_annual;

        security_forecasts.push(SecurityForecast {
            security_id: pattern.security_id,
            security_name: pattern.security_name.clone(),
            security_isin: pattern.security_isin.clone(),
            pattern,
            shares_held,
            estimated_annual: sec_estimated_annual,
            expected_payments,
        });
    }

    // Build monthly breakdown
    let by_month: Vec<MonthForecast> = (1..=12)
        .map(|m| {
            let (est, rec) = month_estimates.get(&m).copied().unwrap_or((0.0, 0.0));
            MonthForecast {
                month: m,
                month_name: get_month_name(m),
                estimated: est,
                received: rec,
                is_past: m < current_month,
            }
        })
        .collect();

    // Sort security forecasts by estimated annual descending
    security_forecasts.sort_by(|a, b| {
        b.estimated_annual
            .partial_cmp(&a.estimated_annual)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(DividendForecast {
        year: forecast_year,
        currency: base_currency,
        total_estimated,
        total_received,
        total_remaining: total_estimated,
        by_month,
        by_security: security_forecasts,
    })
}

/// Get dividend yield for portfolio
#[command]
pub fn get_portfolio_dividend_yield() -> Result<f64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get total dividends in last 12 months
    let end_date = chrono::Utc::now().date_naive();
    let start_date = end_date
        .checked_sub_months(chrono::Months::new(12))
        .unwrap_or(end_date);

    let total_dividends: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(amount), 0) / 100.0
            FROM pp_txn
            WHERE txn_type = 'DIVIDENDS'
              AND date >= ? AND date <= ?
            "#,
            rusqlite::params![start_date.to_string(), end_date.to_string()],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Get total portfolio value
    let total_value: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(
                (SELECT SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) FROM pp_txn t WHERE t.security_id = s.id AND t.owner_type = 'portfolio') / 100000000.0
                * COALESCE((SELECT value / 100000000.0 FROM pp_latest_price WHERE security_id = s.id), 0)
            ), 0)
            FROM pp_security s
            "#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    if total_value <= 0.0 {
        return Ok(0.0);
    }

    Ok((total_dividends / total_value) * 100.0)
}

// ============================================================================
// Ex-Dividend Types
// ============================================================================

/// Ex-dividend event for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExDividend {
    pub id: i64,
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub pay_date: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub frequency: Option<String>,
    pub source: Option<String>,
    pub is_confirmed: bool,
    pub note: Option<String>,
    pub created_at: String,
}

/// Request to create/update an ex-dividend entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExDividendRequest {
    pub security_id: i64,
    pub ex_date: String,
    pub record_date: Option<String>,
    pub pay_date: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub frequency: Option<String>,
    pub source: Option<String>,
    pub is_confirmed: Option<bool>,
    pub note: Option<String>,
}

/// Calendar view combining ex-div dates and payment dates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendCalendarEvent {
    pub date: String,
    pub event_type: String, // "ex_dividend", "record_date", "payment"
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub is_confirmed: bool,
    pub related_ex_date: Option<String>,
}

/// Combined calendar data with both ex-div and payment dates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedMonthCalendarData {
    pub year: i32,
    pub month: u32,
    pub events: Vec<DividendCalendarEvent>,
    pub total_payments: f64,
    pub upcoming_ex_divs: i32,
    pub currency: String,
}

// ============================================================================
// Ex-Dividend Commands
// ============================================================================

/// Get all ex-dividend entries for a date range
#[command]
pub fn get_ex_dividends(
    start_date: Option<String>,
    end_date: Option<String>,
    security_id: Option<i64>,
) -> Result<Vec<ExDividend>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut query = String::from(
        r#"
        SELECT
            e.id, e.security_id, s.name, s.isin,
            e.ex_date, e.record_date, e.pay_date,
            e.amount, e.currency, e.frequency, e.source,
            e.is_confirmed, e.note, e.created_at
        FROM pp_ex_dividend e
        JOIN pp_security s ON s.id = e.security_id
        WHERE 1=1
        "#,
    );

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref start) = start_date {
        query.push_str(" AND e.ex_date >= ?");
        params.push(Box::new(start.clone()));
    }
    if let Some(ref end) = end_date {
        query.push_str(" AND e.ex_date <= ?");
        params.push(Box::new(end.clone()));
    }
    if let Some(sec_id) = security_id {
        query.push_str(" AND e.security_id = ?");
        params.push(Box::new(sec_id));
    }

    query.push_str(" ORDER BY e.ex_date ASC");

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(ExDividend {
                id: row.get(0)?,
                security_id: row.get(1)?,
                security_name: row.get(2)?,
                security_isin: row.get(3)?,
                ex_date: row.get(4)?,
                record_date: row.get(5)?,
                pay_date: row.get(6)?,
                amount: row.get(7)?,
                currency: row.get(8)?,
                frequency: row.get(9)?,
                source: row.get(10)?,
                is_confirmed: row.get::<_, i32>(11)? != 0,
                note: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }

    Ok(result)
}

/// Create a new ex-dividend entry
#[command]
pub fn create_ex_dividend(request: ExDividendRequest) -> Result<ExDividend, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        r#"
        INSERT INTO pp_ex_dividend
            (security_id, ex_date, record_date, pay_date, amount, currency, frequency, source, is_confirmed, note)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        rusqlite::params![
            request.security_id,
            request.ex_date,
            request.record_date,
            request.pay_date,
            request.amount,
            request.currency,
            request.frequency,
            request.source,
            request.is_confirmed.unwrap_or(false) as i32,
            request.note,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Fetch and return the created entry
    let ex_div = conn
        .query_row(
            r#"
            SELECT
                e.id, e.security_id, s.name, s.isin,
                e.ex_date, e.record_date, e.pay_date,
                e.amount, e.currency, e.frequency, e.source,
                e.is_confirmed, e.note, e.created_at
            FROM pp_ex_dividend e
            JOIN pp_security s ON s.id = e.security_id
            WHERE e.id = ?
            "#,
            [id],
            |row| {
                Ok(ExDividend {
                    id: row.get(0)?,
                    security_id: row.get(1)?,
                    security_name: row.get(2)?,
                    security_isin: row.get(3)?,
                    ex_date: row.get(4)?,
                    record_date: row.get(5)?,
                    pay_date: row.get(6)?,
                    amount: row.get(7)?,
                    currency: row.get(8)?,
                    frequency: row.get(9)?,
                    source: row.get(10)?,
                    is_confirmed: row.get::<_, i32>(11)? != 0,
                    note: row.get(12)?,
                    created_at: row.get(13)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(ex_div)
}

/// Update an ex-dividend entry
#[command]
pub fn update_ex_dividend(id: i64, request: ExDividendRequest) -> Result<ExDividend, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        r#"
        UPDATE pp_ex_dividend
        SET security_id = ?1, ex_date = ?2, record_date = ?3, pay_date = ?4,
            amount = ?5, currency = ?6, frequency = ?7, source = ?8,
            is_confirmed = ?9, note = ?10, updated_at = datetime('now')
        WHERE id = ?11
        "#,
        rusqlite::params![
            request.security_id,
            request.ex_date,
            request.record_date,
            request.pay_date,
            request.amount,
            request.currency,
            request.frequency,
            request.source,
            request.is_confirmed.unwrap_or(false) as i32,
            request.note,
            id,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Fetch and return the updated entry
    let ex_div = conn
        .query_row(
            r#"
            SELECT
                e.id, e.security_id, s.name, s.isin,
                e.ex_date, e.record_date, e.pay_date,
                e.amount, e.currency, e.frequency, e.source,
                e.is_confirmed, e.note, e.created_at
            FROM pp_ex_dividend e
            JOIN pp_security s ON s.id = e.security_id
            WHERE e.id = ?
            "#,
            [id],
            |row| {
                Ok(ExDividend {
                    id: row.get(0)?,
                    security_id: row.get(1)?,
                    security_name: row.get(2)?,
                    security_isin: row.get(3)?,
                    ex_date: row.get(4)?,
                    record_date: row.get(5)?,
                    pay_date: row.get(6)?,
                    amount: row.get(7)?,
                    currency: row.get(8)?,
                    frequency: row.get(9)?,
                    source: row.get(10)?,
                    is_confirmed: row.get::<_, i32>(11)? != 0,
                    note: row.get(12)?,
                    created_at: row.get(13)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(ex_div)
}

/// Delete an ex-dividend entry
#[command]
pub fn delete_ex_dividend(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_ex_dividend WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get upcoming ex-dividends for held securities
#[command]
pub fn get_upcoming_ex_dividends(days: Option<i32>) -> Result<Vec<ExDividend>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let today = chrono::Utc::now().date_naive();
    let end_date = today + chrono::Duration::days(days.unwrap_or(30) as i64);

    // Get securities currently held
    let mut held_stmt = conn
        .prepare(
            r#"
            SELECT DISTINCT security_id
            FROM pp_txn
            WHERE owner_type = 'portfolio'
            GROUP BY security_id
            HAVING SUM(CASE
                WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                ELSE 0
            END) > 0
            "#,
        )
        .map_err(|e| e.to_string())?;

    let held_ids: Vec<i64> = held_stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    if held_ids.is_empty() {
        return Ok(Vec::new());
    }

    // Build IN clause
    let placeholders: String = held_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    let query = format!(
        r#"
        SELECT
            e.id, e.security_id, s.name, s.isin,
            e.ex_date, e.record_date, e.pay_date,
            e.amount, e.currency, e.frequency, e.source,
            e.is_confirmed, e.note, e.created_at
        FROM pp_ex_dividend e
        JOIN pp_security s ON s.id = e.security_id
        WHERE e.security_id IN ({})
          AND e.ex_date >= ?
          AND e.ex_date <= ?
        ORDER BY e.ex_date ASC
        "#,
        placeholders
    );

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = held_ids
        .iter()
        .map(|id| Box::new(*id) as Box<dyn rusqlite::ToSql>)
        .collect();
    params.push(Box::new(today.to_string()));
    params.push(Box::new(end_date.to_string()));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(ExDividend {
                id: row.get(0)?,
                security_id: row.get(1)?,
                security_name: row.get(2)?,
                security_isin: row.get(3)?,
                ex_date: row.get(4)?,
                record_date: row.get(5)?,
                pay_date: row.get(6)?,
                amount: row.get(7)?,
                currency: row.get(8)?,
                frequency: row.get(9)?,
                source: row.get(10)?,
                is_confirmed: row.get::<_, i32>(11)? != 0,
                note: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }

    Ok(result)
}

/// Get enhanced calendar with both ex-div dates and payment dates
#[command]
pub fn get_enhanced_dividend_calendar(
    year: i32,
    month: Option<u32>,
) -> Result<Vec<EnhancedMonthCalendarData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let base_currency: String = conn
        .query_row(
            "SELECT base_currency FROM pp_import ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "EUR".to_string());

    // Determine date range
    let (start_date, end_date) = match month {
        Some(m) => {
            let start = format!("{}-{:02}-01", year, m);
            let days_in_month = if m == 12 {
                31
            } else {
                NaiveDate::from_ymd_opt(year, m + 1, 1)
                    .and_then(|d| d.pred_opt())
                    .map(|d| d.day())
                    .unwrap_or(28)
            };
            let end = format!("{}-{:02}-{:02}", year, m, days_in_month);
            (start, end)
        }
        None => (format!("{}-01-01", year), format!("{}-12-31", year)),
    };

    let mut events: Vec<DividendCalendarEvent> = Vec::new();

    // Get ex-dividend dates
    let mut ex_stmt = conn
        .prepare(
            r#"
            SELECT
                e.ex_date, e.record_date, e.pay_date,
                e.security_id, s.name, s.isin,
                e.amount, e.currency, e.is_confirmed
            FROM pp_ex_dividend e
            JOIN pp_security s ON s.id = e.security_id
            WHERE e.ex_date >= ?1 AND e.ex_date <= ?2
            ORDER BY e.ex_date
            "#,
        )
        .map_err(|e| e.to_string())?;

    let ex_rows = ex_stmt
        .query_map([&start_date, &end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<f64>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i32>(8)? != 0,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in ex_rows.flatten() {
        let (ex_date, record_date, _pay_date, sec_id, sec_name, isin, amount, currency, is_confirmed) = row;

        // Add ex-dividend event
        events.push(DividendCalendarEvent {
            date: ex_date.clone(),
            event_type: "ex_dividend".to_string(),
            security_id: sec_id,
            security_name: sec_name.clone(),
            security_isin: isin.clone(),
            amount,
            currency: currency.clone(),
            is_confirmed,
            related_ex_date: None,
        });

        // Add record date event if available
        if let Some(rec_date) = record_date {
            if rec_date >= start_date && rec_date <= end_date {
                events.push(DividendCalendarEvent {
                    date: rec_date,
                    event_type: "record_date".to_string(),
                    security_id: sec_id,
                    security_name: sec_name.clone(),
                    security_isin: isin.clone(),
                    amount,
                    currency: currency.clone(),
                    is_confirmed,
                    related_ex_date: Some(ex_date.clone()),
                });
            }
        }
    }

    // Get actual dividend payments
    let mut pay_stmt = conn
        .prepare(
            r#"
            SELECT
                t.date,
                s.id,
                s.name,
                s.isin,
                (t.amount - COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0)) / 100.0 as net_amount,
                t.currency
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
              AND t.date >= ?1 AND t.date <= ?2
            ORDER BY t.date
            "#,
        )
        .map_err(|e| e.to_string())?;

    let pay_rows = pay_stmt
        .query_map([&start_date, &end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in pay_rows.flatten() {
        let (date, sec_id, sec_name, isin, amount, currency) = row;
        events.push(DividendCalendarEvent {
            date,
            event_type: "payment".to_string(),
            security_id: sec_id,
            security_name: sec_name,
            security_isin: isin,
            amount: Some(amount),
            currency: Some(currency),
            is_confirmed: true,
            related_ex_date: None,
        });
    }

    // Sort events by date
    events.sort_by(|a, b| a.date.cmp(&b.date));

    // Group by month
    let mut month_data: HashMap<u32, EnhancedMonthCalendarData> = HashMap::new();

    for event in events {
        let parsed_date = NaiveDate::parse_from_str(&event.date, "%Y-%m-%d").ok();
        let m = parsed_date.map(|d| d.month()).unwrap_or(1);

        let entry = month_data.entry(m).or_insert_with(|| EnhancedMonthCalendarData {
            year,
            month: m,
            events: Vec::new(),
            total_payments: 0.0,
            upcoming_ex_divs: 0,
            currency: base_currency.clone(),
        });

        if event.event_type == "payment" {
            entry.total_payments += event.amount.unwrap_or(0.0);
        } else if event.event_type == "ex_dividend" {
            entry.upcoming_ex_divs += 1;
        }

        entry.events.push(event);
    }

    // Sort by month
    let mut result: Vec<EnhancedMonthCalendarData> = month_data.into_values().collect();
    result.sort_by_key(|m| m.month);

    Ok(result)
}
