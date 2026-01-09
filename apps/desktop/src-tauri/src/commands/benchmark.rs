//! Benchmark Comparison Commands
//!
//! Compare portfolio performance against benchmark securities.

use crate::db;
use serde::{Deserialize, Serialize};
use tauri::command;

/// Benchmark data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkData {
    pub id: i64,
    pub security_id: i64,
    pub security_name: String,
    pub isin: Option<String>,
    pub start_date: String,
}

/// Benchmark comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub portfolio_return: f64,
    pub benchmark_return: f64,
    pub alpha: f64,
    pub beta: f64,
    pub sharpe_ratio: f64,
    pub correlation: f64,
    pub tracking_error: f64,
    pub information_ratio: f64,
    pub max_drawdown_portfolio: f64,
    pub max_drawdown_benchmark: f64,
}

/// Data point for benchmark comparison chart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkDataPoint {
    pub date: String,
    pub portfolio_value: f64,
    pub portfolio_return: f64,
    pub benchmark_value: f64,
    pub benchmark_return: f64,
}

/// Ensure benchmark table exists
fn ensure_tables(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pp_benchmark (
            id INTEGER PRIMARY KEY,
            security_id INTEGER NOT NULL UNIQUE,
            start_date TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (security_id) REFERENCES pp_security(id)
        )",
        [],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all benchmarks
#[command]
pub fn get_benchmarks() -> Result<Vec<BenchmarkData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    ensure_tables(conn)?;

    let mut stmt = conn.prepare(
        "SELECT b.id, b.security_id, s.name, s.isin, b.start_date
         FROM pp_benchmark b
         JOIN pp_security s ON s.id = b.security_id
         ORDER BY s.name"
    ).map_err(|e| e.to_string())?;

    let benchmarks = stmt.query_map([], |row| {
        Ok(BenchmarkData {
            id: row.get(0)?,
            security_id: row.get(1)?,
            security_name: row.get(2)?,
            isin: row.get(3)?,
            start_date: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;

    benchmarks.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Add a security as a benchmark
#[command]
pub fn add_benchmark(security_id: i64, start_date: Option<String>) -> Result<BenchmarkData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    ensure_tables(conn)?;

    // Get earliest date for this security if not specified
    let start = match start_date {
        Some(d) => d,
        None => {
            conn.query_row(
                "SELECT MIN(date) FROM pp_price WHERE security_id = ?1",
                [security_id],
                |row| row.get::<_, String>(0)
            ).unwrap_or_else(|_| chrono::Local::now().format("%Y-%m-%d").to_string())
        }
    };

    conn.execute(
        "INSERT OR REPLACE INTO pp_benchmark (security_id, start_date) VALUES (?1, ?2)",
        rusqlite::params![security_id, start],
    ).map_err(|e| e.to_string())?;

    // Get the benchmark data
    conn.query_row(
        "SELECT b.id, b.security_id, s.name, s.isin, b.start_date
         FROM pp_benchmark b
         JOIN pp_security s ON s.id = b.security_id
         WHERE b.security_id = ?1",
        [security_id],
        |row| {
            Ok(BenchmarkData {
                id: row.get(0)?,
                security_id: row.get(1)?,
                security_name: row.get(2)?,
                isin: row.get(3)?,
                start_date: row.get(4)?,
            })
        }
    ).map_err(|e| e.to_string())
}

/// Remove a benchmark
#[command]
pub fn remove_benchmark(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_benchmark WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Compare portfolio performance against a benchmark
#[command]
pub fn compare_to_benchmark(
    portfolio_id: Option<i64>,
    benchmark_id: i64,
    start_date: String,
    end_date: String,
) -> Result<BenchmarkComparison, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get benchmark security_id
    let benchmark_security_id: i64 = conn.query_row(
        "SELECT security_id FROM pp_benchmark WHERE id = ?1",
        [benchmark_id],
        |row| row.get(0)
    ).map_err(|e| format!("Benchmark not found: {}", e))?;

    // Get benchmark prices
    let benchmark_prices = get_price_series(conn, benchmark_security_id, &start_date, &end_date)?;

    if benchmark_prices.len() < 2 {
        return Err("Not enough benchmark data".to_string());
    }

    // Calculate benchmark return
    let benchmark_start_price = benchmark_prices.first().map(|(_, p)| *p).unwrap_or(1.0);
    let benchmark_end_price = benchmark_prices.last().map(|(_, p)| *p).unwrap_or(1.0);
    let benchmark_return = ((benchmark_end_price / benchmark_start_price) - 1.0) * 100.0;

    // Get portfolio values over time
    let portfolio_values = get_portfolio_values(conn, portfolio_id, &start_date, &end_date)?;

    if portfolio_values.len() < 2 {
        return Err("Not enough portfolio data".to_string());
    }

    // Calculate portfolio return
    let portfolio_start_value = portfolio_values.first().map(|(_, v)| *v).unwrap_or(1.0);
    let portfolio_end_value = portfolio_values.last().map(|(_, v)| *v).unwrap_or(1.0);
    let portfolio_return = ((portfolio_end_value / portfolio_start_value) - 1.0) * 100.0;

    // Calculate daily returns for both
    let benchmark_daily_returns = calculate_daily_returns(&benchmark_prices);
    let portfolio_daily_returns = calculate_daily_returns(&portfolio_values);

    // Align data by date
    let (aligned_portfolio, aligned_benchmark) = align_returns(&portfolio_daily_returns, &benchmark_daily_returns);

    if aligned_portfolio.is_empty() {
        return Err("No overlapping data between portfolio and benchmark".to_string());
    }

    // Calculate statistics
    let alpha = portfolio_return - benchmark_return;
    let beta = calculate_beta(&aligned_portfolio, &aligned_benchmark);
    let correlation = calculate_correlation(&aligned_portfolio, &aligned_benchmark);
    let tracking_error = calculate_tracking_error(&aligned_portfolio, &aligned_benchmark);
    let sharpe_ratio = calculate_sharpe_ratio(&aligned_portfolio);
    let information_ratio = if tracking_error > 0.0 {
        (portfolio_return - benchmark_return) / (tracking_error * 100.0)
    } else {
        0.0
    };

    // Calculate max drawdowns
    let max_drawdown_portfolio = calculate_max_drawdown(&portfolio_values);
    let max_drawdown_benchmark = calculate_max_drawdown(&benchmark_prices);

    Ok(BenchmarkComparison {
        portfolio_return,
        benchmark_return,
        alpha,
        beta,
        sharpe_ratio,
        correlation,
        tracking_error,
        information_ratio,
        max_drawdown_portfolio,
        max_drawdown_benchmark,
    })
}

/// Get time series data for benchmark comparison chart
#[command]
pub fn get_benchmark_comparison_data(
    portfolio_id: Option<i64>,
    benchmark_id: i64,
    start_date: String,
    end_date: String,
) -> Result<Vec<BenchmarkDataPoint>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get benchmark security_id
    let benchmark_security_id: i64 = conn.query_row(
        "SELECT security_id FROM pp_benchmark WHERE id = ?1",
        [benchmark_id],
        |row| row.get(0)
    ).map_err(|e| format!("Benchmark not found: {}", e))?;

    // Get data
    let benchmark_prices = get_price_series(conn, benchmark_security_id, &start_date, &end_date)?;
    let portfolio_values = get_portfolio_values(conn, portfolio_id, &start_date, &end_date)?;

    if benchmark_prices.is_empty() || portfolio_values.is_empty() {
        return Ok(vec![]);
    }

    // Base values for percentage calculations
    let benchmark_base = benchmark_prices.first().map(|(_, p)| *p).unwrap_or(1.0);
    let portfolio_base = portfolio_values.first().map(|(_, v)| *v).unwrap_or(1.0);

    // Create lookup maps
    let benchmark_map: std::collections::HashMap<_, _> = benchmark_prices.into_iter().collect();
    let portfolio_map: std::collections::HashMap<_, _> = portfolio_values.into_iter().collect();

    // Collect all dates
    let mut all_dates: Vec<String> = benchmark_map.keys().chain(portfolio_map.keys()).cloned().collect();
    all_dates.sort();
    all_dates.dedup();

    // Build data points
    let mut data_points = Vec::new();
    let mut last_benchmark = benchmark_base;
    let mut last_portfolio = portfolio_base;

    for date in all_dates {
        let benchmark_value = benchmark_map.get(&date).copied().unwrap_or(last_benchmark);
        let portfolio_value = portfolio_map.get(&date).copied().unwrap_or(last_portfolio);

        last_benchmark = benchmark_value;
        last_portfolio = portfolio_value;

        data_points.push(BenchmarkDataPoint {
            date,
            portfolio_value,
            portfolio_return: ((portfolio_value / portfolio_base) - 1.0) * 100.0,
            benchmark_value,
            benchmark_return: ((benchmark_value / benchmark_base) - 1.0) * 100.0,
        });
    }

    Ok(data_points)
}

// Helper functions

fn get_price_series(
    conn: &rusqlite::Connection,
    security_id: i64,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<(String, f64)>, String> {
    let mut stmt = conn.prepare(
        "SELECT date, value / 100000000.0 as price
         FROM pp_price
         WHERE security_id = ?1 AND date >= ?2 AND date <= ?3
         ORDER BY date"
    ).map_err(|e| e.to_string())?;

    let prices = stmt.query_map(rusqlite::params![security_id, start_date, end_date], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    }).map_err(|e| e.to_string())?;

    prices.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn get_portfolio_values(
    conn: &rusqlite::Connection,
    portfolio_id: Option<i64>,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<(String, f64)>, String> {
    // Get distinct dates with prices
    let mut stmt = conn.prepare(
        "SELECT DISTINCT date FROM pp_price
         WHERE date >= ?1 AND date <= ?2
         ORDER BY date"
    ).map_err(|e| e.to_string())?;

    let dates: Vec<String> = stmt.query_map(rusqlite::params![start_date, end_date], |row| {
        row.get::<_, String>(0)
    }).map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>()
      .map_err(|e| e.to_string())?;

    let mut values = Vec::new();

    for date in dates {
        // Calculate portfolio value at this date
        let portfolio_filter = match portfolio_id {
            Some(id) => format!("AND t.owner_id = {}", id),
            None => String::new(),
        };

        let sql = format!(
            "SELECT
                SUM(
                    CASE
                        WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END
                ) / 100000000.0 * COALESCE(
                    (SELECT p.value / 100000000.0 FROM pp_price p
                     WHERE p.security_id = t.security_id AND p.date <= ?1
                     ORDER BY p.date DESC LIMIT 1),
                    0
                ) as value
             FROM pp_txn t
             WHERE t.owner_type = 'portfolio'
               AND t.date <= ?1
               AND t.shares IS NOT NULL
               {}
             GROUP BY t.security_id",
            portfolio_filter
        );

        let total: f64 = conn.query_row(&sql, [&date], |row| {
            row.get::<_, Option<f64>>(0).map(|v| v.unwrap_or(0.0))
        }).unwrap_or(0.0);

        if total > 0.0 {
            values.push((date, total));
        }
    }

    Ok(values)
}

fn calculate_daily_returns(series: &[(String, f64)]) -> Vec<(String, f64)> {
    let mut returns = Vec::new();
    for i in 1..series.len() {
        let prev = series[i - 1].1;
        let curr = series[i].1;
        if prev > 0.0 {
            returns.push((series[i].0.clone(), (curr / prev) - 1.0));
        }
    }
    returns
}

fn align_returns(
    portfolio: &[(String, f64)],
    benchmark: &[(String, f64)],
) -> (Vec<f64>, Vec<f64>) {
    let _portfolio_map: std::collections::HashMap<_, _> = portfolio.iter().cloned().collect();
    let benchmark_map: std::collections::HashMap<_, _> = benchmark.iter().cloned().collect();

    let mut aligned_portfolio = Vec::new();
    let mut aligned_benchmark = Vec::new();

    for (date, port_ret) in portfolio {
        if let Some(bench_ret) = benchmark_map.get(date) {
            aligned_portfolio.push(*port_ret);
            aligned_benchmark.push(*bench_ret);
        }
    }

    (aligned_portfolio, aligned_benchmark)
}

fn calculate_beta(portfolio: &[f64], benchmark: &[f64]) -> f64 {
    if portfolio.len() != benchmark.len() || portfolio.is_empty() {
        return 1.0;
    }

    let n = portfolio.len() as f64;
    let port_mean: f64 = portfolio.iter().sum::<f64>() / n;
    let bench_mean: f64 = benchmark.iter().sum::<f64>() / n;

    let mut covariance = 0.0;
    let mut bench_variance = 0.0;

    for i in 0..portfolio.len() {
        covariance += (portfolio[i] - port_mean) * (benchmark[i] - bench_mean);
        bench_variance += (benchmark[i] - bench_mean).powi(2);
    }

    if bench_variance > 0.0 {
        covariance / bench_variance
    } else {
        1.0
    }
}

fn calculate_correlation(portfolio: &[f64], benchmark: &[f64]) -> f64 {
    if portfolio.len() != benchmark.len() || portfolio.is_empty() {
        return 0.0;
    }

    let n = portfolio.len() as f64;
    let port_mean: f64 = portfolio.iter().sum::<f64>() / n;
    let bench_mean: f64 = benchmark.iter().sum::<f64>() / n;

    let mut covariance = 0.0;
    let mut port_variance = 0.0;
    let mut bench_variance = 0.0;

    for i in 0..portfolio.len() {
        let port_diff = portfolio[i] - port_mean;
        let bench_diff = benchmark[i] - bench_mean;
        covariance += port_diff * bench_diff;
        port_variance += port_diff.powi(2);
        bench_variance += bench_diff.powi(2);
    }

    let denominator = (port_variance * bench_variance).sqrt();
    if denominator > 0.0 {
        covariance / denominator
    } else {
        0.0
    }
}

fn calculate_tracking_error(portfolio: &[f64], benchmark: &[f64]) -> f64 {
    if portfolio.len() != benchmark.len() || portfolio.is_empty() {
        return 0.0;
    }

    let differences: Vec<f64> = portfolio.iter()
        .zip(benchmark.iter())
        .map(|(p, b)| p - b)
        .collect();

    let mean: f64 = differences.iter().sum::<f64>() / differences.len() as f64;
    let variance: f64 = differences.iter()
        .map(|d| (d - mean).powi(2))
        .sum::<f64>() / differences.len() as f64;

    variance.sqrt() * (252.0_f64).sqrt()  // Annualized
}

fn calculate_sharpe_ratio(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }

    let n = returns.len() as f64;
    let mean: f64 = returns.iter().sum::<f64>() / n;
    let variance: f64 = returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / n;
    let std_dev = variance.sqrt();

    if std_dev > 0.0 {
        // Annualized Sharpe (assuming 252 trading days)
        (mean * 252.0) / (std_dev * (252.0_f64).sqrt())
    } else {
        0.0
    }
}

fn calculate_max_drawdown(series: &[(String, f64)]) -> f64 {
    if series.is_empty() {
        return 0.0;
    }

    let mut peak = series[0].1;
    let mut max_drawdown = 0.0;

    for (_, value) in series {
        if *value > peak {
            peak = *value;
        }
        let drawdown = (peak - value) / peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    max_drawdown * 100.0  // As percentage
}
