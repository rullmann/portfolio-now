//! Tauri commands for symbol validation
//!
//! Commands for validating and correcting quote source configurations.

use crate::validation::{
    apply_validation_result, get_validation_status, validate_all_securities, validate_security,
    AiConfig, ApiKeys, SecurityForValidation, ValidatedConfig, ValidationResult, ValidationStatusSummary,
};
use crate::db::get_connection;
use anyhow::anyhow;
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// Request to validate securities
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateSecuritiesRequest {
    /// Only validate securities that are currently held
    pub only_held: bool,
    /// Force re-validation even if cached result exists
    pub force: bool,
    /// API keys for providers
    pub api_keys: ApiKeys,
    /// AI configuration (optional)
    pub ai_config: Option<AiConfig>,
}

/// Request to validate a single security
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateSingleRequest {
    pub security_id: i64,
    pub api_keys: ApiKeys,
    pub ai_config: Option<AiConfig>,
}

/// Request to apply validation result
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyValidationRequest {
    pub security_id: i64,
    pub config: ValidatedConfig,
}

/// Response for validation operations
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResponse {
    pub success: bool,
    pub results: Vec<ValidationResult>,
    pub summary: Option<ValidationSummary>,
    pub error: Option<String>,
}

/// Summary of validation results
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationSummary {
    pub total: i32,
    pub validated: i32,
    pub failed: i32,
    pub ai_suggested: i32,
    pub skipped: i32,
}

/// Validate all securities
#[tauri::command]
pub async fn validate_all_securities_cmd(
    request: ValidateSecuritiesRequest,
) -> Result<ValidationResponse, String> {
    match validate_all_securities(
        request.only_held,
        request.force,
        &request.api_keys,
        request.ai_config.as_ref(),
    )
    .await
    {
        Ok(results) => {
            let summary = ValidationSummary {
                total: results.len() as i32,
                validated: results.iter().filter(|r| r.status == crate::validation::ValidationStatus::Validated).count() as i32,
                failed: results.iter().filter(|r| r.status == crate::validation::ValidationStatus::Failed).count() as i32,
                ai_suggested: results.iter().filter(|r| r.status == crate::validation::ValidationStatus::AiSuggested).count() as i32,
                skipped: results.iter().filter(|r| r.status == crate::validation::ValidationStatus::Skipped).count() as i32,
            };

            Ok(ValidationResponse {
                success: true,
                results,
                summary: Some(summary),
                error: None,
            })
        }
        Err(e) => Ok(ValidationResponse {
            success: false,
            results: vec![],
            summary: None,
            error: Some(e.to_string()),
        }),
    }
}

/// Validate a single security
#[tauri::command]
pub async fn validate_security_cmd(
    request: ValidateSingleRequest,
) -> Result<ValidationResult, String> {
    // Get security from database
    let security = get_security_for_validation(request.security_id)
        .map_err(|e| e.to_string())?;

    Ok(validate_security(
        security,
        &request.api_keys,
        request.ai_config.as_ref(),
        true, // Force validation for single security
    )
    .await)
}

/// Apply validation result to security
#[tauri::command]
pub async fn apply_validation_result_cmd(
    request: ApplyValidationRequest,
) -> Result<(), String> {
    apply_validation_result(request.security_id, &request.config)
        .map_err(|e| e.to_string())
}

/// Get validation status summary
#[tauri::command]
pub async fn get_validation_status_cmd(
    only_held: bool,
) -> Result<ValidationStatusSummary, String> {
    get_validation_status(only_held).map_err(|e| e.to_string())
}

/// Get security data for validation
fn get_security_for_validation(security_id: i64) -> anyhow::Result<SecurityForValidation> {
    let conn_guard = get_connection()?;
    let conn = conn_guard.as_ref().ok_or_else(|| anyhow!("Database not initialized"))?;

    conn.query_row(
        r#"
        SELECT id, name, isin, wkn, ticker, currency, feed, feed_url, is_retired
        FROM pp_security
        WHERE id = ?1
        "#,
        params![security_id],
        |row| {
            Ok(SecurityForValidation {
                id: row.get(0)?,
                name: row.get(1)?,
                isin: row.get(2)?,
                wkn: row.get(3)?,
                ticker: row.get(4)?,
                currency: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "EUR".to_string()),
                feed: row.get(6)?,
                feed_url: row.get(7)?,
                is_retired: row.get::<_, i32>(8)? == 1,
            })
        },
    ).map_err(|e| anyhow!("Security not found: {}", e))
}
