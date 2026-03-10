//! Market Screener commands
//!
//! Scans entire market indices (DAX 40, S&P 500, etc.) by fetching
//! live data from Yahoo Finance and running the screener engine.

use crate::ai::{
    claude, gemini, openai, openrouter, perplexity, get_model_upgrade, AiError,
};
use crate::indicators::breakout;
use crate::indicators::market_indices;
use crate::indicators::screener::run_screener;
use crate::indicators::types::{BreakoutScore, OHLCData, ScreenerFilter, ScreenerResult, ScreenerSecurityData};
use crate::quotes;
use crate::security;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{command, AppHandle, Emitter};

static MARKET_SCREENER_CANCEL: AtomicBool = AtomicBool::new(false);

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketScreenerRequest {
    pub index_id: String,
    pub filters: Vec<ScreenerFilter>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketScreenerProgress {
    pub current: usize,
    pub total: usize,
    pub current_symbol: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketScreenerResponse {
    pub results: Vec<ScreenerResult>,
    pub total_scanned: usize,
    pub total_errors: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketScreenerAiRequest {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub index_name: String,
    pub filter_description: String,
    pub results: Vec<MarketScreenerAiResultItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketScreenerAiResultItem {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub currency: String,
    pub change_1d: Option<f64>,
    pub change_5d: Option<f64>,
    pub change_20d: Option<f64>,
    pub rsi: Option<f64>,
    pub matched_filters: Vec<String>,
    pub breakout_score: Option<BreakoutScore>,
}

// ============================================================================
// Helper: Yahoo Quote -> OHLCData
// ============================================================================

fn quote_to_ohlc(q: &quotes::Quote) -> OHLCData {
    OHLCData {
        time: q.date.to_string(),
        open: q.open.unwrap_or(q.close),
        high: q.high.unwrap_or(q.close),
        low: q.low.unwrap_or(q.close),
        close: q.close,
        volume: q.volume.map(|v| v as f64),
    }
}

// ============================================================================
// Commands
// ============================================================================

/// Get available market indices
#[command]
pub fn get_market_indices() -> Vec<market_indices::MarketIndex> {
    market_indices::get_available_indices()
}

/// Cancel a running market screener scan
#[command]
pub fn cancel_market_screener() {
    MARKET_SCREENER_CANCEL.store(true, Ordering::Relaxed);
    log::info!("Market screener cancel requested");
}

/// Scan a market index with Yahoo Finance data
#[command]
pub async fn screen_market(
    app: AppHandle,
    request: MarketScreenerRequest,
) -> Result<MarketScreenerResponse, String> {
    security::check_rate_limit("market_screener", &security::RateLimitConfig {
        min_interval: std::time::Duration::from_secs(10),
        max_requests_per_window: 10,
        window_duration: std::time::Duration::from_secs(300),
    })?;

    MARKET_SCREENER_CANCEL.store(false, Ordering::Relaxed);

    let tickers = market_indices::get_index_tickers(&request.index_id)
        .ok_or_else(|| format!("Unbekannter Index: {}", request.index_id))?;

    let total = tickers.len();
    let today = Utc::now().date_naive();
    let six_months_ago = today - chrono::Duration::days(180);

    log::info!("Market screener: scanning {} tickers for index {}", total, request.index_id);

    // Fetch data concurrently with a semaphore (max 5 parallel)
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(5));
    let app_ref = std::sync::Arc::new(app);
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let handles: Vec<_> = tickers
        .iter()
        .enumerate()
        .map(|(idx, ticker)| {
            let sem = semaphore.clone();
            let symbol = ticker.symbol.clone();
            let name = ticker.name.clone();
            let app_clone = app_ref.clone();
            let cnt = counter.clone();
            let from = six_months_ago;
            let to = today;

            tokio::spawn(async move {
                let _permit = sem.acquire().await.map_err(|e| e.to_string())?;

                // Check cancellation
                if MARKET_SCREENER_CANCEL.load(Ordering::Relaxed) {
                    return Err("Abgebrochen".to_string());
                }

                let current = cnt.fetch_add(1, Ordering::Relaxed) + 1;

                // Emit progress
                let _ = app_clone.emit("market_screener_progress", MarketScreenerProgress {
                    current,
                    total,
                    current_symbol: symbol.clone(),
                    status: "loading".into(),
                });

                // Small delay to avoid Yahoo rate limiting
                if idx > 0 && idx % 5 == 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                }

                // Fetch from Yahoo
                let quotes = quotes::yahoo::fetch_historical(&symbol, from, to, true)
                    .await
                    .map_err(|e| format!("{}: {}", symbol, e))?;

                if quotes.len() < 20 {
                    return Err(format!("{}: Nur {} Datenpunkte (min. 20)", symbol, quotes.len()));
                }

                let ohlc: Vec<OHLCData> = quotes.iter().map(quote_to_ohlc).collect();

                Ok(ScreenerSecurityData {
                    security_id: -(idx as i64 + 1), // negative IDs for external
                    name,
                    ticker: Some(symbol),
                    isin: None,
                    currency: None, // Yahoo doesn't return currency in historical
                    ohlc_data: ohlc,
                })
            })
        })
        .collect();

    // Collect results
    let mut securities_data = Vec::new();
    let mut errors = Vec::new();

    for handle in handles {
        match handle.await {
            Ok(Ok(data)) => securities_data.push(data),
            Ok(Err(e)) => {
                if e == "Abgebrochen" {
                    return Err("Scan abgebrochen".to_string());
                }
                errors.push(e);
            }
            Err(e) => errors.push(format!("Task error: {}", e)),
        }
    }

    let total_scanned = securities_data.len();
    let total_errors = errors.len();

    log::info!(
        "Market screener: {}/{} tickers loaded, {} errors",
        total_scanned,
        total,
        total_errors
    );

    // Emit final progress
    let _ = app_ref.emit("market_screener_progress", MarketScreenerProgress {
        current: total,
        total,
        current_symbol: String::new(),
        status: "done".into(),
    });

    // Run screener filters
    let active_filters: Vec<_> = request.filters.iter().filter(|f| f.enabled).cloned().collect();
    let mut results = if active_filters.is_empty() {
        // No filters: return all with basic data
        run_screener(&securities_data, &[])
    } else {
        run_screener(&securities_data, &active_filters)
    };

    // Calculate breakout scores for all results
    // First, collect OHLC data references for relative strength percentile calculation
    let ohlc_refs: Vec<&[OHLCData]> = securities_data
        .iter()
        .map(|s| s.ohlc_data.as_slice())
        .collect();
    let percentiles = breakout::calculate_relative_strength_percentiles(&ohlc_refs);

    // Build a lookup: security_id -> (index in securities_data, percentile)
    let percentile_map: std::collections::HashMap<i64, f64> = securities_data
        .iter()
        .enumerate()
        .map(|(i, s)| (s.security_id, percentiles[i]))
        .collect();

    // Build OHLC lookup: security_id -> ohlc_data slice
    let ohlc_map: std::collections::HashMap<i64, &[OHLCData]> = securities_data
        .iter()
        .map(|s| (s.security_id, s.ohlc_data.as_slice()))
        .collect();

    // Calculate breakout score for each result
    for result in &mut results {
        if let Some(ohlc) = ohlc_map.get(&result.security_id) {
            let pct = percentile_map.get(&result.security_id).copied();
            result.breakout_score = Some(breakout::score_breakout(ohlc, pct));
        }
    }

    Ok(MarketScreenerResponse {
        results,
        total_scanned,
        total_errors,
        errors: errors.into_iter().take(10).collect(), // Limit error list
    })
}

/// AI analysis of market screener results
#[command]
pub async fn analyze_market_screener_with_ai(
    request: MarketScreenerAiRequest,
) -> Result<String, String> {
    security::check_rate_limit("ai_analysis", &security::limits::ai_analysis())?;

    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    // Build results table
    let mut table = String::from("| # | Symbol | Name | Kurs | 1T% | 5T% | 20T% | RSI | Breakout | Filter |\n");
    table.push_str("|---|--------|------|------|-----|-----|------|-----|----------|--------|\n");
    for (i, r) in request.results.iter().enumerate() {
        let breakout_info = r.breakout_score.as_ref().map(|bs| {
            format!("{}/12 ({})", bs.total_points, bs.classification_label)
        }).unwrap_or_else(|| "-".to_string());

        table.push_str(&format!(
            "| {} | {} | {} | {:.2} {} | {} | {} | {} | {} | {} | {} |\n",
            i + 1,
            r.symbol,
            r.name,
            r.price,
            r.currency,
            r.change_1d.map(|v| format!("{:+.1}%", v)).unwrap_or_default(),
            r.change_5d.map(|v| format!("{:+.1}%", v)).unwrap_or_default(),
            r.change_20d.map(|v| format!("{:+.1}%", v)).unwrap_or_default(),
            r.rsi.map(|v| format!("{:.0}", v)).unwrap_or_default(),
            breakout_info,
            r.matched_filters.join(", "),
        ));
    }

    // Build detailed breakout analysis for top candidates
    let mut breakout_details = String::new();
    for r in request.results.iter() {
        if let Some(bs) = &r.breakout_score {
            if bs.total_points >= 4 {
                breakout_details.push_str(&format!("\n### {} ({})\n", r.symbol, r.name));
                breakout_details.push_str(&format!("**Gesamt: {}/12 — {}**\n", bs.total_points, bs.classification_label));
                if bs.downgraded {
                    if let Some(reason) = &bs.downgrade_reason {
                        breakout_details.push_str(&format!("⚠️ Herabgestuft: {}\n", reason));
                    }
                }
                for rule in &bs.rules {
                    let indicator = if rule.points == rule.max_points { "✅" } else if rule.points > 0 { "🟡" } else { "❌" };
                    breakout_details.push_str(&format!("- {} **{}** ({}/{}): {}\n",
                        indicator, rule.rule_name, rule.points, rule.max_points, rule.description));
                }
            }
        }
    }

    let prompt = format!(
        r##"Du bist ein erfahrener Marktanalyst und Breakout-Trader. Analysiere die folgenden Screener-Ergebnisse.

## Kontext
- **Index:** {}
- **Filter:** {}
- **Anzahl Treffer:** {}

## Ergebnisse

{}

## Breakout-Scoring (6-Regeln-System)

Das Breakout-Scoring bewertet jede Aktie mit 6 Regeln (je 0-2 Punkte, max. 12):
1. **Trendfilter** — Kurs vs. MA50/MA200 (Aufwärtstrend = 2, Neutral = 1, Abwärts = 0)
2. **Konsolidierung** — Enge Range nahe 20-Tage-Hoch (Tight + Near High = 2)
3. **Breakout-Trigger** — Close über 20-Tage-Hoch (Bestätigt = 2, Intraday = 1)
4. **Volumen** — Überdurchschnittliches Volumen (>1.5x = 2, >1.1x = 1)
5. **Pullback-Qualität** — Kontrollierter Rücksetzer zu MA20/50 + Drehung
6. **Relative Stärke** — Performance-Perzentil im Universum (Top 20% = 2)

### Bewertungsklassen:
- **10-12 Punkte: Ausbruch sehr wahrscheinlich** → Sofort handeln, Entry planen
- **7-9 Punkte: Ausbruch wahrscheinlich** → Entry vorbereiten, auf Trigger warten
- **4-6 Punkte: Ausbruch möglich** → Watchlist, auf Verbesserung warten
- **0-3 Punkte: Ausbruch unwahrscheinlich** → Überspringen

### Detail-Analyse der Kandidaten
{}

## Aufgabe

1. **Top 3-5 Kandidaten:** Welche Aktien haben das beste Breakout-Setup? Bewerte anhand des 6-Regeln-Scores und begründe kurz.
2. **Regelanalyse:** Hebe für jeden Top-Kandidaten hervor, welche Regeln stark (2/2) und welche schwach (0/2) sind. Was fehlt für ein perfektes Setup?
3. **Gemeinsame Muster:** Gibt es auffällige Gemeinsamkeiten bei den Treffern (z.B. alle im Aufwärtstrend, alle mit Volumen-Bestätigung)?
4. **Risiken:** Worauf sollte man achten? Gibt es Warnsignale (z.B. Ausbrüche ohne Volumen, beschädigte Trends)?
5. **Handlungsempfehlung:** Gib für jeden Top-Kandidaten eine konkrete Handlungsempfehlung:
   - **Sofort handeln** (10-12 Punkte): Entry-Zone, Stop-Loss, Zielbereich
   - **Vorbereiten** (7-9 Punkte): Worauf warten? Welcher Trigger fehlt?
   - **Beobachten** (4-6 Punkte): Was muss sich verbessern?

Antworte auf Deutsch, strukturiert mit Markdown-Überschriften."##,
        request.index_name,
        request.filter_description,
        request.results.len(),
        table,
        breakout_details,
    );

    let result = match request.provider.as_str() {
        "claude" => claude::complete_text(&model, &request.api_key, &prompt).await,
        "openai" => openai::complete_text(&model, &request.api_key, &prompt).await,
        "gemini" => gemini::complete_text(&model, &request.api_key, &prompt).await,
        "perplexity" => perplexity::complete_text(&model, &request.api_key, &prompt).await,
        "openrouter" => openrouter::complete_text(&model, &request.api_key, &prompt).await,
        _ => Err(AiError::other("Unknown", &model, &format!("Unbekannter Anbieter: {}", request.provider))),
    };

    result.map_err(|e| {
        serde_json::to_string(&e).unwrap_or_else(|_| e.message.clone())
    })
}
