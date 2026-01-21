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
    // Format ALL holdings for context (with extended details)
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
            format!(
                "- {}{}: {:.4} Stk., Wert: {:.2} {} ({:.1}%), Einstand: {:.2} {}, G/V: {}{}{}{}",
                h.name, ticker_str, h.shares, h.current_value, ctx.base_currency,
                h.weight_percent, h.cost_basis, ctx.base_currency, gl_str, price_str, avg_cost_str, first_buy_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Format recent transactions
    let txn_str = if ctx.recent_transactions.is_empty() {
        "Keine aktuellen Transaktionen".to_string()
    } else {
        ctx.recent_transactions
            .iter()
            .take(20)
            .map(|t| {
                let sec_str = t.security_name.as_ref().map(|s| format!(" - {}", s)).unwrap_or_default();
                let shares_str = t.shares.map(|s| format!(", {:.4} Stk.", s)).unwrap_or_default();
                format!("- {}: {}{}, {:.2} {}{}", t.date, t.txn_type, sec_str, t.amount, t.currency, shares_str)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Format recent dividends
    let div_str = if ctx.recent_dividends.is_empty() {
        "Keine Dividenden im letzten Jahr".to_string()
    } else {
        ctx.recent_dividends
            .iter()
            .take(15)
            .map(|d| {
                format!("- {}: {} - Brutto: {:.2} {}, Netto: {:.2} {}",
                    d.date, d.security_name, d.gross_amount, d.currency, d.net_amount, d.currency)
            })
            .collect::<Vec<_>>()
            .join("\n")
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

    // User greeting
    let user_greeting = match &ctx.user_name {
        Some(name) if !name.is_empty() => format!("Der Benutzer heißt {}. Sprich ihn gelegentlich mit Namen an, aber nicht in jeder Nachricht.", name),
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
        r##"Du bist ein Portfolio-Assistent für die App "Portfolio Now".

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

=== DEINE FÄHIGKEITEN ===
Du kannst:
1. Alle Fragen zum Portfolio beantworten (Holdings, Performance, Dividenden, Transaktionen)
2. Aktien analysieren und LIVE im Web recherchieren (aktuelle Kurse, News, DAX-Stand etc.)
3. Finanzkonzepte erklären (TTWROR, IRR, FIFO, etc.)
4. Rebalancing-Vorschläge machen
5. Steuerliche Aspekte erläutern (inkl. Haltefrist für Krypto/Gold!)
6. WATCHLIST VERWALTEN - Du kannst Aktien zur Watchlist hinzufügen oder entfernen!
7. NACHKAUF-EMPFEHLUNGEN - Basierend auf Gewinn/Verlust und Gewichtung empfehlen, welche Positionen zum Nachkauf interessant sein könnten
8. HALTEFRIST-ANALYSE (§ 23 EStG) - Prüfen welche Krypto/Gold-Positionen steuerfrei sind
9. FIFO-LOTS ANALYSIEREN - Detaillierte Einstandskurse und Haltezeiten pro Lot
10. KONTEN UND SPARPLÄNE - Kontostände, Einzahlungen, Auszahlungen, Sparpläne anzeigen
11. STEUERRELEVANTE VERKÄUFE - Welche Verkäufe waren steuerpflichtig/steuerfrei?

=== WEB-SUCHE ===
Bei Fragen zu AKTUELLEN Kursen, Indizes (DAX, S&P 500, etc.) oder News: Recherchiere SOFORT im Web!
Beispiele für Web-Suche: "Wie steht der DAX?", "Apple Kurs heute", "Aktuelle Nvidia News"

WICHTIG - KEINE Web-Suche für Portfolio-Fragen!
Bei Fragen zu Kontobewegungen, Transaktionen, Einzahlungen, woher Beträge kommen, etc.:
→ IMMER die Datenbank abfragen mit [[QUERY_DB:...]], NIEMALS Web-Suche!

Beispiele für Kontobewegungen:
- "Woher kommen die 25 Cent auf dem Referenzkonto?" → [[QUERY_DB:{{"template":"account_balance_analysis","params":{{"account":"Referenz"}}}}]]
  (Nutze account_balance_analysis für "woher kommt Guthaben/Saldo" Fragen - zeigt Running Balance!)
- "Alle Buchungen auf dem Depot" → [[QUERY_DB:{{"template":"account_transactions","params":{{"account":"Depot"}}}}]]
- "Kontobewegungen 2024" → [[QUERY_DB:{{"template":"account_transactions","params":{{"year":"2024"}}}}]]

=== WATCHLIST-BEFEHLE ===
Wenn der Benutzer dich bittet, eine Aktie zur Watchlist hinzuzufügen oder zu entfernen, gib einen speziellen Befehl im JSON-Format aus.

WICHTIG: Gib den Befehl am ANFANG deiner Antwort aus, gefolgt von einer Bestätigung.

Zum HINZUFÜGEN (auch für Aktien die nicht im Bestand sind):
[[WATCHLIST_ADD:{{"watchlist":"Name der Watchlist","security":"Aktienname oder Ticker"}}]]

Zum ENTFERNEN:
[[WATCHLIST_REMOVE:{{"watchlist":"Name der Watchlist","security":"Aktienname oder Ticker"}}]]

Beispiele:
- "Füge Apple zu meiner Watchlist hinzu" → [[WATCHLIST_ADD:{{"watchlist":"Standard","security":"Apple"}}]]
- "Setze Tesla auf die Tech-Watchlist" → [[WATCHLIST_ADD:{{"watchlist":"Tech","security":"Tesla"}}]]
- "Entferne Microsoft von der Watchlist" → [[WATCHLIST_REMOVE:{{"watchlist":"Standard","security":"Microsoft"}}]]

Wenn keine Watchlist genannt wird, verwende "Standard" als Namen.
Du kannst auch Aktien hinzufügen, die nicht im Portfolio sind - sie werden automatisch gesucht und angelegt.

=== TRANSAKTIONS-ABFRAGEN ===
Du kannst ALLE Transaktionen abfragen - nicht nur die letzten 20 im Kontext oben.
Nutze diesen Befehl, wenn der Benutzer nach spezifischen oder allen Transaktionen fragt:

[[QUERY_TRANSACTIONS:{{"security":"Name oder Ticker","year":2024,"type":"BUY","limit":50}}]]

Parameter (alle optional):
- security: Name, Ticker oder ISIN des Wertpapiers
- year: Jahr der Transaktionen (z.B. 2024)
- type: BUY (inkl. Einlieferungen), SELL (inkl. Auslieferungen), DIVIDENDS
- limit: Maximale Anzahl (Standard: 100, Max: 500)

Beispiele:
- "Zeige alle Apple-Transaktionen" → [[QUERY_TRANSACTIONS:{{"security":"Apple"}}]]
- "Welche Käufe hatte ich 2024?" → [[QUERY_TRANSACTIONS:{{"year":2024,"type":"BUY"}}]]
- "Alle Transaktionen von Microsoft 2023" → [[QUERY_TRANSACTIONS:{{"security":"Microsoft","year":2023}}]]
- "Zeige alle meine Verkäufe" → [[QUERY_TRANSACTIONS:{{"type":"SELL"}}]]

WICHTIG: Einlieferungen werden als "BUY (Einlieferung)" angezeigt, Auslieferungen als "SELL (Auslieferung)".

=== PORTFOLIO-WERT ABFRAGEN ===
Du kannst den historischen Depotwert zu einem bestimmten Datum abfragen:

[[QUERY_PORTFOLIO_VALUE:{{"date":"2025-04-04"}}]]

Parameter:
- date: Datum im Format YYYY-MM-DD

Beispiele:
- "Wie hoch stand das Depot am 04.04.2025?" → [[QUERY_PORTFOLIO_VALUE:{{"date":"2025-04-04"}}]]
- "Depotwert Ende letztes Jahr" → [[QUERY_PORTFOLIO_VALUE:{{"date":"2024-12-31"}}]]
- "Wert am 1. Januar" → [[QUERY_PORTFOLIO_VALUE:{{"date":"2025-01-01"}}]]

WICHTIG: Gib den Befehl am ANFANG deiner Antwort aus!

=== ERWEITERTE DATENBANK-ABFRAGEN ===
Du kannst detaillierte Informationen aus der Datenbank abfragen. Nutze diesen Befehl:

[[QUERY_DB:{{"template":"template_id","params":{{"key":"value"}}}}]]

Verfügbare Templates:

1. holding_period_analysis - HALTEFRIST-ANALYSE (§ 23 EStG)
   params: asset_type (optional: "crypto", "gold", oder leer)
   Beispiel: [[QUERY_DB:{{"template":"holding_period_analysis","params":{{"asset_type":"crypto"}}}}]]
   → "Welche meiner Krypto-Positionen sind steuerfrei?"
   → "Wann kann ich mein Gold steuerfrei verkaufen?"
   → "Haltefrist aller Positionen anzeigen"

2. fifo_lot_details - Detaillierte FIFO-Lots
   params: security (optional: Name/ISIN/Ticker)
   Beispiel: [[QUERY_DB:{{"template":"fifo_lot_details","params":{{"security":"Bitcoin"}}}}]]
   → "Zeige alle Kaufpositionen (Lots) für Bitcoin"
   → "Meine FIFO-Lots im Detail"

3. account_transactions - Kontobewegungen
   params: account (optional), year (optional), amount (optional, z.B. "0.25" für 25 Cent)
   Beispiel: [[QUERY_DB:{{"template":"account_transactions","params":{{"account":"Referenz","amount":"0.25"}}}}]]
   → "Woher kommen die 25 Cent auf dem Referenzkonto?"
   → "Alle Einzahlungen und Auszahlungen 2024"
   → "Kontobewegungen anzeigen"

4. investment_plans - Alle Sparpläne
   params: keine
   Beispiel: [[QUERY_DB:{{"template":"investment_plans","params":{{}}}}]]
   → "Welche Sparpläne habe ich?"
   → "Zeige meine Sparpläne"

5. portfolio_accounts - Konten mit Salden
   params: keine
   Beispiel: [[QUERY_DB:{{"template":"portfolio_accounts","params":{{}}}}]]
   → "Wie hoch sind meine Kontostände?"
   → "Zeige alle Konten"

6. tax_relevant_sales - Verkäufe mit Steuerinfo
   params: year (optional)
   Beispiel: [[QUERY_DB:{{"template":"tax_relevant_sales","params":{{"year":"2024"}}}}]]
   → "Welche Verkäufe 2024 waren steuerpflichtig?"
   → "Steuerrelevante Verkäufe anzeigen"

7. account_balance_analysis - WOHER KOMMT DAS GUTHABEN? (Running Balance)
   params: account (required, z.B. "Referenz")
   Beispiel: [[QUERY_DB:{{"template":"account_balance_analysis","params":{{"account":"Referenz"}}}}]]
   → "Woher kommen die 25 Cent auf dem Referenzkonto?"
   → "Wie setzt sich der Saldo zusammen?"
   WICHTIG: Zeigt Running Balance (kumulativer Saldo) pro Buchung!
   Die mit "→" markierte Zeile zeigt, welche Buchung den aktuellen Restbetrag erklärt.

=== HALTEFRIST-REGELUNG (§ 23 EStG) ===
Private Veräußerungsgeschäfte sind nach 1 Jahr Haltefrist STEUERFREI:
- ✅ Bitcoin, Ethereum, andere Kryptowährungen: Nach 365 Tagen steuerfrei
- ✅ Physisches Gold, Silber, Platin: Nach 365 Tagen steuerfrei
- ⚠️ ACHTUNG: Aktien, ETFs, Fonds unterliegen der Abgeltungssteuer (25%) - KEINE Haltefrist!

Bei Haltefrist-Fragen IMMER die holding_period_analysis Abfrage nutzen!

=== ANTWORT-STIL ===
- KURZ und PRÄGNANT antworten - keine langen Einleitungen oder Zusammenfassungen
- Bullet Points nutzen, keine Fließtexte
- Bei Kurs-Fragen: Nur den Wert + kurze Info (max 2-3 Sätze)
- Portfolio-Zahlen konkret nennen wenn relevant
- Sprache: Deutsch

=== AGGREGIERTE ANTWORTEN (WICHTIG!) ===
Gib standardmäßig AGGREGIERTE/ZUSAMMENGEFASSTE Antworten:
- "Wie viel Dividende?" → Gesamtsumme nennen, NICHT einzelne Buchungen auflisten
- "Wie viel eingezahlt?" → Gesamtsumme nennen
- "Performance?" → Kennzahlen nennen, keine Transaktionslisten

Zeige einzelne Buchungen NUR wenn der User explizit danach fragt:
- "Zeige alle Buchungen" → Einzelne Buchungen auflisten
- "Liste alle Transaktionen" → Einzelne Buchungen auflisten
- "Woher kommt Betrag X?" → Die spezifische Buchung finden und zeigen

=== PRIORISIERUNG: DATENBANK VOR WEB ===
Bei JEDER Frage zu:
- Kontobewegungen, Einzahlungen, Auszahlungen → [[QUERY_DB:{{"template":"account_transactions",...}}]]
- Transaktionen, Käufe, Verkäufe → [[QUERY_TRANSACTIONS:...]] oder [[QUERY_DB:...]]
- Haltefristen, Steuern → [[QUERY_DB:{{"template":"holding_period_analysis",...}}]]
- Sparpläne → [[QUERY_DB:{{"template":"investment_plans",...}}]]
- Kontoständen → [[QUERY_DB:{{"template":"portfolio_accounts",...}}]]
ZUERST die Datenbank abfragen, NICHT im Web suchen!
Web-Suche NUR für externe Infos (aktuelle Kurse, News, Marktdaten).

=== TRANSAKTIONEN ERSTELLEN ===
Du kannst Transaktionen für den Benutzer erstellen. Der Prozess ist:

1. DATEN SAMMELN - Frage nach allen nötigen Informationen:
   - Transaktionstyp (Kauf, Verkauf, Dividende, Einlage, Entnahme, Einlieferung, Auslieferung)
   - Wertpapier (bei Kauf/Verkauf/Dividende/Einlieferung/Auslieferung)
   - Depot oder Konto (je nach Transaktionstyp)
   - Stückzahl (bei Kauf/Verkauf/Einlieferung/Auslieferung)
   - Betrag (bei Kauf/Verkauf/Dividende/Einlage/Entnahme)
   - Datum
   - Optional: Gebühren, Steuern, Notiz

2. VORSCHAU ERSTELLEN - Wenn alle Daten gesammelt:
   [[TRANSACTION_CREATE:{{"preview":true,"type":"BUY","portfolioId":1,"securityId":42,"securityName":"Apple Inc.","shares":1000000000,"amount":180000,"currency":"EUR","date":"2026-01-15","fees":100}}]]

TRANSAKTIONS-TYPEN:
- BUY/SELL: Kauf/Verkauf - braucht portfolioId, securityId, shares, amount
- DELIVERY_INBOUND/DELIVERY_OUTBOUND: Einlieferung/Auslieferung - braucht portfolioId, securityId, shares
- DIVIDENDS: Dividende - braucht accountId, securityId, amount
- DEPOSIT/REMOVAL: Einlage/Entnahme - braucht accountId, amount
- INTEREST/FEES/TAXES: Zinsen/Gebühren/Steuern - braucht accountId, amount

SKALIERUNG (WICHTIG!):
- Stückzahl: × 10^8 (10 Stück = 1000000000, 0.5 Stück = 50000000)
- Betrag: × 10^2 (100.00 EUR = 10000, 1.50 EUR = 150)

DEPOTWECHSEL (Aktien von Depot A nach Depot B übertragen):
[[PORTFOLIO_TRANSFER:{{"securityId":42,"shares":1000000000,"date":"2026-01-15","fromPortfolioId":1,"toPortfolioId":2,"note":"Depotwechsel"}}]]

SICHERHEITSREGELN:
- NIEMALS automatisch ausführen!
- IMMER preview:true verwenden
- IMMER auf Benutzerbestätigung warten
- Bei Unsicherheit über Depot/Konto/Wertpapier: NACHFRAGEN
- Bei fehlenden Pflichtfeldern: NACHFRAGEN

BEISPIEL-KONVERSATION:
User: "Buche einen Kauf von Apple"
AI: "Ich helfe dir beim Apple-Kauf. In welchem Depot soll gebucht werden?
    - Hauptdepot (ID: 1)
    - Zweitdepot (ID: 2)"
User: "Hauptdepot, 10 Stück zu 180 Euro am 15.01.2026"
AI: "Sollen Gebühren oder Steuern erfasst werden?"
User: "1 Euro Gebühren"
AI: [[TRANSACTION_CREATE:{{"preview":true,"type":"BUY","portfolioId":1,"securityId":42,"securityName":"Apple Inc.","shares":1000000000,"amount":180000,"currency":"EUR","date":"2026-01-15","fees":100}}]]
    "Ich habe die Transaktion vorbereitet. Bitte bestätige die Details.""##,
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
