//! Performance calculation module
//!
//! Implements Portfolio Performance's performance metrics:
//! - TTWROR (True Time-Weighted Rate of Return)
//! - IRR (Internal Rate of Return / Money-Weighted Return)
//!
//! ## TTWROR Formula (True Time-Weighted Rate of Return)
//!
//! TTWROR measures portfolio performance independent of cash flows.
//! It chains sub-period returns between each cash flow event:
//!
//! ```text
//! For each sub-period i between cash flows:
//!   r_i = (V_end - CF) / V_start - 1
//!
//! Where:
//!   V_end   = Portfolio value at end of sub-period
//!   V_start = Portfolio value at start of sub-period
//!   CF      = Cash flow at end of sub-period (positive = deposit, negative = withdrawal)
//!
//! Total TTWROR = (1 + r_1) × (1 + r_2) × ... × (1 + r_n) - 1
//!
//! Annualized TTWROR = (1 + TTWROR)^(365/days) - 1
//! ```
//!
//! ## IRR Formula (Internal Rate of Return)
//!
//! IRR finds the discount rate where Net Present Value (NPV) equals zero.
//! Uses Newton-Raphson iterative method:
//!
//! ```text
//! NPV = Σ CF_i / (1 + r)^t_i = 0
//!
//! Where:
//!   CF_i = Cash flow at time i (negative = investment, positive = withdrawal)
//!   t_i  = Time in years from first cash flow
//!   r    = IRR (the rate we're solving for)
//!
//! Newton-Raphson iteration:
//!   r_new = r_old - NPV(r_old) / NPV'(r_old)
//!
//! Where NPV' is the derivative:
//!   NPV'(r) = Σ -t_i × CF_i / (1 + r)^(t_i + 1)
//! ```
//!
//! ## Cash Flow Sign Convention
//!
//! This module uses the following sign convention:
//! - Positive cash flow = Money INTO portfolio (investment/deposit)
//! - Negative cash flow = Money OUT OF portfolio (withdrawal/sale proceeds)
//!
//! For IRR calculation, signs are inverted internally since NPV convention is:
//! - Negative = Investment (money paid out)
//! - Positive = Return (money received)
//!
//! Based on: https://github.com/portfolio-performance/portfolio
//! See: PerformanceIndex.java, IRR.java
//!
//! ## ⚠️ KNOWN ISSUES (2026-01)
//!
//! **IRR calculation returns incorrect values (e.g., +1000%)**
//!
//! Implemented fixes (not yet working correctly):
//! - NAV includes cash balances from linked accounts
//! - Cash flows converted to base currency
//! - `get_cash_flows()` searches all linked accounts (not just reference_account)
//! - Fallback to BUY/SELL if no DEPOSIT/REMOVAL found
//! - End-of-day cash flow convention for TTWROR
//! - Flow-adjusted returns for risk metrics
//!
//! Suspected issues:
//! - Scaling factor mismatch (AMOUNT_SCALE vs actual data)
//! - Missing cash flows from certain transaction types
//! - GROSS_VALUE unit type not found in pp_txn_unit
//!
//! Debug hints:
//! - Check log output for "IRR: Found X cash flows"
//! - Verify DEPOSIT/REMOVAL exist in pp_txn with owner_type='account'
//! - Compare sum(cash_flows) vs current_value

use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::{params, Connection};

/// Parse date string flexibly - handles both "YYYY-MM-DD" and "YYYY-MM-DD HH:MM:SS" formats
fn parse_date_flexible(date_str: &str) -> Option<NaiveDate> {
    // Try date-only format first: "2024-01-15"
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        // Then try with time: "2024-01-15 00:00:00"
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
        // Then try ISO8601: "2024-01-15T00:00:00"
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
}

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

/// Calculate True Time-Weighted Rate of Return (TTWROR) for a portfolio
///
/// TTWROR measures portfolio performance independent of external cash flows by
/// chaining sub-period returns between each cash flow event:
///
/// ```text
/// For each sub-period i between cash flows:
///   r_i = V_end / (V_start + CF) - 1
///
/// Total TTWROR = ∏(1 + r_i) - 1  (geometric chaining)
/// Annualized = (1 + TTWROR)^(365/days) - 1
/// ```
///
/// This correctly isolates investment performance from timing of deposits/withdrawals.
pub fn calculate_ttwror(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<TtwrorResult> {
    let days = (end_date - start_date).num_days();

    if days <= 0 {
        return Ok(TtwrorResult {
            total_return: 0.0,
            annualized_return: 0.0,
            days: 0,
            periods: vec![],
        });
    }

    // Get portfolio value history (daily values where we have price data)
    let valuations = get_ttwror_portfolio_values(conn, portfolio_id, start_date, end_date)?;

    if valuations.len() < 2 {
        log::warn!("TTWROR: Not enough valuation data points ({}), falling back to simple return", valuations.len());
        return calculate_ttwror_simple_fallback(conn, portfolio_id, start_date, end_date, days);
    }

    // Get external cash flows (DEPOSIT/REMOVAL only)
    let cash_flows = get_cash_flows(conn, portfolio_id, start_date, end_date)?;

    log::info!(
        "TTWROR: {} valuations, {} cash flows, {} days",
        valuations.len(), cash_flows.len(), days
    );

    // Calculate TTWROR using geometric chaining of sub-period returns
    let (total_return, periods) = calculate_ttwror_from_data(&valuations, &cash_flows);

    // Annualize: (1 + r)^(365/days) - 1
    let annualized_return = if days > 0 && total_return > -1.0 {
        (1.0 + total_return).powf(365.0 / days as f64) - 1.0
    } else {
        0.0
    };

    log::info!(
        "TTWROR result: total={:.4}% ({} periods), annualized={:.4}%",
        total_return * 100.0,
        periods.len(),
        annualized_return * 100.0
    );

    Ok(TtwrorResult {
        total_return,
        annualized_return,
        days,
        periods,
    })
}

/// Calculate TTWROR for multiple portfolios by aggregating valuations and cash flows.
pub fn calculate_ttwror_for_portfolios(
    conn: &Connection,
    portfolio_ids: &[i64],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<TtwrorResult> {
    let days = (end_date - start_date).num_days();

    if days <= 0 || portfolio_ids.is_empty() {
        return Ok(TtwrorResult {
            total_return: 0.0,
            annualized_return: 0.0,
            days: 0,
            periods: vec![],
        });
    }

    let mut aggregated_values: std::collections::BTreeMap<NaiveDate, f64> = std::collections::BTreeMap::new();
    let mut cash_flows: Vec<CashFlow> = Vec::new();

    for pid in portfolio_ids {
        let values = get_ttwror_portfolio_values(conn, Some(*pid), start_date, end_date)?;
        for (date, value) in values {
            *aggregated_values.entry(date).or_insert(0.0) += value;
        }

        let flows = get_cash_flows(conn, Some(*pid), start_date, end_date)?;
        cash_flows.extend(flows);
    }

    let valuations: Vec<(NaiveDate, f64)> = aggregated_values.into_iter().collect();

    if valuations.len() < 2 {
        let mut total_value = 0.0;
        let mut total_cost_basis = 0.0;

        for pid in portfolio_ids {
            total_value += get_portfolio_value_at_date_with_currency(conn, Some(*pid), end_date)?;
            total_cost_basis += get_total_cost_basis_with_currency(conn, Some(*pid))?;
        }

        let total_return = if total_cost_basis > 0.0 {
            (total_value - total_cost_basis) / total_cost_basis
        } else {
            0.0
        };

        let annualized_return = if days > 0 && total_return > -1.0 {
            (1.0 + total_return).powf(365.0 / days as f64) - 1.0
        } else {
            0.0
        };

        return Ok(TtwrorResult {
            total_return,
            annualized_return,
            days,
            periods: vec![PeriodReturn {
                start_date,
                end_date,
                start_value: total_cost_basis,
                end_value: total_value,
                cash_flow: 0.0,
                return_rate: total_return,
            }],
        });
    }

    cash_flows.sort_by(|a, b| a.date.cmp(&b.date));
    let (total_return, periods) = calculate_ttwror_from_data(&valuations, &cash_flows);

    let annualized_return = if days > 0 && total_return > -1.0 {
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

/// Calculate TTWROR from valuation and cash flow data
///
/// Algorithm:
/// 1. For each cash flow, find the closest valuation before and after
/// 2. Calculate sub-period return: r = V_end / (V_start + CF) - 1
/// 3. Chain all sub-period returns: ∏(1 + r_i) - 1
fn calculate_ttwror_from_data(
    valuations: &[(NaiveDate, f64)],
    cash_flows: &[CashFlow],
) -> (f64, Vec<PeriodReturn>) {
    if valuations.len() < 2 {
        return (0.0, vec![]);
    }

    let mut periods = Vec::new();
    let mut cumulative_return = 1.0;

    // Create a list of all relevant dates (cash flow dates + start + end)
    let mut cf_dates: Vec<NaiveDate> = cash_flows.iter().map(|cf| cf.date).collect();

    // If no cash flows, just calculate simple return over entire period
    if cf_dates.is_empty() {
        let start_value = valuations.first().unwrap().1;
        let end_value = valuations.last().unwrap().1;

        if start_value > 0.0 {
            let period_return = end_value / start_value - 1.0;
            periods.push(PeriodReturn {
                start_date: valuations.first().unwrap().0,
                end_date: valuations.last().unwrap().0,
                start_value,
                end_value,
                cash_flow: 0.0,
                return_rate: period_return,
            });
            return (period_return, periods);
        }
        return (0.0, periods);
    }

    // Sort cash flow dates
    cf_dates.sort();

    // Process sub-periods between cash flows
    let first_val_date = valuations.first().unwrap().0;
    let last_val_date = valuations.last().unwrap().0;

    // Build sub-periods: start → cf1 → cf2 → ... → end
    let mut period_boundaries: Vec<NaiveDate> = vec![first_val_date];
    for cf_date in &cf_dates {
        if *cf_date > first_val_date && *cf_date < last_val_date {
            period_boundaries.push(*cf_date);
        }
    }
    period_boundaries.push(last_val_date);
    period_boundaries.dedup();

    // Calculate return for each sub-period
    for i in 0..period_boundaries.len() - 1 {
        let period_start = period_boundaries[i];
        let period_end = period_boundaries[i + 1];

        // Find valuations closest to period boundaries
        let start_value = find_value_at_or_near(valuations, period_start);
        let end_value = find_value_at_or_near(valuations, period_end);

        // Sum cash flows that occurred at the END of this period (end-of-day convention)
        // User chose end-of-day: CF affects the ending NAV, not the starting capital
        let period_cash_flow: f64 = cash_flows
            .iter()
            .filter(|cf| cf.date == period_end)
            .map(|cf| cf.amount)
            .sum();

        // Calculate sub-period return using END-OF-DAY convention:
        // r = (V_end - CF) / V_start - 1
        // CF is subtracted from end value because it arrived at end of period
        let period_return = if start_value > 0.0 {
            (end_value - period_cash_flow) / start_value - 1.0
        } else {
            0.0
        };

        // Geometric chaining: multiply (1 + r_i)
        cumulative_return *= 1.0 + period_return;

        periods.push(PeriodReturn {
            start_date: period_start,
            end_date: period_end,
            start_value,
            end_value,
            cash_flow: period_cash_flow,
            return_rate: period_return,
        });

        log::debug!(
            "TTWROR period {}-{}: start={:.2}, end={:.2}, cf={:.2}, return={:.4}%",
            period_start, period_end, start_value, end_value, period_cash_flow, period_return * 100.0
        );
    }

    // Total return = ∏(1 + r_i) - 1
    let total_return = cumulative_return - 1.0;

    (total_return, periods)
}

/// Find portfolio value at or near a specific date
fn find_value_at_or_near(valuations: &[(NaiveDate, f64)], target_date: NaiveDate) -> f64 {
    // First try exact match
    for (date, value) in valuations {
        if *date == target_date {
            return *value;
        }
    }

    // Find closest date before target
    let mut closest_before: Option<(NaiveDate, f64)> = None;
    let mut closest_after: Option<(NaiveDate, f64)> = None;

    for (date, value) in valuations {
        if *date < target_date {
            match closest_before {
                None => closest_before = Some((*date, *value)),
                Some((prev_date, _)) if *date > prev_date => closest_before = Some((*date, *value)),
                _ => {}
            }
        } else if *date > target_date {
            match closest_after {
                None => closest_after = Some((*date, *value)),
                Some((prev_date, _)) if *date < prev_date => closest_after = Some((*date, *value)),
                _ => {}
            }
        }
    }

    // Prefer value before target date (more conservative)
    closest_before
        .or(closest_after)
        .map(|(_, v)| v)
        .unwrap_or(0.0)
}

/// Fallback to simple return when not enough valuation data
///
/// Fix: Now uses portfolio value at end_date (not always today) for historical periods
fn calculate_ttwror_simple_fallback(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    days: i64,
) -> Result<TtwrorResult> {
    // Get portfolio value at end_date (not today!) for correct historical calculations
    let end_value = get_portfolio_value_at_date_with_currency(conn, portfolio_id, end_date)?;

    // Get cost basis at start_date
    let cost_basis = get_total_cost_basis_with_currency(conn, portfolio_id)?;

    log::info!(
        "TTWROR fallback: EndValue={:.2} (at {}), CostBasis={:.2}",
        end_value, end_date, cost_basis
    );

    let total_return = if cost_basis > 0.0 {
        (end_value - cost_basis) / cost_basis
    } else {
        0.0
    };

    let annualized_return = if days > 0 && total_return > -1.0 {
        (1.0 + total_return).powf(365.0 / days as f64) - 1.0
    } else {
        0.0
    };

    Ok(TtwrorResult {
        total_return,
        annualized_return,
        days,
        periods: vec![PeriodReturn {
            start_date,
            end_date,
            start_value: cost_basis,
            end_value,
            cash_flow: 0.0,
            return_rate: total_return,
        }],
    })
}

/// Get portfolio values for TTWROR calculation with currency conversion
fn get_ttwror_portfolio_values(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>> {
    use crate::currency;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    // Get all unique dates with prices in range
    let dates_sql = r#"
        SELECT DISTINCT date(date) as d
        FROM pp_price
        WHERE date(date) >= ? AND date(date) <= ?
        ORDER BY d
    "#;

    let mut dates: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare(dates_sql)?;
        let rows = stmt.query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| row.get::<_, String>(0),
        )?;
        for row in rows.flatten() {
            dates.push(row);
        }
    }

    let mut values: Vec<(NaiveDate, f64)> = Vec::new();

    for date_str in dates {
        let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Get holdings as of this date
        let holdings_sql = format!(
            r#"
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
              AND date(t.date) <= ?
              {}
            GROUP BY t.security_id
            HAVING net_shares > 0
            "#,
            portfolio_filter
        );

        let mut total_value = 0.0;
        {
            let mut stmt = conn.prepare(&holdings_sql)?;
            let rows = stmt.query_map([&date_str], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, i64>(2)?,
                ))
            })?;

            for row in rows.flatten() {
                let (security_id, sec_currency, share_count) = row;

                // Get price at this date
                let price_sql = r#"
                    SELECT value FROM pp_price
                    WHERE security_id = ? AND date(date) <= ?
                    ORDER BY date DESC LIMIT 1
                "#;

                if let Ok(price) = conn.query_row(price_sql, params![security_id, date_str], |row| row.get::<_, i64>(0)) {
                    let shares_f = share_count as f64 / SHARES_SCALE;
                    let mut price_f = price as f64 / 100_000_000.0;

                    // GBX/GBp correction
                    let convert_currency = if sec_currency == "GBX" || sec_currency == "GBp" {
                        price_f /= 100.0;
                        "GBP"
                    } else {
                        sec_currency.as_str()
                    };

                    let value = shares_f * price_f;

                    // Convert to base currency using the same date for consistency
                    let value_base = if !convert_currency.is_empty() && convert_currency != base_currency {
                        currency::convert(conn, value, convert_currency, &base_currency, date)
                            .unwrap_or(value)
                    } else {
                        value
                    };

                    total_value += value_base;
                }
            }
        }

        // Add cash balance from linked accounts (Phase 2: NAV inkl. Cash)
        if let Some(pid) = portfolio_id {
            if let Ok(cash) = get_total_cash_balance_converted(conn, pid, date, &base_currency) {
                total_value += cash;
            }
        }

        if total_value > 0.0 {
            values.push((date, total_value));
        }
    }

    log::info!("TTWROR: Got {} portfolio values (incl. cash) from {} to {}", values.len(), start_date, end_date);
    Ok(values)
}

/// Get portfolio value at a specific date with currency conversion to base currency
///
/// Fix: Now takes valuation_date parameter instead of always using today.
/// This enables correct historical TTWROR calculations.
pub fn get_portfolio_value_at_date_with_currency(
    conn: &Connection,
    portfolio_id: Option<i64>,
    valuation_date: NaiveDate,
) -> Result<f64> {
    use crate::currency;

    // Get base currency
    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    let date_str = valuation_date.to_string();

    // Get holdings as of valuation_date with security currency
    let holdings_sql = format!(
        r#"
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
          AND date(t.date) <= ?1
          {}
        GROUP BY t.security_id
        HAVING net_shares > 0
        "#,
        portfolio_filter
    );

    let mut total_value = 0.0;

    {
        let mut stmt = conn.prepare(&holdings_sql)?;
        let rows = stmt.query_map([&date_str], |row| {
            Ok((
                row.get::<_, i64>(0)?,                                // security_id
                row.get::<_, Option<String>>(1)?.unwrap_or_default(), // currency
                row.get::<_, i64>(2)?,                                // net_shares
            ))
        })?;

        for row in rows.flatten() {
            let (security_id, security_currency, share_count) = row;

            // Get price at or before valuation_date (not always latest!)
            let price_sql = r#"
                SELECT value FROM pp_price
                WHERE security_id = ?1 AND date(date) <= ?2
                ORDER BY date DESC LIMIT 1
            "#;

            let price: Option<i64> = conn
                .query_row(price_sql, params![security_id, date_str], |row| row.get(0))
                .ok()
                .or_else(|| {
                    // Fallback to latest_price if no historical price found
                    conn.query_row(
                        "SELECT value FROM pp_latest_price WHERE security_id = ?1",
                        [security_id],
                        |row| row.get(0),
                    )
                    .ok()
                });

            if let Some(p) = price {
                let shares_f = share_count as f64 / SHARES_SCALE;
                let mut price_f = p as f64 / 100_000_000.0;

                // GBX/GBp correction
                let convert_currency = if security_currency == "GBX" || security_currency == "GBp" {
                    price_f /= 100.0;
                    "GBP"
                } else {
                    security_currency.as_str()
                };

                let value_in_security_currency = shares_f * price_f;

                // Convert to base currency using valuation_date for FX rate
                let value_in_base = if !convert_currency.is_empty() && convert_currency != base_currency.as_str() {
                    currency::convert(conn, value_in_security_currency, convert_currency, &base_currency, valuation_date)
                        .unwrap_or(value_in_security_currency)
                } else {
                    value_in_security_currency
                };

                total_value += value_in_base;
            }
        }
    }

    // Add cash balance from linked accounts
    if let Some(pid) = portfolio_id {
        if let Ok(cash) = get_total_cash_balance_converted(conn, pid, valuation_date, &base_currency) {
            total_value += cash;
        }
    }

    log::info!("TTWROR: Portfolio value at {} with currency conversion: {:.2}", valuation_date, total_value);
    Ok(total_value)
}

/// Get current portfolio value with currency conversion to base currency
/// Convenience wrapper for get_portfolio_value_at_date_with_currency with today's date
#[allow(dead_code)]
fn get_portfolio_value_with_currency(
    conn: &Connection,
    portfolio_id: Option<i64>,
) -> Result<f64> {
    get_portfolio_value_at_date_with_currency(conn, portfolio_id, chrono::Utc::now().date_naive())
}

/// Get total cost basis from FIFO lots with currency conversion
/// Uses SINGLE SOURCE OF TRUTH: fifo::get_total_cost_basis_converted()
fn get_total_cost_basis_with_currency(
    conn: &Connection,
    portfolio_id: Option<i64>,
) -> Result<f64> {
    use crate::currency;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());

    // Use central SSOT function - converts each lot individually
    let total_cost = crate::fifo::get_total_cost_basis_converted(conn, portfolio_id, &base_currency)
        .map_err(|e| anyhow::anyhow!(e))?;

    log::info!("TTWROR: Cost basis with currency conversion: {:.2}", total_cost);
    Ok(total_cost)
}

/// Get price at or near a specific date (with fallback to future prices and latest price)
#[allow(dead_code)]
fn get_price_at_or_near_date(
    conn: &Connection,
    security_id: i64,
    date: &str,
) -> Option<i64> {
    // First try: price at or before date (use date() function for proper comparison)
    let before_sql = r#"
        SELECT value FROM pp_price
        WHERE security_id = ?1 AND date(date) <= date(?2)
        ORDER BY date DESC
        LIMIT 1
    "#;

    if let Ok(price) = conn.query_row(before_sql, params![security_id, date], |row| row.get::<_, i64>(0)) {
        return Some(price);
    }

    // Fallback 1: first price after date (for new holdings without historical prices)
    let after_sql = r#"
        SELECT value FROM pp_price
        WHERE security_id = ?1 AND date(date) > date(?2)
        ORDER BY date ASC
        LIMIT 1
    "#;

    if let Ok(price) = conn.query_row(after_sql, params![security_id, date], |row| row.get::<_, i64>(0)) {
        return Some(price);
    }

    // Fallback 2: latest price (for securities without any historical prices)
    let latest_sql = "SELECT value FROM pp_latest_price WHERE security_id = ?1";
    conn.query_row(latest_sql, params![security_id], |row| row.get::<_, i64>(0)).ok()
}

/// Calculate portfolio value at a specific date
#[allow(dead_code)]
fn calculate_portfolio_value_at_date(
    conn: &Connection,
    portfolio_id: Option<i64>,
    date: NaiveDate,
) -> Result<f64> {
    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

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

    let date_str = date.to_string();
    let mut holdings: Vec<(i64, i64)> = Vec::new();
    {
        let mut stmt = conn.prepare(&holdings_sql)?;
        let rows = stmt.query_map([&date_str], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows.flatten() {
            holdings.push(row);
        }
    }

    if holdings.is_empty() {
        log::debug!("TTWROR: No holdings found at date {}", date);
        return Ok(0.0);
    }

    // Calculate total value using prices at or near this date
    let mut total_value = 0.0;
    let mut prices_found = 0;
    for (security_id, share_count) in &holdings {
        let price = get_price_at_or_near_date(conn, *security_id, &date_str);

        if let Some(p) = price {
            let shares_f = *share_count as f64 / SHARES_SCALE;
            let price_f = p as f64 / 100_000_000.0;
            total_value += shares_f * price_f;
            prices_found += 1;
        }
    }

    log::info!(
        "TTWROR: Holdings at {}: {} securities, {}/{} prices found, value={:.2}",
        date, holdings.len(), prices_found, holdings.len(), total_value
    );

    Ok(total_value)
}

/// Get initial investment amount (sum of BUY amounts) up to a date
/// Used as fallback when no price data is available
#[allow(dead_code)]
fn get_initial_investment_amount(
    conn: &Connection,
    portfolio_id: Option<i64>,
    date: NaiveDate,
) -> Result<f64> {
    let portfolio_filter = portfolio_id
        .map(|id| format!("AND owner_id = {}", id))
        .unwrap_or_default();

    let sql = format!(
        r#"
        SELECT COALESCE(SUM(amount), 0)
        FROM pp_txn
        WHERE owner_type = 'portfolio'
          AND txn_type IN ('BUY', 'DELIVERY_INBOUND')
          AND amount IS NOT NULL
          AND date(date) <= ?
          {}
        "#,
        portfolio_filter
    );

    let date_str = date.to_string();
    let total: i64 = conn.query_row(&sql, [&date_str], |row| row.get(0))?;

    Ok(total as f64 / AMOUNT_SCALE)
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
#[allow(dead_code)]
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
        let date = match parse_date_flexible(&date_str) {
            Some(d) => d,
            None => continue, // Skip unparseable dates
        };

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

/// Get EXTERNAL cash flows (deposits and withdrawals only) for TTWROR calculation
///
/// Phase 3 fix: Now converts all cash flows to base currency for consistency.
///
/// For TTWROR, we need only EXTERNAL cash flows - money coming into or leaving
/// the portfolio from outside. This does NOT include:
/// - BUY/SELL: These are INTERNAL transactions (cash ↔ securities within portfolio)
/// - TRANSFER_IN/OUT: Asset movements, not cash
/// - DELIVERY_INBOUND/OUTBOUND: Asset movements, not cash
///
/// Only DEPOSIT and REMOVAL represent external cash flows.
///
/// IMPORTANT: This function has NO fallback to BUY/SELL. For IRR with fallback,
/// use `get_cash_flows_with_fallback()` instead.
fn get_cash_flows(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<CashFlow>> {
    use crate::currency;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());
    let mut cash_flows: Vec<CashFlow> = Vec::new();

    // Get account-level external cash flows (DEPOSIT/REMOVAL) with account currency
    if let Some(pid) = portfolio_id {
        // FIX: Get ALL linked accounts, not just reference_account
        // This includes reference_account + accounts linked via CrossEntry (BUY/SELL)
        let linked_account_ids = get_linked_account_ids(conn, pid)?;

        if linked_account_ids.is_empty() {
            log::warn!("No linked accounts found for portfolio {}", pid);
            return Ok(cash_flows);
        }

        // Build SQL with IN clause for all linked accounts
        let placeholders: Vec<String> = linked_account_ids.iter().enumerate()
            .map(|(i, _)| format!("?{}", i + 3))  // ?3, ?4, ?5, ...
            .collect();
        let in_clause = placeholders.join(", ");

        let account_sql = format!(
            r#"
            SELECT t.date, t.txn_type, t.amount, a.currency
            FROM pp_txn t
            JOIN pp_account a ON a.id = t.owner_id
            WHERE t.owner_type = 'account'
              AND t.txn_type IN ('DEPOSIT', 'REMOVAL')
              AND t.owner_id IN ({})
              AND date(t.date) >= ?1 AND date(t.date) <= ?2
            ORDER BY t.date
            "#,
            in_clause
        );

        let mut stmt = conn.prepare(&account_sql)?;

        // Build params: start_date, end_date, account_id1, account_id2, ...
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(start_date.to_string()),
            Box::new(end_date.to_string()),
        ];
        for account_id in &linked_account_ids {
            params_vec.push(Box::new(*account_id));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?.unwrap_or_else(|| "EUR".to_string()),
            ))
        })?;

        for row in rows.flatten() {
            let (date_str, txn_type, amount, account_currency) = row;
            if let Some(date) = parse_date_flexible(&date_str) {
                let amount_f = amount as f64 / AMOUNT_SCALE;
                let cf_amount = match txn_type.as_str() {
                    "DEPOSIT" => amount_f,   // Money added (positive)
                    "REMOVAL" => -amount_f,  // Money removed (negative)
                    _ => 0.0,
                };

                if cf_amount != 0.0 {
                    // Convert to base currency (Phase 3 fix)
                    let cf_base = if account_currency != base_currency && !account_currency.is_empty() {
                        currency::convert(conn, cf_amount, &account_currency, &base_currency, date)
                            .unwrap_or(cf_amount)
                    } else {
                        cf_amount
                    };
                    cash_flows.push(CashFlow { date, amount: cf_base });
                }
            }
        }

        log::info!("Found {} DEPOSIT/REMOVAL from {} linked accounts for portfolio {}",
                   cash_flows.len(), linked_account_ids.len(), pid);
    } else {
        // For all portfolios, get all account DEPOSIT/REMOVAL with currency
        let sql = r#"
            SELECT t.date, t.txn_type, t.amount, a.currency
            FROM pp_txn t
            JOIN pp_account a ON a.id = t.owner_id
            WHERE t.owner_type = 'account'
              AND t.txn_type IN ('DEPOSIT', 'REMOVAL')
              AND date(t.date) >= ?1 AND date(t.date) <= ?2
            ORDER BY t.date
        "#;

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?.unwrap_or_else(|| "EUR".to_string()),
                ))
            },
        )?;

        for row in rows.flatten() {
            let (date_str, txn_type, amount, account_currency) = row;
            if let Some(date) = parse_date_flexible(&date_str) {
                let amount_f = amount as f64 / AMOUNT_SCALE;
                let cf_amount = match txn_type.as_str() {
                    "DEPOSIT" => amount_f,   // Money added (positive)
                    "REMOVAL" => -amount_f,  // Money removed (negative)
                    _ => 0.0,
                };

                if cf_amount != 0.0 {
                    // Convert to base currency (Phase 3 fix)
                    let cf_base = if account_currency != base_currency && !account_currency.is_empty() {
                        currency::convert(conn, cf_amount, &account_currency, &base_currency, date)
                            .unwrap_or(cf_amount)
                    } else {
                        cf_amount
                    };
                    cash_flows.push(CashFlow { date, amount: cf_base });
                }
            }
        }
    }

    // Sort by date
    cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    // NO FALLBACK: TTWROR and Risk Metrics must use only external cash flows
    // For IRR with fallback, use get_cash_flows_with_fallback() instead
    log::info!("TTWROR/Risk: Found {} external cash flows (DEPOSIT/REMOVAL, converted to {})",
               cash_flows.len(), base_currency);

    Ok(cash_flows)
}

/// Get cash flows for IRR calculation
///
/// This function collects ALL external capital flows for IRR:
/// 1. Account-level: DEPOSIT, REMOVAL
/// 2. Portfolio-level: DELIVERY_INBOUND, DELIVERY_OUTBOUND (asset transfers with value)
///
/// If no cash flows found, falls back to BUY/SELL as proxy.
///
/// ONLY use this for IRR calculation, NOT for TTWROR or Risk Metrics!
pub fn get_cash_flows_with_fallback(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<CashFlow>> {
    // Start with DEPOSIT/REMOVAL
    let mut cash_flows = get_cash_flows(conn, portfolio_id, start_date, end_date)?;
    let deposit_removal_count = cash_flows.len();

    // ALWAYS add DELIVERY_INBOUND/OUTBOUND - these are external asset flows with monetary value
    // (like receiving/sending securities from/to another broker)
    let delivery_flows = get_delivery_cash_flows(conn, portfolio_id, start_date, end_date)?;
    let delivery_count = delivery_flows.len();
    cash_flows.extend(delivery_flows);

    // FALLBACK: If still no cash flows, use BUY/SELL as proxy for invested capital
    if cash_flows.is_empty() {
        log::warn!("No DEPOSIT/REMOVAL/DELIVERY found, using Portfolio BUY/SELL as fallback for IRR");
        cash_flows = get_buy_sell_cash_flows(conn, portfolio_id, start_date, end_date)?;
    }

    // Sort by date after merging
    cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    log::info!("IRR: Found {} cash flows ({} DEPOSIT/REMOVAL + {} DELIVERY)",
               cash_flows.len(), deposit_removal_count, delivery_count);

    Ok(cash_flows)
}

/// Fallback: Get BUY/SELL from Portfolio transactions as cash flow proxy
/// Used when no DEPOSIT/REMOVAL transactions are found
fn get_buy_sell_cash_flows(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<CashFlow>> {
    use crate::currency;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());
    let mut cash_flows: Vec<CashFlow> = Vec::new();

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    // Get Portfolio BUY/SELL transactions with amount from transaction units
    // BUY = money out (negative), SELL = money in (positive)
    let sql = format!(
        r#"
        SELECT t.date, t.txn_type, t.currency,
               COALESCE(
                   (SELECT SUM(u.amount) FROM pp_txn_unit u WHERE u.txn_id = t.id AND u.unit_type = 'GROSS_VALUE'),
                   0
               ) as gross_value
        FROM pp_txn t
        WHERE t.owner_type = 'portfolio'
          AND t.txn_type IN ('BUY', 'SELL')
          {}
          AND date(t.date) >= ?1 AND date(t.date) <= ?2
        ORDER BY t.date
        "#,
        portfolio_filter
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![start_date.to_string(), end_date.to_string()],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "EUR".to_string()),
                row.get::<_, i64>(3)?,
            ))
        },
    )?;

    for row in rows.flatten() {
        let (date_str, txn_type, txn_currency, gross_value) = row;
        if let Some(date) = parse_date_flexible(&date_str) {
            if gross_value == 0 {
                continue;
            }

            let amount_f = gross_value as f64 / AMOUNT_SCALE;
            let cf_amount = match txn_type.as_str() {
                "BUY" => amount_f,   // Money invested (positive = capital outflow for IRR)
                "SELL" => -amount_f, // Money returned (negative = capital inflow for IRR)
                _ => 0.0,
            };

            if cf_amount != 0.0 {
                // Convert to base currency
                let cf_base = if txn_currency != base_currency && !txn_currency.is_empty() {
                    currency::convert(conn, cf_amount, &txn_currency, &base_currency, date)
                        .unwrap_or(cf_amount)
                } else {
                    cf_amount
                };
                cash_flows.push(CashFlow { date, amount: cf_base });
            }
        }
    }

    cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    log::info!("Fallback: Found {} BUY/SELL as cash flow proxy", cash_flows.len());

    Ok(cash_flows)
}

/// Get DELIVERY_INBOUND/OUTBOUND as cash flows
/// These represent external asset transfers (e.g., from another broker)
/// DELIVERY_INBOUND = assets received = money invested (positive for IRR inversion)
/// DELIVERY_OUTBOUND = assets sent = money returned (negative for IRR inversion)
fn get_delivery_cash_flows(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<CashFlow>> {
    use crate::currency;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());
    let mut cash_flows: Vec<CashFlow> = Vec::new();

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    // Get Portfolio DELIVERY_INBOUND/OUTBOUND transactions
    // The value is stored in t.amount (NOT in pp_txn_unit.GROSS_VALUE for most deliveries!)
    // DELIVERY_INBOUND = assets received (like money invested)
    // DELIVERY_OUTBOUND = assets sent (like money returned)
    let sql = format!(
        r#"
        SELECT t.date, t.txn_type, t.currency, t.amount
        FROM pp_txn t
        WHERE t.owner_type = 'portfolio'
          AND t.txn_type IN ('DELIVERY_INBOUND', 'DELIVERY_OUTBOUND')
          {}
          AND date(t.date) >= ?1 AND date(t.date) <= ?2
        ORDER BY t.date
        "#,
        portfolio_filter
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![start_date.to_string(), end_date.to_string()],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?.unwrap_or_else(|| "EUR".to_string()),
                row.get::<_, i64>(3)?,
            ))
        },
    )?;

    for row in rows.flatten() {
        let (date_str, txn_type, txn_currency, amount) = row;
        if let Some(date) = parse_date_flexible(&date_str) {
            if amount == 0 {
                continue;
            }

            let amount_f = amount as f64 / AMOUNT_SCALE;
            // DELIVERY_INBOUND = money invested (positive, will be inverted to negative in IRR calc)
            // DELIVERY_OUTBOUND = money returned (negative, will be inverted to positive in IRR calc)
            let cf_amount = match txn_type.as_str() {
                "DELIVERY_INBOUND" => amount_f,   // Assets received = money invested
                "DELIVERY_OUTBOUND" => -amount_f, // Assets sent = money returned
                _ => 0.0,
            };

            if cf_amount != 0.0 {
                // Convert to base currency
                let cf_base = if txn_currency != base_currency && !txn_currency.is_empty() {
                    currency::convert(conn, cf_amount, &txn_currency, &base_currency, date)
                        .unwrap_or(cf_amount)
                } else {
                    cf_amount
                };
                cash_flows.push(CashFlow { date, amount: cf_base });
            }
        }
    }

    cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    log::info!("Found {} DELIVERY_INBOUND/OUTBOUND as external cash flows", cash_flows.len());

    Ok(cash_flows)
}

/// Calculate performance for all holdings
///
/// Phase 6 fix: IRR uses today as final date (not last transaction date),
/// and current_value now includes cash balance with currency conversion.
pub fn calculate_portfolio_performance(
    conn: &Connection,
    portfolio_id: Option<i64>,
) -> Result<(TtwrorResult, IrrResult)> {
    // Get date range from transactions
    let (start_date, _end_date) = get_transaction_date_range(conn, portfolio_id)?;

    // Use today as the actual valuation date (Phase 6 fix)
    let today = chrono::Utc::now().date_naive();

    let ttwror = calculate_ttwror(conn, portfolio_id, start_date, today)?;

    // Get cash flows for IRR (from first transaction to today)
    // Use with_fallback variant for IRR - falls back to BUY/SELL if no DEPOSIT/REMOVAL
    let cash_flows = get_cash_flows_with_fallback(conn, portfolio_id, start_date, today)?;

    // Get current portfolio value (now includes cash + currency conversion)
    let current_value = get_current_portfolio_value(conn, portfolio_id)?;

    // DEBUG: Write detailed cash flow information to file
    let total_cf: f64 = cash_flows.iter().map(|cf| cf.amount).sum();
    {
        use std::io::Write;
        if let Ok(mut file) = std::fs::File::create("/tmp/irr-debug-output.txt") {
            let _ = writeln!(file, "IRR DEBUG: {} cash flows, total={:.2}, current_value={:.2}, start={}, end={}",
                cash_flows.len(), total_cf, current_value, start_date, today);
            for (i, cf) in cash_flows.iter().take(20).enumerate() {
                let _ = writeln!(file, "  CF[{}]: date={}, amount={:.2}", i, cf.date, cf.amount);
            }
            if cash_flows.len() > 20 {
                let _ = writeln!(file, "  ... and {} more cash flows", cash_flows.len() - 20);
            }
        }
    }

    // IRR final date = today (not last transaction date)
    let irr = calculate_irr(&cash_flows, current_value, today)?;

    // Append IRR result to debug file
    {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open("/tmp/irr-debug-output.txt") {
            let _ = writeln!(file, "\nPortfolio performance: TTWROR={:.2}%, IRR={:.2}% (converged={}), Value={:.2}",
                ttwror.total_return * 100.0,
                irr.irr * 100.0,
                irr.converged,
                current_value);
        }
    }

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
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());

    let end = max_date
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    Ok((start, end))
}

/// Get current portfolio value with currency conversion and cash balance
///
/// Phase 2 fix: Now includes currency conversion and cash from linked accounts
fn get_current_portfolio_value(conn: &Connection, portfolio_id: Option<i64>) -> Result<f64> {
    use crate::currency;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());
    let today = chrono::Utc::now().date_naive();

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    // Get holdings with security currency
    let holdings_sql = format!(
        r#"
        SELECT
            t.security_id,
            s.currency,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares,
            lp.value as latest_price
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
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
            Ok((
                row.get::<_, i64>(0)?,                                // security_id
                row.get::<_, Option<String>>(1)?.unwrap_or_default(), // currency
                row.get::<_, i64>(2)?,                                // net_shares
                row.get::<_, Option<i64>>(3)?,                        // latest_price
            ))
        })?;

        for row in rows.flatten() {
            let (_security_id, security_currency, share_count, price_opt) = row;

            if let Some(price) = price_opt {
                let shares_f = share_count as f64 / SHARES_SCALE;
                let mut price_f = price as f64 / 100_000_000.0;

                // GBX/GBp correction
                let convert_currency = if security_currency == "GBX" || security_currency == "GBp" {
                    price_f /= 100.0;
                    "GBP"
                } else {
                    security_currency.as_str()
                };

                let value_in_security_currency = shares_f * price_f;

                // Convert to base currency
                let value_in_base = if !convert_currency.is_empty() && convert_currency != base_currency.as_str() {
                    currency::convert(conn, value_in_security_currency, convert_currency, &base_currency, today)
                        .unwrap_or(value_in_security_currency)
                } else {
                    value_in_security_currency
                };

                total_value += value_in_base;
            }
        }
    }

    // Add cash balance from linked accounts (Phase 2: NAV inkl. Cash)
    if let Some(pid) = portfolio_id {
        if let Ok(cash) = get_total_cash_balance_converted(conn, pid, today, &base_currency) {
            total_value += cash;
        }
    }

    log::info!("Current portfolio value (incl. cash, converted): {:.2} {}", total_value, base_currency);
    Ok(total_value)
}

// =====================================================
// Risk Metrics: Sharpe, Sortino, Drawdown, Volatility, Beta
// =====================================================

/// Risk metrics result
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskMetrics {
    /// Sharpe Ratio (excess return / volatility)
    pub sharpe_ratio: f64,
    /// Sortino Ratio (excess return / downside deviation)
    pub sortino_ratio: f64,
    /// Maximum Drawdown (largest peak-to-trough decline)
    pub max_drawdown: f64,
    /// Maximum Drawdown start date
    pub max_drawdown_start: Option<String>,
    /// Maximum Drawdown end date
    pub max_drawdown_end: Option<String>,
    /// Annualized volatility (standard deviation of returns)
    pub volatility: f64,
    /// Beta vs benchmark (if provided)
    pub beta: Option<f64>,
    /// Alpha vs benchmark (if provided)
    pub alpha: Option<f64>,
    /// Calmar Ratio (annualized return / max drawdown)
    pub calmar_ratio: Option<f64>,
    /// Number of data points used
    pub data_points: usize,
}

/// Calculate risk metrics for a portfolio
///
/// Phase 4 fix: Uses flow-adjusted daily returns to prevent cash flow distortion.
/// Risk-free rate default: 3% (typical for EUR savings)
pub fn calculate_risk_metrics(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
    benchmark_id: Option<i64>,
    risk_free_rate: Option<f64>,
) -> Result<RiskMetrics> {
    let rf_rate = risk_free_rate.unwrap_or(0.03); // 3% default

    // Get portfolio value history
    let portfolio_values = get_portfolio_value_history(conn, portfolio_id, start_date, end_date)?;

    if portfolio_values.len() < 2 {
        return Ok(RiskMetrics {
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            max_drawdown: 0.0,
            max_drawdown_start: None,
            max_drawdown_end: None,
            volatility: 0.0,
            beta: None,
            alpha: None,
            calmar_ratio: None,
            data_points: portfolio_values.len(),
        });
    }

    // Get cash flows for flow-adjusted returns (Phase 4 fix)
    let cash_flows = get_cash_flows(conn, portfolio_id, start_date, end_date)?;

    // Calculate flow-adjusted daily returns (removes cash flow distortion)
    let returns = calculate_flow_adjusted_returns(&portfolio_values, &cash_flows);

    if returns.is_empty() {
        return Ok(RiskMetrics {
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            max_drawdown: 0.0,
            max_drawdown_start: None,
            max_drawdown_end: None,
            volatility: 0.0,
            beta: None,
            alpha: None,
            calmar_ratio: None,
            data_points: 0,
        });
    }

    // Calculate volatility (annualized standard deviation)
    let volatility = calculate_volatility(&returns);

    // Calculate downside deviation (for Sortino)
    let daily_rf = rf_rate / 252.0;
    let downside_deviation = calculate_downside_deviation(&returns, daily_rf);

    // Calculate mean return and annualize
    let mean_return = returns.iter().sum::<f64>() / returns.len() as f64;
    let annualized_return = mean_return * 252.0;

    // Sharpe Ratio = (Return - RiskFreeRate) / Volatility
    let sharpe_ratio = if volatility > 0.0 {
        (annualized_return - rf_rate) / volatility
    } else {
        0.0
    };

    // Sortino Ratio = (Return - RiskFreeRate) / DownsideDeviation
    let sortino_ratio = if downside_deviation > 0.0 {
        (annualized_return - rf_rate) / downside_deviation
    } else {
        0.0
    };

    // Calculate Maximum Drawdown
    let (max_drawdown, dd_start, dd_end) = calculate_max_drawdown(&portfolio_values);

    // Calmar Ratio = Annualized Return / Max Drawdown
    let calmar_ratio = if max_drawdown > 0.0 {
        Some(annualized_return / max_drawdown)
    } else {
        None
    };

    // Calculate Beta and Alpha if benchmark provided
    // Fix: Now passes portfolio_id to calculate for specific portfolio
    let (beta, alpha) = if let Some(bench_id) = benchmark_id {
        calculate_beta_alpha(conn, portfolio_id, bench_id, start_date, end_date, rf_rate)?
    } else {
        (None, None)
    };

    Ok(RiskMetrics {
        sharpe_ratio,
        sortino_ratio,
        max_drawdown,
        max_drawdown_start: dd_start,
        max_drawdown_end: dd_end,
        volatility,
        beta,
        alpha,
        calmar_ratio,
        data_points: returns.len(),
    })
}

/// Get portfolio value history for risk calculations
fn get_portfolio_value_history(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<(NaiveDate, f64)>> {
    use crate::currency;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());

    let portfolio_filter = portfolio_id
        .map(|id| format!("AND t.owner_id = {}", id))
        .unwrap_or_default();

    // Get all unique dates with prices in range
    let dates_sql = r#"
        SELECT DISTINCT date(date) as d
        FROM pp_price
        WHERE date(date) >= ? AND date(date) <= ?
        ORDER BY d
    "#;

    let mut dates: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare(dates_sql)?;
        let rows = stmt.query_map(
            params![start_date.to_string(), end_date.to_string()],
            |row| row.get::<_, String>(0),
        )?;
        for row in rows.flatten() {
            dates.push(row);
        }
    }

    let mut values: Vec<(NaiveDate, f64)> = Vec::new();

    for date_str in dates {
        let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };

        // Get holdings as of this date
        let holdings_sql = format!(
            r#"
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
              AND date(t.date) <= ?
              {}
            GROUP BY t.security_id
            HAVING net_shares > 0
            "#,
            portfolio_filter
        );

        let mut total_value = 0.0;
        {
            let mut stmt = conn.prepare(&holdings_sql)?;
            let rows = stmt.query_map([&date_str], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    row.get::<_, i64>(2)?,
                ))
            })?;

            for row in rows.flatten() {
                let (security_id, sec_currency, share_count) = row;

                // Get price at this date
                let price_sql = r#"
                    SELECT value FROM pp_price
                    WHERE security_id = ? AND date(date) <= ?
                    ORDER BY date DESC LIMIT 1
                "#;

                if let Ok(price) = conn.query_row(price_sql, params![security_id, date_str], |row| row.get::<_, i64>(0)) {
                    let shares_f = share_count as f64 / SHARES_SCALE;
                    let mut price_f = price as f64 / 100_000_000.0;

                    // GBX correction
                    let convert_currency = if sec_currency == "GBX" || sec_currency == "GBp" {
                        price_f /= 100.0;
                        "GBP"
                    } else {
                        sec_currency.as_str()
                    };

                    let value = shares_f * price_f;

                    // Convert to base currency
                    let value_base = if !convert_currency.is_empty() && convert_currency != base_currency {
                        currency::convert(conn, value, convert_currency, &base_currency, date)
                            .unwrap_or(value)
                    } else {
                        value
                    };

                    total_value += value_base;
                }
            }
        }

        // Add cash balance from linked accounts (Phase 2: NAV inkl. Cash)
        if let Some(pid) = portfolio_id {
            if let Ok(cash) = get_total_cash_balance_converted(conn, pid, date, &base_currency) {
                total_value += cash;
            }
        }

        if total_value > 0.0 {
            values.push((date, total_value));
        }
    }

    log::info!("Risk metrics: Got {} portfolio values (incl. cash) from {} to {}", values.len(), start_date, end_date);
    Ok(values)
}

/// Calculate daily returns from value series (without flow adjustment)
/// Note: Kept for reference/testing, main code now uses calculate_flow_adjusted_returns()
#[allow(dead_code)]
fn calculate_daily_returns(values: &[(NaiveDate, f64)]) -> Vec<f64> {
    values
        .windows(2)
        .filter_map(|w| {
            let (_, prev_val) = w[0];
            let (_, curr_val) = w[1];
            if prev_val > 0.0 {
                Some((curr_val - prev_val) / prev_val)
            } else {
                None
            }
        })
        .collect()
}

/// Calculate flow-adjusted daily returns (Phase 4 fix)
///
/// Adjusts returns for external cash flows using end-of-day convention:
/// r_t = (NAV_t - CF_t) / NAV_{t-1} - 1
///
/// This removes the effect of deposits/withdrawals from the return calculation,
/// preventing artificial spikes in volatility and risk metrics.
fn calculate_flow_adjusted_returns(
    values: &[(NaiveDate, f64)],
    cash_flows: &[CashFlow],
) -> Vec<f64> {
    use std::collections::HashMap;

    // Build a map of cash flows by date for O(1) lookup
    let cf_by_date: HashMap<NaiveDate, f64> = cash_flows
        .iter()
        .fold(HashMap::new(), |mut acc, cf| {
            *acc.entry(cf.date).or_insert(0.0) += cf.amount;
            acc
        });

    values
        .windows(2)
        .filter_map(|w| {
            let (prev_date, prev_val) = w[0];
            let (curr_date, curr_val) = w[1];

            if prev_val <= 0.0 {
                return None;
            }

            // Get cash flow at end of current day (end-of-day convention)
            let cf = cf_by_date.get(&curr_date).copied().unwrap_or(0.0);

            // Flow-adjusted return: (NAV_end - CF) / NAV_start - 1
            let adjusted_return = (curr_val - cf) / prev_val - 1.0;

            // Log significant cash flow adjustments
            if cf.abs() > 0.01 {
                log::debug!(
                    "Flow-adjusted return {}-{}: NAV {:.2} → {:.2}, CF={:.2}, r={:.4}%",
                    prev_date, curr_date, prev_val, curr_val, cf, adjusted_return * 100.0
                );
            }

            Some(adjusted_return)
        })
        .collect()
}

/// Calculate annualized volatility (standard deviation of returns)
fn calculate_volatility(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }

    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / returns.len() as f64;

    let daily_std = variance.sqrt();

    // Annualize: daily_std * sqrt(252 trading days)
    daily_std * (252.0_f64).sqrt()
}

/// Calculate downside deviation (only negative returns below target)
fn calculate_downside_deviation(returns: &[f64], target: f64) -> f64 {
    let downside_returns: Vec<f64> = returns
        .iter()
        .filter_map(|&r| {
            if r < target {
                Some((r - target).powi(2))
            } else {
                None
            }
        })
        .collect();

    if downside_returns.is_empty() {
        return 0.0;
    }

    let downside_variance = downside_returns.iter().sum::<f64>() / returns.len() as f64;

    // Annualize
    downside_variance.sqrt() * (252.0_f64).sqrt()
}

/// Calculate maximum drawdown from value series
fn calculate_max_drawdown(values: &[(NaiveDate, f64)]) -> (f64, Option<String>, Option<String>) {
    if values.len() < 2 {
        return (0.0, None, None);
    }

    let mut max_value = values[0].1;
    let mut max_date = values[0].0;
    let mut max_drawdown = 0.0;
    let mut dd_start: Option<NaiveDate> = None;
    let mut dd_end: Option<NaiveDate> = None;

    for (date, value) in values.iter() {
        if *value > max_value {
            max_value = *value;
            max_date = *date;
        }

        let drawdown = (max_value - value) / max_value;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
            dd_start = Some(max_date);
            dd_end = Some(*date);
        }
    }

    (
        max_drawdown,
        dd_start.map(|d| d.to_string()),
        dd_end.map(|d| d.to_string()),
    )
}

/// Calculate Beta and Alpha vs benchmark
///
/// Phase 5 fix: Uses date-based matching instead of length truncation,
/// and converts benchmark prices to base currency.
///
/// Fix from review: Now accepts portfolio_id to calculate Beta/Alpha for specific portfolio
/// instead of all portfolios combined.
fn calculate_beta_alpha(
    conn: &Connection,
    portfolio_id: Option<i64>,  // Now uses portfolio_id instead of unused _portfolio_returns
    benchmark_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
    risk_free_rate: f64,
) -> Result<(Option<f64>, Option<f64>)> {
    use crate::currency;
    use std::collections::HashMap;

    let base_currency = currency::get_base_currency(conn).unwrap_or_else(|_| "EUR".to_string());

    // Get benchmark currency for conversion
    let bench_currency: String = conn
        .query_row(
            "SELECT currency FROM pp_security WHERE id = ?1",
            [benchmark_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or_else(|| "EUR".to_string());

    // Get benchmark price history with currency conversion
    let bench_sql = r#"
        SELECT date(date) as d, value
        FROM pp_price
        WHERE security_id = ? AND date(date) >= ? AND date(date) <= ?
        ORDER BY d
    "#;

    let mut bench_values: Vec<(NaiveDate, f64)> = Vec::new();
    {
        let mut stmt = conn.prepare(bench_sql)?;
        let rows = stmt.query_map(
            params![benchmark_id, start_date.to_string(), end_date.to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as f64 / 100_000_000.0)),
        )?;

        for row in rows.flatten() {
            let (date_str, mut price) = row;
            if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                // GBX/GBp correction
                let convert_from = if bench_currency == "GBX" || bench_currency == "GBp" {
                    price /= 100.0;
                    "GBP"
                } else {
                    bench_currency.as_str()
                };

                // Convert to base currency (Phase 5 fix)
                let price_base = if convert_from != base_currency && !convert_from.is_empty() {
                    currency::convert(conn, price, convert_from, &base_currency, date)
                        .unwrap_or(price)
                } else {
                    price
                };

                bench_values.push((date, price_base));
            }
        }
    }

    if bench_values.len() < 2 {
        return Ok((None, None));
    }

    // Calculate benchmark returns WITH dates
    let bench_returns_dated: Vec<(NaiveDate, f64)> = bench_values
        .windows(2)
        .filter_map(|w| {
            let (_, prev) = w[0];
            let (date, curr) = w[1];
            if prev > 0.0 {
                Some((date, (curr - prev) / prev))
            } else {
                None
            }
        })
        .collect();

    // Build benchmark returns map by date
    let bench_map: HashMap<NaiveDate, f64> = bench_returns_dated
        .iter()
        .cloned()
        .collect();

    // Get portfolio values to build dated returns (now uses specific portfolio_id!)
    let portfolio_values = get_portfolio_value_history(conn, portfolio_id, start_date, end_date)?;
    let cash_flows = get_cash_flows(conn, portfolio_id, start_date, end_date)?;

    // Build portfolio returns map by date (flow-adjusted)
    let cf_by_date: HashMap<NaiveDate, f64> = cash_flows
        .iter()
        .fold(HashMap::new(), |mut acc, cf| {
            *acc.entry(cf.date).or_insert(0.0) += cf.amount;
            acc
        });

    let port_returns_dated: Vec<(NaiveDate, f64)> = portfolio_values
        .windows(2)
        .filter_map(|w| {
            let (_, prev_val) = w[0];
            let (date, curr_val) = w[1];
            if prev_val <= 0.0 {
                return None;
            }
            let cf = cf_by_date.get(&date).copied().unwrap_or(0.0);
            Some((date, (curr_val - cf) / prev_val - 1.0))
        })
        .collect();

    let port_map: HashMap<NaiveDate, f64> = port_returns_dated
        .iter()
        .cloned()
        .collect();

    // Date-based intersection (Phase 5 fix: proper alignment)
    let mut common_dates: Vec<NaiveDate> = port_map
        .keys()
        .filter(|d| bench_map.contains_key(d))
        .cloned()
        .collect();
    common_dates.sort();

    if common_dates.len() < 10 {
        log::warn!("Beta/Alpha: Only {} common dates, need at least 10", common_dates.len());
        return Ok((None, None));
    }

    // Extract aligned returns
    let port_ret: Vec<f64> = common_dates.iter().map(|d| port_map[d]).collect();
    let bench_ret: Vec<f64> = common_dates.iter().map(|d| bench_map[d]).collect();

    // Simple linear regression: portfolio = alpha + beta * benchmark
    let n = common_dates.len() as f64;
    let sum_x: f64 = bench_ret.iter().sum();
    let sum_y: f64 = port_ret.iter().sum();
    let sum_xy: f64 = port_ret.iter().zip(bench_ret.iter()).map(|(y, x)| x * y).sum();
    let sum_xx: f64 = bench_ret.iter().map(|x| x * x).sum();

    let denom = n * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-10 {
        return Ok((None, None));
    }

    let beta = (n * sum_xy - sum_x * sum_y) / denom;

    // Alpha = annualized excess return above what beta predicts
    let mean_port = sum_y / n;
    let mean_bench = sum_x / n;
    let daily_rf = risk_free_rate / 252.0;

    // Jensen's Alpha (annualized)
    let alpha = (mean_port - daily_rf - beta * (mean_bench - daily_rf)) * 252.0;

    log::info!("Beta/Alpha: {} common dates, beta={:.3}, alpha={:.4}%", common_dates.len(), beta, alpha * 100.0);
    Ok((Some(beta), Some(alpha)))
}

// =====================================================
// Cash Balance Functions (Phase 1: NAV inkl. Cash)
// =====================================================

/// Get all account IDs linked to a portfolio
///
/// Includes:
/// - The portfolio's reference_account_id
/// - Any accounts connected via CrossEntry (Buy/Sell transactions)
fn get_linked_account_ids(conn: &Connection, portfolio_id: i64) -> Result<Vec<i64>> {
    let sql = r#"
        SELECT DISTINCT account_id FROM (
            -- Reference Account
            SELECT reference_account_id as account_id
            FROM pp_portfolio
            WHERE id = ?1 AND reference_account_id IS NOT NULL

            UNION

            -- Accounts via CrossEntry (Buy/Sell link portfolio txn to account txn)
            SELECT at.owner_id as account_id
            FROM pp_txn pt
            JOIN pp_cross_entry ce ON ce.portfolio_txn_id = pt.id
            JOIN pp_txn at ON at.id = ce.account_txn_id
            WHERE pt.owner_type = 'portfolio'
              AND pt.owner_id = ?1
              AND at.owner_type = 'account'
        )
        WHERE account_id IS NOT NULL
    "#;

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([portfolio_id], |row| row.get::<_, i64>(0))?;

    let mut account_ids = Vec::new();
    for row in rows.flatten() {
        account_ids.push(row);
    }

    log::debug!("Found {} linked accounts for portfolio {}", account_ids.len(), portfolio_id);
    Ok(account_ids)
}

/// Calculate account cash balance at a specific date
///
/// Returns (balance in cents, currency)
/// Balance = sum of all credits minus debits up to the given date
fn get_account_balance_at_date(
    conn: &Connection,
    account_id: i64,
    date: NaiveDate,
) -> Result<(f64, String)> {
    let sql = r#"
        SELECT
            a.currency,
            COALESCE(SUM(
                CASE
                    -- Credits (money into account)
                    WHEN t.txn_type IN ('DEPOSIT', 'INTEREST', 'DIVIDENDS', 'TAX_REFUND',
                                        'FEES_REFUND', 'TRANSFER_IN', 'SELL') THEN t.amount
                    -- Debits (money out of account)
                    WHEN t.txn_type IN ('REMOVAL', 'FEES', 'TAXES', 'INTEREST_CHARGE',
                                        'TRANSFER_OUT', 'BUY') THEN -t.amount
                    ELSE 0
                END
            ), 0) as balance
        FROM pp_account a
        LEFT JOIN pp_txn t ON t.owner_type = 'account' AND t.owner_id = a.id
            AND date(t.date) <= ?2
        WHERE a.id = ?1
        GROUP BY a.id
    "#;

    let result: (String, i64) = conn.query_row(sql, params![account_id, date.to_string()], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "EUR".to_string()),
            row.get::<_, i64>(1)?,
        ))
    }).map_err(|e| anyhow::anyhow!("Failed to get account balance: {}", e))?;

    let balance_f = result.1 as f64 / AMOUNT_SCALE;
    log::debug!("Account {} balance at {}: {:.2} {}", account_id, date, balance_f, result.0);

    Ok((balance_f, result.0))
}

/// Get total cash balance from all linked accounts, converted to base currency
///
/// This is the sum of all account cash balances at a given date,
/// with each account's balance converted to the portfolio's base currency.
fn get_total_cash_balance_converted(
    conn: &Connection,
    portfolio_id: i64,
    date: NaiveDate,
    base_currency: &str,
) -> Result<f64> {
    use crate::currency;

    let account_ids = get_linked_account_ids(conn, portfolio_id)?;

    if account_ids.is_empty() {
        return Ok(0.0);
    }

    let mut total_cash = 0.0;

    for account_id in account_ids {
        let (balance, account_currency) = get_account_balance_at_date(conn, account_id, date)?;

        // Convert to base currency if needed
        let balance_base = if account_currency != base_currency && !account_currency.is_empty() {
            currency::convert(conn, balance, &account_currency, base_currency, date)
                .unwrap_or(balance)
        } else {
            balance
        };

        total_cash += balance_base;
    }

    log::debug!("Total cash balance for portfolio {} at {}: {:.2} {}",
                portfolio_id, date, total_cash, base_currency);
    Ok(total_cash)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== TTWROR Tests ====================

    #[test]
    fn test_ttwror_no_cash_flows() {
        // Simple case: Start 1000, End 1100, no cash flows
        // TTWROR = 1100/1000 - 1 = 0.10 (10%)
        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 1000.0),
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 1100.0),
        ];
        let cash_flows = vec![];

        let (total_return, periods) = calculate_ttwror_from_data(&valuations, &cash_flows);

        assert!((total_return - 0.10).abs() < 0.001, "Expected 10%, got {:.2}%", total_return * 100.0);
        assert_eq!(periods.len(), 1);
    }

    #[test]
    fn test_ttwror_with_deposit() {
        // End-of-day convention: Cashflow affects ending NAV, is subtracted from return
        //
        // Start: 1000, Deposit 500 mid-year, End: 1600
        // NAV at mid-year INCLUDES the deposit (end-of-day: CF already in)
        // Period 1: 1000 → 1550 (value AFTER deposit), CF=500 at period end
        //           r = (1550 - 500) / 1000 - 1 = 5%
        // Period 2: 1550 → 1600, CF=0
        //           r = (1600 - 0) / 1550 - 1 = 3.23%
        // TTWROR = (1.05 * 1.0323) - 1 = 8.39%
        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 1000.0),
            (NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(), 1550.0),  // Value AFTER deposit (incl. 500)
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 1600.0),
        ];
        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
                amount: 500.0,  // Deposit at end of period 1
            },
        ];

        let (total_return, periods) = calculate_ttwror_from_data(&valuations, &cash_flows);

        // End-of-day formula: r = (V_end - CF) / V_start - 1
        // Period 1: (1550 - 500) / 1000 - 1 = 0.05 (5%)
        // Period 2: (1600 - 0) / 1550 - 1 = 0.0323 (3.23%)
        // Total: (1.05 * 1.0323) - 1 = 0.0839 (8.39%)
        assert!((total_return - 0.0839).abs() < 0.01, "Expected ~8.4%, got {:.2}%", total_return * 100.0);
        assert_eq!(periods.len(), 2);
    }

    #[test]
    fn test_ttwror_with_withdrawal() {
        // End-of-day convention: Cashflow affects ending NAV, is subtracted from return
        //
        // Start: 2000, Withdrawal 500 mid-year, End: 1600
        // NAV at mid-year is AFTER the withdrawal (end-of-day: CF already out)
        // Period 1: 2000 → 1400 (value AFTER withdrawal), CF=-500 at period end
        //           r = (1400 - (-500)) / 2000 - 1 = 1900/2000 - 1 = -5%
        // Period 2: 1400 → 1600, CF=0
        //           r = (1600 - 0) / 1400 - 1 = 14.29%
        // TTWROR = (0.95 * 1.1429) - 1 = 8.57%
        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 2000.0),
            (NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(), 1400.0),  // Value AFTER withdrawal (excl. 500)
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 1600.0),
        ];
        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
                amount: -500.0,  // Withdrawal (negative) at end of period 1
            },
        ];

        let (total_return, periods) = calculate_ttwror_from_data(&valuations, &cash_flows);

        // End-of-day formula: r = (V_end - CF) / V_start - 1
        // Period 1: (1400 - (-500)) / 2000 - 1 = 1900/2000 - 1 = -0.05 (-5%)
        // Period 2: (1600 - 0) / 1400 - 1 = 0.1429 (14.29%)
        // Total: (0.95 * 1.1429) - 1 = 0.0857 (8.57%)
        assert!((total_return - 0.0857).abs() < 0.01, "Expected ~8.6%, got {:.2}%", total_return * 100.0);
        assert_eq!(periods.len(), 2);
    }

    #[test]
    fn test_ttwror_geometric_chaining() {
        // Verify geometric vs arithmetic difference
        // 3 periods: +10%, -5%, +8%
        // Geometric: (1.10 * 0.95 * 1.08) - 1 = 12.86%
        // Arithmetic (wrong): 10 - 5 + 8 = 13%
        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 1000.0),
            (NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(), 1100.0),  // +10%
            (NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(), 1045.0),  // -5% from 1100
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 1128.6), // +8% from 1045
        ];
        let cash_flows = vec![];  // No cash flows - pure performance

        let (total_return, periods) = calculate_ttwror_from_data(&valuations, &cash_flows);

        // Expected: (1.10 * 0.95 * 1.08) - 1 = 0.1286 (12.86%)
        assert!((total_return - 0.1286).abs() < 0.01, "Expected ~12.86%, got {:.2}%", total_return * 100.0);
        assert_eq!(periods.len(), 1);  // No cash flows = single period
    }

    #[test]
    fn test_ttwror_negative_return() {
        // Loss scenario: Start 1000, End 800
        // TTWROR = 800/1000 - 1 = -20%
        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 1000.0),
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 800.0),
        ];
        let cash_flows = vec![];

        let (total_return, _) = calculate_ttwror_from_data(&valuations, &cash_flows);

        assert!((total_return - (-0.20)).abs() < 0.001, "Expected -20%, got {:.2}%", total_return * 100.0);
    }

    // ==================== IRR Tests ====================

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

    #[test]
    fn test_irr_with_dividend() {
        // Invest 1000, receive 50 dividend mid-year, end value 1050
        // Total return: 1000 → 1050 + 50 = 1100 (10%)
        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                amount: 1000.0,  // Initial investment
            },
            CashFlow {
                date: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
                amount: -50.0,  // Dividend received (negative = return to investor)
            },
        ];

        let result = calculate_irr(
            &cash_flows,
            1050.0,  // End value (without the already-received dividend)
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        ).unwrap();

        assert!(result.converged);
        // IRR should be around 10% (slightly higher due to early dividend)
        assert!(result.irr > 0.09 && result.irr < 0.12, "IRR should be ~10%, got {:.2}%", result.irr * 100.0);
    }

    #[test]
    fn test_irr_negative_return() {
        // Invest 1000, get back 900 = -10% return
        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                amount: 1000.0,
            },
        ];

        let result = calculate_irr(
            &cash_flows,
            900.0,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ).unwrap();

        assert!(result.converged);
        assert!((result.irr - (-0.10)).abs() < 0.01, "Expected -10%, got {:.2}%", result.irr * 100.0);
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_find_value_at_or_near_exact() {
        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 1000.0),
            (NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), 1100.0),
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 1200.0),
        ];

        let value = find_value_at_or_near(&valuations, NaiveDate::from_ymd_opt(2024, 6, 1).unwrap());
        assert!((value - 1100.0).abs() < 0.01);
    }

    #[test]
    fn test_find_value_at_or_near_closest() {
        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 1000.0),
            (NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(), 1100.0),
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 1200.0),
        ];

        // Looking for May 15 should find June 1 (closest before doesn't exist, use closest after)
        // Actually, closest BEFORE should be Jan 1
        let value = find_value_at_or_near(&valuations, NaiveDate::from_ymd_opt(2024, 5, 15).unwrap());
        assert!((value - 1000.0).abs() < 0.01, "Should use closest before (Jan 1 = 1000)");
    }

    // ==================== E2E Tests with Database ====================

    /// Create an in-memory test database with the required schema
    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(r#"
            -- Core tables
            CREATE TABLE pp_account (
                id INTEGER PRIMARY KEY,
                uuid TEXT,
                name TEXT,
                currency TEXT DEFAULT 'EUR',
                is_retired INTEGER DEFAULT 0
            );

            CREATE TABLE pp_portfolio (
                id INTEGER PRIMARY KEY,
                uuid TEXT,
                name TEXT,
                reference_account_id INTEGER,
                is_retired INTEGER DEFAULT 0
            );

            CREATE TABLE pp_security (
                id INTEGER PRIMARY KEY,
                uuid TEXT,
                name TEXT,
                currency TEXT DEFAULT 'EUR',
                isin TEXT
            );

            CREATE TABLE pp_txn (
                id INTEGER PRIMARY KEY,
                uuid TEXT,
                owner_type TEXT,
                owner_id INTEGER,
                security_id INTEGER,
                txn_type TEXT,
                date TEXT,
                amount INTEGER,
                currency TEXT,
                shares INTEGER,
                cross_entry_uuid TEXT
            );

            CREATE TABLE pp_cross_entry (
                id INTEGER PRIMARY KEY,
                portfolio_txn_id INTEGER,
                account_txn_id INTEGER
            );

            CREATE TABLE pp_price (
                security_id INTEGER,
                date TEXT,
                value INTEGER,
                PRIMARY KEY (security_id, date)
            );

            CREATE TABLE pp_latest_price (
                security_id INTEGER PRIMARY KEY,
                date TEXT,
                value INTEGER
            );

            CREATE TABLE pp_exchange_rate (
                base_currency TEXT,
                target_currency TEXT,
                date TEXT,
                rate REAL,
                PRIMARY KEY (base_currency, target_currency, date)
            );

            CREATE TABLE pp_settings (
                id INTEGER PRIMARY KEY,
                import_id INTEGER,
                settings_json TEXT
            );

            -- Insert base currency setting
            INSERT INTO pp_settings (id, import_id, settings_json)
            VALUES (1, 1, '{"baseCurrency": "EUR"}');
        "#).unwrap();

        conn
    }

    #[test]
    fn test_e2e_account_balance_calculation() {
        let conn = create_test_db();

        // Create account
        conn.execute(
            "INSERT INTO pp_account (id, uuid, name, currency) VALUES (1, 'acc-1', 'Test Account', 'EUR')",
            []
        ).unwrap();

        // Add transactions: DEPOSIT 1000, BUY 500, DIVIDENDS 50
        conn.execute(
            "INSERT INTO pp_txn (id, owner_type, owner_id, txn_type, date, amount) VALUES (1, 'account', 1, 'DEPOSIT', '2024-01-01', 100000)",
            []
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_txn (id, owner_type, owner_id, txn_type, date, amount) VALUES (2, 'account', 1, 'BUY', '2024-01-15', 50000)",
            []
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_txn (id, owner_type, owner_id, txn_type, date, amount) VALUES (3, 'account', 1, 'DIVIDENDS', '2024-06-01', 5000)",
            []
        ).unwrap();

        // Test balance: 1000 - 500 + 50 = 550 EUR
        let (balance, currency) = get_account_balance_at_date(
            &conn, 1, NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()
        ).unwrap();

        assert_eq!(currency, "EUR");
        assert!((balance - 550.0).abs() < 0.01, "Expected 550, got {}", balance);
    }

    #[test]
    fn test_e2e_linked_accounts() {
        let conn = create_test_db();

        // Create account and portfolio with reference
        conn.execute(
            "INSERT INTO pp_account (id, uuid, name, currency) VALUES (1, 'acc-1', 'Test Account', 'EUR')",
            []
        ).unwrap();
        conn.execute(
            "INSERT INTO pp_portfolio (id, uuid, name, reference_account_id) VALUES (1, 'port-1', 'Test Portfolio', 1)",
            []
        ).unwrap();

        // Test linked accounts
        let linked = get_linked_account_ids(&conn, 1).unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0], 1);
    }

    #[test]
    fn test_e2e_flow_adjusted_returns() {
        // Test the flow-adjusted returns calculation
        let values = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 1000.0),
            (NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(), 1550.0),  // 1050 market + 500 deposit
            (NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(), 1600.0),
        ];

        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                amount: 500.0,  // Deposit
            },
        ];

        let returns = calculate_flow_adjusted_returns(&values, &cash_flows);

        // Day 1→2: (1550 - 500) / 1000 - 1 = 5%
        // Day 2→3: (1600 - 0) / 1550 - 1 = 3.23%
        assert_eq!(returns.len(), 2);
        assert!((returns[0] - 0.05).abs() < 0.001, "Expected 5%, got {:.2}%", returns[0] * 100.0);
        assert!((returns[1] - 0.0323).abs() < 0.01, "Expected ~3.2%, got {:.2}%", returns[1] * 100.0);
    }

    #[test]
    fn test_e2e_ttwror_formula_verification() {
        // Verify the end-of-day TTWROR formula with a concrete example
        //
        // Scenario:
        // - Start with 10,000 EUR
        // - Market gains 5% → 10,500 EUR
        // - Deposit 5,000 EUR at end of period → NAV = 15,500 EUR
        // - Market gains 10% → 17,050 EUR
        //
        // Expected TTWROR:
        // Period 1: (15500 - 5000) / 10000 - 1 = 5%
        // Period 2: (17050 - 0) / 15500 - 1 = 10%
        // Total: (1.05 × 1.10) - 1 = 15.5%

        let valuations = vec![
            (NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 10000.0),
            (NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(), 15500.0),  // After 5% gain + 5000 deposit
            (NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(), 17050.0), // After 10% gain
        ];

        let cash_flows = vec![
            CashFlow {
                date: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
                amount: 5000.0,
            },
        ];

        let (total_return, periods) = calculate_ttwror_from_data(&valuations, &cash_flows);

        // Verify individual periods
        assert_eq!(periods.len(), 2);

        // Period 1: 5% return
        assert!(
            (periods[0].return_rate - 0.05).abs() < 0.001,
            "Period 1 expected 5%, got {:.2}%", periods[0].return_rate * 100.0
        );

        // Period 2: 10% return
        assert!(
            (periods[1].return_rate - 0.10).abs() < 0.001,
            "Period 2 expected 10%, got {:.2}%", periods[1].return_rate * 100.0
        );

        // Total: 15.5%
        assert!(
            (total_return - 0.155).abs() < 0.001,
            "Total TTWROR expected 15.5%, got {:.2}%", total_return * 100.0
        );
    }
}
