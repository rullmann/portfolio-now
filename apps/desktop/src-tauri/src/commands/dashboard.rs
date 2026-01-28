//! Dashboard Widget System
//!
//! Manages customizable dashboard layouts with drag-and-drop widgets.

use crate::db;
use serde::{Deserialize, Serialize};

/// Widget types available in the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    PortfolioValue,
    Performance,
    HoldingsTable,
    HoldingsPie,
    RecentTransactions,
    Dividends,
    Watchlist,
    Heatmap,
    YearReturns,
    Alerts,
    Chart,
    Benchmark,
}

// Note: Widget size and labels are defined in the frontend (TypeScript)
// to keep the dashboard system flexible and allow i18n.

/// Widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub widget_type: WidgetType,
    pub title: Option<String>,
    pub position: Position,
    pub size: Size,
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// Dashboard layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    pub id: i64,
    pub name: String,
    pub columns: u32,
    pub widgets: Vec<WidgetConfig>,
    pub is_default: bool,
}

/// Widget definition for catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetDefinition {
    pub widget_type: WidgetType,
    pub label: String,
    pub description: String,
    pub default_width: u32,
    pub default_height: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub configurable: bool,
}

/// Get available widget definitions
#[tauri::command]
pub fn get_available_widgets() -> Result<Vec<WidgetDefinition>, String> {
    let widgets = vec![
        WidgetDefinition {
            widget_type: WidgetType::PortfolioValue,
            label: "Depotwert".to_string(),
            description: "Zeigt den aktuellen Depotwert mit Sparkline".to_string(),
            default_width: 2,
            default_height: 1,
            min_width: 1,
            min_height: 1,
            max_width: 4,
            max_height: 2,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::Performance,
            label: "Performance".to_string(),
            description: "TTWROR, IRR und Gewinn/Verlust".to_string(),
            default_width: 2,
            default_height: 1,
            min_width: 2,
            min_height: 1,
            max_width: 4,
            max_height: 2,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::HoldingsTable,
            label: "Bestände (Tabelle)".to_string(),
            description: "Tabellarische Ansicht der Bestände".to_string(),
            default_width: 2,
            default_height: 2,
            min_width: 2,
            min_height: 2,
            max_width: 6,
            max_height: 4,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::HoldingsPie,
            label: "Bestände (Diagramm)".to_string(),
            description: "Donut-Chart der Allokation".to_string(),
            default_width: 2,
            default_height: 2,
            min_width: 2,
            min_height: 2,
            max_width: 4,
            max_height: 4,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::RecentTransactions,
            label: "Letzte Buchungen".to_string(),
            description: "Die letzten N Transaktionen".to_string(),
            default_width: 2,
            default_height: 2,
            min_width: 2,
            min_height: 2,
            max_width: 4,
            max_height: 4,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::Dividends,
            label: "Dividenden".to_string(),
            description: "Nächste und letzte Dividenden".to_string(),
            default_width: 2,
            default_height: 1,
            min_width: 2,
            min_height: 1,
            max_width: 4,
            max_height: 2,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::Watchlist,
            label: "Watchlist".to_string(),
            description: "Mini-Watchlist mit Kursen".to_string(),
            default_width: 2,
            default_height: 2,
            min_width: 2,
            min_height: 2,
            max_width: 4,
            max_height: 4,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::Heatmap,
            label: "Heatmap".to_string(),
            description: "Monatsrenditen als Heatmap".to_string(),
            default_width: 4,
            default_height: 2,
            min_width: 3,
            min_height: 2,
            max_width: 6,
            max_height: 4,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::YearReturns,
            label: "Jahresrenditen".to_string(),
            description: "Renditen pro Jahr".to_string(),
            default_width: 4,
            default_height: 1,
            min_width: 3,
            min_height: 1,
            max_width: 6,
            max_height: 2,
            configurable: false,
        },
        WidgetDefinition {
            widget_type: WidgetType::Alerts,
            label: "Warnungen".to_string(),
            description: "Limit-Überschreitungen".to_string(),
            default_width: 2,
            default_height: 1,
            min_width: 2,
            min_height: 1,
            max_width: 4,
            max_height: 2,
            configurable: false,
        },
        WidgetDefinition {
            widget_type: WidgetType::Chart,
            label: "Kurschart".to_string(),
            description: "Performance-Chart über Zeit".to_string(),
            default_width: 4,
            default_height: 2,
            min_width: 3,
            min_height: 2,
            max_width: 6,
            max_height: 4,
            configurable: true,
        },
        WidgetDefinition {
            widget_type: WidgetType::Benchmark,
            label: "Benchmark".to_string(),
            description: "Vergleich mit Benchmark".to_string(),
            default_width: 2,
            default_height: 2,
            min_width: 2,
            min_height: 2,
            max_width: 4,
            max_height: 4,
            configurable: true,
        },
    ];

    Ok(widgets)
}

/// Get dashboard layout
#[tauri::command]
pub fn get_dashboard_layout(layout_id: Option<i64>) -> Result<Option<DashboardLayout>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let query = if let Some(id) = layout_id {
        format!(
            "SELECT id, name, columns, widgets_json, is_default FROM pp_widget_layout WHERE id = {}",
            id
        )
    } else {
        "SELECT id, name, columns, widgets_json, is_default FROM pp_widget_layout WHERE is_default = 1 LIMIT 1".to_string()
    };

    let result = conn
        .query_row(&query, [], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let columns: u32 = row.get(2)?;
            let widgets_json: String = row.get(3)?;
            let is_default: bool = row.get::<_, i64>(4)? == 1;

            let widgets: Vec<WidgetConfig> =
                serde_json::from_str(&widgets_json).unwrap_or_default();

            Ok(DashboardLayout {
                id,
                name,
                columns,
                widgets,
                is_default,
            })
        })
        .ok();

    Ok(result)
}

/// Save dashboard layout (internal function)
fn save_dashboard_layout_internal(
    conn: &rusqlite::Connection,
    layout: &DashboardLayout,
) -> Result<i64, String> {
    let widgets_json =
        serde_json::to_string(&layout.widgets).map_err(|e| format!("JSON error: {}", e))?;

    if layout.id > 0 {
        // Update existing
        conn.execute(
            "UPDATE pp_widget_layout SET name = ?1, columns = ?2, widgets_json = ?3, is_default = ?4, updated_at = CURRENT_TIMESTAMP WHERE id = ?5",
            rusqlite::params![
                layout.name,
                layout.columns,
                widgets_json,
                if layout.is_default { 1 } else { 0 },
                layout.id
            ],
        )
        .map_err(|e| format!("Update error: {}", e))?;

        Ok(layout.id)
    } else {
        // Insert new
        conn.execute(
            "INSERT INTO pp_widget_layout (name, columns, widgets_json, is_default) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                layout.name,
                layout.columns,
                widgets_json,
                if layout.is_default { 1 } else { 0 }
            ],
        )
        .map_err(|e| format!("Insert error: {}", e))?;

        Ok(conn.last_insert_rowid())
    }
}

/// Save dashboard layout
#[tauri::command]
pub fn save_dashboard_layout(layout: DashboardLayout) -> Result<i64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    save_dashboard_layout_internal(conn, &layout)
}

/// Delete dashboard layout
#[tauri::command]
pub fn delete_dashboard_layout(layout_id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_widget_layout WHERE id = ?1", [layout_id])
        .map_err(|e| format!("Delete error: {}", e))?;

    Ok(())
}

/// Get all dashboard layouts
#[tauri::command]
pub fn get_all_dashboard_layouts() -> Result<Vec<DashboardLayout>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, name, columns, widgets_json, is_default FROM pp_widget_layout ORDER BY is_default DESC, name")
        .map_err(|e| e.to_string())?;

    let layouts = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let columns: u32 = row.get(2)?;
            let widgets_json: String = row.get(3)?;
            let is_default: bool = row.get::<_, i64>(4)? == 1;

            let widgets: Vec<WidgetConfig> =
                serde_json::from_str(&widgets_json).unwrap_or_default();

            Ok(DashboardLayout {
                id,
                name,
                columns,
                widgets,
                is_default,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(layouts)
}

/// Create default dashboard layout
#[tauri::command]
pub fn create_default_dashboard_layout() -> Result<DashboardLayout, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let default_widgets = vec![
        WidgetConfig {
            id: "portfolio-value".to_string(),
            widget_type: WidgetType::PortfolioValue,
            title: Some("Depotwert".to_string()),
            position: Position { x: 0, y: 0 },
            size: Size { width: 2, height: 1 },
            settings: serde_json::json!({}),
        },
        WidgetConfig {
            id: "performance".to_string(),
            widget_type: WidgetType::Performance,
            title: Some("Performance".to_string()),
            position: Position { x: 2, y: 0 },
            size: Size { width: 2, height: 1 },
            settings: serde_json::json!({ "timeRange": "YTD" }),
        },
        WidgetConfig {
            id: "alerts".to_string(),
            widget_type: WidgetType::Alerts,
            title: Some("Warnungen".to_string()),
            position: Position { x: 4, y: 0 },
            size: Size { width: 2, height: 1 },
            settings: serde_json::json!({}),
        },
        WidgetConfig {
            id: "chart".to_string(),
            widget_type: WidgetType::Chart,
            title: Some("Portfolio-Entwicklung".to_string()),
            position: Position { x: 0, y: 1 },
            size: Size { width: 4, height: 2 },
            settings: serde_json::json!({ "timeRange": "1Y" }),
        },
        WidgetConfig {
            id: "holdings-table".to_string(),
            widget_type: WidgetType::HoldingsTable,
            title: Some("Bestände".to_string()),
            position: Position { x: 4, y: 1 },
            size: Size { width: 2, height: 2 },
            settings: serde_json::json!({ "limit": 10 }),
        },
    ];

    let layout = DashboardLayout {
        id: 0,
        name: "Standard".to_string(),
        columns: 6,
        widgets: default_widgets,
        is_default: true,
    };

    let id = save_dashboard_layout_internal(conn, &layout)?;

    Ok(DashboardLayout { id, ..layout })
}
