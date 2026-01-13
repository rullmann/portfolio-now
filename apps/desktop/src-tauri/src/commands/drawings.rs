//! Chart drawing commands for saving and loading user-drawn chart elements.

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::get_connection;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub time: Option<String>,
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartDrawing {
    pub id: Option<String>,
    pub security_id: i64,
    pub drawing_type: String,
    pub points: Vec<Point>,
    pub color: String,
    pub line_width: i32,
    pub fib_levels: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartDrawingResponse {
    pub id: String,
    pub uuid: String,
    pub security_id: i64,
    pub drawing_type: String,
    pub points: Vec<Point>,
    pub color: String,
    pub line_width: i32,
    pub fib_levels: Option<Vec<f64>>,
    pub is_visible: bool,
    pub created_at: String,
}

// ============================================================================
// Commands
// ============================================================================

/// Save a chart drawing to the database.
#[tauri::command]
pub async fn save_chart_drawing(drawing: ChartDrawing) -> Result<ChartDrawingResponse, String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let uuid = Uuid::new_v4().to_string();
    let points_json = serde_json::to_string(&drawing.points).map_err(|e| e.to_string())?;
    let fib_levels_json = drawing
        .fib_levels
        .as_ref()
        .map(|l| serde_json::to_string(l))
        .transpose()
        .map_err(|e| e.to_string())?;

    conn.execute(
        r#"
        INSERT INTO pp_chart_drawing (
            uuid, security_id, drawing_type, points_json, color, line_width, fib_levels_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        params![
            uuid,
            drawing.security_id,
            drawing.drawing_type,
            points_json,
            drawing.color,
            drawing.line_width,
            fib_levels_json,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(ChartDrawingResponse {
        id: id.to_string(),
        uuid,
        security_id: drawing.security_id,
        drawing_type: drawing.drawing_type,
        points: drawing.points,
        color: drawing.color,
        line_width: drawing.line_width,
        fib_levels: drawing.fib_levels,
        is_visible: true,
        created_at,
    })
}

/// Get all drawings for a security.
#[tauri::command]
pub async fn get_chart_drawings(security_id: i64) -> Result<Vec<ChartDrawingResponse>, String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, uuid, security_id, drawing_type, points_json, color, line_width,
                   fib_levels_json, is_visible, created_at
            FROM pp_chart_drawing
            WHERE security_id = ? AND is_visible = 1
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let drawings: Vec<ChartDrawingResponse> = stmt
        .query_map(params![security_id], |row| {
            let id: i64 = row.get(0)?;
            let points_json: String = row.get(4)?;
            let fib_levels_json: Option<String> = row.get(7)?;

            let points: Vec<Point> = serde_json::from_str(&points_json).unwrap_or_default();
            let fib_levels: Option<Vec<f64>> = fib_levels_json
                .as_ref()
                .and_then(|j| serde_json::from_str(j).ok());

            Ok(ChartDrawingResponse {
                id: id.to_string(),
                uuid: row.get(1)?,
                security_id: row.get(2)?,
                drawing_type: row.get(3)?,
                points,
                color: row.get(5)?,
                line_width: row.get(6)?,
                fib_levels,
                is_visible: row.get::<_, i32>(8)? == 1,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(drawings)
}

/// Delete a chart drawing.
#[tauri::command]
pub async fn delete_chart_drawing(drawing_id: i64) -> Result<(), String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    conn.execute("DELETE FROM pp_chart_drawing WHERE id = ?", params![drawing_id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Delete all drawings for a security.
#[tauri::command]
pub async fn clear_chart_drawings(security_id: i64) -> Result<(), String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    conn.execute(
        "DELETE FROM pp_chart_drawing WHERE security_id = ?",
        params![security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
