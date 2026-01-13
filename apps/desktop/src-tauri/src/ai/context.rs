//! Portfolio context loading for AI analysis
//!
//! This module provides functions to load portfolio data from the database
//! and prepare it for AI analysis (chat, insights, etc.)

use crate::ai::{
    DividendPayment, FeesAndTaxesSummary, HoldingSummary, InvestmentSummary, PortfolioExtremes,
    PortfolioInsightsContext, QuoteSyncInfo, QuoteProviderStatusSummary, RecentTransaction,
    SectorAllocation, SoldPosition, WatchlistItem, YearlyFeesAndTaxes, YearlyOverview,
};
use crate::currency;
use crate::db;
use crate::performance;
use crate::pp::common::{prices, shares};
use chrono::{Datelike, NaiveDate, Utc};
use rusqlite::Connection;

/// Load portfolio context from database for AI analysis
///
/// # Arguments
/// * `base_currency` - Base currency for value conversion (e.g., "EUR")
/// * `user_name` - Optional user name for personalized chat
///
/// # Returns
/// Complete portfolio context including holdings, transactions, dividends,
/// watchlist, fees/taxes, sector allocation, and historical data
pub fn load_portfolio_context(
    base_currency: &str,
    user_name: Option<String>,
) -> Result<PortfolioInsightsContext, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let today = Utc::now().date_naive();

    // Load ALL holdings with current values (matching get_all_holdings logic)
    let holdings_sql = r#"
        SELECT security_id, name, currency, isin, ticker, net_shares, latest_price
        FROM (
            SELECT
                s.id as security_id,
                s.name,
                s.currency,
                s.isin,
                s.ticker,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) as net_shares,
                lp.value as latest_price
            FROM pp_txn t
            JOIN pp_portfolio p ON p.id = t.owner_id AND t.owner_type = 'portfolio'
            JOIN pp_security s ON s.id = t.security_id
            LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
            WHERE t.shares IS NOT NULL
            GROUP BY s.id
        )
        WHERE net_shares > 0
        ORDER BY net_shares * COALESCE(latest_price, 0) DESC
    "#;

    let mut stmt = conn.prepare(holdings_sql).map_err(|e| e.to_string())?;
    let rows = stmt.query([]).map_err(|e| e.to_string())?;

    let mut holdings: Vec<HoldingSummary> = Vec::new();
    let mut total_value: f64 = 0.0;
    let mut total_cost_basis: f64 = 0.0;
    let mut currency_values: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

    let holdings_data: Vec<(i64, String, Option<String>, Option<String>, String, i64, Option<i64>)> = rows
        .mapped(|row| {
            Ok((
                row.get::<_, i64>(0)?,              // security_id
                row.get::<_, String>(1)?,           // name
                row.get::<_, Option<String>>(3)?,   // isin
                row.get::<_, Option<String>>(4)?,   // ticker
                row.get::<_, String>(2)?,           // currency
                row.get::<_, i64>(5)?,              // shares (scaled)
                row.get::<_, Option<i64>>(6)?,      // latest_price (scaled)
            ))
        })
        .filter_map(|r| r.ok())
        .collect();

    // Get cost basis for all securities using SINGLE SOURCE OF TRUTH
    let cost_map = crate::fifo::get_cost_basis_by_security_id_converted(conn, base_currency)
        .unwrap_or_default();

    // Get first buy date per security
    let first_buy_sql = r#"
        SELECT security_id, MIN(date) as first_buy
        FROM pp_txn
        WHERE txn_type IN ('BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN')
          AND security_id IS NOT NULL
        GROUP BY security_id
    "#;
    let mut first_buy_map: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    if let Ok(mut stmt) = conn.prepare(first_buy_sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                first_buy_map.insert(row.0, row.1);
            }
        }
    }

    // Get fees and taxes per security
    let fees_taxes_sql = r#"
        SELECT t.security_id,
               COALESCE(SUM(CASE WHEN u.unit_type = 'FEE' THEN u.amount ELSE 0 END), 0) as fees,
               COALESCE(SUM(CASE WHEN u.unit_type = 'TAX' THEN u.amount ELSE 0 END), 0) as taxes
        FROM pp_txn t
        LEFT JOIN pp_txn_unit u ON u.txn_id = t.id
        WHERE t.security_id IS NOT NULL
        GROUP BY t.security_id
    "#;
    let mut fees_taxes_map: std::collections::HashMap<i64, (f64, f64)> = std::collections::HashMap::new();
    if let Ok(mut stmt) = conn.prepare(fees_taxes_sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                let (sec_id, fees_cents, taxes_cents) = row;
                fees_taxes_map.insert(sec_id, (fees_cents as f64 / 100.0, taxes_cents as f64 / 100.0));
            }
        }
    }

    for (security_id, name, isin, ticker, security_currency, shares_scaled, price_scaled) in holdings_data {
        let shares_val = shares::to_decimal(shares_scaled);

        // Handle GBX/GBp (British Pence) - divide by 100 to get GBP
        let price_val = price_scaled.map(|p| {
            let price_decimal = prices::to_decimal(p);
            if security_currency == "GBX" || security_currency == "GBp" {
                price_decimal / 100.0
            } else {
                price_decimal
            }
        }).unwrap_or(0.0);

        // For currency conversion: GBX/GBp values are now in GBP
        let convert_currency = if security_currency == "GBX" || security_currency == "GBp" {
            "GBP".to_string()
        } else {
            security_currency.clone()
        };

        // Calculate value in security currency
        let value_in_security = shares_val * price_val;

        // Convert to base currency
        let current_value = if convert_currency == base_currency {
            value_in_security
        } else {
            currency::convert(conn, value_in_security, &convert_currency, base_currency, today)
                .unwrap_or(value_in_security)
        };

        // Get cost basis (already converted to base currency by SSOT function)
        let cost_basis_converted = cost_map.get(&security_id).copied().unwrap_or(0.0);

        if current_value > 0.0 {
            let gain_loss = if cost_basis_converted > 0.0 {
                Some((current_value - cost_basis_converted) / cost_basis_converted * 100.0)
            } else {
                None
            };

            // Calculate average cost per share
            let avg_cost = if shares_val > 0.0 && cost_basis_converted > 0.0 {
                Some(cost_basis_converted / shares_val)
            } else {
                None
            };

            // Get first buy date and fees/taxes for this security
            let first_buy = first_buy_map.get(&security_id).cloned();
            let (sec_fees, sec_taxes) = fees_taxes_map.get(&security_id).cloned().unwrap_or((0.0, 0.0));

            holdings.push(HoldingSummary {
                name,
                isin,
                ticker,
                shares: shares_val,
                current_value,
                current_price: Some(price_val),
                cost_basis: cost_basis_converted,
                weight_percent: 0.0, // Calculate after total
                gain_loss_percent: gain_loss,
                currency: security_currency.clone(),
                avg_cost_per_share: avg_cost,
                first_buy_date: first_buy,
                total_fees: sec_fees,
                total_taxes: sec_taxes,
            });

            total_value += current_value;
            total_cost_basis += cost_basis_converted;

            *currency_values.entry(convert_currency).or_insert(0.0) += current_value;
        }
    }

    // Calculate weight percentages
    for h in &mut holdings {
        h.weight_percent = if total_value > 0.0 {
            h.current_value / total_value * 100.0
        } else {
            0.0
        };
    }

    // Currency allocation as percentages
    let currency_allocation: Vec<(String, f64)> = currency_values
        .into_iter()
        .map(|(c, v)| (c, if total_value > 0.0 { v / total_value * 100.0 } else { 0.0 }))
        .collect();

    // Top positions
    let top_positions: Vec<(String, f64)> = holdings
        .iter()
        .take(5)
        .map(|h| (h.name.clone(), h.weight_percent))
        .collect();

    // Calculate total gain/loss percent
    let total_gain_loss_percent = if total_cost_basis > 0.0 {
        (total_value - total_cost_basis) / total_cost_basis * 100.0
    } else {
        0.0
    };

    // Calculate annual dividends
    let dividends_sql = r#"
        SELECT COALESCE(SUM(amount), 0)
        FROM pp_txn
        WHERE txn_type = 'DIVIDENDS'
        AND date >= date('now', '-1 year')
    "#;
    let annual_dividends: f64 = conn
        .query_row(dividends_sql, [], |row| row.get::<_, i64>(0))
        .map(|v| v as f64 / 100.0)
        .unwrap_or(0.0);

    // Calculate dividend yield
    let dividend_yield = if total_value > 0.0 {
        Some(annual_dividends / total_value * 100.0)
    } else {
        None
    };

    // Load recent dividends with details (last 12 months)
    let recent_dividends = load_recent_dividends(conn)?;

    // Load recent transactions (last 30)
    let recent_transactions = load_recent_transactions(conn)?;

    // Load watchlist items
    let watchlist = load_watchlist(conn);

    // Get portfolio age (first transaction date)
    let first_txn_sql = "SELECT MIN(date) FROM pp_txn WHERE date IS NOT NULL";
    let first_txn_date: Option<String> = conn
        .query_row(first_txn_sql, [], |row| row.get(0))
        .ok();

    let first_date = first_txn_date
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

    let portfolio_age_days = first_date
        .map(|d| (Utc::now().date_naive() - d).num_days().max(0) as u32)
        .unwrap_or(0);

    // Load sold positions
    let sold_positions = load_sold_positions(conn);

    // Load yearly overview
    let yearly_overview = load_yearly_overview(conn);

    // Calculate TTWROR performance
    let (ttwror, ttwror_annualized) = if let Some(start_date) = first_date {
        let end_date = Utc::now().date_naive();
        match performance::calculate_ttwror(conn, None, start_date, end_date) {
            Ok(result) => (
                Some(result.total_return * 100.0),
                Some(result.annualized_return * 100.0),
            ),
            Err(e) => {
                log::warn!("Failed to calculate TTWROR: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    // Load provider status for AI context
    let provider_status = load_provider_status_for_ai(conn);

    // Load fees and taxes summary
    let fees_and_taxes = load_fees_and_taxes(conn);

    // Load investment summary
    let investment_summary = load_investment_summary(conn);

    // Load sector/taxonomy allocation
    let sector_allocation = load_sector_allocation(conn, total_value);

    // Load portfolio extremes (high/low)
    let portfolio_extremes = load_portfolio_extremes(conn);

    Ok(PortfolioInsightsContext {
        holdings,
        total_value,
        total_cost_basis,
        total_gain_loss_percent,
        ttwror,
        ttwror_annualized,
        irr: None, // IRR calculation is expensive, skip for chat context
        currency_allocation,
        top_positions,
        dividend_yield,
        annual_dividends,
        recent_dividends,
        recent_transactions,
        watchlist,
        sold_positions,
        yearly_overview,
        portfolio_age_days,
        analysis_date: Utc::now().format("%d.%m.%Y").to_string(),
        base_currency: base_currency.to_string(),
        user_name,
        provider_status,
        fees_and_taxes,
        investment_summary,
        sector_allocation,
        portfolio_extremes,
    })
}

/// Load recent dividends (last 12 months)
fn load_recent_dividends(conn: &Connection) -> Result<Vec<DividendPayment>, String> {
    let sql = r#"
        SELECT t.date, s.name, t.amount,
               COALESCE((SELECT SUM(u.amount) FROM pp_txn_unit u WHERE u.txn_id = t.id AND u.unit_type = 'TAX'), 0) as taxes,
               t.currency
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        WHERE t.txn_type = 'DIVIDENDS'
        AND t.date >= date('now', '-1 year')
        ORDER BY t.date DESC
        LIMIT 50
    "#;
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let dividends: Vec<DividendPayment> = stmt
        .query_map([], |row| {
            let gross = row.get::<_, i64>(2)? as f64 / 100.0;
            let taxes = row.get::<_, i64>(3)? as f64 / 100.0;
            Ok(DividendPayment {
                date: row.get(0)?,
                security_name: row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "Unbekannt".to_string()),
                gross_amount: gross,
                net_amount: gross - taxes,
                currency: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(dividends)
}

/// Load recent transactions (last 30)
fn load_recent_transactions(conn: &Connection) -> Result<Vec<RecentTransaction>, String> {
    let sql = r#"
        SELECT t.date, t.txn_type, s.name, t.shares, t.amount, t.currency
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        WHERE t.date IS NOT NULL
        ORDER BY t.date DESC
        LIMIT 30
    "#;
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let transactions: Vec<RecentTransaction> = stmt
        .query_map([], |row| {
            Ok(RecentTransaction {
                date: row.get(0)?,
                txn_type: row.get(1)?,
                security_name: row.get(2)?,
                shares: row.get::<_, Option<i64>>(3)?.map(|s| shares::to_decimal(s)),
                amount: row.get::<_, i64>(4)? as f64 / 100.0,
                currency: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(transactions)
}

/// Load watchlist items
fn load_watchlist(conn: &Connection) -> Vec<WatchlistItem> {
    let sql = r#"
        SELECT s.name, s.isin, s.ticker, lp.value, s.currency
        FROM pp_watchlist_security ws
        JOIN pp_security s ON s.id = ws.security_id
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        ORDER BY s.name
    "#;
    conn.prepare(sql)
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok(WatchlistItem {
                    name: row.get(0)?,
                    isin: row.get(1)?,
                    ticker: row.get(2)?,
                    current_price: row.get::<_, Option<i64>>(3)?.map(prices::to_decimal),
                    currency: row.get(4)?,
                })
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_else(|_| Vec::new())
}

/// Load sold positions (securities that were held but now have 0 shares)
fn load_sold_positions(conn: &Connection) -> Vec<SoldPosition> {
    let sql = r#"
        WITH position_summary AS (
            SELECT
                s.id as security_id,
                s.name,
                s.ticker,
                s.isin,
                SUM(CASE WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares ELSE 0 END) as bought,
                SUM(CASE WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN t.shares ELSE 0 END) as sold,
                MAX(t.date) as last_txn_date
            FROM pp_security s
            JOIN pp_txn t ON t.security_id = s.id AND t.owner_type = 'portfolio'
            WHERE t.shares IS NOT NULL
            GROUP BY s.id
            HAVING (bought - sold) <= 0 AND sold > 0
        )
        SELECT security_id, name, ticker, isin, bought, sold, last_txn_date
        FROM position_summary
        ORDER BY last_txn_date DESC
    "#;

    let raw: Vec<(i64, String, Option<String>, Option<String>, i64, i64, String)> = conn
        .prepare(sql)
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        })
        .unwrap_or_else(|_| Vec::new());

    let mut result = Vec::new();
    for (security_id, name, ticker, isin, bought, sold, last_date) in raw {
        let gain_sql = r#"
            SELECT COALESCE(SUM(
                (t.amount * fc.shares_consumed / t.shares) - fc.gross_amount
            ), 0)
            FROM pp_fifo_consumption fc
            JOIN pp_fifo_lot l ON l.id = fc.lot_id
            JOIN pp_txn t ON t.id = fc.sale_txn_id
            WHERE l.security_id = ?1
        "#;
        let realized_gain: f64 = conn
            .query_row(gain_sql, [security_id], |row| row.get::<_, i64>(0))
            .map(|v| v as f64 / 100.0)
            .unwrap_or(0.0);

        result.push(SoldPosition {
            name,
            ticker,
            isin,
            total_bought_shares: shares::to_decimal(bought),
            total_sold_shares: shares::to_decimal(sold),
            realized_gain_loss: realized_gain,
            last_transaction_date: last_date,
        });
    }
    result
}

/// Load yearly overview (last 5 years)
fn load_yearly_overview(conn: &Connection) -> Vec<YearlyOverview> {
    let current_year = Utc::now().year();
    let mut yearly_overview = Vec::new();

    for year in (current_year - 4)..=current_year {
        // Realized gains for the year
        let gains_sql = r#"
            SELECT COALESCE(SUM(
                (t.amount * fc.shares_consumed / t.shares) - fc.gross_amount
            ), 0)
            FROM pp_fifo_consumption fc
            JOIN pp_fifo_lot l ON l.id = fc.lot_id
            JOIN pp_txn t ON t.id = fc.sale_txn_id
            WHERE strftime('%Y', t.date) = ?1
        "#;
        let realized_gains: f64 = conn
            .query_row(gains_sql, [year.to_string()], |row| row.get::<_, i64>(0))
            .map(|v| v as f64 / 100.0)
            .unwrap_or(0.0);

        // Dividends for the year
        let dividends_sql = r#"
            SELECT COALESCE(SUM(amount), 0)
            FROM pp_txn
            WHERE txn_type = 'DIVIDENDS' AND strftime('%Y', date) = ?1
        "#;
        let dividends: f64 = conn
            .query_row(dividends_sql, [year.to_string()], |row| row.get::<_, i64>(0))
            .map(|v| v as f64 / 100.0)
            .unwrap_or(0.0);

        // Transaction count for the year
        let txn_count_sql = r#"
            SELECT COUNT(*)
            FROM pp_txn
            WHERE strftime('%Y', date) = ?1
        "#;
        let transaction_count: i32 = conn
            .query_row(txn_count_sql, [year.to_string()], |row| row.get(0))
            .unwrap_or(0);

        // Only add years that have data
        if transaction_count > 0 || realized_gains != 0.0 || dividends != 0.0 {
            yearly_overview.push(YearlyOverview {
                year,
                realized_gains,
                dividends,
                transaction_count,
            });
        }
    }
    yearly_overview
}

/// Load provider status for AI context (simplified version without API keys)
pub fn load_provider_status_for_ai(conn: &Connection) -> Option<QuoteProviderStatusSummary> {
    let today = Utc::now().date_naive();
    let today_str = today.to_string();

    // Providers that require API keys
    let api_key_providers = ["FINNHUB", "ALPHAVANTAGE", "TWELVEDATA"];

    let sql = r#"
        SELECT
            s.id,
            s.name,
            COALESCE(s.latest_feed, s.feed, '') as provider,
            s.ticker,
            s.isin,
            lp.date as last_quote_date,
            julianday(?) - julianday(lp.date) as days_old
        FROM pp_security s
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        WHERE s.is_retired = 0
          AND (
              SELECT COALESCE(SUM(CASE
                  WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                  WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                  ELSE 0
              END), 0)
              FROM pp_txn t
              WHERE t.security_id = s.id AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
          ) > 0
    "#;

    let mut stmt = conn.prepare(sql).ok()?;
    let rows = stmt.query([&today_str]).ok()?;

    let securities: Vec<(i64, String, String, Option<String>, Option<String>, Option<String>, Option<f64>)> = rows
        .mapped(|row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<f64>>(6)?,
            ))
        })
        .filter_map(|r| r.ok())
        .collect();

    let mut can_sync_count = 0;
    let mut cannot_sync_count = 0;
    let mut missing_api_keys: Vec<String> = Vec::new();
    let mut issues: Vec<String> = Vec::new();

    let held_count = securities.len();
    let mut synced_today_count = 0;
    let mut outdated: Vec<String> = Vec::new();

    for (_id, name, provider, ticker, isin, last_quote_date, days_old_f) in &securities {
        let provider_upper = provider.to_uppercase();
        let has_symbol = ticker.is_some() || isin.is_some();

        let days_old = days_old_f.map(|d| d.round() as i64);
        let is_today = match last_quote_date {
            Some(date) => date == &today_str,
            None => false,
        };

        if is_today || days_old == Some(0) {
            synced_today_count += 1;
        } else {
            let days_str = match days_old {
                Some(d) if d == 1 => "1 Tag alt".to_string(),
                Some(d) => format!("{} Tage alt", d),
                None => "kein Kurs".to_string(),
            };
            let ticker_str = ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
            outdated.push(format!("{}{}: {}", name, ticker_str, days_str));
        }

        let needs_api_key = api_key_providers.iter().any(|p| provider_upper.contains(p));

        if provider_upper.is_empty() || provider_upper == "MANUAL" || provider_upper == "GENERIC_HTML_TABLE" {
            cannot_sync_count += 1;
            issues.push(format!("{}: Kein Provider konfiguriert", name));
        } else if !has_symbol {
            cannot_sync_count += 1;
            issues.push(format!("{}: Kein Ticker oder ISIN", name));
        } else if needs_api_key {
            cannot_sync_count += 1;
            let provider_name = if provider_upper.contains("FINNHUB") {
                "Finnhub"
            } else if provider_upper.contains("ALPHA") {
                "Alpha Vantage"
            } else if provider_upper.contains("TWELVE") {
                "TwelveData"
            } else {
                provider
            };
            issues.push(format!("{}: {} benötigt API-Key", name, provider_name));

            if !missing_api_keys.contains(&provider_name.to_string()) {
                missing_api_keys.push(provider_name.to_string());
            }
        } else {
            can_sync_count += 1;
        }
    }

    let outdated_count = outdated.len();

    let quote_sync = QuoteSyncInfo {
        held_count,
        synced_today_count,
        outdated_count,
        today: today_str,
        outdated,
    };

    Some(QuoteProviderStatusSummary {
        can_sync_count,
        cannot_sync_count,
        missing_api_keys,
        issues,
        quote_sync,
    })
}

/// Load fees and taxes summary
pub fn load_fees_and_taxes(conn: &Connection) -> FeesAndTaxesSummary {
    let current_year = Utc::now().year();

    let sql = r#"
        SELECT
            strftime('%Y', t.date) as year,
            COALESCE(SUM(CASE WHEN u.unit_type = 'FEE' THEN u.amount ELSE 0 END), 0) as fees,
            COALESCE(SUM(CASE WHEN u.unit_type = 'TAX' THEN u.amount ELSE 0 END), 0) as taxes
        FROM pp_txn t
        LEFT JOIN pp_txn_unit u ON u.txn_id = t.id
        WHERE u.unit_type IN ('FEE', 'TAX')
        GROUP BY strftime('%Y', t.date)
        ORDER BY year DESC
    "#;

    let mut total_fees = 0.0;
    let mut total_taxes = 0.0;
    let mut fees_this_year = 0.0;
    let mut taxes_this_year = 0.0;
    let mut by_year: Vec<YearlyFeesAndTaxes> = Vec::new();

    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                let (year_str, fees_cents, taxes_cents) = row;
                let year: i32 = year_str.parse().unwrap_or(0);
                let fees = fees_cents as f64 / 100.0;
                let taxes = taxes_cents as f64 / 100.0;

                total_fees += fees;
                total_taxes += taxes;

                if year == current_year {
                    fees_this_year = fees;
                    taxes_this_year = taxes;
                }

                by_year.push(YearlyFeesAndTaxes { year, fees, taxes });
            }
        }
    }

    FeesAndTaxesSummary {
        total_fees,
        total_taxes,
        fees_this_year,
        taxes_this_year,
        by_year,
    }
}

/// Load investment summary (total invested, withdrawn, deposits, etc.)
pub fn load_investment_summary(conn: &Connection) -> InvestmentSummary {
    let buy_sql = r#"
        SELECT COALESCE(SUM(amount), 0) FROM pp_txn
        WHERE txn_type IN ('BUY', 'DELIVERY_INBOUND') AND owner_type = 'portfolio'
    "#;
    let total_invested: f64 = conn.query_row(buy_sql, [], |row| row.get::<_, i64>(0))
        .map(|v| v as f64 / 100.0)
        .unwrap_or(0.0);

    let sell_sql = r#"
        SELECT COALESCE(SUM(amount), 0) FROM pp_txn
        WHERE txn_type IN ('SELL', 'DELIVERY_OUTBOUND') AND owner_type = 'portfolio'
    "#;
    let total_withdrawn: f64 = conn.query_row(sell_sql, [], |row| row.get::<_, i64>(0))
        .map(|v| v as f64 / 100.0)
        .unwrap_or(0.0);

    let deposit_sql = r#"
        SELECT COALESCE(SUM(amount), 0) FROM pp_txn
        WHERE txn_type = 'DEPOSIT' AND owner_type = 'account'
    "#;
    let total_deposits: f64 = conn.query_row(deposit_sql, [], |row| row.get::<_, i64>(0))
        .map(|v| v as f64 / 100.0)
        .unwrap_or(0.0);

    let removal_sql = r#"
        SELECT COALESCE(SUM(amount), 0) FROM pp_txn
        WHERE txn_type = 'REMOVAL' AND owner_type = 'account'
    "#;
    let total_removals: f64 = conn.query_row(removal_sql, [], |row| row.get::<_, i64>(0))
        .map(|v| v as f64 / 100.0)
        .unwrap_or(0.0);

    let first_date_sql = r#"
        SELECT MIN(date) FROM pp_txn
        WHERE txn_type IN ('BUY', 'DELIVERY_INBOUND', 'DEPOSIT')
    "#;
    let first_investment_date: Option<String> = conn.query_row(first_date_sql, [], |row| row.get(0))
        .ok();

    InvestmentSummary {
        total_invested,
        total_withdrawn,
        net_invested: total_invested - total_withdrawn,
        total_deposits,
        total_removals,
        first_investment_date,
    }
}

/// Load sector/taxonomy allocation
pub fn load_sector_allocation(conn: &Connection, total_value: f64) -> Vec<SectorAllocation> {
    if total_value <= 0.0 {
        return Vec::new();
    }

    let sql = r#"
        SELECT
            tax.name as taxonomy_name,
            cls.name as classification_name,
            COALESCE(SUM(
                CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END
            ), 0) as net_shares,
            lp.value as price,
            ca.weight as weight
        FROM pp_taxonomy tax
        JOIN pp_classification cls ON cls.taxonomy_id = tax.id
        JOIN pp_classification_assignment ca ON ca.classification_id = cls.id
        JOIN pp_security s ON s.id = ca.security_id
        LEFT JOIN pp_txn t ON t.security_id = s.id AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        WHERE s.is_retired = 0
        GROUP BY tax.id, cls.id, s.id
        HAVING net_shares > 0
    "#;

    let mut taxonomy_map: std::collections::HashMap<String, Vec<(String, f64)>> = std::collections::HashMap::new();

    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, i32>(4)?,
            ))
        }) {
            for row in rows.flatten() {
                let (tax_name, cls_name, shares_scaled, price_scaled, weight) = row;
                let shares_val = shares::to_decimal(shares_scaled);
                let price_val = price_scaled.map(|p| prices::to_decimal(p)).unwrap_or(0.0);
                let position_value = shares_val * price_val;
                let weighted_value = position_value * (weight as f64 / 10000.0);

                let entry = taxonomy_map.entry(tax_name).or_default();
                if let Some(existing) = entry.iter_mut().find(|(name, _)| name == &cls_name) {
                    existing.1 += weighted_value;
                } else {
                    entry.push((cls_name, weighted_value));
                }
            }
        }
    }

    taxonomy_map
        .into_iter()
        .map(|(tax_name, mut allocations)| {
            for (_, value) in &mut allocations {
                *value = (*value / total_value) * 100.0;
            }
            allocations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            SectorAllocation {
                taxonomy_name: tax_name,
                allocations,
            }
        })
        .collect()
}

/// Load portfolio historical extremes (all-time high/low, year high/low)
pub fn load_portfolio_extremes(conn: &Connection) -> Option<PortfolioExtremes> {
    let current_year = Utc::now().year();
    let year_start = format!("{}-01-01", current_year);

    // All-time high
    let ath_sql = r#"
        SELECT date, value FROM pp_portfolio_history
        WHERE value = (SELECT MAX(value) FROM pp_portfolio_history)
        LIMIT 1
    "#;
    let (all_time_high, all_time_high_date) = conn.query_row(ath_sql, [], |row| {
        Ok((row.get::<_, i64>(1)?, row.get::<_, String>(0)?))
    }).map(|(v, d)| (v as f64 / 100.0, d)).ok()?;

    // All-time low (excluding zeros)
    let atl_sql = r#"
        SELECT date, value FROM pp_portfolio_history
        WHERE value > 0 AND value = (SELECT MIN(value) FROM pp_portfolio_history WHERE value > 0)
        LIMIT 1
    "#;
    let (all_time_low, all_time_low_date) = conn.query_row(atl_sql, [], |row| {
        Ok((row.get::<_, i64>(1)?, row.get::<_, String>(0)?))
    }).map(|(v, d)| (v as f64 / 100.0, d)).unwrap_or((0.0, String::new()));

    // Year high
    let yh_sql = r#"
        SELECT date, value FROM pp_portfolio_history
        WHERE date >= ? AND value = (SELECT MAX(value) FROM pp_portfolio_history WHERE date >= ?)
        LIMIT 1
    "#;
    let (year_high, year_high_date) = conn.query_row(yh_sql, [&year_start, &year_start], |row| {
        Ok((row.get::<_, i64>(1)?, row.get::<_, String>(0)?))
    }).map(|(v, d)| (v as f64 / 100.0, d)).unwrap_or((all_time_high, all_time_high_date.clone()));

    // Year low (excluding zeros)
    let yl_sql = r#"
        SELECT date, value FROM pp_portfolio_history
        WHERE date >= ? AND value > 0 AND value = (SELECT MIN(value) FROM pp_portfolio_history WHERE date >= ? AND value > 0)
        LIMIT 1
    "#;
    let (year_low, year_low_date) = conn.query_row(yl_sql, [&year_start, &year_start], |row| {
        Ok((row.get::<_, i64>(1)?, row.get::<_, String>(0)?))
    }).map(|(v, d)| (v as f64 / 100.0, d)).unwrap_or((all_time_low, all_time_low_date.clone()));

    Some(PortfolioExtremes {
        all_time_high,
        all_time_high_date,
        all_time_low,
        all_time_low_date,
        year_high,
        year_high_date,
        year_low,
        year_low_date,
    })
}
