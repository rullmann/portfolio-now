//! Taxonomy management commands for Tauri
//!
//! Taxonomies provide hierarchical classification of securities for analysis.

use crate::db;
use serde::{Deserialize, Serialize};
use tauri::command;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxonomyData {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub source: Option<String>,
    pub classifications_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationData {
    pub id: i64,
    pub uuid: String,
    pub taxonomy_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    /// Weight in basis points (10000 = 100%)
    pub weight: Option<i32>,
    pub rank: Option<i32>,
    pub children: Vec<ClassificationData>,
    pub assignments_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationAssignmentData {
    pub id: i64,
    pub classification_id: i64,
    pub classification_name: String,
    pub vehicle_type: String,
    pub vehicle_uuid: String,
    pub vehicle_name: String,
    /// Weight in basis points (10000 = 100%)
    pub weight: i32,
    pub rank: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxonomyAllocation {
    pub classification_id: i64,
    pub classification_name: String,
    pub color: Option<String>,
    pub path: Vec<String>,
    /// Value in base currency (scaled by 100)
    pub value: i64,
    /// Percentage of total (0.0 - 100.0)
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaxonomyRequest {
    pub name: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaxonomyRequest {
    pub name: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateClassificationRequest {
    pub taxonomy_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    pub weight: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateClassificationRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub weight: Option<i32>,
    pub parent_id: Option<i64>,
    pub rank: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignSecurityRequest {
    pub classification_id: i64,
    pub security_id: i64,
    /// Weight in basis points (10000 = 100%)
    pub weight: i32,
}

// ============================================================================
// Taxonomy CRUD
// ============================================================================

/// Get all taxonomies
#[command]
pub fn get_taxonomies() -> Result<Vec<TaxonomyData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                t.id, t.uuid, t.name, t.source,
                (SELECT COUNT(*) FROM pp_classification WHERE taxonomy_id = t.id) as cnt
            FROM pp_taxonomy t
            ORDER BY t.name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(TaxonomyData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                source: row.get(3)?,
                classifications_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Get a single taxonomy with its full classification tree
#[command]
pub fn get_taxonomy(id: i64) -> Result<TaxonomyData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.query_row(
        r#"
        SELECT
            t.id, t.uuid, t.name, t.source,
            (SELECT COUNT(*) FROM pp_classification WHERE taxonomy_id = t.id) as cnt
        FROM pp_taxonomy t
        WHERE t.id = ?
        "#,
        [id],
        |row| {
            Ok(TaxonomyData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                source: row.get(3)?,
                classifications_count: row.get(4)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// Create a new taxonomy
#[command]
pub fn create_taxonomy(data: CreateTaxonomyRequest) -> Result<TaxonomyData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let uuid = uuid::Uuid::new_v4().to_string();

    // Get import_id
    let import_id: i64 = conn
        .query_row("SELECT id FROM pp_import ORDER BY id DESC LIMIT 1", [], |r| r.get(0))
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO pp_taxonomy (import_id, uuid, name, source) VALUES (?, ?, ?, ?)",
        rusqlite::params![import_id, uuid, data.name, data.source],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Create root classification
    let root_uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO pp_classification (taxonomy_id, uuid, parent_id, name, color, weight) VALUES (?, ?, NULL, ?, NULL, 10000)",
        rusqlite::params![id, root_uuid, data.name],
    )
    .map_err(|e| e.to_string())?;

    Ok(TaxonomyData {
        id,
        uuid,
        name: data.name,
        source: data.source,
        classifications_count: 1,
    })
}

/// Update a taxonomy
#[command]
pub fn update_taxonomy(id: i64, data: UpdateTaxonomyRequest) -> Result<TaxonomyData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref name) = data.name {
        updates.push("name = ?");
        params.push(Box::new(name.clone()));
    }
    if let Some(ref source) = data.source {
        updates.push("source = ?");
        params.push(Box::new(source.clone()));
    }

    if updates.is_empty() {
        return get_taxonomy(id);
    }

    params.push(Box::new(id));
    let sql = format!("UPDATE pp_taxonomy SET {} WHERE id = ?", updates.join(", "));

    conn.execute(
        &sql,
        rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
    )
    .map_err(|e| e.to_string())?;

    get_taxonomy(id)
}

/// Delete a taxonomy and all its classifications
#[command]
pub fn delete_taxonomy(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_taxonomy WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Classification CRUD
// ============================================================================

/// Get all classifications for a taxonomy as a flat list
#[command]
pub fn get_classifications(taxonomy_id: i64) -> Result<Vec<ClassificationData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                c.id, c.uuid, c.taxonomy_id, c.parent_id, c.name, c.color, c.weight, c.rank,
                (SELECT COUNT(*) FROM pp_classification_assignment WHERE classification_id = c.id) as cnt
            FROM pp_classification c
            WHERE c.taxonomy_id = ?
            ORDER BY c.rank, c.name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([taxonomy_id], |row| {
            Ok(ClassificationData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                taxonomy_id: row.get(2)?,
                parent_id: row.get(3)?,
                name: row.get(4)?,
                color: row.get(5)?,
                weight: row.get(6)?,
                rank: row.get(7)?,
                children: Vec::new(),
                assignments_count: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Get classifications as a tree structure
#[command]
pub fn get_classification_tree(taxonomy_id: i64) -> Result<Vec<ClassificationData>, String> {
    let flat = get_classifications(taxonomy_id)?;
    Ok(build_tree(flat))
}

fn build_tree(flat: Vec<ClassificationData>) -> Vec<ClassificationData> {
    use std::collections::HashMap;

    let mut by_id: HashMap<i64, ClassificationData> = flat.into_iter().map(|c| (c.id, c)).collect();
    let mut roots = Vec::new();

    // Collect parent-child relationships
    let parent_map: Vec<(i64, Option<i64>)> = by_id.iter().map(|(id, c)| (*id, c.parent_id)).collect();

    for (id, parent_id) in parent_map {
        if let Some(pid) = parent_id {
            if let Some(child) = by_id.remove(&id) {
                if let Some(parent) = by_id.get_mut(&pid) {
                    parent.children.push(child);
                }
            }
        }
    }

    // Collect roots (no parent)
    for (_, c) in by_id {
        roots.push(c);
    }

    // Sort children recursively
    fn sort_children(node: &mut ClassificationData) {
        node.children.sort_by(|a, b| {
            a.rank.cmp(&b.rank).then_with(|| a.name.cmp(&b.name))
        });
        for child in &mut node.children {
            sort_children(child);
        }
    }

    for root in &mut roots {
        sort_children(root);
    }

    roots
}

/// Create a new classification
#[command]
pub fn create_classification(data: CreateClassificationRequest) -> Result<ClassificationData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let uuid = uuid::Uuid::new_v4().to_string();

    // Get max rank for siblings
    let max_rank: Option<i32> = conn
        .query_row(
            "SELECT MAX(rank) FROM pp_classification WHERE taxonomy_id = ? AND parent_id IS ?",
            rusqlite::params![data.taxonomy_id, data.parent_id],
            |row| row.get(0),
        )
        .ok();

    let rank = max_rank.unwrap_or(0) + 1;

    conn.execute(
        r#"
        INSERT INTO pp_classification (taxonomy_id, uuid, parent_id, name, color, weight, rank)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
        rusqlite::params![
            data.taxonomy_id,
            uuid,
            data.parent_id,
            data.name,
            data.color,
            data.weight.unwrap_or(10000),
            rank
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(ClassificationData {
        id,
        uuid,
        taxonomy_id: data.taxonomy_id,
        parent_id: data.parent_id,
        name: data.name,
        color: data.color,
        weight: Some(data.weight.unwrap_or(10000)),
        rank: Some(rank),
        children: Vec::new(),
        assignments_count: 0,
    })
}

/// Update a classification
#[command]
pub fn update_classification(id: i64, data: UpdateClassificationRequest) -> Result<ClassificationData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref name) = data.name {
        updates.push("name = ?");
        params.push(Box::new(name.clone()));
    }
    if let Some(ref color) = data.color {
        updates.push("color = ?");
        params.push(Box::new(color.clone()));
    }
    if let Some(weight) = data.weight {
        updates.push("weight = ?");
        params.push(Box::new(weight));
    }
    if let Some(parent_id) = data.parent_id {
        updates.push("parent_id = ?");
        params.push(Box::new(parent_id));
    }
    if let Some(rank) = data.rank {
        updates.push("rank = ?");
        params.push(Box::new(rank));
    }

    if !updates.is_empty() {
        params.push(Box::new(id));
        let sql = format!("UPDATE pp_classification SET {} WHERE id = ?", updates.join(", "));

        conn.execute(
            &sql,
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
        )
        .map_err(|e| e.to_string())?;
    }

    // Fetch updated classification
    conn.query_row(
        r#"
        SELECT
            c.id, c.uuid, c.taxonomy_id, c.parent_id, c.name, c.color, c.weight, c.rank,
            (SELECT COUNT(*) FROM pp_classification_assignment WHERE classification_id = c.id) as cnt
        FROM pp_classification c
        WHERE c.id = ?
        "#,
        [id],
        |row| {
            Ok(ClassificationData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                taxonomy_id: row.get(2)?,
                parent_id: row.get(3)?,
                name: row.get(4)?,
                color: row.get(5)?,
                weight: row.get(6)?,
                rank: row.get(7)?,
                children: Vec::new(),
                assignments_count: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// Delete a classification (moves children to parent)
#[command]
pub fn delete_classification(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get parent_id of the classification being deleted
    let parent_id: Option<i64> = conn
        .query_row(
            "SELECT parent_id FROM pp_classification WHERE id = ?",
            [id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Move children to parent
    conn.execute(
        "UPDATE pp_classification SET parent_id = ? WHERE parent_id = ?",
        rusqlite::params![parent_id, id],
    )
    .map_err(|e| e.to_string())?;

    // Delete assignments
    conn.execute(
        "DELETE FROM pp_classification_assignment WHERE classification_id = ?",
        [id],
    )
    .map_err(|e| e.to_string())?;

    // Delete the classification
    conn.execute("DELETE FROM pp_classification WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Assignment CRUD
// ============================================================================

/// Get all assignments for a classification
#[command]
pub fn get_classification_assignments(classification_id: i64) -> Result<Vec<ClassificationAssignmentData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                a.id, a.classification_id, c.name, a.vehicle_type, a.vehicle_uuid, a.weight, a.rank,
                CASE a.vehicle_type
                    WHEN 'security' THEN (SELECT name FROM pp_security WHERE uuid = a.vehicle_uuid)
                    WHEN 'account' THEN (SELECT name FROM pp_account WHERE uuid = a.vehicle_uuid)
                END as vehicle_name
            FROM pp_classification_assignment a
            JOIN pp_classification c ON c.id = a.classification_id
            WHERE a.classification_id = ?
            ORDER BY a.rank, vehicle_name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([classification_id], |row| {
            Ok(ClassificationAssignmentData {
                id: row.get(0)?,
                classification_id: row.get(1)?,
                classification_name: row.get(2)?,
                vehicle_type: row.get(3)?,
                vehicle_uuid: row.get(4)?,
                weight: row.get(5)?,
                rank: row.get(6)?,
                vehicle_name: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Get all assignments for a security across all taxonomies
#[command]
pub fn get_security_assignments(security_id: i64) -> Result<Vec<ClassificationAssignmentData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get security UUID
    let security_uuid: String = conn
        .query_row("SELECT uuid FROM pp_security WHERE id = ?", [security_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                a.id, a.classification_id, c.name, a.vehicle_type, a.vehicle_uuid, a.weight, a.rank,
                s.name as vehicle_name
            FROM pp_classification_assignment a
            JOIN pp_classification c ON c.id = a.classification_id
            JOIN pp_security s ON s.uuid = a.vehicle_uuid
            WHERE a.vehicle_type = 'security' AND a.vehicle_uuid = ?
            ORDER BY c.name
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([&security_uuid], |row| {
            Ok(ClassificationAssignmentData {
                id: row.get(0)?,
                classification_id: row.get(1)?,
                classification_name: row.get(2)?,
                vehicle_type: row.get(3)?,
                vehicle_uuid: row.get(4)?,
                weight: row.get(5)?,
                rank: row.get(6)?,
                vehicle_name: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Assign a security to a classification
#[command]
pub fn assign_security(data: AssignSecurityRequest) -> Result<ClassificationAssignmentData, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get security UUID and name
    let (security_uuid, security_name): (String, String) = conn
        .query_row(
            "SELECT uuid, name FROM pp_security WHERE id = ?",
            [data.security_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    // Get classification name
    let classification_name: String = conn
        .query_row(
            "SELECT name FROM pp_classification WHERE id = ?",
            [data.classification_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Check if assignment already exists
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM pp_classification_assignment WHERE classification_id = ? AND vehicle_uuid = ?",
            rusqlite::params![data.classification_id, security_uuid],
            |row| row.get(0),
        )
        .ok();

    if let Some(existing_id) = existing {
        // Update existing
        conn.execute(
            "UPDATE pp_classification_assignment SET weight = ? WHERE id = ?",
            rusqlite::params![data.weight, existing_id],
        )
        .map_err(|e| e.to_string())?;

        return Ok(ClassificationAssignmentData {
            id: existing_id,
            classification_id: data.classification_id,
            classification_name,
            vehicle_type: "security".to_string(),
            vehicle_uuid: security_uuid,
            vehicle_name: security_name,
            weight: data.weight,
            rank: None,
        });
    }

    // Create new assignment
    conn.execute(
        r#"
        INSERT INTO pp_classification_assignment (classification_id, vehicle_type, vehicle_uuid, weight)
        VALUES (?, 'security', ?, ?)
        "#,
        rusqlite::params![data.classification_id, security_uuid, data.weight],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(ClassificationAssignmentData {
        id,
        classification_id: data.classification_id,
        classification_name,
        vehicle_type: "security".to_string(),
        vehicle_uuid: security_uuid,
        vehicle_name: security_name,
        weight: data.weight,
        rank: None,
    })
}

/// Remove a security assignment
#[command]
pub fn remove_assignment(id: i64) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute("DELETE FROM pp_classification_assignment WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Allocation Analysis
// ============================================================================

/// Calculate portfolio allocation by taxonomy
#[command]
pub fn get_taxonomy_allocation(taxonomy_id: i64, portfolio_id: Option<i64>) -> Result<Vec<TaxonomyAllocation>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Build holdings query with optional portfolio filter
    let holdings_query = if let Some(pid) = portfolio_id {
        format!(
            r#"
            SELECT s.uuid, SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares, lp.value as price
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
            WHERE t.owner_type = 'portfolio' AND t.owner_id = {} AND t.shares IS NOT NULL
            GROUP BY s.id
            HAVING net_shares > 0
            "#,
            pid
        )
    } else {
        r#"
            SELECT s.uuid, SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares, lp.value as price
            FROM pp_txn t
            JOIN pp_security s ON s.id = t.security_id
            LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
            WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
            GROUP BY s.id
            HAVING net_shares > 0
        "#
        .to_string()
    };

    // Get holdings with values
    let mut holdings: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut total_value: i64 = 0;

    {
        let mut stmt = conn.prepare(&holdings_query).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows.flatten() {
            let (uuid, shares, price) = row;
            if let Some(p) = price {
                // Value = (shares / 10^8) * (price / 10^8) * 100 (to get cents)
                let value = (shares as f64 / 100_000_000.0) * (p as f64 / 100_000_000.0) * 100.0;
                let value_cents = value as i64;
                holdings.insert(uuid, value_cents);
                total_value += value_cents;
            }
        }
    }

    if total_value == 0 {
        return Ok(Vec::new());
    }

    // Get classifications with paths
    let mut stmt = conn
        .prepare(
            r#"
            WITH RECURSIVE path_cte AS (
                SELECT id, name, color, parent_id, name as path
                FROM pp_classification
                WHERE taxonomy_id = ? AND parent_id IS NULL
                UNION ALL
                SELECT c.id, c.name, c.color, c.parent_id, p.path || ' > ' || c.name
                FROM pp_classification c
                JOIN path_cte p ON c.parent_id = p.id
            )
            SELECT c.id, c.name, c.color, c.path
            FROM path_cte c
            "#,
        )
        .map_err(|e| e.to_string())?;

    let classifications: Vec<(i64, String, Option<String>, String)> = stmt
        .query_map([taxonomy_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Calculate allocation per classification
    let mut allocations: Vec<TaxonomyAllocation> = Vec::new();

    for (class_id, class_name, color, path) in classifications {
        // Get assignments for this classification
        let mut stmt = conn
            .prepare(
                "SELECT vehicle_uuid, weight FROM pp_classification_assignment WHERE classification_id = ? AND vehicle_type = 'security'",
            )
            .map_err(|e| e.to_string())?;

        let assignments: Vec<(String, i32)> = stmt
            .query_map([class_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mut class_value: i64 = 0;
        for (uuid, weight) in assignments {
            if let Some(&holding_value) = holdings.get(&uuid) {
                // Apply weight (10000 = 100%)
                class_value += (holding_value as f64 * weight as f64 / 10000.0) as i64;
            }
        }

        if class_value > 0 {
            let percentage = (class_value as f64 / total_value as f64) * 100.0;
            let path_parts: Vec<String> = path.split(" > ").map(|s| s.to_string()).collect();

            allocations.push(TaxonomyAllocation {
                classification_id: class_id,
                classification_name: class_name,
                color,
                path: path_parts,
                value: class_value,
                percentage,
            });
        }
    }

    // Sort by value descending
    allocations.sort_by(|a, b| b.value.cmp(&a.value));

    Ok(allocations)
}

// ============================================================================
// Standard Taxonomies
// ============================================================================

/// Create standard asset class taxonomy
#[command]
pub fn create_standard_taxonomies() -> Result<Vec<TaxonomyData>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let import_id: i64 = conn
        .query_row("SELECT id FROM pp_import ORDER BY id DESC LIMIT 1", [], |r| r.get(0))
        .unwrap_or(1);

    let mut created = Vec::new();

    // Asset Classes
    let asset_classes = [
        ("Aktien", "#4CAF50"),
        ("Anleihen", "#2196F3"),
        ("Immobilien", "#FF9800"),
        ("Rohstoffe", "#795548"),
        ("Bargeld", "#9E9E9E"),
        ("Kryptowährungen", "#9C27B0"),
        ("Sonstiges", "#607D8B"),
    ];

    let tax_uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO pp_taxonomy (import_id, uuid, name, source) VALUES (?, ?, 'Asset-Klassen', 'user')",
        rusqlite::params![import_id, tax_uuid],
    )
    .map_err(|e| e.to_string())?;

    let tax_id = conn.last_insert_rowid();
    if tax_id > 0 {
        // Create root
        let root_uuid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO pp_classification (taxonomy_id, uuid, parent_id, name, weight) VALUES (?, ?, NULL, 'Asset-Klassen', 10000)",
            rusqlite::params![tax_id, root_uuid],
        )
        .map_err(|e| e.to_string())?;
        let root_id = conn.last_insert_rowid();

        for (i, (name, color)) in asset_classes.iter().enumerate() {
            let uuid = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO pp_classification (taxonomy_id, uuid, parent_id, name, color, rank) VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![tax_id, uuid, root_id, name, color, i + 1],
            )
            .map_err(|e| e.to_string())?;
        }

        created.push(TaxonomyData {
            id: tax_id,
            uuid: tax_uuid,
            name: "Asset-Klassen".to_string(),
            source: Some("user".to_string()),
            classifications_count: 8,
        });
    }

    // Regions
    let regions = [
        ("Nordamerika", "#1976D2"),
        ("Europa", "#388E3C"),
        ("Asien-Pazifik", "#F57C00"),
        ("Schwellenländer", "#7B1FA2"),
        ("Global", "#455A64"),
    ];

    let tax_uuid = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO pp_taxonomy (import_id, uuid, name, source) VALUES (?, ?, 'Regionen', 'user')",
        rusqlite::params![import_id, tax_uuid],
    )
    .map_err(|e| e.to_string())?;

    let tax_id = conn.last_insert_rowid();
    if tax_id > 0 {
        let root_uuid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO pp_classification (taxonomy_id, uuid, parent_id, name, weight) VALUES (?, ?, NULL, 'Regionen', 10000)",
            rusqlite::params![tax_id, root_uuid],
        )
        .map_err(|e| e.to_string())?;
        let root_id = conn.last_insert_rowid();

        for (i, (name, color)) in regions.iter().enumerate() {
            let uuid = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO pp_classification (taxonomy_id, uuid, parent_id, name, color, rank) VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![tax_id, uuid, root_id, name, color, i + 1],
            )
            .map_err(|e| e.to_string())?;
        }

        created.push(TaxonomyData {
            id: tax_id,
            uuid: tax_uuid,
            name: "Regionen".to_string(),
            source: Some("user".to_string()),
            classifications_count: 6,
        });
    }

    Ok(created)
}
