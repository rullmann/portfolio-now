//! Verify holdings from Portfolio.portfolio match DivvyDiary CSV
//!
//! Run with: cargo run --bin verify_holdings

use anyhow::Result;
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

const TOLERANCE: f64 = 0.0001; // Allow small floating point differences

fn main() -> Result<()> {
    println!("=== Holdings Verification Test ===\n");

    let portfolio_path = PathBuf::from("/Users/ricoullmann/Documents/PP/Portfolio.portfolio");

    if !portfolio_path.exists() {
        eprintln!("ERROR: Portfolio file not found at {}", portfolio_path.display());
        std::process::exit(1);
    }

    println!("Source: {}", portfolio_path.display());
    println!("Expected positions: {}\n", EXPECTED_HOLDINGS.len());

    // Parse portfolio file
    println!("Parsing portfolio file...");
    let client = app_lib::protobuf::parse_portfolio_file(&portfolio_path)?;

    println!("Parsed {} securities, {} portfolios\n",
             client.securities.len(), client.portfolios.len());

    // Build ISIN to security map
    let isin_to_security: HashMap<String, &app_lib::pp::Security> = client.securities
        .iter()
        .filter_map(|s| s.isin.as_ref().map(|isin| (isin.clone(), s)))
        .collect();

    // Calculate holdings per security UUID across all portfolios
    let mut holdings_by_uuid: HashMap<String, f64> = HashMap::new();

    const SHARES_SCALE: f64 = 100_000_000.0;

    // Transaction types that add shares
    let buy_types = ["BUY", "DELIVERY_INBOUND", "TRANSFER_IN"];
    // Transaction types that remove shares
    let sell_types = ["SELL", "DELIVERY_OUTBOUND", "TRANSFER_OUT"];

    for portfolio in &client.portfolios {
        for tx in &portfolio.transactions {
            if let Some(ref sec_uuid) = tx.security_uuid {
                let shares = tx.shares as f64 / SHARES_SCALE;
                let tx_type = tx.transaction_type.as_str();

                let entry = holdings_by_uuid.entry(sec_uuid.clone()).or_insert(0.0);

                if buy_types.contains(&tx_type) {
                    *entry += shares;
                } else if sell_types.contains(&tx_type) {
                    *entry -= shares;
                }
            }
        }
    }

    // Filter to positive holdings
    holdings_by_uuid.retain(|_, shares| *shares > TOLERANCE);

    // Build UUID to ISIN map
    let uuid_to_isin: HashMap<String, String> = client.securities
        .iter()
        .filter_map(|s| s.isin.as_ref().map(|isin| (s.uuid.clone(), isin.clone())))
        .collect();

    // Convert to holdings by ISIN
    let mut holdings_by_isin: HashMap<String, f64> = HashMap::new();
    for (uuid, shares) in &holdings_by_uuid {
        if let Some(isin) = uuid_to_isin.get(uuid) {
            *holdings_by_isin.entry(isin.clone()).or_insert(0.0) += *shares;
        }
    }

    // Verify against expected
    println!("=== Verification Results ===\n");

    let mut passed = 0;
    let mut failed = 0;
    let mut missing = 0;

    for (isin, name, expected) in EXPECTED_HOLDINGS {
        let actual = holdings_by_isin.get(*isin).copied().unwrap_or(0.0);
        let diff = (actual - expected).abs();

        if diff < TOLERANCE {
            println!("✓ {} ({})", name, isin);
            println!("  Expected: {:.6}, Actual: {:.6}", expected, actual);
            passed += 1;
        } else if actual < TOLERANCE {
            println!("✗ {} ({}) - MISSING", name, isin);
            println!("  Expected: {:.6}, Actual: 0 (not found)", expected);
            missing += 1;
            failed += 1;
        } else {
            println!("✗ {} ({})", name, isin);
            println!("  Expected: {:.6}, Actual: {:.6}, Diff: {:.6}", expected, actual, diff);
            failed += 1;
        }
        println!();
    }

    // Check for unexpected holdings
    println!("=== Unexpected Holdings ===\n");
    let expected_isins: std::collections::HashSet<&str> =
        EXPECTED_HOLDINGS.iter().map(|(isin, _, _)| *isin).collect();

    let mut unexpected = 0;
    for (isin, shares) in &holdings_by_isin {
        if !expected_isins.contains(isin.as_str()) {
            // Find security name
            let name = isin_to_security.get(isin)
                .map(|s| s.name.as_str())
                .unwrap_or("Unknown");
            println!("? {} ({}) - {:.6} shares", name, isin, shares);
            unexpected += 1;
        }
    }

    if unexpected == 0 {
        println!("None\n");
    }

    // Summary
    println!("\n=== Summary ===");
    println!("Expected positions: {}", EXPECTED_HOLDINGS.len());
    println!("Actual positions:   {}", holdings_by_isin.len());
    println!("Passed:            {}", passed);
    println!("Failed:            {}", failed);
    println!("Missing:           {}", missing);
    println!("Unexpected:        {}", unexpected);

    if failed == 0 && unexpected == 0 {
        println!("\n✓ ALL TESTS PASSED");
        Ok(())
    } else {
        println!("\n✗ TESTS FAILED");
        std::process::exit(1);
    }
}
