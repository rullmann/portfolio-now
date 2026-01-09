//! Performance calculation module
//!
//! Implements Portfolio Performance's performance metrics:
//! - TTWROR (True Time-Weighted Rate of Return)
//! - IRR (Internal Rate of Return / Money-Weighted Return)
//!
//! Based on: https://github.com/portfolio-performance/portfolio
//! See: PerformanceIndex.java, IRR.java

use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Scale factors
const SHARES_SCALE: f64 = 100_000_000.0;
const AMOUNT_SCALE: f64 = 100.0;

/// A cash flow event (deposit, withdrawal, dividend, etc.)
#[derive(Debug, Clone)]
pub struct CashFlow {
    pub date: NaiveDate,
    pub amount: f64, // Positive = inflow, Negative = outflow
}

/// Portfolio value at a specific date
#[derive(Debug, Clone)]
pub struct PortfolioValue {
    pub date: NaiveDate,
    pub value: f64,
}

/// TTWROR calculation result
#[derive(Debug, Clone)]
pub struct TtwrorResult {
    /// Total return as decimal (0.1 = 10%)
    pub total_return: f64,
    /// Annualized return as decimal
    pub annualized_return: f64,
    /// Number of days in the period
    pub days: i64,
    /// Sub-period returns for detailed analysis
    pub periods: Vec<PeriodReturn>,
}

/// A sub-period return (between cash flows)
#[derive(Debug, Clone)]
pub struct PeriodReturn {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub start_value: f64,
    pub end_value: f64,
    pub cash_flow: f64,
    pub return_rate: f64,
}

/// IRR calculation result
#[derive(Debug, Clone)]
pub struct IrrResult {
    /// IRR as decimal (0.1 = 10%)
    pub irr: f64,
    /// Whether the calculation converged
    pub converged: bool,
    /// Number of iterations
    pub iterations: i32,
}

/// Calculate TTWROR for a portfolio
///
/// TTWROR Formula:
/// For each sub-period between cash flows:
///   r_i = (V_end - V_start - CF) / V_start
/// Total: (1 + r_1) × (1 + r_2) × ... × (1 + r_n) - 1
pub fn calculate_ttwror(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<TtwrorResult> {
    // Get portfolio values for each date
    let values = get_portfolio_values(conn, portfolio_id, start_date, end_date)?;

    if values.is_empty() {
        return Ok(TtwrorResult {
            total_return: 0.0,
            annualized_return: 0.0,
            days: 0,
            periods: vec![],
        });
    }

    // Get cash flows (deposits/withdrawals)
    let cash_flows = get_cash_flows(conn, portfolio_id, start_date, end_date)?;

    // Build a map of date -> cash flow
    let cf_map: HashMap<NaiveDate, f64> = cash_flows
        .iter()
        .map(|cf| (cf.date, cf.amount))
        .collect();

    // Calculate sub-period returns
    let mut periods: Vec<PeriodReturn> = Vec::new();
    let mut cumulative_return = 1.0;

    for i in 1..values.len() {
        let start_val = &values[i - 1];
        let end_val = &values[i];

        // Get cash flow on end date (if any)
        let cf = cf_map.get(&end_val.date).copied().unwrap_or(0.0);

        // Calculate period return
        // r = (V_end - CF) / V_start - 1
        // This assumes cash flow happens at end of day
        if start_val.value > 0.0 {
            let adjusted_end = end_val.value - cf;
            let period_return = adjusted_end / start_val.value;
            cumulative_return *= period_return;

            periods.push(PeriodReturn {
                start_date: start_val.date,
                end_date: end_val.date,
                start_value: start_val.value,
                end_value: end_val.value,
                cash_flow: cf,
                return_rate: period_return - 1.0,
            });
        }
    }

    let total_return = cumulative_return - 1.0;
    let days = (end_date - start_date).num_days();

    // Annualize: (1 + r)^(365/days) - 1
    let annualized_return = if days > 0 {
        (1.0 + total_return).powf(365.0 / days as f64) - 1.0
    } else {
        0.0
    };

    Ok(TtwrorResult {
        total_return,
        annualized_return,
        days,
        periods,
    })
}

/// Calculate IRR (Internal Rate of Return)
///
/// Uses Newton-Raphson method to find the rate r where NPV = 0
/// NPV = Σ CF_i / (1 + r)^t_i = 0
///
/// For investments, CF_0 is typically negative (initial investment)
/// and CF_n is typically positive (final value + dividends)
pub fn calculate_irr(cash_flows: &[CashFlow], final_value: f64, final_date: NaiveDate) -> Result<IrrResult> {
    if cash_flows.is_empty() {
        return Ok(IrrResult {
            irr: 0.0,
            converged: true,
            iterations: 0,
        });
    }

    let first_date = cash_flows.first().unwrap().date;

    // Create cash flow series with final value
    let mut cf_series: Vec<(f64, f64)> = cash_flows
        .iter()
        .map(|cf| {
            let years = (cf.date - first_date).num_days() as f64 / 365.0;
            (-cf.amount, years) // Invert: deposits are negative for IRR
        })
        .collect();

    // Add final value as positive cash flow
    let final_years = (final_date - first_date).num_days() as f64 / 365.0;
    cf_series.push((final_value, final_years));

    // Newton-Raphson iteration
    let mut rate = 0.1; // Initial guess: 10%
    let max_iterations = 100;
    let tolerance = 1e-10;

    for iteration in 0..max_iterations {
        let (npv, dnpv) = calculate_npv_and_derivative(&cf_series, rate);

        if dnpv.abs() < tolerance {
            // Derivative too small, can't continue
            return Ok(IrrResult {
                irr: rate,
                converged: false,
                iterations: iteration,
            });
        }

        let new_rate = rate - npv / dnpv;

        if (new_rate - rate).abs() < tolerance {
            return Ok(IrrResult {
                irr: new_rate,
                converged: true,
                iterations: iteration,
            });
        }

        rate = new_rate;

        // Bound the rate to reasonable values
        if rate < -0.99 {
            rate = -0.99;
        } else if rate > 10.0 {
            rate = 10.0;
        }
    }

    Ok(IrrResult {
        irr: rate,
        converged: false,
        iterations: max_iterations,
    })
}

/// Calculate NPV and its derivative for Newton-Raphson
fn calculate_npv_and_derivative(cash_flows: &[(f64, f64)], rate: f64) -> (f64, f64) {
    let mut npv = 0.0;
    let mut dnpv = 0.0;

    for (cf, years) in cash_flows {
        let discount = (1.0 + rate).powf(*years);
        npv += cf / discount;

        // Derivative: d/dr [cf / (1+r)^t] = -t * cf / (1+r)^(t+1)
        if discount > 0.0 {
            dnpv -= years * cf / (discount * (1.0 + rate));
        }
    }

    (npv, dnpv)
}

/// Get portfolio values for each date in the range
fn get_portfolio_values(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<PortfolioValue>> {
    // Get holdings at start date by summing transactions up to that date
    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    // Get all price dates in range
    let dates_sql = format!(
        r#"
        SELECT DISTINCT date
        FROM pp_price
        WHERE date >= ? AND date <= ?
        ORDER BY date
        "#
    );

    let mut dates: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare(&dates_sql)?;
        let rows = stmt.query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| row.get(0),
        )?;
        for row in rows {
            if let Ok(date) = row {
                dates.push(date);
            }
        }
    }

    // For each date, calculate portfolio value
    let mut values: Vec<PortfolioValue> = Vec::new();

    for date_str in dates {
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;

        // Get holdings as of this date
        let holdings_sql = format!(
            r#"
            SELECT
                t.security_id,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) as net_shares
            FROM pp_txn t
            WHERE t.owner_type = 'portfolio'
              AND t.shares IS NOT NULL
              AND date(t.date) <= ?
              {}
            GROUP BY t.security_id
            HAVING net_shares > 0
            "#,
            portfolio_filter
        );

        let mut holdings: Vec<(i64, i64)> = Vec::new();
        {
            let mut stmt = conn.prepare(&holdings_sql)?;
            let rows = stmt.query_map([&date_str], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                if let Ok(holding) = row {
                    holdings.push(holding);
                }
            }
        }

        if holdings.is_empty() {
            continue;
        }

        // Get prices for each security on this date (or latest before)
        let mut total_value = 0.0;
        for (security_id, share_count) in holdings {
            let price_sql = r#"
                SELECT value FROM pp_price
                WHERE security_id = ? AND date <= ?
                ORDER BY date DESC
                LIMIT 1
            "#;

            let price: Option<i64> = conn
                .query_row(price_sql, params![security_id, date_str], |row| row.get(0))
                .ok();

            if let Some(p) = price {
                let shares_f = share_count as f64 / SHARES_SCALE;
                let price_f = p as f64 / 100_000_000.0; // prices are scaled by 10^8
                total_value += shares_f * price_f;
            }
        }

        if total_value > 0.0 {
            values.push(PortfolioValue {
                date,
                value: total_value,
            });
        }
    }

    Ok(values)
}

/// Get cash flows (deposits and withdrawals) for the period
fn get_cash_flows(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<CashFlow>> {
    let mut cash_flows: Vec<CashFlow> = Vec::new();

    // If portfolio_id is specified, only look at transactions for that portfolio's reference account
    // Otherwise, look at all accounts
    if let Some(pid) = portfolio_id {
        let sql = r#"
            SELECT t.date, t.txn_type, t.amount
            FROM pp_txn t
            JOIN pp_portfolio p ON p.reference_account_id = t.owner_id
            WHERE t.owner_type = 'account'
              AND t.txn_type IN ('DEPOSIT', 'REMOVAL')
              AND p.id = ?1
              AND date(t.date) >= ?2 AND date(t.date) <= ?3
            ORDER BY t.date
        "#;

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(
            params![pid, start_date.to_string(), end_date.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;

        for row in rows.flatten() {
            let (date_str, txn_type, amount) = row;
            if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                let amount_f = amount as f64 / AMOUNT_SCALE;
                let cf_amount = match txn_type.as_str() {
                    "DEPOSIT" => amount_f,
                    "REMOVAL" => -amount_f,
                    _ => 0.0,
                };
                if cf_amount != 0.0 {
                    cash_flows.push(CashFlow { date, amount: cf_amount });
                }
            }
        }
    } else {
        let sql = r#"
            SELECT date, txn_type, amount
            FROM pp_txn
            WHERE owner_type = 'account'
              AND txn_type IN ('DEPOSIT', 'REMOVAL')
              AND date(date) >= ?1 AND date(date) <= ?2
            ORDER BY date
        "#;

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;

        for row in rows.flatten() {
            let (date_str, txn_type, amount) = row;
            if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                let amount_f = amount as f64 / AMOUNT_SCALE;
                let cf_amount = match txn_type.as_str() {
                    "DEPOSIT" => amount_f,
                    "REMOVAL" => -amount_f,
                    _ => 0.0,
                };
                if cf_amount != 0.0 {
                    cash_flows.push(CashFlow { date, amount: cf_amount });
                }
            }
        }
    }

    Ok(cash_flows)
}

/// Calculate performance for all holdings
pub fn calculate_portfolio_performance(
    conn: &Connection,
    portfolio_id: Option<i64>,
) -> Result<(TtwrorResult, IrrResult)> {
    // Get date range from transactions
    let (start_date, end_date) = get_transaction_date_range(conn, portfolio_id)?;

    let ttwror = calculate_ttwror(conn, portfolio_id, start_date, end_date)?;

    // Get cash flows for IRR
    let cash_flows = get_cash_flows(conn, portfolio_id, start_date, end_date)?;

    // Get current portfolio value
    let current_value = get_current_portfolio_value(conn, portfolio_id)?;

    let irr = calculate_irr(&cash_flows, current_value, end_date)?;

    Ok((ttwror, irr))
}

/// Get the date range of transactions
fn get_transaction_date_range(
    conn: &Connection,
    portfolio_id: Option<i64>,
) -> Result<(NaiveDate, NaiveDate)> {
    let portfolio_filter = portfolio_id
        .map(|id| format!("AND owner_id = {}", id))
        .unwrap_or_default();

    let sql = format!(
        r#"
        SELECT MIN(date), MAX(date)
        FROM pp_txn
        WHERE owner_type = 'portfolio'
          {}
        "#,
        portfolio_filter
    );

    let (min_date, max_date): (Option<String>, Option<String>) =
        conn.query_row(&sql, [], |row| Ok((row.get(0)?, row.get(1)?)))?;

    let start = min_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());

    let end = max_date
        .and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    Ok((start, end))
}

/// Get current portfolio value
fn get_current_portfolio_value(conn: &Connection, portfolio_id: Option<i64>) -> Result<f64> {
    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    let holdings_sql = format!(
        r#"
        SELECT
            t.security_id,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares
        FROM pp_txn t
        WHERE t.owner_type = 'portfolio'
          AND t.shares IS NOT NULL
          {}
        GROUP BY t.security_id
        HAVING net_shares > 0
        "#,
        portfolio_filter
    );

    let mut total_value = 0.0;

    {
        let mut stmt = conn.prepare(&holdings_sql)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            if let Ok((security_id, share_count)) = row {
                // Get latest price
                let price_sql = r#"
                    SELECT value FROM pp_latest_price WHERE security_id = ?
                "#;

                let price: Option<i64> = conn
                    .query_row(price_sql, [security_id], |row| row.get(0))
                    .ok();

                if let Some(p) = price {
                    let shares_f = share_count as f64 / SHARES_SCALE;
                    let price_f = p as f64 / 100_000_000.0;
                    total_value += shares_f * price_f;
                }
            }
        }
    }

    Ok(total_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irr_simple() {
        // Invest 1000, get back 1100 after 1 year = 10% return
        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                amount: 1000.0,
            },
        ];

        let result = calculate_irr(
            &cash_flows,
            1100.0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ).unwrap();

        assert!(result.converged);
        assert!((result.irr - 0.1).abs() < 0.001); // ~10%
    }

    #[test]
    fn test_irr_multiple_flows() {
        // Invest 1000 at start, 500 after 6 months, get 1700 after 1 year
        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                amount: 1000.0,
            },
            CashFlow {
                date: NaiveDate::from_ymd_opt(2023, 7, 1).unwrap(),
                amount: 500.0,
            },
        ];

        let result = calculate_irr(
            &cash_flows,
            1700.0,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ).unwrap();

        assert!(result.converged);
        // Return should be positive since we got back more than invested
        assert!(result.irr > 0.0);
    }
}
