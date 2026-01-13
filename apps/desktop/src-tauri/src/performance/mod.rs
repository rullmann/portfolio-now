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

/// Calculate TTWROR for a portfolio using Simple Return based on Cost Basis
///
/// Simplified formula that matches the Dashboard box values:
/// ```text
/// TTWROR = (Current Value - Cost Basis) / Cost Basis
/// ```
///
/// This approach:
/// - Uses current portfolio value with currency conversion (same as "Portfolio" box)
/// - Uses FIFO cost basis from lots (same as "Einstand" box)
/// - Provides consistent results with Dashboard display
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

    // Get current portfolio value with currency conversion (matches "Portfolio" box)
    let current_value = get_portfolio_value_with_currency(conn, portfolio_id)?;

    // Get cost basis from FIFO lots with currency conversion (matches "Einstand" box)
    let cost_basis = get_total_cost_basis_with_currency(conn, portfolio_id)?;

    log::info!(
        "TTWROR: Current Value={:.2}, Cost Basis={:.2}, days={}",
        current_value, cost_basis, days
    );

    // Simple return: (Current - Cost) / Cost
    let total_return = if cost_basis > 0.0 {
        (current_value - cost_basis) / cost_basis
    } else {
        log::warn!("TTWROR: Cost basis is 0, cannot calculate return");
        0.0
    };

    // Annualize: (1 + r)^(365/days) - 1
    let annualized_return = if days > 0 && total_return > -1.0 {
        (1.0 + total_return).powf(365.0 / days as f64) - 1.0
    } else {
        0.0
    };

    log::info!(
        "TTWROR result: total={:.2}%, annualized={:.2}%",
        total_return * 100.0,
        annualized_return * 100.0
    );

    // Create a single period for the entire range
    let periods = vec![PeriodReturn {
        start_date,
        end_date,
        start_value: cost_basis,
        end_value: current_value,
        cash_flow: 0.0,
        return_rate: total_return,
    }];

    Ok(TtwrorResult {
        total_return,
        annualized_return,
        days,
        periods,
    })
}

/// Get current portfolio value with currency conversion to base currency
/// This matches the calculation used in get_all_holdings() for the Dashboard
fn get_portfolio_value_with_currency(
    conn: &Connection,
    portfolio_id: Option<i64>,
) -> Result<f64> {
    use crate::currency;

    // Get base currency
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
                row.get::<_, i64>(0)?,                           // security_id
                row.get::<_, Option<String>>(1)?.unwrap_or_default(), // currency
                row.get::<_, i64>(2)?,                           // net_shares
                row.get::<_, Option<i64>>(3)?,                   // latest_price
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

    log::info!("TTWROR: Portfolio value with currency conversion: {:.2}", total_value);
    Ok(total_value)
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
/// For TTWROR, we need only EXTERNAL cash flows - money coming into or leaving
/// the portfolio from outside. This does NOT include:
/// - BUY/SELL: These are INTERNAL transactions (cash ↔ securities within portfolio)
/// - TRANSFER_IN/OUT: Asset movements, not cash
/// - DELIVERY_INBOUND/OUTBOUND: Asset movements, not cash
///
/// Only DEPOSIT and REMOVAL represent external cash flows.
fn get_cash_flows(
    conn: &Connection,
    portfolio_id: Option<i64>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<Vec<CashFlow>> {
    let mut cash_flows: Vec<CashFlow> = Vec::new();

    // Get account-level external cash flows (DEPOSIT/REMOVAL)
    if let Some(pid) = portfolio_id {
        // For specific portfolio, use its reference account
        let account_sql = r#"
            SELECT t.date, t.txn_type, t.amount
            FROM pp_txn t
            JOIN pp_portfolio p ON p.reference_account_id = t.owner_id
            WHERE t.owner_type = 'account'
              AND t.txn_type IN ('DEPOSIT', 'REMOVAL')
              AND p.id = ?1
              AND date(t.date) >= ?2 AND date(t.date) <= ?3
            ORDER BY t.date
        "#;

        let mut stmt = conn.prepare(account_sql)?;
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
            if let Some(date) = parse_date_flexible(&date_str) {
                let amount_f = amount as f64 / AMOUNT_SCALE;
                let cf_amount = match txn_type.as_str() {
                    "DEPOSIT" => amount_f,   // Money added (positive)
                    "REMOVAL" => -amount_f,  // Money removed (negative)
                    _ => 0.0,
                };
                if cf_amount != 0.0 {
                    cash_flows.push(CashFlow { date, amount: cf_amount });
                }
            }
        }
    } else {
        // For all portfolios, get all account DEPOSIT/REMOVAL
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
            if let Some(date) = parse_date_flexible(&date_str) {
                let amount_f = amount as f64 / AMOUNT_SCALE;
                let cf_amount = match txn_type.as_str() {
                    "DEPOSIT" => amount_f,   // Money added (positive)
                    "REMOVAL" => -amount_f,  // Money removed (negative)
                    _ => 0.0,
                };
                if cf_amount != 0.0 {
                    cash_flows.push(CashFlow { date, amount: cf_amount });
                }
            }
        }
    }

    // Sort by date
    cash_flows.sort_by(|a, b| a.date.cmp(&b.date));

    log::info!("TTWROR: Found {} external cash flows (DEPOSIT/REMOVAL only)", cash_flows.len());

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
        .and_then(|s| parse_date_flexible(&s))
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());

    let end = max_date
        .and_then(|s| parse_date_flexible(&s))
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
