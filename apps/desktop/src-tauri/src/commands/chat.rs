//! Chat history persistence commands.
//!
//! Provides Tauri commands for saving, loading, and managing chat history
//! in SQLite for the portfolio assistant chatbot.
//!
//! Supports multiple conversations that users can switch between.

use crate::db;
use serde::{Deserialize, Serialize};
use tauri::command;

/// A chat conversation (session)
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub message_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// An image attachment stored with a chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredChatAttachment {
    pub data: String,      // Base64 encoded image data
    pub mime_type: String, // e.g., "image/png", "image/jpeg"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// A chat message from the database
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatHistoryMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub conversation_id: Option<i64>,
    /// Image attachments (empty array if none)
    #[serde(default)]
    pub attachments: Vec<StoredChatAttachment>,
}

/// Save a chat message to the database.
/// Returns the ID of the inserted message.
/// Also updates the conversation's updated_at timestamp.
/// Attachments are stored as JSON in the attachments_json column.
#[command]
pub fn save_chat_message(
    role: String,
    content: String,
    conversation_id: i64,
    attachments: Option<Vec<StoredChatAttachment>>,
) -> Result<i64, String> {
    // Validate role
    if role != "user" && role != "assistant" {
        return Err(format!("Invalid role: {}. Must be 'user' or 'assistant'", role));
    }

    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Serialize attachments to JSON if present
    let attachments_json: Option<String> = attachments
        .filter(|a| !a.is_empty())
        .map(|a| serde_json::to_string(&a).ok())
        .flatten();

    conn.execute(
        "INSERT INTO pp_chat_history (role, content, conversation_id, attachments_json) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![role, content, conversation_id, attachments_json],
    )
    .map_err(|e| format!("Failed to save chat message: {}", e))?;

    let id = conn.last_insert_rowid();

    // Update the conversation's updated_at timestamp
    conn.execute(
        "UPDATE pp_chat_conversation SET updated_at = datetime('now') WHERE id = ?1",
        [conversation_id],
    )
    .map_err(|e| format!("Failed to update conversation timestamp: {}", e))?;

    Ok(id)
}

/// Helper function to parse attachments from JSON
fn parse_attachments(json_str: Option<String>) -> Vec<StoredChatAttachment> {
    json_str
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Get chat history from the database for a specific conversation.
/// If limit is provided, returns only the last N messages.
/// Messages are returned in chronological order (oldest first).
/// Includes image attachments if stored.
#[command]
pub fn get_chat_history(conversation_id: i64, limit: Option<i64>) -> Result<Vec<ChatHistoryMessage>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let messages: Vec<ChatHistoryMessage> = if let Some(limit) = limit {
        // Get last N messages using a subquery to maintain chronological order
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, role, content, created_at, conversation_id, attachments_json
                FROM (
                    SELECT id, role, content, created_at, conversation_id, attachments_json
                    FROM pp_chat_history
                    WHERE conversation_id = ?1
                    ORDER BY created_at DESC, id DESC
                    LIMIT ?2
                )
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let rows = stmt.query_map(rusqlite::params![conversation_id, limit], |row| {
            let attachments_json: Option<String> = row.get(5)?;
            Ok(ChatHistoryMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                conversation_id: row.get(4)?,
                attachments: parse_attachments(attachments_json),
            })
        })
        .map_err(|e| format!("Failed to query chat history: {}", e))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect chat history: {}", e))?
    } else {
        // Get all messages in chronological order
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, role, content, created_at, conversation_id, attachments_json
                FROM pp_chat_history
                WHERE conversation_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let rows = stmt.query_map([conversation_id], |row| {
            let attachments_json: Option<String> = row.get(5)?;
            Ok(ChatHistoryMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                conversation_id: row.get(4)?,
                attachments: parse_attachments(attachments_json),
            })
        })
        .map_err(|e| format!("Failed to query chat history: {}", e))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect chat history: {}", e))?
    };

    Ok(messages)
}

/// Clear chat history for a specific conversation.
/// This deletes all messages but keeps the conversation itself.
#[command]
pub fn clear_chat_history(conversation_id: i64) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    conn.execute("DELETE FROM pp_chat_history WHERE conversation_id = ?1", [conversation_id])
        .map_err(|e| format!("Failed to clear chat history: {}", e))?;

    Ok(())
}

/// Delete a single chat message by ID.
#[command]
pub fn delete_chat_message(id: i64) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let rows_affected = conn
        .execute("DELETE FROM pp_chat_history WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete chat message: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Chat message with id {} not found", id));
    }

    Ok(())
}

// ============================================================================
// Chat Suggestions Persistence
// ============================================================================

/// A chat suggestion from the database
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSuggestion {
    pub id: i64,
    pub message_id: i64,
    pub action_type: String,
    pub description: String,
    pub payload: String,
    pub status: String, // "pending", "confirmed", "declined"
    pub created_at: String,
}

/// Save a chat suggestion to the database.
/// Returns the ID of the inserted suggestion.
#[command]
pub fn save_chat_suggestion(
    message_id: i64,
    conversation_id: i64,
    action_type: String,
    description: String,
    payload: String,
) -> Result<i64, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    conn.execute(
        "INSERT INTO pp_chat_suggestion (message_id, conversation_id, action_type, description, payload, status) VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
        rusqlite::params![message_id, conversation_id, action_type, description, payload],
    )
    .map_err(|e| format!("Failed to save chat suggestion: {}", e))?;

    let id = conn.last_insert_rowid();
    Ok(id)
}

/// Update the status of a chat suggestion.
#[command]
pub fn update_suggestion_status(id: i64, status: String) -> Result<(), String> {
    // Validate status
    if status != "pending" && status != "confirmed" && status != "declined" {
        return Err(format!("Invalid status: {}. Must be 'pending', 'confirmed', or 'declined'", status));
    }

    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let rows_affected = conn
        .execute(
            "UPDATE pp_chat_suggestion SET status = ?1 WHERE id = ?2",
            rusqlite::params![status, id],
        )
        .map_err(|e| format!("Failed to update suggestion status: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Suggestion with id {} not found", id));
    }

    Ok(())
}

/// Get all suggestions for a specific message.
#[command]
pub fn get_suggestions_for_message(message_id: i64) -> Result<Vec<ChatSuggestion>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, message_id, action_type, description, payload, status, created_at
            FROM pp_chat_suggestion
            WHERE message_id = ?1
            ORDER BY id ASC
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt.query_map([message_id], |row| {
        Ok(ChatSuggestion {
            id: row.get(0)?,
            message_id: row.get(1)?,
            action_type: row.get(2)?,
            description: row.get(3)?,
            payload: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
        })
    })
    .map_err(|e| format!("Failed to query suggestions: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect suggestions: {}", e))
}

/// Get all pending suggestions for a specific conversation.
#[command]
pub fn get_pending_suggestions(conversation_id: i64) -> Result<Vec<ChatSuggestion>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, message_id, action_type, description, payload, status, created_at
            FROM pp_chat_suggestion
            WHERE conversation_id = ?1 AND status = 'pending'
            ORDER BY id ASC
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt.query_map([conversation_id], |row| {
        Ok(ChatSuggestion {
            id: row.get(0)?,
            message_id: row.get(1)?,
            action_type: row.get(2)?,
            description: row.get(3)?,
            payload: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
        })
    })
    .map_err(|e| format!("Failed to query pending suggestions: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect pending suggestions: {}", e))
}

// ============================================================================
// Conversation Management
// ============================================================================

/// Get all conversations, sorted by updated_at (most recent first).
#[command]
pub fn get_conversations() -> Result<Vec<Conversation>, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                c.id,
                c.title,
                COALESCE((SELECT COUNT(*) FROM pp_chat_history WHERE conversation_id = c.id), 0) as message_count,
                c.created_at,
                c.updated_at
            FROM pp_chat_conversation c
            ORDER BY c.updated_at DESC
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt.query_map([], |row| {
        Ok(Conversation {
            id: row.get(0)?,
            title: row.get(1)?,
            message_count: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })
    .map_err(|e| format!("Failed to query conversations: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect conversations: {}", e))
}

/// Create a new conversation.
/// Returns the created conversation.
#[command]
pub fn create_conversation(title: Option<String>) -> Result<Conversation, String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let actual_title = title.unwrap_or_else(|| "Neuer Chat".to_string());

    conn.execute(
        "INSERT INTO pp_chat_conversation (title) VALUES (?1)",
        [&actual_title],
    )
    .map_err(|e| format!("Failed to create conversation: {}", e))?;

    let id = conn.last_insert_rowid();

    // Fetch the created conversation
    let conversation = conn
        .query_row(
            "SELECT id, title, created_at, updated_at FROM pp_chat_conversation WHERE id = ?1",
            [id],
            |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    message_count: 0,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .map_err(|e| format!("Failed to fetch created conversation: {}", e))?;

    Ok(conversation)
}

/// Update the title of a conversation.
#[command]
pub fn update_conversation_title(id: i64, title: String) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let rows_affected = conn
        .execute(
            "UPDATE pp_chat_conversation SET title = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![title, id],
        )
        .map_err(|e| format!("Failed to update conversation title: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Conversation with id {} not found", id));
    }

    Ok(())
}

/// Delete a conversation and all its messages (cascade).
#[command]
pub fn delete_conversation(id: i64) -> Result<(), String> {
    let guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    // Delete messages first (SQLite foreign key cascade might not always work)
    conn.execute("DELETE FROM pp_chat_suggestion WHERE conversation_id = ?1", [id])
        .map_err(|e| format!("Failed to delete conversation suggestions: {}", e))?;

    conn.execute("DELETE FROM pp_chat_history WHERE conversation_id = ?1", [id])
        .map_err(|e| format!("Failed to delete conversation messages: {}", e))?;

    let rows_affected = conn
        .execute("DELETE FROM pp_chat_conversation WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete conversation: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Conversation with id {} not found", id));
    }

    Ok(())
}
