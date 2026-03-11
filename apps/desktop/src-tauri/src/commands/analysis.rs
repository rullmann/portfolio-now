//! Advanced Analysis Commands for Tauri
//!
//! Provides commands for CVaR, Drawdown, Rolling Returns, Monte Carlo, GARCH.
//! These operate on price data fetched from the database.

use crate::analysis;
use crate::db;
use serde::{Deserialize, Serialize};
use tauri::command;

/// Price point input from frontend
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricePoint {
    pub date: String,
    pub close: f64,
}

fn to_tuples(prices: &[PricePoint]) -> Vec<(String, f64)> {
    prices.iter().map(|p| (p.date.clone(), p.close)).collect()
}

// ============================================================================
// VaR / CVaR
// ============================================================================

#[command]
pub fn calculate_var_cvar(prices: Vec<PricePoint>) -> Result<analysis::VaRResult, String> {
    let data = to_tuples(&prices);
    Ok(analysis::calculate_var_cvar(&data))
}

/// Calculate VaR/CVaR for a portfolio using portfolio value history
#[command]
pub fn calculate_portfolio_var_cvar(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<analysis::VaRResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;

    let end = end_date
        .and_then(|s| crate::pp::parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let start = start_date
        .and_then(|s| crate::pp::parse_date_flexible(&s))
        .unwrap_or_else(|| end - chrono::Duration::days(365));

    let values = get_portfolio_values(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    Ok(analysis::calculate_var_cvar(&values))
}

// ============================================================================
// Drawdown Analysis
// ============================================================================

#[command]
pub fn calculate_drawdowns(prices: Vec<PricePoint>) -> Result<analysis::DrawdownResult, String> {
    let data = to_tuples(&prices);
    Ok(analysis::calculate_drawdowns(&data))
}

#[command]
pub fn calculate_portfolio_drawdowns(
    portfolio_id: Option<i64>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<analysis::DrawdownResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("Database not initialized")?;

    let end = end_date
        .and_then(|s| crate::pp::parse_date_flexible(&s))
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let start = start_date
        .and_then(|s| crate::pp::parse_date_flexible(&s))
        .unwrap_or_else(|| end - chrono::Duration::days(365 * 3));

    let values = get_portfolio_values(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    Ok(analysis::calculate_drawdowns(&values))
}

// ============================================================================
// Rolling Returns
// ============================================================================

/// Calculate rolling returns for multiple window sizes at once
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllRollingReturns {
    pub windows: Vec<analysis::RollingReturnsResult>,
}

#[command]
pub fn calculate_rolling_returns(
    prices: Vec<PricePoint>,
    windows: Option<Vec<RollingWindow>>,
) -> Result<AllRollingReturns, String> {
    let data = to_tuples(&prices);

    let window_configs = windows.unwrap_or_else(|| vec![
        RollingWindow { days: 21, label: "1M".into() },
        RollingWindow { days: 63, label: "3M".into() },
        RollingWindow { days: 126, label: "6M".into() },
        RollingWindow { days: 252, label: "1J".into() },
    ]);

    let results: Vec<analysis::RollingReturnsResult> = window_configs.iter()
        .map(|w| analysis::calculate_rolling_returns(&data, w.days, &w.label))
        .collect();

    Ok(AllRollingReturns { windows: results })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollingWindow {
    pub days: usize,
    pub label: String,
}

// ============================================================================
// Monte Carlo Simulation
// ============================================================================

#[command]
pub fn run_monte_carlo(
    prices: Vec<PricePoint>,
    num_simulations: Option<usize>,
    days_forward: Option<usize>,
) -> Result<analysis::MonteCarloResult, String> {
    let data = to_tuples(&prices);
    let sims = num_simulations.unwrap_or(5000);
    let days = days_forward.unwrap_or(252); // Default: 1 year

    Ok(analysis::run_monte_carlo(&data, sims, days))
}

// ============================================================================
// GARCH Volatility
// ============================================================================

#[command]
pub fn calculate_garch(
    prices: Vec<PricePoint>,
    forecast_days: Option<usize>,
) -> Result<analysis::GarchResult, String> {
    let data = to_tuples(&prices);
    let days = forecast_days.unwrap_or(30);

    Ok(analysis::calculate_garch(&data, days))
}

// ============================================================================
// Combined Analysis (all at once for AI context)
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullAnalysisResult {
    pub var_cvar: analysis::VaRResult,
    pub drawdowns: analysis::DrawdownResult,
    pub rolling_returns: AllRollingReturns,
    pub monte_carlo: analysis::MonteCarloResult,
    pub garch: analysis::GarchResult,
    /// Combined AI summary for all analyses
    pub ai_summary: String,
}

/// Run all analyses at once — useful for AI context generation
#[command]
pub fn full_risk_analysis(
    prices: Vec<PricePoint>,
    monte_carlo_simulations: Option<usize>,
    monte_carlo_days: Option<usize>,
    garch_forecast_days: Option<usize>,
) -> Result<FullAnalysisResult, String> {
    let data = to_tuples(&prices);

    let var_cvar = analysis::calculate_var_cvar(&data);
    let drawdowns = analysis::calculate_drawdowns(&data);

    let rolling_windows = vec![
        ("1M", 21), ("3M", 63), ("6M", 126), ("1J", 252),
    ];
    let rolling_results: Vec<analysis::RollingReturnsResult> = rolling_windows.iter()
        .map(|(label, days)| analysis::calculate_rolling_returns(&data, *days, label))
        .collect();
    let rolling_returns = AllRollingReturns { windows: rolling_results };

    let monte_carlo = analysis::run_monte_carlo(
        &data,
        monte_carlo_simulations.unwrap_or(5000),
        monte_carlo_days.unwrap_or(252),
    );

    let garch = analysis::calculate_garch(&data, garch_forecast_days.unwrap_or(30));

    let ai_summary = format!(
        "=== Vollständige Risikoanalyse ===\n\n\
         **VaR/CVaR:** {}\n\n\
         **Drawdowns:** {}\n\n\
         **Rolling Returns (1J):** {}\n\n\
         **Monte Carlo:** {}\n\n\
         **GARCH Volatilität:** {}",
        var_cvar.ai_summary,
        drawdowns.ai_summary,
        rolling_returns.windows.iter()
            .find(|w| w.window_label == "1J")
            .map(|w| w.ai_summary.clone())
            .unwrap_or_default(),
        monte_carlo.ai_summary,
        garch.ai_summary,
    );

    Ok(FullAnalysisResult {
        var_cvar,
        drawdowns,
        rolling_returns,
        monte_carlo,
        garch,
        ai_summary,
    })
}

// ============================================================================
// Chart Events (Dividends + Earnings markers on chart)
// ============================================================================

/// A chart event for markers on the price chart
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartEvent {
    pub date: String,
    /// "dividend", "ex_dividend", "earnings_beat", "earnings_miss", "earnings_inline"
    pub event_type: String,
    /// Display label (e.g. "0.82 EUR" or "EPS: 1.23 (+5.2%)")
    pub label: String,
    /// Numeric value (dividend amount or EPS)
    pub value: Option<f64>,
}

/// Get all chart events (dividends + earnings) for a security.
/// - Dividends: Yahoo Finance v8 chart API (full corporate history, no key needed)
/// - Earnings: Finnhub /stock/earnings (primary, needs API key) + Yahoo fallback
#[command]
pub async fn get_chart_events(
    security_id: i64,
    finnhub_api_key: Option<String>,
) -> Result<Vec<ChartEvent>, String> {
    // Get ticker from DB
    let ticker = {
        let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
        let conn = conn_guard.as_ref().ok_or("Database not initialized")?;

        let (ticker, _name): (Option<String>, String) = conn.query_row(
            "SELECT ticker, name FROM pp_security WHERE id = ?1",
            rusqlite::params![security_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(|e| format!("Security not found: {}", e))?;

        match ticker {
            Some(t) if !t.is_empty() => t,
            _ => return Err("Kein Ticker hinterlegt".into()),
        }
    };

    let mut events = Vec::new();

    // 1. Fetch dividends from Yahoo (full corporate dividend history, no API key needed)
    match crate::quotes::yahoo::fetch_dividend_events(&ticker).await {
        Ok(divs) => {
            for div in divs {
                events.push(ChartEvent {
                    date: div.date,
                    event_type: "dividend".into(),
                    label: format!("{:.4} {}", div.amount, div.currency),
                    value: Some(div.amount),
                });
            }
            log::info!("Loaded {} Yahoo dividend events for {}", events.len(), ticker);
        }
        Err(e) => {
            log::warn!("Failed to fetch Yahoo dividend events for {}: {}", ticker, e);
        }
    }

    // 2. Earnings: Fetch from APIs, cache in SQLite, return full accumulated history
    //    Finnhub gives ~4 quarters, Yahoo ~4 quarters. Each fetch merges into cache.
    //    Over time the cache accumulates a complete earnings history.

    // 2a. Fetch from Finnhub (primary source with surprise %)
    if let Some(ref api_key) = finnhub_api_key {
        if !api_key.is_empty() {
            match crate::quotes::finnhub::fetch_earnings(&ticker, api_key).await {
                Ok(earnings) => {
                    log::info!("Finnhub: {} earnings for {}", earnings.len(), ticker);
                    cache_earnings_from_finnhub(&ticker, &earnings);
                }
                Err(e) => {
                    log::warn!("Finnhub earnings failed for {}: {}", ticker, e);
                }
            }

            // Check for upcoming earnings
            if let Ok(Some(next_date)) = crate::quotes::finnhub::fetch_next_earnings_date(&ticker, api_key).await {
                events.push(ChartEvent {
                    date: next_date,
                    event_type: "earnings_upcoming".into(),
                    label: "Earnings".into(),
                    value: None,
                });
            }
        }
    }

    // 2b. Also fetch from Yahoo quoteSummary (may add different quarters)
    match crate::quotes::yahoo::fetch_earnings(&ticker).await {
        Ok(yahoo_earnings) => {
            log::info!("Yahoo: {} quarterly earnings for {}", yahoo_earnings.quarterly_earnings.len(), ticker);
            cache_earnings_from_yahoo(&ticker, &yahoo_earnings.quarterly_earnings);
            if let Some(ref next) = yahoo_earnings.next_earnings_date {
                if !events.iter().any(|e| e.event_type == "earnings_upcoming") {
                    events.push(ChartEvent {
                        date: next.clone(),
                        event_type: "earnings_upcoming".into(),
                        label: "Earnings".into(),
                        value: None,
                    });
                }
            }
        }
        Err(e) => {
            log::debug!("Yahoo earnings failed for {}: {}", ticker, e);
        }
    }

    // 2c. Read ALL cached earnings from DB (full accumulated history)
    let cached = load_cached_earnings(&ticker);
    for (date, eps_actual, eps_estimate, surprise_percent) in &cached {
        let (event_type, label) = classify_earnings(*eps_actual, *eps_estimate, *surprise_percent);
        events.push(ChartEvent {
            date: date.clone(),
            event_type: event_type.into(),
            label,
            value: *eps_actual,
        });
    }
    log::info!("Earnings cache: {} total entries for {}", cached.len(), ticker);

    // Sort by date
    events.sort_by(|a, b| a.date.cmp(&b.date));

    log::info!("Chart events for {} (id={}): {} total ({} div, {} earnings)",
        ticker, security_id, events.len(),
        events.iter().filter(|e| e.event_type == "dividend").count(),
        events.iter().filter(|e| e.event_type.starts_with("earnings")).count(),
    );

    Ok(events)
}

// ============================================================================
// Earnings Cache Helpers
// ============================================================================

/// Save Finnhub earnings into the cache (INSERT OR REPLACE to update existing)
fn cache_earnings_from_finnhub(ticker: &str, earnings: &[crate::quotes::finnhub::FinnhubEarningsEvent]) {
    let Ok(conn_guard) = db::get_connection() else { return };
    let Some(conn) = conn_guard.as_ref() else { return };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    for e in earnings {
        if e.eps_actual.is_none() { continue; }
        let _ = conn.execute(
            "INSERT OR REPLACE INTO pp_earnings_cache (ticker, date, eps_actual, eps_estimate, surprise_percent, source, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 'finnhub', ?6)",
            rusqlite::params![ticker, e.date, e.eps_actual, e.eps_estimate, e.surprise_percent, now],
        );
    }
}

/// Save Yahoo earnings into the cache (only if not already present from Finnhub)
fn cache_earnings_from_yahoo(ticker: &str, quarters: &[crate::quotes::yahoo::EarningsQuarter]) {
    let Ok(conn_guard) = db::get_connection() else { return };
    let Some(conn) = conn_guard.as_ref() else { return };
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    for q in quarters {
        let date = match &q.date {
            Some(d) if !d.is_empty() => d.clone(),
            _ => continue,
        };
        if q.eps_actual.is_none() { continue; }
        let surprise_pct = q.eps_surprise_pct.map(|s| s * 100.0);
        // INSERT OR IGNORE: don't overwrite Finnhub data (which has better surprise %)
        let _ = conn.execute(
            "INSERT OR IGNORE INTO pp_earnings_cache (ticker, date, eps_actual, eps_estimate, surprise_percent, source, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 'yahoo', ?6)",
            rusqlite::params![ticker, date, q.eps_actual, q.eps_estimate, surprise_pct, now],
        );
    }
}

/// Load all cached earnings for a ticker, sorted by date
fn load_cached_earnings(ticker: &str) -> Vec<(String, Option<f64>, Option<f64>, Option<f64>)> {
    let Ok(conn_guard) = db::get_connection() else { return Vec::new() };
    let Some(conn) = conn_guard.as_ref() else { return Vec::new() };

    let mut stmt = match conn.prepare(
        "SELECT date, eps_actual, eps_estimate, surprise_percent FROM pp_earnings_cache WHERE ticker = ?1 ORDER BY date"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    stmt.query_map(rusqlite::params![ticker], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<f64>>(2)?,
            row.get::<_, Option<f64>>(3)?,
        ))
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// Classify earnings as beat/miss/inline based on EPS data
fn classify_earnings(
    eps_actual: Option<f64>,
    eps_estimate: Option<f64>,
    surprise_percent: Option<f64>,
) -> (&'static str, String) {
    match (eps_actual, eps_estimate) {
        (Some(actual), Some(_est)) => {
            let surprise = surprise_percent.unwrap_or_else(|| {
                if _est.abs() > 0.001 { ((actual - _est) / _est.abs()) * 100.0 } else { 0.0 }
            });
            if surprise > 1.0 {
                ("earnings_beat", format!("EPS {:.2} (+{:.1}%)", actual, surprise))
            } else if surprise < -1.0 {
                ("earnings_miss", format!("EPS {:.2} ({:.1}%)", actual, surprise))
            } else {
                ("earnings_inline", format!("EPS {:.2} ({:.1}%)", actual, surprise))
            }
        }
        (Some(actual), None) => ("earnings_inline", format!("EPS {:.2}", actual)),
        _ => ("earnings_inline", "Earnings".into()),
    }
}

// ============================================================================
// Helper: Get portfolio value history from DB
// ============================================================================

fn get_portfolio_values(
    conn: &rusqlite::Connection,
    portfolio_id: Option<i64>,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Result<Vec<(String, f64)>, anyhow::Error> {
    let values = crate::performance::get_portfolio_value_history_public(conn, portfolio_id, start_date, end_date)?;

    Ok(values.into_iter()
        .map(|(date, value)| (date.to_string(), value))
        .collect())
}
