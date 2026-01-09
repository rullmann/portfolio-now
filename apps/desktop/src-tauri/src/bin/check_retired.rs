//! Check is_retired field in portfolios and accounts
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from("/Users/ricoullmann/Documents/PP/Portfolio.portfolio");
    let client = app_lib::protobuf::parse_portfolio_file(&path)?;

    println!("=== Portfolios ({}) ===", client.portfolios.len());
    for p in &client.portfolios {
        println!("  {} - is_retired: {}", p.name, p.is_retired);
    }

    println!("\n=== Accounts ({}) ===", client.accounts.len());
    for a in &client.accounts {
        println!("  {} - is_retired: {}", a.name, a.is_retired);
    }

    Ok(())
}
