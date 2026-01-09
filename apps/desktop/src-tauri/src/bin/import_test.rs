//! Test binary to import Portfolio.portfolio into SQLite
//!
//! Run with: cargo run --bin import_test

use anyhow::Result;
use rusqlite::params;
use std::path::PathBuf;

fn main() -> Result<()> {
    let portfolio_path = PathBuf::from("/Users/ricoullmann/Documents/PP/Portfolio.portfolio");
    let db_path = PathBuf::from("/Users/ricoullmann/Documents/PP/test_import.db");

    println!("=== Portfolio Performance Import Test ===\n");
    println!("Source: {}", portfolio_path.display());
    println!("Target: {}", db_path.display());

    // Remove old test DB if exists
    if db_path.exists() {
        std::fs::remove_file(&db_path)?;
        println!("Removed old test database\n");
    }

    // Initialize database
    println!("Initializing database...");
    let conn = init_database(&db_path)?;

    // Parse portfolio file
    println!("Parsing portfolio file...");
    let client = app_lib::protobuf::parse_portfolio_file(&portfolio_path)?;

    println!("\n=== Parsed Data ===");
    println!("Version: {}", client.version);
    println!("Base Currency: {}", client.base_currency);
    println!("Securities: {}", client.securities.len());
    println!("Accounts: {}", client.accounts.len());
    println!("Portfolios: {}", client.portfolios.len());

    let total_portfolio_txns: usize = client.portfolios.iter().map(|p| p.transactions.len()).sum();
    let total_account_txns: usize = client.accounts.iter().map(|a| a.transactions.len()).sum();
    println!("Portfolio Transactions: {}", total_portfolio_txns);
    println!("Account Transactions: {}", total_account_txns);

    // Import to database
    println!("\n=== Importing to SQLite ===");
    import_client_to_db(&conn, &portfolio_path, &client)?;

    // Verify import
    println!("\n=== Verifying Import ===");
    verify_import(&conn)?;

    // Calculate holdings
    println!("\n=== Calculating Holdings ===");
    calculate_holdings(&conn)?;

    println!("\n=== Import Complete ===");
    println!("Database saved to: {}", db_path.display());

    Ok(())
}

fn init_database(path: &PathBuf) -> Result<rusqlite::Connection> {
    let conn = rusqlite::Connection::open(path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS pp_import (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            file_hash TEXT,
            imported_at TEXT NOT NULL DEFAULT (datetime('now')),
            version INTEGER NOT NULL,
            base_currency TEXT NOT NULL,
            securities_count INTEGER NOT NULL DEFAULT 0,
            accounts_count INTEGER NOT NULL DEFAULT 0,
            portfolios_count INTEGER NOT NULL DEFAULT 0,
            transactions_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS pp_security (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            currency TEXT NOT NULL DEFAULT 'EUR',
            isin TEXT,
            wkn TEXT,
            ticker TEXT,
            is_retired INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS pp_price (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            security_id INTEGER NOT NULL,
            date TEXT NOT NULL,
            value INTEGER NOT NULL,
            UNIQUE(security_id, date),
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS pp_latest_price (
            security_id INTEGER PRIMARY KEY,
            date TEXT,
            value INTEGER,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS pp_account (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            currency TEXT NOT NULL DEFAULT 'EUR',
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS pp_portfolio (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS pp_txn (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            owner_type TEXT NOT NULL CHECK(owner_type IN ('account', 'portfolio')),
            owner_id INTEGER NOT NULL,
            txn_type TEXT NOT NULL,
            date TEXT NOT NULL,
            amount INTEGER NOT NULL,
            currency TEXT NOT NULL,
            shares INTEGER,
            security_id INTEGER,
            note TEXT,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_pp_security_isin ON pp_security(isin);
        CREATE INDEX IF NOT EXISTS idx_pp_txn_owner ON pp_txn(owner_type, owner_id);
        CREATE INDEX IF NOT EXISTS idx_pp_txn_security ON pp_txn(security_id);
        "#,
    )?;

    Ok(conn)
}

fn import_client_to_db(
    conn: &rusqlite::Connection,
    path: &PathBuf,
    client: &app_lib::pp::Client,
) -> Result<i64> {
    let tx = conn.unchecked_transaction()?;

    // Calculate totals
    let total_txns: usize = client.portfolios.iter().map(|p| p.transactions.len()).sum::<usize>()
        + client.accounts.iter().map(|a| a.transactions.len()).sum::<usize>();

    // Create import record
    tx.execute(
        "INSERT INTO pp_import (file_path, version, base_currency, securities_count, accounts_count, portfolios_count, transactions_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            path.to_string_lossy(),
            client.version,
            client.base_currency,
            client.securities.len(),
            client.accounts.len(),
            client.portfolios.len(),
            total_txns,
        ],
    )?;
    let import_id = tx.last_insert_rowid();
    println!("Created import record: {}", import_id);

    // Import securities
    println!("Importing {} securities...", client.securities.len());
    for security in &client.securities {
        tx.execute(
            "INSERT INTO pp_security (import_id, uuid, name, currency, isin, wkn, ticker, is_retired)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                import_id,
                security.uuid,
                security.name,
                security.currency,
                security.isin,
                security.wkn,
                security.ticker,
                security.is_retired as i32,
            ],
        )?;

        let security_id: i64 = tx.query_row(
            "SELECT id FROM pp_security WHERE uuid = ?1",
            params![security.uuid],
            |row| row.get(0),
        )?;

        // Insert latest price
        if let Some(ref latest) = security.latest {
            if latest.value.is_some() {
                tx.execute(
                    "INSERT OR REPLACE INTO pp_latest_price (security_id, date, value)
                     VALUES (?1, ?2, ?3)",
                    params![
                        security_id,
                        latest.date.map(|d| d.to_string()),
                        latest.value,
                    ],
                )?;
            }
        }
    }

    // Import accounts
    println!("Importing {} accounts...", client.accounts.len());
    for account in &client.accounts {
        tx.execute(
            "INSERT INTO pp_account (import_id, uuid, name, currency)
             VALUES (?1, ?2, ?3, ?4)",
            params![import_id, account.uuid, account.name, account.currency],
        )?;

        let account_id: i64 = tx.query_row(
            "SELECT id FROM pp_account WHERE uuid = ?1",
            params![account.uuid],
            |row| row.get(0),
        )?;

        // Import account transactions
        for txn in &account.transactions {
            let security_id: Option<i64> = txn.security_uuid.as_ref().and_then(|uuid| {
                tx.query_row(
                    "SELECT id FROM pp_security WHERE uuid = ?1",
                    params![uuid],
                    |row| row.get(0),
                )
                .ok()
            });

            tx.execute(
                "INSERT INTO pp_txn (uuid, owner_type, owner_id, txn_type, date, amount, currency, shares, security_id, note)
                 VALUES (?1, 'account', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    txn.uuid,
                    account_id,
                    txn.transaction_type.as_str(),
                    txn.date.to_string(),
                    txn.amount.amount,
                    txn.amount.currency,
                    txn.shares,
                    security_id,
                    txn.note,
                ],
            )?;
        }
    }

    // Import portfolios
    println!("Importing {} portfolios...", client.portfolios.len());
    for portfolio in &client.portfolios {
        tx.execute(
            "INSERT INTO pp_portfolio (import_id, uuid, name)
             VALUES (?1, ?2, ?3)",
            params![import_id, portfolio.uuid, portfolio.name],
        )?;

        let portfolio_id: i64 = tx.query_row(
            "SELECT id FROM pp_portfolio WHERE uuid = ?1",
            params![portfolio.uuid],
            |row| row.get(0),
        )?;

        // Import portfolio transactions
        for txn in &portfolio.transactions {
            let security_id: Option<i64> = txn.security_uuid.as_ref().and_then(|uuid| {
                tx.query_row(
                    "SELECT id FROM pp_security WHERE uuid = ?1",
                    params![uuid],
                    |row| row.get(0),
                )
                .ok()
            });

            tx.execute(
                "INSERT INTO pp_txn (uuid, owner_type, owner_id, txn_type, date, amount, currency, shares, security_id, note)
                 VALUES (?1, 'portfolio', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    txn.uuid,
                    portfolio_id,
                    txn.transaction_type.as_str(),
                    txn.date.to_string(),
                    txn.amount.amount,
                    txn.amount.currency,
                    Some(txn.shares),
                    security_id,
                    txn.note,
                ],
            )?;
        }
    }

    tx.commit()?;
    Ok(import_id)
}

fn verify_import(conn: &rusqlite::Connection) -> Result<()> {
    let security_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_security", [], |r| r.get(0))?;
    let account_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_account", [], |r| r.get(0))?;
    let portfolio_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_portfolio", [], |r| r.get(0))?;
    let txn_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_txn", [], |r| r.get(0))?;
    let portfolio_txn_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'portfolio'",
        [],
        |r| r.get(0),
    )?;
    let account_txn_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'account'",
        [],
        |r| r.get(0),
    )?;

    println!("Securities in DB: {}", security_count);
    println!("Accounts in DB: {}", account_count);
    println!("Portfolios in DB: {}", portfolio_count);
    println!("Total Transactions: {}", txn_count);
    println!("  - Portfolio Transactions: {}", portfolio_txn_count);
    println!("  - Account Transactions: {}", account_txn_count);

    // Show transaction types
    println!("\nTransaction types:");
    let mut stmt = conn.prepare(
        "SELECT txn_type, COUNT(*) FROM pp_txn WHERE owner_type = 'portfolio' GROUP BY txn_type ORDER BY COUNT(*) DESC",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let txn_type: String = row.get(0)?;
        let count: i32 = row.get(1)?;
        println!("  {}: {}", txn_type, count);
    }

    Ok(())
}

fn calculate_holdings(conn: &rusqlite::Connection) -> Result<()> {
    println!("\nHoldings per Portfolio (shares > 0):\n");

    let sql = r#"
        SELECT
            p.name as portfolio_name,
            s.name as security_name,
            s.isin,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT') THEN -t.shares
                ELSE 0
            END) as net_shares,
            lp.value as latest_price
        FROM pp_txn t
        JOIN pp_portfolio p ON t.owner_id = p.id AND t.owner_type = 'portfolio'
        JOIN pp_security s ON t.security_id = s.id
        LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
        GROUP BY p.id, s.id
        HAVING net_shares > 0
        ORDER BY p.name, net_shares DESC
    "#;

    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query([])?;

    let mut current_portfolio = String::new();
    let mut total_holdings = 0;
    let mut total_value: f64 = 0.0;

    while let Some(row) = rows.next()? {
        let portfolio: String = row.get(0)?;
        let security: String = row.get(1)?;
        let isin: Option<String> = row.get(2)?;
        let shares: i64 = row.get(3)?;
        let price: Option<i64> = row.get(4)?;

        if portfolio != current_portfolio {
            if !current_portfolio.is_empty() {
                println!();
            }
            println!("Portfolio: {}", portfolio);
            current_portfolio = portfolio;
        }

        let shares_decimal = shares as f64 / 100_000_000.0;
        let price_decimal = price.map(|p| p as f64 / 100_000_000.0).unwrap_or(0.0);
        let value = shares_decimal * price_decimal;
        total_value += value;
        total_holdings += 1;

        println!(
            "  {} | {} | {:.4} Stück | {:.2} EUR | Wert: {:.2} EUR",
            isin.as_deref().unwrap_or("-"),
            security,
            shares_decimal,
            price_decimal,
            value
        );
    }

    println!("\n=== ZUSAMMENFASSUNG ===");
    println!("Aktive Positionen: {}", total_holdings);
    println!("Gesamtwert: {:.2} EUR", total_value);

    Ok(())
}
