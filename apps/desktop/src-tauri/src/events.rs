//! Tauri event types for notifying the frontend of data changes.
//!
//! When backend operations modify data (transactions, imports, etc.),
//! emit these events to trigger frontend cache invalidation.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// Event name constant
pub const DATA_CHANGED_EVENT: &str = "data_changed";

/// Payload for data change events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataChangedPayload {
    /// The entity type that changed (e.g., "transaction", "security", "holding")
    pub entity: String,
    /// The action that occurred (e.g., "created", "updated", "deleted", "imported")
    pub action: String,
    /// Optional: affected security IDs (for targeted cache invalidation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_ids: Option<Vec<i64>>,
    /// Optional: affected portfolio IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portfolio_ids: Option<Vec<i64>>,
}

impl DataChangedPayload {
    /// Create a transaction change event
    pub fn transaction(action: &str, security_id: Option<i64>) -> Self {
        Self {
            entity: "transaction".to_string(),
            action: action.to_string(),
            security_ids: security_id.map(|id| vec![id]),
            portfolio_ids: None,
        }
    }

    /// Create a bulk import event
    pub fn import(security_ids: Vec<i64>) -> Self {
        Self {
            entity: "transaction".to_string(),
            action: "imported".to_string(),
            security_ids: if security_ids.is_empty() {
                None
            } else {
                Some(security_ids)
            },
            portfolio_ids: None,
        }
    }

    /// Create a rebalancing event
    pub fn rebalance(security_ids: Vec<i64>) -> Self {
        Self {
            entity: "transaction".to_string(),
            action: "rebalanced".to_string(),
            security_ids: if security_ids.is_empty() {
                None
            } else {
                Some(security_ids)
            },
            portfolio_ids: None,
        }
    }

    /// Create an investment plan execution event
    pub fn investment_plan_executed(security_id: i64) -> Self {
        Self {
            entity: "transaction".to_string(),
            action: "plan_executed".to_string(),
            security_ids: Some(vec![security_id]),
            portfolio_ids: None,
        }
    }
}

/// Emit a data changed event to the frontend
pub fn emit_data_changed(app: &AppHandle, payload: DataChangedPayload) {
    if let Err(e) = app.emit(DATA_CHANGED_EVENT, payload) {
        log::warn!("Failed to emit data_changed event: {}", e);
    }
}
