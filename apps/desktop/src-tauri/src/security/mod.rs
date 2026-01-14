//! Security utilities for the application
//!
//! This module provides security-related functionality including:
//! - Path validation to prevent directory traversal attacks
//! - Rate limiting for API calls
//! - Input sanitization
//!
//! Note: Some functions are prepared for future use and may not be called yet.

use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

/// Allowed directories for file operations
/// These are relative to the user's home directory or absolute paths
const ALLOWED_RELATIVE_DIRS: &[&str] = &[
    "Documents",
    "Downloads",
    "Desktop",
];

/// File extensions allowed for portfolio files
const ALLOWED_EXTENSIONS: &[&str] = &[
    "portfolio",
    "csv",
    "pdf",
    "json",
];

/// Validates a file path to ensure it's within allowed directories
///
/// # Security
/// This function prevents:
/// - Directory traversal attacks (../)
/// - Symlink attacks
/// - Access to system directories
/// - Access to hidden files (starting with .)
///
/// # Arguments
/// * `path` - The path provided by the user/frontend
/// * `allowed_dirs` - Optional list of additional allowed directories
///
/// # Returns
/// * `Ok(PathBuf)` - The validated, canonicalized path
/// * `Err(String)` - Error message if validation fails
pub fn validate_file_path(path: &str, allowed_dirs: Option<&[PathBuf]>) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);

    // Reject paths with .. components (directory traversal)
    if path.components().any(|c| c.as_os_str() == "..") {
        return Err("Path contains directory traversal sequences (..)".to_string());
    }

    // Reject hidden files/directories (starting with .)
    if path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| s.starts_with('.') && s != ".")
            .unwrap_or(false)
    }) {
        return Err("Access to hidden files/directories is not allowed".to_string());
    }

    // Check if path exists and canonicalize it
    let canonical_path = if path.exists() {
        path.canonicalize()
            .map_err(|e| format!("Failed to resolve path: {}", e))?
    } else {
        // For new files, canonicalize the parent directory
        let parent = path.parent().ok_or("Invalid path: no parent directory")?;
        if !parent.exists() {
            return Err(format!("Parent directory does not exist: {:?}", parent));
        }
        let canonical_parent = parent
            .canonicalize()
            .map_err(|e| format!("Failed to resolve parent path: {}", e))?;
        canonical_parent.join(path.file_name().ok_or("Invalid filename")?)
    };

    // Get home directory
    let home_dir = dirs::home_dir().ok_or("Could not determine home directory")?;

    // Build list of allowed directories
    let mut all_allowed_dirs: Vec<PathBuf> = ALLOWED_RELATIVE_DIRS
        .iter()
        .map(|d| home_dir.join(d))
        .collect();

    // Add any additional allowed directories
    if let Some(extra_dirs) = allowed_dirs {
        all_allowed_dirs.extend(extra_dirs.iter().cloned());
    }

    // Check if the path is within an allowed directory
    let is_allowed = all_allowed_dirs.iter().any(|allowed_dir| {
        if let Ok(canonical_allowed) = allowed_dir.canonicalize() {
            canonical_path.starts_with(&canonical_allowed)
        } else {
            // If allowed dir doesn't exist, check if it would be a prefix
            canonical_path.starts_with(allowed_dir)
        }
    });

    if !is_allowed {
        return Err(format!(
            "Path is outside allowed directories. Allowed: {:?}",
            all_allowed_dirs
        ));
    }

    Ok(canonical_path)
}

/// Validates a file path with extension check
///
/// # Arguments
/// * `path` - The path provided by the user/frontend
/// * `allowed_extensions` - Optional list of allowed file extensions (without dot)
pub fn validate_file_path_with_extension(
    path: &str,
    allowed_extensions: Option<&[&str]>,
) -> Result<PathBuf, String> {
    let validated_path = validate_file_path(path, None)?;

    let extensions = allowed_extensions.unwrap_or(ALLOWED_EXTENSIONS);

    if let Some(ext) = validated_path.extension() {
        let ext_str = ext.to_str().unwrap_or("");
        if !extensions.contains(&ext_str) {
            return Err(format!(
                "File extension '{}' is not allowed. Allowed: {:?}",
                ext_str, extensions
            ));
        }
    } else {
        return Err("File must have an extension".to_string());
    }

    Ok(validated_path)
}

/// Get the app data directory for storing application-specific files
///
/// This is the secure location for storing:
/// - Database files
/// - Configuration
/// - Cached data
/// - API keys (encrypted)
#[allow(dead_code)] // Planned API: Will be used for secure storage implementation
pub fn get_app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))
}

/// Sanitize a string to prevent injection attacks
///
/// Removes or escapes potentially dangerous characters
#[allow(dead_code)] // Planned API: For input sanitization in future commands
pub fn sanitize_string(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            c.is_alphanumeric()
                || *c == ' '
                || *c == '-'
                || *c == '_'
                || *c == '.'
                || *c == ','
                || *c == '('
                || *c == ')'
        })
        .collect()
}

/// Sanitize a filename to be safe for filesystem operations
#[allow(dead_code)] // Planned API: For safe filename creation in future exports
pub fn sanitize_filename(filename: &str) -> String {
    let sanitized: String = filename
        .chars()
        .filter(|c| {
            c.is_alphanumeric()
                || *c == '-'
                || *c == '_'
                || *c == '.'
        })
        .collect();

    // Ensure it doesn't start with a dot (hidden file)
    if sanitized.starts_with('.') {
        format!("_{}", &sanitized[1..])
    } else {
        sanitized
    }
}

// ============================================================================
// Rate Limiting
// ============================================================================

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Global rate limiter state
#[allow(dead_code)] // Planned API: Documented in CLAUDE.md security section
static RATE_LIMITERS: once_cell::sync::Lazy<Mutex<HashMap<String, RateLimiterState>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

/// State for a single rate limiter
#[allow(dead_code)] // Planned API: Used by check_rate_limit
struct RateLimiterState {
    last_request: Instant,
    request_count: u32,
    window_start: Instant,
}

/// Rate limiter configuration
#[allow(dead_code)] // Planned API: Documented in CLAUDE.md security section
pub struct RateLimitConfig {
    /// Minimum time between requests
    pub min_interval: Duration,
    /// Maximum requests per window
    pub max_requests_per_window: u32,
    /// Window duration for counting requests
    pub window_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            min_interval: Duration::from_secs(1),
            max_requests_per_window: 60,
            window_duration: Duration::from_secs(60),
        }
    }
}

/// Check if a request should be rate limited
///
/// # Arguments
/// * `key` - Unique identifier for this rate limit (e.g., "sync_all_prices")
/// * `config` - Rate limit configuration
///
/// # Returns
/// * `Ok(())` - Request is allowed
/// * `Err(String)` - Request is rate limited with error message
#[allow(dead_code)] // Planned API: Documented in CLAUDE.md security section
pub fn check_rate_limit(key: &str, config: &RateLimitConfig) -> Result<(), String> {
    let mut limiters = RATE_LIMITERS.lock().map_err(|e| format!("Rate limiter lock error: {}", e))?;

    let now = Instant::now();

    let state = limiters.entry(key.to_string()).or_insert_with(|| RateLimiterState {
        last_request: now - config.min_interval, // Allow first request
        request_count: 0,
        window_start: now,
    });

    // Check minimum interval
    let elapsed = now.duration_since(state.last_request);
    if elapsed < config.min_interval {
        let wait_time = config.min_interval - elapsed;
        return Err(format!(
            "Rate limit: Bitte warte noch {} Sekunden.",
            wait_time.as_secs() + 1
        ));
    }

    // Check window limit
    if now.duration_since(state.window_start) > config.window_duration {
        // Reset window
        state.window_start = now;
        state.request_count = 0;
    }

    if state.request_count >= config.max_requests_per_window {
        let remaining = config.window_duration.saturating_sub(now.duration_since(state.window_start));
        return Err(format!(
            "Rate limit: Maximale Anfragen erreicht. Bitte warte {} Sekunden.",
            remaining.as_secs() + 1
        ));
    }

    // Update state
    state.last_request = now;
    state.request_count += 1;

    Ok(())
}

/// Pre-configured rate limits for common operations
#[allow(dead_code)] // Planned API: Rate limit presets for commands
pub mod limits {
    use super::*;

    /// Rate limit for price sync operations (1 per minute)
    pub fn price_sync() -> RateLimitConfig {
        RateLimitConfig {
            min_interval: Duration::from_secs(60),
            max_requests_per_window: 5,
            window_duration: Duration::from_secs(300), // 5 requests per 5 minutes
        }
    }

    /// Rate limit for AI analysis operations (1 per 5 seconds)
    pub fn ai_analysis() -> RateLimitConfig {
        RateLimitConfig {
            min_interval: Duration::from_secs(5),
            max_requests_per_window: 20,
            window_duration: Duration::from_secs(60),
        }
    }

    /// Rate limit for PDF imports (1 per 10 seconds)
    pub fn pdf_import() -> RateLimitConfig {
        RateLimitConfig {
            min_interval: Duration::from_secs(10),
            max_requests_per_window: 10,
            window_duration: Duration::from_secs(300),
        }
    }

    /// Rate limit for file exports (1 per 5 seconds)
    pub fn file_export() -> RateLimitConfig {
        RateLimitConfig {
            min_interval: Duration::from_secs(5),
            max_requests_per_window: 20,
            window_duration: Duration::from_secs(60),
        }
    }
}

// ============================================================================
// Secure Storage (TODO: Full implementation)
// ============================================================================

/// SECURITY TODO: API keys are currently stored in browser localStorage.
/// This module provides the infrastructure for secure storage that should
/// be used in a future migration:
///
/// 1. Use tauri-plugin-store for encrypted storage
/// 2. Or implement OS keychain integration (macOS Keychain, Windows Credential Manager)
/// 3. Or encrypt with a key derived from the user's password
///
/// For now, API keys are passed directly to commands from the frontend.
/// The keys are not logged and are only held in memory during API calls.

/// Placeholder for secure credential storage
/// TODO: Implement with tauri-plugin-store or OS keychain
#[allow(dead_code)] // Planned API: Backend secure storage (frontend uses tauri-plugin-store)
pub mod secure_storage {
    use std::path::PathBuf;

    /// Store a sensitive value securely
    ///
    /// SECURITY: Currently a no-op placeholder. In production, this should:
    /// - Encrypt the value before storage
    /// - Use OS keychain if available
    /// - Store in app_data_dir with restricted permissions
    pub fn store_secret(_key: &str, _value: &str, _app_data_dir: &PathBuf) -> Result<(), String> {
        // TODO: Implement secure storage
        // For now, we don't store secrets at all - they're managed by the frontend
        Err("Secure storage not yet implemented - use frontend settings".to_string())
    }

    /// Retrieve a sensitive value
    pub fn get_secret(_key: &str, _app_data_dir: &PathBuf) -> Result<Option<String>, String> {
        // TODO: Implement secure retrieval
        Err("Secure storage not yet implemented".to_string())
    }

    /// Delete a sensitive value
    pub fn delete_secret(_key: &str, _app_data_dir: &PathBuf) -> Result<(), String> {
        // TODO: Implement secure deletion
        Err("Secure storage not yet implemented".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_directory_traversal() {
        let result = validate_file_path("/home/user/../etc/passwd", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("directory traversal"));
    }

    #[test]
    fn test_reject_hidden_files() {
        let result = validate_file_path("/home/user/.ssh/id_rsa", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hidden files"));
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("Hello World!"), "Hello World");
        assert_eq!(sanitize_string("test<script>"), "testscript");
        assert_eq!(sanitize_string("file.txt"), "file.txt");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("my file.pdf"), "myfile.pdf");
        assert_eq!(sanitize_filename(".hidden"), "_hidden");
        assert_eq!(sanitize_filename("test<>file.txt"), "testfile.txt");
    }
}
