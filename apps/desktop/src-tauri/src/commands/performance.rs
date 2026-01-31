//! Performance calculation commands for Tauri

use crate::db;
use crate::performance;
use crate::pp::parse_date_flexible; // SSOT: centralized date parsing
use chrono::NaiveDate;
use serde::Serialize;
use tauri::command;

/// Performance result for frontend
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceResult {
    /// TTWROR (True Time-Weighted Rate of Return) as percentage
    pub ttwror: f64,
    /// Annualized TTWROR as percentage
    pub ttwror_annualized: f64,
    /// IRR (Internal Rate of Return) as percentage
    pub irr: f64,
    /// Whether IRR calculation converged
    pub irr_converged: bool,
    /// Number of days in the period
    pub days: i64,
    /// Start date
    pub start_date: String,
    /// End date
    pub end_date: String,
    /// Current portfolio value
    pub current_value: f64,
    /// Total invested (sum of deposits)
    pub total_invested: f64,
    /// Absolute gain/loss
    pub absolute_gain: f64,
}

/// Detailed period return for charts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodReturnData {
    pub start_date: String,
    pub end_date: String,
    pub start_value: f64,
    pub end_value: f64,
    pub cash_flow: f64,
    pub return_rate: f64,
}

/// Calculate performance metrics for a portfolio
#[command]
pub fn calculate_performance(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<PerformanceResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Parse dates or use defaults
    let start = start_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| {
            // Default to first transaction date
            get_first_transaction_date(conn, portfolio_id)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        });

    let end = end_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    // Calculate TTWROR
    let ttwror_result = performance::calculate_ttwror(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    // Get cash flows for IRR calculation
    let mut cash_flows = get_cash_flows_for_irr(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    // Get portfolio value at START date (initial investment)
    // IRR requires the initial portfolio value as a cash flow (like Portfolio Performance)
    let start_value = performance::get_portfolio_value_at_date_with_currency(conn, portfolio_id, start)
        .map_err(|e| e.to_string())?;

    // Add initial value as cash flow at start - represents existing investment at period start
    if start_value > 0.0 {
        cash_flows.insert(0, performance::CashFlow { date: start, amount: start_value });
    }

    // Get portfolio value at end_date using SSOT
    let current_value = performance::get_portfolio_value_at_date_with_currency(conn, portfolio_id, end)
        .map_err(|e| e.to_string())?;

    // Calculate IRR
    let irr_result = performance::calculate_irr(&cash_flows, current_value, end)
        .map_err(|e| e.to_string())?;

    // Calculate total invested (positive cash flows = money invested)
    let total_invested: f64 = cash_flows.iter().filter(|cf| cf.amount > 0.0).map(|cf| cf.amount).sum();

    // Calculate total withdrawn (negative cash flows = money withdrawn)
    let total_withdrawn: f64 = cash_flows.iter().filter(|cf| cf.amount < 0.0).map(|cf| -cf.amount).sum();

    // Absolute gain = current value + withdrawals - investments
    let absolute_gain = current_value + total_withdrawn - total_invested;

    Ok(PerformanceResult {
        ttwror: ttwror_result.total_return * 100.0,
        ttwror_annualized: ttwror_result.annualized_return * 100.0,
        irr: irr_result.irr * 100.0,
        irr_converged: irr_result.converged,
        days: ttwror_result.days,
        start_date: start.to_string(),
        end_date: end.to_string(),
        current_value,
        total_invested,
        absolute_gain,
    })
}

/// Get period returns for detailed analysis
#[command]
pub fn get_period_returns(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<Vec<PeriodReturnData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let start = start_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| {
            get_first_transaction_date(conn, portfolio_id)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 1, 1).unwrap())
        });

    let end = end_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let ttwror_result = performance::calculate_ttwror(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    let periods: Vec<PeriodReturnData> = ttwror_result
        .periods
        .into_iter()
        .map(|p| PeriodReturnData {
            start_date: p.start_date.to_string(),
            end_date: p.end_date.to_string(),
            start_value: p.start_value,
            end_value: p.end_value,
            cash_flow: p.cash_flow,
            return_rate: p.return_rate * 100.0,
        })
        .collect();

    Ok(periods)
}

/// Helper: Get first transaction date
fn get_first_transaction_date(
    conn: &rusqlite::Connection,
    portfolio_id: Option<i64>,
) -> Option<NaiveDate> {
    let sql = if portfolio_id.is_some() {
        "SELECT MIN(date) FROM pp_txn WHERE owner_type = 'portfolio' AND owner_id = ?1"
    } else {
        "SELECT MIN(date) FROM pp_txn WHERE owner_type = 'portfolio'"
    };

    let result: Option<String> = if let Some(pid) = portfolio_id {
        conn.query_row(sql, [pid], |row| row.get(0)).ok()
    } else {
        conn.query_row(sql, [], |row| row.get(0)).ok()
    };

    result.and_then(|s| parse_date_flexible(&s))
}

/// Helper: Get cash flows for IRR
///
/// Uses the central SSOT function from performance module which:
/// - Only includes EXTERNAL cash flows (DEPOSIT/REMOVAL)
/// - Converts all amounts to base currency
/// - Falls back to BUY/SELL only when no DEPOSIT/REMOVAL exist
///
/// IMPORTANT: This does NOT mix BUY/SELL with DEPOSIT/REMOVAL to avoid double-counting!
/// BUY/SELL are internal transactions (cash ↔ securities), not external cash flows.
fn get_cash_flows_for_irr(
    conn: &rusqlite::Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<performance::CashFlow>, anyhow::Error> {
    // Use central SSOT function with fallback (for IRR only)
    performance::get_cash_flows_with_fallback(conn, portfolio_id, start_date, end_date)
}

/// Calculate risk metrics for a portfolio
///
/// Returns Sharpe, Sortino, Max Drawdown, Volatility, Beta/Alpha
#[command]
pub fn calculate_risk_metrics(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    benchmark_id: Option<i64>,
    risk_free_rate: Option<f64>,
) -> Result<performance::RiskMetrics, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Parse dates or use defaults (1 year back)
    let end = end_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    let start = start_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| {
            // Default: 1 year back from end date
            end - chrono::Duration::days(365)
        });

    performance::calculate_risk_metrics(conn, portfolio_id, start, end, benchmark_id, risk_free_rate)
        .map_err(|e| e.to_string())
}
