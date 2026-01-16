//! CSV import and export commands for Tauri

use crate::db;
use crate::events::{emit_data_changed, DataChangedPayload};
use crate::pp::common::{prices, shares};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use tauri::{command, AppHandle};

// ============================================================================
// AI CSV Analysis Types
// ============================================================================

/// Request for AI-assisted CSV analysis
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCsvAnalysisRequest {
    pub csv_content: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
}

/// AI suggestion for column mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMappingSuggestion {
    pub field: String,
    pub column_index: Option<usize>,
    pub column_name: Option<String>,
    pub confidence: f32,
    pub reason: String,
}

/// AI CSV analysis response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCsvAnalysisResponse {
    pub detected_broker: Option<String>,
    pub broker_confidence: f32,
    pub mapping_suggestions: Vec<AiMappingSuggestion>,
    pub analysis_notes: String,
    pub raw_response: String,
}

// ============================================================================
// Export Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvExportResult {
    pub path: String,
    pub rows_exported: usize,
}

// ============================================================================
// Export Commands
// ============================================================================

/// Export transactions to CSV
#[command]
pub fn export_transactions_csv(
    path: String,
    owner_type: Option<String>,
    owner_id: Option<i64>,
) -> Result<CsvExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut query = String::from(
        r#"
        SELECT
            t.date,
            t.txn_type,
            COALESCE(s.name, '') as security_name,
            COALESCE(s.isin, '') as isin,
            t.shares,
            t.amount,
            t.currency,
            CASE t.owner_type
                WHEN 'portfolio' THEN (SELECT name FROM pp_portfolio WHERE id = t.owner_id)
                WHEN 'account' THEN (SELECT name FROM pp_account WHERE id = t.owner_id)
            END as owner_name,
            t.owner_type,
            COALESCE(t.note, '') as note,
            (SELECT COALESCE(SUM(amount), 0) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'FEE') as fees,
            (SELECT COALESCE(SUM(amount), 0) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX') as taxes
        FROM pp_txn t
        LEFT JOIN pp_security s ON s.id = t.security_id
        WHERE 1=1
        "#,
    );

    // Build parameterized query to prevent SQL injection
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref ot) = owner_type {
        query.push_str(" AND t.owner_type = ?");
        params.push(Box::new(ot.clone()));
    }
    if let Some(oid) = owner_id {
        query.push_str(" AND t.owner_id = ?");
        params.push(Box::new(oid));
    }
    query.push_str(" ORDER BY t.date DESC");

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut file = File::create(&path).map_err(|e| e.to_string())?;

    // Write header
    writeln!(
        file,
        "Datum;Typ;Wertpapier;ISIN;Stück;Betrag;Währung;Konto/Depot;Bereich;Notiz;Gebühren;Steuern"
    )
    .map_err(|e| e.to_string())?;

    let mut count = 0;
    for row in rows.flatten() {
        let (date, txn_type, security, isin, shares_val, amount, currency, owner, owner_type, note, fees, taxes) = row;

        let shares_str = shares_val
            .map(|s| format!("{:.6}", shares::to_decimal(s)))
            .unwrap_or_default();

        let amount_str = format!("{:.2}", amount as f64 / 100.0);
        let fees_str = format!("{:.2}", fees as f64 / 100.0);
        let taxes_str = format!("{:.2}", taxes as f64 / 100.0);

        writeln!(
            file,
            "{};{};{};{};{};{};{};{};{};{};{};{}",
            date,
            txn_type,
            security,
            isin,
            shares_str,
            amount_str,
            currency,
            owner.unwrap_or_default(),
            owner_type,
            note.replace(';', ","),
            fees_str,
            taxes_str
        )
        .map_err(|e| e.to_string())?;

        count += 1;
    }

    Ok(CsvExportResult {
        path,
        rows_exported: count,
    })
}

/// Export holdings to CSV
#[command]
pub fn export_holdings_csv(path: String) -> Result<CsvExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let query = r#"
        SELECT
            s.name,
            COALESCE(s.isin, '') as isin,
            COALESCE(s.ticker, '') as ticker,
            s.currency,
            SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) as net_shares,
            (SELECT value FROM pp_latest_price WHERE security_id = s.id) as latest_price,
            p.name as portfolio_name
        FROM pp_txn t
        JOIN pp_security s ON s.id = t.security_id
        JOIN pp_portfolio p ON p.id = t.owner_id
        WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
        GROUP BY s.id, p.id
        HAVING net_shares > 0
        ORDER BY s.name, p.name
    "#;

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut file = File::create(&path).map_err(|e| e.to_string())?;

    // Write header
    writeln!(
        file,
        "Wertpapier;ISIN;Ticker;Währung;Stück;Kurs;Wert;Depot"
    )
    .map_err(|e| e.to_string())?;

    let mut count = 0;
    for row in rows.flatten() {
        let (name, isin, ticker, currency, net_shares, latest_price, portfolio) = row;

        let shares_decimal = shares::to_decimal(net_shares);
        let price_decimal = latest_price.map(prices::to_decimal).unwrap_or(0.0);
        let value = shares_decimal * price_decimal;

        writeln!(
            file,
            "{};{};{};{};{:.6};{:.4};{:.2};{}",
            name, isin, ticker, currency, shares_decimal, price_decimal, value, portfolio
        )
        .map_err(|e| e.to_string())?;

        count += 1;
    }

    Ok(CsvExportResult {
        path,
        rows_exported: count,
    })
}

/// Export securities to CSV
#[command]
pub fn export_securities_csv(path: String) -> Result<CsvExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let query = r#"
        SELECT
            s.name,
            COALESCE(s.isin, '') as isin,
            COALESCE(s.wkn, '') as wkn,
            COALESCE(s.ticker, '') as ticker,
            s.currency,
            COALESCE(s.feed, '') as feed,
            s.is_retired,
            (SELECT COUNT(*) FROM pp_price WHERE security_id = s.id) as price_count,
            (SELECT value FROM pp_latest_price WHERE security_id = s.id) as latest_price,
            (SELECT date FROM pp_latest_price WHERE security_id = s.id) as latest_date
        FROM pp_security s
        ORDER BY s.name
    "#;

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, bool>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut file = File::create(&path).map_err(|e| e.to_string())?;

    // Write header
    writeln!(
        file,
        "Name;ISIN;WKN;Ticker;Währung;Kursquelle;Inaktiv;Kurse;Letzter Kurs;Kursdatum"
    )
    .map_err(|e| e.to_string())?;

    let mut count = 0;
    for row in rows.flatten() {
        let (name, isin, wkn, ticker, currency, feed, is_retired, price_count, latest_price, latest_date) = row;

        let price_str = latest_price
            .map(|p| format!("{:.4}", prices::to_decimal(p)))
            .unwrap_or_default();

        writeln!(
            file,
            "{};{};{};{};{};{};{};{};{};{}",
            name,
            isin,
            wkn,
            ticker,
            currency,
            feed,
            if is_retired { "Ja" } else { "Nein" },
            price_count,
            price_str,
            latest_date.unwrap_or_default()
        )
        .map_err(|e| e.to_string())?;

        count += 1;
    }

    Ok(CsvExportResult {
        path,
        rows_exported: count,
    })
}

/// Export account balances to CSV
#[command]
pub fn export_accounts_csv(path: String) -> Result<CsvExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let query = r#"
        SELECT
            a.name,
            a.currency,
            a.is_retired,
            (SELECT COUNT(*) FROM pp_txn WHERE owner_type = 'account' AND owner_id = a.id) as txn_count,
            COALESCE((
                SELECT SUM(CASE
                    WHEN txn_type IN ('DEPOSIT', 'INTEREST', 'DIVIDENDS', 'SELL', 'TRANSFER_IN', 'TAX_REFUND', 'FEES_REFUND') THEN amount
                    WHEN txn_type IN ('REMOVAL', 'INTEREST_CHARGE', 'BUY', 'TRANSFER_OUT', 'TAXES', 'FEES') THEN -amount
                    ELSE 0
                END)
                FROM pp_txn
                WHERE owner_type = 'account' AND owner_id = a.id
            ), 0) as balance
        FROM pp_account a
        ORDER BY a.name
    "#;

    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut file = File::create(&path).map_err(|e| e.to_string())?;

    // Write header
    writeln!(file, "Konto;Währung;Inaktiv;Buchungen;Saldo")
        .map_err(|e| e.to_string())?;

    let mut count = 0;
    for row in rows.flatten() {
        let (name, currency, is_retired, txn_count, balance) = row;

        writeln!(
            file,
            "{};{};{};{};{:.2}",
            name,
            currency,
            if is_retired { "Ja" } else { "Nein" },
            txn_count,
            balance as f64 / 100.0
        )
        .map_err(|e| e.to_string())?;

        count += 1;
    }

    Ok(CsvExportResult {
        path,
        rows_exported: count,
    })
}

// ============================================================================
// Import Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvColumn {
    pub index: usize,
    pub name: String,
    pub sample_values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvPreview {
    pub columns: Vec<CsvColumn>,
    pub row_count: usize,
    pub delimiter: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvColumnMapping {
    /// Maps target field to source column index
    pub date: Option<usize>,
    pub txn_type: Option<usize>,
    pub security_name: Option<usize>,
    pub isin: Option<usize>,
    pub shares: Option<usize>,
    pub amount: Option<usize>,
    pub currency: Option<usize>,
    pub fees: Option<usize>,
    pub taxes: Option<usize>,
    pub note: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportResult {
    pub rows_imported: usize,
    pub rows_skipped: usize,
    pub errors: Vec<String>,
}

// ============================================================================
// Import Commands
// ============================================================================

/// Preview a CSV file for import
#[command]
pub fn preview_csv(path: String) -> Result<CsvPreview, String> {
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    // Try to detect delimiter
    let lines: Vec<String> = reader.lines().take(10).filter_map(|l| l.ok()).collect();

    if lines.is_empty() {
        return Err("Empty file".to_string());
    }

    let delimiter = detect_delimiter(&lines[0]);

    // Parse header
    let header = &lines[0];
    let headers: Vec<&str> = header.split(delimiter).collect();

    // Build columns with sample values
    let mut columns: Vec<CsvColumn> = headers
        .iter()
        .enumerate()
        .map(|(i, name)| CsvColumn {
            index: i,
            name: name.trim().to_string(),
            sample_values: Vec::new(),
        })
        .collect();

    // Add sample values from data rows
    for line in lines.iter().skip(1).take(5) {
        let values: Vec<&str> = line.split(delimiter).collect();
        for (i, col) in columns.iter_mut().enumerate() {
            if let Some(v) = values.get(i) {
                col.sample_values.push(v.trim().to_string());
            }
        }
    }

    // Count total rows
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let row_count = reader.lines().count().saturating_sub(1); // Exclude header

    Ok(CsvPreview {
        columns,
        row_count,
        delimiter,
    })
}

/// Import transactions from CSV
#[command]
pub fn import_transactions_csv(
    app: AppHandle,
    path: String,
    mapping: CsvColumnMapping,
    portfolio_id: i64,
    delimiter: Option<char>,
) -> Result<CsvImportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    if lines.is_empty() {
        return Err("Empty file".to_string());
    }

    let delim = delimiter.unwrap_or_else(|| detect_delimiter(&lines[0]));

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors: Vec<String> = Vec::new();
    // Track affected securities for FIFO rebuild
    let mut affected_security_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

    // Get latest import_id
    let import_id: i64 = conn
        .query_row("SELECT id FROM pp_import ORDER BY id DESC LIMIT 1", [], |r| r.get(0))
        .unwrap_or(1);

    for (line_num, line) in lines.iter().enumerate().skip(1) {
        let values: Vec<&str> = line.split(delim).collect();

        // Parse date
        let date = mapping
            .date
            .and_then(|i| values.get(i))
            .and_then(|v| parse_date(v.trim()));

        if date.is_none() {
            errors.push(format!("Zeile {}: Ungültiges Datum", line_num + 1));
            skipped += 1;
            continue;
        }

        // Parse transaction type
        let txn_type = mapping
            .txn_type
            .and_then(|i| values.get(i))
            .map(|v| map_transaction_type(v.trim()))
            .unwrap_or("BUY".to_string());

        // Parse shares
        let shares = mapping
            .shares
            .and_then(|i| values.get(i))
            .and_then(|v| parse_decimal(v.trim()))
            .map(|s| (s * 100_000_000.0) as i64);

        // Parse amount
        let amount = mapping
            .amount
            .and_then(|i| values.get(i))
            .and_then(|v| parse_decimal(v.trim()))
            .map(|a| (a * 100.0) as i64)
            .unwrap_or(0);

        // Parse currency
        let currency = mapping
            .currency
            .and_then(|i| values.get(i))
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|| "EUR".to_string());

        // Parse fees and taxes
        let fees = mapping
            .fees
            .and_then(|i| values.get(i))
            .and_then(|v| parse_decimal(v.trim()))
            .map(|f| (f * 100.0) as i64)
            .unwrap_or(0);

        let taxes = mapping
            .taxes
            .and_then(|i| values.get(i))
            .and_then(|v| parse_decimal(v.trim()))
            .map(|t| (t * 100.0) as i64)
            .unwrap_or(0);

        // Parse note
        let note = mapping
            .note
            .and_then(|i| values.get(i))
            .map(|v| v.trim().to_string());

        // Find or create security
        let security_id = if let Some(isin_idx) = mapping.isin {
            if let Some(isin) = values.get(isin_idx).map(|v| v.trim()) {
                if !isin.is_empty() {
                    find_or_create_security(conn, isin, mapping.security_name.and_then(|i| values.get(i).map(|v| v.trim())), &currency, import_id)
                        .map_err(|e| e.to_string())?
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Generate UUID
        let uuid = uuid::Uuid::new_v4().to_string();

        // Insert transaction
        let result = conn.execute(
            r#"
            INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
            VALUES (?, ?, 'portfolio', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                import_id,
                uuid,
                portfolio_id,
                security_id,
                txn_type,
                date.unwrap().to_string(),
                amount,
                currency,
                shares,
                note
            ],
        );

        match result {
            Ok(_) => {
                let txn_id = conn.last_insert_rowid();

                // Insert fees unit if present
                if fees > 0 {
                    let _ = conn.execute(
                        "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency) VALUES (?, 'FEE', ?, ?)",
                        rusqlite::params![txn_id, fees, currency],
                    );
                }

                // Insert taxes unit if present
                if taxes > 0 {
                    let _ = conn.execute(
                        "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency) VALUES (?, 'TAX', ?, ?)",
                        rusqlite::params![txn_id, taxes, currency],
                    );
                }

                // Track security for FIFO rebuild
                if let Some(sec_id) = security_id {
                    affected_security_ids.insert(sec_id);
                }

                imported += 1;
            }
            Err(e) => {
                errors.push(format!("Zeile {}: {}", line_num + 1, e));
                skipped += 1;
            }
        }
    }

    // Rebuild FIFO lots for all affected securities
    for sec_id in &affected_security_ids {
        if let Err(e) = crate::fifo::build_fifo_lots(conn, *sec_id) {
            log::warn!("CSV Import: Failed to rebuild FIFO lots for security {}: {}", sec_id, e);
        }
    }
    if !affected_security_ids.is_empty() {
        log::info!("CSV Import: Rebuilt FIFO lots for {} securities", affected_security_ids.len());
    }

    // Emit data changed event for frontend refresh
    emit_data_changed(
        &app,
        DataChangedPayload::import(affected_security_ids.into_iter().collect()),
    );

    Ok(CsvImportResult {
        rows_imported: imported,
        rows_skipped: skipped,
        errors,
    })
}

/// Import prices from CSV for a security
#[command]
pub fn import_prices_csv(
    path: String,
    security_id: i64,
    date_column: usize,
    price_column: usize,
    delimiter: Option<char>,
) -> Result<CsvImportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    if lines.is_empty() {
        return Err("Empty file".to_string());
    }

    let delim = delimiter.unwrap_or_else(|| detect_delimiter(&lines[0]));

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors: Vec<String> = Vec::new();

    for (line_num, line) in lines.iter().enumerate().skip(1) {
        let values: Vec<&str> = line.split(delim).collect();

        let date = values
            .get(date_column)
            .and_then(|v| parse_date(v.trim()));

        let price = values
            .get(price_column)
            .and_then(|v| parse_decimal(v.trim()));

        if date.is_none() || price.is_none() {
            errors.push(format!("Zeile {}: Ungültige Daten", line_num + 1));
            skipped += 1;
            continue;
        }

        let price_scaled = (price.unwrap() * 100_000_000.0) as i64;

        let result = conn.execute(
            "INSERT OR REPLACE INTO pp_price (security_id, date, value) VALUES (?, ?, ?)",
            rusqlite::params![security_id, date.unwrap().to_string(), price_scaled],
        );

        match result {
            Ok(_) => imported += 1,
            Err(e) => {
                errors.push(format!("Zeile {}: {}", line_num + 1, e));
                skipped += 1;
            }
        }
    }

    // Update latest price
    if imported > 0 {
        let _ = conn.execute(
            r#"
            INSERT OR REPLACE INTO pp_latest_price (security_id, date, value)
            SELECT security_id, date, value FROM pp_price
            WHERE security_id = ?
            ORDER BY date DESC LIMIT 1
            "#,
            [security_id],
        );
    }

    Ok(CsvImportResult {
        rows_imported: imported,
        rows_skipped: skipped,
        errors,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn detect_delimiter(line: &str) -> char {
    let semicolons = line.matches(';').count();
    let commas = line.matches(',').count();
    let tabs = line.matches('\t').count();

    if semicolons >= commas && semicolons >= tabs {
        ';'
    } else if tabs >= commas {
        '\t'
    } else {
        ','
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    // Try common formats
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%d.%m.%Y"))
        .or_else(|_| NaiveDate::parse_from_str(s, "%d/%m/%Y"))
        .or_else(|_| NaiveDate::parse_from_str(s, "%m/%d/%Y"))
        .ok()
}

fn parse_decimal(s: &str) -> Option<f64> {
    // Handle German format (1.234,56) and US format (1,234.56)
    let cleaned = s
        .replace(" ", "")
        .replace("€", "")
        .replace("$", "")
        .replace("EUR", "")
        .replace("USD", "");

    // If contains both . and ,
    if cleaned.contains('.') && cleaned.contains(',') {
        // German: 1.234,56 -> 1234.56
        if cleaned.rfind(',') > cleaned.rfind('.') {
            cleaned.replace(".", "").replace(",", ".").parse().ok()
        } else {
            // US: 1,234.56 -> 1234.56
            cleaned.replace(",", "").parse().ok()
        }
    } else if cleaned.contains(',') {
        // Could be German decimal (1,5) or US thousands (1,000)
        if cleaned.len() - cleaned.rfind(',').unwrap_or(0) <= 3 {
            cleaned.replace(",", ".").parse().ok()
        } else {
            cleaned.replace(",", "").parse().ok()
        }
    } else {
        cleaned.parse().ok()
    }
}

fn map_transaction_type(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.contains("kauf") || lower.contains("buy") || lower.contains("purchase") {
        "BUY".to_string()
    } else if lower.contains("verkauf") || lower.contains("sell") || lower.contains("sale") {
        "SELL".to_string()
    } else if lower.contains("dividend") {
        "DIVIDENDS".to_string()
    } else if lower.contains("einlage") || lower.contains("deposit") {
        "DEPOSIT".to_string()
    } else if lower.contains("entnahme") || lower.contains("withdrawal") || lower.contains("removal") {
        "REMOVAL".to_string()
    } else if lower.contains("zins") || lower.contains("interest") {
        "INTEREST".to_string()
    } else if lower.contains("gebühr") || lower.contains("fee") {
        "FEES".to_string()
    } else if lower.contains("steuer") || lower.contains("tax") {
        "TAXES".to_string()
    } else if lower.contains("einlieferung") || lower.contains("delivery") && lower.contains("in") {
        "DELIVERY_INBOUND".to_string()
    } else if lower.contains("auslieferung") || lower.contains("delivery") && lower.contains("out") {
        "DELIVERY_OUTBOUND".to_string()
    } else {
        "BUY".to_string() // Default
    }
}

fn find_or_create_security(
    conn: &rusqlite::Connection,
    isin: &str,
    name: Option<&str>,
    currency: &str,
    import_id: i64,
) -> Result<Option<i64>, rusqlite::Error> {
    // Try to find existing
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM pp_security WHERE isin = ?",
            [isin],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing {
        return Ok(Some(id));
    }

    // Create new security
    let uuid = uuid::Uuid::new_v4().to_string();
    let sec_name = name.unwrap_or(isin);

    conn.execute(
        r#"
        INSERT INTO pp_security (import_id, uuid, name, currency, isin)
        VALUES (?, ?, ?, ?, ?)
        "#,
        rusqlite::params![import_id, uuid, sec_name, currency, isin],
    )?;

    Ok(Some(conn.last_insert_rowid()))
}

// ============================================================================
// Broker Template Commands
// ============================================================================

/// Detect broker format from CSV file headers
#[command]
pub fn detect_csv_broker(path: String) -> Result<crate::csv_import::BrokerDetectionResult, String> {
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    let first_line = reader
        .lines()
        .next()
        .ok_or_else(|| "Empty file".to_string())?
        .map_err(|e| e.to_string())?;

    let delimiter = detect_delimiter(&first_line);
    let headers: Vec<String> = first_line
        .split(delimiter)
        .map(|s| s.trim().to_string())
        .collect();

    Ok(crate::csv_import::detect_broker(&headers))
}

/// Get list of available broker templates
#[command]
pub fn get_broker_templates() -> Vec<crate::csv_import::BrokerTemplateSummary> {
    crate::csv_import::get_all_templates()
        .into_iter()
        .map(|t| crate::csv_import::BrokerTemplateSummary {
            id: t.id.to_string(),
            name: t.name.to_string(),
            description: Some(t.description.to_string()),
        })
        .collect()
}

/// Import transactions using a broker template
#[command]
pub fn import_csv_with_template(
    app: AppHandle,
    path: String,
    template_id: String,
    portfolio_id: i64,
) -> Result<CsvImportResult, String> {
    let template = crate::csv_import::get_template(&template_id)
        .ok_or_else(|| format!("Template '{}' not found", template_id))?;

    // Use the template's mapping
    import_transactions_csv(
        app,
        path,
        template.mapping.clone(),
        portfolio_id,
        Some(template.delimiter),
    )
}

// ============================================================================
// AI-Assisted CSV Analysis (Code-first, AI fallback)
// ============================================================================

/// AI prompt for CSV analysis
const CSV_ANALYSIS_PROMPT: &str = r#"Du bist ein Experte für Broker-CSV-Formate. Analysiere den folgenden CSV-Inhalt und identifiziere:

1. **Broker-Erkennung**: Welcher Broker (Trade Republic, Scalable Capital, ING-DiBa, DKB, DEGIRO, Comdirect, Consorsbank, Interactive Brokers) könnte diese CSV exportiert haben?

2. **Spalten-Mapping**: Ordne die Spalten den folgenden Feldern zu:
   - date: Transaktionsdatum
   - txnType: Transaktionstyp (Kauf, Verkauf, Dividende, etc.)
   - isin: ISIN-Nummer (12-stellig)
   - securityName: Wertpapiername
   - shares: Anzahl Stück
   - amount: Betrag
   - currency: Währung
   - fees: Gebühren
   - taxes: Steuern
   - note: Notiz/Beschreibung

Antworte AUSSCHLIESSLICH im folgenden JSON-Format:
{
  "broker": "Name des erkannten Brokers oder null",
  "brokerConfidence": 0.0-1.0,
  "mappings": [
    {"field": "date", "columnIndex": 0, "columnName": "Datum", "confidence": 0.95, "reason": "Header enthält 'Datum'"},
    {"field": "amount", "columnIndex": 3, "columnName": "Betrag", "confidence": 0.9, "reason": "Numerische Werte mit Komma"}
  ],
  "notes": "Zusätzliche Hinweise zur Analyse"
}

CSV-INHALT:
"#;

/// Analyze CSV with AI assistance
///
/// This is the AI fallback when automatic detection fails or has low confidence.
/// The AI analyzes the CSV structure and suggests column mappings.
#[command]
pub async fn analyze_csv_with_ai(
    request: AiCsvAnalysisRequest,
) -> Result<AiCsvAnalysisResponse, String> {
    use crate::ai::{claude, openai, gemini, perplexity, get_model_upgrade, ChatMessage,
        PortfolioInsightsContext, FeesAndTaxesSummary, InvestmentSummary};

    // Auto-upgrade deprecated models
    let model = if let Some(upgraded) = get_model_upgrade(&request.model) {
        log::info!("Auto-upgrading deprecated model {} to {}", request.model, upgraded);
        upgraded.to_string()
    } else {
        request.model.clone()
    };

    // Build the prompt with CSV content
    let prompt = format!("{}\n{}", CSV_ANALYSIS_PROMPT, request.csv_content);

    // Create a simple chat message
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: prompt,
    }];

    // Use a minimal context for text-only analysis (no portfolio data needed)
    let empty_context = PortfolioInsightsContext {
        holdings: vec![],
        total_value: 0.0,
        total_cost_basis: 0.0,
        total_gain_loss_percent: 0.0,
        ttwror: None,
        ttwror_annualized: None,
        irr: None,
        currency_allocation: vec![],
        top_positions: vec![],
        dividend_yield: None,
        annual_dividends: 0.0,
        recent_dividends: vec![],
        recent_transactions: vec![],
        watchlist: vec![],
        sold_positions: vec![],
        yearly_overview: vec![],
        portfolio_age_days: 0,
        analysis_date: "".to_string(),
        base_currency: "EUR".to_string(),
        user_name: None,
        provider_status: None,
        fees_and_taxes: FeesAndTaxesSummary {
            total_fees: 0.0,
            total_taxes: 0.0,
            fees_this_year: 0.0,
            taxes_this_year: 0.0,
            by_year: vec![],
        },
        investment_summary: InvestmentSummary {
            total_invested: 0.0,
            total_withdrawn: 0.0,
            net_invested: 0.0,
            total_deposits: 0.0,
            total_removals: 0.0,
            first_investment_date: None,
        },
        sector_allocation: vec![],
        portfolio_extremes: None,
    };

    // Call the appropriate AI provider
    let result = match request.provider.as_str() {
        "claude" => claude::chat(&model, &request.api_key, &messages, &empty_context).await,
        "openai" => openai::chat(&model, &request.api_key, &messages, &empty_context).await,
        "gemini" => gemini::chat(&model, &request.api_key, &messages, &empty_context).await,
        "perplexity" => perplexity::chat(&model, &request.api_key, &messages, &empty_context).await,
        _ => return Err(format!("Unbekannter Anbieter: {}", request.provider)),
    };

    match result {
        Ok(chat_response) => {
            // Try to parse the JSON response
            let raw_response = chat_response.response.clone();

            // Extract JSON from response (might be wrapped in markdown code blocks)
            let json_str = extract_json_from_response(&raw_response);

            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(json) => {
                    let broker = json.get("broker").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let broker_confidence = json.get("brokerConfidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as f32;

                    let mappings = json.get("mappings")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter().filter_map(|m| {
                                Some(AiMappingSuggestion {
                                    field: m.get("field")?.as_str()?.to_string(),
                                    column_index: m.get("columnIndex").and_then(|v| v.as_u64()).map(|n| n as usize),
                                    column_name: m.get("columnName").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    confidence: m.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32,
                                    reason: m.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                })
                            }).collect()
                        })
                        .unwrap_or_default();

                    let notes = json.get("notes")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    Ok(AiCsvAnalysisResponse {
                        detected_broker: broker,
                        broker_confidence,
                        mapping_suggestions: mappings,
                        analysis_notes: notes,
                        raw_response,
                    })
                }
                Err(_) => {
                    // If JSON parsing fails, return raw response with empty suggestions
                    Ok(AiCsvAnalysisResponse {
                        detected_broker: None,
                        broker_confidence: 0.0,
                        mapping_suggestions: vec![],
                        analysis_notes: "KI-Antwort konnte nicht als JSON geparst werden.".to_string(),
                        raw_response,
                    })
                }
            }
        }
        Err(e) => Err(format!("KI-Analyse fehlgeschlagen: {}", e.message)),
    }
}

/// Extract JSON from AI response (handles markdown code blocks)
fn extract_json_from_response(response: &str) -> String {
    // Try to find JSON in code block
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start..].find("```\n").or_else(|| response[start..].rfind("```")) {
            let json_start = start + 7; // Skip "```json"
            let json_end = start + end;
            if json_end > json_start {
                return response[json_start..json_end].trim().to_string();
            }
        }
    }

    // Try to find JSON in generic code block
    if let Some(start) = response.find("```\n") {
        if let Some(end) = response[start + 4..].find("```") {
            let json_start = start + 4;
            let json_end = start + 4 + end;
            return response[json_start..json_end].trim().to_string();
        }
    }

    // Try to find JSON object directly
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return response[start..=end].to_string();
            }
        }
    }

    response.to_string()
}
