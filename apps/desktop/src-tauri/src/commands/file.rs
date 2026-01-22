use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use crate::db;
use crate::pp::{
    common::Money,
    transaction::{
        AccountTransaction, AccountTransactionType, PortfolioTransaction,
        PortfolioTransactionType,
    },
    Account, Client, LatestPrice, Portfolio, PriceEntry, Security,
};
use crate::protobuf;
use crate::security;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
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
///
/// # Security
/// Path is validated to prevent directory traversal and access to unauthorized locations.
#[command]
pub async fn open_portfolio_file(path: String) -> Result<OpenResult, String> {
    // SECURITY: Validate the file path
    let validated_path = security::validate_file_path_with_extension(&path, Some(&["portfolio"]))?;

    if !validated_path.exists() {
        return Err("File does not exist".to_string());
    }

    let portfolio = protobuf::parse_portfolio_file(&validated_path).map_err(|e| e.to_string())?;

    Ok(OpenResult {
        path: validated_path.to_string_lossy().to_string(),
        portfolio,
    })
}

/// Save a portfolio file to a given path
///
/// # Security
/// Path is validated to prevent directory traversal and access to unauthorized locations.
#[command]
pub async fn save_portfolio_file(path: String, client: Client) -> Result<(), String> {
    // SECURITY: Validate the file path
    let validated_path = security::validate_file_path_with_extension(&path, Some(&["portfolio"]))?;
    protobuf::write_portfolio_file(&validated_path, &client).map_err(|e| e.to_string())
}

/// Get the file extension for Portfolio Performance files
#[command]
pub fn get_file_extension() -> String {
    "portfolio".to_string()
}

/// Validate a portfolio file without fully loading it
///
/// # Security
/// Path is validated to prevent directory traversal and access to unauthorized locations.
#[command]
pub async fn validate_portfolio_file(path: String) -> Result<bool, String> {
    // SECURITY: Validate the file path
    let validated_path = security::validate_file_path_with_extension(&path, Some(&["portfolio"]))?;

    if !validated_path.exists() {
        return Err("File does not exist".to_string());
    }

    match protobuf::parse_portfolio_file(&validated_path) {
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

/// Export database to a .portfolio file
///
/// # Security
/// Path is validated to prevent directory traversal and access to unauthorized locations.
#[command]
pub async fn export_database_to_portfolio(path: String) -> Result<ExportResult, String> {
    // SECURITY: Validate the file path
    let path_buf = security::validate_file_path_with_extension(&path, Some(&["portfolio"]))?;

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get base currency from import
    let base_currency: String = conn
        .query_row(
            "SELECT base_currency FROM pp_import ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "EUR".to_string());

    let mut client = Client::new(&base_currency);

    // Load securities
    let securities = load_securities_from_db(conn).map_err(|e| e.to_string())?;
    let securities_count = securities.len();
    client.securities = securities;

    // Load accounts with transactions
    let accounts = load_accounts_from_db(conn).map_err(|e| e.to_string())?;
    let accounts_count = accounts.len();
    client.accounts = accounts;

    // Load portfolios with transactions
    let portfolios = load_portfolios_from_db(conn).map_err(|e| e.to_string())?;
    let portfolios_count = portfolios.len();
    client.portfolios = portfolios;

    // Write to file
    protobuf::write_portfolio_file(&path_buf, &client).map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path: path_buf.to_string_lossy().to_string(),
        securities_count,
        accounts_count,
        portfolios_count,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub securities_count: usize,
    pub accounts_count: usize,
    pub portfolios_count: usize,
}

/// Load securities from database
fn load_securities_from_db(conn: &rusqlite::Connection) -> Result<Vec<Security>, rusqlite::Error> {
    let mut securities = Vec::new();

    let mut stmt = conn.prepare(
        r#"
        SELECT uuid, name, currency, isin, wkn, ticker, feed, feed_url, is_retired
        FROM pp_security
        ORDER BY name
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, bool>(8)?,
        ))
    })?;

    for row in rows.flatten() {
        let (uuid, name, currency, isin, wkn, ticker, feed, feed_url, is_retired) = row;
        let mut sec = Security::new(uuid.clone(), name, currency);
        sec.isin = isin;
        sec.wkn = wkn;
        sec.ticker = ticker;
        sec.feed = feed;
        sec.feed_url = feed_url;
        sec.is_retired = is_retired;

        // Load prices for this security
        let sec_id: i64 = conn.query_row(
            "SELECT id FROM pp_security WHERE uuid = ?",
            [&uuid],
            |row| row.get(0),
        )?;

        let mut price_stmt = conn.prepare(
            "SELECT date, value FROM pp_price WHERE security_id = ? ORDER BY date",
        )?;
        let price_rows = price_stmt.query_map([sec_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for pr in price_rows.flatten() {
            if let Ok(date) = NaiveDate::parse_from_str(&pr.0, "%Y-%m-%d") {
                sec.prices.push(PriceEntry::new(date, pr.1));
            }
        }

        // Load latest price
        if let Ok((date_str, value, high, low, volume)) = conn.query_row::<(String, i64, Option<i64>, Option<i64>, Option<i64>), _, _>(
            "SELECT date, value, high, low, volume FROM pp_latest_price WHERE security_id = ?",
            [sec_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ) {
            sec.latest = Some(LatestPrice {
                date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok(),
                value: Some(value),
                high,
                low,
                volume,
            });
        }

        securities.push(sec);
    }

    Ok(securities)
}

/// Load accounts from database
fn load_accounts_from_db(conn: &rusqlite::Connection) -> Result<Vec<Account>, rusqlite::Error> {
    let mut accounts = Vec::new();

    let mut stmt = conn.prepare(
        r#"
        SELECT id, uuid, name, currency, is_retired
        FROM pp_account
        ORDER BY name
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, bool>(4)?,
        ))
    })?;

    for row in rows.flatten() {
        let (id, uuid, name, currency, is_retired) = row;
        let mut acc = Account::new(uuid.clone(), name, currency);
        acc.is_retired = is_retired;

        // Load transactions for this account
        let txns = load_account_transactions(conn, id)?;
        acc.transactions = txns;

        accounts.push(acc);
    }

    Ok(accounts)
}

/// Load account transactions
fn load_account_transactions(
    conn: &rusqlite::Connection,
    account_id: i64,
) -> Result<Vec<AccountTransaction>, rusqlite::Error> {
    let mut txns = Vec::new();

    let mut stmt = conn.prepare(
        r#"
        SELECT t.uuid, t.txn_type, t.date, t.amount, t.currency, t.shares, t.note,
               s.uuid as security_uuid
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        WHERE t.owner_type = 'account' AND t.owner_id = ?
        ORDER BY t.date
        "#,
    )?;

    let rows = stmt.query_map([account_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    })?;

    for row in rows.flatten() {
        let (uuid, txn_type, date_str, amount, currency, shares, note, security_uuid) = row;

        let tx_type = match txn_type.as_str() {
            "DEPOSIT" => AccountTransactionType::Deposit,
            "REMOVAL" => AccountTransactionType::Removal,
            "DIVIDENDS" => AccountTransactionType::Dividends,
            "INTEREST" => AccountTransactionType::Interest,
            "INTEREST_CHARGE" => AccountTransactionType::InterestCharge,
            "TAXES" => AccountTransactionType::Taxes,
            "TAX_REFUND" => AccountTransactionType::TaxRefund,
            "FEES" => AccountTransactionType::Fees,
            "FEES_REFUND" => AccountTransactionType::FeesRefund,
            "BUY" => AccountTransactionType::Buy,
            "SELL" => AccountTransactionType::Sell,
            "TRANSFER_IN" => AccountTransactionType::TransferIn,
            "TRANSFER_OUT" => AccountTransactionType::TransferOut,
            _ => continue,
        };

        if let Ok(date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            let datetime = date.and_hms_opt(0, 0, 0).unwrap();
            let mut tx = AccountTransaction::new(
                uuid,
                datetime,
                tx_type,
                Money::new(amount, currency),
            );
            tx.shares = shares;
            tx.note = note;
            tx.security_uuid = security_uuid;
            txns.push(tx);
        }
    }

    Ok(txns)
}

/// Load portfolios from database
fn load_portfolios_from_db(conn: &rusqlite::Connection) -> Result<Vec<Portfolio>, rusqlite::Error> {
    let mut portfolios = Vec::new();

    let mut stmt = conn.prepare(
        r#"
        SELECT p.id, p.uuid, p.name, p.is_retired, a.uuid as ref_account_uuid
        FROM pp_portfolio p
        LEFT JOIN pp_account a ON a.id = p.reference_account_id
        ORDER BY p.name
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, bool>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    for row in rows.flatten() {
        let (id, uuid, name, is_retired, ref_account_uuid) = row;
        let mut port = Portfolio::new(uuid.clone(), name);
        port.is_retired = is_retired;
        port.reference_account_uuid = ref_account_uuid;

        // Load transactions for this portfolio
        let txns = load_portfolio_transactions(conn, id)?;
        port.transactions = txns;

        portfolios.push(port);
    }

    Ok(portfolios)
}

/// Load portfolio transactions
fn load_portfolio_transactions(
    conn: &rusqlite::Connection,
    portfolio_id: i64,
) -> Result<Vec<PortfolioTransaction>, rusqlite::Error> {
    let mut txns = Vec::new();

    let mut stmt = conn.prepare(
        r#"
        SELECT t.uuid, t.txn_type, t.date, t.amount, t.currency, t.shares, t.note,
               s.uuid as security_uuid
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        WHERE t.owner_type = 'portfolio' AND t.owner_id = ?
        ORDER BY t.date
        "#,
    )?;

    let rows = stmt.query_map([portfolio_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    })?;

    for row in rows.flatten() {
        let (uuid, txn_type, date_str, amount, currency, shares, note, security_uuid) = row;

        let tx_type = match txn_type.as_str() {
            "BUY" => PortfolioTransactionType::Buy,
            "SELL" => PortfolioTransactionType::Sell,
            "TRANSFER_IN" => PortfolioTransactionType::TransferIn,
            "TRANSFER_OUT" => PortfolioTransactionType::TransferOut,
            "DELIVERY_INBOUND" => PortfolioTransactionType::DeliveryInbound,
            "DELIVERY_OUTBOUND" => PortfolioTransactionType::DeliveryOutbound,
            _ => continue,
        };

        if let Ok(date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
            let datetime = date.and_hms_opt(0, 0, 0).unwrap();
            let mut tx = PortfolioTransaction::new(
                uuid,
                datetime,
                tx_type,
                Money::new(amount, currency),
                shares,
            );
            tx.note = note;
            tx.security_uuid = security_uuid;
            txns.push(tx);
        }
    }

    Ok(txns)
}

/// Read a file as base64 for chat attachments
///
/// # Security
/// Path is validated. Only PDF files are allowed for security reasons.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBase64Result {
    pub data: String,
    pub mime_type: String,
    pub filename: String,
}

#[command]
pub async fn read_file_as_base64(path: String) -> Result<FileBase64Result, String> {
    // SECURITY: Validate the file path - only allow PDF files
    let validated_path = security::validate_file_path_with_extension(&path, Some(&["pdf"]))?;

    if !validated_path.exists() {
        return Err("File does not exist".to_string());
    }

    // Read the file
    let data = std::fs::read(&validated_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Check file size (max 50MB for PDFs)
    const MAX_SIZE: usize = 50 * 1024 * 1024;
    if data.len() > MAX_SIZE {
        return Err(format!("File too large (max {} MB)", MAX_SIZE / 1024 / 1024));
    }

    // Encode as base64
    let base64_data = BASE64.encode(&data);

    // Extract filename
    let filename = validated_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf")
        .to_string();

    Ok(FileBase64Result {
        data: base64_data,
        mime_type: "application/pdf".to_string(),
        filename,
    })
}
