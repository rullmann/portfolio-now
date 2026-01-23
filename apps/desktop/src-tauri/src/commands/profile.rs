//! User profile commands for profile picture management.
//!
//! Profile pictures are stored as base64-encoded strings in the pp_settings table.

use rusqlite::params;

use crate::db::get_connection;

const PROFILE_PICTURE_KEY: &str = "user_profile_picture";

/// Set the user's profile picture.
/// Pass None to remove the profile picture.
#[tauri::command]
pub async fn set_user_profile_picture(picture_base64: Option<String>) -> Result<(), String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    match picture_base64 {
        Some(data) => {
            // Validate it's not too large (max 10MB after base64 encoding)
            if data.len() > 10 * 1024 * 1024 {
                return Err("Profile picture too large (max 10MB)".to_string());
            }

            // Upsert the profile picture
            conn.execute(
                r#"
                INSERT INTO pp_settings (key, value) VALUES (?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value
                "#,
                params![PROFILE_PICTURE_KEY, data],
            )
            .map_err(|e| format!("Failed to save profile picture: {}", e))?;
        }
        None => {
            // Remove the profile picture
            conn.execute(
                "DELETE FROM pp_settings WHERE key = ?",
                params![PROFILE_PICTURE_KEY],
            )
            .map_err(|e| format!("Failed to remove profile picture: {}", e))?;
        }
    }

    Ok(())
}

/// Get the user's profile picture.
/// Returns None if no profile picture is set.
#[tauri::command]
pub async fn get_user_profile_picture() -> Result<Option<String>, String> {
    let guard = get_connection().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("Database not initialized")?;

    let result: Option<String> = conn
        .query_row(
            "SELECT value FROM pp_settings WHERE key = ?",
            params![PROFILE_PICTURE_KEY],
            |row| row.get(0),
        )
        .ok();

    Ok(result)
}
