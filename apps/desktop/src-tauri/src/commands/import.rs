//! Import commands for Portfolio Performance files.
//!
//! This module provides Tauri commands for importing PP files (protobuf binary format)
//! into the SQLite database with full data fidelity.

use crate::db;
use crate::pp::{
    self, Account, Classification, Client, Portfolio, Security,
    security::SecurityEventKind, taxonomy::Taxonomy, transaction::AccountTransaction,
    transaction::PortfolioTransaction,
};
use crate::protobuf;
use crate::quotes::ExchangeRate;
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, AppHandle, Emitter};

/// Progress information during import
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub stage: String,
    pub message: String,
    pub percent: u32,
    pub current: Option<usize>,
    pub total: Option<usize>,
}

/// Result of a successful import
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub import_id: i64,
    pub file_path: String,
    pub version: i32,
    pub base_currency: String,
    pub securities_count: usize,
    pub accounts_count: usize,
    pub portfolios_count: usize,
    pub transactions_count: usize,
    pub prices_count: usize,
    pub warnings: Vec<String>,
}

/// Import a Portfolio Performance XML file into the database
#[command]
pub async fn import_pp_file(path: String, app: AppHandle) -> Result<ImportResult, String> {
    // Emit progress: starting
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "reading".to_string(),
            message: "Reading file...".to_string(),
            percent: 0,
            current: None,
            total: None,
        },
    );

    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("File does not exist".to_string());
    }

    // Emit progress: parsing
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "parsing".to_string(),
            message: "Parsing protobuf...".to_string(),
            percent: 10,
            current: None,
            total: None,
        },
    );

    // Parse protobuf to Client
    let client = protobuf::parse_portfolio_file(&path_buf)
        .map_err(|e| format!("Failed to parse portfolio file: {}", e))?;

    // Emit progress: saving
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "saving".to_string(),
            message: "Saving to database...".to_string(),
            percent: 30,
            current: None,
            total: None,
        },
    );

    // Save to database
    let result =
        save_client_to_db(&path, &client, &app).map_err(|e| format!("Failed to save to database: {}", e))?;

    // Fetch exchange rates from ECB (non-blocking, errors are logged but don't fail import)
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "exchange_rates".to_string(),
            message: "Fetching exchange rates...".to_string(),
            percent: 96,
            current: None,
            total: None,
        },
    );

    match crate::quotes::ecb::fetch_latest_rates().await {
        Ok(rates) => {
            if let Err(e) = save_exchange_rates(&rates) {
                log::warn!("Failed to save exchange rates: {}", e);
            } else {
                log::info!("Saved {} exchange rates from ECB", rates.len());
            }
        }
        Err(e) => {
            log::warn!("Failed to fetch exchange rates from ECB: {}", e);
        }
    }

    // Emit progress: complete
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "complete".to_string(),
            message: "Import complete".to_string(),
            percent: 100,
            current: None,
            total: None,
        },
    );

    Ok(result)
}

/// Save the parsed Client to the database
fn save_client_to_db(path: &str, client: &Client, app: &AppHandle) -> Result<ImportResult> {
    let mut conn_guard = db::get_connection()?;
    let conn = conn_guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

    // Start transaction for atomic import
    let tx = conn.transaction()?;

    // Calculate total transactions
    let total_transactions: usize = client
        .accounts
        .iter()
        .map(|a| a.transactions.len())
        .sum::<usize>()
        + client
            .portfolios
            .iter()
            .map(|p| p.transactions.len())
            .sum::<usize>();

    // Calculate total prices
    let total_prices: usize = client.securities.iter().map(|s| s.prices.len()).sum();

    // Create import record
    tx.execute(
        "INSERT INTO pp_import (file_path, version, base_currency, securities_count, accounts_count, portfolios_count, transactions_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            path,
            client.version,
            client.base_currency,
            client.securities.len(),
            client.accounts.len(),
            client.portfolios.len(),
            total_transactions,
        ],
    )?;
    let import_id = tx.last_insert_rowid();

    let mut warnings = Vec::new();

    // Import securities
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "securities".to_string(),
            message: "Importing securities...".to_string(),
            percent: 35,
            current: Some(0),
            total: Some(client.securities.len()),
        },
    );

    for (i, security) in client.securities.iter().enumerate() {
        if let Err(e) = insert_security(&tx, import_id, security) {
            warnings.push(format!("Security {}: {}", security.name, e));
        }

        if i % 10 == 0 {
            let _ = app.emit(
                "import-progress",
                ImportProgress {
                    stage: "securities".to_string(),
                    message: format!("Importing security: {}", security.name),
                    percent: 35 + (i * 15 / client.securities.len().max(1)) as u32,
                    current: Some(i),
                    total: Some(client.securities.len()),
                },
            );
        }
    }

    // Import accounts
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "accounts".to_string(),
            message: "Importing accounts...".to_string(),
            percent: 50,
            current: Some(0),
            total: Some(client.accounts.len()),
        },
    );

    for (i, account) in client.accounts.iter().enumerate() {
        if let Err(e) = insert_account(&tx, import_id, account) {
            warnings.push(format!("Account {}: {}", account.name, e));
        }

        let _ = app.emit(
            "import-progress",
            ImportProgress {
                stage: "accounts".to_string(),
                message: format!("Importing account: {}", account.name),
                percent: 50 + (i * 10 / client.accounts.len().max(1)) as u32,
                current: Some(i),
                total: Some(client.accounts.len()),
            },
        );
    }

    // Import portfolios
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "portfolios".to_string(),
            message: "Importing portfolios...".to_string(),
            percent: 60,
            current: Some(0),
            total: Some(client.portfolios.len()),
        },
    );

    for (i, portfolio) in client.portfolios.iter().enumerate() {
        if let Err(e) = insert_portfolio(&tx, import_id, portfolio) {
            warnings.push(format!("Portfolio {}: {}", portfolio.name, e));
        }

        let _ = app.emit(
            "import-progress",
            ImportProgress {
                stage: "portfolios".to_string(),
                message: format!("Importing portfolio: {}", portfolio.name),
                percent: 60 + (i * 10 / client.portfolios.len().max(1)) as u32,
                current: Some(i),
                total: Some(client.portfolios.len()),
            },
        );
    }

    // Import taxonomies
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "taxonomies".to_string(),
            message: "Importing taxonomies...".to_string(),
            percent: 70,
            current: Some(0),
            total: Some(client.taxonomies.len()),
        },
    );

    for (i, taxonomy) in client.taxonomies.iter().enumerate() {
        if let Err(e) = insert_taxonomy(&tx, import_id, taxonomy) {
            warnings.push(format!("Taxonomy {}: {}", taxonomy.name, e));
        }

        let _ = app.emit(
            "import-progress",
            ImportProgress {
                stage: "taxonomies".to_string(),
                message: format!("Importing taxonomy: {}", taxonomy.name),
                percent: 70 + (i * 10 / client.taxonomies.len().max(1)) as u32,
                current: Some(i),
                total: Some(client.taxonomies.len()),
            },
        );
    }

    // Import watchlists
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "watchlists".to_string(),
            message: "Importing watchlists...".to_string(),
            percent: 80,
            current: Some(0),
            total: Some(client.watchlists.len()),
        },
    );

    for watchlist in &client.watchlists {
        if let Err(e) = insert_watchlist(&tx, import_id, watchlist) {
            warnings.push(format!("Watchlist {}: {}", watchlist.name, e));
        }
    }

    // Link cross-entries
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "linking".to_string(),
            message: "Linking cross-entries...".to_string(),
            percent: 85,
            current: None,
            total: None,
        },
    );

    if let Err(e) = link_cross_entries(&tx, client) {
        warnings.push(format!("Cross-entries: {}", e));
    }

    // Build FIFO lots for cost basis calculation
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            stage: "fifo".to_string(),
            message: "Building FIFO lots...".to_string(),
            percent: 92,
            current: None,
            total: None,
        },
    );

    if let Err(e) = crate::fifo::build_all_fifo_lots(&tx) {
        warnings.push(format!("FIFO calculation: {}", e));
    }

    // Commit transaction
    tx.commit()?;

    Ok(ImportResult {
        import_id,
        file_path: path.to_string(),
        version: client.version,
        base_currency: client.base_currency.clone(),
        securities_count: client.securities.len(),
        accounts_count: client.accounts.len(),
        portfolios_count: client.portfolios.len(),
        transactions_count: total_transactions,
        prices_count: total_prices,
        warnings,
    })
}

/// Insert a security into the database
fn insert_security(tx: &rusqlite::Transaction, import_id: i64, security: &Security) -> Result<i64> {
    tx.execute(
        "INSERT INTO pp_security (import_id, uuid, name, currency, online_id, isin, wkn, ticker, calendar, feed, feed_url, latest_feed, latest_feed_url, is_retired, note, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT(uuid) DO UPDATE SET
           name = excluded.name,
           currency = excluded.currency,
           isin = excluded.isin,
           wkn = excluded.wkn,
           ticker = excluded.ticker,
           is_retired = excluded.is_retired,
           updated_at = excluded.updated_at",
        params![
            import_id,
            security.uuid,
            security.name,
            security.currency,
            security.online_id,
            security.isin,
            security.wkn,
            security.ticker,
            security.calendar,
            security.feed,
            security.feed_url,
            security.latest_feed,
            security.latest_feed_url,
            security.is_retired as i32,
            security.note,
            security.updated_at,
        ],
    )?;

    // Get the security ID
    let security_id: i64 = tx.query_row(
        "SELECT id FROM pp_security WHERE uuid = ?1",
        params![security.uuid],
        |row| row.get(0),
    )?;

    // Insert prices
    for price in &security.prices {
        tx.execute(
            "INSERT OR REPLACE INTO pp_price (security_id, date, value)
             VALUES (?1, ?2, ?3)",
            params![security_id, price.date.to_string(), price.value],
        )?;
    }

    // Insert latest price
    if let Some(ref latest) = security.latest {
        if latest.value.is_some() || latest.date.is_some() {
            tx.execute(
                "INSERT OR REPLACE INTO pp_latest_price (security_id, date, value, high, low, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    security_id,
                    latest.date.map(|d| d.to_string()),
                    latest.value,
                    latest.high,
                    latest.low,
                    latest.volume,
                ],
            )?;
        }
    }

    // Insert events
    for event_kind in &security.events {
        match event_kind {
            SecurityEventKind::Event(event) => {
                tx.execute(
                    "INSERT INTO pp_security_event (security_id, event_type, date, details, source, payment_date, amount, amount_currency)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        security_id,
                        event.event_type.as_str(),
                        event.date.to_string(),
                        event.details,
                        Option::<String>::None,
                        Option::<String>::None,
                        Option::<i64>::None,
                        Option::<String>::None,
                    ],
                )?;
            }
            SecurityEventKind::Dividend(dividend) => {
                tx.execute(
                    "INSERT INTO pp_security_event (security_id, event_type, date, details, source, payment_date, amount, amount_currency)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        security_id,
                        "DIVIDEND_PAYMENT",
                        dividend.date.to_string(),
                        Option::<String>::None,
                        dividend.source,
                        dividend.payment_date.map(|d| d.to_string()),
                        dividend.amount.as_ref().map(|m| m.amount),
                        dividend.amount.as_ref().map(|m| m.currency.clone()),
                    ],
                )?;
            }
        }
    }

    Ok(security_id)
}

/// Insert an account into the database
fn insert_account(tx: &rusqlite::Transaction, import_id: i64, account: &Account) -> Result<i64> {
    tx.execute(
        "INSERT INTO pp_account (import_id, uuid, name, currency, is_retired, note, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(uuid) DO UPDATE SET
           name = excluded.name,
           currency = excluded.currency,
           is_retired = excluded.is_retired,
           updated_at = excluded.updated_at",
        params![
            import_id,
            account.uuid,
            account.name,
            account.currency,
            account.is_retired as i32,
            account.note,
            account.updated_at,
        ],
    )?;

    // Get the account ID
    let account_id: i64 = tx.query_row(
        "SELECT id FROM pp_account WHERE uuid = ?1",
        params![account.uuid],
        |row| row.get(0),
    )?;

    // Insert transactions
    for txn in &account.transactions {
        insert_account_transaction(tx, account_id, txn)?;
    }

    Ok(account_id)
}

/// Insert an account transaction into the database
fn insert_account_transaction(
    tx: &rusqlite::Transaction,
    account_id: i64,
    txn: &AccountTransaction,
) -> Result<i64> {
    // Find security ID if exists
    let security_id: Option<i64> = txn.security_uuid.as_ref().and_then(|uuid| {
        tx.query_row(
            "SELECT id FROM pp_security WHERE uuid = ?1",
            params![uuid],
            |row| row.get(0),
        )
        .ok()
    });

    tx.execute(
        "INSERT INTO pp_txn (uuid, owner_type, owner_id, txn_type, date, amount, currency, shares, security_id, note, source, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(uuid) DO UPDATE SET
           txn_type = excluded.txn_type,
           date = excluded.date,
           amount = excluded.amount,
           shares = excluded.shares,
           updated_at = excluded.updated_at",
        params![
            txn.uuid,
            "account",
            account_id,
            txn.transaction_type.as_str(),
            txn.date.to_string(),
            txn.amount.amount,
            txn.amount.currency,
            txn.shares,
            security_id,
            txn.note,
            txn.source,
            txn.updated_at,
        ],
    )?;

    // Get the transaction ID
    let txn_id: i64 = tx.query_row(
        "SELECT id FROM pp_txn WHERE uuid = ?1",
        params![txn.uuid],
        |row| row.get(0),
    )?;

    // Delete existing units (for re-import scenarios to avoid duplicates)
    tx.execute("DELETE FROM pp_txn_unit WHERE txn_id = ?1", params![txn_id])?;

    // Insert units
    for unit in &txn.units {
        let (forex_amount, forex_currency, exchange_rate) = match &unit.forex {
            Some(forex) => (
                Some(forex.amount.amount),
                Some(forex.amount.currency.clone()),
                Some(forex.exchange_rate),
            ),
            None => (None, None, None),
        };

        tx.execute(
            "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency, forex_amount, forex_currency, exchange_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                txn_id,
                unit.unit_type.as_str(),
                unit.amount.amount,
                unit.amount.currency,
                forex_amount,
                forex_currency,
                exchange_rate,
            ],
        )?;
    }

    Ok(txn_id)
}

/// Insert a portfolio into the database
fn insert_portfolio(
    tx: &rusqlite::Transaction,
    import_id: i64,
    portfolio: &Portfolio,
) -> Result<i64> {
    // Find reference account ID if exists
    let ref_account_id: Option<i64> = portfolio.reference_account_uuid.as_ref().and_then(|uuid| {
        tx.query_row(
            "SELECT id FROM pp_account WHERE uuid = ?1",
            params![uuid],
            |row| row.get(0),
        )
        .ok()
    });

    tx.execute(
        "INSERT INTO pp_portfolio (import_id, uuid, name, reference_account_id, is_retired, note, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(uuid) DO UPDATE SET
           name = excluded.name,
           reference_account_id = excluded.reference_account_id,
           is_retired = excluded.is_retired,
           updated_at = excluded.updated_at",
        params![
            import_id,
            portfolio.uuid,
            portfolio.name,
            ref_account_id,
            portfolio.is_retired as i32,
            portfolio.note,
            portfolio.updated_at,
        ],
    )?;

    // Get the portfolio ID
    let portfolio_id: i64 = tx.query_row(
        "SELECT id FROM pp_portfolio WHERE uuid = ?1",
        params![portfolio.uuid],
        |row| row.get(0),
    )?;

    // Insert transactions
    for txn in &portfolio.transactions {
        insert_portfolio_transaction(tx, portfolio_id, txn)?;
    }

    Ok(portfolio_id)
}

/// Insert a portfolio transaction into the database
fn insert_portfolio_transaction(
    tx: &rusqlite::Transaction,
    portfolio_id: i64,
    txn: &PortfolioTransaction,
) -> Result<i64> {
    // Find security ID if exists
    let security_id: Option<i64> = txn.security_uuid.as_ref().and_then(|uuid| {
        tx.query_row(
            "SELECT id FROM pp_security WHERE uuid = ?1",
            params![uuid],
            |row| row.get(0),
        )
        .ok()
    });

    tx.execute(
        "INSERT INTO pp_txn (uuid, owner_type, owner_id, txn_type, date, amount, currency, shares, security_id, note, source, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(uuid) DO UPDATE SET
           txn_type = excluded.txn_type,
           date = excluded.date,
           amount = excluded.amount,
           shares = excluded.shares,
           updated_at = excluded.updated_at",
        params![
            txn.uuid,
            "portfolio",
            portfolio_id,
            txn.transaction_type.as_str(),
            txn.date.to_string(),
            txn.amount.amount,
            txn.amount.currency,
            Some(txn.shares),
            security_id,
            txn.note,
            txn.source,
            txn.updated_at,
        ],
    )?;

    // Get the transaction ID
    let txn_id: i64 = tx.query_row(
        "SELECT id FROM pp_txn WHERE uuid = ?1",
        params![txn.uuid],
        |row| row.get(0),
    )?;

    // Delete existing units (for re-import scenarios to avoid duplicates)
    tx.execute("DELETE FROM pp_txn_unit WHERE txn_id = ?1", params![txn_id])?;

    // Insert units
    for unit in &txn.units {
        let (forex_amount, forex_currency, exchange_rate) = match &unit.forex {
            Some(forex) => (
                Some(forex.amount.amount),
                Some(forex.amount.currency.clone()),
                Some(forex.exchange_rate),
            ),
            None => (None, None, None),
        };

        tx.execute(
            "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency, forex_amount, forex_currency, exchange_rate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                txn_id,
                unit.unit_type.as_str(),
                unit.amount.amount,
                unit.amount.currency,
                forex_amount,
                forex_currency,
                exchange_rate,
            ],
        )?;
    }

    Ok(txn_id)
}

/// Insert a taxonomy into the database
fn insert_taxonomy(tx: &rusqlite::Transaction, import_id: i64, taxonomy: &Taxonomy) -> Result<i64> {
    tx.execute(
        "INSERT INTO pp_taxonomy (import_id, uuid, name, source)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(uuid) DO UPDATE SET
           name = excluded.name,
           source = excluded.source",
        params![import_id, taxonomy.id, taxonomy.name, taxonomy.source],
    )?;

    let taxonomy_id: i64 = tx.query_row(
        "SELECT id FROM pp_taxonomy WHERE uuid = ?1",
        params![taxonomy.id],
        |row| row.get(0),
    )?;

    // Insert root classification and its children
    if let Some(ref root) = taxonomy.root {
        insert_classification(tx, taxonomy_id, None, root)?;
    }

    Ok(taxonomy_id)
}

/// Recursively insert classifications
fn insert_classification(
    tx: &rusqlite::Transaction,
    taxonomy_id: i64,
    parent_id: Option<i64>,
    classification: &Classification,
) -> Result<i64> {
    tx.execute(
        "INSERT INTO pp_classification (taxonomy_id, uuid, parent_id, name, color, weight, rank)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(uuid) DO UPDATE SET
           name = excluded.name,
           color = excluded.color,
           weight = excluded.weight",
        params![
            taxonomy_id,
            classification.id,
            parent_id,
            classification.name,
            classification.color,
            classification.weight,
            classification.rank,
        ],
    )?;

    let class_id: i64 = tx.query_row(
        "SELECT id FROM pp_classification WHERE uuid = ?1",
        params![classification.id],
        |row| row.get(0),
    )?;

    // Insert assignments
    for assignment in &classification.assignments {
        tx.execute(
            "INSERT OR REPLACE INTO pp_classification_assignment (classification_id, vehicle_type, vehicle_uuid, weight, rank)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                class_id,
                assignment.vehicle_class,
                assignment.vehicle_uuid,
                assignment.weight,
                assignment.rank,
            ],
        )?;
    }

    // Recursively insert children
    for child in &classification.children {
        insert_classification(tx, taxonomy_id, Some(class_id), child)?;
    }

    Ok(class_id)
}

/// Insert a watchlist into the database
fn insert_watchlist(
    tx: &rusqlite::Transaction,
    import_id: i64,
    watchlist: &pp::Watchlist,
) -> Result<i64> {
    tx.execute(
        "INSERT INTO pp_watchlist (import_id, name)
         VALUES (?1, ?2)",
        params![import_id, watchlist.name],
    )?;

    let watchlist_id = tx.last_insert_rowid();

    // Link securities
    for sec_uuid in &watchlist.security_uuids {
        // Try to find the security
        if let Ok(security_id) = tx.query_row::<i64, _, _>(
            "SELECT id FROM pp_security WHERE uuid = ?1",
            params![sec_uuid],
            |row| row.get(0),
        ) {
            tx.execute(
                "INSERT OR IGNORE INTO pp_watchlist_security (watchlist_id, security_id)
                 VALUES (?1, ?2)",
                params![watchlist_id, security_id],
            )?;
        }
    }

    Ok(watchlist_id)
}

/// Link cross-entries between transactions
fn link_cross_entries(tx: &rusqlite::Transaction, client: &Client) -> Result<()> {
    use crate::pp::transaction::CrossEntryType;
    use std::collections::HashSet;

    // Track processed cross-entries by their source+target UUIDs to avoid duplicates
    let mut processed: HashSet<(String, String)> = HashSet::new();

    // Helper to insert cross-entry and update transactions
    let mut insert_cross_entry = |cross: &crate::pp::transaction::CrossEntry, _txn_uuid: &str| -> Result<()> {
        // Avoid duplicates (each cross-entry is referenced by both source and target txn)
        let key = if cross.source_uuid < cross.target_uuid {
            (cross.source_uuid.clone(), cross.target_uuid.clone())
        } else {
            (cross.target_uuid.clone(), cross.source_uuid.clone())
        };

        if processed.contains(&key) {
            return Ok(());
        }
        processed.insert(key);

        // Find source and target transaction IDs
        let source_id: i64 = tx.query_row(
            "SELECT id FROM pp_txn WHERE uuid = ?1",
            params![cross.source_uuid],
            |row| row.get(0),
        )?;

        let target_id: i64 = tx.query_row(
            "SELECT id FROM pp_txn WHERE uuid = ?1",
            params![cross.target_uuid],
            |row| row.get(0),
        )?;

        // Generate UUID for cross-entry
        let ce_uuid = uuid::Uuid::new_v4().to_string();

        // Insert based on entry type
        match cross.entry_type {
            CrossEntryType::PortfolioTransfer => {
                // source = TRANSFER_OUT, target = TRANSFER_IN
                tx.execute(
                    "INSERT INTO pp_cross_entry (uuid, entry_type, from_txn_id, to_txn_id)
                     VALUES (?1, 'PORTFOLIO_TRANSFER', ?2, ?3)",
                    params![ce_uuid, source_id, target_id],
                )?;
            }
            CrossEntryType::AccountTransfer => {
                tx.execute(
                    "INSERT INTO pp_cross_entry (uuid, entry_type, from_txn_id, to_txn_id)
                     VALUES (?1, 'ACCOUNT_TRANSFER', ?2, ?3)",
                    params![ce_uuid, source_id, target_id],
                )?;
            }
            CrossEntryType::BuySell => {
                // Determine which is portfolio and which is account
                let source_owner: String = tx.query_row(
                    "SELECT owner_type FROM pp_txn WHERE id = ?1",
                    params![source_id],
                    |row| row.get(0),
                )?;

                let (portfolio_txn_id, account_txn_id) = if source_owner == "portfolio" {
                    (source_id, target_id)
                } else {
                    (target_id, source_id)
                };

                tx.execute(
                    "INSERT INTO pp_cross_entry (uuid, entry_type, portfolio_txn_id, account_txn_id)
                     VALUES (?1, 'BUY_SELL', ?2, ?3)",
                    params![ce_uuid, portfolio_txn_id, account_txn_id],
                )?;
            }
        }

        // Get the cross_entry ID
        let cross_entry_id = tx.last_insert_rowid();

        // Update both transactions with cross_entry_id
        tx.execute(
            "UPDATE pp_txn SET cross_entry_id = ?1 WHERE id = ?2",
            params![cross_entry_id, source_id],
        )?;
        tx.execute(
            "UPDATE pp_txn SET cross_entry_id = ?1 WHERE id = ?2",
            params![cross_entry_id, target_id],
        )?;

        Ok(())
    };

    // Process account transactions
    for account in &client.accounts {
        for txn in &account.transactions {
            if let Some(ref cross) = txn.cross_entry {
                if let Err(e) = insert_cross_entry(cross, &txn.uuid) {
                    log::warn!("Failed to link cross-entry for account txn {}: {}", txn.uuid, e);
                }
            }
        }
    }

    // Process portfolio transactions
    for portfolio in &client.portfolios {
        for txn in &portfolio.transactions {
            if let Some(ref cross) = txn.cross_entry {
                if let Err(e) = insert_cross_entry(cross, &txn.uuid) {
                    log::warn!("Failed to link cross-entry for portfolio txn {}: {}", txn.uuid, e);
                }
            }
        }
    }

    Ok(())
}

/// Get list of previous imports
#[command]
pub fn get_imports() -> Result<Vec<ImportInfo>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, file_path, imported_at, version, base_currency,
                    securities_count, accounts_count, portfolios_count, transactions_count
             FROM pp_import ORDER BY imported_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let imports = stmt
        .query_map([], |row| {
            Ok(ImportInfo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                imported_at: row.get(2)?,
                version: row.get(3)?,
                base_currency: row.get(4)?,
                securities_count: row.get(5)?,
                accounts_count: row.get(6)?,
                portfolios_count: row.get(7)?,
                transactions_count: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(imports)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportInfo {
    pub id: i64,
    pub file_path: String,
    pub imported_at: String,
    pub version: i32,
    pub base_currency: String,
    pub securities_count: i32,
    pub accounts_count: i32,
    pub portfolios_count: i32,
    pub transactions_count: i32,
}

/// Delete an import and all related data
#[command]
pub fn delete_import(import_id: i64) -> Result<(), String> {
    let mut conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_mut()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Delete will cascade to all related tables
    conn.execute("DELETE FROM pp_import WHERE id = ?1", params![import_id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Save exchange rates to the database
fn save_exchange_rates(rates: &[ExchangeRate]) -> Result<()> {
    let mut conn_guard = db::get_connection()?;
    let conn = conn_guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

    let tx = conn.transaction()?;

    for rate in rates {
        // Store rate as decimal string for precision
        tx.execute(
            "INSERT OR REPLACE INTO pp_exchange_rate (base_currency, term_currency, date, rate)
             VALUES (?, ?, ?, ?)",
            params![rate.base, rate.target, rate.date.to_string(), rate.rate.to_string()],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Rebuild all FIFO lots from transactions
/// Call this after fixing FIFO calculation logic to recalculate cost basis
#[command]
pub fn rebuild_fifo_lots() -> Result<RebuildFifoResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Count securities before rebuild
    let security_count: i64 = conn
        .query_row("SELECT COUNT(DISTINCT id) FROM pp_security", [], |row| row.get(0))
        .unwrap_or(0);

    // Rebuild FIFO lots for all securities
    crate::fifo::build_all_fifo_lots(conn).map_err(|e| e.to_string())?;

    // Count lots after rebuild
    let lot_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM pp_fifo_lot WHERE remaining_shares > 0", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(RebuildFifoResult {
        securities_processed: security_count as usize,
        lots_created: lot_count as usize,
    })
}

/// Result of FIFO rebuild
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildFifoResult {
    pub securities_processed: usize,
    pub lots_created: usize,
}
