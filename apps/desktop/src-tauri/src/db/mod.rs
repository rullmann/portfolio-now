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
            import_id INTEGER,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE SET NULL,
            FOREIGN KEY (cross_entry_id) REFERENCES pp_cross_entry(id) ON DELETE SET NULL,
            FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
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

        -- Corporate Actions (stock splits, mergers, ISIN changes)
        -- Used for automatic detection and tracking of corporate events
        CREATE TABLE IF NOT EXISTS pp_corporate_action (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            security_id INTEGER NOT NULL,
            action_type TEXT NOT NULL CHECK(action_type IN ('STOCK_SPLIT', 'REVERSE_SPLIT', 'ISIN_CHANGE', 'MERGER', 'SPINOFF', 'SYMBOL_CHANGE')),
            effective_date TEXT NOT NULL,
            -- For splits: ratio numerator (e.g., 4 for 4:1 split)
            ratio_from INTEGER,
            -- For splits: ratio denominator (e.g., 1 for 4:1 split)
            ratio_to INTEGER,
            -- For ISIN/symbol changes
            old_identifier TEXT,
            new_identifier TEXT,
            -- For mergers/spinoffs: successor security
            successor_security_id INTEGER,
            -- Data source and confidence
            source TEXT NOT NULL CHECK(source IN ('YAHOO', 'PP_IMPORT', 'DETECTED', 'USER')),
            confidence REAL DEFAULT 1.0,
            -- Tracking
            is_applied INTEGER NOT NULL DEFAULT 0,
            is_confirmed INTEGER NOT NULL DEFAULT 0,
            note TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE,
            FOREIGN KEY (successor_security_id) REFERENCES pp_security(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_pp_corporate_action_security ON pp_corporate_action(security_id);
        CREATE INDEX IF NOT EXISTS idx_pp_corporate_action_date ON pp_corporate_action(effective_date);
        CREATE INDEX IF NOT EXISTS idx_pp_corporate_action_applied ON pp_corporate_action(is_applied);
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

    // Helper to check if a table exists
    fn table_exists(conn: &Connection, table: &str) -> bool {
        conn.query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |_| Ok(()),
        )
        .is_ok()
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

    // Migration: Create pp_corporate_action table if missing
    if !table_exists(conn, "pp_corporate_action") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_corporate_action (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                security_id INTEGER NOT NULL,
                action_type TEXT NOT NULL CHECK(action_type IN ('STOCK_SPLIT', 'REVERSE_SPLIT', 'ISIN_CHANGE', 'MERGER', 'SPINOFF', 'SYMBOL_CHANGE')),
                effective_date TEXT NOT NULL,
                ratio_from INTEGER,
                ratio_to INTEGER,
                old_identifier TEXT,
                new_identifier TEXT,
                successor_security_id INTEGER,
                source TEXT NOT NULL CHECK(source IN ('YAHOO', 'PP_IMPORT', 'DETECTED', 'USER')),
                confidence REAL DEFAULT 1.0,
                is_applied INTEGER NOT NULL DEFAULT 0,
                is_confirmed INTEGER NOT NULL DEFAULT 0,
                note TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE,
                FOREIGN KEY (successor_security_id) REFERENCES pp_security(id) ON DELETE SET NULL
            );
            CREATE INDEX idx_pp_corporate_action_security ON pp_corporate_action(security_id);
            CREATE INDEX idx_pp_corporate_action_date ON pp_corporate_action(effective_date);
            CREATE INDEX idx_pp_corporate_action_applied ON pp_corporate_action(is_applied);
            "#,
        )?;
        log::info!("Migration: Created pp_corporate_action table");
    }

    // Migration: Add attributes column to pp_security (stores JSON)
    if !column_exists(conn, "pp_security", "attributes") {
        conn.execute("ALTER TABLE pp_security ADD COLUMN attributes TEXT", [])?;
        log::info!("Migration: Added attributes column to pp_security");
    }

    // Migration: Add attributes column to pp_account (stores JSON)
    if !column_exists(conn, "pp_account", "attributes") {
        conn.execute("ALTER TABLE pp_account ADD COLUMN attributes TEXT", [])?;
        log::info!("Migration: Added attributes column to pp_account");
    }

    // Migration: Add attributes column to pp_portfolio (stores JSON)
    if !column_exists(conn, "pp_portfolio", "attributes") {
        conn.execute("ALTER TABLE pp_portfolio ADD COLUMN attributes TEXT", [])?;
        log::info!("Migration: Added attributes column to pp_portfolio");
    }

    // Migration: Add other_account_id and other_portfolio_id to pp_txn for transfer tracking
    if !column_exists(conn, "pp_txn", "other_account_id") {
        conn.execute("ALTER TABLE pp_txn ADD COLUMN other_account_id INTEGER", [])?;
        log::info!("Migration: Added other_account_id column to pp_txn");
    }

    if !column_exists(conn, "pp_txn", "other_portfolio_id") {
        conn.execute("ALTER TABLE pp_txn ADD COLUMN other_portfolio_id INTEGER", [])?;
        log::info!("Migration: Added other_portfolio_id column to pp_txn");
    }

    // Migration: Add import_id to pp_txn for tracking which import created the transaction
    if !column_exists(conn, "pp_txn", "import_id") {
        conn.execute("ALTER TABLE pp_txn ADD COLUMN import_id INTEGER", [])?;
        log::info!("Migration: Added import_id column to pp_txn");
    }

    // Migration: Create pp_client_properties table for client-level key-value settings
    if !table_exists(conn, "pp_client_properties") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_client_properties (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                import_id INTEGER,
                key TEXT NOT NULL,
                value TEXT,
                UNIQUE(import_id, key),
                FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_client_properties_import ON pp_client_properties(import_id);
            "#,
        )?;
        log::info!("Migration: Created pp_client_properties table");
    }

    // Migration: Create pp_dashboard table for dashboard configurations
    if !table_exists(conn, "pp_dashboard") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_dashboard (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                import_id INTEGER,
                dashboard_id TEXT,
                name TEXT NOT NULL,
                columns_json TEXT,
                configuration_json TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_dashboard_import ON pp_dashboard(import_id);
            "#,
        )?;
        log::info!("Migration: Created pp_dashboard table");
    }

    // Migration: Create pp_settings table for client settings
    if !table_exists(conn, "pp_settings") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                import_id INTEGER,
                settings_json TEXT,
                UNIQUE(import_id),
                FOREIGN KEY (import_id) REFERENCES pp_import(id) ON DELETE CASCADE
            );
            "#,
        )?;
        log::info!("Migration: Created pp_settings table");
    }

    // Migration: Add PP-specific fields to pp_investment_plan if table exists
    if table_exists(conn, "pp_investment_plan") {
        // Add fees column
        if !column_exists(conn, "pp_investment_plan", "fees") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN fees INTEGER NOT NULL DEFAULT 0", [])?;
            log::info!("Migration: Added fees column to pp_investment_plan");
        }
        // Add taxes column
        if !column_exists(conn, "pp_investment_plan", "taxes") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN taxes INTEGER NOT NULL DEFAULT 0", [])?;
            log::info!("Migration: Added taxes column to pp_investment_plan");
        }
        // Add plan_type column (PURCHASE_OR_DELIVERY=0, DEPOSIT=1, REMOVAL=2, INTEREST=3)
        if !column_exists(conn, "pp_investment_plan", "plan_type") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN plan_type INTEGER NOT NULL DEFAULT 0", [])?;
            log::info!("Migration: Added plan_type column to pp_investment_plan");
        }
        // Add auto_generate column
        if !column_exists(conn, "pp_investment_plan", "auto_generate") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN auto_generate INTEGER NOT NULL DEFAULT 0", [])?;
            log::info!("Migration: Added auto_generate column to pp_investment_plan");
        }
        // Add attributes column (JSON)
        if !column_exists(conn, "pp_investment_plan", "attributes") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN attributes TEXT", [])?;
            log::info!("Migration: Added attributes column to pp_investment_plan");
        }
        // Add note column
        if !column_exists(conn, "pp_investment_plan", "note") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN note TEXT", [])?;
            log::info!("Migration: Added note column to pp_investment_plan");
        }
        // Add uuid column for PP import tracking
        if !column_exists(conn, "pp_investment_plan", "uuid") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN uuid TEXT", [])?;
            log::info!("Migration: Added uuid column to pp_investment_plan");
        }
        // Add transactions column (JSON array of generated transaction UUIDs)
        if !column_exists(conn, "pp_investment_plan", "transactions") {
            conn.execute("ALTER TABLE pp_investment_plan ADD COLUMN transactions TEXT", [])?;
            log::info!("Migration: Added transactions column to pp_investment_plan");
        }
    }

    // Migration: Add target_currency and properties to pp_security
    if table_exists(conn, "pp_security") {
        if !column_exists(conn, "pp_security", "target_currency") {
            conn.execute("ALTER TABLE pp_security ADD COLUMN target_currency TEXT", [])?;
            log::info!("Migration: Added target_currency column to pp_security");
        }
        if !column_exists(conn, "pp_security", "properties") {
            conn.execute("ALTER TABLE pp_security ADD COLUMN properties TEXT", [])?;
            log::info!("Migration: Added properties column to pp_security");
        }
    }

    // Migration: Add other_updated_at to pp_txn
    if table_exists(conn, "pp_txn") {
        if !column_exists(conn, "pp_txn", "other_updated_at") {
            conn.execute("ALTER TABLE pp_txn ADD COLUMN other_updated_at TEXT", [])?;
            log::info!("Migration: Added other_updated_at column to pp_txn");
        }
    }

    // Migration: Create pp_chart_annotation table for AI-generated chart annotations
    if !table_exists(conn, "pp_chart_annotation") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_chart_annotation (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT UNIQUE NOT NULL,
                security_id INTEGER NOT NULL,
                annotation_type TEXT NOT NULL CHECK(annotation_type IN ('support', 'resistance', 'trendline', 'pattern', 'signal', 'target', 'stoploss', 'note')),
                price REAL NOT NULL,
                time TEXT,
                time_end TEXT,
                title TEXT NOT NULL,
                description TEXT,
                confidence REAL NOT NULL DEFAULT 0.8,
                signal TEXT CHECK(signal IN ('bullish', 'bearish', 'neutral')),
                source TEXT NOT NULL DEFAULT 'ai' CHECK(source IN ('ai', 'user')),
                provider TEXT,
                model TEXT,
                is_visible INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_chart_annotation_security ON pp_chart_annotation(security_id);
            CREATE INDEX idx_pp_chart_annotation_visible ON pp_chart_annotation(security_id, is_visible);
            "#,
        )?;
        log::info!("Migration: Created pp_chart_annotation table");
    }

    // Migration: Create pp_price_alert table for price alerts
    if !table_exists(conn, "pp_price_alert") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_price_alert (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT UNIQUE NOT NULL,
                security_id INTEGER NOT NULL,
                alert_type TEXT NOT NULL CHECK(alert_type IN (
                    'price_above', 'price_below', 'price_crosses',
                    'rsi_above', 'rsi_below',
                    'volume_spike', 'divergence',
                    'pattern_detected', 'support_break', 'resistance_break'
                )),
                target_value REAL NOT NULL,
                target_value_2 REAL,
                is_active INTEGER NOT NULL DEFAULT 1,
                is_triggered INTEGER NOT NULL DEFAULT 0,
                trigger_count INTEGER NOT NULL DEFAULT 0,
                last_triggered_at TEXT,
                last_triggered_price REAL,
                note TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_price_alert_security ON pp_price_alert(security_id);
            CREATE INDEX idx_pp_price_alert_active ON pp_price_alert(is_active);
            CREATE INDEX idx_pp_price_alert_security_active ON pp_price_alert(security_id, is_active);
            "#,
        )?;
        log::info!("Migration: Created pp_price_alert table");
    }

    // Migration: Create pp_pattern_history table for tracking pattern success rates
    if !table_exists(conn, "pp_pattern_history") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_pattern_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                security_id INTEGER NOT NULL,
                pattern_type TEXT NOT NULL,
                detected_at TEXT NOT NULL,
                price_at_detection REAL NOT NULL,
                predicted_direction TEXT NOT NULL CHECK(predicted_direction IN ('bullish', 'bearish', 'neutral')),
                actual_outcome TEXT CHECK(actual_outcome IN ('success', 'failure', 'pending')),
                price_after_5d REAL,
                price_after_10d REAL,
                price_change_5d_percent REAL,
                price_change_10d_percent REAL,
                evaluated_at TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_pattern_history_security ON pp_pattern_history(security_id);
            CREATE INDEX idx_pp_pattern_history_pattern ON pp_pattern_history(pattern_type);
            CREATE INDEX idx_pp_pattern_history_outcome ON pp_pattern_history(actual_outcome);
            "#,
        )?;
        log::info!("Migration: Created pp_pattern_history table");
    }

    // Migration: Create pp_chart_drawing table for user-drawn chart elements
    if !table_exists(conn, "pp_chart_drawing") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_chart_drawing (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT UNIQUE NOT NULL,
                security_id INTEGER NOT NULL,
                drawing_type TEXT NOT NULL CHECK(drawing_type IN ('trendline', 'horizontal', 'fibonacci', 'rectangle', 'text')),
                points_json TEXT NOT NULL,
                color TEXT NOT NULL DEFAULT '#2563eb',
                line_width INTEGER NOT NULL DEFAULT 2,
                fib_levels_json TEXT,
                is_visible INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_chart_drawing_security ON pp_chart_drawing(security_id);
            CREATE INDEX idx_pp_chart_drawing_visible ON pp_chart_drawing(security_id, is_visible);
            "#,
        )?;
        log::info!("Migration: Created pp_chart_drawing table");
    }

    // Migration: Create pp_tax_settings table for German tax settings
    if !table_exists(conn, "pp_tax_settings") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_tax_settings (
                year INTEGER PRIMARY KEY,
                is_married INTEGER NOT NULL DEFAULT 0,
                kirchensteuer_rate REAL,
                bundesland TEXT,
                freistellung_used REAL NOT NULL DEFAULT 0
            );
            "#,
        )?;
        log::info!("Migration: Created pp_tax_settings table");
    }

    // Migration: Create pp_allocation_target table for portfolio rebalancing alerts
    if !table_exists(conn, "pp_allocation_target") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_allocation_target (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                portfolio_id INTEGER NOT NULL,
                security_id INTEGER,
                taxonomy_id INTEGER,
                classification_id INTEGER,
                target_weight REAL NOT NULL,
                threshold REAL NOT NULL DEFAULT 0.05,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT,
                FOREIGN KEY (portfolio_id) REFERENCES pp_portfolio(id) ON DELETE CASCADE,
                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE,
                FOREIGN KEY (taxonomy_id) REFERENCES pp_taxonomy(id) ON DELETE CASCADE,
                FOREIGN KEY (classification_id) REFERENCES pp_classification(id) ON DELETE CASCADE,
                UNIQUE(portfolio_id, security_id),
                UNIQUE(portfolio_id, taxonomy_id, classification_id)
            );
            CREATE INDEX idx_pp_allocation_target_portfolio ON pp_allocation_target(portfolio_id);
            CREATE INDEX idx_pp_allocation_target_security ON pp_allocation_target(security_id);
            "#,
        )?;
        log::info!("Migration: Created pp_allocation_target table");
    }

    // Migration: Create pp_attribute_type table for custom attribute definitions
    if !table_exists(conn, "pp_attribute_type") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_attribute_type (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                column_label TEXT,
                target TEXT NOT NULL DEFAULT 'security' CHECK(target IN ('security', 'account', 'portfolio')),
                data_type TEXT NOT NULL DEFAULT 'STRING' CHECK(data_type IN (
                    'STRING', 'LONG_NUMBER', 'DOUBLE_NUMBER', 'DATE', 'BOOLEAN', 'LIMIT_PRICE', 'SHARE'
                )),
                converter_class TEXT,
                source TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT
            );
            CREATE INDEX idx_pp_attribute_type_target ON pp_attribute_type(target);
            "#,
        )?;
        log::info!("Migration: Created pp_attribute_type table");
    }

    // Migration: Create pp_ex_dividend table for ex-dividend dates
    if !table_exists(conn, "pp_ex_dividend") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_ex_dividend (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                security_id INTEGER NOT NULL,
                ex_date TEXT NOT NULL,
                record_date TEXT,
                pay_date TEXT,
                amount REAL,
                currency TEXT,
                frequency TEXT,
                source TEXT,
                is_confirmed INTEGER NOT NULL DEFAULT 0,
                note TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT,
                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE,
                UNIQUE(security_id, ex_date)
            );
            CREATE INDEX idx_pp_ex_dividend_ex_date ON pp_ex_dividend(ex_date);
            CREATE INDEX idx_pp_ex_dividend_security ON pp_ex_dividend(security_id);
            "#,
        )?;
        log::info!("Migration: Created pp_ex_dividend table");
    }

    // Migration: Create pp_consortium table for portfolio groups
    if !table_exists(conn, "pp_consortium") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_consortium (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                portfolio_ids TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )?;
        log::info!("Migration: Created pp_consortium table");
    }

    // Migration: Create pp_symbol_mapping table for symbol validation cache
    if !table_exists(conn, "pp_symbol_mapping") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_symbol_mapping (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                security_id INTEGER NOT NULL UNIQUE,

                -- Validated configuration
                validated_feed TEXT NOT NULL,
                validated_feed_url TEXT,
                validated_ticker TEXT,
                validated_exchange TEXT,

                -- Provider search results (JSON)
                provider_results TEXT,

                -- Status
                validation_status TEXT NOT NULL DEFAULT 'pending',
                confidence REAL NOT NULL DEFAULT 0.0,
                validation_method TEXT,

                -- AI suggestion (JSON)
                ai_suggestion_json TEXT,

                -- Timestamps
                last_validated_at TEXT,
                price_check_success INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (security_id) REFERENCES pp_security(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_symbol_mapping_security ON pp_symbol_mapping(security_id);
            CREATE INDEX idx_pp_symbol_mapping_status ON pp_symbol_mapping(validation_status);
            "#,
        )?;
        log::info!("Migration: Created pp_symbol_mapping table");
    }

    // Migration: Create pp_validation_run table for tracking validation runs
    if !table_exists(conn, "pp_validation_run") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_validation_run (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                total_securities INTEGER DEFAULT 0,
                validated_count INTEGER DEFAULT 0,
                failed_count INTEGER DEFAULT 0,
                ai_suggested_count INTEGER DEFAULT 0,
                status TEXT DEFAULT 'running',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )?;
        log::info!("Migration: Created pp_validation_run table");
    }

    // Migration: Create pp_chat_history table for chat message persistence
    if !table_exists(conn, "pp_chat_history") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_chat_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX idx_pp_chat_history_created ON pp_chat_history(created_at);
            "#,
        )?;
        log::info!("Migration: Created pp_chat_history table");
    }

    // Migration: Create pp_chat_suggestion table for suggestion persistence with status
    if !table_exists(conn, "pp_chat_suggestion") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_chat_suggestion (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                description TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'confirmed', 'declined')),
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (message_id) REFERENCES pp_chat_history(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_pp_chat_suggestion_message ON pp_chat_suggestion(message_id);
            "#,
        )?;
        log::info!("Migration: Created pp_chat_suggestion table");
    }

    // Migration: Create pp_chat_conversation table for multiple chat sessions
    if !table_exists(conn, "pp_chat_conversation") {
        conn.execute_batch(
            r#"
            CREATE TABLE pp_chat_conversation (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX idx_pp_chat_conversation_updated ON pp_chat_conversation(updated_at DESC);
            "#,
        )?;
        log::info!("Migration: Created pp_chat_conversation table");
    }

    // Migration: Add conversation_id to pp_chat_history
    if table_exists(conn, "pp_chat_history") && !column_exists(conn, "pp_chat_history", "conversation_id") {
        conn.execute(
            "ALTER TABLE pp_chat_history ADD COLUMN conversation_id INTEGER REFERENCES pp_chat_conversation(id) ON DELETE CASCADE",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pp_chat_history_conversation ON pp_chat_history(conversation_id)",
            [],
        )?;
        log::info!("Migration: Added conversation_id column to pp_chat_history");

        // Migrate existing messages to a default conversation if any exist
        let has_messages: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pp_chat_history WHERE conversation_id IS NULL)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if has_messages {
            // Create default conversation for existing messages
            conn.execute(
                r#"
                INSERT INTO pp_chat_conversation (title, created_at, updated_at)
                SELECT 'Chat-Verlauf',
                       COALESCE((SELECT MIN(created_at) FROM pp_chat_history), datetime('now')),
                       COALESCE((SELECT MAX(created_at) FROM pp_chat_history), datetime('now'))
                "#,
                [],
            )?;

            // Update existing messages to use the new conversation
            conn.execute(
                "UPDATE pp_chat_history SET conversation_id = (SELECT id FROM pp_chat_conversation ORDER BY id LIMIT 1) WHERE conversation_id IS NULL",
                [],
            )?;
            log::info!("Migration: Migrated existing chat messages to default conversation");
        }
    }

    // Migration: Add conversation_id to pp_chat_suggestion (for cascade delete)
    if table_exists(conn, "pp_chat_suggestion") && !column_exists(conn, "pp_chat_suggestion", "conversation_id") {
        conn.execute(
            "ALTER TABLE pp_chat_suggestion ADD COLUMN conversation_id INTEGER REFERENCES pp_chat_conversation(id) ON DELETE CASCADE",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pp_chat_suggestion_conversation ON pp_chat_suggestion(conversation_id)",
            [],
        )?;

        // Update existing suggestions with conversation_id from their parent messages
        conn.execute(
            r#"
            UPDATE pp_chat_suggestion
            SET conversation_id = (
                SELECT conversation_id FROM pp_chat_history WHERE pp_chat_history.id = pp_chat_suggestion.message_id
            )
            WHERE conversation_id IS NULL
            "#,
            [],
        )?;
        log::info!("Migration: Added conversation_id column to pp_chat_suggestion");
    }

    // Migration: Add attachments_json column to pp_chat_history for image storage
    if table_exists(conn, "pp_chat_history") && !column_exists(conn, "pp_chat_history", "attachments_json") {
        conn.execute(
            "ALTER TABLE pp_chat_history ADD COLUMN attachments_json TEXT",
            [],
        )?;
        log::info!("Migration: Added attachments_json column to pp_chat_history for image storage");
    }

    // Migration: Create ai_user_template table for user-defined SQL query templates
    if !table_exists(conn, "ai_user_template") {
        conn.execute_batch(
            r#"
            CREATE TABLE ai_user_template (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id TEXT UNIQUE NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                sql_query TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX idx_ai_user_template_enabled ON ai_user_template(enabled);
            "#,
        )?;
        log::info!("Migration: Created ai_user_template table");
    }

    // Migration: Create ai_user_template_param table for template parameters
    if !table_exists(conn, "ai_user_template_param") {
        conn.execute_batch(
            r#"
            CREATE TABLE ai_user_template_param (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id INTEGER NOT NULL,
                param_name TEXT NOT NULL,
                param_type TEXT NOT NULL,
                required INTEGER NOT NULL DEFAULT 0,
                description TEXT NOT NULL,
                default_value TEXT,
                FOREIGN KEY (template_id) REFERENCES ai_user_template(id) ON DELETE CASCADE,
                UNIQUE(template_id, param_name)
            );
            CREATE INDEX idx_ai_user_template_param_template ON ai_user_template_param(template_id);
            "#,
        )?;
        log::info!("Migration: Created ai_user_template_param table");
    }

    Ok(())
}

pub fn get_connection() -> Result<std::sync::MutexGuard<'static, Option<Connection>>> {
    Ok(DB.lock().map_err(|e| anyhow::anyhow!("Failed to lock database: {}", e))?)
}
