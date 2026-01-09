//! Rebuild FIFO lots after database cleanup

use anyhow::Result;
use app_lib::{db, fifo};
use std::path::PathBuf;

fn main() -> Result<()> {
    // macOS: ~/Library/Application Support/com.portfolio-performance.desktop/portfolio.db
    let db_path = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join("Library/Application Support/com.portfolio-performance.desktop/portfolio.db");

    println!("Opening database: {:?}", db_path);
    db::init_database(&db_path)?;

    let conn_guard = db::get_connection()?;
    let conn = conn_guard.as_ref().unwrap();

    println!("Rebuilding all FIFO lots...");
    fifo::build_all_fifo_lots(conn)?;

    println!("Done!");
    Ok(())
}
