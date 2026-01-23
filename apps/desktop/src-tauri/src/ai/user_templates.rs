//! User-defined Query Templates
//!
//! Allows users to create custom SQL query templates for the ChatBot.
//! Only SELECT statements are allowed for security.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// User template stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTemplate {
    pub id: i64,
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub sql_query: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub parameters: Vec<UserTemplateParam>,
}

/// Parameter definition for user template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTemplateParam {
    pub id: Option<i64>,
    pub param_name: String,
    pub param_type: String, // "string", "number", "date", "year"
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
}

/// Input for creating/updating a user template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTemplateInput {
    pub name: String,
    pub description: String,
    pub sql_query: String,
    pub enabled: Option<bool>,
    pub parameters: Vec<UserTemplateParam>,
}

// ============================================================================
// SQL Validation (SECURITY CRITICAL!)
// ============================================================================

/// Validates user SQL before storage.
/// Only SELECT statements are allowed, and dangerous keywords are blocked.
pub fn validate_user_sql(sql: &str) -> Result<(), String> {
    let sql_upper = sql.to_uppercase();
    let sql_trimmed = sql_upper.trim();

    // Only SELECT allowed
    if !sql_trimmed.starts_with("SELECT") {
        return Err("Nur SELECT-Statements sind erlaubt".into());
    }

    // Forbidden keywords that could modify data or schema
    let forbidden = [
        "INSERT",
        "UPDATE",
        "DELETE",
        "DROP",
        "ALTER",
        "CREATE",
        "TRUNCATE",
        "REPLACE",
        "ATTACH",
        "DETACH",
        "PRAGMA",
        "VACUUM",
        "REINDEX",
        "ANALYZE", // Could be slow on large tables
    ];

    for kw in forbidden {
        // Check for keyword as word boundary (not part of identifier)
        // e.g., "SELECT * FROM my_update_table" should be allowed
        let patterns = [
            format!(" {} ", kw),
            format!(" {}\n", kw),
            format!(" {}\t", kw),
            format!("\n{} ", kw),
            format!("\t{} ", kw),
            format!("({})", kw),
            format!("({} ", kw),
            format!(" {})", kw),
        ];

        for pattern in &patterns {
            if sql_upper.contains(pattern) {
                return Err(format!("Verbotenes Keyword: {}", kw));
            }
        }

        // Also check at start/end
        if sql_trimmed.starts_with(&format!("{} ", kw))
            || sql_trimmed.starts_with(&format!("{}(", kw))
        {
            return Err(format!("Verbotenes Keyword: {}", kw));
        }
    }

    // No multiple statements (semicolon check)
    // Allow semicolon only at the very end
    let sql_no_trailing = sql.trim().trim_end_matches(';');
    if sql_no_trailing.contains(';') {
        return Err("Mehrere Statements sind nicht erlaubt".into());
    }

    // Basic syntax check - must have FROM clause
    if !sql_upper.contains("FROM") {
        return Err("SELECT muss eine FROM-Klausel enthalten".into());
    }

    Ok(())
}

/// Generates a valid template_id from the name
pub fn generate_template_id(name: &str) -> String {
    let cleaned: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' {
                '_'
            } else {
                '_'
            }
        })
        .collect();

    // Ensure prefix and clean up multiple underscores
    let mut result = format!("user_{}", cleaned);
    while result.contains("__") {
        result = result.replace("__", "_");
    }
    result.trim_end_matches('_').to_string()
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Get all user templates
pub fn get_all_user_templates(conn: &Connection) -> Result<Vec<UserTemplate>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, template_id, name, description, sql_query, enabled, created_at, updated_at
            FROM ai_user_template
            ORDER BY name
            "#,
        )
        .map_err(|e| format!("SQL-Fehler: {}", e))?;

    let templates: Vec<UserTemplate> = stmt
        .query_map([], |row| {
            Ok(UserTemplate {
                id: row.get(0)?,
                template_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                sql_query: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                parameters: Vec::new(), // Filled below
            })
        })
        .map_err(|e| format!("Query-Fehler: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // Load parameters for each template
    let mut result = Vec::new();
    for mut template in templates {
        template.parameters = get_template_params(conn, template.id)?;
        result.push(template);
    }

    Ok(result)
}

/// Get only enabled user templates (for ChatBot prompt)
pub fn get_enabled_user_templates(conn: &Connection) -> Result<Vec<UserTemplate>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, template_id, name, description, sql_query, enabled, created_at, updated_at
            FROM ai_user_template
            WHERE enabled = 1
            ORDER BY name
            "#,
        )
        .map_err(|e| format!("SQL-Fehler: {}", e))?;

    let templates: Vec<UserTemplate> = stmt
        .query_map([], |row| {
            Ok(UserTemplate {
                id: row.get(0)?,
                template_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                sql_query: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                parameters: Vec::new(),
            })
        })
        .map_err(|e| format!("Query-Fehler: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut result = Vec::new();
    for mut template in templates {
        template.parameters = get_template_params(conn, template.id)?;
        result.push(template);
    }

    Ok(result)
}

/// Get a user template by its template_id
pub fn get_user_template_by_id(
    conn: &Connection,
    template_id: &str,
) -> Result<UserTemplate, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, template_id, name, description, sql_query, enabled, created_at, updated_at
            FROM ai_user_template
            WHERE template_id = ?1
            "#,
        )
        .map_err(|e| format!("SQL-Fehler: {}", e))?;

    let mut template: UserTemplate = stmt
        .query_row(params![template_id], |row| {
            Ok(UserTemplate {
                id: row.get(0)?,
                template_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                sql_query: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                parameters: Vec::new(),
            })
        })
        .map_err(|_| format!("Template '{}' nicht gefunden", template_id))?;

    template.parameters = get_template_params(conn, template.id)?;
    Ok(template)
}

/// Get parameters for a template
fn get_template_params(conn: &Connection, template_db_id: i64) -> Result<Vec<UserTemplateParam>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, param_name, param_type, required, description, default_value
            FROM ai_user_template_param
            WHERE template_id = ?1
            ORDER BY id
            "#,
        )
        .map_err(|e| format!("SQL-Fehler: {}", e))?;

    let params: Vec<UserTemplateParam> = stmt
        .query_map(params![template_db_id], |row| {
            Ok(UserTemplateParam {
                id: Some(row.get(0)?),
                param_name: row.get(1)?,
                param_type: row.get(2)?,
                required: row.get::<_, i32>(3)? != 0,
                description: row.get(4)?,
                default_value: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query-Fehler: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(params)
}

/// Create a new user template
pub fn create_user_template(
    conn: &Connection,
    input: &UserTemplateInput,
) -> Result<UserTemplate, String> {
    // Validate SQL
    validate_user_sql(&input.sql_query)?;

    // Generate template_id
    let template_id = generate_template_id(&input.name);

    // Check for duplicates
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM ai_user_template WHERE template_id = ?1)",
            params![&template_id],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if exists {
        return Err(format!(
            "Template mit ID '{}' existiert bereits",
            template_id
        ));
    }

    let enabled = input.enabled.unwrap_or(true);

    // Insert template
    conn.execute(
        r#"
        INSERT INTO ai_user_template (template_id, name, description, sql_query, enabled)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        params![
            &template_id,
            &input.name,
            &input.description,
            &input.sql_query,
            enabled as i32
        ],
    )
    .map_err(|e| format!("Fehler beim Erstellen: {}", e))?;

    let id = conn.last_insert_rowid();

    // Insert parameters
    for param in &input.parameters {
        conn.execute(
            r#"
            INSERT INTO ai_user_template_param (template_id, param_name, param_type, required, description, default_value)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                id,
                &param.param_name,
                &param.param_type,
                param.required as i32,
                &param.description,
                &param.default_value
            ],
        )
        .map_err(|e| format!("Fehler beim Parameter: {}", e))?;
    }

    get_user_template_by_id(conn, &template_id)
}

/// Update an existing user template
pub fn update_user_template(
    conn: &Connection,
    id: i64,
    input: &UserTemplateInput,
) -> Result<UserTemplate, String> {
    // Validate SQL
    validate_user_sql(&input.sql_query)?;

    // Get current template_id
    let current_template_id: String = conn
        .query_row(
            "SELECT template_id FROM ai_user_template WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|_| "Template nicht gefunden".to_string())?;

    let enabled = input.enabled.unwrap_or(true);

    // Update template
    conn.execute(
        r#"
        UPDATE ai_user_template
        SET name = ?1, description = ?2, sql_query = ?3, enabled = ?4, updated_at = datetime('now')
        WHERE id = ?5
        "#,
        params![
            &input.name,
            &input.description,
            &input.sql_query,
            enabled as i32,
            id
        ],
    )
    .map_err(|e| format!("Fehler beim Aktualisieren: {}", e))?;

    // Delete old parameters and insert new ones
    conn.execute(
        "DELETE FROM ai_user_template_param WHERE template_id = ?1",
        params![id],
    )
    .map_err(|e| format!("Fehler beim Löschen der Parameter: {}", e))?;

    for param in &input.parameters {
        conn.execute(
            r#"
            INSERT INTO ai_user_template_param (template_id, param_name, param_type, required, description, default_value)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                id,
                &param.param_name,
                &param.param_type,
                param.required as i32,
                &param.description,
                &param.default_value
            ],
        )
        .map_err(|e| format!("Fehler beim Parameter: {}", e))?;
    }

    get_user_template_by_id(conn, &current_template_id)
}

/// Delete a user template
pub fn delete_user_template(conn: &Connection, id: i64) -> Result<(), String> {
    let affected = conn
        .execute("DELETE FROM ai_user_template WHERE id = ?1", params![id])
        .map_err(|e| format!("Fehler beim Löschen: {}", e))?;

    if affected == 0 {
        return Err("Template nicht gefunden".into());
    }

    Ok(())
}

// ============================================================================
// Query Execution
// ============================================================================

use super::query_templates::QueryResult;

/// Execute a user template with parameter substitution
pub fn execute_user_template(
    conn: &Connection,
    template: &UserTemplate,
    params: &HashMap<String, String>,
) -> Result<QueryResult, String> {
    // Validate SQL again at execution time (paranoia check)
    validate_user_sql(&template.sql_query)?;

    // Substitute named parameters :param_name with ? placeholders
    let (sql, values) = substitute_named_params(&template.sql_query, &template.parameters, params)?;

    // Execute query
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("SQL-Fehler: {}", e))?;

    // Get column names
    let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    // Execute with dynamic parameters
    let rows_result = match values.len() {
        0 => stmt.query([]),
        1 => stmt.query(params![values[0]]),
        2 => stmt.query(params![values[0], values[1]]),
        3 => stmt.query(params![values[0], values[1], values[2]]),
        4 => stmt.query(params![values[0], values[1], values[2], values[3]]),
        5 => stmt.query(params![
            values[0], values[1], values[2], values[3], values[4]
        ]),
        _ => return Err("Zu viele Parameter (max 5)".into()),
    };

    let mut rows_iter = rows_result.map_err(|e| format!("Query-Fehler: {}", e))?;

    let mut rows: Vec<HashMap<String, serde_json::Value>> = Vec::new();
    while let Some(row) = rows_iter.next().map_err(|e| format!("Row-Fehler: {}", e))? {
        let mut row_map = HashMap::new();
        for (i, col) in columns.iter().enumerate() {
            let value: serde_json::Value = match row.get_ref(i) {
                Ok(rusqlite::types::ValueRef::Null) => serde_json::Value::Null,
                Ok(rusqlite::types::ValueRef::Integer(n)) => serde_json::json!(n),
                Ok(rusqlite::types::ValueRef::Real(f)) => {
                    // Round to 4 decimal places for display
                    serde_json::json!((f * 10000.0).round() / 10000.0)
                }
                Ok(rusqlite::types::ValueRef::Text(t)) => {
                    serde_json::json!(String::from_utf8_lossy(t).to_string())
                }
                Ok(rusqlite::types::ValueRef::Blob(b)) => {
                    serde_json::json!(format!("<blob:{} bytes>", b.len()))
                }
                Err(_) => serde_json::Value::Null,
            };
            row_map.insert(col.clone(), value);
        }
        rows.push(row_map);
    }

    let row_count = rows.len();

    // Format as markdown table
    let formatted_markdown = format_as_markdown(&template.name, &columns, &rows);

    Ok(QueryResult {
        template_id: template.template_id.clone(),
        columns,
        rows,
        row_count,
        formatted_markdown,
    })
}

/// Substitute named parameters (:param_name) with ? placeholders
fn substitute_named_params(
    sql: &str,
    param_defs: &[UserTemplateParam],
    params: &HashMap<String, String>,
) -> Result<(String, Vec<String>), String> {
    let mut result_sql = sql.to_string();
    let mut values: Vec<String> = Vec::new();

    // Check required parameters
    for def in param_defs {
        if def.required && !params.contains_key(&def.param_name) {
            // Try default value
            if def.default_value.is_none() {
                return Err(format!("Parameter '{}' ist erforderlich", def.param_name));
            }
        }
    }

    // Replace :param_name with ? and collect values in order
    for def in param_defs {
        let placeholder = format!(":{}", def.param_name);
        if result_sql.contains(&placeholder) {
            let value = params
                .get(&def.param_name)
                .or(def.default_value.as_ref())
                .cloned()
                .unwrap_or_default();

            // Validate value type
            validate_param_value(&value, &def.param_type)?;

            result_sql = result_sql.replace(&placeholder, "?");
            values.push(value);
        }
    }

    Ok((result_sql, values))
}

/// Validate parameter value against expected type
fn validate_param_value(value: &str, param_type: &str) -> Result<(), String> {
    match param_type {
        "number" => {
            value
                .parse::<f64>()
                .map_err(|_| format!("'{}' ist keine gültige Zahl", value))?;
        }
        "year" => {
            let year: i32 = value
                .parse()
                .map_err(|_| format!("'{}' ist kein gültiges Jahr", value))?;
            if !(1900..=2100).contains(&year) {
                return Err(format!("Jahr '{}' außerhalb des gültigen Bereichs", value));
            }
        }
        "date" => {
            // Basic date format check YYYY-MM-DD
            if value.len() != 10 || value.chars().nth(4) != Some('-') || value.chars().nth(7) != Some('-') {
                return Err(format!("'{}' ist kein gültiges Datum (YYYY-MM-DD)", value));
            }
        }
        _ => {} // "string" and others - no validation
    }
    Ok(())
}

/// Format query result as markdown table
fn format_as_markdown(
    name: &str,
    columns: &[String],
    rows: &[HashMap<String, serde_json::Value>],
) -> String {
    if rows.is_empty() {
        return format!("### {}\n\nKeine Ergebnisse gefunden.", name);
    }

    let mut md = format!("### {}\n\n", name);

    // Header
    md.push_str("| ");
    for col in columns {
        md.push_str(col);
        md.push_str(" | ");
    }
    md.push('\n');

    // Separator
    md.push_str("| ");
    for _ in columns {
        md.push_str("--- | ");
    }
    md.push('\n');

    // Rows (limit to 50 for readability)
    for row in rows.iter().take(50) {
        md.push_str("| ");
        for col in columns {
            let value = row.get(col).unwrap_or(&serde_json::Value::Null);
            let display = match value {
                serde_json::Value::Null => "-".to_string(),
                serde_json::Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        format!("{:.2}", f)
                    } else {
                        n.to_string()
                    }
                }
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            md.push_str(&display);
            md.push_str(" | ");
        }
        md.push('\n');
    }

    if rows.len() > 50 {
        md.push_str(&format!("\n*...und {} weitere Zeilen*\n", rows.len() - 50));
    }

    md.push_str(&format!("\n**{} Ergebnisse**", rows.len()));

    md
}

// ============================================================================
// Helper for checking if a template_id is user-defined
// ============================================================================

/// Check if a template_id refers to a user-defined template
pub fn is_user_template(template_id: &str) -> bool {
    template_id.starts_with("user_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_user_sql_valid() {
        assert!(validate_user_sql("SELECT * FROM pp_security").is_ok());
        assert!(validate_user_sql("SELECT name, ticker FROM pp_security WHERE id = ?1").is_ok());
        assert!(validate_user_sql(
            "SELECT s.name FROM pp_security s JOIN pp_txn t ON s.id = t.security_id"
        )
        .is_ok());
    }

    #[test]
    fn test_validate_user_sql_invalid() {
        assert!(validate_user_sql("INSERT INTO pp_security (name) VALUES ('test')").is_err());
        assert!(validate_user_sql("UPDATE pp_security SET name = 'test'").is_err());
        assert!(validate_user_sql("DELETE FROM pp_security").is_err());
        assert!(validate_user_sql("DROP TABLE pp_security").is_err());
        assert!(validate_user_sql("SELECT * FROM pp_security; DROP TABLE pp_security").is_err());
        assert!(validate_user_sql("PRAGMA table_info(pp_security)").is_err());
    }

    #[test]
    fn test_generate_template_id() {
        assert_eq!(generate_template_id("Top Performer"), "user_top_performer");
        assert_eq!(
            generate_template_id("Meine Dividenden"),
            "user_meine_dividenden"
        );
        assert_eq!(
            generate_template_id("Test--Template  Name"),
            "user_test_template_name"
        );
    }
}
