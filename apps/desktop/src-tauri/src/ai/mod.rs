//! AI-powered chart analysis module.
//!
//! Supports multiple providers: Claude (Anthropic), GPT-4 (OpenAI), Gemini (Google)

pub mod claude;
pub mod openai;
pub mod gemini;

use serde::{Deserialize, Serialize};

/// Request for chart analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartAnalysisRequest {
    pub image_base64: String,
    pub provider: String,
    pub api_key: String,
    pub context: ChartContext,
}

/// Context about the chart being analyzed
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartContext {
    pub security_name: String,
    pub ticker: Option<String>,
    pub currency: String,
    pub current_price: f64,
    pub timeframe: String,
    pub indicators: Vec<String>,
}

/// Response from AI analysis
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartAnalysisResponse {
    pub analysis: String,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

/// Build the analysis prompt with chart context
pub fn build_analysis_prompt(ctx: &ChartContext) -> String {
    let indicators_str = if ctx.indicators.is_empty() {
        "Keine".to_string()
    } else {
        ctx.indicators.join(", ")
    };

    format!(
        r#"Du bist ein erfahrener technischer Analyst. Analysiere den beigefügten Chart.

**Wertpapier:** {} ({})
**Zeitraum:** {}
**Aktueller Kurs:** {:.2} {}
**Aktive Indikatoren:** {}

Analysiere bitte:

1. **Trend**: Primärer Trend, Trendstärke, mögliche Trendwenden
2. **Unterstützung & Widerstand**: Wichtige Preisniveaus identifizieren
3. **Chartmuster**: Erkennbare Formationen (Dreiecke, Flaggen, Kopf-Schulter, etc.)
4. **Indikatoren-Interpretation**: Was sagen die aktiven Indikatoren aus?
5. **Volumen**: Bestätigt das Volumen die Preisbewegung?
6. **Einschätzung**: Kurz- und mittelfristige Perspektive

Gib eine strukturierte, sachliche Analyse. Erwähne Chancen UND Risiken.

Hinweis: Dies dient nur zu Informationszwecken und ist keine Anlageberatung."#,
        ctx.security_name,
        ctx.ticker.as_deref().unwrap_or("-"),
        ctx.timeframe,
        ctx.current_price,
        ctx.currency,
        indicators_str
    )
}
