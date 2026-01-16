//! Portfolio Optimization Module
//!
//! Implements Modern Portfolio Theory (Markowitz):
//! - Correlation Matrix calculation
//! - Efficient Frontier computation
//! - Minimum Variance Portfolio
//! - Maximum Sharpe Ratio Portfolio
//! - Portfolio Risk/Return analysis

use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::HashMap;
use tauri::command;

use crate::db;

const PRICE_SCALE: f64 = 100_000_000.0;

// ============================================================================
// Data Types
// ============================================================================

/// Correlation between two securities
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationPair {
    pub security1_id: i64,
    pub security1_name: String,
    pub security2_id: i64,
    pub security2_name: String,
    pub correlation: f64,
}

/// Full correlation matrix result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationMatrix {
    pub securities: Vec<SecurityInfo>,
    pub matrix: Vec<Vec<f64>>,
    pub pairs: Vec<CorrelationPair>,
}

/// Security info for matrix
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityInfo {
    pub id: i64,
    pub name: String,
    pub ticker: Option<String>,
}

/// A point on the efficient frontier
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EfficientFrontierPoint {
    pub expected_return: f64,
    pub volatility: f64,
    pub sharpe_ratio: f64,
    pub weights: HashMap<i64, f64>,
}

/// Efficient frontier result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EfficientFrontier {
    pub points: Vec<EfficientFrontierPoint>,
    pub min_variance_portfolio: EfficientFrontierPoint,
    pub max_sharpe_portfolio: EfficientFrontierPoint,
    pub current_portfolio: EfficientFrontierPoint,
    pub securities: Vec<SecurityInfo>,
}

/// Security statistics for optimization
#[derive(Debug, Clone)]
struct SecurityStats {
    id: i64,
    name: String,
    expected_return: f64,
    volatility: f64,
    returns: Vec<f64>,
}

// ============================================================================
// Commands
// ============================================================================

/// Calculate correlation matrix for portfolio holdings
#[command]
pub fn calculate_correlation_matrix(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<CorrelationMatrix, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let start = start_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| {
            let now = chrono::Utc::now().date_naive();
            now - chrono::Duration::days(365)
        });

    let end = end_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    compute_correlation_matrix(conn, portfolio_id, start, end).map_err(|e| e.to_string())
}

/// Calculate efficient frontier for portfolio
#[command]
pub fn calculate_efficient_frontier(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
    risk_free_rate: Option<f64>,
    num_points: Option<usize>,
) -> Result<EfficientFrontier, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let rf_rate = risk_free_rate.unwrap_or(0.03);
    let points = num_points.unwrap_or(50);

    let start = start_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| {
            let now = chrono::Utc::now().date_naive();
            now - chrono::Duration::days(365)
        });

    let end = end_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    compute_efficient_frontier(conn, portfolio_id, start, end, rf_rate, points)
        .map_err(|e| e.to_string())
}

/// Get optimal portfolio weights for target return
#[command]
pub fn get_optimal_weights(
    portfolio_id: Option<i64>,
    target_return: f64,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<HashMap<i64, f64>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let start = start_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| {
            let now = chrono::Utc::now().date_naive();
            now - chrono::Duration::days(365)
        });

    let end = end_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    compute_optimal_weights(conn, portfolio_id, target_return, start, end)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Implementation
// ============================================================================

/// Get held securities with price history
fn get_held_securities(
    conn: &Connection,
    portfolio_id: Option<i64>,
) -> Result<Vec<(i64, String, Option<String>)>> {
    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    let sql = format!(
        r#"
        SELECT DISTINCT s.id, s.name, s.ticker
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        WHERE t.owner_type = 'portfolio'
          AND t.shares IS NOT NULL
          {}
        GROUP BY s.id
        HAVING SUM(CASE
            WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
            WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
            ELSE 0
        END) > 0
        "#,
        portfolio_filter
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    let mut securities = Vec::new();
    for row in rows.flatten() {
        securities.push(row);
    }

    Ok(securities)
}

/// Get daily returns for a security
fn get_security_returns(
    conn: &Connection,
    security_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<(String, f64)>> {
    let sql = r#"
        SELECT date(date) as d, value
        FROM pp_price
        WHERE security_id = ?
          AND date(date) >= ?
          AND date(date) <= ?
        ORDER BY d
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(
        params![security_id, start_date.to_string(), end_date.to_string()],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )?;

    let mut prices: Vec<(String, f64)> = Vec::new();
    for row in rows.flatten() {
        let (date, price) = row;
        prices.push((date, price as f64 / PRICE_SCALE));
    }

    // Calculate returns
    let mut returns: Vec<(String, f64)> = Vec::new();
    for i in 1..prices.len() {
        let prev_price = prices[i - 1].1;
        let curr_price = prices[i].1;
        if prev_price > 0.0 {
            let ret = (curr_price - prev_price) / prev_price;
            returns.push((prices[i].0.clone(), ret));
        }
    }

    Ok(returns)
}

/// Calculate correlation between two return series
fn calculate_correlation(returns1: &[f64], returns2: &[f64]) -> f64 {
    if returns1.len() != returns2.len() || returns1.is_empty() {
        return 0.0;
    }

    let n = returns1.len() as f64;
    let mean1 = returns1.iter().sum::<f64>() / n;
    let mean2 = returns2.iter().sum::<f64>() / n;

    let mut cov = 0.0;
    let mut var1 = 0.0;
    let mut var2 = 0.0;

    for i in 0..returns1.len() {
        let d1 = returns1[i] - mean1;
        let d2 = returns2[i] - mean2;
        cov += d1 * d2;
        var1 += d1 * d1;
        var2 += d2 * d2;
    }

    if var1 > 0.0 && var2 > 0.0 {
        cov / (var1.sqrt() * var2.sqrt())
    } else {
        0.0
    }
}

/// Compute full correlation matrix
fn compute_correlation_matrix(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<CorrelationMatrix> {
    let held_securities = get_held_securities(conn, portfolio_id)?;

    if held_securities.is_empty() {
        return Ok(CorrelationMatrix {
            securities: vec![],
            matrix: vec![],
            pairs: vec![],
        });
    }

    // Get returns for each security
    let mut all_returns: HashMap<i64, HashMap<String, f64>> = HashMap::new();
    let mut securities: Vec<SecurityInfo> = Vec::new();

    for (id, name, ticker) in &held_securities {
        let returns = get_security_returns(conn, *id, start_date, end_date)?;
        let return_map: HashMap<String, f64> = returns.into_iter().collect();
        all_returns.insert(*id, return_map);
        securities.push(SecurityInfo {
            id: *id,
            name: name.clone(),
            ticker: ticker.clone(),
        });
    }

    // Find common dates
    let mut common_dates: Vec<String> = Vec::new();
    if let Some(first_returns) = all_returns.values().next() {
        for date in first_returns.keys() {
            let all_have = all_returns.values().all(|r| r.contains_key(date));
            if all_have {
                common_dates.push(date.clone());
            }
        }
    }
    common_dates.sort();

    // Build aligned return vectors
    let mut aligned_returns: HashMap<i64, Vec<f64>> = HashMap::new();
    for (id, returns) in &all_returns {
        let vec: Vec<f64> = common_dates
            .iter()
            .filter_map(|d| returns.get(d).copied())
            .collect();
        aligned_returns.insert(*id, vec);
    }

    // Calculate correlation matrix
    let n = securities.len();
    let mut matrix = vec![vec![0.0; n]; n];
    let mut pairs: Vec<CorrelationPair> = Vec::new();

    for i in 0..n {
        for j in 0..n {
            let id1 = securities[i].id;
            let id2 = securities[j].id;

            if i == j {
                matrix[i][j] = 1.0;
            } else {
                let returns1 = aligned_returns.get(&id1).map(|v| v.as_slice()).unwrap_or(&[]);
                let returns2 = aligned_returns.get(&id2).map(|v| v.as_slice()).unwrap_or(&[]);
                let corr = calculate_correlation(returns1, returns2);
                matrix[i][j] = corr;

                if i < j {
                    pairs.push(CorrelationPair {
                        security1_id: id1,
                        security1_name: securities[i].name.clone(),
                        security2_id: id2,
                        security2_name: securities[j].name.clone(),
                        correlation: corr,
                    });
                }
            }
        }
    }

    // Sort pairs by absolute correlation (most correlated first)
    pairs.sort_by(|a, b| b.correlation.abs().partial_cmp(&a.correlation.abs()).unwrap());

    Ok(CorrelationMatrix {
        securities,
        matrix,
        pairs,
    })
}

/// Compute efficient frontier using Monte Carlo simulation
/// (Simplified approach without matrix operations library)
fn compute_efficient_frontier(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    risk_free_rate: f64,
    num_points: usize,
) -> Result<EfficientFrontier> {
    let held_securities = get_held_securities(conn, portfolio_id)?;

    if held_securities.len() < 2 {
        return Err(anyhow::anyhow!(
            "Need at least 2 securities for efficient frontier"
        ));
    }

    // Get security statistics
    let mut stats: Vec<SecurityStats> = Vec::new();
    let mut all_returns: HashMap<i64, Vec<f64>> = HashMap::new();

    for (id, name, _) in &held_securities {
        let returns = get_security_returns(conn, *id, start_date, end_date)?;
        let return_values: Vec<f64> = returns.iter().map(|(_, r)| *r).collect();

        if return_values.is_empty() {
            continue;
        }

        let mean_return = return_values.iter().sum::<f64>() / return_values.len() as f64;
        let variance = return_values
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f64>()
            / return_values.len() as f64;
        let volatility = variance.sqrt() * (252.0_f64).sqrt();
        let annualized_return = mean_return * 252.0;

        all_returns.insert(*id, return_values.clone());

        stats.push(SecurityStats {
            id: *id,
            name: name.clone(),
            expected_return: annualized_return,
            volatility,
            returns: return_values,
        });
    }

    if stats.len() < 2 {
        return Err(anyhow::anyhow!(
            "Need at least 2 securities with price data"
        ));
    }

    let securities: Vec<SecurityInfo> = stats
        .iter()
        .map(|s| SecurityInfo {
            id: s.id,
            name: s.name.clone(),
            ticker: None,
        })
        .collect();

    // Calculate covariance matrix
    let n = stats.len();
    let mut cov_matrix = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..n {
            let returns_i = &stats[i].returns;
            let returns_j = &stats[j].returns;

            let min_len = returns_i.len().min(returns_j.len());
            if min_len == 0 {
                continue;
            }

            let mean_i = returns_i[..min_len].iter().sum::<f64>() / min_len as f64;
            let mean_j = returns_j[..min_len].iter().sum::<f64>() / min_len as f64;

            let cov: f64 = (0..min_len)
                .map(|k| (returns_i[k] - mean_i) * (returns_j[k] - mean_j))
                .sum::<f64>()
                / min_len as f64;

            // Annualize covariance
            cov_matrix[i][j] = cov * 252.0;
        }
    }

    // Monte Carlo simulation to generate efficient frontier
    let mut portfolios: Vec<EfficientFrontierPoint> = Vec::new();
    let num_simulations = num_points * 100;

    for _ in 0..num_simulations {
        // Generate random weights
        let mut weights: Vec<f64> = (0..n).map(|_| rand_simple()).collect();
        let sum: f64 = weights.iter().sum();
        for w in &mut weights {
            *w /= sum;
        }

        // Calculate portfolio return
        let port_return: f64 = weights
            .iter()
            .zip(stats.iter())
            .map(|(w, s)| w * s.expected_return)
            .sum();

        // Calculate portfolio variance
        let mut port_variance = 0.0;
        for i in 0..n {
            for j in 0..n {
                port_variance += weights[i] * weights[j] * cov_matrix[i][j];
            }
        }
        let port_volatility = port_variance.sqrt();

        // Sharpe ratio
        let sharpe = if port_volatility > 0.0 {
            (port_return - risk_free_rate) / port_volatility
        } else {
            0.0
        };

        let weight_map: HashMap<i64, f64> = stats
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id, weights[i]))
            .collect();

        portfolios.push(EfficientFrontierPoint {
            expected_return: port_return,
            volatility: port_volatility,
            sharpe_ratio: sharpe,
            weights: weight_map,
        });
    }

    // Find min variance and max sharpe portfolios
    let min_variance = portfolios
        .iter()
        .min_by(|a, b| a.volatility.partial_cmp(&b.volatility).unwrap())
        .cloned()
        .unwrap();

    let max_sharpe = portfolios
        .iter()
        .max_by(|a, b| a.sharpe_ratio.partial_cmp(&b.sharpe_ratio).unwrap())
        .cloned()
        .unwrap();

    // Get current portfolio weights
    let current_weights = get_current_weights(conn, portfolio_id, &stats)?;
    let current_return: f64 = current_weights
        .iter()
        .map(|(id, w)| {
            stats
                .iter()
                .find(|s| s.id == *id)
                .map(|s| w * s.expected_return)
                .unwrap_or(0.0)
        })
        .sum();

    let mut current_variance = 0.0;
    for (i, si) in stats.iter().enumerate() {
        for (j, sj) in stats.iter().enumerate() {
            let wi = current_weights.get(&si.id).unwrap_or(&0.0);
            let wj = current_weights.get(&sj.id).unwrap_or(&0.0);
            current_variance += wi * wj * cov_matrix[i][j];
        }
    }
    let current_volatility = current_variance.sqrt();

    let current_sharpe = if current_volatility > 0.0 {
        (current_return - risk_free_rate) / current_volatility
    } else {
        0.0
    };

    let current_portfolio = EfficientFrontierPoint {
        expected_return: current_return,
        volatility: current_volatility,
        sharpe_ratio: current_sharpe,
        weights: current_weights,
    };

    // Select efficient frontier points (upper envelope)
    portfolios.sort_by(|a, b| a.volatility.partial_cmp(&b.volatility).unwrap());

    let mut frontier_points: Vec<EfficientFrontierPoint> = Vec::new();
    let mut max_return = f64::NEG_INFINITY;

    for p in portfolios {
        if p.expected_return > max_return {
            frontier_points.push(p.clone());
            max_return = p.expected_return;
        }
    }

    // Reduce to requested number of points
    let step = frontier_points.len() / num_points.max(1);
    let final_points: Vec<EfficientFrontierPoint> = if step > 0 {
        frontier_points
            .into_iter()
            .step_by(step.max(1))
            .take(num_points)
            .collect()
    } else {
        frontier_points
    };

    Ok(EfficientFrontier {
        points: final_points,
        min_variance_portfolio: min_variance,
        max_sharpe_portfolio: max_sharpe,
        current_portfolio,
        securities,
    })
}

/// Get current portfolio weights
fn get_current_weights(
    conn: &Connection,
    portfolio_id: Option<i64>,
    stats: &[SecurityStats],
) -> Result<HashMap<i64, f64>> {
    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    // Get holdings with values
    let sql = format!(
        r#"
        SELECT
            t.security_id,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares,
            lp.value as price
        FROM pp_txn t
        LEFT JOIN pp_latest_price lp ON lp.security_id = t.security_id
        WHERE t.owner_type = 'portfolio'
          AND t.shares IS NOT NULL
          {}
        GROUP BY t.security_id
        HAVING net_shares > 0
        "#,
        portfolio_filter
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<i64>>(2)?,
        ))
    })?;

    let mut values: HashMap<i64, f64> = HashMap::new();
    let mut total_value = 0.0;

    for row in rows.flatten() {
        let (security_id, shares, price_opt) = row;

        // Only include securities in our stats list
        if !stats.iter().any(|s| s.id == security_id) {
            continue;
        }

        if let Some(price) = price_opt {
            let shares_f = shares as f64 / 100_000_000.0;
            let price_f = price as f64 / PRICE_SCALE;
            let value = shares_f * price_f;
            values.insert(security_id, value);
            total_value += value;
        }
    }

    // Convert to weights
    let weights: HashMap<i64, f64> = if total_value > 0.0 {
        values
            .into_iter()
            .map(|(id, v)| (id, v / total_value))
            .collect()
    } else {
        HashMap::new()
    };

    Ok(weights)
}

/// Compute optimal weights for a target return
fn compute_optimal_weights(
    conn: &Connection,
    portfolio_id: Option<i64>,
    target_return: f64,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<HashMap<i64, f64>> {
    // Use efficient frontier to find closest portfolio
    let frontier =
        compute_efficient_frontier(conn, portfolio_id, start_date, end_date, 0.03, 100)?;

    // Find point closest to target return
    let closest = frontier
        .points
        .iter()
        .min_by(|a, b| {
            let diff_a = (a.expected_return - target_return).abs();
            let diff_b = (b.expected_return - target_return).abs();
            diff_a.partial_cmp(&diff_b).unwrap()
        })
        .cloned();

    closest.map(|p| p.weights).ok_or_else(|| {
        anyhow::anyhow!("Could not find optimal weights for target return")
    })
}

/// Simple pseudo-random number generator (0 to 1)
/// Used for Monte Carlo simulation
fn rand_simple() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    static mut SEED: u64 = 0;

    unsafe {
        if SEED == 0 {
            SEED = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(12345);
        }
        // LCG parameters
        SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
        ((SEED >> 16) & 0x7FFF) as f64 / 32767.0
    }
}
