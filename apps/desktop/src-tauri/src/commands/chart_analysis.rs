//! Persistence for AI chart analysis results
//!
//! Stores the analysis text, trend info, alerts, and risk/reward data
//! so they survive app restarts.

use crate::db;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedChartAnalysis {
    pub id: i64,
    pub security_id: i64,
    pub analysis_text: String,
    pub trend_direction: Option<String>,
    pub trend_strength: Option<String>,
    pub trend_confidence: Option<f64>,
    pub alerts_json: Option<String>,
    pub risk_reward_json: Option<String>,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveChartAnalysisRequest {
    pub security_id: i64,
    pub analysis_text: String,
    pub trend_direction: Option<String>,
    pub trend_strength: Option<String>,
    pub trend_confidence: Option<f64>,
    pub alerts_json: Option<String>,
    pub risk_reward_json: Option<String>,
    pub provider: String,
    pub model: String,
    pub tokens_used: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

/// Save chart analysis for a security (replaces any existing analysis for this security)
#[command]
pub fn save_chart_analysis(request: SaveChartAnalysisRequest) -> Result<PersistedChartAnalysis, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Delete any existing analysis for this security (keep only the latest)
    conn.execute(
        "DELETE FROM pp_chart_analysis WHERE security_id = ?1",
        [request.security_id],
    )
    .map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"INSERT INTO pp_chart_analysis
           (security_id, analysis_text, trend_direction, trend_strength, trend_confidence,
            alerts_json, risk_reward_json, provider, model, tokens_used, input_tokens, output_tokens, created_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
        rusqlite::params![
            request.security_id,
            request.analysis_text,
            request.trend_direction,
            request.trend_strength,
            request.trend_confidence,
            request.alerts_json,
            request.risk_reward_json,
            request.provider,
            request.model,
            request.tokens_used,
            request.input_tokens,
            request.output_tokens,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(PersistedChartAnalysis {
        id,
        security_id: request.security_id,
        analysis_text: request.analysis_text,
        trend_direction: request.trend_direction,
        trend_strength: request.trend_strength,
        trend_confidence: request.trend_confidence,
        alerts_json: request.alerts_json,
        risk_reward_json: request.risk_reward_json,
        provider: request.provider,
        model: request.model,
        tokens_used: request.tokens_used,
        input_tokens: request.input_tokens,
        output_tokens: request.output_tokens,
        created_at: now,
    })
}

/// Get the latest chart analysis for a security
#[command]
pub fn get_chart_analysis(security_id: i64) -> Result<Option<PersistedChartAnalysis>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"SELECT id, security_id, analysis_text, trend_direction, trend_strength,
                      trend_confidence, alerts_json, risk_reward_json, provider, model,
                      tokens_used, input_tokens, output_tokens, created_at
               FROM pp_chart_analysis
               WHERE security_id = ?1
               ORDER BY created_at DESC
               LIMIT 1"#,
        )
        .map_err(|e| e.to_string())?;

    let result = stmt
        .query_row([security_id], |row| {
            Ok(PersistedChartAnalysis {
                id: row.get(0)?,
                security_id: row.get(1)?,
                analysis_text: row.get(2)?,
                trend_direction: row.get(3)?,
                trend_strength: row.get(4)?,
                trend_confidence: row.get(5)?,
                alerts_json: row.get(6)?,
                risk_reward_json: row.get(7)?,
                provider: row.get(8)?,
                model: row.get(9)?,
                tokens_used: row.get(10)?,
                input_tokens: row.get(11)?,
                output_tokens: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .ok(); // Returns None if no row found

    Ok(result)
}

/// Delete chart analysis for a security
#[command]
pub fn delete_chart_analysis(security_id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "DELETE FROM pp_chart_analysis WHERE security_id = ?1",
        [security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
