//! Verify holdings from SQLite database match DivvyDiary CSV
//!
//! Run with: cargo run --bin verify_db_holdings

use anyhow::Result;
use rusqlite::params;
use std::collections::HashMap;
use std::path::PathBuf;

/// Expected holdings from DivvyDiary CSV (positions with quantity > 0)
/// Format: (ISIN, Name, Expected Quantity)
const EXPECTED_HOLDINGS: &[(&str, &str, f64)] = &[
    ("US02079K3059", "Alphabet Inc Class A (GOOGL)", 87.0),
    ("LU1681038243", "Amundi Nasdaq-100 (ANX)", 9.956856),
    ("US0378331005", "Apple Inc (AAPL)", 205.0),
    ("EU000A2YZK67", "Bitcoin (BTC)", 0.0491102),
    ("US09290D1019", "BlackRock Inc (BLK)", 12.0),
    ("IE00BFZXGZ54", "Invesco EQQQ NASDAQ-100 Acc (EQAC)", 51.762),
    ("IE000S9YS762", "Linde PLC (LIN)", 9.837936),
    ("FR0000121014", "LVMH (MOH)", 20.0),
    ("US5949181045", "Microsoft Corp (MSFT)", 84.0),
    ("CH0038863350", "Nestle SA (NESN)", 8.0),
    ("US67066G1040", "NVIDIA Corp (NVDA)", 42.548563),
    ("IE00BMC38736", "VanEck Semiconductor (SMH)", 361.0),
    ("XC0009655157", "Gold Feinunze (XAU)", 0.35),
    ("DE000A0S9GB0", "Xetra-Gold (4GLD)", 42.66001),
];

const TOLERANCE: f64 = 0.0001;
const SHARES_SCALE: f64 = 100_000_000.0;

fn main() -> Result<()> {
    println!("=== SQLite Holdings Verification Test ===\n");

    let db_path = PathBuf::from("/Users/ricoullmann/Library/Application Support/com.portfolio-performance.desktop/portfolio.db");

    if !db_path.exists() {
        eprintln!("ERROR: Database not found at {}", db_path.display());
        eprintln!("Please import the portfolio first using the app.");
        std::process::exit(1);
    }

    println!("Database: {}", db_path.display());
    println!("Expected positions: {}\n", EXPECTED_HOLDINGS.len());

    let conn = rusqlite::Connection::open(&db_path)?;

    // Query holdings from database
    let sql = r#"
        SELECT
            s.isin,
            s.name,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares
        FROM pp_txn t
        JOIN pp_security s ON t.security_id = s.id
        WHERE t.owner_type = 'portfolio'
          AND t.shares IS NOT NULL
          AND s.isin IS NOT NULL
        GROUP BY s.isin
        HAVING net_shares > 0
        ORDER BY s.name
    "#;

    let mut stmt = conn.prepare(sql)?;
    let mut holdings_by_isin: HashMap<String, (String, f64)> = HashMap::new();

    let rows = stmt.query_map([], |row| {
        let isin: String = row.get(0)?;
        let name: String = row.get(1)?;
        let shares_raw: i64 = row.get(2)?;
        let shares = shares_raw as f64 / SHARES_SCALE;
        Ok((isin, name, shares))
    })?;

    for row in rows {
        let (isin, name, shares) = row?;
        holdings_by_isin.insert(isin, (name, shares));
    }

    println!("Found {} positions in database\n", holdings_by_isin.len());

    // Verify against expected
    println!("=== Verification Results ===\n");

    let mut passed = 0;
    let mut failed = 0;
    let mut missing = 0;

    for (isin, expected_name, expected) in EXPECTED_HOLDINGS {
        if let Some((db_name, actual)) = holdings_by_isin.get(*isin) {
            let diff = (actual - expected).abs();

            if diff < TOLERANCE {
                println!("✓ {} ({})", expected_name, isin);
                println!("  DB Name: {}", db_name);
                println!("  Expected: {:.6}, Actual: {:.6}", expected, actual);
                passed += 1;
            } else {
                println!("✗ {} ({})", expected_name, isin);
                println!("  DB Name: {}", db_name);
                println!("  Expected: {:.6}, Actual: {:.6}, Diff: {:.6}", expected, actual, diff);
                failed += 1;
            }
        } else {
            println!("✗ {} ({}) - MISSING FROM DB", expected_name, isin);
            println!("  Expected: {:.6}", expected);
            missing += 1;
            failed += 1;
        }
        println!();
    }

    // Check for unexpected holdings
    println!("=== Unexpected Holdings in DB ===\n");
    let expected_isins: std::collections::HashSet<&str> =
        EXPECTED_HOLDINGS.iter().map(|(isin, _, _)| *isin).collect();

    let mut unexpected = 0;
    for (isin, (name, shares)) in &holdings_by_isin {
        if !expected_isins.contains(isin.as_str()) {
            println!("? {} ({}) - {:.6} shares", name, isin, shares);
            unexpected += 1;
        }
    }

    if unexpected == 0 {
        println!("None\n");
    }

    // Additional DB stats
    println!("\n=== Database Statistics ===");
    let import_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_import", [], |r| r.get(0))?;
    let security_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_security", [], |r| r.get(0))?;
    let portfolio_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_portfolio", [], |r| r.get(0))?;
    let txn_count: i32 = conn.query_row("SELECT COUNT(*) FROM pp_txn", [], |r| r.get(0))?;
    let portfolio_txn_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'portfolio'",
        [],
        |r| r.get(0),
    )?;

    println!("Imports: {}", import_count);
    println!("Securities: {}", security_count);
    println!("Portfolios: {}", portfolio_count);
    println!("Total Transactions: {}", txn_count);
    println!("Portfolio Transactions: {}", portfolio_txn_count);

    // Summary
    println!("\n=== Summary ===");
    println!("Expected positions: {}", EXPECTED_HOLDINGS.len());
    println!("DB positions:       {}", holdings_by_isin.len());
    println!("Passed:            {}", passed);
    println!("Failed:            {}", failed);
    println!("Missing:           {}", missing);
    println!("Unexpected:        {}", unexpected);

    if failed == 0 && unexpected == 0 {
        println!("\n✓ ALL TESTS PASSED - Database matches expected holdings");
        Ok(())
    } else {
        println!("\n✗ TESTS FAILED - Database does not match expected holdings");
        std::process::exit(1);
    }
}
