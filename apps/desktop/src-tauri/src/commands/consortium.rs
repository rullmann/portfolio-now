//! Consortium (Portfolio Group) Commands
//!
//! Allows combining multiple portfolios into a "virtual portfolio" for
//! consolidated performance analysis and comparison.
//!
//! ## Key Concepts
//!
//! - **Consortium**: A named group of portfolios that can be analyzed together
//! - **Virtual Portfolio**: The combined view of all portfolios in a consortium
//! - **Performance Metrics**: TTWROR, IRR, and risk metrics calculated across all portfolios
//!
//! ## Usage
//!
//! ```rust
//! // Create a consortium
//! create_consortium(CreateConsortiumRequest { name: "Family Portfolio", portfolio_ids: vec![1, 2, 3] })
//!
//! // Get combined performance
//! get_consortium_performance(consortium_id)
//!
//! // Compare portfolios side-by-side
//! compare_portfolios(vec![1, 2, 3])
//! ```

use crate::currency;
use crate::db;
use crate::fifo;
use crate::performance::{self, CashFlow};
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// A portfolio group (consortium) for combined analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Consortium {
    pub id: i64,
    pub name: String,
    pub portfolio_ids: Vec<i64>,
    pub created_at: String,
}

/// Request to create a consortium
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConsortiumRequest {
    pub name: String,
    pub portfolio_ids: Vec<i64>,
}

/// Combined performance result for a consortium
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsortiumPerformance {
    pub consortium_id: i64,
    pub consortium_name: String,
    /// Total current value of all portfolios
    pub total_value: f64,
    /// Total cost basis (FIFO SSOT)
    pub total_cost_basis: f64,
    /// Absolute gain/loss
    pub total_gain_loss: f64,
    /// Gain/loss as percentage
    pub total_gain_loss_percent: f64,
    /// TTWROR (True Time-Weighted Rate of Return) as percentage
    pub ttwror: f64,
    /// Annualized TTWROR as percentage
    pub ttwror_annualized: f64,
    /// IRR (Internal Rate of Return) as percentage
    pub irr: f64,
    /// Whether IRR calculation converged
    pub irr_converged: bool,
    /// Total invested capital (sum of deposits)
    pub total_invested: f64,
    /// Number of days since first transaction
    pub days: i64,
    /// Start date
    pub start_date: String,
    /// End date
    pub end_date: String,
    /// Base currency
    pub currency: String,
    /// Risk metrics (if enough data available)
    pub risk_metrics: Option<ConsortiumRiskMetrics>,
    /// Performance per portfolio in the consortium
    pub by_portfolio: Vec<PortfolioPerformanceSummary>,
}

/// Risk metrics for consortium
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsortiumRiskMetrics {
    /// Annualized volatility
    pub volatility: f64,
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    /// Sortino ratio
    pub sortino_ratio: f64,
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Max drawdown start date
    pub max_drawdown_start: Option<String>,
    /// Max drawdown end date
    pub max_drawdown_end: Option<String>,
}

/// Performance summary for a single portfolio
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioPerformanceSummary {
    pub portfolio_id: i64,
    pub portfolio_name: String,
    /// Current value
    pub value: f64,
    /// Cost basis
    pub cost_basis: f64,
    /// Absolute gain/loss
    pub gain_loss: f64,
    /// Gain/loss percentage
    pub gain_loss_percent: f64,
    /// TTWROR percentage
    pub ttwror: f64,
    /// Annualized TTWROR
    pub ttwror_annualized: f64,
    /// IRR percentage
    pub irr: f64,
    /// Weight in consortium (% of total value)
    pub weight: f64,
}

/// Comparison data for multiple portfolios side-by-side
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioComparison {
    pub portfolios: Vec<PortfolioComparisonEntry>,
    /// Combined totals
    pub combined: CombinedComparison,
}

/// Entry for portfolio comparison
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioComparisonEntry {
    pub portfolio_id: i64,
    pub portfolio_name: String,
    pub current_value: f64,
    pub cost_basis: f64,
    pub absolute_gain: f64,
    pub percent_gain: f64,
    pub ttwror: f64,
    pub ttwror_annualized: f64,
    pub irr: f64,
    pub days: i64,
    /// Color for chart display
    pub color: String,
}

/// Combined comparison totals
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombinedComparison {
    pub total_value: f64,
    pub total_cost_basis: f64,
    pub total_gain: f64,
    pub total_gain_percent: f64,
    pub combined_ttwror: f64,
    pub combined_ttwror_annualized: f64,
    pub combined_irr: f64,
}

/// Historical performance data point for charts
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceHistoryPoint {
    pub date: String,
    pub value: f64,
    pub cumulative_return: f64,
}

/// Historical performance for consortium
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsortiumHistory {
    pub consortium_id: i64,
    pub currency: String,
    /// Combined value history
    pub combined: Vec<PerformanceHistoryPoint>,
    /// Per-portfolio history
    pub by_portfolio: Vec<PortfolioHistory>,
}

/// Historical data for a single portfolio
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioHistory {
    pub portfolio_id: i64,
    pub portfolio_name: String,
    pub color: String,
    pub data: Vec<PerformanceHistoryPoint>,
}

// Chart colors for portfolios
const PORTFOLIO_COLORS: &[&str] = &[
    "#3b82f6", // blue
    "#10b981", // green
    "#f59e0b", // amber
    "#ef4444", // red
    "#8b5cf6", // purple
    "#ec4899", // pink
    "#06b6d4", // cyan
    "#84cc16", // lime
];

/// Get all consortiums
#[tauri::command]
pub fn get_consortiums() -> Result<Vec<Consortium>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, portfolio_ids, created_at FROM pp_consortium ORDER BY name",
        )
        .map_err(|e| e.to_string())?;

    let consortiums = stmt
        .query_map([], |row| {
            let portfolio_ids_str: String = row.get(2)?;
            let portfolio_ids: Vec<i64> = serde_json::from_str(&portfolio_ids_str).unwrap_or_default();
            Ok(Consortium {
                id: row.get(0)?,
                name: row.get(1)?,
                portfolio_ids,
                created_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(consortiums)
}

/// Create a new consortium
#[tauri::command]
pub fn create_consortium(request: CreateConsortiumRequest) -> Result<Consortium, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let portfolio_ids_json = serde_json::to_string(&request.portfolio_ids)
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO pp_consortium (name, portfolio_ids, created_at) VALUES (?1, ?2, ?3)",
        params![request.name, portfolio_ids_json, now],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(Consortium {
        id,
        name: request.name,
        portfolio_ids: request.portfolio_ids,
        created_at: now,
    })
}

/// Update a consortium
#[tauri::command]
pub fn update_consortium(id: i64, request: CreateConsortiumRequest) -> Result<Consortium, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let portfolio_ids_json = serde_json::to_string(&request.portfolio_ids)
        .map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE pp_consortium SET name = ?1, portfolio_ids = ?2 WHERE id = ?3",
        params![request.name, portfolio_ids_json, id],
    )
    .map_err(|e| e.to_string())?;

    let created_at: String = conn
        .query_row(
            "SELECT created_at FROM pp_consortium WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(Consortium {
        id,
        name: request.name,
        portfolio_ids: request.portfolio_ids,
        created_at,
    })
}

/// Delete a consortium
#[tauri::command]
pub fn delete_consortium(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_consortium WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get first and last transaction dates for a portfolio
fn get_portfolio_date_range(conn: &Connection, portfolio_id: i64) -> (NaiveDate, NaiveDate) {
    let sql = r#"
        SELECT MIN(date), MAX(date)
        FROM pp_txn
        WHERE owner_type = 'portfolio' AND owner_id = ?1
    "#;

    let default_start = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let default_end = chrono::Utc::now().date_naive();

    if let Ok((min, max)) = conn.query_row(sql, params![portfolio_id], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
        ))
    }) {
        let start = min
            .and_then(|s| NaiveDate::parse_from_str(&s.split('T').next().unwrap_or(&s).split(' ').next().unwrap_or(&s), "%Y-%m-%d").ok())
            .unwrap_or(default_start);
        let end = max
            .and_then(|s| NaiveDate::parse_from_str(&s.split('T').next().unwrap_or(&s).split(' ').next().unwrap_or(&s), "%Y-%m-%d").ok())
            .unwrap_or(default_end);
        (start, end)
    } else {
        (default_start, default_end)
    }
}

/// Calculate combined date range for multiple portfolios
fn get_combined_date_range(conn: &Connection, portfolio_ids: &[i64]) -> (NaiveDate, NaiveDate) {
    let mut min_date = chrono::Utc::now().date_naive();
    let mut max_date = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();

    for pid in portfolio_ids {
        let (start, end) = get_portfolio_date_range(conn, *pid);
        if start < min_date {
            min_date = start;
        }
        if end > max_date {
            max_date = end;
        }
    }

    (min_date, max_date)
}

/// Calculate performance metrics for a single portfolio
fn calculate_portfolio_metrics(
    conn: &Connection,
    portfolio_id: i64,
    base_currency: &str,
    valuation_date: NaiveDate,
) -> PortfolioPerformanceSummary {
    // Get portfolio name
    let portfolio_name: String = conn
        .query_row(
            "SELECT name FROM pp_portfolio WHERE id = ?1",
            params![portfolio_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| format!("Portfolio {}", portfolio_id));

    let (start_date, _) = get_portfolio_date_range(conn, portfolio_id);
    let end_date = valuation_date;

    // Get portfolio value at end_date using SSOT
    let value = performance::get_portfolio_value_at_date_with_currency(conn, Some(portfolio_id), end_date)
        .unwrap_or(0.0);

    // Get cost basis using FIFO SSOT
    let cost_basis = fifo::get_total_cost_basis_converted(conn, Some(portfolio_id), base_currency)
        .unwrap_or(0.0);

    let gain_loss = value - cost_basis;
    let gain_loss_percent = if cost_basis > 0.0 {
        gain_loss / cost_basis * 100.0
    } else {
        0.0
    };

    let ttwror_result = performance::calculate_ttwror(conn, Some(portfolio_id), start_date, end_date)
        .unwrap_or(performance::TtwrorResult {
            total_return: 0.0,
            annualized_return: 0.0,
            days: 0,
            periods: vec![],
        });

    let ttwror = ttwror_result.total_return * 100.0;
    let ttwror_annualized = ttwror_result.annualized_return * 100.0;

    let cash_flows = performance::get_cash_flows_with_fallback(conn, Some(portfolio_id), start_date, end_date)
        .unwrap_or_default();
    let irr = if !cash_flows.is_empty() {
        performance::calculate_irr(&cash_flows, value, end_date)
            .map(|r| r.irr * 100.0)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    PortfolioPerformanceSummary {
        portfolio_id,
        portfolio_name,
        value,
        cost_basis,
        gain_loss,
        gain_loss_percent,
        ttwror,
        ttwror_annualized,
        irr,
        weight: 0.0, // Will be calculated after totals
    }
}

/// Calculate combined performance for a consortium
#[tauri::command]
pub fn get_consortium_performance(consortium_id: i64) -> Result<ConsortiumPerformance, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get consortium
    let (name, portfolio_ids_str): (String, String) = conn
        .query_row(
            "SELECT name, portfolio_ids FROM pp_consortium WHERE id = ?1",
            params![consortium_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Consortium not found: {}", e))?;

    let portfolio_ids: Vec<i64> = serde_json::from_str(&portfolio_ids_str).unwrap_or_default();
    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());

    // Get combined date range
    let (start_date, end_date) = get_combined_date_range(conn, &portfolio_ids);

    // Calculate performance for each portfolio (aligned to consortium end_date)
    let mut by_portfolio: Vec<PortfolioPerformanceSummary> = Vec::new();
    let mut total_value = 0.0;
    let mut total_cost_basis = 0.0;

    for pid in &portfolio_ids {
        let metrics = calculate_portfolio_metrics(conn, *pid, &base_currency, end_date);
        total_value += metrics.value;
        total_cost_basis += metrics.cost_basis;
        by_portfolio.push(metrics);
    }

    // Calculate weights
    for portfolio in &mut by_portfolio {
        portfolio.weight = if total_value > 0.0 {
            portfolio.value / total_value * 100.0
        } else {
            0.0
        };
    }

    // Calculate combined metrics
    let total_gain_loss = total_value - total_cost_basis;
    let total_gain_loss_percent = if total_cost_basis > 0.0 {
        total_gain_loss / total_cost_basis * 100.0
    } else {
        0.0
    };

    let ttwror_result = performance::calculate_ttwror_for_portfolios(conn, &portfolio_ids, start_date, end_date)
        .unwrap_or(performance::TtwrorResult {
            total_return: 0.0,
            annualized_return: 0.0,
            days: 0,
            periods: vec![],
        });

    let ttwror = ttwror_result.total_return * 100.0;
    let ttwror_annualized = ttwror_result.annualized_return * 100.0;
    let days = ttwror_result.days;

    // Combine all cash flows for IRR (SSOT with fallback)
    let mut all_cash_flows: Vec<CashFlow> = Vec::new();
    for pid in &portfolio_ids {
        if let Ok(cf) = performance::get_cash_flows_with_fallback(conn, Some(*pid), start_date, end_date) {
            all_cash_flows.extend(cf);
        }
    }
    all_cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    let (irr, irr_converged) = if !all_cash_flows.is_empty() {
        performance::calculate_irr(&all_cash_flows, total_value, end_date)
            .map(|r| (r.irr * 100.0, r.converged))
            .unwrap_or((0.0, false))
    } else {
        (0.0, false)
    };

    // Calculate total invested
    let total_invested: f64 = all_cash_flows
        .iter()
        .filter(|cf| cf.amount > 0.0)
        .map(|cf| cf.amount)
        .sum();

    // Calculate risk metrics if enough data
    let risk_metrics = calculate_consortium_risk_metrics(conn, &portfolio_ids);

    Ok(ConsortiumPerformance {
        consortium_id,
        consortium_name: name,
        total_value,
        total_cost_basis,
        total_gain_loss,
        total_gain_loss_percent,
        ttwror,
        ttwror_annualized,
        irr,
        irr_converged,
        total_invested,
        days,
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        currency: base_currency,
        risk_metrics,
        by_portfolio,
    })
}

/// Calculate risk metrics for combined portfolios
fn calculate_consortium_risk_metrics(
    conn: &Connection,
    portfolio_ids: &[i64],
) -> Option<ConsortiumRiskMetrics> {
    let today = chrono::Utc::now().date_naive();
    let one_year_ago = today - chrono::Duration::days(365);

    // Get combined portfolio value history
    let values = get_combined_value_history(conn, portfolio_ids, one_year_ago, today);

    if values.len() < 30 {
        return None;
    }

    // Calculate daily returns
    let returns: Vec<f64> = values
        .windows(2)
        .filter_map(|w| {
            let prev = &w[0];
            let curr = &w[1];
            if prev.1 > 0.0 {
                Some((curr.1 - prev.1) / prev.1)
            } else {
                None
            }
        })
        .collect();

    if returns.is_empty() {
        return None;
    }

    // Volatility (annualized standard deviation)
    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|r| (r - mean_return).powi(2)).sum::<f64>() / returns.len() as f64;
    let volatility = variance.sqrt() * (252.0_f64).sqrt();

    // Annualized return
    let annualized_return = mean_return * 252.0;

    // Sharpe ratio (assuming 3% risk-free rate)
    let risk_free_rate = 0.03;
    let sharpe_ratio = if volatility > 0.0 {
        (annualized_return - risk_free_rate) / volatility
    } else {
        0.0
    };

    // Sortino ratio (downside deviation)
    let daily_rf = risk_free_rate / 252.0;
    let downside_returns: Vec<f64> = returns
        .iter()
        .filter_map(|&r| {
            if r < daily_rf {
                Some((r - daily_rf).powi(2))
            } else {
                None
            }
        })
        .collect();

    let downside_deviation = if !downside_returns.is_empty() {
        (downside_returns.iter().sum::<f64>() / returns.len() as f64).sqrt() * (252.0_f64).sqrt()
    } else {
        0.0
    };

    let sortino_ratio = if downside_deviation > 0.0 {
        (annualized_return - risk_free_rate) / downside_deviation
    } else {
        0.0
    };

    // Maximum drawdown
    let mut max_value = values[0].1;
    let mut max_date = values[0].0.clone();
    let mut max_drawdown = 0.0;
    let mut dd_start: Option<String> = None;
    let mut dd_end: Option<String> = None;

    for (date, value) in &values {
        if *value > max_value {
            max_value = *value;
            max_date = date.clone();
        }

        let drawdown = (max_value - value) / max_value;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
            dd_start = Some(max_date.clone());
            dd_end = Some(date.clone());
        }
    }

    Some(ConsortiumRiskMetrics {
        volatility: volatility * 100.0, // as percentage
        sharpe_ratio,
        sortino_ratio,
        max_drawdown: max_drawdown * 100.0, // as percentage
        max_drawdown_start: dd_start,
        max_drawdown_end: dd_end,
    })
}

/// Get combined value history for multiple portfolios
fn get_combined_value_history(
    conn: &Connection,
    portfolio_ids: &[i64],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Vec<(String, f64)> {
    // Get all unique dates with prices in range
    let dates_sql = r#"
        SELECT DISTINCT date(date) as d
        FROM pp_price
        WHERE date(date) >= ? AND date(date) <= ?
        ORDER BY d
    "#;

    let mut dates: Vec<String> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(dates_sql) {
        if let Ok(rows) = stmt.query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| row.get::<_, String>(0),
        ) {
            for row in rows.flatten() {
                dates.push(row);
            }
        }
    }

    let mut values: Vec<(String, f64)> = Vec::new();

    for date_str in dates {
        let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        let mut total_value = 0.0;

        for pid in portfolio_ids {
            if let Ok(value) = performance::get_portfolio_value_at_date_with_currency(conn, Some(*pid), date) {
                total_value += value;
            }
        }

        if total_value > 0.0 {
            values.push((date_str, total_value));
        }
    }

    values
}

/// Compare multiple portfolios side-by-side
#[tauri::command]
pub fn compare_portfolios(portfolio_ids: Vec<i64>) -> Result<PortfolioComparison, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());
    let mut portfolios: Vec<PortfolioComparisonEntry> = Vec::new();
    let (combined_start, combined_end) = get_combined_date_range(conn, &portfolio_ids);

    for (idx, pid) in portfolio_ids.iter().enumerate() {
        let (start, end) = get_portfolio_date_range(conn, *pid);
        let metrics = calculate_portfolio_metrics(conn, *pid, &base_currency, end);
        let days = (end - start).num_days();

        portfolios.push(PortfolioComparisonEntry {
            portfolio_id: *pid,
            portfolio_name: metrics.portfolio_name,
            current_value: metrics.value,
            cost_basis: metrics.cost_basis,
            absolute_gain: metrics.gain_loss,
            percent_gain: metrics.gain_loss_percent,
            ttwror: metrics.ttwror,
            ttwror_annualized: metrics.ttwror_annualized,
            irr: metrics.irr,
            days,
            color: PORTFOLIO_COLORS[idx % PORTFOLIO_COLORS.len()].to_string(),
        });
    }

    // Calculate combined metrics
    let total_value: f64 = portfolio_ids
        .iter()
        .map(|pid| performance::get_portfolio_value_at_date_with_currency(conn, Some(*pid), combined_end).unwrap_or(0.0))
        .sum();
    let total_cost_basis: f64 = portfolio_ids
        .iter()
        .map(|pid| fifo::get_total_cost_basis_converted(conn, Some(*pid), &base_currency).unwrap_or(0.0))
        .sum();
    let total_gain = total_value - total_cost_basis;
    let total_gain_percent = if total_cost_basis > 0.0 {
        total_gain / total_cost_basis * 100.0
    } else {
        0.0
    };

    let combined_ttwror_result =
        performance::calculate_ttwror_for_portfolios(conn, &portfolio_ids, combined_start, combined_end)
            .unwrap_or(performance::TtwrorResult {
                total_return: 0.0,
                annualized_return: 0.0,
                days: 0,
                periods: vec![],
            });
    let combined_ttwror = combined_ttwror_result.total_return * 100.0;
    let combined_ttwror_annualized = combined_ttwror_result.annualized_return * 100.0;

    // Combined IRR
    let mut all_cash_flows: Vec<CashFlow> = Vec::new();
    for pid in &portfolio_ids {
        if let Ok(cf) = performance::get_cash_flows_with_fallback(conn, Some(*pid), combined_start, combined_end) {
            all_cash_flows.extend(cf);
        }
    }
    all_cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    let combined_irr = if !all_cash_flows.is_empty() {
        performance::calculate_irr(&all_cash_flows, total_value, combined_end)
            .map(|r| r.irr * 100.0)
            .unwrap_or(0.0)
    } else {
        0.0
    };

    Ok(PortfolioComparison {
        portfolios,
        combined: CombinedComparison {
            total_value,
            total_cost_basis,
            total_gain,
            total_gain_percent,
            combined_ttwror,
            combined_ttwror_annualized,
            combined_irr,
        },
    })
}

/// Get historical performance data for a consortium (for charts)
#[tauri::command]
pub fn get_consortium_history(
    consortium_id: i64,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<ConsortiumHistory, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get consortium
    let portfolio_ids_str: String = conn
        .query_row(
            "SELECT portfolio_ids FROM pp_consortium WHERE id = ?1",
            params![consortium_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Consortium not found: {}", e))?;

    let portfolio_ids: Vec<i64> = serde_json::from_str(&portfolio_ids_str).unwrap_or_default();
    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());

    // Parse date range
    let (combined_start, combined_end) = get_combined_date_range(conn, &portfolio_ids);

    let start = start_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or(combined_start);

    let end = end_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or(combined_end);

    // Get combined history
    let combined_values = get_combined_value_history(conn, &portfolio_ids, start, end);

    // Calculate cumulative returns for combined
    let combined: Vec<PerformanceHistoryPoint> = if !combined_values.is_empty() {
        let first_value = combined_values[0].1;
        combined_values
            .iter()
            .map(|(date, value)| PerformanceHistoryPoint {
                date: date.clone(),
                value: *value,
                cumulative_return: if first_value > 0.0 {
                    (*value - first_value) / first_value * 100.0
                } else {
                    0.0
                },
            })
            .collect()
    } else {
        vec![]
    };

    // Get per-portfolio history
    let mut by_portfolio: Vec<PortfolioHistory> = Vec::new();
    for (idx, pid) in portfolio_ids.iter().enumerate() {
        let portfolio_name: String = conn
            .query_row(
                "SELECT name FROM pp_portfolio WHERE id = ?1",
                params![pid],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| format!("Portfolio {}", pid));

        let values = get_single_portfolio_value_history(conn, *pid, start, end);

        let data: Vec<PerformanceHistoryPoint> = if !values.is_empty() {
            let first_value = values[0].1;
            values
                .iter()
                .map(|(date, value)| PerformanceHistoryPoint {
                    date: date.clone(),
                    value: *value,
                    cumulative_return: if first_value > 0.0 {
                        (*value - first_value) / first_value * 100.0
                    } else {
                        0.0
                    },
                })
                .collect()
        } else {
            vec![]
        };

        by_portfolio.push(PortfolioHistory {
            portfolio_id: *pid,
            portfolio_name,
            color: PORTFOLIO_COLORS[idx % PORTFOLIO_COLORS.len()].to_string(),
            data,
        });
    }

    Ok(ConsortiumHistory {
        consortium_id,
        currency: base_currency,
        combined,
        by_portfolio,
    })
}

/// Get value history for a single portfolio
fn get_single_portfolio_value_history(
    conn: &Connection,
    portfolio_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Vec<(String, f64)> {
    let dates_sql = r#"
        SELECT DISTINCT date(date) as d
        FROM pp_price
        WHERE date(date) >= ? AND date(date) <= ?
        ORDER BY d
    "#;

    let mut dates: Vec<String> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(dates_sql) {
        if let Ok(rows) = stmt.query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| row.get::<_, String>(0),
        ) {
            for row in rows.flatten() {
                dates.push(row);
            }
        }
    }

    let mut values: Vec<(String, f64)> = Vec::new();

    for date_str in dates {
        let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        if let Ok(value) = performance::get_portfolio_value_at_date_with_currency(conn, Some(portfolio_id), date) {
            if value > 0.0 {
                values.push((date_str, value));
            }
        }
    }

    values
}
