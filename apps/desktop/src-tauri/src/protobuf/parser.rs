//! Parser for Portfolio Performance protobuf binary files.

use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use prost::Message;
use zip::ZipArchive;

use super::schema::{self, PClient};
use crate::pp::{
    common::{ForexInfo, Money},
    taxonomy::{Classification, ClassificationAssignment},
    transaction::{
        AccountTransaction, AccountTransactionType, CrossEntry, CrossEntryType,
        PortfolioTransaction, PortfolioTransactionType, TransactionUnit, UnitType,
    },
    Account, Client, Dashboard, DashboardColumn, DashboardWidget, LatestPrice, Portfolio,
    PriceEntry, Security, Taxonomy,
};

/// Parse a .portfolio file (ZIP archive) into Client
pub fn parse_portfolio_file(path: &Path) -> Result<Client> {
    let file = std::fs::File::open(path).context("Failed to open file")?;
    let mut archive = ZipArchive::new(file).context("Failed to read ZIP archive")?;

    let mut data_file = archive
        .by_name("data.portfolio")
        .context("data.portfolio not found in archive")?;

    let mut data = Vec::new();
    data_file
        .read_to_end(&mut data)
        .context("Failed to read data.portfolio")?;

    parse_to_client(&data)
}

/// Parse raw protobuf data into Client
pub fn parse_to_client(data: &[u8]) -> Result<Client> {
    // Verify header
    if data.len() < super::HEADER_LEN {
        bail!("Data too short for PP protobuf format");
    }

    if &data[0..6] != super::HEADER {
        bail!(
            "Invalid header, expected PPPBV1, got {:?}",
            String::from_utf8_lossy(&data[0..6])
        );
    }

    // Skip header and parse protobuf
    let proto_data = &data[super::HEADER_LEN..];
    let pb_client =
        PClient::decode(proto_data).context("Failed to decode protobuf Client message")?;

    // Convert to pp::Client
    convert_client(&pb_client)
}

/// Convert protobuf Client to pp::Client
fn convert_client(pb: &PClient) -> Result<Client> {
    let mut client = Client::new(&pb.base_currency);
    client.version = pb.version;

    // First pass: convert all securities and build UUID lookup
    let mut security_uuids: Vec<String> = Vec::new();
    for pb_sec in &pb.securities {
        let security = convert_security(pb_sec)?;
        security_uuids.push(security.uuid.clone());
        client.securities.push(security);
    }

    // Convert accounts
    let mut account_uuids: Vec<String> = Vec::new();
    for pb_acc in &pb.accounts {
        let account = convert_account(pb_acc)?;
        account_uuids.push(account.uuid.clone());
        client.accounts.push(account);
    }

    // Convert portfolios
    for pb_port in &pb.portfolios {
        let portfolio = convert_portfolio(pb_port)?;
        client.portfolios.push(portfolio);
    }

    // Process transactions from tag 5 and add to portfolios/accounts
    for pb_tx in &pb.transactions {
        // Handle portfolio transactions
        if let Some(portfolio_uuid) = &pb_tx.portfolio {
            // For SECURITY_TRANSFER: pb_tx.portfolio is SOURCE, pb_tx.other_portfolio is DESTINATION
            // For other types: pb_tx.portfolio is the owning portfolio
            if pb_tx.transaction_type == super::schema::transaction_type::SECURITY_TRANSFER {
                // TRANSFER: portfolio = source (TRANSFER_OUT), other_portfolio = destination (TRANSFER_IN)
                // UUIDs: TRANSFER_OUT = "{uuid}-out", TRANSFER_IN = "{uuid}"
                let transfer_out_uuid = format!("{}-out", pb_tx.uuid);
                let transfer_in_uuid = pb_tx.uuid.clone();

                // Add TRANSFER_OUT to source portfolio (pb_tx.portfolio)
                if let Some(source_portfolio) = client
                    .portfolios
                    .iter_mut()
                    .find(|p| &p.uuid == portfolio_uuid)
                {
                    if let Some(mut tx) = convert_transaction(pb_tx, true)? {
                        // Set cross-entry to link TRANSFER_OUT -> TRANSFER_IN
                        tx.cross_entry = Some(CrossEntry::portfolio_transfer(
                            transfer_out_uuid.clone(),
                            transfer_in_uuid.clone(),
                        ));
                        source_portfolio.transactions.push(tx);
                    }
                }

                // Add TRANSFER_IN to destination portfolio (pb_tx.other_portfolio)
                if let Some(other_portfolio_uuid) = &pb_tx.other_portfolio {
                    if let Some(dest_portfolio) = client
                        .portfolios
                        .iter_mut()
                        .find(|p| &p.uuid == other_portfolio_uuid)
                    {
                        if let Some(mut tx) = convert_transaction(pb_tx, false)? {
                            // Set cross-entry to link TRANSFER_OUT -> TRANSFER_IN
                            tx.cross_entry = Some(CrossEntry::portfolio_transfer(
                                transfer_out_uuid.clone(),
                                transfer_in_uuid.clone(),
                            ));
                            dest_portfolio.transactions.push(tx);
                        }
                    }
                }
            } else {
                // Non-transfer: add transaction to owning portfolio
                if let Some(portfolio) = client
                    .portfolios
                    .iter_mut()
                    .find(|p| &p.uuid == portfolio_uuid)
                {
                    if let Some(tx) = convert_transaction(pb_tx, false)? {
                        portfolio.transactions.push(tx);
                    }
                }
            }
        }

        // Handle account transactions
        if let Some(account_uuid) = &pb_tx.account {
            if let Some(account) = client
                .accounts
                .iter_mut()
                .find(|a| &a.uuid == account_uuid)
            {
                if let Some(tx) = convert_account_transaction(pb_tx)? {
                    account.transactions.push(tx);
                }
            }

            // For cash transfers (CASH_TRANSFER=5), also create transfer for target account
            if pb_tx.transaction_type == super::schema::transaction_type::CASH_TRANSFER {
                if let Some(other_account_uuid) = &pb_tx.other_account {
                    // UUIDs: TRANSFER_OUT = "{uuid}", TRANSFER_IN = "{uuid}-in"
                    let transfer_out_uuid = pb_tx.uuid.clone();
                    let transfer_in_uuid = format!("{}-in", pb_tx.uuid);

                    // Update the source transaction with cross-entry
                    if let Some(source_account) = client
                        .accounts
                        .iter_mut()
                        .find(|a| &a.uuid == account_uuid)
                    {
                        if let Some(source_tx) = source_account.transactions.last_mut() {
                            source_tx.cross_entry = Some(CrossEntry::account_transfer(
                                transfer_out_uuid.clone(),
                                transfer_in_uuid.clone(),
                            ));
                        }
                    }

                    if let Some(target_account) = client
                        .accounts
                        .iter_mut()
                        .find(|a| &a.uuid == other_account_uuid)
                    {
                        // Create transfer-in transaction for target account
                        let mut target_tx = pb_tx.clone();
                        // Swap the direction for the target account
                        target_tx.account = Some(other_account_uuid.clone());
                        target_tx.other_account = Some(account_uuid.clone());
                        target_tx.uuid = transfer_in_uuid.clone();

                        if let Some(mut tx) = convert_account_transaction(&target_tx)? {
                            // Set cross-entry to link TRANSFER_OUT -> TRANSFER_IN
                            tx.cross_entry = Some(CrossEntry::account_transfer(
                                transfer_out_uuid.clone(),
                                transfer_in_uuid.clone(),
                            ));
                            target_account.transactions.push(tx);
                        }
                    }
                }
            }
        }
    }

    // Convert taxonomies
    for pb_tax in &pb.taxonomies {
        let taxonomy = convert_taxonomy(pb_tax)?;
        client.taxonomies.push(taxonomy);
    }

    // Note: Watchlists and Investment plans are not present in current file format

    // Convert dashboards
    for pb_dash in &pb.dashboards {
        let dashboard = convert_dashboard(pb_dash)?;
        client.dashboards.push(dashboard);
    }

    Ok(client)
}

/// Convert protobuf Security to pp::Security
fn convert_security(pb: &schema::PSecurity) -> Result<Security> {
    // currency_code is now optional, default to empty string
    let currency = pb.currency_code.clone().unwrap_or_default();
    let mut sec = Security::new(pb.uuid.clone(), pb.name.clone(), currency);

    sec.online_id = pb.online_id.clone();
    sec.isin = pb.isin.clone();
    sec.wkn = pb.wkn.clone();
    sec.ticker = pb.ticker_symbol.clone();
    sec.calendar = pb.calendar.clone();
    sec.feed = pb.feed.clone();
    sec.feed_url = pb.feed_url.clone();

    // Convert prices
    for pb_price in &pb.prices {
        if let Some(date) = days_to_date(pb_price.date) {
            sec.prices.push(PriceEntry::new(date, pb_price.value));
        }
    }

    // Set latest from protobuf latest field, or from last price if not available
    if let Some(pb_latest) = &pb.latest {
        if let Some(date) = days_to_date(pb_latest.date) {
            sec.latest = Some(LatestPrice {
                date: Some(date),
                value: Some(pb_latest.close),
                high: pb_latest.high,
                low: pb_latest.low,
                volume: pb_latest.volume,
            });
        }
    } else if let Some(last_price) = sec.prices.last() {
        sec.latest = Some(LatestPrice {
            date: Some(last_price.date),
            value: Some(last_price.value),
            high: None,
            low: None,
            volume: None,
        });
    }

    Ok(sec)
}

/// Convert days since epoch to NaiveDate
fn days_to_date(days: i64) -> Option<NaiveDate> {
    if days == 0 {
        return None;
    }
    // PP uses days since Unix epoch (1970-01-01)
    NaiveDate::from_num_days_from_ce_opt((days + 719163) as i32)
}

/// Convert days to datetime (midnight)
#[allow(dead_code)]
fn days_to_datetime(days: i64) -> Option<NaiveDateTime> {
    days_to_date(days).map(|d| d.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()))
}

/// Convert protobuf Account to pp::Account
fn convert_account(pb: &schema::PAccount) -> Result<Account> {
    let acc = Account::new(pb.uuid.clone(), pb.name.clone(), pb.currency_code.clone());
    // Note: Transactions are simplified in this file format
    // They only contain timestamp and amount, not full transaction details
    Ok(acc)
}

/// Convert protobuf Portfolio to pp::Portfolio
fn convert_portfolio(pb: &schema::PPortfolio) -> Result<Portfolio> {
    let mut port = Portfolio::new(pb.uuid.clone(), pb.name.clone());

    // Copy is_retired flag
    port.is_retired = pb.is_retired;

    // Copy note if present
    port.note = pb.note.clone();

    // Reference account is now an optional UUID string
    if let Some(ref_acc) = &pb.reference_account {
        if !ref_acc.is_empty() {
            port.reference_account_uuid = Some(ref_acc.clone());
        }
    }

    // Convert updated_at timestamp
    if let Some(ts) = &pb.updated_at {
        if let Some(dt) = chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32) {
            port.updated_at = Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
        }
    }

    // Transactions will be added from PClient.transactions
    Ok(port)
}

/// Convert protobuf Transaction to pp::PortfolioTransaction
/// If `is_outbound` is true, create a TRANSFER_OUT transaction (for source portfolio in transfers)
fn convert_transaction(
    pb: &schema::PTransaction,
    is_outbound: bool,
) -> Result<Option<PortfolioTransaction>> {
    // Get timestamp from date field (PTimestamp with seconds/nanos)
    let timestamp = pb.date.as_ref().map(|ts| ts.seconds).unwrap_or(0);

    if timestamp == 0 {
        return Ok(None);
    }

    // Convert Unix timestamp to NaiveDateTime
    let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.naive_utc())
        .unwrap_or_default();

    // Map transaction type using OFFICIAL PP enum values
    // PURCHASE=0, SALE=1, INBOUND_DELIVERY=2, OUTBOUND_DELIVERY=3, SECURITY_TRANSFER=4
    let tx_type = if is_outbound {
        // For the source portfolio in a transfer, create TRANSFER_OUT (not DELIVERY_OUTBOUND!)
        PortfolioTransactionType::TransferOut
    } else {
        use super::schema::transaction_type::*;
        match pb.transaction_type {
            PURCHASE => PortfolioTransactionType::Buy,
            SALE => PortfolioTransactionType::Sell,
            INBOUND_DELIVERY => PortfolioTransactionType::DeliveryInbound,
            OUTBOUND_DELIVERY => PortfolioTransactionType::DeliveryOutbound,
            SECURITY_TRANSFER => PortfolioTransactionType::TransferIn, // Transfer treated as inbound
            _ => PortfolioTransactionType::Buy, // Default for non-portfolio types
        }
    };

    // Generate a unique UUID for outbound transactions
    let uuid = if is_outbound {
        format!("{}-out", pb.uuid)
    } else {
        pb.uuid.clone()
    };

    let mut tx = PortfolioTransaction::new(
        uuid.clone(),
        datetime,
        tx_type,
        Money::new(pb.amount, pb.currency_code.clone()),
        pb.shares.unwrap_or(0),
    );

    tx.security_uuid = pb.security.clone();
    tx.source = pb.source.clone();
    tx.note = pb.note.clone();

    // Convert updated_at timestamp
    if let Some(ref updated) = pb.updated_at {
        tx.updated_at = Some(format_timestamp(updated));
    }

    // Convert transaction units (fees, taxes, gross value)
    for pb_unit in &pb.units {
        if let Some(unit) = convert_unit(pb_unit) {
            tx.units.push(unit);
        }
    }

    // Map cross-entry if present
    if let Some(ref other_uuid) = pb.other_uuid {
        if !other_uuid.is_empty() {
            let entry_type = determine_cross_entry_type(pb.transaction_type);
            tx.cross_entry = Some(CrossEntry {
                entry_type,
                source_uuid: uuid,
                target_uuid: other_uuid.clone(),
            });
        }
    }

    Ok(Some(tx))
}

/// Convert protobuf Transaction to pp::AccountTransaction
fn convert_account_transaction(pb: &schema::PTransaction) -> Result<Option<AccountTransaction>> {
    // Get timestamp from date field (PTimestamp with seconds/nanos)
    let timestamp = pb.date.as_ref().map(|ts| ts.seconds).unwrap_or(0);

    if timestamp == 0 {
        return Ok(None);
    }

    // Convert Unix timestamp to NaiveDateTime
    let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.naive_utc())
        .unwrap_or_default();

    // Map transaction type using OFFICIAL PP enum values
    use super::schema::transaction_type::*;
    let tx_type = match pb.transaction_type {
        DEPOSIT => AccountTransactionType::Deposit,
        REMOVAL => AccountTransactionType::Removal,
        DIVIDEND => AccountTransactionType::Dividends,
        INTEREST => AccountTransactionType::Interest,
        INTEREST_CHARGE => AccountTransactionType::InterestCharge,
        TAX => AccountTransactionType::Taxes,
        TAX_REFUND => AccountTransactionType::TaxRefund,
        FEE => AccountTransactionType::Fees,
        FEE_REFUND => AccountTransactionType::FeesRefund,
        CASH_TRANSFER => {
            // Determine direction based on other_account presence
            if pb.other_account.is_some() {
                AccountTransactionType::TransferOut
            } else {
                AccountTransactionType::TransferIn
            }
        }
        PURCHASE => AccountTransactionType::Buy,
        SALE => AccountTransactionType::Sell,
        _ => return Ok(None), // Not an account transaction type
    };

    let mut tx = AccountTransaction::new(
        pb.uuid.clone(),
        datetime,
        tx_type,
        Money::new(pb.amount, pb.currency_code.clone()),
    );

    tx.security_uuid = pb.security.clone();
    tx.shares = pb.shares;
    tx.source = pb.source.clone();
    tx.note = pb.note.clone();

    // Convert updated_at timestamp
    if let Some(ref updated) = pb.updated_at {
        tx.updated_at = Some(format_timestamp(updated));
    }

    // Convert transaction units (fees, taxes, gross value)
    for pb_unit in &pb.units {
        if let Some(unit) = convert_unit(pb_unit) {
            tx.units.push(unit);
        }
    }

    // Map cross-entry if present
    if let Some(ref other_uuid) = pb.other_uuid {
        if !other_uuid.is_empty() {
            let entry_type = determine_cross_entry_type(pb.transaction_type);
            tx.cross_entry = Some(CrossEntry {
                entry_type,
                source_uuid: pb.uuid.clone(),
                target_uuid: other_uuid.clone(),
            });
        }
    }

    Ok(Some(tx))
}

/// Convert a protobuf TransactionUnit to pp::TransactionUnit
fn convert_unit(pb: &schema::PTransactionUnit) -> Option<TransactionUnit> {
    use super::schema::unit_type::*;

    let unit_type = match pb.unit_type {
        GROSS_VALUE => UnitType::GrossValue,
        TAX => UnitType::Tax,
        FEE => UnitType::Fee,
        _ => return None,
    };

    let mut unit = TransactionUnit::new(
        unit_type,
        Money::new(pb.amount, pb.currency_code.clone()),
    );

    // Add forex information if present
    if let (Some(fx_amount), Some(ref fx_currency)) = (pb.fx_amount, &pb.fx_currency_code) {
        let exchange_rate = pb
            .fx_rate_to_base
            .as_ref()
            .map(|rate| decode_decimal_value(rate))
            .unwrap_or(1.0);

        unit.forex = Some(ForexInfo::new(
            Money::new(fx_amount, fx_currency.clone()),
            exchange_rate,
        ));
    }

    Some(unit)
}

/// Decode PDecimalValue to f64
fn decode_decimal_value(dec: &schema::PDecimalValue) -> f64 {
    if dec.value.is_empty() {
        return 1.0;
    }

    // BigDecimal stored as: scale, precision, and big-endian byte array
    // The value represents: integer_value / 10^scale
    let mut int_value: i128 = 0;
    for &byte in &dec.value {
        int_value = (int_value << 8) | (byte as i128);
    }

    // Check if negative (first bit set)
    if !dec.value.is_empty() && (dec.value[0] & 0x80) != 0 {
        // Two's complement for negative numbers
        let bit_len = dec.value.len() * 8;
        int_value -= 1i128 << bit_len;
    }

    let divisor = 10f64.powi(dec.scale as i32);
    int_value as f64 / divisor
}

/// Determine CrossEntryType based on transaction type
fn determine_cross_entry_type(transaction_type: i32) -> CrossEntryType {
    use super::schema::transaction_type::*;
    match transaction_type {
        SECURITY_TRANSFER => CrossEntryType::PortfolioTransfer,
        CASH_TRANSFER => CrossEntryType::AccountTransfer,
        PURCHASE | SALE => CrossEntryType::BuySell,
        _ => CrossEntryType::BuySell,
    }
}

/// Format a PTimestamp to ISO 8601 string
fn format_timestamp(ts: &schema::PTimestamp) -> String {
    chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string())
        .unwrap_or_default()
}

/// Convert protobuf Taxonomy to pp::Taxonomy
fn convert_taxonomy(pb: &schema::PTaxonomy) -> Result<Taxonomy> {
    let mut tax = Taxonomy::new(pb.id.clone(), pb.name.clone());
    tax.dimensions = pb.dimensions.clone();

    // Convert classifications from flat list to tree structure
    if !pb.classifications.is_empty() {
        tax.root = build_classification_tree(&pb.classifications);
    }

    Ok(tax)
}

/// Build a classification tree from a flat list of PClassification
fn build_classification_tree(classifications: &[schema::PClassification]) -> Option<Classification> {
    use std::collections::HashMap;

    if classifications.is_empty() {
        return None;
    }

    // First pass: convert all classifications
    let mut nodes: HashMap<String, Classification> = HashMap::new();
    let mut root_id: Option<String> = None;

    for pb_class in classifications {
        let mut class = Classification::new(pb_class.id.clone(), pb_class.name.clone());
        class.color = pb_class.color.clone();
        class.weight = pb_class.weight;

        // Convert assignments
        for pb_assign in &pb_class.assignments {
            let assign = ClassificationAssignment {
                vehicle_class: "security".to_string(), // Default to security
                vehicle_uuid: pb_assign.vehicle_uuid.clone(),
                weight: pb_assign.weight.unwrap_or(10000),
                rank: pb_assign.rank,
            };
            class.assignments.push(assign);
        }

        // Convert data attributes
        if let Some(ref data) = pb_class.data {
            if let Some(ref value) = data.value {
                if let Some(ref v) = value.value {
                    class.data.insert(data.key.clone(), v.clone());
                }
            }
        }

        // Track root (classification without parent)
        if pb_class.parent_id.is_none() || pb_class.parent_id.as_deref() == Some("") {
            root_id = Some(pb_class.id.clone());
        }

        nodes.insert(pb_class.id.clone(), class);
    }

    // Second pass: build parent-child relationships
    // We need to iterate multiple times to handle deep hierarchies
    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();

    for pb_class in classifications {
        if let Some(ref parent_id) = pb_class.parent_id {
            if !parent_id.is_empty() {
                children_map
                    .entry(parent_id.clone())
                    .or_default()
                    .push(pb_class.id.clone());
            }
        }
    }

    // Recursively attach children
    fn attach_children(
        node_id: &str,
        nodes: &mut HashMap<String, Classification>,
        children_map: &HashMap<String, Vec<String>>,
    ) -> Option<Classification> {
        let mut node = nodes.remove(node_id)?;

        if let Some(child_ids) = children_map.get(node_id) {
            for child_id in child_ids {
                if let Some(child) = attach_children(child_id, nodes, children_map) {
                    node.children.push(child);
                }
            }
        }

        Some(node)
    }

    // Build tree starting from root
    if let Some(ref root) = root_id {
        attach_children(root, &mut nodes, &children_map)
    } else {
        // If no explicit root, try to find nodes without parents
        let orphan_ids: Vec<String> = nodes
            .keys()
            .filter(|id| {
                !classifications
                    .iter()
                    .any(|c| c.parent_id.as_ref() == Some(*id))
            })
            .cloned()
            .collect();

        if orphan_ids.len() == 1 {
            attach_children(&orphan_ids[0], &mut nodes, &children_map)
        } else if !orphan_ids.is_empty() {
            // Create a synthetic root if multiple orphans
            let mut root = Classification::new("root".to_string(), "Root".to_string());
            for orphan_id in orphan_ids {
                if let Some(child) = attach_children(&orphan_id, &mut nodes, &children_map) {
                    root.children.push(child);
                }
            }
            Some(root)
        } else {
            None
        }
    }
}

/// Convert protobuf Dashboard to pp::Dashboard
fn convert_dashboard(pb: &schema::PDashboard) -> Result<Dashboard> {
    let mut dash = Dashboard {
        name: pb.name.clone(),
        id: non_empty(&pb.id),
        columns: Vec::new(),
        configuration: serde_json::Value::Null,
    };

    for col in &pb.columns {
        let mut column = DashboardColumn {
            weight: (col.weight != 0).then_some(col.weight),
            widgets: Vec::new(),
        };

        for widget in &col.widgets {
            column.widgets.push(DashboardWidget {
                widget_type: widget.widget_type.clone(),
                label: non_empty(&widget.label),
                configuration: serde_json::Value::Null,
            });
        }

        dash.columns.push(column);
    }

    Ok(dash)
}

/// Convert empty string to None
fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;
    use std::path::PathBuf;

    #[test]
    fn test_header_validation() {
        let invalid = b"INVALID";
        assert!(parse_to_client(invalid).is_err());

        let too_short = b"PPPB";
        assert!(parse_to_client(too_short).is_err());
    }

    #[test]
    fn test_days_to_date() {
        // 2024-01-01 is 19723 days since 1970-01-01
        let date = days_to_date(19723);
        assert!(date.is_some());
        let d = date.unwrap();
        assert_eq!(d.year(), 2024);
        assert_eq!(d.month(), 1);
        assert_eq!(d.day(), 1);
    }

    #[test]
    fn test_parse_real_file() {
        let path = PathBuf::from("/Users/ricoullmann/Documents/PP/Portfolio.portfolio");
        if path.exists() {
            let result = parse_portfolio_file(&path);
            match result {
                Ok(client) => {
                    println!("Version: {}", client.version);
                    println!("Base currency: {}", client.base_currency);
                    println!("Parsed {} securities", client.securities.len());
                    println!("Parsed {} accounts", client.accounts.len());
                    println!("Parsed {} portfolios", client.portfolios.len());

                    // Print first security details
                    if let Some(sec) = client.securities.first() {
                        println!("First security: {} ({:?})", sec.name, sec.isin);
                        println!("  Prices: {}", sec.prices.len());
                    }

                    // Analyze transactions per portfolio
                    println!("\n=== PORTFOLIO TRANSACTIONS ===");
                    for portfolio in &client.portfolios {
                        println!("\nPortfolio: {} ({} transactions)", portfolio.name, portfolio.transactions.len());

                        // Count by type
                        let mut type_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
                        for tx in &portfolio.transactions {
                            *type_counts.entry(tx.transaction_type.as_str().to_string()).or_insert(0) += 1;
                        }
                        for (tx_type, count) in &type_counts {
                            println!("  {}: {}", tx_type, count);
                        }

                        // Calculate holdings
                        let holdings = portfolio.holdings();
                        println!("  Active holdings: {}", holdings.len());

                        // Show first few transactions
                        for tx in portfolio.transactions.iter().take(3) {
                            println!("    {} {:?} {} shares",
                                tx.date.format("%Y-%m-%d"),
                                tx.transaction_type,
                                crate::pp::common::shares::to_decimal(tx.shares));
                        }
                    }

                    // Analyze account transactions
                    println!("\n=== ACCOUNT TRANSACTIONS ===");
                    for account in &client.accounts {
                        println!("\nAccount: {} ({} transactions)", account.name, account.transactions.len());

                        let mut type_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
                        for tx in &account.transactions {
                            *type_counts.entry(tx.transaction_type.as_str().to_string()).or_insert(0) += 1;
                        }
                        for (tx_type, count) in &type_counts {
                            println!("  {}: {}", tx_type, count);
                        }
                    }

                    assert!(!client.securities.is_empty() || !client.accounts.is_empty());
                }
                Err(e) => {
                    println!("Parse error: {:?}", e);
                    // Try to decode raw to see what's happening
                    use std::io::Read;
                    let file = std::fs::File::open(&path).unwrap();
                    let mut archive = zip::ZipArchive::new(file).unwrap();
                    let mut data_file = archive.by_name("data.portfolio").unwrap();
                    let mut data = Vec::new();
                    data_file.read_to_end(&mut data).unwrap();
                    println!("Data length: {} bytes", data.len());
                    println!("Header: {:?}", &data[0..6]);
                    println!("First proto bytes: {:02x?}", &data[6..20]);

                    // Try raw decode
                    use prost::Message;
                    let proto_data = &data[6..];
                    match schema::PClient::decode(proto_data) {
                        Ok(pb) => {
                            println!("Raw decode succeeded!");
                            println!("Version: {}", pb.version);
                            println!("Securities: {}", pb.securities.len());
                            println!("Transactions in file: {}", pb.transactions.len());

                            // Analyze raw transactions
                            let mut type_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
                            for tx in &pb.transactions {
                                *type_counts.entry(tx.transaction_type).or_insert(0) += 1;
                            }
                            println!("Transaction types (raw):");
                            for (tx_type, count) in &type_counts {
                                println!("  Type {}: {}", tx_type, count);
                            }
                        }
                        Err(e2) => {
                            println!("Raw decode error: {:?}", e2);
                        }
                    }
                }
            }
        }
    }
}

