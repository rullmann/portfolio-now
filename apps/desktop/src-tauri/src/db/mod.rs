use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub static DB: once_cell::sync::Lazy<Mutex<Option<Connection>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

pub fn init_database(path: &Path) -> Result<()> {
    let conn = Connection::open(path)?;

    // Enable WAL mode for better concurrent access
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

    conn.execute_batch(
        r#"
        -- =============================================================================
        -- Legacy tables (for backward compatibility)
        -- =============================================================================

        CREATE TABLE IF NOT EXISTS portfolios (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            base_currency TEXT NOT NULL DEFAULT 'EUR',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            note TEXT
        );

        CREATE TABLE IF NOT EXISTS securities (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            security_type TEXT NOT NULL,
            isin TEXT,
            wkn TEXT,
            ticker TEXT,
            currency TEXT NOT NULL DEFAULT 'EUR',
            feed TEXT,
            feed_url TEXT,
            latest_price REAL,
            latest_price_date TEXT,
            note TEXT,
            is_retired INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            portfolio_id TEXT NOT NULL,
            name TEXT NOT NULL,
            account_type TEXT NOT NULL,
            currency TEXT NOT NULL DEFAULT 'EUR',
            is_retired INTEGER NOT NULL DEFAULT 0,
            note TEXT,
            FOREIGN KEY (portfolio_id) REFERENCES portfolios(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS transactions (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            transaction_type TEXT NOT NULL,
            date TEXT NOT NULL,
            security_id TEXT,
            shares REAL,
            amount REAL NOT NULL,
            currency_gross_amount REAL,
            exchange_rate REAL,
            fees REAL NOT NULL DEFAULT 0,
            taxes REAL NOT NULL DEFAULT 0,
            note TEXT,
            source TEXT,
            FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
            FOREIGN KEY (security_id) REFERENCES securities(id)
        );

        CREATE TABLE IF NOT EXISTS prices (
            security_id TEXT NOT NULL,
            date TEXT NOT NULL,
            close REAL NOT NULL,
            high REAL,
            low REAL,
            volume INTEGER,
            PRIMARY KEY (security_id, date),
            FOREIGN KEY (security_id) REFERENCES securities(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS taxonomies (
            id TEXT PRIMARY KEY,
            portfolio_id TEXT NOT NULL,
            name TEXT NOT NULL,
            FOREIGN KEY (portfolio_id) REFERENCES portfolios(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS taxonomy_classifications (
            id TEXT PRIMARY KEY,
            taxonomy_id TEXT NOT NULL,
            parent_id TEXT,
            name TEXT NOT NULL,
            color TEXT,
            weight REAL,
            FOREIGN KEY (taxonomy_id) REFERENCES taxonomies(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_id) REFERENCES taxonomy_classifications(id)
        );

        CREATE TABLE IF NOT EXISTS taxonomy_assignments (
            classification_id TEXT NOT NULL,
            security_id TEXT NOT NULL,
            weight REAL NOT NULL DEFAULT 100,
            PRIMARY KEY (classification_id, security_id),
            FOREIGN KEY (classification_id) REFERENCES taxonomy_classifications(id) ON DELETE CASCADE,
            FOREIGN KEY (security_id) REFERENCES securities(id) ON DELETE CASCADE
        );

        -- =============================================================================
        -- New PP (Portfolio Performance) tables for full XML import support
        -- =============================================================================

        -- Import history to track imported files
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

        -- PP Securities with full PP field support
        CREATE TABLE IF NOT EXISTS pp_security (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            currency TEXT NOT NULL DEFAULT 'EUR',
            online_id TEXT,
            isin TEXT,
            wkn TEXT,
            ticker TEXT,
            calendar TEXT,
            feed TEXT,
            feed_url TEXT,
            latest_feed TEXT,
            latest_feed_url TEXT,
            is_retired INTEGER NOT NULL DEFAULT 0,
            note TEXT,
            updated_at TEXT,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
        );

        -- PP Prices (stored as value × 10^8)
        CREATE TABLE IF NOT EXISTS pp_price (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            security_id INTEGER NOT NULL,
            date TEXT NOT NULL,
            value INTEGER NOT NULL,
            UNIQUE(security_id, date),
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
        );

        -- PP Latest price with market data
        CREATE TABLE IF NOT EXISTS pp_latest_price (
            security_id INTEGER PRIMARY KEY,
            date TEXT,
            value INTEGER,
            high INTEGER,
            low INTEGER,
            volume INTEGER,
            updated_at TEXT,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
        );

        -- PP Security events (stock splits, dividends)
        CREATE TABLE IF NOT EXISTS pp_security_event (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            security_id INTEGER NOT NULL,
            event_type TEXT NOT NULL,
            date TEXT NOT NULL,
            details TEXT,
            source TEXT,
            payment_date TEXT,
            amount INTEGER,
            amount_currency TEXT,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
        );

        -- PP Accounts (cash accounts)
        CREATE TABLE IF NOT EXISTS pp_account (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            currency TEXT NOT NULL DEFAULT 'EUR',
            is_retired INTEGER NOT NULL DEFAULT 0,
            note TEXT,
            updated_at TEXT,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
        );

        -- PP Portfolios (securities depots)
        CREATE TABLE IF NOT EXISTS pp_portfolio (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            reference_account_id INTEGER,
            is_retired INTEGER NOT NULL DEFAULT 0,
            note TEXT,
            updated_at TEXT,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE,
            FOREIGN KEY (reference_account_id) REFERENCES pp_account(id) ON DELETE SET NULL
        );

        -- PP Transactions (unified for account and portfolio transactions)
        -- Follows Portfolio Performance model:
        -- Account types: DEPOSIT, REMOVAL, INTEREST, INTEREST_CHARGE, DIVIDENDS,
        --                FEES, FEES_REFUND, TAXES, TAX_REFUND, BUY, SELL,
        --                TRANSFER_IN, TRANSFER_OUT
        -- Portfolio types: BUY, SELL, TRANSFER_IN, TRANSFER_OUT,
        --                  DELIVERY_INBOUND, DELIVERY_OUTBOUND
        CREATE TABLE IF NOT EXISTS pp_txn (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            owner_type TEXT NOT NULL CHECK(owner_type IN ('account', 'portfolio')),
            owner_id INTEGER NOT NULL,
            txn_type TEXT NOT NULL,
            date TEXT NOT NULL,
            amount INTEGER NOT NULL,          -- scale: 10^2 (cents)
            currency TEXT NOT NULL,
            shares INTEGER,                   -- scale: 10^8
            security_id INTEGER,
            note TEXT,
            source TEXT,
            cross_entry_id INTEGER,           -- links to pp_cross_entry
            updated_at TEXT,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE SET NULL,
            FOREIGN KEY (cross_entry_id) REFERENCES pp_cross_entry(id) ON DELETE SET NULL
        );

        -- PP Transaction Units (fees, taxes, forex)
        CREATE TABLE IF NOT EXISTS pp_txn_unit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            txn_id INTEGER NOT NULL,
            unit_type TEXT NOT NULL CHECK(unit_type IN ('FEE', 'TAX', 'GROSS_VALUE')),
            amount INTEGER NOT NULL,
            currency TEXT NOT NULL,
            forex_amount INTEGER,
            forex_currency TEXT,
            exchange_rate REAL,
            FOREIGN KEY (txn_id) REFERENCES pp_txn(id) ON DELETE CASCADE
        );

        -- PP Cross-entries (linked transactions)
        -- BUY_SELL: Portfolio transaction + Account transaction (buy/sell pair)
        -- ACCOUNT_TRANSFER: Two account transactions (transfer between accounts)
        -- PORTFOLIO_TRANSFER: Two portfolio transactions (transfer between portfolios)
        CREATE TABLE IF NOT EXISTS pp_cross_entry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT UNIQUE NOT NULL,
            entry_type TEXT NOT NULL CHECK(entry_type IN ('BUY_SELL', 'ACCOUNT_TRANSFER', 'PORTFOLIO_TRANSFER')),
            source TEXT,                       -- data source reference
            -- For BUY_SELL: links portfolio txn (source) and account txn (target)
            -- For transfers: links source txn and target txn
            portfolio_txn_id INTEGER,          -- portfolio side of BUY_SELL
            account_txn_id INTEGER,            -- account side of BUY_SELL
            from_txn_id INTEGER,               -- source of transfer
            to_txn_id INTEGER,                 -- destination of transfer
            FOREIGN KEY (portfolio_txn_id) REFERENCES pp_txn(id) ON DELETE CASCADE,
            FOREIGN KEY (account_txn_id) REFERENCES pp_txn(id) ON DELETE CASCADE,
            FOREIGN KEY (from_txn_id) REFERENCES pp_txn(id) ON DELETE CASCADE,
            FOREIGN KEY (to_txn_id) REFERENCES pp_txn(id) ON DELETE CASCADE
        );

        -- PP FIFO Lots (tracks cost basis for each purchase lot)
        -- Used for accurate Einstandswert calculation
        CREATE TABLE IF NOT EXISTS pp_fifo_lot (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            security_id INTEGER NOT NULL,
            portfolio_id INTEGER NOT NULL,
            purchase_txn_id INTEGER NOT NULL,  -- the transaction that created this lot
            purchase_date TEXT NOT NULL,
            original_shares INTEGER NOT NULL,  -- scale: 10^8, original purchase amount
            remaining_shares INTEGER NOT NULL, -- scale: 10^8, shares still held
            gross_amount INTEGER NOT NULL,     -- scale: 10^2, cost INCLUDING fees/taxes (= Purchase Value per PP)
            net_amount INTEGER NOT NULL,       -- scale: 10^2, cost EXCLUDING fees/taxes
            currency TEXT NOT NULL,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE,
            FOREIGN KEY (portfolio_id) REFERENCES pp_portfolio(id) ON DELETE CASCADE,
            FOREIGN KEY (purchase_txn_id) REFERENCES pp_txn(id) ON DELETE CASCADE
        );

        -- PP FIFO Lot Consumptions (tracks which lots were used for sales)
        CREATE TABLE IF NOT EXISTS pp_fifo_consumption (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lot_id INTEGER NOT NULL,
            sale_txn_id INTEGER NOT NULL,      -- the sell transaction
            shares_consumed INTEGER NOT NULL,  -- scale: 10^8
            gross_amount INTEGER NOT NULL,     -- scale: 10^2, proportional cost
            net_amount INTEGER NOT NULL,       -- scale: 10^2
            FOREIGN KEY (lot_id) REFERENCES pp_fifo_lot(id) ON DELETE CASCADE,
            FOREIGN KEY (sale_txn_id) REFERENCES pp_txn(id) ON DELETE CASCADE
        );

        -- PP Taxonomies
        CREATE TABLE IF NOT EXISTS pp_taxonomy (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            uuid TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            source TEXT,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
        );

        -- PP Classifications (hierarchical)
        CREATE TABLE IF NOT EXISTS pp_classification (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            taxonomy_id INTEGER NOT NULL,
            uuid TEXT UNIQUE NOT NULL,
            parent_id INTEGER,
            name TEXT NOT NULL,
            color TEXT,
            weight INTEGER,
            rank INTEGER,
            FOREIGN KEY (taxonomy_id) REFERENCES pp_taxonomy(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_id) REFERENCES pp_classification(id) ON DELETE CASCADE
        );

        -- PP Classification assignments
        CREATE TABLE IF NOT EXISTS pp_classification_assignment (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            classification_id INTEGER NOT NULL,
            vehicle_type TEXT NOT NULL CHECK(vehicle_type IN ('security', 'account')),
            vehicle_uuid TEXT NOT NULL,
            weight INTEGER NOT NULL DEFAULT 10000,
            rank INTEGER,
            FOREIGN KEY (classification_id) REFERENCES pp_classification(id) ON DELETE CASCADE
        );

        -- PP Watchlists
        CREATE TABLE IF NOT EXISTS pp_watchlist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            import_id INTEGER,
            name TEXT NOT NULL,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS pp_watchlist_security (
            watchlist_id INTEGER NOT NULL,
            security_id INTEGER NOT NULL,
            PRIMARY KEY (watchlist_id, security_id),
            FOREIGN KEY (watchlist_id) REFERENCES pp_watchlist(id) ON DELETE CASCADE,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
        );

        -- =============================================================================
        -- Indices for legacy tables
        -- =============================================================================

        CREATE INDEX IF NOT EXISTS idx_accounts_portfolio ON accounts(portfolio_id);
        CREATE INDEX IF NOT EXISTS idx_transactions_account ON transactions(account_id);
        CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
        CREATE INDEX IF NOT EXISTS idx_prices_security ON prices(security_id);
        CREATE INDEX IF NOT EXISTS idx_securities_isin ON securities(isin);

        -- =============================================================================
        -- Indices for PP tables
        -- =============================================================================

        CREATE INDEX IF NOT EXISTS idx_pp_security_isin ON pp_security(isin);
        CREATE INDEX IF NOT EXISTS idx_pp_security_uuid ON pp_security(uuid);
        CREATE INDEX IF NOT EXISTS idx_pp_price_security_date ON pp_price(security_id, date);
        CREATE INDEX IF NOT EXISTS idx_pp_account_uuid ON pp_account(uuid);
        CREATE INDEX IF NOT EXISTS idx_pp_portfolio_uuid ON pp_portfolio(uuid);
        CREATE INDEX IF NOT EXISTS idx_pp_txn_owner ON pp_txn(owner_type, owner_id);
        CREATE INDEX IF NOT EXISTS idx_pp_txn_date ON pp_txn(date);
        CREATE INDEX IF NOT EXISTS idx_pp_txn_security ON pp_txn(security_id);
        CREATE INDEX IF NOT EXISTS idx_pp_txn_uuid ON pp_txn(uuid);
        CREATE INDEX IF NOT EXISTS idx_pp_txn_unit_txn ON pp_txn_unit(txn_id);
        CREATE INDEX IF NOT EXISTS idx_pp_classification_taxonomy ON pp_classification(taxonomy_id);

        -- FIFO lot indices
        CREATE INDEX IF NOT EXISTS idx_pp_fifo_lot_security ON pp_fifo_lot(security_id);
        CREATE INDEX IF NOT EXISTS idx_pp_fifo_lot_portfolio ON pp_fifo_lot(portfolio_id);
        CREATE INDEX IF NOT EXISTS idx_pp_fifo_lot_remaining ON pp_fifo_lot(remaining_shares) WHERE remaining_shares > 0;
        CREATE INDEX IF NOT EXISTS idx_pp_fifo_consumption_lot ON pp_fifo_consumption(lot_id);
        CREATE INDEX IF NOT EXISTS idx_pp_fifo_consumption_sale ON pp_fifo_consumption(sale_txn_id);

        -- Cross-entry indices
        CREATE INDEX IF NOT EXISTS idx_pp_cross_entry_uuid ON pp_cross_entry(uuid);
        CREATE INDEX IF NOT EXISTS idx_pp_cross_entry_portfolio ON pp_cross_entry(portfolio_txn_id);
        CREATE INDEX IF NOT EXISTS idx_pp_cross_entry_account ON pp_cross_entry(account_txn_id);

        -- PP Exchange rates for forex calculations
        CREATE TABLE IF NOT EXISTS pp_exchange_rate (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            base_currency TEXT NOT NULL,
            term_currency TEXT NOT NULL,
            date TEXT NOT NULL,
            rate TEXT NOT NULL,                -- stored as decimal string for precision
            provider TEXT,
            UNIQUE(base_currency, term_currency, date)
        );

        CREATE INDEX IF NOT EXISTS idx_pp_exchange_rate_currencies ON pp_exchange_rate(base_currency, term_currency, date);

        -- Client settings
        CREATE TABLE IF NOT EXISTS pp_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;

    // Run migrations for existing databases
    run_migrations(&conn)?;

    *DB.lock().unwrap() = Some(conn);
    Ok(())
}

/// Run database migrations to add missing columns to existing tables
fn run_migrations(conn: &Connection) -> Result<()> {
    // Helper to check if a column exists in a table
    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let sql = format!("PRAGMA table_info({})", table);
        if let Ok(mut stmt) = conn.prepare(&sql) {
            if let Ok(rows) = stmt.query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            }) {
                for name in rows.flatten() {
                    if name == column {
                        return true;
                    }
                }
            }
        }
        false
    }

    // Migration: Add is_retired column to pp_portfolio if missing
    if !column_exists(conn, "pp_portfolio", "is_retired") {
        conn.execute(
            "ALTER TABLE pp_portfolio ADD COLUMN is_retired INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        log::info!("Migration: Added is_retired column to pp_portfolio");
    }

    // Migration: Add note column to pp_portfolio if missing
    if !column_exists(conn, "pp_portfolio", "note") {
        conn.execute("ALTER TABLE pp_portfolio ADD COLUMN note TEXT", [])?;
        log::info!("Migration: Added note column to pp_portfolio");
    }

    // Migration: Add updated_at column to pp_portfolio if missing
    if !column_exists(conn, "pp_portfolio", "updated_at") {
        conn.execute("ALTER TABLE pp_portfolio ADD COLUMN updated_at TEXT", [])?;
        log::info!("Migration: Added updated_at column to pp_portfolio");
    }

    // Migration: Add is_retired column to pp_account if missing
    if !column_exists(conn, "pp_account", "is_retired") {
        conn.execute(
            "ALTER TABLE pp_account ADD COLUMN is_retired INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        log::info!("Migration: Added is_retired column to pp_account");
    }

    // Migration: Add is_retired column to pp_security if missing
    if !column_exists(conn, "pp_security", "is_retired") {
        conn.execute(
            "ALTER TABLE pp_security ADD COLUMN is_retired INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
        log::info!("Migration: Added is_retired column to pp_security");
    }

    // Migration: Add updated_at column to pp_latest_price if missing
    if !column_exists(conn, "pp_latest_price", "updated_at") {
        conn.execute("ALTER TABLE pp_latest_price ADD COLUMN updated_at TEXT", [])?;
        log::info!("Migration: Added updated_at column to pp_latest_price");
    }

    // Migration: Add custom_logo column to pp_security for user-uploaded logos
    if !column_exists(conn, "pp_security", "custom_logo") {
        conn.execute("ALTER TABLE pp_security ADD COLUMN custom_logo TEXT", [])?;
        log::info!("Migration: Added custom_logo column to pp_security");
    }

    Ok(())
}

pub fn get_connection() -> Result<std::sync::MutexGuard<'static, Option<Connection>>> {
    Ok(DB.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?)
}
