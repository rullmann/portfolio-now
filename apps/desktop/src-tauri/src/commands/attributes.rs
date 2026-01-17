//! Custom attribute management commands
//!
//! Handles attribute type definitions and attribute values on securities/accounts.

use serde::{Deserialize, Serialize};
use tauri::command;
use uuid::Uuid;

use crate::db;

/// Attribute type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeType {
    pub id: i64,
    pub uuid: String,
    pub name: String,
    pub column_label: Option<String>,
    pub target: String,       // "security", "account", "portfolio"
    pub data_type: String,    // "STRING", "LONG_NUMBER", "DOUBLE_NUMBER", "DATE", "BOOLEAN"
    pub converter_class: Option<String>,
    pub source: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Request to create a new attribute type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAttributeTypeRequest {
    pub name: String,
    pub column_label: Option<String>,
    pub target: Option<String>,    // defaults to "security"
    pub data_type: Option<String>, // defaults to "STRING"
    pub converter_class: Option<String>,
    pub source: Option<String>,
}

/// Request to update an attribute type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAttributeTypeRequest {
    pub name: Option<String>,
    pub column_label: Option<String>,
    pub data_type: Option<String>,
    pub converter_class: Option<String>,
    pub source: Option<String>,
}

/// Attribute value for a security
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeValue {
    pub attribute_type_id: i64,
    pub attribute_type_name: String,
    pub attribute_type_uuid: String,
    pub data_type: String,
    pub value: Option<String>,
}

/// Request to set an attribute value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAttributeValueRequest {
    pub security_id: i64,
    pub attribute_type_id: i64,
    pub value: String,
}

// ============================================================================
// Attribute Type Commands
// ============================================================================

/// Get all attribute types, optionally filtered by target
#[command]
pub fn get_attribute_types(target: Option<String>) -> Result<Vec<AttributeType>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let (query, params): (&str, Vec<&dyn rusqlite::ToSql>) = if let Some(ref t) = target {
        (
            "SELECT id, uuid, name, column_label, target, data_type, converter_class, source, created_at, updated_at
             FROM pp_attribute_type WHERE target = ? ORDER BY name",
            vec![t as &dyn rusqlite::ToSql],
        )
    } else {
        (
            "SELECT id, uuid, name, column_label, target, data_type, converter_class, source, created_at, updated_at
             FROM pp_attribute_type ORDER BY name",
            vec![],
        )
    };

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params.as_slice(), |row| {
            Ok(AttributeType {
                id: row.get(0)?,
                uuid: row.get(1)?,
                name: row.get(2)?,
                column_label: row.get(3)?,
                target: row.get(4)?,
                data_type: row.get(5)?,
                converter_class: row.get(6)?,
                source: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }

    Ok(result)
}

/// Create a new attribute type
#[command]
pub fn create_attribute_type(request: CreateAttributeTypeRequest) -> Result<AttributeType, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let uuid = Uuid::new_v4().to_string();
    let target = request.target.unwrap_or_else(|| "security".to_string());
    let data_type = request.data_type.unwrap_or_else(|| "STRING".to_string());

    conn.execute(
        "INSERT INTO pp_attribute_type (uuid, name, column_label, target, data_type, converter_class, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            uuid,
            request.name,
            request.column_label,
            target,
            data_type,
            request.converter_class,
            request.source,
        ],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    // Fetch the created record
    let attr_type = conn
        .query_row(
            "SELECT id, uuid, name, column_label, target, data_type, converter_class, source, created_at, updated_at
             FROM pp_attribute_type WHERE id = ?",
            [id],
            |row| {
                Ok(AttributeType {
                    id: row.get(0)?,
                    uuid: row.get(1)?,
                    name: row.get(2)?,
                    column_label: row.get(3)?,
                    target: row.get(4)?,
                    data_type: row.get(5)?,
                    converter_class: row.get(6)?,
                    source: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(attr_type)
}

/// Update an existing attribute type
#[command]
pub fn update_attribute_type(
    id: i64,
    request: UpdateAttributeTypeRequest,
) -> Result<AttributeType, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Build dynamic UPDATE query
    let mut updates = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(name) = &request.name {
        updates.push("name = ?");
        params.push(Box::new(name.clone()));
    }
    if let Some(column_label) = &request.column_label {
        updates.push("column_label = ?");
        params.push(Box::new(column_label.clone()));
    }
    if let Some(data_type) = &request.data_type {
        updates.push("data_type = ?");
        params.push(Box::new(data_type.clone()));
    }
    if let Some(converter_class) = &request.converter_class {
        updates.push("converter_class = ?");
        params.push(Box::new(converter_class.clone()));
    }
    if let Some(source) = &request.source {
        updates.push("source = ?");
        params.push(Box::new(source.clone()));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    updates.push("updated_at = datetime('now')");
    params.push(Box::new(id));

    let query = format!(
        "UPDATE pp_attribute_type SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| e.to_string())?;

    // Fetch updated record
    let attr_type = conn
        .query_row(
            "SELECT id, uuid, name, column_label, target, data_type, converter_class, source, created_at, updated_at
             FROM pp_attribute_type WHERE id = ?",
            [id],
            |row| {
                Ok(AttributeType {
                    id: row.get(0)?,
                    uuid: row.get(1)?,
                    name: row.get(2)?,
                    column_label: row.get(3)?,
                    target: row.get(4)?,
                    data_type: row.get(5)?,
                    converter_class: row.get(6)?,
                    source: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(attr_type)
}

/// Delete an attribute type
#[command]
pub fn delete_attribute_type(id: i64) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Get the attribute type UUID for cleanup
    let uuid: String = conn
        .query_row("SELECT uuid FROM pp_attribute_type WHERE id = ?", [id], |row| row.get(0))
        .map_err(|e| format!("Attribute type not found: {}", e))?;

    // Remove this attribute from all securities that have it
    // The attributes are stored as JSON in pp_security.attributes
    // We need to update all securities to remove this key
    let securities: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, attributes FROM pp_security WHERE attributes IS NOT NULL AND attributes != ''")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    for (sec_id, attrs_json) in securities {
        if let Ok(mut attrs) = serde_json::from_str::<std::collections::HashMap<String, String>>(&attrs_json) {
            if attrs.remove(&uuid).is_some() {
                let new_json = serde_json::to_string(&attrs).unwrap_or_default();
                conn.execute(
                    "UPDATE pp_security SET attributes = ? WHERE id = ?",
                    rusqlite::params![new_json, sec_id],
                )
                .ok();
            }
        }
    }

    // Delete the attribute type
    conn.execute("DELETE FROM pp_attribute_type WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Security Attribute Value Commands
// ============================================================================

/// Get all attribute values for a security
#[command]
pub fn get_security_attributes(security_id: i64) -> Result<Vec<AttributeValue>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Get the security's attributes JSON
    let attrs_json: Option<String> = conn
        .query_row(
            "SELECT attributes FROM pp_security WHERE id = ?",
            [security_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Security not found: {}", e))?;

    let attrs: std::collections::HashMap<String, String> = attrs_json
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    // Get all attribute types for securities
    let attr_types = get_attribute_types(Some("security".to_string()))?;

    // Build result with values (None if not set)
    let result: Vec<AttributeValue> = attr_types
        .into_iter()
        .map(|at| AttributeValue {
            attribute_type_id: at.id,
            attribute_type_name: at.name,
            attribute_type_uuid: at.uuid.clone(),
            data_type: at.data_type,
            value: attrs.get(&at.uuid).cloned(),
        })
        .collect();

    Ok(result)
}

/// Set an attribute value for a security
#[command]
pub fn set_security_attribute(request: SetAttributeValueRequest) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Get the attribute type UUID
    let attr_uuid: String = conn
        .query_row(
            "SELECT uuid FROM pp_attribute_type WHERE id = ?",
            [request.attribute_type_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Attribute type not found: {}", e))?;

    // Get current attributes
    let attrs_json: Option<String> = conn
        .query_row(
            "SELECT attributes FROM pp_security WHERE id = ?",
            [request.security_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Security not found: {}", e))?;

    let mut attrs: std::collections::HashMap<String, String> = attrs_json
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    // Set the value
    attrs.insert(attr_uuid, request.value);

    // Save back
    let new_json = serde_json::to_string(&attrs).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE pp_security SET attributes = ? WHERE id = ?",
        rusqlite::params![new_json, request.security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove an attribute value from a security
#[command]
pub fn remove_security_attribute(security_id: i64, attribute_type_id: i64) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Get the attribute type UUID
    let attr_uuid: String = conn
        .query_row(
            "SELECT uuid FROM pp_attribute_type WHERE id = ?",
            [attribute_type_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Attribute type not found: {}", e))?;

    // Get current attributes
    let attrs_json: Option<String> = conn
        .query_row(
            "SELECT attributes FROM pp_security WHERE id = ?",
            [security_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Security not found: {}", e))?;

    let mut attrs: std::collections::HashMap<String, String> = attrs_json
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();

    // Remove the value
    attrs.remove(&attr_uuid);

    // Save back
    let new_json = serde_json::to_string(&attrs).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE pp_security SET attributes = ? WHERE id = ?",
        rusqlite::params![new_json, security_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get all securities with their attribute values for a specific attribute type
#[command]
pub fn get_securities_by_attribute(attribute_type_id: i64) -> Result<Vec<(i64, String, Option<String>)>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Get the attribute type UUID
    let attr_uuid: String = conn
        .query_row(
            "SELECT uuid FROM pp_attribute_type WHERE id = ?",
            [attribute_type_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Attribute type not found: {}", e))?;

    // Get all securities with this attribute
    let mut stmt = conn
        .prepare("SELECT id, name, attributes FROM pp_security WHERE is_retired = 0 ORDER BY name")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let attrs_json: Option<String> = row.get(2)?;
            Ok((id, name, attrs_json))
        })
        .map_err(|e| e.to_string())?;

    let result: Vec<(i64, String, Option<String>)> = rows
        .filter_map(|r| r.ok())
        .map(|(id, name, attrs_json)| {
            let value = attrs_json
                .and_then(|json| serde_json::from_str::<std::collections::HashMap<String, String>>(&json).ok())
                .and_then(|attrs| attrs.get(&attr_uuid).cloned());
            (id, name, value)
        })
        .collect();

    Ok(result)
}
