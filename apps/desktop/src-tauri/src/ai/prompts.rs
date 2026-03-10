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
    build_chat_system_prompt_with_options(ctx, false)
}

/// Build the system prompt for portfolio chat with options
/// `has_images` controls whether the broker/image extraction section is included
pub fn build_chat_system_prompt_with_options(ctx: &PortfolioInsightsContext, has_images: bool) -> String {
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

    // Format holdings for context — limit to top 20 by value, summarize rest
    let max_holdings = 20;
    let holdings_str = {
        let mut sorted: Vec<_> = ctx.holdings.iter().collect();
        sorted.sort_by(|a, b| b.current_value.partial_cmp(&a.current_value).unwrap_or(std::cmp::Ordering::Equal));

        let shown: Vec<String> = sorted.iter().take(max_holdings).map(|h| {
            let gl_str = h.gain_loss_percent.map(|g| format!("{:+.1}%", g)).unwrap_or_else(|| "-".to_string());
            let ticker_str = h.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
            format!(
                "- {}{}: {:.4} Stk., Wert: {:.2} {}, G/V: {}",
                h.name, ticker_str, h.shares, h.current_value, ctx.base_currency, gl_str
            )
        }).collect();

        if ctx.holdings.len() > max_holdings {
            let rest_value: f64 = sorted.iter().skip(max_holdings).map(|h| h.current_value).sum();
            let rest_count = ctx.holdings.len() - max_holdings;
            format!("{}\n(+ {} weitere Positionen, Wert: {:.2} {} — für Details SQL verwenden)",
                shown.join("\n"), rest_count, rest_value, ctx.base_currency)
        } else {
            shown.join("\n")
        }
    };

    // Format recent transactions (with truncation hint)
    let txn_limit = 10;
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
    let div_limit = 10;
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

    // Format watchlist — grouped by watchlist name, including empty watchlists
    let watchlist_str = if ctx.watchlist_names.is_empty() {
        "Keine Watchlists vorhanden".to_string()
    } else {
        ctx.watchlist_names.iter().map(|wl_name| {
            let items: Vec<String> = ctx.watchlist.iter()
                .filter(|w| w.watchlist_name == *wl_name)
                .map(|w| {
                    let ticker_str = w.ticker.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default();
                    let price_str = w.current_price.map(|p| format!(", Kurs: {:.2} {}", p, w.currency)).unwrap_or_default();
                    format!("  - {}{}{}", w.name, ticker_str, price_str)
                })
                .collect();
            if items.is_empty() {
                format!("Watchlist \"{}\": (leer)", wl_name)
            } else {
                format!("Watchlist \"{}\":\n{}", wl_name, items.join("\n"))
            }
        }).collect::<Vec<_>>().join("\n")
    };

    // Format sold positions — limit to 10 most recent
    let max_sold = 10;
    let sold_positions_str = if ctx.sold_positions.is_empty() {
        "Keine".to_string()
    } else {
        let items: Vec<String> = ctx.sold_positions.iter().take(max_sold).map(|s| {
            let gain_str = if s.realized_gain_loss >= 0.0 { format!("+{:.2}", s.realized_gain_loss) } else { format!("{:.2}", s.realized_gain_loss) };
            format!("- {}: {} {}, {}", s.name, gain_str, ctx.base_currency, s.last_transaction_date)
        }).collect();
        if ctx.sold_positions.len() > max_sold {
            format!("{}\n(+ {} weitere — SQL für Details)", items.join("\n"), ctx.sold_positions.len() - max_sold)
        } else {
            items.join("\n")
        }
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
                    .take(3)
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
        Some(e) => {
            let y = chrono::Utc::now().format("%Y");
            format!(
                "ATH: {:.2} {} ({}), ATL: {:.2} {} ({}), {}-Hoch: {:.2} ({}), {}-Tief: {:.2} ({})",
                e.all_time_high, ctx.base_currency, e.all_time_high_date,
                e.all_time_low, ctx.base_currency, e.all_time_low_date,
                y, e.year_high, e.year_high_date,
                y, e.year_low, e.year_low_date,
            )
        }
        None => "Keine".to_string(),
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

    // Build image extraction section only when images are attached
    let image_section = if has_images {
        r##"
=== BILD-EXTRAKTION (KRITISCH bei Bildern!) ===
Bei JEDEM Bild MUSST du den Command ausgeben!

REIHENFOLGE: 1. ZUERST [[EXTRACTED_TRANSACTIONS:{{...}}]] Command, 2. DANACH Zusammenfassung

[[EXTRACTED_TRANSACTIONS:{{"transactions":[{{"date":"2026-01-29","txnType":"BUY","securityName":"Microsoft Corp","isin":"US5949181045","shares":1.0,"pricePerShare":423.85,"pricePerShareCurrency":"USD","grossAmount":423.85,"grossCurrency":"USD","exchangeRate":1.1939,"amount":355.01,"currency":"EUR","fees":2.89}}],"sourceDescription":"DEGIRO Kauf"}}]]

Felder: date, txnType (BUY/SELL/DIVIDENDS/DEPOSIT/REMOVAL), securityName, isin?, ticker?, shares, pricePerShare?, pricePerShareCurrency?, grossAmount?, grossCurrency?, exchangeRate?, amount, currency, fees?, taxes?, note?

DATUM: DEGIRO DD/MM/YYYY, DE-Broker DD.MM.YYYY, US-Broker MM/DD/YYYY
GEBÜHREN: Alle addieren (Provision+AutoFX+Börsengebühr+Fremdspesen)
WECHSELKURS: exchangeRate = EUR/Foreign (1.1939 = 1 EUR = 1.1939 USD). amount(EUR) = grossAmount / exchangeRate
GBX/GBp: Durch 100 teilen für GBP-Wert"##.to_string()
    } else {
        String::new()
    };

    let lang_str = match ctx.language.as_deref() {
        Some("en") => "Respond in English.",
        _ => "Antworte auf Deutsch.",
    };

    format!(
        r##"Du bist ein Portfolio-Assistent für "Portfolio Now". {lang}

{user}

=== PORTFOLIO ({date}) ===
Wert: {value:.2} {cur}, Einstand: {cost:.2} {cur}, Rendite: {gl:+.1}%, {perf}
Dividenden/Jahr: {div:.2} {cur} ({div_yield:.2}%), Währungen: {currencies}, Alter: {age} Tage{provider}

=== DEPOTS ===
{portfolios}

=== HOLDINGS ({h_count} Positionen) ===
{holdings}

=== TRANSAKTIONEN (letzte) ===
{txns}

=== DIVIDENDEN (12 Mon.) ===
{divs}

=== WATCHLIST ===
{watchlist}

=== VERKAUFTE POSITIONEN ===
{sold}

=== JAHRESÜBERSICHT ===
{yearly}

=== GEBÜHREN/STEUERN ===
{fees}

=== INVESTITION ===
{invest}

=== SEKTOREN ===
{sectors}

=== EXTREMWERTE ===
{extremes}

=== BEFEHLE ===
WATCHLIST:
Wenn der User eine Aktie auf die Watchlist setzen will:
- Gibt es nur EINE Watchlist → SOFORT den Command ausgeben, NICHT fragen.
- Gibt es mehrere Watchlists → fragen auf welche.
- Neue Watchlist → nach Namen fragen, dann Command.
PFLICHT: Du MUSST den Command-Tag ausgeben, sonst passiert nichts!
Hinzufügen: [[WATCHLIST_ADD:{{"watchlist":"<Name>","security":"Apple"}}]]
Entfernen: [[WATCHLIST_REMOVE:{{"watchlist":"<Name>","security":"Apple"}}]]
Der security-Wert muss der EXAKTE Name des Wertpapiers aus dem Kontext sein (z.B. "Amazon.com Inc.").
TRANSAKTION: [[TRANSACTION_CREATE:{{"preview":true,"type":"BUY","accountId":1,"amount":10000,"currency":"EUR","date":"2026-01-21"}}]]
Skalierung: amount×100, shares×100000000

=== WERTPAPIERSUCHE ===
Wenn der User nach einem Wertpapier, einer ISIN, WKN oder einem Ticker sucht → SEARCH_SECURITY Command!
[[SEARCH_SECURITY:{{"query":"Silber"}}]]
Sucht ZUERST in der lokalen Datenbank (Name, ISIN, Ticker, WKN), DANN extern über Yahoo Finance.
Die Ergebnisse werden automatisch als Tabelle angezeigt.
WICHTIG: Verwende SEARCH_SECURITY statt SQL wenn der User nach Wertpapieren, ISINs, WKNs oder Tickern sucht!
Bei der Suche nach Rohstoffen (Gold, Silber, Öl) verwende den englischen Begriff als query (z.B. "Silver", "Gold", "Oil").

=== TECHNISCHE ANALYSE / CHART ===
Wenn der User einen Chart, technische Analyse oder Kursverlauf sehen will → SHOW_CHART Command!
Syntax: [[SHOW_CHART:{{"securityName":"Linde","timeRange":"1Y","indicators":["sma20","sma50","rsi14","macd"]}}]]
Erlaubte timeRange-Werte: 1M, 3M, 6M, 1Y, 2Y, 5Y, MAX (Standard: 1Y)
Erlaubte Indikatoren: sma20, sma50, sma200, ema20, ema50, rsi14, macd, bollinger, atr14, vwap, stochastic, obv, adx14, ichimoku
Wenn der User bestimmte Indikatoren erwähnt oder du welche empfiehlst, setze sie in indicators.
Wenn der User einen bestimmten Zeitraum nennt (z.B. "6 Monate", "letzte 2 Jahre"), setze timeRange entsprechend.
Verwende den Security-Namen EXAKT wie im Kontext oben. Gib eine kurze Zusammenfassung dazu, was im Chart zu sehen sein wird.

=== SCREENER ===
Wenn der User Aktien suchen/filtern/scannen will oder nach Kaufgelegenheiten, Ausbrüchen, Trends, Momentum fragt → SCREENER_RUN Command!
Erkläre KURZ die Strategie, dann gib den Command.

Syntax: [[SCREENER_RUN:{{"filters":[{{"indicator":"rsi","condition":"below","value":30}}],"mode":"market","indexId":"dax40"}}]]

Strategien → Filter-Kombinationen:
- "Überverkaufte Aktien" → rsi below 30
- "Überkaufte Aktien" → rsi above 70
- "Ausbruchs-Kandidaten" → bollinger_width below 5 + volume above 150 (Squeeze + Volumen-Spike)
- "Starker Aufwärtstrend" → adx above 25 + sma_50 above sma_200 (via di_plus above, vergleicht DI+>DI-)
- "Momentum-Aktien" → macd_histogram above 0 + change_5d above 3
- "Mean Reversion" → rsi below 35 + stochastic_k below 20 + sma_200 above 0
- "Trendumkehr" → change_20d below -10 + rsi below 35 (stark gefallen + überverkauft)
- "Breakout mit Volumen" → change_1d above 2 + volume above 200

Filter: rsi, price, volume, macd, macd_signal, macd_histogram, bollinger_upper/lower/width, stochastic_k/d, adx, di_plus/minus, obv, sma_20/50/200, change_1d/5d/20d
Bedingungen: above, below, crosses_above/below, between, increasing, decreasing
di_plus mit "above" vergleicht DI+ > DI- (nicht gegen value). Analog di_minus.
mode: "market" (Index scannen), "local" (eigene Wertpapiere)
Indizes: dax40, eurostoxx50, smi20, atx20, sp500, nasdaq100, dowjones30, ftse100

Wenn User unsicher ist → schlage 2-3 passende Scans vor und lass ihn wählen.

=== TA-WISSEN ===
RSI(14): <30=überverkauft(Kaufgelegenheit), >70=überkauft. Divergenz=starkes Umkehrsignal.
MACD(12,26,9): Signal-Kreuzung=Ein/Ausstieg. Histogramm=Momentum-Stärke.
Bollinger(20,2): Squeeze(Breite<5%)=Ausbruch erwartet. Oberes Band=überkauft, unteres=überverkauft.
ADX(14): >25=starker Trend, <20=seitwärts. +DI>-DI=Aufwärts, -DI>+DI=Abwärts.
SMA: Golden Cross(50>200)=bullisch, Death Cross=bärisch. Kurs>SMA200=Aufwärtstrend.
Stochastic(14,3): <20=überverkauft, >80=überkauft. %K>%D=Kaufsignal.
ATR(14): Volatilität. Stop-Loss=2×ATR unter Einstieg.
OBV: Steigende OBV+Kurs=Trendbestätigung. Divergenz=Umkehr.
Breakout-Score(0-12): Trend+Konsolidierung+Trigger+Volumen+Pullback+Rel.Stärke (je 0-2). 10-12=sehr wahrscheinlich, 7-9=wahrscheinlich, 4-6=möglich, 0-3=unwahrscheinlich.

=== SQL ===
Bei Datenfragen die NICHT im Kontext stehen → direkt ```sql``` Block ausgeben! Ohne SQL-Block passiert NICHTS.
KEIN SQL bei: Allgemeinen Fragen, Finanzwissen, oder wenn Daten bereits im Kontext stehen.
Nur SELECT auf pp_* Tabellen.

Skalierung: amount÷100, shares÷100000000, value/price÷100000000

Wichtige Spalten:
- pp_security: id, name, currency, isin, wkn, ticker, feed, is_retired
- pp_txn: id, owner_type('portfolio'|'account'), owner_id, security_id, txn_type, date, amount, currency, shares, note
- pp_txn_unit: txn_id, unit_type('FEE'|'TAX'|'GROSS_VALUE'), amount, currency, forex_amount, forex_currency
- pp_latest_price: security_id, date, value, high, low, volume
- pp_price: security_id, date, value, volume, open, high, low
- pp_fifo_lot: id, security_id, portfolio_id, purchase_date, remaining_shares, gross_amount, net_amount, currency
- pp_account: id, name, currency, is_retired
- pp_portfolio: id, name, reference_account_id

Txn-Types:
- Portfolio (owner_type='portfolio'): BUY, SELL, DELIVERY_INBOUND, DELIVERY_OUTBOUND, TRANSFER_IN, TRANSFER_OUT
- Account (owner_type='account'): DEPOSIT, REMOVAL, DIVIDENDS, INTEREST, FEES, TAXES, TAX_REFUND, BUY, SELL
- WICHTIG: Käufe = BUY + DELIVERY_INBOUND, Verkäufe = SELL + DELIVERY_OUTBOUND
- WICHTIG: Dividenden liegen in pp_txn mit owner_type='account' (NICHT 'portfolio'!)

Holdings-Berechnung:
SUM(CASE WHEN txn_type IN ('BUY','TRANSFER_IN','DELIVERY_INBOUND') THEN shares WHEN txn_type IN ('SELL','TRANSFER_OUT','DELIVERY_OUTBOUND') THEN -shares END)/100000000.0

Datum: date('now'), date('now','-7 days'), date('now','start of month','-1 month'), date('now','start of year'), strftime('%Y',date)

```sql
-- Letzte Käufe
SELECT t.date, s.name, t.shares/100000000.0 as stk, t.amount/100.0 as eur
FROM pp_txn t JOIN pp_security s ON s.id=t.security_id
WHERE t.txn_type IN ('BUY','DELIVERY_INBOUND') AND t.owner_type='portfolio'
ORDER BY t.date DESC LIMIT 15
```

```sql
-- Dividenden eines Jahres
SELECT s.name, SUM(t.amount)/100.0 as brutto, COUNT(*) as anzahl
FROM pp_txn t JOIN pp_security s ON s.id=t.security_id
WHERE t.txn_type='DIVIDENDS' AND t.owner_type='account' AND t.date>=date('now','start of year')
GROUP BY s.id ORDER BY brutto DESC
```

Alle Tabellen: pp_security, pp_txn, pp_portfolio, pp_account, pp_price, pp_latest_price, pp_fifo_lot, pp_fifo_consumption, pp_watchlist, pp_watchlist_security, pp_exchange_rate, pp_txn_unit, pp_taxonomy, pp_classification, pp_classification_assignment, pp_cross_entry, pp_investment_plan{image_section}

=== STIL ===
Kurz, prägnant, Bullet Points. Kontext-Daten zuerst nutzen, SQL nur für fehlende Daten. Bei Screener-Fragen proaktiv Scan-Vorschläge machen."##,
        lang = lang_str,
        user = user_greeting,
        date = ctx.analysis_date,
        value = ctx.total_value,
        cur = ctx.base_currency,
        cost = ctx.total_cost_basis,
        gl = ctx.total_gain_loss_percent,
        perf = perf_str,
        div = ctx.annual_dividends,
        div_yield = ctx.dividend_yield.unwrap_or(0.0),
        currencies = currency_str,
        age = ctx.portfolio_age_days,
        provider = provider_status_str,
        portfolios = portfolios_str,
        h_count = ctx.holdings.len(),
        holdings = holdings_str,
        txns = txn_str,
        divs = div_str,
        watchlist = watchlist_str,
        sold = sold_positions_str,
        yearly = yearly_str,
        fees = fees_taxes_str,
        invest = investment_str,
        sectors = sector_str,
        extremes = extremes_str,
        image_section = image_section,
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

