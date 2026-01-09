//! Report generation commands for Tauri
//!
//! Generates various financial reports:
//! - Dividend Report: All dividends received in a period
//! - Tax Report: Taxable events (dividends, realized gains)
//! - Realized Gains Report: Gains/losses from sales

use crate::db;
use crate::pp::common::{prices, shares};
use serde::{Deserialize, Serialize};
use tauri::command;

// ============================================================================
// Types
// ============================================================================

/// Single dividend payment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendEntry {
    pub date: String,
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub portfolio_name: String,
    /// Gross dividend amount (before taxes)
    pub gross_amount: f64,
    pub currency: String,
    /// Tax withheld
    pub taxes: f64,
    /// Net dividend (after taxes)
    pub net_amount: f64,
    /// Shares held at dividend date
    pub shares: Option<f64>,
    /// Dividend per share (if calculable)
    pub per_share: Option<f64>,
}

/// Dividend report summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendReport {
    pub start_date: String,
    pub end_date: String,
    pub total_gross: f64,
    pub total_taxes: f64,
    pub total_net: f64,
    pub currency: String,
    pub entries: Vec<DividendEntry>,
    /// Grouped by security
    pub by_security: Vec<DividendBySecurity>,
    /// Grouped by month
    pub by_month: Vec<DividendByMonth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendBySecurity {
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub total_gross: f64,
    pub total_taxes: f64,
    pub total_net: f64,
    pub payment_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DividendByMonth {
    pub month: String, // YYYY-MM
    pub total_gross: f64,
    pub total_taxes: f64,
    pub total_net: f64,
}

/// Realized gain/loss from a sale
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealizedGain {
    pub date: String,
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub portfolio_name: String,
    /// Number of shares sold
    pub shares: f64,
    /// Sale proceeds (gross)
    pub proceeds: f64,
    /// Cost basis of sold shares
    pub cost_basis: f64,
    /// Realized gain/loss
    pub gain: f64,
    /// Gain as percentage
    pub gain_percent: f64,
    /// Holding period in days
    pub holding_days: i32,
    /// True if held > 1 year (for tax purposes)
    pub is_long_term: bool,
    pub currency: String,
    /// Fees paid on sale
    pub fees: f64,
    /// Taxes paid on sale
    pub taxes: f64,
}

/// Realized gains report summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealizedGainsReport {
    pub start_date: String,
    pub end_date: String,
    pub total_proceeds: f64,
    pub total_cost_basis: f64,
    pub total_gain: f64,
    pub total_fees: f64,
    pub total_taxes: f64,
    pub currency: String,
    /// All individual gains
    pub entries: Vec<RealizedGain>,
    /// Gains grouped by security
    pub by_security: Vec<GainBySecurity>,
    /// Short-term vs long-term summary
    pub short_term_gain: f64,
    pub long_term_gain: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GainBySecurity {
    pub security_id: i64,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub total_proceeds: f64,
    pub total_cost_basis: f64,
    pub total_gain: f64,
    pub sale_count: i32,
}

/// Tax report combining dividends and realized gains
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxReport {
    pub year: i32,
    pub currency: String,
    /// Dividend income
    pub dividend_income: f64,
    pub dividend_taxes_withheld: f64,
    /// Capital gains
    pub short_term_gains: f64,
    pub long_term_gains: f64,
    pub total_capital_gains: f64,
    /// Fees (may be tax-deductible)
    pub total_fees: f64,
    /// Taxes paid on capital gains
    pub capital_gains_taxes: f64,
    /// Detailed reports
    pub dividends: DividendReport,
    pub realized_gains: RealizedGainsReport,
}

// ============================================================================
// Commands
// ============================================================================

/// Generate dividend report for a period
#[command]
pub fn generate_dividend_report(
    start_date: String,
    end_date: String,
    portfolio_id: Option<i64>,
) -> Result<DividendReport, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get base currency
    let base_currency: String = conn
        .query_row(
            "SELECT base_currency FROM pp_import ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "EUR".to_string());

    // Build query with optional portfolio filter using parameterized query
    let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(pid) = portfolio_id {
        (
            r#"
            SELECT
                t.date,
                s.id as security_id,
                s.name as security_name,
                s.isin,
                CASE t.owner_type
                    WHEN 'account' THEN (SELECT name FROM pp_account WHERE id = t.owner_id)
                    ELSE 'N/A'
                END as portfolio_name,
                t.amount as gross_amount,
                t.currency,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0) as taxes,
                t.shares
            FROM pp_txn t
            LEFT JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
              AND t.date >= ?1 AND t.date <= ?2
              AND t.owner_id = ?3
            ORDER BY t.date DESC
            "#.to_string(),
            vec![
                Box::new(start_date.clone()) as Box<dyn rusqlite::ToSql>,
                Box::new(end_date.clone()),
                Box::new(pid),
            ],
        )
    } else {
        (
            r#"
            SELECT
                t.date,
                s.id as security_id,
                s.name as security_name,
                s.isin,
                CASE t.owner_type
                    WHEN 'account' THEN (SELECT name FROM pp_account WHERE id = t.owner_id)
                    ELSE 'N/A'
                END as portfolio_name,
                t.amount as gross_amount,
                t.currency,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0) as taxes,
                t.shares
            FROM pp_txn t
            LEFT JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
              AND t.date >= ?1 AND t.date <= ?2
            ORDER BY t.date DESC
            "#.to_string(),
            vec![
                Box::new(start_date.clone()) as Box<dyn rusqlite::ToSql>,
                Box::new(end_date.clone()),
            ],
        )
    };

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, Option<i64>>(8)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut entries: Vec<DividendEntry> = Vec::new();
    let mut total_gross = 0.0;
    let mut total_taxes = 0.0;

    for row in rows.flatten() {
        let (date, security_id, security_name, isin, portfolio_name, gross_raw, currency, taxes_raw, shares_raw) = row;

        let gross = gross_raw as f64 / 100.0;
        let taxes = taxes_raw as f64 / 100.0;
        let net = gross - taxes;
        let shares_val = shares_raw.map(|s| shares::to_decimal(s));
        let per_share = shares_val.filter(|s| *s > 0.0).map(|s| gross / s);

        total_gross += gross;
        total_taxes += taxes;

        entries.push(DividendEntry {
            date,
            security_id,
            security_name,
            security_isin: isin,
            portfolio_name,
            gross_amount: gross,
            currency,
            taxes,
            net_amount: net,
            shares: shares_val,
            per_share,
        });
    }

    // Group by security
    let mut by_security_map: std::collections::HashMap<i64, DividendBySecurity> = std::collections::HashMap::new();
    for entry in &entries {
        let sec = by_security_map.entry(entry.security_id).or_insert(DividendBySecurity {
            security_id: entry.security_id,
            security_name: entry.security_name.clone(),
            security_isin: entry.security_isin.clone(),
            total_gross: 0.0,
            total_taxes: 0.0,
            total_net: 0.0,
            payment_count: 0,
        });
        sec.total_gross += entry.gross_amount;
        sec.total_taxes += entry.taxes;
        sec.total_net += entry.net_amount;
        sec.payment_count += 1;
    }
    let mut by_security: Vec<DividendBySecurity> = by_security_map.into_values().collect();
    by_security.sort_by(|a, b| b.total_gross.partial_cmp(&a.total_gross).unwrap_or(std::cmp::Ordering::Equal));

    // Group by month
    let mut by_month_map: std::collections::HashMap<String, DividendByMonth> = std::collections::HashMap::new();
    for entry in &entries {
        let month = entry.date[..7].to_string(); // YYYY-MM
        let m = by_month_map.entry(month.clone()).or_insert(DividendByMonth {
            month,
            total_gross: 0.0,
            total_taxes: 0.0,
            total_net: 0.0,
        });
        m.total_gross += entry.gross_amount;
        m.total_taxes += entry.taxes;
        m.total_net += entry.net_amount;
    }
    let mut by_month: Vec<DividendByMonth> = by_month_map.into_values().collect();
    by_month.sort_by(|a, b| a.month.cmp(&b.month));

    Ok(DividendReport {
        start_date,
        end_date,
        total_gross,
        total_taxes,
        total_net: total_gross - total_taxes,
        currency: base_currency,
        entries,
        by_security,
        by_month,
    })
}

/// Generate realized gains report for a period
#[command]
pub fn generate_realized_gains_report(
    start_date: String,
    end_date: String,
    portfolio_id: Option<i64>,
) -> Result<RealizedGainsReport, String> {
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

    // Get SELL transactions with FIFO cost basis - use parameterized query
    let (query, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(pid) = portfolio_id {
        (
            r#"
            SELECT
                t.id,
                t.date,
                s.id as security_id,
                s.name as security_name,
                s.isin,
                p.name as portfolio_name,
                t.shares,
                t.amount as proceeds,
                t.currency,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'FEE'), 0) as fees,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0) as taxes
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            JOIN pp_portfolio p ON p.id = t.owner_id
            WHERE t.txn_type = 'SELL'
              AND t.owner_type = 'portfolio'
              AND t.date >= ?1 AND t.date <= ?2
              AND t.owner_id = ?3
            ORDER BY t.date DESC
            "#.to_string(),
            vec![
                Box::new(start_date.clone()) as Box<dyn rusqlite::ToSql>,
                Box::new(end_date.clone()),
                Box::new(pid),
            ],
        )
    } else {
        (
            r#"
            SELECT
                t.id,
                t.date,
                s.id as security_id,
                s.name as security_name,
                s.isin,
                p.name as portfolio_name,
                t.shares,
                t.amount as proceeds,
                t.currency,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'FEE'), 0) as fees,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0) as taxes
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            JOIN pp_portfolio p ON p.id = t.owner_id
            WHERE t.txn_type = 'SELL'
              AND t.owner_type = 'portfolio'
              AND t.date >= ?1 AND t.date <= ?2
            ORDER BY t.date DESC
            "#.to_string(),
            vec![
                Box::new(start_date.clone()) as Box<dyn rusqlite::ToSql>,
                Box::new(end_date.clone()),
            ],
        )
    };

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut entries: Vec<RealizedGain> = Vec::new();
    let mut total_proceeds = 0.0;
    let mut total_cost_basis = 0.0;
    let mut total_fees = 0.0;
    let mut total_taxes = 0.0;
    let mut short_term_gain = 0.0;
    let mut long_term_gain = 0.0;

    for row in rows.flatten() {
        let (txn_id, date, security_id, security_name, isin, portfolio_name, shares_raw, proceeds_raw, currency, fees_raw, taxes_raw) = row;

        let shares_sold = shares::to_decimal(shares_raw);
        let proceeds = proceeds_raw as f64 / 100.0;
        let fees = fees_raw as f64 / 100.0;
        let taxes = taxes_raw as f64 / 100.0;

        // Get FIFO cost basis for this sale from consumption records
        // Use gross_amount from consumption which is the proportional cost basis (INCLUDING fees/taxes)
        let (cost_basis, avg_holding_days): (f64, i32) = conn
            .query_row(
                r#"
                SELECT
                    COALESCE(SUM(fc.gross_amount) / 100.0, 0),
                    COALESCE(AVG(julianday(?) - julianday(fl.purchase_date)), 0)
                FROM pp_fifo_consumption fc
                JOIN pp_fifo_lot fl ON fl.id = fc.lot_id
                WHERE fc.sale_txn_id = ?
                "#,
                rusqlite::params![date, txn_id],
                |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)? as i32)),
            )
            .unwrap_or((0.0, 0));

        let gain = proceeds - cost_basis - fees;
        let gain_percent = if cost_basis > 0.0 { (gain / cost_basis) * 100.0 } else { 0.0 };
        let is_long_term = avg_holding_days > 365;

        total_proceeds += proceeds;
        total_cost_basis += cost_basis;
        total_fees += fees;
        total_taxes += taxes;

        if is_long_term {
            long_term_gain += gain;
        } else {
            short_term_gain += gain;
        }

        entries.push(RealizedGain {
            date,
            security_id,
            security_name,
            security_isin: isin,
            portfolio_name,
            shares: shares_sold,
            proceeds,
            cost_basis,
            gain,
            gain_percent,
            holding_days: avg_holding_days,
            is_long_term,
            currency,
            fees,
            taxes,
        });
    }

    // Group by security
    let mut by_security_map: std::collections::HashMap<i64, GainBySecurity> = std::collections::HashMap::new();
    for entry in &entries {
        let sec = by_security_map.entry(entry.security_id).or_insert(GainBySecurity {
            security_id: entry.security_id,
            security_name: entry.security_name.clone(),
            security_isin: entry.security_isin.clone(),
            total_proceeds: 0.0,
            total_cost_basis: 0.0,
            total_gain: 0.0,
            sale_count: 0,
        });
        sec.total_proceeds += entry.proceeds;
        sec.total_cost_basis += entry.cost_basis;
        sec.total_gain += entry.gain;
        sec.sale_count += 1;
    }
    let mut by_security: Vec<GainBySecurity> = by_security_map.into_values().collect();
    by_security.sort_by(|a, b| b.total_gain.partial_cmp(&a.total_gain).unwrap_or(std::cmp::Ordering::Equal));

    Ok(RealizedGainsReport {
        start_date,
        end_date,
        total_proceeds,
        total_cost_basis,
        total_gain: total_proceeds - total_cost_basis - total_fees,
        total_fees,
        total_taxes,
        currency: base_currency,
        entries,
        by_security,
        short_term_gain,
        long_term_gain,
    })
}

/// Generate combined tax report for a year
#[command]
pub fn generate_tax_report(year: i32) -> Result<TaxReport, String> {
    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-12-31", year);

    let dividends = generate_dividend_report(start_date.clone(), end_date.clone(), None)?;
    let realized_gains = generate_realized_gains_report(start_date, end_date, None)?;

    Ok(TaxReport {
        year,
        currency: dividends.currency.clone(),
        dividend_income: dividends.total_gross,
        dividend_taxes_withheld: dividends.total_taxes,
        short_term_gains: realized_gains.short_term_gain,
        long_term_gains: realized_gains.long_term_gain,
        total_capital_gains: realized_gains.total_gain,
        total_fees: realized_gains.total_fees,
        capital_gains_taxes: realized_gains.total_taxes,
        dividends,
        realized_gains,
    })
}

/// Get dividend yield for a security based on last 12 months
#[command]
pub fn get_dividend_yield(security_id: i64) -> Result<f64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get total dividends in last 12 months
    let end_date = chrono::Utc::now().date_naive();
    let start_date = end_date - chrono::Duration::days(365);

    let total_dividends: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(amount), 0) / 100.0
            FROM pp_txn
            WHERE txn_type = 'DIVIDENDS' AND security_id = ?
              AND date >= ? AND date <= ?
            "#,
            rusqlite::params![security_id, start_date.to_string(), end_date.to_string()],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Get current shares held
    let shares_held: i64 = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(CASE
                WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                ELSE 0
            END), 0)
            FROM pp_txn
            WHERE security_id = ? AND owner_type = 'portfolio'
            "#,
            [security_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if shares_held == 0 {
        return Ok(0.0);
    }

    // Get current price
    let current_price: Option<i64> = conn
        .query_row(
            "SELECT value FROM pp_latest_price WHERE security_id = ?",
            [security_id],
            |row| row.get(0),
        )
        .ok();

    let price = current_price.map(prices::to_decimal).unwrap_or(0.0);
    if price <= 0.0 {
        return Ok(0.0);
    }

    let shares_decimal = shares::to_decimal(shares_held);
    let total_value = shares_decimal * price;
    let dividend_yield = (total_dividends / total_value) * 100.0;

    Ok(dividend_yield)
}
