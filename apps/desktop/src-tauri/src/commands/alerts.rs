//! Alert management commands for Tauri
//!
//! This module provides:
//! 1. Price Alerts - Alerts when securities reach specific price levels
//! 2. Allocation Alerts - Alerts when portfolio allocation deviates from targets

use crate::db;
use serde::{Deserialize, Serialize};
use tauri::command;
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAlert {
    pub id: i64,
    pub uuid: String,
    pub security_id: i64,
    pub security_name: Option<String>,
    pub security_ticker: Option<String>,
    pub alert_type: String,
    pub target_value: f64,
    pub target_value_2: Option<f64>,
    pub is_active: bool,
    pub is_triggered: bool,
    pub trigger_count: i64,
    pub last_triggered_at: Option<String>,
    pub last_triggered_price: Option<f64>,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlertRequest {
    pub security_id: i64,
    pub alert_type: String,
    pub target_value: f64,
    pub target_value_2: Option<f64>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertRequest {
    pub id: i64,
    pub target_value: Option<f64>,
    pub target_value_2: Option<f64>,
    pub is_active: Option<bool>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggeredAlert {
    pub alert: PriceAlert,
    pub current_price: f64,
    pub trigger_reason: String,
}

// ============================================================================
// Alert CRUD
// ============================================================================

/// Get all price alerts, optionally filtered by security
#[command]
pub fn get_price_alerts(security_id: Option<i64>) -> Result<Vec<PriceAlert>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let results = if let Some(sec_id) = security_id {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    a.id, a.uuid, a.security_id,
                    s.name as security_name,
                    s.ticker as security_ticker,
                    a.alert_type, a.target_value, a.target_value_2,
                    a.is_active, a.is_triggered, a.trigger_count,
                    a.last_triggered_at, a.last_triggered_price,
                    a.note, a.created_at
                FROM pp_price_alert a
                LEFT JOIN pp_security s ON s.id = a.security_id
                WHERE a.security_id = ?1
                ORDER BY a.created_at DESC
                "#,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([sec_id], map_alert_row)
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    a.id, a.uuid, a.security_id,
                    s.name as security_name,
                    s.ticker as security_ticker,
                    a.alert_type, a.target_value, a.target_value_2,
                    a.is_active, a.is_triggered, a.trigger_count,
                    a.last_triggered_at, a.last_triggered_price,
                    a.note, a.created_at
                FROM pp_price_alert a
                LEFT JOIN pp_security s ON s.id = a.security_id
                ORDER BY a.created_at DESC
                "#,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], map_alert_row)
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    Ok(results)
}

/// Get only active alerts for checking
#[command]
pub fn get_active_alerts() -> Result<Vec<PriceAlert>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                a.id, a.uuid, a.security_id,
                s.name as security_name,
                s.ticker as security_ticker,
                a.alert_type, a.target_value, a.target_value_2,
                a.is_active, a.is_triggered, a.trigger_count,
                a.last_triggered_at, a.last_triggered_price,
                a.note, a.created_at
            FROM pp_price_alert a
            LEFT JOIN pp_security s ON s.id = a.security_id
            WHERE a.is_active = 1
            ORDER BY a.created_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], map_alert_row)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Create a new price alert
#[command]
pub fn create_price_alert(request: CreateAlertRequest) -> Result<PriceAlert, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let uuid = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO pp_price_alert (
            uuid, security_id, alert_type, target_value, target_value_2, note, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        rusqlite::params![
            uuid,
            request.security_id,
            request.alert_type,
            request.target_value,
            request.target_value_2,
            request.note,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Fetch the created alert with security info
    let alert = conn
        .query_row(
            r#"
            SELECT
                a.id, a.uuid, a.security_id,
                s.name as security_name,
                s.ticker as security_ticker,
                a.alert_type, a.target_value, a.target_value_2,
                a.is_active, a.is_triggered, a.trigger_count,
                a.last_triggered_at, a.last_triggered_price,
                a.note, a.created_at
            FROM pp_price_alert a
            LEFT JOIN pp_security s ON s.id = a.security_id
            WHERE a.id = ?1
            "#,
            [id],
            map_alert_row,
        )
        .map_err(|e| e.to_string())?;

    Ok(alert)
}

/// Update an existing price alert
#[command]
pub fn update_price_alert(request: UpdateAlertRequest) -> Result<PriceAlert, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Build dynamic update query
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(val) = request.target_value {
        updates.push("target_value = ?");
        params.push(Box::new(val));
    }

    if let Some(val) = request.target_value_2 {
        updates.push("target_value_2 = ?");
        params.push(Box::new(val));
    }

    if let Some(active) = request.is_active {
        updates.push("is_active = ?");
        params.push(Box::new(active as i32));
    }

    if let Some(ref note) = request.note {
        updates.push("note = ?");
        params.push(Box::new(note.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    params.push(Box::new(request.id));

    let query = format!(
        "UPDATE pp_price_alert SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| e.to_string())?;

    // Fetch the updated alert
    let alert = conn
        .query_row(
            r#"
            SELECT
                a.id, a.uuid, a.security_id,
                s.name as security_name,
                s.ticker as security_ticker,
                a.alert_type, a.target_value, a.target_value_2,
                a.is_active, a.is_triggered, a.trigger_count,
                a.last_triggered_at, a.last_triggered_price,
                a.note, a.created_at
            FROM pp_price_alert a
            LEFT JOIN pp_security s ON s.id = a.security_id
            WHERE a.id = ?1
            "#,
            [request.id],
            map_alert_row,
        )
        .map_err(|e| e.to_string())?;

    Ok(alert)
}

/// Delete a price alert
#[command]
pub fn delete_price_alert(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_price_alert WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Toggle alert active status
#[command]
pub fn toggle_price_alert(id: i64) -> Result<PriceAlert, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "UPDATE pp_price_alert SET is_active = NOT is_active WHERE id = ?",
        [id],
    )
    .map_err(|e| e.to_string())?;

    // Fetch the updated alert
    let alert = conn
        .query_row(
            r#"
            SELECT
                a.id, a.uuid, a.security_id,
                s.name as security_name,
                s.ticker as security_ticker,
                a.alert_type, a.target_value, a.target_value_2,
                a.is_active, a.is_triggered, a.trigger_count,
                a.last_triggered_at, a.last_triggered_price,
                a.note, a.created_at
            FROM pp_price_alert a
            LEFT JOIN pp_security s ON s.id = a.security_id
            WHERE a.id = ?1
            "#,
            [id],
            map_alert_row,
        )
        .map_err(|e| e.to_string())?;

    Ok(alert)
}

// ============================================================================
// Alert Checking
// ============================================================================

/// Check all active alerts against current prices
/// Returns list of triggered alerts
#[command]
pub fn check_price_alerts() -> Result<Vec<TriggeredAlert>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut triggered = Vec::new();

    // Get all active alerts with current prices
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                a.id, a.uuid, a.security_id,
                s.name as security_name,
                s.ticker as security_ticker,
                a.alert_type, a.target_value, a.target_value_2,
                a.is_active, a.is_triggered, a.trigger_count,
                a.last_triggered_at, a.last_triggered_price,
                a.note, a.created_at,
                lp.value as current_price
            FROM pp_price_alert a
            LEFT JOIN pp_security s ON s.id = a.security_id
            LEFT JOIN pp_latest_price lp ON lp.security_id = a.security_id
            WHERE a.is_active = 1 AND lp.value IS NOT NULL
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let alert = PriceAlert {
                id: row.get(0)?,
                uuid: row.get(1)?,
                security_id: row.get(2)?,
                security_name: row.get(3)?,
                security_ticker: row.get(4)?,
                alert_type: row.get(5)?,
                target_value: row.get(6)?,
                target_value_2: row.get(7)?,
                is_active: row.get::<_, i32>(8)? != 0,
                is_triggered: row.get::<_, i32>(9)? != 0,
                trigger_count: row.get(10)?,
                last_triggered_at: row.get(11)?,
                last_triggered_price: row.get(12)?,
                note: row.get(13)?,
                created_at: row.get(14)?,
            };
            let current_price: f64 = row.get(15)?;
            Ok((alert, current_price))
        })
        .map_err(|e| e.to_string())?;

    let alerts_with_prices: Vec<(PriceAlert, f64)> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Check each alert
    for (alert, current_price) in alerts_with_prices {
        let (is_triggered, reason) = check_alert_condition(&alert, current_price);

        if is_triggered {
            triggered.push(TriggeredAlert {
                alert: alert.clone(),
                current_price,
                trigger_reason: reason,
            });

            // Update alert as triggered
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                r#"
                UPDATE pp_price_alert
                SET is_triggered = 1,
                    trigger_count = trigger_count + 1,
                    last_triggered_at = ?1,
                    last_triggered_price = ?2
                WHERE id = ?3
                "#,
                rusqlite::params![now, current_price, alert.id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(triggered)
}

/// Reset triggered status for an alert (to allow re-triggering)
#[command]
pub fn reset_alert_trigger(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        "UPDATE pp_price_alert SET is_triggered = 0 WHERE id = ?",
        [id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn map_alert_row(row: &rusqlite::Row) -> rusqlite::Result<PriceAlert> {
    Ok(PriceAlert {
        id: row.get(0)?,
        uuid: row.get(1)?,
        security_id: row.get(2)?,
        security_name: row.get(3)?,
        security_ticker: row.get(4)?,
        alert_type: row.get(5)?,
        target_value: row.get(6)?,
        target_value_2: row.get(7)?,
        is_active: row.get::<_, i32>(8)? != 0,
        is_triggered: row.get::<_, i32>(9)? != 0,
        trigger_count: row.get(10)?,
        last_triggered_at: row.get(11)?,
        last_triggered_price: row.get(12)?,
        note: row.get(13)?,
        created_at: row.get(14)?,
    })
}

fn check_alert_condition(alert: &PriceAlert, current_price: f64) -> (bool, String) {
    match alert.alert_type.as_str() {
        "price_above" => {
            let triggered = current_price > alert.target_value;
            let reason = format!(
                "Kurs ({:.2}) liegt über {:.2}",
                current_price, alert.target_value
            );
            (triggered, reason)
        }
        "price_below" => {
            let triggered = current_price < alert.target_value;
            let reason = format!(
                "Kurs ({:.2}) liegt unter {:.2}",
                current_price, alert.target_value
            );
            (triggered, reason)
        }
        "price_crosses" => {
            // For price_crosses, we need to check if price moved from one side to the other
            // This requires the last triggered price
            if let Some(last_price) = alert.last_triggered_price {
                let crossed_up =
                    last_price < alert.target_value && current_price >= alert.target_value;
                let crossed_down =
                    last_price > alert.target_value && current_price <= alert.target_value;
                let triggered = crossed_up || crossed_down;
                let direction = if crossed_up { "aufwärts" } else { "abwärts" };
                let reason = format!(
                    "Kurs ({:.2}) hat {:.2} {} gekreuzt",
                    current_price, alert.target_value, direction
                );
                (triggered, reason)
            } else {
                // First time checking, just check if we're at the level
                let triggered = (current_price - alert.target_value).abs()
                    < alert.target_value * 0.001;
                let reason = format!(
                    "Kurs ({:.2}) liegt nahe {:.2}",
                    current_price, alert.target_value
                );
                (triggered, reason)
            }
        }
        "support_break" => {
            let triggered = current_price < alert.target_value;
            let reason = format!(
                "Support-Level ({:.2}) wurde gebrochen, Kurs bei {:.2}",
                alert.target_value, current_price
            );
            (triggered, reason)
        }
        "resistance_break" => {
            let triggered = current_price > alert.target_value;
            let reason = format!(
                "Resistance-Level ({:.2}) wurde durchbrochen, Kurs bei {:.2}",
                alert.target_value, current_price
            );
            (triggered, reason)
        }
        // RSI alerts would need additional calculation - for now just return false
        "rsi_above" | "rsi_below" => (
            false,
            "RSI-Alerts benötigen Indikator-Berechnung".to_string(),
        ),
        // Pattern/divergence alerts are triggered by signal detection, not price
        "volume_spike" | "divergence" | "pattern_detected" => (
            false,
            "Dieser Alert-Typ wird durch Signal-Erkennung ausgelöst".to_string(),
        ),
        _ => (false, "Unbekannter Alert-Typ".to_string()),
    }
}

// ============================================================================
// Allocation Alerts
// ============================================================================

/// An allocation target for a security or classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationTarget {
    pub id: i64,
    pub portfolio_id: i64,
    pub security_id: Option<i64>,
    pub security_name: Option<String>,
    pub taxonomy_id: Option<i64>,
    pub taxonomy_name: Option<String>,
    pub classification_id: Option<i64>,
    pub classification_name: Option<String>,
    /// Target weight as decimal (0.0 - 1.0)
    pub target_weight: f64,
    /// Threshold for alerts as decimal (e.g., 0.05 = ±5%)
    pub threshold: f64,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Request to create or update an allocation target
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAllocationTargetRequest {
    pub portfolio_id: i64,
    pub security_id: Option<i64>,
    pub taxonomy_id: Option<i64>,
    pub classification_id: Option<i64>,
    /// Target weight as decimal (0.0 - 1.0)
    pub target_weight: f64,
    /// Threshold for alerts as decimal (e.g., 0.05 = ±5%)
    pub threshold: f64,
}

/// An alert indicating allocation deviation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationAlert {
    /// Type of alert: "over_weight" or "under_weight"
    pub alert_type: String,
    /// Name of the entity (security or classification)
    pub entity_name: String,
    /// Target weight as decimal
    pub target_weight: f64,
    /// Current actual weight as decimal
    pub current_weight: f64,
    /// Deviation from target (can be positive or negative)
    pub deviation: f64,
    /// Severity: "warning" (within 2x threshold) or "critical" (beyond 2x threshold)
    pub severity: String,
    /// Associated security ID if applicable
    pub security_id: Option<i64>,
    /// Associated classification ID if applicable
    pub classification_id: Option<i64>,
}

/// Alert count summary for badge display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllocationAlertCount {
    pub total: usize,
    pub critical: usize,
    pub warning: usize,
}

/// Get all allocation targets for a portfolio
#[command]
pub fn get_allocation_targets(portfolio_id: i64) -> Result<Vec<AllocationTarget>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                t.id,
                t.portfolio_id,
                t.security_id,
                s.name as security_name,
                t.taxonomy_id,
                tax.name as taxonomy_name,
                t.classification_id,
                c.name as classification_name,
                t.target_weight,
                t.threshold,
                t.created_at,
                t.updated_at
            FROM pp_allocation_target t
            LEFT JOIN pp_security s ON s.id = t.security_id
            LEFT JOIN pp_taxonomy tax ON tax.id = t.taxonomy_id
            LEFT JOIN pp_classification c ON c.id = t.classification_id
            WHERE t.portfolio_id = ?
            ORDER BY t.target_weight DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([portfolio_id], |row| {
            Ok(AllocationTarget {
                id: row.get(0)?,
                portfolio_id: row.get(1)?,
                security_id: row.get(2)?,
                security_name: row.get(3)?,
                taxonomy_id: row.get(4)?,
                taxonomy_name: row.get(5)?,
                classification_id: row.get(6)?,
                classification_name: row.get(7)?,
                target_weight: row.get(8)?,
                threshold: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Set (create or update) an allocation target
#[command]
pub fn set_allocation_target(request: SetAllocationTargetRequest) -> Result<i64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Validate that either security_id or (taxonomy_id + classification_id) is provided
    if request.security_id.is_none()
        && (request.taxonomy_id.is_none() || request.classification_id.is_none())
    {
        return Err(
            "Either security_id or both taxonomy_id and classification_id must be provided"
                .to_string(),
        );
    }

    // Validate weight is between 0 and 1
    if request.target_weight < 0.0 || request.target_weight > 1.0 {
        return Err("Target weight must be between 0.0 and 1.0".to_string());
    }

    // Validate threshold is positive
    if request.threshold <= 0.0 {
        return Err("Threshold must be positive".to_string());
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Try to find existing target
    let existing_id: Option<i64> = if let Some(security_id) = request.security_id {
        conn.query_row(
            "SELECT id FROM pp_allocation_target WHERE portfolio_id = ? AND security_id = ?",
            [request.portfolio_id, security_id],
            |row| row.get(0),
        )
        .ok()
    } else {
        conn.query_row(
            "SELECT id FROM pp_allocation_target WHERE portfolio_id = ? AND taxonomy_id = ? AND classification_id = ?",
            rusqlite::params![request.portfolio_id, request.taxonomy_id, request.classification_id],
            |row| row.get(0),
        )
        .ok()
    };

    if let Some(id) = existing_id {
        // Update existing target
        conn.execute(
            "UPDATE pp_allocation_target SET target_weight = ?, threshold = ?, updated_at = ? WHERE id = ?",
            rusqlite::params![request.target_weight, request.threshold, now, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(id)
    } else {
        // Insert new target
        conn.execute(
            r#"
            INSERT INTO pp_allocation_target (portfolio_id, security_id, taxonomy_id, classification_id, target_weight, threshold, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                request.portfolio_id,
                request.security_id,
                request.taxonomy_id,
                request.classification_id,
                request.target_weight,
                request.threshold,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }
}

/// Delete an allocation target
#[command]
pub fn delete_allocation_target(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_allocation_target WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get allocation alerts for a portfolio
///
/// Compares current holdings allocation against defined targets and returns
/// alerts for any deviations exceeding the threshold.
#[command]
pub fn get_allocation_alerts(portfolio_id: Option<i64>) -> Result<Vec<AllocationAlert>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut alerts = Vec::new();

    // Get all allocation targets (optionally filtered by portfolio)
    let targets: Vec<AllocationTarget> = if let Some(pid) = portfolio_id {
        get_allocation_targets(pid)?
    } else {
        // Get all targets for all portfolios
        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    t.id,
                    t.portfolio_id,
                    t.security_id,
                    s.name as security_name,
                    t.taxonomy_id,
                    tax.name as taxonomy_name,
                    t.classification_id,
                    c.name as classification_name,
                    t.target_weight,
                    t.threshold,
                    t.created_at,
                    t.updated_at
                FROM pp_allocation_target t
                LEFT JOIN pp_security s ON s.id = t.security_id
                LEFT JOIN pp_taxonomy tax ON tax.id = t.taxonomy_id
                LEFT JOIN pp_classification c ON c.id = t.classification_id
                ORDER BY t.portfolio_id, t.target_weight DESC
                "#,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(AllocationTarget {
                    id: row.get(0)?,
                    portfolio_id: row.get(1)?,
                    security_id: row.get(2)?,
                    security_name: row.get(3)?,
                    taxonomy_id: row.get(4)?,
                    taxonomy_name: row.get(5)?,
                    classification_id: row.get(6)?,
                    classification_name: row.get(7)?,
                    target_weight: row.get(8)?,
                    threshold: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    // Get total portfolio value
    let total_value: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(value), 0)
            FROM (
                SELECT
                    COALESCE(lp.value, 0) / 100000000.0 *
                    COALESCE((
                        SELECT SUM(CASE
                            WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                            WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                            ELSE 0
                        END) / 100000000.0
                        FROM pp_txn
                        WHERE owner_type = 'portfolio' AND security_id = s.id
                    ), 0) as value
                FROM pp_security s
                LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
            )
            "#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    if total_value <= 0.0 {
        return Ok(alerts);
    }

    for target in targets {
        let current_weight = if let Some(security_id) = target.security_id {
            // Calculate weight for specific security
            let security_value: f64 = conn
                .query_row(
                    r#"
                    SELECT COALESCE(
                        (SELECT COALESCE(lp.value, 0) / 100000000.0 FROM pp_latest_price lp WHERE lp.security_id = ?1) *
                        COALESCE((
                            SELECT SUM(CASE
                                WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                                WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                                ELSE 0
                            END) / 100000000.0
                            FROM pp_txn
                            WHERE owner_type = 'portfolio' AND security_id = ?1
                        ), 0),
                    0)
                    "#,
                    [security_id],
                    |row| row.get(0),
                )
                .unwrap_or(0.0);

            security_value / total_value
        } else if let Some(classification_id) = target.classification_id {
            // Calculate weight for classification (all securities in this classification)
            let classification_value: f64 = conn
                .query_row(
                    r#"
                    SELECT COALESCE(SUM(
                        (SELECT COALESCE(lp.value, 0) / 100000000.0 FROM pp_latest_price lp WHERE lp.security_id = s.id) *
                        COALESCE((
                            SELECT SUM(CASE
                                WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                                WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                                ELSE 0
                            END) / 100000000.0
                            FROM pp_txn
                            WHERE owner_type = 'portfolio' AND security_id = s.id
                        ), 0)
                    ), 0)
                    FROM pp_security s
                    JOIN pp_classification_assignment ca ON ca.vehicle_uuid = s.uuid AND ca.vehicle_type = 'security'
                    WHERE ca.classification_id = ?
                    "#,
                    [classification_id],
                    |row| row.get(0),
                )
                .unwrap_or(0.0);

            classification_value / total_value
        } else {
            continue;
        };

        let deviation = current_weight - target.target_weight;
        let abs_deviation = deviation.abs();

        // Check if deviation exceeds threshold
        if abs_deviation > target.threshold {
            let alert_type = if deviation > 0.0 {
                "over_weight"
            } else {
                "under_weight"
            };

            let severity = if abs_deviation > target.threshold * 2.0 {
                "critical"
            } else {
                "warning"
            };

            let entity_name = target
                .security_name
                .clone()
                .or(target.classification_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            alerts.push(AllocationAlert {
                alert_type: alert_type.to_string(),
                entity_name,
                target_weight: target.target_weight,
                current_weight,
                deviation,
                severity: severity.to_string(),
                security_id: target.security_id,
                classification_id: target.classification_id,
            });
        }
    }

    // Sort alerts by severity (critical first) then by absolute deviation
    alerts.sort_by(|a, b| {
        let severity_order = |s: &str| match s {
            "critical" => 0,
            "warning" => 1,
            _ => 2,
        };
        severity_order(&a.severity)
            .cmp(&severity_order(&b.severity))
            .then_with(|| {
                b.deviation
                    .abs()
                    .partial_cmp(&a.deviation.abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    Ok(alerts)
}

/// Get the count of active allocation alerts (for badge display)
#[command]
pub fn get_allocation_alert_count(portfolio_id: Option<i64>) -> Result<AllocationAlertCount, String> {
    let alerts = get_allocation_alerts(portfolio_id)?;

    let critical = alerts.iter().filter(|a| a.severity == "critical").count();
    let warning = alerts.iter().filter(|a| a.severity == "warning").count();

    Ok(AllocationAlertCount {
        total: alerts.len(),
        critical,
        warning,
    })
}
