use crate::pp::Client;
use crate::protobuf;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::command;

/// Information about a recently opened file
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    pub last_opened: String,
}

/// Result of opening a file including the path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenResult {
    pub path: String,
    pub portfolio: Client,
}

/// Create a new empty portfolio file
#[command]
pub fn create_new_portfolio(base_currency: Option<String>) -> Client {
    Client::new(base_currency.as_deref().unwrap_or("EUR"))
}

/// Open a portfolio file from a given path
#[command]
pub async fn open_portfolio_file(path: String) -> Result<OpenResult, String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err("File does not exist".to_string());
    }

    let portfolio = protobuf::parse_portfolio_file(&path_buf).map_err(|e| e.to_string())?;

    Ok(OpenResult { path, portfolio })
}

/// Save a portfolio file to a given path
/// Note: Writing protobuf format is not yet implemented
#[command]
pub async fn save_portfolio_file(_path: String, _client: Client) -> Result<(), String> {
    // TODO: Implement protobuf writing
    Err("Saving protobuf format is not yet implemented".to_string())
}

/// Get the file extension for Portfolio Performance files
#[command]
pub fn get_file_extension() -> String {
    "portfolio".to_string()
}

/// Validate a portfolio file without fully loading it
#[command]
pub async fn validate_portfolio_file(path: String) -> Result<bool, String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err("File does not exist".to_string());
    }

    match protobuf::parse_portfolio_file(&path_buf) {
        Ok(_) => Ok(true),
        Err(e) => Err(format!("Invalid portfolio file: {}", e)),
    }
}

/// Export portfolio statistics as JSON
#[command]
pub fn get_portfolio_stats(client: Client) -> PortfolioStats {
    PortfolioStats {
        version: client.version,
        base_currency: client.base_currency,
        securities_count: client.securities.len(),
        accounts_count: client.accounts.len(),
        portfolios_count: client.portfolios.len(),
        watchlists_count: client.watchlists.len(),
        taxonomies_count: client.taxonomies.len(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioStats {
    pub version: i32,
    pub base_currency: String,
    pub securities_count: usize,
    pub accounts_count: usize,
    pub portfolios_count: usize,
    pub watchlists_count: usize,
    pub taxonomies_count: usize,
}
