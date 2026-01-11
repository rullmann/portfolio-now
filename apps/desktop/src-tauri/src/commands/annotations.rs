//! Chart annotation management commands for Tauri
//!
//! Annotations are AI-generated or user-created markers on charts,
//! such as support/resistance levels, patterns, and signals.

use crate::db;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::command;
use uuid::Uuid;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationData {
    pub id: i64,
    pub uuid: String,
    pub security_id: i64,
    pub annotation_type: String,
    pub price: f64,
    pub time: Option<String>,
    pub time_end: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub confidence: f64,
    pub signal: Option<String>,
    pub source: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub is_visible: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAnnotationRequest {
    pub annotation_type: String,
    pub price: f64,
    pub time: Option<String>,
    pub time_end: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub confidence: f64,
    pub signal: Option<String>,
    pub source: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

// ============================================================================
// Commands
// ============================================================================

/// Save multiple annotations for a security (batch insert)
///
/// If `clear_existing` is true, all existing AI annotations for this security
/// will be deleted before inserting the new ones.
#[command]
pub fn save_annotations(
    security_id: i64,
    annotations: Vec<SaveAnnotationRequest>,
    clear_existing: bool,
) -> Result<Vec<AnnotationData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Optionally clear existing AI annotations
    if clear_existing {
        conn.execute(
            "DELETE FROM pp_chart_annotation WHERE security_id = ?1 AND source = 'ai'",
            [security_id],
        )
        .map_err(|e| e.to_string())?;
    }

    let now = Utc::now().to_rfc3339();
    let mut saved = Vec::new();

    for ann in annotations {
        let uuid = Uuid::new_v4().to_string();
        let source = ann.source.unwrap_or_else(|| "ai".to_string());

        conn.execute(
            r#"INSERT INTO pp_chart_annotation
               (uuid, security_id, annotation_type, price, time, time_end, title, description,
                confidence, signal, source, provider, model, is_visible, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1, ?14)"#,
            rusqlite::params![
                uuid,
                security_id,
                ann.annotation_type,
                ann.price,
                ann.time,
                ann.time_end,
                ann.title,
                ann.description,
                ann.confidence,
                ann.signal,
                source,
                ann.provider,
                ann.model,
                now
            ],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();
        saved.push(AnnotationData {
            id,
            uuid,
            security_id,
            annotation_type: ann.annotation_type,
            price: ann.price,
            time: ann.time,
            time_end: ann.time_end,
            title: ann.title,
            description: ann.description,
            confidence: ann.confidence,
            signal: ann.signal,
            source,
            provider: ann.provider,
            model: ann.model,
            is_visible: true,
            created_at: now.clone(),
        });
    }

    Ok(saved)
}

/// Get annotations for a security
///
/// If `visible_only` is true, only visible annotations are returned.
#[command]
pub fn get_annotations(security_id: i64, visible_only: bool) -> Result<Vec<AnnotationData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let sql = if visible_only {
        r#"SELECT id, uuid, security_id, annotation_type, price, time, time_end,
                  title, description, confidence, signal, source, provider, model,
                  is_visible, created_at
           FROM pp_chart_annotation
           WHERE security_id = ?1 AND is_visible = 1
           ORDER BY created_at DESC"#
    } else {
        r#"SELECT id, uuid, security_id, annotation_type, price, time, time_end,
                  title, description, confidence, signal, source, provider, model,
                  is_visible, created_at
           FROM pp_chart_annotation
           WHERE security_id = ?1
           ORDER BY created_at DESC"#
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([security_id], |row| {
            Ok(AnnotationData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                security_id: row.get(2)?,
                annotation_type: row.get(3)?,
                price: row.get(4)?,
                time: row.get(5)?,
                time_end: row.get(6)?,
                title: row.get(7)?,
                description: row.get(8)?,
                confidence: row.get(9)?,
                signal: row.get(10)?,
                source: row.get(11)?,
                provider: row.get(12)?,
                model: row.get(13)?,
                is_visible: row.get::<_, i32>(14)? == 1,
                created_at: row.get(15)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

/// Delete a single annotation
#[command]
pub fn delete_annotation(annotation_id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let rows = conn
        .execute(
            "DELETE FROM pp_chart_annotation WHERE id = ?1",
            [annotation_id],
        )
        .map_err(|e| e.to_string())?;

    if rows == 0 {
        return Err(format!("Annotation with id {} not found", annotation_id));
    }

    Ok(())
}

/// Toggle annotation visibility
///
/// Returns the new visibility state.
#[command]
pub fn toggle_annotation_visibility(annotation_id: i64) -> Result<bool, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Toggle the visibility
    conn.execute(
        "UPDATE pp_chart_annotation SET is_visible = 1 - is_visible WHERE id = ?1",
        [annotation_id],
    )
    .map_err(|e| e.to_string())?;

    // Return the new state
    let is_visible: i32 = conn
        .query_row(
            "SELECT is_visible FROM pp_chart_annotation WHERE id = ?1",
            [annotation_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(is_visible == 1)
}

/// Clear all AI annotations for a security
///
/// Returns the number of deleted annotations.
#[command]
pub fn clear_ai_annotations(security_id: i64) -> Result<i32, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let rows = conn
        .execute(
            "DELETE FROM pp_chart_annotation WHERE security_id = ?1 AND source = 'ai'",
            [security_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(rows as i32)
}
