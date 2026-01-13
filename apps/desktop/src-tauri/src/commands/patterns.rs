//! Pattern tracking commands for candlestick pattern success rate analysis.
//!
//! This module provides commands to:
//! - Save detected patterns to the database
//! - Evaluate pattern outcomes after 5/10 days
//! - Get pattern statistics and success rates

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::get_connection;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternDetection {
    pub security_id: i64,
    pub pattern_type: String,
    pub detected_at: String,
    pub price_at_detection: f64,
    pub predicted_direction: String, // "bullish", "bearish", "neutral"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternHistory {
    pub id: i64,
    pub security_id: i64,
    pub pattern_type: String,
    pub detected_at: String,
    pub price_at_detection: f64,
    pub predicted_direction: String,
    pub actual_outcome: Option<String>,
    pub price_after_5d: Option<f64>,
    pub price_after_10d: Option<f64>,
    pub price_change_5d_percent: Option<f64>,
    pub price_change_10d_percent: Option<f64>,
    pub evaluated_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternStatistics {
    pub pattern_type: String,
    pub total_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub pending_count: i64,
    pub success_rate: f64,
    pub avg_gain_on_success: Option<f64>,
    pub avg_loss_on_failure: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternEvaluationResult {
    pub patterns_evaluated: i64,
    pub successes: i64,
    pub failures: i64,
}

// ============================================================================
// Commands
// ============================================================================

/// Save a detected pattern to the database for tracking.
#[tauri::command]
pub async fn save_pattern_detection(pattern: PatternDetection) -> Result<i64, String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Check if this pattern was already detected (same security, pattern, date)
    let existing: Option<i64> = conn
        .query_row(
            r#"
            SELECT id FROM pp_pattern_history
            WHERE security_id = ? AND pattern_type = ? AND detected_at = ?
            "#,
            params![pattern.security_id, pattern.pattern_type, pattern.detected_at],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        return Ok(id); // Pattern already tracked
    }

    conn.execute(
        r#"
        INSERT INTO pp_pattern_history (
            security_id, pattern_type, detected_at, price_at_detection,
            predicted_direction, actual_outcome
        ) VALUES (?, ?, ?, ?, ?, 'pending')
        "#,
        params![
            pattern.security_id,
            pattern.pattern_type,
            pattern.detected_at,
            pattern.price_at_detection,
            pattern.predicted_direction,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

/// Evaluate pending patterns that are old enough (5+ days).
/// Compares the price at detection with current/later prices to determine success.
#[tauri::command]
pub async fn evaluate_pattern_outcomes() -> Result<PatternEvaluationResult, String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Get pending patterns that are at least 5 days old
    let mut stmt = conn
        .prepare(
            r#"
            SELECT ph.id, ph.security_id, ph.detected_at, ph.price_at_detection,
                   ph.predicted_direction
            FROM pp_pattern_history ph
            WHERE ph.actual_outcome = 'pending'
              AND date(ph.detected_at) <= date('now', '-5 days')
            "#,
        )
        .map_err(|e| e.to_string())?;

    let patterns: Vec<(i64, i64, String, f64, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let mut successes = 0i64;
    let mut failures = 0i64;
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();

    for (id, security_id, detected_at, price_at_detection, predicted_direction) in &patterns {
        // Get price 5 days after detection
        let price_5d: Option<f64> = conn
            .query_row(
                r#"
                SELECT value / 100000000.0
                FROM pp_price
                WHERE security_id = ? AND date >= date(?, '+5 days')
                ORDER BY date ASC
                LIMIT 1
                "#,
                params![security_id, detected_at],
                |row| row.get(0),
            )
            .ok();

        // Get price 10 days after detection
        let price_10d: Option<f64> = conn
            .query_row(
                r#"
                SELECT value / 100000000.0
                FROM pp_price
                WHERE security_id = ? AND date >= date(?, '+10 days')
                ORDER BY date ASC
                LIMIT 1
                "#,
                params![security_id, detected_at],
                |row| row.get(0),
            )
            .ok();

        // Calculate price changes
        let change_5d = price_5d.map(|p| ((p - price_at_detection) / price_at_detection) * 100.0);
        let change_10d = price_10d.map(|p| ((p - price_at_detection) / price_at_detection) * 100.0);

        // Determine outcome based on predicted direction and actual price movement
        let outcome = if let Some(change) = change_5d {
            let is_success = match predicted_direction.as_str() {
                "bullish" => change > 1.0, // At least 1% gain for bullish
                "bearish" => change < -1.0, // At least 1% drop for bearish
                _ => false, // Neutral patterns are not evaluated
            };

            if predicted_direction == "neutral" {
                None // Don't evaluate neutral patterns
            } else if is_success {
                successes += 1;
                Some("success")
            } else {
                failures += 1;
                Some("failure")
            }
        } else {
            None // Not enough data yet
        };

        // Update the pattern record
        if let Some(outcome) = outcome {
            conn.execute(
                r#"
                UPDATE pp_pattern_history
                SET actual_outcome = ?,
                    price_after_5d = ?,
                    price_after_10d = ?,
                    price_change_5d_percent = ?,
                    price_change_10d_percent = ?,
                    evaluated_at = ?
                WHERE id = ?
                "#,
                params![outcome, price_5d, price_10d, change_5d, change_10d, now, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(PatternEvaluationResult {
        patterns_evaluated: successes + failures,
        successes,
        failures,
    })
}

/// Get statistics for all pattern types.
#[tauri::command]
pub async fn get_pattern_statistics() -> Result<Vec<PatternStatistics>, String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                pattern_type,
                COUNT(*) as total,
                SUM(CASE WHEN actual_outcome = 'success' THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN actual_outcome = 'failure' THEN 1 ELSE 0 END) as failure_count,
                SUM(CASE WHEN actual_outcome = 'pending' THEN 1 ELSE 0 END) as pending_count,
                AVG(CASE WHEN actual_outcome = 'success' THEN price_change_5d_percent END) as avg_gain,
                AVG(CASE WHEN actual_outcome = 'failure' THEN price_change_5d_percent END) as avg_loss
            FROM pp_pattern_history
            WHERE predicted_direction != 'neutral'
            GROUP BY pattern_type
            ORDER BY total DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let stats: Vec<PatternStatistics> = stmt
        .query_map([], |row| {
            let total: i64 = row.get(1)?;
            let success_count: i64 = row.get(2)?;
            let failure_count: i64 = row.get(3)?;
            let pending_count: i64 = row.get(4)?;
            let evaluated = success_count + failure_count;
            let success_rate = if evaluated > 0 {
                (success_count as f64 / evaluated as f64) * 100.0
            } else {
                0.0
            };

            Ok(PatternStatistics {
                pattern_type: row.get(0)?,
                total_count: total,
                success_count,
                failure_count,
                pending_count,
                success_rate,
                avg_gain_on_success: row.get(5).ok(),
                avg_loss_on_failure: row.get(6).ok(),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(stats)
}

/// Get pattern history for a specific security.
#[tauri::command]
pub async fn get_pattern_history(security_id: i64) -> Result<Vec<PatternHistory>, String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, security_id, pattern_type, detected_at, price_at_detection,
                   predicted_direction, actual_outcome, price_after_5d, price_after_10d,
                   price_change_5d_percent, price_change_10d_percent, evaluated_at, created_at
            FROM pp_pattern_history
            WHERE security_id = ?
            ORDER BY detected_at DESC
            LIMIT 100
            "#,
        )
        .map_err(|e| e.to_string())?;

    let history: Vec<PatternHistory> = stmt
        .query_map(params![security_id], |row| {
            Ok(PatternHistory {
                id: row.get(0)?,
                security_id: row.get(1)?,
                pattern_type: row.get(2)?,
                detected_at: row.get(3)?,
                price_at_detection: row.get(4)?,
                predicted_direction: row.get(5)?,
                actual_outcome: row.get(6)?,
                price_after_5d: row.get(7)?,
                price_after_10d: row.get(8)?,
                price_change_5d_percent: row.get(9)?,
                price_change_10d_percent: row.get(10)?,
                evaluated_at: row.get(11)?,
                created_at: row.get(12)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(history)
}
