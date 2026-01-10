//! Writer for Portfolio Performance protobuf binary files.
//!
//! Creates .portfolio files compatible with Portfolio Performance.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use prost::Message;
use zip::write::FileOptions;
use zip::ZipWriter;

use super::schema::{
    self, PAccount, PClassification, PClassificationAssignment, PClient, PDashboard,
    PDashboardColumn, PDashboardWidget, PFullHistoricalPrice, PInvestmentPlan, PPortfolio, PPrice,
    PSecurity, PTimestamp, PTaxonomy, PTransaction, PTransactionUnit, PWatchlist,
};
use crate::pp::{
    taxonomy::Classification,
    transaction::{
        AccountTransaction, AccountTransactionType, PortfolioTransaction, PortfolioTransactionType,
        TransactionUnit, UnitType,
    },
    Account, Client, Portfolio, Security,
};

/// Write a Client to a .portfolio file (ZIP archive)
pub fn write_portfolio_file(path: &Path, client: &Client) -> Result<()> {
    let data = serialize_client(client)?;

    let file = std::fs::File::create(path).context("Failed to create file")?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    zip.start_file("data.portfolio", options)
        .context("Failed to start ZIP entry")?;

    zip.write_all(&data)
        .context("Failed to write data to ZIP")?;

    zip.finish().context("Failed to finish ZIP archive")?;

    Ok(())
}

/// Serialize a Client to protobuf bytes with header
pub fn serialize_client(client: &Client) -> Result<Vec<u8>> {
    let pb_client = convert_to_protobuf(client)?;

    // Encode protobuf
    let mut proto_bytes = Vec::new();
    pb_client
        .encode(&mut proto_bytes)
        .context("Failed to encode protobuf")?;

    // Prepend header
    let mut data = Vec::with_capacity(super::HEADER_LEN + proto_bytes.len());
    data.extend_from_slice(super::HEADER);
    data.extend_from_slice(&proto_bytes);

    Ok(data)
}

/// Convert pp::Client to protobuf PClient
fn convert_to_protobuf(client: &Client) -> Result<PClient> {
    let mut pb = PClient {
        version: client.version,
        base_currency: client.base_currency.clone(),
        securities: Vec::new(),
        accounts: Vec::new(),
        portfolios: Vec::new(),
        transactions: Vec::new(),
        plans: Vec::new(),
        watchlists: Vec::new(),
        taxonomies: Vec::new(),
        dashboards: Vec::new(),
        properties: Vec::new(),
        settings: None,
    };

    // Convert securities
    for sec in &client.securities {
        pb.securities.push(convert_security(sec));
    }

    // Convert accounts
    for acc in &client.accounts {
        pb.accounts.push(convert_account(acc));
    }

    // Convert portfolios
    for port in &client.portfolios {
        pb.portfolios.push(convert_portfolio(port));
    }

    // Collect all transactions into a unified list
    // Portfolio transactions
    for portfolio in &client.portfolios {
        for tx in &portfolio.transactions {
            if let Some(pb_tx) = convert_portfolio_transaction(tx, &portfolio.uuid) {
                pb.transactions.push(pb_tx);
            }
        }
    }

    // Account transactions
    for account in &client.accounts {
        for tx in &account.transactions {
            if let Some(pb_tx) = convert_account_transaction(tx, &account.uuid) {
                pb.transactions.push(pb_tx);
            }
        }
    }

    // Convert watchlists
    for wl in &client.watchlists {
        pb.watchlists.push(PWatchlist {
            name: wl.name.clone(),
            securities: wl.security_uuids.clone(),
        });
    }

    // Convert investment plans
    for plan in &client.plans {
        pb.plans.push(convert_investment_plan(plan));
    }

    // Convert taxonomies
    for tax in &client.taxonomies {
        pb.taxonomies.push(convert_taxonomy(tax));
    }

    // Convert dashboards
    for dash in &client.dashboards {
        pb.dashboards.push(convert_dashboard(dash));
    }

    // Convert settings
    pb.settings = convert_settings_to_protobuf(&client.settings);

    // Convert properties
    if let Some(props) = client.properties.as_object() {
        for (key, value) in props {
            if let Some(val_str) = value.as_str() {
                pb.properties.push(schema::PProperty {
                    key: key.clone(),
                    value: val_str.to_string(),
                });
            }
        }
    }

    Ok(pb)
}

/// Convert pp::Security to PSecurity
fn convert_security(sec: &Security) -> PSecurity {
    let mut pb = PSecurity {
        uuid: sec.uuid.clone(),
        name: sec.name.clone(),
        currency_code: Some(sec.currency.clone()),
        target_currency_code: sec.target_currency.clone(),
        online_id: sec.online_id.clone(),
        isin: sec.isin.clone(),
        ticker_symbol: sec.ticker.clone(),
        wkn: sec.wkn.clone(),
        calendar: sec.calendar.clone(),
        feed: sec.feed.clone(),
        feed_url: sec.feed_url.clone(),
        is_retired: sec.is_retired,
        note: sec.note.clone(),
        updated_at: sec.updated_at.as_ref().and_then(|s| parse_timestamp(s)),
        latest_feed: sec.latest_feed.clone(),
        latest_feed_url: sec.latest_feed_url.clone(),
        attributes: convert_attributes_to_protobuf(&sec.attributes),
        properties: convert_attributes_to_protobuf(&sec.properties),
        ..Default::default()
    };

    // Convert prices
    for price in &sec.prices {
        pb.prices.push(PPrice {
            date: date_to_days(price.date),
            value: price.value,
        });
    }

    // Convert latest price
    if let Some(ref latest) = sec.latest {
        if let Some(date) = latest.date {
            pb.latest = Some(PFullHistoricalPrice {
                date: date_to_days(date),
                close: latest.value.unwrap_or(0),
                high: latest.high,
                low: latest.low,
                volume: latest.volume,
            });
        }
    }

    pb
}

/// Convert pp::Account to PAccount
fn convert_account(acc: &Account) -> PAccount {
    PAccount {
        uuid: acc.uuid.clone(),
        name: acc.name.clone(),
        currency_code: acc.currency.clone(),
        note: acc.note.clone(),
        is_retired: acc.is_retired,
        attributes: convert_attributes_to_protobuf(&acc.attributes),
        updated_at: acc.updated_at.as_ref().and_then(|s| parse_timestamp(s)),
    }
}

/// Convert pp::Portfolio to PPortfolio
fn convert_portfolio(port: &Portfolio) -> PPortfolio {
    PPortfolio {
        uuid: port.uuid.clone(),
        name: port.name.clone(),
        note: port.note.clone(),
        is_retired: port.is_retired,
        reference_account: port.reference_account_uuid.clone(),
        attributes: convert_attributes_to_protobuf(&port.attributes),
        updated_at: port.updated_at.as_ref().and_then(|s| parse_timestamp(s)),
    }
}

/// Convert pp::PortfolioTransaction to PTransaction
fn convert_portfolio_transaction(tx: &PortfolioTransaction, portfolio_uuid: &str) -> Option<PTransaction> {
    use schema::transaction_type::*;

    let transaction_type = match tx.transaction_type {
        PortfolioTransactionType::Buy => PURCHASE,
        PortfolioTransactionType::Sell => SALE,
        PortfolioTransactionType::DeliveryInbound => INBOUND_DELIVERY,
        PortfolioTransactionType::DeliveryOutbound => OUTBOUND_DELIVERY,
        PortfolioTransactionType::TransferIn => SECURITY_TRANSFER,
        PortfolioTransactionType::TransferOut => {
            // TRANSFER_OUT is the outgoing side - skip it, only write SECURITY_TRANSFER once
            return None;
        }
    };

    let mut pb = PTransaction {
        uuid: tx.uuid.clone(),
        transaction_type,
        portfolio: Some(portfolio_uuid.to_string()),
        account: None,
        other_account: None,
        other_portfolio: None,
        other_uuid: tx.cross_entry.as_ref().map(|ce| ce.target_uuid.clone()),
        other_updated_at: tx.other_updated_at.as_ref().and_then(|s| parse_timestamp(s)),
        date: Some(datetime_to_timestamp(tx.date)),
        currency_code: tx.amount.currency.clone(),
        amount: tx.amount.amount,
        shares: Some(tx.shares),
        note: tx.note.clone(),
        security: tx.security_uuid.clone(),
        units: Vec::new(),
        updated_at: tx.updated_at.as_ref().and_then(|s| parse_timestamp(s)),
        source: tx.source.clone(),
    };

    // Convert transaction units
    for unit in &tx.units {
        pb.units.push(convert_transaction_unit(unit));
    }

    Some(pb)
}

/// Convert pp::AccountTransaction to PTransaction
fn convert_account_transaction(tx: &AccountTransaction, account_uuid: &str) -> Option<PTransaction> {
    use schema::transaction_type::*;

    let transaction_type = match tx.transaction_type {
        AccountTransactionType::Deposit => DEPOSIT,
        AccountTransactionType::Removal => REMOVAL,
        AccountTransactionType::Dividends => DIVIDEND,
        AccountTransactionType::Interest => INTEREST,
        AccountTransactionType::InterestCharge => INTEREST_CHARGE,
        AccountTransactionType::Taxes => TAX,
        AccountTransactionType::TaxRefund => TAX_REFUND,
        AccountTransactionType::Fees => FEE,
        AccountTransactionType::FeesRefund => FEE_REFUND,
        AccountTransactionType::Buy => PURCHASE,
        AccountTransactionType::Sell => SALE,
        AccountTransactionType::TransferIn => {
            // Skip TRANSFER_IN, only write CASH_TRANSFER once from the source
            return None;
        }
        AccountTransactionType::TransferOut => CASH_TRANSFER,
    };

    let mut pb = PTransaction {
        uuid: tx.uuid.clone(),
        transaction_type,
        account: Some(account_uuid.to_string()),
        portfolio: None,
        other_account: tx.cross_entry.as_ref().map(|ce| ce.target_uuid.clone()),
        other_portfolio: None,
        other_uuid: tx.cross_entry.as_ref().map(|ce| ce.target_uuid.clone()),
        other_updated_at: tx.other_updated_at.as_ref().and_then(|s| parse_timestamp(s)),
        date: Some(datetime_to_timestamp(tx.date)),
        currency_code: tx.amount.currency.clone(),
        amount: tx.amount.amount,
        shares: tx.shares,
        note: tx.note.clone(),
        security: tx.security_uuid.clone(),
        units: Vec::new(),
        updated_at: tx.updated_at.as_ref().and_then(|s| parse_timestamp(s)),
        source: tx.source.clone(),
    };

    // Convert transaction units
    for unit in &tx.units {
        pb.units.push(convert_transaction_unit(unit));
    }

    Some(pb)
}

/// Convert TransactionUnit to PTransactionUnit
fn convert_transaction_unit(unit: &TransactionUnit) -> PTransactionUnit {
    use schema::unit_type::*;

    let unit_type = match unit.unit_type {
        UnitType::GrossValue => GROSS_VALUE,
        UnitType::Tax => TAX,
        UnitType::Fee => FEE,
    };

    let mut pb = PTransactionUnit {
        unit_type,
        amount: unit.amount.amount,
        currency_code: unit.amount.currency.clone(),
        fx_amount: None,
        fx_currency_code: None,
        fx_rate_to_base: None,
    };

    // Add forex info if present
    if let Some(ref forex) = unit.forex {
        pb.fx_amount = Some(forex.amount.amount);
        pb.fx_currency_code = Some(forex.amount.currency.clone());
        pb.fx_rate_to_base = Some(encode_decimal_value(forex.exchange_rate));
    }

    pb
}

/// Convert InvestmentPlan to PInvestmentPlan
fn convert_investment_plan(plan: &crate::pp::InvestmentPlan) -> PInvestmentPlan {
    PInvestmentPlan {
        name: plan.name.clone(),
        note: plan.note.clone(),
        security: plan.security_uuid.clone(),
        portfolio: plan.portfolio_uuid.clone(),
        account: plan.account_uuid.clone(),
        attributes: convert_attributes_to_protobuf(&plan.attributes),
        auto_generate: plan.auto_generate,
        date: plan
            .start
            .as_ref()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(date_to_days)
            .unwrap_or(0),
        interval: plan.interval,
        amount: plan.amount,
        fees: plan.fees,
        transactions: plan.transactions.clone(),
        taxes: plan.taxes,
        plan_type: plan.plan_type,
    }
}

/// Convert Taxonomy to PTaxonomy
fn convert_taxonomy(tax: &crate::pp::Taxonomy) -> PTaxonomy {
    let mut classifications = Vec::new();

    // Flatten the classification tree
    if let Some(ref root) = tax.root {
        flatten_classification(root, None, &mut classifications);
    }

    PTaxonomy {
        id: tax.id.clone(),
        name: tax.name.clone(),
        dimensions: tax.dimensions.clone(),
        classifications,
    }
}

/// Flatten a classification tree into a list
fn flatten_classification(
    class: &Classification,
    parent_id: Option<&str>,
    out: &mut Vec<PClassification>,
) {
    let pb = PClassification {
        id: class.id.clone(),
        parent_id: parent_id.map(String::from),
        name: class.name.clone(),
        color: class.color.clone(),
        weight: class.weight,
        data: None,
        assignments: class
            .assignments
            .iter()
            .map(|a| PClassificationAssignment {
                vehicle_uuid: a.vehicle_uuid.clone(),
                weight: Some(a.weight),
                rank: a.rank,
            })
            .collect(),
    };
    out.push(pb);

    // Recursively flatten children
    for child in &class.children {
        flatten_classification(child, Some(&class.id), out);
    }
}

/// Convert Dashboard to PDashboard
fn convert_dashboard(dash: &crate::pp::Dashboard) -> PDashboard {
    PDashboard {
        name: dash.name.clone(),
        id: dash.id.clone().unwrap_or_default(),
        columns: dash
            .columns
            .iter()
            .map(|col| PDashboardColumn {
                weight: col.weight.unwrap_or(100),
                widgets: col
                    .widgets
                    .iter()
                    .map(|w| PDashboardWidget {
                        widget_type: w.widget_type.clone(),
                        label: w.label.clone().unwrap_or_default(),
                        configuration: Vec::new(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Convert HashMap attributes to protobuf PKeyValue list
fn convert_attributes_to_protobuf(
    attrs: &std::collections::HashMap<String, String>,
) -> Vec<schema::PKeyValue> {
    attrs
        .iter()
        .map(|(key, value)| schema::PKeyValue {
            key: key.clone(),
            value: Some(schema::PAnyValue {
                kind: Some(schema::PAnyValueKind::String(value.clone())),
            }),
        })
        .collect()
}

/// Convert settings JSON back to PSettings
fn convert_settings_to_protobuf(settings: &serde_json::Value) -> Option<schema::PSettings> {
    if settings.is_null() {
        return None;
    }

    let mut pb_settings = schema::PSettings {
        bookmarks: Vec::new(),
        attribute_types: Vec::new(),
        configuration_sets: Vec::new(),
    };

    // Convert bookmarks
    if let Some(bookmarks) = settings.get("bookmarks").and_then(|v| v.as_array()) {
        for b in bookmarks {
            pb_settings.bookmarks.push(schema::PBookmark {
                label: b.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                pattern: b.get("pattern").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }

    // Convert attribute types
    if let Some(attr_types) = settings.get("attributeTypes").and_then(|v| v.as_array()) {
        for a in attr_types {
            pb_settings.attribute_types.push(schema::PAttributeType {
                id: a.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: a.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                column_label: a.get("columnLabel").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                source: a.get("source").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                target: a.get("target").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                attr_type: a.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }

    // Convert configuration sets
    if let Some(config_sets) = settings.get("configurationSets").and_then(|v| v.as_array()) {
        for c in config_sets {
            pb_settings.configuration_sets.push(schema::PConfigurationSet {
                key: c.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                uuid: c.get("uuid").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                data: c.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }

    Some(pb_settings)
}

/// Convert NaiveDate to days since epoch
fn date_to_days(date: NaiveDate) -> i64 {
    // PP uses days since Unix epoch (1970-01-01)
    // NaiveDate::from_num_days_from_ce uses days since year 1
    // Epoch is day 719163 in CE
    (date.num_days_from_ce() - 719163) as i64
}

/// Convert NaiveDateTime to PTimestamp
fn datetime_to_timestamp(dt: NaiveDateTime) -> PTimestamp {
    let secs = dt.and_utc().timestamp();
    let nanos = dt.nanosecond() as i32;
    PTimestamp {
        seconds: secs,
        nanos,
    }
}

/// Parse ISO timestamp string to PTimestamp
fn parse_timestamp(s: &str) -> Option<PTimestamp> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .ok()
        .map(datetime_to_timestamp)
}

/// Encode f64 to PDecimalValue
fn encode_decimal_value(value: f64) -> schema::PDecimalValue {
    // Convert to a scaled integer representation
    // We use scale=8 for 8 decimal places (matching PP's precision)
    let scale = 8u32;
    let scaled = (value * 10f64.powi(scale as i32)).round() as i128;

    // Convert to big-endian bytes
    let mut bytes = Vec::new();
    let mut v = scaled.abs();

    if v == 0 {
        bytes.push(0);
    } else {
        while v > 0 {
            bytes.push((v & 0xff) as u8);
            v >>= 8;
        }
        bytes.reverse();

        // Handle sign (two's complement)
        if scaled < 0 {
            // Negate using two's complement
            let mut carry = true;
            for b in bytes.iter_mut().rev() {
                let (new_b, new_carry) = (!*b).overflowing_add(if carry { 1 } else { 0 });
                *b = new_b;
                carry = new_carry;
            }
            // Ensure high bit is set for negative numbers
            if bytes[0] & 0x80 == 0 {
                bytes.insert(0, 0xff);
            }
        } else if bytes[0] & 0x80 != 0 {
            // Positive number but high bit set, add a zero byte
            bytes.insert(0, 0);
        }
    }

    schema::PDecimalValue {
        scale,
        precision: bytes.len() as u32 * 8,
        value: bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pp::PriceEntry;
    use chrono::NaiveDate;

    #[test]
    fn test_date_to_days() {
        // 2024-01-01 should be ~19723 days since 1970-01-01
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let days = date_to_days(date);
        assert_eq!(days, 19723);
    }

    #[test]
    fn test_roundtrip_date() {
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let days = date_to_days(date);

        // Reverse: days_to_date from parser
        let recovered = NaiveDate::from_num_days_from_ce_opt((days + 719163) as i32).unwrap();
        assert_eq!(date, recovered);
    }

    #[test]
    fn test_encode_decimal() {
        let dec = encode_decimal_value(1.5);
        assert_eq!(dec.scale, 8);
        assert!(!dec.value.is_empty());

        // 1.5 * 10^8 = 150_000_000
        // Should decode back correctly
    }

    #[test]
    fn test_serialize_empty_client() {
        let client = Client::new("EUR");
        let data = serialize_client(&client).unwrap();

        // Should start with header
        assert_eq!(&data[0..6], b"PPPBV1");

        // Should be valid protobuf
        let proto_data = &data[6..];
        let decoded = PClient::decode(proto_data).unwrap();
        assert_eq!(decoded.base_currency, "EUR");
    }

    #[test]
    fn test_serialize_with_security() {
        let mut client = Client::new("EUR");

        let mut sec = Security::new("sec-1".into(), "Apple Inc.".into(), "USD".into());
        sec.isin = Some("US0378331005".into());
        sec.prices.push(PriceEntry::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            15000000000, // $150.00
        ));
        client.securities.push(sec);

        let data = serialize_client(&client).unwrap();

        // Verify it decodes correctly
        let proto_data = &data[6..];
        let decoded = PClient::decode(proto_data).unwrap();
        assert_eq!(decoded.securities.len(), 1);
        assert_eq!(decoded.securities[0].name, "Apple Inc.");
        assert_eq!(decoded.securities[0].isin, Some("US0378331005".into()));
    }
}
