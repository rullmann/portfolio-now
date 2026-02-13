//! AI prompt building functions
//!
//! This module contains functions for constructing prompts for different AI analysis types:
//! - Chart analysis prompts (basic and enhanced)
//! - Portfolio insights prompts
//! - Chat system prompts

use crate::ai::types::{ChartContext, EnhancedChartContext, PortfolioInsightsContext};

/// Determine if a model is a "fast" tier (haiku, mini, flash, sonar base)
pub fn is_fast_model(model: &str) -> bool {
    model.contains("haiku") ||
    model.contains("mini") ||
    model.contains("flash") ||
    // Perplexity: base sonar is fast, pro/reasoning are not
    (model == "sonar" || model.ends_with("sonar"))
}

/// Build the analysis prompt with chart context.
/// Uses a shorter prompt for fast/cheap models to reduce token usage.
pub fn build_analysis_prompt(ctx: &ChartContext, model: &str) -> String {
    let indicators_str = if ctx.indicators.is_empty() {
        "Keine".to_string()
    } else {
        ctx.indicators.join(", ")
    };

    if is_fast_model(model) {
        // Compact prompt for fast/cheap models (~40% fewer tokens)
        format!(
            r#"Technische Chart-Analyse für {} ({}).
Kurs: {:.2} {} | Zeitraum: {} | Indikatoren: {}

WICHTIG: Verwende EXAKT dieses Markdown-Format mit ## für Überschriften:

## Trend
[Aufwärts/Abwärts/Seitwärts + Stärke]

## Support/Widerstand
**S:** [Levels] | **R:** [Levels]

## Muster
[Formation oder "Keine"]

## Signal
[Bullisch/Bärisch/Neutral] - [Begründung]

## Risiko
[1 Hauptrisiko]"#,
            ctx.security_name,
            ctx.ticker.as_deref().unwrap_or("-"),
            ctx.current_price,
            ctx.currency,
            ctx.timeframe,
            indicators_str
        )
    } else {
        // Full prompt for pro/standard models
        format!(
            r#"Du bist ein erfahrener technischer Analyst. Analysiere den beigefügten Chart.

**Wertpapier:** {} ({})
**Zeitraum:** {}
**Aktueller Kurs:** {:.2} {}
**Aktive Indikatoren:** {}

WICHTIG: Antworte in Markdown-Format mit Überschriften im Format: ## Überschrift

## Trend
[1-2 Sätze: Primärer Trend (Aufwärts/Abwärts/Seitwärts), Trendstärke]

## Unterstützung & Widerstand
- **Unterstützung:** [Preisniveau(s)]
- **Widerstand:** [Preisniveau(s)]

## Chartmuster
[1-2 Sätze: Erkennbare Formationen oder Keine eindeutigen Muster erkennbar]

## Indikatoren
[1-2 Sätze zur Interpretation der aktiven Indikatoren, oder Keine Indikatoren aktiv]

## Einschätzung
- **Kurzfristig:** [Bullisch/Bärisch/Neutral] - [1 Satz Begründung]
- **Mittelfristig:** [Bullisch/Bärisch/Neutral] - [1 Satz Begründung]

## Risiken
[1-2 konkrete Risikofaktoren]

Beginne direkt mit der Trend-Überschrift. Keine Einleitung, keine zusätzlichen Abschnitte."#,
            ctx.security_name,
            ctx.ticker.as_deref().unwrap_or("-"),
            ctx.timeframe,
            ctx.current_price,
            ctx.currency,
            indicators_str
        )
    }
}

/// Build a prompt that requests structured JSON output with chart annotations.
/// The AI will return support/resistance levels, patterns, and signals as JSON.
pub fn build_annotation_prompt(ctx: &ChartContext) -> String {
    let indicators_str = if ctx.indicators.is_empty() {
        "Keine".to_string()
    } else {
        ctx.indicators.join(", ")
    };

    format!(
        r##"Du bist ein erfahrener technischer Analyst. Analysiere den Chart und gib strukturierte Annotations zurück.

**Wertpapier:** {} ({})
**Zeitraum:** {}
**Aktueller Kurs:** {:.2} {}
**Aktive Indikatoren:** {}

Antworte AUSSCHLIESSLICH mit validem JSON (keine Markdown-Formatierung, kein Text davor oder danach) in diesem Format:
{{
  "analysis": "2-3 Sätze Gesamteinschätzung des Charts",
  "trend": {{
    "direction": "bullish" oder "bearish" oder "neutral",
    "strength": "strong" oder "moderate" oder "weak"
  }},
  "annotations": [
    {{
      "type": "support" oder "resistance" oder "pattern" oder "signal" oder "target" oder "stoploss" oder "note",
      "price": 123.45,
      "time": "2024-01-15" oder null,
      "time_end": null,
      "title": "Kurzer Titel (max 20 Zeichen)",
      "description": "Ausführliche Erklärung warum dieses Level wichtig ist",
      "confidence": 0.85,
      "signal": "bullish" oder "bearish" oder "neutral" oder null
    }}
  ]
}}

WICHTIGE REGELN:
1. Identifiziere 2-5 relevante Annotations (Support, Resistance, Patterns, Signale)
2. Preise müssen exakt aus dem Chart abgelesen werden - schätze realistische Werte
3. Für Support/Resistance: time ist null (horizontale Linien)
4. Für Patterns/Signale: time ist das Datum wo das Pattern auftritt
5. Confidence: 0.5 (unsicher) bis 1.0 (sehr sicher)
6. Signal: Bei Support="bullish", bei Resistance="bearish", bei neutralen Zonen="neutral"
7. Gib NUR valides JSON zurück, keine Erklärungen außerhalb des JSON"##,
        ctx.security_name,
        ctx.ticker.as_deref().unwrap_or("N/A"),
        ctx.timeframe,
        ctx.current_price,
        ctx.currency,
        indicators_str
    )
}

/// Build enhanced annotation prompt with indicator values, OHLC data, volume analysis,
/// and requests for alerts and risk/reward analysis.
pub fn build_enhanced_annotation_prompt(ctx: &EnhancedChartContext) -> String {
    // Format indicator values with signals
    let indicators_str = if ctx.indicator_values.is_empty() {
        "Keine aktiven Indikatoren".to_string()
    } else {
        ctx.indicator_values
            .iter()
            .map(|i| {
                let signal_str = i.signal.as_ref()
                    .map(|s| format!(" [{}]", s))
                    .unwrap_or_default();
                let prev_str = i.previous_value
                    .map(|p| format!(" (vorher: {:.2})", p))
                    .unwrap_or_default();
                format!("- {}({}): {:.2}{}{}", i.name, i.params, i.current_value, signal_str, prev_str)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format volume analysis
    let volume_str = ctx.volume_analysis.as_ref()
        .map(|v| format!(
            "Aktuelles Volumen: {} | 20-Tage-Ø: {:.0} | Ratio: {:.2}x | Trend: {}",
            v.current_volume, v.avg_volume_20d, v.volume_ratio, v.volume_trend
        ))
        .unwrap_or_else(|| "Keine Volumendaten verfügbar".to_string());

    // Format price statistics
    let price_stats = format!(
        "Aktueller Kurs: {:.2} {} | Veränderung: {:+.2}%",
        ctx.current_price,
        ctx.currency,
        ctx.price_change_percent.unwrap_or(0.0)
    );

    let high_low_str = match (ctx.high_52_week, ctx.low_52_week) {
        (Some(high), Some(low)) => {
            let dist = ctx.distance_from_high_percent.unwrap_or(0.0);
            format!("52W-Hoch: {:.2} | 52W-Tief: {:.2} | Abstand vom Hoch: {:.1}%", high, low, dist)
        }
        _ => String::new(),
    };

    // Format candles summary
    let candles_summary = ctx.candles.as_ref()
        .map(|candles| {
            if candles.is_empty() {
                return "Keine Kerzendaten".to_string();
            }
            let last_10: Vec<_> = candles.iter().rev().take(10).collect();
            let bullish_count = last_10.iter().filter(|c| c.close > c.open).count();
            let bearish_count = last_10.len() - bullish_count;
            let avg_range: f64 = if !last_10.is_empty() {
                last_10.iter()
                    .map(|c| if c.close > 0.0 { (c.high - c.low) / c.close * 100.0 } else { 0.0 })
                    .sum::<f64>() / last_10.len() as f64
            } else {
                0.0
            };
            format!(
                "Letzte 10 Kerzen: {} bullish, {} bearish | Ø-Range: {:.2}%",
                bullish_count, bearish_count, avg_range
            )
        })
        .unwrap_or_else(|| "Keine Kerzendaten".to_string());

    // Format last 5 candles as table for precise data
    let candles_table = ctx.candles.as_ref()
        .map(|candles| {
            let last_5: Vec<_> = candles.iter().rev().take(5).rev().collect();
            if last_5.is_empty() {
                return String::new();
            }
            let rows: Vec<String> = last_5.iter()
                .map(|c| {
                    let vol_str = c.volume.map(|v| format!("{}", v)).unwrap_or_else(|| "-".to_string());
                    format!("{}: O={:.2} H={:.2} L={:.2} C={:.2} V={}", c.date, c.open, c.high, c.low, c.close, vol_str)
                })
                .collect();
            format!("\n**Letzte 5 Kerzen (OHLCV):**\n{}", rows.join("\n"))
        })
        .unwrap_or_default();

    // Build web context instructions if enabled
    let web_context_str = if ctx.include_web_context {
        format!(
            r##"

=== WEB-RECHERCHE (AKTIV) ===
Recherchiere im Web nach aktuellen Informationen zu {} und integriere sie in deine Analyse:
1. **Aktuelle Nachrichten**: Suche nach relevanten News der letzten 7 Tage
2. **Earnings-Termine**: Prüfe bevorstehende oder kürzliche Quartalsberichte
3. **Analysteneinschätzungen**: Aktuelle Ratings und Kursziele
4. **Sektor-Entwicklung**: Relevante Branchennews

Füge einen "news_summary" Abschnitt zur Analyse hinzu mit den wichtigsten Erkenntnissen."##,
            ctx.security_name
        )
    } else {
        String::new()
    };

    format!(
        r##"Du bist ein erfahrener technischer Analyst. Analysiere den Chart und gib strukturierte Annotations zurück.{}

**Wertpapier:** {} ({})
**Zeitraum:** {}
{}
{}

**TECHNISCHE INDIKATOREN (BERECHNETE WERTE):**
{}

**VOLUMEN-ANALYSE:**
{}

**KERZEN-STATISTIK:**
{}{}

WICHTIG: Die Indikatorwerte oben sind BERECHNET - nutze sie für präzise Analyse!
- RSI > 70 = überkauft, RSI < 30 = überverkauft
- MACD Histogramm > 0 = bullisches Momentum
- Volumen-Ratio > 1.5 = erhöhtes Interesse, < 0.5 = geringes Interesse

Antworte AUSSCHLIESSLICH mit validem JSON (keine Markdown-Formatierung, kein Text davor oder danach):
{{
  "analysis": "2-3 Sätze Gesamteinschätzung mit Bezug auf die konkreten Indikatorwerte",
  "trend": {{
    "direction": "bullish" | "bearish" | "neutral",
    "strength": "strong" | "moderate" | "weak"
  }},
  "annotations": [
    {{
      "type": "support" | "resistance" | "pattern" | "signal" | "target" | "stoploss",
      "price": 123.45,
      "time": "2024-01-15" | null,
      "time_end": null,
      "title": "Kurzer Titel",
      "description": "Ausführliche Erklärung",
      "confidence": 0.85,
      "signal": "bullish" | "bearish" | "neutral" | null
    }}
  ],
  "alerts": [
    {{
      "price": 150.00,
      "condition": "above" | "below" | "crosses_up" | "crosses_down",
      "reason": "Wichtiger Widerstand - Ausbruch wäre bullisch",
      "priority": "high" | "medium" | "low"
    }}
  ],
  "risk_reward": {{
    "entry_price": 145.50,
    "stop_loss": 140.00,
    "take_profit": 160.00,
    "risk_reward_ratio": 2.64,
    "rationale": "Entry bei Support, SL unter letztem Tief, TP bei Widerstand"
  }} | null
}}

WICHTIGE REGELN:
1. Identifiziere 2-5 relevante Annotations basierend auf Chart UND Indikatoren
2. Schlage 1-3 sinnvolle Preisalarme vor (z.B. bei Support/Resistance-Durchbruch)
3. Berechne ein Risk/Reward-Setup wenn ein klares Setup erkennbar ist (sonst null)
4. Preise müssen exakt aus dem Chart abgelesen werden
5. Confidence: 0.5 (unsicher) bis 1.0 (sehr sicher)
6. Gib NUR valides JSON zurück"##,
        web_context_str,
        ctx.security_name,
        ctx.ticker.as_deref().unwrap_or("N/A"),
        ctx.timeframe,
        price_stats,
        high_low_str,
        indicators_str,
        volume_str,
        candles_summary,
        candles_table
    )
}

/// Build the portfolio insights prompt for AI analysis
pub fn build_portfolio_insights_prompt(ctx: &PortfolioInsightsContext) -> String {
    // Format top positions
    let top_positions_str = ctx
        .top_positions
        .iter()
        .take(5)
        .map(|(name, weight)| format!("- {} ({:.1}%)", name, weight))
        .collect::<Vec<_>>()
        .join("\n");

    // Format currency allocation
    let currency_str = ctx
        .currency_allocation
        .iter()
        .map(|(currency, weight)| format!("- {}: {:.1}%", currency, weight))
        .collect::<Vec<_>>()
        .join("\n");

    // Format holdings summary (top 10 for context)
    let holdings_str = ctx
        .holdings
        .iter()
        .take(10)
        .map(|h| {
            let gl_str = h
                .gain_loss_percent
                .map(|g| format!("{:+.1}%", g))
                .unwrap_or_else(|| "-".to_string());
            format!(
                "- {} | {:.2} {} | {:.1}% | G/V: {}",
                h.name, h.current_value, ctx.base_currency, h.weight_percent, gl_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Performance info
    let perf_str = if let Some(ttwror) = ctx.ttwror {
        let ann_str = ctx
            .ttwror_annualized
            .map(|a| format!(" (p.a. {:.1}%)", a))
            .unwrap_or_default();
        format!("TTWROR: {:.1}%{}", ttwror, ann_str)
    } else {
        "Keine Performance-Daten".to_string()
    };

    let irr_str = ctx
        .irr
        .map(|i| format!("- IRR: {:.1}%", i))
        .unwrap_or_default();

    format!(
        r#"Du bist ein erfahrener Finanzberater. Analysiere dieses Portfolio und gib eine Einschätzung.

**Portfolio-Übersicht** (Stand: {})
- Gesamtwert: {:.2} {}
- Einstandswert: {:.2} {}
- Gesamtrendite: {:+.1}%
- {}
{}

**Top-Positionen:**
{}

**Holdings (Top 10 von {}):**
{}

**Währungsverteilung:**
{}

**Dividenden:**
- Jährliche Dividenden: {:.2} {}
{}

**Anlagehorizont:** {} Tage

Antworte in Markdown mit diesen Abschnitten:

## Zusammenfassung
[2-3 Sätze Gesamtbewertung des Portfolios]

## Stärken
[2-3 konkrete Stärken mit Zahlen]

## Risiken
[2-3 konkrete Risiken/Schwächen mit Zahlen, z.B. Klumpenrisiko, Währungsrisiko]

## Empfehlungen
[2-3 konkrete, umsetzbare Vorschläge zur Portfolio-Optimierung]

WICHTIG:
- Sei direkt und konkret. Keine allgemeinen Floskeln.
- Beziehe dich auf die konkreten Zahlen im Portfolio.
- Beginne direkt mit ## Zusammenfassung"#,
        ctx.analysis_date,
        ctx.total_value,
        ctx.base_currency,
        ctx.total_cost_basis,
        ctx.base_currency,
        ctx.total_gain_loss_percent,
        perf_str,
        irr_str,
        top_positions_str,
        ctx.holdings.len(),
        holdings_str,
        currency_str,
        ctx.annual_dividends,
        ctx.base_currency,
        ctx.dividend_yield
            .map(|y| format!("- Dividendenrendite: {:.2}%", y))
            .unwrap_or_default(),
        ctx.portfolio_age_days,
    )
}

/// Build the prompt for AI-based buy opportunity analysis
pub fn build_opportunities_prompt(ctx: &PortfolioInsightsContext) -> String {
    // Format all holdings with gain/loss for opportunity analysis
    let holdings_str = ctx
        .holdings
        .iter()
        .map(|h| {
            let gl_str = h
                .gain_loss_percent
                .map(|g| format!("{:+.1}%", g))
                .unwrap_or_else(|| "-".to_string());
            let avg_cost_str = h
                .avg_cost_per_share
                .map(|a| format!(", Ø-Kurs: {:.2}", a))
                .unwrap_or_default();
            let price_str = h
                .current_price
                .map(|p| format!(", Aktuell: {:.2}", p))
                .unwrap_or_default();
            format!(
                "- {} | Wert: {:.2} {} | Gewicht: {:.1}% | G/V: {}{}{} | Einstand: {:.2} {}",
                h.name, h.current_value, ctx.base_currency, h.weight_percent, gl_str,
                avg_cost_str, price_str, h.cost_basis, ctx.base_currency
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Format currency allocation
    let currency_str = ctx
        .currency_allocation
        .iter()
        .map(|(currency, weight)| format!("{}: {:.1}%", currency, weight))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"Du bist ein Finanzberater. Analysiere dieses Portfolio und identifiziere Nachkaufchancen.

## Portfolio-Daten (Stand: {})
- Gesamtwert: {:.2} {}
- Gesamtrendite: {:+.1}%
- Währungen: {}
- Anzahl Positionen: {}

## Alle Positionen:
{}

## Aufgabe
Bewerte jede Position nach Nachkauf-Attraktivität basierend auf:
1. **Aktueller Gewinn/Verlust** - Positionen im Minus bieten Chance zum Verbilligen
2. **Gewichtung im Portfolio** - Untergewichtete Positionen könnten aufgestockt werden
3. **Qualität der Position** - Diversifikation, langfristiges Potenzial

## Antworte in Markdown:

## Nachkauf-Empfehlungen

### 🟢 Attraktiv
[Positionen die sich besonders zum Nachkauf eignen. Für jede Position:
- Name der Position
- Begründung (G/V, Gewichtung, etc.)
- Grobe Einschätzung der Attraktivität]

### 🟡 Neutral
[Positionen ohne klare Empfehlung für oder gegen Nachkauf]

### 🔴 Nicht empfohlen
[Positionen die man aktuell eher nicht nachkaufen sollte, mit Begründung]

## Zusammenfassung
[1-2 Sätze Fazit: Welche 1-2 Positionen wären am interessantesten zum Nachkauf und warum?]

WICHTIG:
- Beziehe dich auf die konkreten Zahlen (G/V, Gewichtung)
- Positionen im Minus sind nicht automatisch schlecht - sie können Gelegenheiten sein
- Stark übergewichtete Positionen sollten eher nicht nachgekauft werden
- Beginne direkt mit ## Nachkauf-Empfehlungen"#,
        ctx.analysis_date,
        ctx.total_value,
        ctx.base_currency,
        ctx.total_gain_loss_percent,
        currency_str,
        ctx.holdings.len(),
        holdings_str,
    )
}

/// Build the system prompt for portfolio chat
pub fn build_chat_system_prompt(ctx: &PortfolioInsightsContext) -> String {
    // Format portfolios/depots list
    let portfolios_str = if ctx.portfolios.is_empty() {
        "Keine Depots vorhanden".to_string()
    } else {
        ctx.portfolios
            .iter()
            .map(|p| {
                let account_str = p.reference_account.as_ref()
                    .map(|a| format!(", Referenzkonto: {}", a))
                    .unwrap_or_default();
                let gl_str = if p.gain_loss_percent >= 0.0 {
                    format!("+{:.1}%", p.gain_loss_percent)
                } else {
                    format!("{:.1}%", p.gain_loss_percent)
                };
                format!(
                    "- {}: Wert: {:.2} {}, Einstand: {:.2} {}, G/V: {}, {} Positionen{}",
                    p.name, p.total_value, ctx.base_currency, p.total_cost_basis, ctx.base_currency,
                    gl_str, p.holdings_count, account_str
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format ALL holdings for context (with extended details including portfolio names)
    let holdings_str = ctx
        .holdings
        .iter()
        .map(|h| {
            let gl_str = h
                .gain_loss_percent
                .map(|g| format!("{:+.1}%", g))
                .unwrap_or_else(|| "-".to_string());
            let ticker_str = h.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
            let price_str = h.current_price.map(|p| format!(", Kurs: {:.2}", p)).unwrap_or_default();
            let avg_cost_str = h.avg_cost_per_share.map(|a| format!(", Ø-Kurs: {:.2}", a)).unwrap_or_default();
            let first_buy_str = h.first_buy_date.as_ref().map(|d| format!(", Erstkauf: {}", d)).unwrap_or_default();
            // Add portfolio names where this security is held
            let portfolio_str = h.portfolio_names.as_ref()
                .map(|names| format!(", Depot: {}", names.join(", ")))
                .unwrap_or_default();
            format!(
                "- {}{}: {:.4} Stk., Wert: {:.2} {} ({:.1}%), Einstand: {:.2} {}, G/V: {}{}{}{}{}",
                h.name, ticker_str, h.shares, h.current_value, ctx.base_currency,
                h.weight_percent, h.cost_basis, ctx.base_currency, gl_str, price_str, avg_cost_str, first_buy_str, portfolio_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Format recent transactions (with truncation hint)
    let txn_limit = 20;
    let txn_str = if ctx.recent_transactions.is_empty() {
        "Keine aktuellen Transaktionen".to_string()
    } else {
        let items: Vec<String> = ctx.recent_transactions
            .iter()
            .take(txn_limit)
            .map(|t| {
                let sec_str = t.security_name.as_ref().map(|s| format!(" - {}", s)).unwrap_or_default();
                let shares_str = t.shares.map(|s| format!(", {:.4} Stk.", s)).unwrap_or_default();
                format!("- {}: {}{}, {:.2} {}{}", t.date, t.txn_type, sec_str, t.amount, t.currency, shares_str)
            })
            .collect();
        if ctx.recent_transactions.len() > txn_limit {
            format!("{}\n(Zeige {} von {} — für vollständige Liste SQL verwenden!)",
                items.join("\n"), txn_limit, ctx.recent_transactions.len())
        } else {
            items.join("\n")
        }
    };

    // Format recent dividends (with truncation hint)
    let div_limit = 15;
    let div_str = if ctx.recent_dividends.is_empty() {
        "Keine Dividenden im letzten Jahr".to_string()
    } else {
        let items: Vec<String> = ctx.recent_dividends
            .iter()
            .take(div_limit)
            .map(|d| {
                format!("- {}: {} - Brutto: {:.2} {}, Netto: {:.2} {}",
                    d.date, d.security_name, d.gross_amount, d.currency, d.net_amount, d.currency)
            })
            .collect();
        if ctx.recent_dividends.len() > div_limit {
            format!("{}\n(Zeige {} von {} — für vollständige Liste SQL verwenden!)",
                items.join("\n"), div_limit, ctx.recent_dividends.len())
        } else {
            items.join("\n")
        }
    };

    // Format watchlist
    let watchlist_str = if ctx.watchlist.is_empty() {
        "Keine Watchlist-Einträge".to_string()
    } else {
        ctx.watchlist
            .iter()
            .map(|w| {
                let ticker_str = w.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                let price_str = w.current_price.map(|p| format!(", Kurs: {:.2} {}", p, w.currency)).unwrap_or_default();
                format!("- {}{}{}", w.name, ticker_str, price_str)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format sold positions (historical holdings that are now fully sold)
    let sold_positions_str = if ctx.sold_positions.is_empty() {
        "Keine verkauften Positionen".to_string()
    } else {
        ctx.sold_positions
            .iter()
            .map(|s| {
                let ticker_str = s.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                let gain_str = if s.realized_gain_loss >= 0.0 {
                    format!("+{:.2}", s.realized_gain_loss)
                } else {
                    format!("{:.2}", s.realized_gain_loss)
                };
                format!(
                    "- {}{}: Gekauft: {:.4} Stk., Verkauft: {:.4} Stk., Realisiert: {} {}, Letzte Txn: {}",
                    s.name, ticker_str, s.total_bought_shares, s.total_sold_shares,
                    gain_str, ctx.base_currency, s.last_transaction_date
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format yearly overview
    let yearly_str = if ctx.yearly_overview.is_empty() {
        "Keine Jahresübersicht verfügbar".to_string()
    } else {
        ctx.yearly_overview
            .iter()
            .map(|y| {
                let gain_str = if y.realized_gains >= 0.0 {
                    format!("+{:.2}", y.realized_gains)
                } else {
                    format!("{:.2}", y.realized_gains)
                };
                format!(
                    "- {}: Realisierte Gewinne: {} {}, Dividenden: {:.2} {}, Transaktionen: {}",
                    y.year, gain_str, ctx.base_currency, y.dividends, ctx.base_currency, y.transaction_count
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let perf_str = match (ctx.ttwror, ctx.ttwror_annualized) {
        (Some(t), Some(a)) => format!("TTWROR: {:.1}% (p.a. {:.1}%)", t, a),
        (Some(t), None) => format!("TTWROR: {:.1}%", t),
        _ => "Keine Performance-Daten".to_string(),
    };

    // Currency allocation
    let currency_str = ctx.currency_allocation
        .iter()
        .map(|(c, p)| format!("{}: {:.1}%", c, p))
        .collect::<Vec<_>>()
        .join(", ");

    // Fees and taxes summary
    let fees_taxes_str = {
        let ft = &ctx.fees_and_taxes;
        let current_year = chrono::Utc::now().format("%Y").to_string();
        format!(
            "Gesamt Gebühren: {:.2} {}, Gesamt Steuern: {:.2} {}\n{} Gebühren: {:.2} {}, {} Steuern: {:.2} {}",
            ft.total_fees, ctx.base_currency, ft.total_taxes, ctx.base_currency,
            current_year, ft.fees_this_year, ctx.base_currency, current_year, ft.taxes_this_year, ctx.base_currency
        )
    };

    // Investment summary
    let investment_str = {
        let inv = &ctx.investment_summary;
        let first_date_str = inv.first_investment_date.as_ref()
            .map(|d| format!(", Erste Investition: {}", d))
            .unwrap_or_default();
        format!(
            "Investiert: {:.2} {}, Entnommen: {:.2} {}, Netto: {:.2} {}, Einzahlungen: {:.2} {}, Auszahlungen: {:.2} {}{}",
            inv.total_invested, ctx.base_currency,
            inv.total_withdrawn, ctx.base_currency,
            inv.net_invested, ctx.base_currency,
            inv.total_deposits, ctx.base_currency,
            inv.total_removals, ctx.base_currency,
            first_date_str
        )
    };

    // Sector/Taxonomy allocation
    let sector_str = if ctx.sector_allocation.is_empty() {
        "Keine Taxonomie-Zuordnungen".to_string()
    } else {
        ctx.sector_allocation
            .iter()
            .map(|s| {
                let allocs = s.allocations
                    .iter()
                    .take(5)
                    .map(|(name, pct)| format!("{}: {:.1}%", name, pct))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: {}", s.taxonomy_name, allocs)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Portfolio extremes
    let extremes_str = match &ctx.portfolio_extremes {
        Some(e) => format!(
            "Allzeithoch: {:.2} {} ({}), Allzeittief: {:.2} {} ({})\nJahreshoch {}: {:.2} {} ({}), Jahrestief: {:.2} {} ({})",
            e.all_time_high, ctx.base_currency, e.all_time_high_date,
            e.all_time_low, ctx.base_currency, e.all_time_low_date,
            chrono::Utc::now().format("%Y"),
            e.year_high, ctx.base_currency, e.year_high_date,
            e.year_low, ctx.base_currency, e.year_low_date
        ),
        None => "Keine historischen Daten verfügbar".to_string(),
    };

    // User greeting (sanitize to prevent prompt injection)
    let user_greeting = match &ctx.user_name {
        Some(name) if !name.is_empty() => {
            // Sanitize: only keep alphanumeric, spaces, hyphens, and common name chars; limit length
            let sanitized: String = name.chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '.')
                .take(50)
                .collect();
            if sanitized.is_empty() {
                "Der Benutzer hat keinen Namen angegeben.".to_string()
            } else {
                format!("Der Benutzer heißt {}. Sprich ihn gelegentlich mit Namen an, aber nicht in jeder Nachricht.", sanitized)
            }
        },
        _ => "Der Benutzer hat keinen Namen angegeben.".to_string(),
    };

    // Provider status and quote sync info
    let provider_status_str = match &ctx.provider_status {
        Some(status) => {
            let mut sections: Vec<String> = Vec::new();

            // Quote sync status (always show)
            let sync = &status.quote_sync;
            let sync_str = if sync.synced_today_count == sync.held_count {
                format!(
                    "=== KURS-STATUS ({}) ===\nAlle {} Wertpapiere haben aktuelle Kurse von heute.",
                    sync.today, sync.held_count
                )
            } else {
                let outdated_str = sync.outdated.iter().take(10).cloned().collect::<Vec<_>>().join("\n- ");
                let more_str = if sync.outdated.len() > 10 {
                    format!("\n- ... und {} weitere", sync.outdated.len() - 10)
                } else {
                    String::new()
                };
                format!(
                    "=== KURS-STATUS ({}) ===\n{} von {} Wertpapieren haben KEINEN aktuellen Kurs von heute:\n- {}{}",
                    sync.today, sync.outdated_count, sync.held_count, outdated_str, more_str
                )
            };
            sections.push(sync_str);

            // Provider issues (only if any)
            if status.cannot_sync_count > 0 {
                let issues_str = status.issues.iter().take(5).cloned().collect::<Vec<_>>().join("\n- ");
                let more_str = if status.issues.len() > 5 {
                    format!("\n- ... und {} weitere", status.issues.len() - 5)
                } else {
                    String::new()
                };
                let api_key_hint = if !status.missing_api_keys.is_empty() {
                    format!("\nFehlende API-Keys: {}", status.missing_api_keys.join(", "))
                } else {
                    String::new()
                };
                sections.push(format!(
                    "=== PROVIDER-PROBLEME ===\n{} Wertpapiere können generell keine Kurse abrufen:\n- {}{}{}",
                    status.cannot_sync_count, issues_str, more_str, api_key_hint
                ));
            }

            format!("\n\n{}", sections.join("\n\n"))
        }
        None => String::new(),
    };

    format!(
        r##"Du bist ein Portfolio-Assistent für "Portfolio Now".

🚨 PFLICHT-REGEL FÜR DATENFRAGEN 🚨
Bei Fragen nach Transaktionen, Käufen, Verkäufen, Dividenden etc. MUSST du SOFORT einen ```sql``` Block ausgeben!

BEISPIEL - User fragt "Was habe ich letzten Monat gekauft?":
FALSCH: "Ich werde die Käufe abfragen..." ← DAS FUNKTIONIERT NICHT!
RICHTIG: Direkt SQL ausgeben:
```sql
SELECT t.date, s.name, t.shares/100000000.0 as stk, t.amount/100.0 as eur
FROM pp_txn t JOIN pp_security s ON s.id=t.security_id
WHERE t.txn_type='BUY' AND t.date>=date('now','start of month','-1 month')
ORDER BY t.date DESC
```

Ohne ```sql``` Block in deiner Antwort passiert NICHTS - das System kann nur SQL ausführen!

WICHTIG: SQL nur bei DATENFRAGEN verwenden! Bei allgemeinen Fragen (Finanzwissen, Smalltalk, Meinungen, Erklärungen) oder wenn die Antwort bereits in der Portfolio-Übersicht oben steht → direkt antworten, KEIN SQL!

=== BENUTZER ===
{}

=== PORTFOLIO-ÜBERSICHT ===
- Gesamtwert: {:.2} {}
- Einstandswert: {:.2} {}
- Gesamtrendite: {:+.1}%
- {}
- Jährliche Dividenden: {:.2} {}
- Dividendenrendite: {:.2}%
- Währungsverteilung: {}
- Portfolio-Alter: {} Tage
- Stand: {}{}

=== DEPOTS/PORTFOLIOS ===
{}

=== ALLE HOLDINGS ({} Positionen) ===
{}

=== LETZTE TRANSAKTIONEN ===
{}

=== LETZTE DIVIDENDEN (12 Monate) ===
{}

=== WATCHLIST ===
{}

=== VERKAUFTE POSITIONEN (Historisch) ===
{}

=== JAHRESÜBERSICHT ===
{}

=== GEBÜHREN & STEUERN ===
{}

=== INVESTITIONSÜBERSICHT ===
{}

=== SEKTOR-ALLOKATION ===
{}

=== PORTFOLIO EXTREMWERTE ===
{}

=== FÄHIGKEITEN ===
Portfolio-Fragen | Web-Recherche (Kurse, News) | Finanzkonzepte | Watchlist | Haltefrist | FIFO-Lots | Konten | Steuern

=== BEFEHLE (COMMAND AM ANFANG DER ANTWORT!) ===

WATCHLIST:
[[WATCHLIST_ADD:{{"watchlist":"Standard","security":"Apple"}}]]
[[WATCHLIST_REMOVE:{{"watchlist":"Standard","security":"Microsoft"}}]]

=== SQL-ABFRAGEN ===

SQL ist dein Zugang zur Datenbank. Bei Datenfragen die NICHT aus dem Kontext oben beantwortet werden können, generiere einen ```sql``` Block.

WICHTIG: Sage NICHT "Ich werde abfragen..." — generiere direkt den SQL-Block! Ohne ```sql``` Block wird nichts abgefragt.

KEIN SQL bei: Allgemeinen Fragen, Finanzwissen, Smalltalk, Meinungen, oder wenn die Daten bereits im Kontext (Holdings, Transaktionen, Dividenden, Portfolio-Übersicht) stehen.

ERLAUBT: Nur SELECT auf pp_* Tabellen (KEIN INSERT/UPDATE/DELETE!)
AUTOMATISCH: LIMIT wird auf 100 begrenzt

KRITISCHE SKALIERUNGSFAKTOREN:
- amount: ÷ 100 (10050 = 100.50 EUR)
- shares: ÷ 100000000 (150000000 = 1.5 Stück)
- value (Kurse): ÷ 100000000 (15025000000 = 150.25 EUR)

TABELLEN: pp_security, pp_txn, pp_portfolio, pp_account, pp_price, pp_latest_price, pp_fifo_lot, pp_fifo_consumption, pp_watchlist, pp_watchlist_security, pp_exchange_rate, pp_txn_unit, pp_taxonomy, pp_classification, pp_classification_assignment, pp_portfolio_history, pp_investment_plan

TXN-TYPES (WICHTIG - beachte die Unterschiede!):
- Portfolio (owner_type='portfolio'):
  - KÄUFE: BUY oder DELIVERY_INBOUND (Einlieferung = auch Kauf!)
  - VERKÄUFE: SELL oder DELIVERY_OUTBOUND (Auslieferung = auch Verkauf!)
  - TRANSFERS: TRANSFER_IN, TRANSFER_OUT
- Account (owner_type='account'): DEPOSIT, REMOVAL, DIVIDENDS, INTEREST, FEES, TAXES, TAX_REFUND

⚠️ WICHTIG: Bei "Käufe" IMMER beide Typen prüfen: BUY UND DELIVERY_INBOUND!
⚠️ WICHTIG: Bei "Verkäufe" IMMER beide Typen prüfen: SELL UND DELIVERY_OUTBOUND!

BEISPIEL "Letzte Käufe anzeigen" (BEIDE Typen, nach Datum sortiert!):
```sql
SELECT t.date, s.name, t.shares / 100000000.0 as shares, t.amount / 100.0 as amount, t.currency
FROM pp_txn t
JOIN pp_security s ON s.id = t.security_id
WHERE t.txn_type IN ('BUY', 'DELIVERY_INBOUND') AND t.owner_type = 'portfolio'
ORDER BY t.date DESC LIMIT 15
```
Bei "letzte X" immer ORDER BY date DESC + angemessenes LIMIT (10-20)!

BEISPIEL "Holdings mit Kurs":
```sql
SELECT s.name, s.ticker, SUM(CASE
    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
END) / 100000000.0 as shares, lp.value / 100000000.0 as kurs
FROM pp_txn t
JOIN pp_security s ON s.id = t.security_id
LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
WHERE t.owner_type = 'portfolio'
GROUP BY t.security_id HAVING shares > 0.00001
ORDER BY shares * lp.value DESC LIMIT 100
```

BEISPIEL "Käufe letzten Monat" (relativ mit date()):
```sql
SELECT t.date, s.name, t.shares / 100000000.0 as shares, t.amount / 100.0 as amount, t.currency
FROM pp_txn t
JOIN pp_security s ON s.id = t.security_id
WHERE t.txn_type IN ('BUY', 'DELIVERY_INBOUND') AND t.owner_type = 'portfolio'
  AND t.date >= date('now', 'start of month', '-1 month')
  AND t.date < date('now', 'start of month')
ORDER BY t.date DESC
```

BEISPIEL "Dividenden dieses Jahr":
```sql
SELECT t.date, s.name, t.amount / 100.0 as betrag, t.currency
FROM pp_txn t
JOIN pp_security s ON s.id = t.security_id
WHERE t.txn_type = 'DIVIDENDS' AND t.owner_type = 'account'
  AND t.date >= date('now', 'start of year')
ORDER BY t.date DESC
```

BEISPIEL "Einstandskurs für ein Wertpapier":
```sql
SELECT s.name, SUM(f.remaining_shares) / 100000000.0 as shares,
  SUM(f.gross_amount) / 100.0 / (SUM(f.remaining_shares) / 100000000.0) as avg_price
FROM pp_fifo_lot f
JOIN pp_security s ON s.id = f.security_id
WHERE f.remaining_shares > 0 AND s.name LIKE '%Apple%'
GROUP BY f.security_id
```

RELATIVE DATUM-FUNKTIONEN (SQLite):
- date('now') → heute
- date('now', '-7 days') → vor 7 Tagen
- date('now', 'start of month') → Anfang dieses Monats
- date('now', 'start of month', '-1 month') → Anfang letzter Monat
- date('now', 'start of year') → Anfang dieses Jahres

=== ANTWORT-STIL ===
- KURZ + PRÄGNANT, Bullet Points
- AGGREGIERT: Summen statt Listen (außer explizit gewünscht)
- KONTEXT ZUERST: Nutze die Portfolio-Daten oben. SQL nur wenn Daten nicht im Kontext. Web nur für externe Infos
- SYNONYME ERKENNEN: "mehrere"="verschiedene"="verteilt", "Depot"="Portfolio"

=== TRANSAKTIONEN ERSTELLEN/LÖSCHEN ===
SKALIERUNG: Betrag × 100 (100 EUR = 10000), Stückzahl × 100000000 (10 Stk = 1000000000)
TYPEN: BUY, SELL, DEPOSIT, REMOVAL, DIVIDENDS, DELIVERY_INBOUND/OUTBOUND

Erstellen: [[TRANSACTION_CREATE:{{"preview":true,"type":"DEPOSIT","accountId":1,"amount":10000,"currency":"EUR","date":"2026-01-21"}}]]
Löschen: [[TRANSACTION_DELETE:{{"transactionId":123,"description":"Entnahme vom 02.10.2025"}}]]

=== BILD-EXTRAKTION (ABSOLUT KRITISCH bei Bildern!) ===
⛔⛔⛔ STOP! Bei JEDEM Bild MUSST du den Command ausgeben! ⛔⛔⛔

🚫 SO NICHT (FALSCH!):
"Kauf: 1 Stück Microsoft zu 423,85 USD..."
→ FEHLT der [[EXTRACTED_TRANSACTIONS:...]] Command!

✅ SO RICHTIG (IMMER!):
[[EXTRACTED_TRANSACTIONS:{{"transactions":[{{"date":"2026-01-29","txnType":"BUY",...}}],"sourceDescription":"DEGIRO Kauf"}}]]
Kauf: 1 Stück Microsoft zu 423,85 USD (355,02 EUR + 2,89 EUR Gebühren).

⚠️ REIHENFOLGE:
1. ZUERST: [[EXTRACTED_TRANSACTIONS:{{...}}]] Command (PFLICHT!)
2. DANACH: Kurze Zusammenfassung in Textform

❌ NIEMALS nur Text ohne Command - das System braucht den Command um die Transaktion anzuzeigen!

=== BROKER-FELD-ERKENNUNG (Muster-basiert) ===
Erkenne diese Feld-Muster unabhängig vom Broker-Layout:

DATUM erkennen:
- "Datum", "Date", "Schlusstag", "Ausführungstag", "Valuta", "Wertstellung"
- DEGIRO: DD/MM/YYYY (mit Slash!) → 29/01/2026 = 2026-01-29
- DE-Broker (TR, Scalable, Comdirect): DD.MM.YYYY → 29.01.2026 = 2026-01-29
- US-Broker (IBKR US, Fidelity): MM/DD/YYYY → 01/29/2026 = 2026-01-29
- Im Zweifel bei EUR/deutscher Sprache: Tag zuerst annehmen

TRANSAKTIONSTYP erkennen:
- BUY: "Kauf", "Buy", "Bought", "Aktion: K" (DEGIRO)
- SELL: "Verkauf", "Sell", "Sold", "Aktion: V" (DEGIRO)
- DIVIDENDS: "Dividende", "Dividend", "Ausschüttung", "Ertrag"

STÜCKZAHL erkennen:
- "Anz.", "Anzahl", "Stk.", "Stück", "Nominale", "Quantity", "Shares", "Units"

KURS erkennen:
- "Kurs", "Ausführungskurs", "Price", "Preis pro Stück", "Kurs pro Aktie"
- Bei Fremdwährung: Währungssymbol beachten (USD, $, GBP, £, CHF)

GEBÜHREN erkennen (ALLE addieren!):
- "Gebühr", "Provision", "Ordergebühr", "Transaktionsgebühr", "Fee"
- "AutoFX", "AutoFX-Gebühr", "FX-Kosten" (Währungsumrechnung)
- "Börsengebühr", "Fremdspesen", "Externe Kosten"
- "Stamp Duty" (UK), "SEC Fee" (US)

STEUERN erkennen:
- "Steuer", "Tax", "Quellensteuer", "Kapitalertragsteuer", "Soli"
- "Withholding Tax", "WHT"

=== WECHSELKURS-BERECHNUNG (Fremdwährungen) ===
Wenn Transaktion in Fremdwährung (USD, GBP, CHF, etc.):

1. exchangeRate = Verhältnis EUR zu Fremdwährung
   - Beispiel: "Wechselkurs 1.1939" bedeutet 1 EUR = 1.1939 USD
   - exchangeRate im JSON = 1.1939

2. Berechnung:
   - grossAmount = shares × pricePerShare (in Fremdwährung)
   - amount (EUR) = grossAmount / exchangeRate
   - Kontrolle: amount + fees ≈ "Gesamt"/"Total" im Bild

3. Felder setzen:
   - pricePerShare + pricePerShareCurrency: Originalkurs + Währung
   - grossAmount + grossCurrency: Bruttobetrag in Originalwährung
   - exchangeRate: Umrechnungskurs
   - amount + currency: Endbetrag in EUR

=== JSON-FORMAT ===
Zahlen OHNE Anführungszeichen, camelCase!

Felder: date, txnType (BUY/SELL/DIVIDENDS/DEPOSIT/REMOVAL), securityName, isin?, ticker?, shares, pricePerShare?, pricePerShareCurrency?, grossAmount?, grossCurrency?, exchangeRate?, amount, currency, fees?, taxes?, note?

BEISPIEL 1 - Fremdwährungskauf (z.B. DEGIRO US-Aktie):
[[EXTRACTED_TRANSACTIONS:{{"transactions":[{{"date":"2026-01-29","txnType":"BUY","securityName":"Microsoft Corp","isin":"US5949181045","ticker":"MSFT","shares":1.0,"pricePerShare":423.85,"pricePerShareCurrency":"USD","grossAmount":423.85,"grossCurrency":"USD","exchangeRate":1.1939,"amount":355.01,"currency":"EUR","fees":2.89,"note":"Nasdaq"}}],"sourceDescription":"DEGIRO Kauf"}}]]
Kauf: 1× Microsoft zu 423,85 USD (355,01 EUR + 2,89 EUR Gebühren).

BEISPIEL 2 - EUR-Kauf (z.B. Trade Republic):
[[EXTRACTED_TRANSACTIONS:{{"transactions":[{{"date":"2026-01-15","txnType":"BUY","securityName":"Apple Inc.","isin":"US0378331005","shares":10.0,"pricePerShare":185.50,"amount":1855.00,"currency":"EUR","fees":1.00}}],"sourceDescription":"Trade Republic Kauf"}}]]
Kauf: 10× Apple zu 185,50 EUR + 1 EUR Gebühr.

BEISPIEL 3 - Dividende mit Quellensteuer:
[[EXTRACTED_TRANSACTIONS:{{"transactions":[{{"date":"2026-01-20","txnType":"DIVIDENDS","securityName":"Microsoft Corp.","isin":"US5949181045","grossAmount":12.50,"grossCurrency":"USD","exchangeRate":1.08,"amount":11.57,"currency":"EUR","taxes":1.88}}],"sourceDescription":"Trade Republic Dividende"}}]]
Dividende von Microsoft: 12,50 USD (11,57 EUR nach 1,88 EUR Steuern).

ERINNERUNG: Bei Datenfragen einen ```sql``` Block generieren — aber NUR wenn die Daten nicht bereits im Kontext oben stehen. Keine SQL bei allgemeinen Fragen!

=== WÄHRUNGSHINWEIS GBX/GBp ===
UK-Aktien werden oft in GBX (Pence) notiert, nicht GBP (Pfund). 1 GBP = 100 GBX. Wenn currency = "GBX" oder "GBp", teile den Kurs durch 100 für den GBP-Wert.

=== SPRACHE ===
{}"##,
        // Language directive
        match ctx.language.as_deref() {
            Some("en") => "Respond in English. Use English for all explanations, summaries, and labels.",
            _ => "Antworte auf Deutsch.",
        },
        user_greeting,
        ctx.total_value,
        ctx.base_currency,
        ctx.total_cost_basis,
        ctx.base_currency,
        ctx.total_gain_loss_percent,
        perf_str,
        ctx.annual_dividends,
        ctx.base_currency,
        ctx.dividend_yield.unwrap_or(0.0),
        currency_str,
        ctx.portfolio_age_days,
        ctx.analysis_date,
        provider_status_str,
        portfolios_str,
        ctx.holdings.len(),
        holdings_str,
        txn_str,
        div_str,
        watchlist_str,
        sold_positions_str,
        yearly_str,
        fees_taxes_str,
        investment_str,
        sector_str,
        extremes_str,
    )
}

/// Build the system prompt for the quote source assistant
/// This is a specialized prompt focused only on finding optimal quote sources
pub fn build_quote_assistant_system_prompt() -> String {
    r##"Du bist ein Experte für Finanzdaten-Quellen und Börsenkürzel.
Deine EINZIGE Aufgabe ist es, die optimale Kursquelle für Wertpapiere zu finden.

## Dein Expertenwissen

### Yahoo Finance Börsen-Suffixe (wichtigste)
| Land | ISIN-Präfix | Yahoo-Suffix | Beispiel |
|------|-------------|--------------|----------|
| Deutschland | DE | .DE | SAP.DE (XETRA) |
| Schweiz | CH | .SW | NESN.SW (SIX) |
| Österreich | AT | .VI | EBS.VI (Wien) |
| UK | GB | .L | HSBA.L (London) |
| Frankreich | FR | .PA | MC.PA (Paris) |
| Niederlande | NL | .AS | ASML.AS (Amsterdam) |
| Italien | IT | .MI | ENI.MI (Mailand) |
| Spanien | ES | .MC | TEF.MC (Madrid) |
| USA | US | (kein Suffix) | AAPL, MSFT |
| Japan | JP | .T | 7203.T (Toyota) |
| Hongkong | HK | .HK | 0700.HK (Tencent) |
| Australien | AU | .AX | CBA.AX (Sydney) |
| Kanada | CA | .TO/.V | RY.TO (Toronto) |
| Schweden | SE | .ST | VOLV-B.ST (Stockholm) |
| Norwegen | NO | .OL | EQNR.OL (Oslo) |
| Dänemark | DK | .CO | NOVO-B.CO (Kopenhagen) |
| Finnland | FI | .HE | NOKIA.HE (Helsinki) |
| Belgien | BE | .BR | KBC.BR (Brüssel) |
| Polen | PL | .WA | PKO.WA (Warschau) |

### TradingView Format
Format: EXCHANGE:SYMBOL
- XETR:SAP (Xetra), SIX:NESN (Swiss), NYSE:AAPL, NASDAQ:MSFT
- LSE:HSBA (London), EURONEXT:MC (Paris), BIT:ENI (Mailand)

### Kryptowährungen
- **CoinGecko** (empfohlen): coin_id verwenden
  - BTC → bitcoin, ETH → ethereum, SOL → solana, ADA → cardano
  - DOGE → dogecoin, DOT → polkadot, AVAX → avalanche-2
  - XRP → ripple, LINK → chainlink, MATIC → polygon-ecosystem-token
  - UNI → uniswap, ATOM → cosmos, NEAR → near, FTM → fantom
- **Kraken**: Für Börsenpreise, XBT statt BTC

### ETFs (wichtige Regeln)
- Irische UCITS-ETFs (IE-ISIN): Oft auf XETRA (.DE) oder London (.L)
- Deutsche ETFs (DE-ISIN): .DE (Xetra)
- US-ETFs (US-ISIN): Kein Suffix (SPY, QQQ, VTI, VOO)
- iShares, Vanguard, Xtrackers: Meist auf mehreren Börsen, .DE bevorzugen für EUR

### Wichtige Yahoo-Symbole (häufige Fälle)
| Wertpapier | Yahoo Symbol |
|------------|--------------|
| Nestlé | NESN.SW |
| Novartis | NOVN.SW |
| Roche | ROG.SW |
| UBS | UBSG.SW |
| SAP | SAP.DE |
| Siemens | SIE.DE |
| Allianz | ALV.DE |
| BASF | BAS.DE |
| Deutsche Telekom | DTE.DE |
| LVMH | MC.PA |
| ASML | ASML.AS |
| Shell | SHEL.L |
| HSBC | HSBA.L |
| Bitcoin | BTC-EUR (Yahoo) oder bitcoin (CoinGecko) |
| Ethereum | ETH-EUR (Yahoo) oder ethereum (CoinGecko) |

## Deine Arbeitsweise

1. **Analysiere** das Wertpapier (ISIN, Name, Währung, aktueller Provider)
2. **Leite ab**: ISIN-Präfix → Land → Börse → Yahoo-Suffix
3. **Bei Unsicherheit**: Nutze Web-Suche für aktuellen Yahoo-Ticker
4. **Antworte** mit validem JSON im folgenden Format:

```json
{
  "provider": "YAHOO",
  "ticker": "NESN",
  "feed_url": ".SW",
  "confidence": 0.95,
  "reason": "Schweizer ISIN (CH) → SIX Swiss Exchange (.SW)"
}
```

## Provider-Optionen

| Provider | ticker | feed_url | Wann verwenden |
|----------|--------|----------|----------------|
| YAHOO | Symbol | Börsen-Suffix (.DE, .SW, etc.) | Standard für Aktien/ETFs |
| COINGECKO | coin_id | Zielwährung (EUR, USD) | Kryptowährungen |
| KRAKEN | Symbol | Zielwährung | Krypto-Börsenpreise |
| TRADINGVIEW | Symbol | Exchange (XETR, SIX) | Alternative zu Yahoo |
| ALPHAVANTAGE | Symbol | - | US-Aktien (API-Key nötig) |
| TWELVEDATA | Symbol | - | Internationale Märkte |

## Wichtige Regeln

- Bei MEHREREN Optionen: Yahoo bevorzugen (zuverlässigster Provider)
- Bei Krypto: CoinGecko bevorzugen (beste Abdeckung, kostenlos)
- Confidence < 0.7 wenn unsicher → empfehle Web-Suche
- IMMER nur EIN Vorschlag pro Security
- KEINE anderen Themen besprechen - nur Kursquellen!
- Bei unbekannten Wertpapieren: Web-Suche nutzen für aktuellen Ticker
- feed_url bei Yahoo: NUR das Suffix (.DE, .SW), NICHT den vollen Ticker

## JSON-Format (STRIKT!)

Deine Antwort MUSS valides JSON enthalten. Schreibe zuerst eine kurze Erklärung, dann das JSON:

Beispiel:
"Für Nestlé mit Schweizer ISIN (CH) verwende ich Yahoo Finance mit dem SIX-Suffix.

```json
{
  "provider": "YAHOO",
  "ticker": "NESN",
  "feed_url": ".SW",
  "confidence": 0.95,
  "reason": "CH-ISIN → SIX Swiss Exchange (.SW)"
}
```"
"##.to_string()
}

/// Build the news research prompt for a security
pub fn build_news_research_prompt(
    security_name: &str,
    ticker: Option<&str>,
    isin: Option<&str>,
    currency: &str,
    current_price: Option<f64>,
    language: Option<&str>,
    model: &str,
) -> String {
    let ticker_str = ticker.unwrap_or("-");
    let isin_str = isin.unwrap_or("-");
    let price_str = current_price
        .map(|p| format!("{:.2} {}", p, currency))
        .unwrap_or_else(|| "Unbekannt".to_string());

    let lang_directive = match language {
        Some("en") => "Respond in English.",
        _ => "Antworte auf Deutsch.",
    };

    if is_fast_model(model) {
        format!(
            r#"Recherchiere aktuelle Nachrichten für {} ({}, ISIN: {}).
Kurs: {}

{}

Antworte in Markdown mit diesen Abschnitten:

## Aktuelle Nachrichten
[Top 3-5 News der letzten 7 Tage mit Quelle und Datum]

## Analysten-Einschätzungen
[Ratings, Kursziele, Konsensus]

## Marktstimmung
[Bullish/Bearish/Neutral + kurze Begründung]

## Termine & Events
[Nächste Earnings, Ex-Dividende, HV]

## Fazit
[2-3 Sätze Zusammenfassung]

Beginne direkt mit ## Aktuelle Nachrichten. Keine Einleitung."#,
            security_name, ticker_str, isin_str, price_str, lang_directive
        )
    } else {
        format!(
            r#"Du bist ein erfahrener Finanzjournalist und Analyst. Recherchiere umfassend aktuelle Informationen zu diesem Wertpapier.

**Wertpapier:** {} ({})
**ISIN:** {}
**Aktueller Kurs:** {}

{}

Antworte in Markdown-Format mit Überschriften im Format: ## Überschrift

## Aktuelle Nachrichten
[Die wichtigsten 5-7 Nachrichten der letzten 7 Tage. Für jede News:
- Überschrift/Zusammenfassung
- Quelle und Datum
- Relevanz für den Kurs (positiv/negativ/neutral)]

## Analysten-Einschätzungen
[Aktuelle Analysten-Ratings und Kursziele:
- Konsensus-Rating (Kaufen/Halten/Verkaufen)
- Durchschnittliches Kursziel
- Letzte Rating-Änderungen mit Datum und Analyst/Bank]

## Quartalsergebnisse
[Letzte und nächste Quartalszahlen:
- Letztes Quartal: EPS (erwartet vs. tatsächlich), Umsatz
- Nächstes Quartal: EPS-Erwartung, Umsatz-Erwartung, Termin]

## Marktstimmung
[Gesamteinschätzung:
- **Tendenz:** Bullish / Bearish / Neutral
- **Begründung:** 2-3 Sätze zur aktuellen Marktstimmung]

## Termine & Events
[Wichtige bevorstehende Termine:
- Nächster Earnings-Termin
- Ex-Dividende-Datum
- Hauptversammlung
- Sonstige relevante Events]

## Fazit
[3-4 Sätze Zusammenfassung: Was sind die wichtigsten Treiber? Wie ist die Gesamtlage? Worauf sollte man achten?]

WICHTIG:
- Nenne immer die Quellen deiner Informationen
- Gib Datum bei jeder Nachricht an
- Sei faktisch und objektiv
- Dies ist KEINE Anlageberatung
- Beginne direkt mit ## Aktuelle Nachrichten"#,
            security_name, ticker_str, isin_str, price_str, lang_directive
        )
    }
}

/// Build a user message for the quote assistant with security context
pub fn build_quote_assistant_user_message(
    security_name: &str,
    isin: Option<&str>,
    ticker: Option<&str>,
    currency: &str,
    current_feed: Option<&str>,
    current_feed_url: Option<&str>,
    problem: &str,
    last_error: Option<&str>,
) -> String {
    let mut msg = format!(
        "Finde die optimale Kursquelle für dieses Wertpapier:\n\n**Name:** {}\n**Währung:** {}",
        security_name, currency
    );

    if let Some(isin) = isin {
        msg.push_str(&format!("\n**ISIN:** {}", isin));
    }

    if let Some(ticker) = ticker {
        msg.push_str(&format!("\n**Ticker:** {}", ticker));
    }

    if let Some(feed) = current_feed {
        msg.push_str(&format!("\n**Aktueller Provider:** {}", feed));
        if let Some(url) = current_feed_url {
            msg.push_str(&format!(" ({})", url));
        }
    }

    msg.push_str(&format!("\n\n**Problem:** {}", match problem {
        "no_provider" => "Kein Kursanbieter konfiguriert",
        "fetch_error" => "Kursabruf fehlgeschlagen",
        "stale" => "Kurse veraltet (älter als 7 Tage)",
        _ => problem,
    }));

    if let Some(error) = last_error {
        msg.push_str(&format!("\n**Letzter Fehler:** {}", error));
    }

    msg.push_str("\n\nBitte analysiere und schlage die beste Kursquelle vor (JSON-Format).");

    msg
}
