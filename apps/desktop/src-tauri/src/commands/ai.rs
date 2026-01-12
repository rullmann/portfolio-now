//! AI chart analysis commands

use crate::ai::{
    claude, gemini, openai, perplexity,
    list_claude_models, list_openai_models, list_gemini_models, list_perplexity_models,
    get_model_upgrade, get_models_for_provider, ModelInfo,
    AiModelInfo, AiError, ChartAnalysisRequest, ChartAnalysisResponse, AnnotationAnalysisResponse,
    HoldingSummary, PortfolioInsightsContext, PortfolioInsightsResponse,
    ChatMessage, PortfolioChatResponse,
    RecentTransaction, DividendPayment, WatchlistItem,
    SoldPosition, YearlyOverview,
};
use crate::commands::ai_helpers;
use crate::currency;
use crate::db;
use crate::performance;
use crate::pp::common::{prices, shares};
use chrono::{Datelike, NaiveDate, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};

/// Analyze a chart using AI
///
/// Returns ChartAnalysisResponse on success, or a JSON-serialized AiError on failure.
/// Automatically upgrades deprecated models to their replacements.
#[command]
pub async fn analyze_chart_with_ai(
    request: ChartAnalysisRequest,
) -> Result<ChartAnalysisResponse, String> {
    // Check if the model is deprecated and auto-upgrade
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    let result = match request.provider.as_str() {
        "claude" => claude::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        "openai" => openai::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        "gemini" => gemini::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        "perplexity" => perplexity::analyze(&request.image_base64, &model, &request.api_key, &request.context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    // Convert AiError to JSON string for frontend parsing
    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}

/// Fetch available models for a given AI provider
#[command]
pub async fn get_ai_models(
    provider: String,
    api_key: String,
) -> Result<Vec<AiModelInfo>, String> {
    match provider.as_str() {
        "claude" => list_claude_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        "openai" => list_openai_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        "gemini" => list_gemini_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        "perplexity" => list_perplexity_models(&api_key)
            .await
            .map_err(|e| e.to_string()),
        _ => Err(format!("Unknown AI provider: {}", provider)),
    }
}

/// Analyze a chart using AI and return structured annotations
///
/// Returns AnnotationAnalysisResponse on success with support/resistance levels,
/// patterns, and signals as structured JSON instead of markdown text.
#[command]
pub async fn analyze_chart_with_annotations(
    request: ChartAnalysisRequest,
) -> Result<AnnotationAnalysisResponse, String> {
    // Check if the model is deprecated and auto-upgrade
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    let result = match request.provider.as_str() {
        "claude" => claude::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        "openai" => openai::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        "gemini" => gemini::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        "perplexity" => perplexity::analyze_with_annotations(&request.image_base64, &model, &request.api_key, &request.context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    // Convert AiError to JSON string for frontend parsing
    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}

/// Get vision-capable models for a provider from the centralized registry.
///
/// Returns the list of vision models from the static registry.
/// No API key required - this uses the internal model definitions.
#[command]
pub fn get_vision_models(provider: String) -> Vec<ModelInfo> {
    get_models_for_provider(&provider)
        .into_iter()
        .map(ModelInfo::from)
        .collect()
}

// ============================================================================
// Portfolio Insights Commands
// ============================================================================

/// Request for portfolio insights analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioInsightsRequest {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_currency: String,
}

/// Load portfolio context from database for AI analysis
fn load_portfolio_context(base_currency: &str) -> Result<PortfolioInsightsContext, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let today = Utc::now().date_naive();

    // Load ALL holdings with current values (matching get_all_holdings logic)
    // Use subquery to calculate net_shares first, then filter
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

    // Get cost basis for all securities
    let cost_basis_sql = r#"
        SELECT security_id, currency,
               SUM(CASE WHEN original_shares > 0
                   THEN (remaining_shares * gross_amount / original_shares)
                   ELSE 0 END) as cost_basis
        FROM pp_fifo_lot
        WHERE remaining_shares > 0
        GROUP BY security_id
    "#;
    let mut cost_map: std::collections::HashMap<i64, (f64, String)> = std::collections::HashMap::new();
    if let Ok(mut cost_stmt) = conn.prepare(cost_basis_sql) {
        let cost_rows = cost_stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        });
        if let Ok(rows) = cost_rows {
            for row in rows.flatten() {
                let (sec_id, lot_currency, cost_cents) = row;
                let cost = cost_cents as f64 / 100.0;
                cost_map.insert(sec_id, (cost, lot_currency));
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

        // Get and convert cost basis
        let (cost_basis_val, cost_currency) = cost_map
            .get(&security_id)
            .cloned()
            .unwrap_or((0.0, base_currency.to_string()));

        let cost_basis_converted = if cost_currency == base_currency {
            cost_basis_val
        } else {
            currency::convert(conn, cost_basis_val, &cost_currency, base_currency, today)
                .unwrap_or(cost_basis_val)
        };

        if current_value > 0.0 {
            let gain_loss = if cost_basis_converted > 0.0 {
                Some((current_value - cost_basis_converted) / cost_basis_converted * 100.0)
            } else {
                None
            };

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
    let recent_dividends_sql = r#"
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
    let mut div_stmt = conn.prepare(recent_dividends_sql).map_err(|e| e.to_string())?;
    let recent_dividends: Vec<DividendPayment> = div_stmt
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

    // Load recent transactions (last 30)
    let recent_txn_sql = r#"
        SELECT t.date, t.txn_type, s.name, t.shares, t.amount, t.currency
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        WHERE t.date IS NOT NULL
        ORDER BY t.date DESC
        LIMIT 30
    "#;
    let mut txn_stmt = conn.prepare(recent_txn_sql).map_err(|e| e.to_string())?;
    let recent_transactions: Vec<RecentTransaction> = txn_stmt
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

    // Load watchlist items
    let watchlist_sql = r#"
        SELECT s.name, s.isin, s.ticker, lp.value, s.currency
        FROM pp_watchlist_security ws
        JOIN pp_security s ON s.id = ws.security_id
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        ORDER BY s.name
    "#;
    let watchlist: Vec<WatchlistItem> = conn
        .prepare(watchlist_sql)
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
        .unwrap_or_else(|_| Vec::new());

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

    // Load sold positions (securities that were held but now have 0 shares)
    let sold_positions_sql = r#"
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

    let sold_positions_raw: Vec<(i64, String, Option<String>, Option<String>, i64, i64, String)> = conn
        .prepare(sold_positions_sql)
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,           // security_id
                    row.get::<_, String>(1)?,        // name
                    row.get::<_, Option<String>>(2)?, // ticker
                    row.get::<_, Option<String>>(3)?, // isin
                    row.get::<_, i64>(4)?,           // bought
                    row.get::<_, i64>(5)?,           // sold
                    row.get::<_, String>(6)?,        // last_txn_date
                ))
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
        })
        .unwrap_or_else(|_| Vec::new());

    // Calculate realized gains for each sold position
    let mut sold_positions_with_gains: Vec<SoldPosition> = Vec::new();
    for (security_id, name, ticker, isin, bought, sold, last_date) in sold_positions_raw {
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

        sold_positions_with_gains.push(SoldPosition {
            name,
            ticker,
            isin,
            total_bought_shares: shares::to_decimal(bought),
            total_sold_shares: shares::to_decimal(sold),
            realized_gain_loss: realized_gain,
            last_transaction_date: last_date,
        });
    }

    // Load yearly overview (last 5 years)
    let current_year = Utc::now().year();
    let mut yearly_overview: Vec<YearlyOverview> = Vec::new();

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
        sold_positions: sold_positions_with_gains,
        yearly_overview,
        portfolio_age_days,
        analysis_date: Utc::now().format("%d.%m.%Y").to_string(),
        base_currency: base_currency.to_string(),
    })
}

/// Analyze portfolio with AI to get insights
///
/// Returns a markdown-formatted analysis with strengths, weaknesses, and recommendations.
#[command]
pub async fn analyze_portfolio_with_ai(
    request: PortfolioInsightsRequest,
) -> Result<PortfolioInsightsResponse, String> {
    // Load portfolio context from database
    let context = load_portfolio_context(&request.base_currency)?;

    // Check if portfolio has holdings
    if context.holdings.is_empty() {
        return Err("Keine Holdings im Portfolio gefunden. Bitte importiere zuerst Transaktionen.".to_string());
    }

    // Auto-upgrade deprecated models
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    // Call the appropriate provider
    let result = match request.provider.as_str() {
        "claude" => claude::analyze_portfolio(&model, &request.api_key, &context).await,
        "openai" => openai::analyze_portfolio(&model, &request.api_key, &context).await,
        "gemini" => gemini::analyze_portfolio(&model, &request.api_key, &context).await,
        "perplexity" => perplexity::analyze_portfolio(&model, &request.api_key, &context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}

// ============================================================================
// Portfolio Chat Commands
// ============================================================================

/// Request for portfolio chat
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioChatRequest {
    pub messages: Vec<ChatMessage>,
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_currency: String,
    pub alpha_vantage_api_key: Option<String>,
}

/// Watchlist command parsed from AI response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchlistCommand {
    pub action: String, // "add" or "remove"
    pub watchlist: String,
    pub security: String,
}

/// Parse watchlist commands from AI response
fn parse_watchlist_commands(response: &str) -> (Vec<WatchlistCommand>, String) {
    let mut commands = Vec::new();
    let mut cleaned_response = response.to_string();

    // Parse WATCHLIST_ADD commands: [[WATCHLIST_ADD:{"watchlist":"...","security":"..."}]]
    let add_re = Regex::new(r#"\[\[WATCHLIST_ADD:\s*\{[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*"security"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in add_re.captures_iter(response) {
        commands.push(WatchlistCommand {
            action: "add".to_string(),
            watchlist: cap[1].to_string(),
            security: cap[2].to_string(),
        });
    }
    // Also try reversed order
    let add_re2 = Regex::new(r#"\[\[WATCHLIST_ADD:\s*\{[^}]*"security"\s*:\s*"([^"]+)"[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in add_re2.captures_iter(response) {
        // Check if not already added
        let cmd = WatchlistCommand {
            action: "add".to_string(),
            watchlist: cap[2].to_string(),
            security: cap[1].to_string(),
        };
        if !commands.iter().any(|c| c.watchlist == cmd.watchlist && c.security == cmd.security) {
            commands.push(cmd);
        }
    }

    // Parse WATCHLIST_REMOVE commands
    let remove_re = Regex::new(r#"\[\[WATCHLIST_REMOVE:\s*\{[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*"security"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in remove_re.captures_iter(response) {
        commands.push(WatchlistCommand {
            action: "remove".to_string(),
            watchlist: cap[1].to_string(),
            security: cap[2].to_string(),
        });
    }
    let remove_re2 = Regex::new(r#"\[\[WATCHLIST_REMOVE:\s*\{[^}]*"security"\s*:\s*"([^"]+)"[^}]*"watchlist"\s*:\s*"([^"]+)"[^}]*\}\]\]"#).unwrap();
    for cap in remove_re2.captures_iter(response) {
        let cmd = WatchlistCommand {
            action: "remove".to_string(),
            watchlist: cap[2].to_string(),
            security: cap[1].to_string(),
        };
        if !commands.iter().any(|c| c.watchlist == cmd.watchlist && c.security == cmd.security && c.action == cmd.action) {
            commands.push(cmd);
        }
    }

    // Remove all command tags from response
    let clean_re = Regex::new(r#"\[\[WATCHLIST_(ADD|REMOVE):[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();

    // Clean up extra whitespace/newlines at the start
    cleaned_response = cleaned_response.trim_start().to_string();

    (commands, cleaned_response)
}

/// Execute watchlist commands
async fn execute_watchlist_commands(
    commands: &[WatchlistCommand],
    alpha_vantage_api_key: Option<String>,
) -> Vec<String> {
    let mut results = Vec::new();

    for cmd in commands {
        let result = match cmd.action.as_str() {
            "add" => {
                match ai_helpers::ai_add_to_watchlist(
                    cmd.watchlist.clone(),
                    cmd.security.clone(),
                    alpha_vantage_api_key.clone(),
                ).await {
                    Ok(r) => r.message,
                    Err(e) => format!("Fehler: {}", e),
                }
            }
            "remove" => {
                match ai_helpers::ai_remove_from_watchlist(
                    cmd.watchlist.clone(),
                    cmd.security.clone(),
                ) {
                    Ok(r) => r.message,
                    Err(e) => format!("Fehler: {}", e),
                }
            }
            _ => continue,
        };
        results.push(result);
    }

    results
}

/// Transaction query parsed from AI response
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionQuery {
    pub security: Option<String>,
    pub year: Option<i32>,
    pub txn_type: Option<String>,
    pub limit: Option<i32>,
}

/// Parse transaction query commands from AI response
fn parse_transaction_queries(response: &str) -> (Vec<TransactionQuery>, String) {
    let mut queries = Vec::new();
    let mut cleaned_response = response.to_string();

    // Parse QUERY_TRANSACTIONS commands: [[QUERY_TRANSACTIONS:{"security":"...","year":2024}]]
    let query_re = Regex::new(r#"\[\[QUERY_TRANSACTIONS:\s*\{([^}]*)\}\]\]"#).unwrap();

    for cap in query_re.captures_iter(response) {
        let json_content = &cap[1];

        // Parse individual fields
        let security = Regex::new(r#""security"\s*:\s*"([^"]+)""#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .map(|c| c[1].to_string());

        let year = Regex::new(r#""year"\s*:\s*(\d{4})"#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .and_then(|c| c[1].parse::<i32>().ok());

        let txn_type = Regex::new(r#""type"\s*:\s*"([^"]+)""#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .map(|c| c[1].to_string());

        let limit = Regex::new(r#""limit"\s*:\s*(\d+)"#)
            .ok()
            .and_then(|re| re.captures(json_content))
            .and_then(|c| c[1].parse::<i32>().ok());

        queries.push(TransactionQuery {
            security,
            year,
            txn_type,
            limit,
        });
    }

    // Remove all query tags from response
    let clean_re = Regex::new(r#"\[\[QUERY_TRANSACTIONS:[^\]]*\]\]"#).unwrap();
    cleaned_response = clean_re.replace_all(&cleaned_response, "").to_string();
    cleaned_response = cleaned_response.trim_start().to_string();

    (queries, cleaned_response)
}

/// Execute transaction queries and return formatted results
fn execute_transaction_queries(queries: &[TransactionQuery]) -> Vec<String> {
    let mut results = Vec::new();

    for query in queries {
        match ai_helpers::ai_query_transactions(
            query.security.clone(),
            query.year,
            query.txn_type.clone(),
            query.limit,
        ) {
            Ok(result) => {
                if result.transactions.is_empty() {
                    results.push("Keine Transaktionen gefunden.".to_string());
                } else {
                    let mut output = format!("**{}**\n\n", result.message);
                    for txn in &result.transactions {
                        let sec_str = txn.security_name.as_ref()
                            .map(|s| {
                                let ticker = txn.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                                format!(" - {}{}", s, ticker)
                            })
                            .unwrap_or_default();
                        let shares_str = txn.shares.map(|s| format!(", {:.4} Stk.", s)).unwrap_or_default();
                        output.push_str(&format!(
                            "- {}: {}{}, {:.2} {}{}\n",
                            txn.date, txn.txn_type, sec_str, txn.amount, txn.currency, shares_str
                        ));
                    }
                    results.push(output);
                }
            }
            Err(e) => {
                results.push(format!("Fehler bei Transaktionsabfrage: {}", e));
            }
        }
    }

    results
}

/// Chat with portfolio assistant
///
/// Sends user messages to AI with portfolio context injected.
/// The AI can execute watchlist commands embedded in responses.
#[command]
pub async fn chat_with_portfolio_assistant(
    app: AppHandle,
    request: PortfolioChatRequest,
) -> Result<PortfolioChatResponse, String> {
    // Load portfolio context from database
    let context = load_portfolio_context(&request.base_currency)?;

    // Auto-upgrade deprecated models
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    // Call the appropriate provider
    let result = match request.provider.as_str() {
        "claude" => claude::chat(&model, &request.api_key, &request.messages, &context).await,
        "openai" => openai::chat(&model, &request.api_key, &request.messages, &context).await,
        "gemini" => gemini::chat(&model, &request.api_key, &request.messages, &context).await,
        "perplexity" => perplexity::chat(&model, &request.api_key, &request.messages, &context).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    // Process the result
    match result {
        Ok(mut response) => {
            let mut current_response = response.response.clone();
            let mut additional_results: Vec<String> = Vec::new();

            // Parse and execute watchlist commands
            let (wl_commands, cleaned) = parse_watchlist_commands(&current_response);
            current_response = cleaned;

            if !wl_commands.is_empty() {
                let results = execute_watchlist_commands(&wl_commands, request.alpha_vantage_api_key).await;
                let _ = app.emit("watchlist-updated", ());
                additional_results.extend(results);
            }

            // Parse and execute transaction queries
            let (txn_queries, cleaned) = parse_transaction_queries(&current_response);
            current_response = cleaned;

            if !txn_queries.is_empty() {
                let results = execute_transaction_queries(&txn_queries);
                additional_results.extend(results);
            }

            // Update response
            response.response = current_response;

            // If there are command results, append or replace
            if !additional_results.is_empty() {
                if response.response.trim().is_empty() || response.response.len() < 10 {
                    response.response = additional_results.join("\n\n");
                } else {
                    response.response = format!("{}\n\n{}", response.response, additional_results.join("\n\n"));
                }
            }

            Ok(response)
        }
        Err(e) => Err(serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())),
    }
}
