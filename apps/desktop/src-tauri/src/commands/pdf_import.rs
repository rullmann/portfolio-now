//! PDF Import Commands
//!
//! Tauri commands for importing bank statements from PDF files.

use crate::db;
use crate::pdf_import::{
    extract_pdf_text, parse_pdf, parse_pdf_content, ParsedTransaction, ParsedTransactionType,
    ParseResult,
};
use serde::{Deserialize, Serialize};
use tauri::command;

/// Preview result showing what will be imported
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfImportPreview {
    pub bank: String,
    pub transactions: Vec<ParsedTransaction>,
    pub warnings: Vec<String>,
    pub new_securities: Vec<SecurityMatch>,
    pub matched_securities: Vec<SecurityMatch>,
    pub potential_duplicates: Vec<PotentialDuplicate>,
}

/// Potential duplicate transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PotentialDuplicate {
    pub transaction_index: usize,
    pub existing_txn_id: i64,
    pub date: String,
    pub amount: f64,
    pub security_name: Option<String>,
    pub txn_type: String,
}

/// Security matching result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityMatch {
    pub isin: Option<String>,
    pub wkn: Option<String>,
    pub name: Option<String>,
    pub existing_id: Option<i64>,
    pub existing_name: Option<String>,
}

/// Import result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfImportResult {
    pub success: bool,
    pub bank: String,
    pub transactions_imported: i32,
    pub transactions_skipped: i32,
    pub securities_created: i32,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Supported banks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedBank {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// Get list of supported banks
#[command]
pub fn get_supported_banks() -> Vec<SupportedBank> {
    vec![
        SupportedBank {
            id: "dkb".to_string(),
            name: "DKB".to_string(),
            description: "Deutsche Kreditbank AG".to_string(),
        },
        SupportedBank {
            id: "ing".to_string(),
            name: "ING".to_string(),
            description: "ING-DiBa AG".to_string(),
        },
        SupportedBank {
            id: "comdirect".to_string(),
            name: "Comdirect".to_string(),
            description: "Comdirect Bank AG".to_string(),
        },
        SupportedBank {
            id: "trade_republic".to_string(),
            name: "Trade Republic".to_string(),
            description: "Trade Republic Bank GmbH".to_string(),
        },
        SupportedBank {
            id: "scalable".to_string(),
            name: "Scalable Capital".to_string(),
            description: "Scalable Capital GmbH (via Baader Bank)".to_string(),
        },
    ]
}

/// Preview PDF import without making changes
#[command]
pub fn preview_pdf_import(pdf_path: String) -> Result<PdfImportPreview, String> {
    log::info!("PDF Import: Starting preview for {}", pdf_path);

    // Parse the PDF
    let result = match parse_pdf(&pdf_path) {
        Ok(r) => {
            log::info!("PDF Import: Successfully parsed PDF, found {} transactions", r.transactions.len());
            r
        }
        Err(e) => {
            log::error!("PDF Import: Failed to parse PDF: {}", e);
            return Err(e);
        }
    };

    // Check for matching securities in DB
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut new_securities = Vec::new();
    let mut matched_securities = Vec::new();
    let mut seen_isins = std::collections::HashSet::new();

    for txn in &result.transactions {
        if let Some(isin) = &txn.isin {
            if seen_isins.contains(isin) {
                continue;
            }
            seen_isins.insert(isin.clone());

            // Try to find existing security
            let existing: Option<(i64, String)> = conn
                .query_row(
                    "SELECT id, name FROM pp_security WHERE isin = ?1 LIMIT 1",
                    [isin],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .ok();

            let security_match = SecurityMatch {
                isin: Some(isin.clone()),
                wkn: txn.wkn.clone(),
                name: txn.security_name.clone(),
                existing_id: existing.as_ref().map(|(id, _)| *id),
                existing_name: existing.map(|(_, name)| name),
            };

            if security_match.existing_id.is_some() {
                matched_securities.push(security_match);
            } else {
                new_securities.push(security_match);
            }
        }
    }

    // Convert ParseWarning to strings for backward compatibility
    let warnings: Vec<String> = result.warnings.iter().map(|w| {
        format!("[{}] {}: {} (Wert: '{}')",
            match w.severity {
                crate::pdf_import::WarningSeverity::Info => "Info",
                crate::pdf_import::WarningSeverity::Warning => "Warnung",
                crate::pdf_import::WarningSeverity::Error => "Fehler",
            },
            w.field,
            w.message,
            w.raw_value
        )
    }).collect();

    // Check for potential duplicates
    let mut potential_duplicates = Vec::new();
    for (idx, txn) in result.transactions.iter().enumerate() {
        if let Some(isin) = &txn.isin {
            // Look up security ID
            let security_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM pp_security WHERE isin = ?1 LIMIT 1",
                    [isin],
                    |row| row.get(0),
                )
                .ok();

            if let Some(sec_id) = security_id {
                let amount_cents = (txn.net_amount * 100.0) as i64;
                let txn_type_str = txn.txn_type.to_portfolio_type()
                    .unwrap_or_else(|| txn.txn_type.to_account_type());

                // Check if similar transaction exists (same security, date, type, amount within 1 cent)
                let existing: Option<i64> = conn
                    .query_row(
                        r#"
                        SELECT id FROM pp_txn
                        WHERE security_id = ?1
                          AND date = ?2
                          AND txn_type = ?3
                          AND ABS(amount - ?4) <= 1
                        LIMIT 1
                        "#,
                        rusqlite::params![sec_id, txn.date.to_string(), txn_type_str, amount_cents],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(existing_id) = existing {
                    potential_duplicates.push(PotentialDuplicate {
                        transaction_index: idx,
                        existing_txn_id: existing_id,
                        date: txn.date.to_string(),
                        amount: txn.net_amount,
                        security_name: txn.security_name.clone(),
                        txn_type: txn_type_str.to_string(),
                    });
                }
            }
        }
    }

    Ok(PdfImportPreview {
        bank: result.bank,
        transactions: result.transactions,
        warnings,
        new_securities,
        matched_securities,
        potential_duplicates,
    })
}

/// Import transactions from PDF
#[command]
pub fn import_pdf_transactions(
    pdf_path: String,
    portfolio_id: i64,
    account_id: i64,
    create_missing_securities: bool,
    skip_duplicates: Option<bool>,
    type_overrides: Option<std::collections::HashMap<usize, String>>,
    fee_overrides: Option<std::collections::HashMap<usize, f64>>,
) -> Result<PdfImportResult, String> {
    let skip_duplicates = skip_duplicates.unwrap_or(true);
    let type_overrides = type_overrides.unwrap_or_default();
    let fee_overrides = fee_overrides.unwrap_or_default();
    let result = parse_pdf(&pdf_path)?;

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut transactions_imported = 0;
    let mut transactions_skipped = 0;
    let mut securities_created = 0;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Get import_id from portfolio
    let import_id: i64 = conn
        .query_row(
            "SELECT import_id FROM pp_portfolio WHERE id = ?1",
            [portfolio_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Portfolio not found: {}", e))?;

    for (idx, txn) in result.transactions.iter().enumerate() {
        // Check for type override
        let effective_type = if let Some(override_type) = type_overrides.get(&idx) {
            match override_type.as_str() {
                "Buy" => ParsedTransactionType::Buy,
                "Sell" => ParsedTransactionType::Sell,
                "TransferIn" => ParsedTransactionType::TransferIn,
                "TransferOut" => ParsedTransactionType::TransferOut,
                "Dividend" => ParsedTransactionType::Dividend,
                "Interest" => ParsedTransactionType::Interest,
                "Deposit" => ParsedTransactionType::Deposit,
                "Withdrawal" => ParsedTransactionType::Withdrawal,
                "Fee" => ParsedTransactionType::Fee,
                _ => txn.txn_type,
            }
        } else {
            txn.txn_type
        };

        // Find or create security
        let security_id: Option<i64> = if let Some(isin) = &txn.isin {
            let existing: Option<i64> = conn
                .query_row(
                    "SELECT id FROM pp_security WHERE isin = ?1 LIMIT 1",
                    [isin],
                    |row| row.get(0),
                )
                .ok();

            match existing {
                Some(id) => Some(id),
                None if create_missing_securities => {
                    // Create new security
                    let uuid = uuid::Uuid::new_v4().to_string();
                    let name = txn.security_name.clone().unwrap_or_else(|| isin.clone());

                    conn.execute(
                        "INSERT INTO pp_security (import_id, uuid, name, currency, isin, wkn, is_retired)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                        rusqlite::params![
                            import_id,
                            uuid,
                            name,
                            txn.currency,
                            isin,
                            txn.wkn,
                        ],
                    ).map_err(|e| format!("Failed to create security: {}", e))?;

                    securities_created += 1;
                    Some(conn.last_insert_rowid())
                }
                None => {
                    warnings.push(format!(
                        "Security {} not found, skipping transaction",
                        isin
                    ));
                    transactions_skipped += 1;
                    continue;
                }
            }
        } else {
            None
        };

        // Check for duplicate if skip_duplicates is enabled
        if skip_duplicates {
            if let Some(sec_id) = security_id {
                let amount_cents = (txn.net_amount * 100.0) as i64;
                let txn_type_str = effective_type.to_portfolio_type()
                    .unwrap_or_else(|| effective_type.to_account_type());

                let is_duplicate: bool = conn
                    .query_row(
                        r#"
                        SELECT 1 FROM pp_txn
                        WHERE security_id = ?1
                          AND date = ?2
                          AND txn_type = ?3
                          AND ABS(amount - ?4) <= 1
                        LIMIT 1
                        "#,
                        rusqlite::params![sec_id, txn.date.to_string(), txn_type_str, amount_cents],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                if is_duplicate {
                    warnings.push(format!(
                        "Transaktion vom {} übersprungen (Duplikat: {} {})",
                        txn.date,
                        txn_type_str,
                        txn.security_name.as_deref().unwrap_or("Unbekannt")
                    ));
                    transactions_skipped += 1;
                    continue;
                }
            }
        }

        // Determine transaction category based on effective type
        let is_portfolio_txn = matches!(
            effective_type,
            ParsedTransactionType::Buy | ParsedTransactionType::Sell |
            ParsedTransactionType::TransferIn | ParsedTransactionType::TransferOut
        );
        let is_delivery = matches!(
            effective_type,
            ParsedTransactionType::TransferIn | ParsedTransactionType::TransferOut
        );

        // Create transaction
        let uuid = uuid::Uuid::new_v4().to_string();
        let amount_cents = (txn.net_amount * 100.0) as i64;
        let shares_scaled = txn.shares.map(|s| (s * 100_000_000.0) as i64);

        if is_portfolio_txn {
            // Portfolio transaction (BUY/SELL/TRANSFER_IN/TRANSFER_OUT)
            let txn_type = match effective_type {
                ParsedTransactionType::Buy => "BUY",
                ParsedTransactionType::Sell => "SELL",
                ParsedTransactionType::TransferIn => "DELIVERY_INBOUND",
                ParsedTransactionType::TransferOut => "DELIVERY_OUTBOUND",
                _ => continue,
            };

            // Insert portfolio transaction
            conn.execute(
                "INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
                 VALUES (?1, ?2, 'portfolio', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    import_id,
                    uuid,
                    portfolio_id,
                    security_id,
                    txn_type,
                    txn.date.to_string(),
                    amount_cents,
                    txn.currency,
                    shares_scaled,
                    txn.note,
                ],
            ).map_err(|e| {
                errors.push(format!("Failed to insert portfolio transaction: {}", e));
                e.to_string()
            })?;

            let portfolio_txn_id = conn.last_insert_rowid();

            // For delivery transactions (TransferIn/TransferOut), skip account transaction
            // They don't affect cash, just add/remove securities
            if !is_delivery {
                // Insert corresponding account transaction for BUY/SELL
                let account_uuid = uuid::Uuid::new_v4().to_string();
                let account_txn_type = match effective_type {
                    ParsedTransactionType::Buy => "BUY",
                    ParsedTransactionType::Sell => "SELL",
                    _ => continue,
                };

                conn.execute(
                    "INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
                     VALUES (?1, ?2, 'account', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        import_id,
                        account_uuid,
                        account_id,
                        security_id,
                        account_txn_type,
                        txn.date.to_string(),
                        amount_cents,
                        txn.currency,
                        shares_scaled,
                        txn.note,
                    ],
                ).map_err(|e| e.to_string())?;

                let account_txn_id = conn.last_insert_rowid();

                // Create cross entry
                conn.execute(
                    "INSERT INTO pp_cross_entry (entry_type, portfolio_txn_id, account_txn_id)
                     VALUES ('BUY_SELL', ?1, ?2)",
                    [portfolio_txn_id, account_txn_id],
                ).map_err(|e| e.to_string())?;
            }

            // Add fee unit if present (use override if available)
            let effective_fee = fee_overrides.get(&idx).copied().unwrap_or(txn.fees);
            if effective_fee > 0.0 {
                let fee_cents = (effective_fee * 100.0) as i64;
                conn.execute(
                    "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency)
                     VALUES (?1, 'FEE', ?2, ?3)",
                    rusqlite::params![portfolio_txn_id, fee_cents, txn.currency],
                ).map_err(|e| e.to_string())?;
            }

            // Add tax unit if present
            if txn.taxes > 0.0 {
                let tax_cents = (txn.taxes * 100.0) as i64;
                conn.execute(
                    "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency)
                     VALUES (?1, 'TAX', ?2, ?3)",
                    rusqlite::params![portfolio_txn_id, tax_cents, txn.currency],
                ).map_err(|e| e.to_string())?;
            }
        } else {
            // Account-only transaction (DIVIDEND, INTEREST, etc.)
            let txn_type = effective_type.to_account_type();

            conn.execute(
                "INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
                 VALUES (?1, ?2, 'account', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    import_id,
                    uuid,
                    account_id,
                    security_id,
                    txn_type,
                    txn.date.to_string(),
                    amount_cents,
                    txn.currency,
                    shares_scaled,
                    txn.note,
                ],
            ).map_err(|e| {
                errors.push(format!("Failed to insert account transaction: {}", e));
                e.to_string()
            })?;

            let txn_id = conn.last_insert_rowid();

            // Add tax unit for dividends
            if txn.taxes > 0.0 {
                let tax_cents = (txn.taxes * 100.0) as i64;
                conn.execute(
                    "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency)
                     VALUES (?1, 'TAX', ?2, ?3)",
                    rusqlite::params![txn_id, tax_cents, txn.currency],
                ).map_err(|e| e.to_string())?;
            }
        }

        transactions_imported += 1;
    }

    Ok(PdfImportResult {
        success: errors.is_empty(),
        bank: result.bank,
        transactions_imported,
        transactions_skipped,
        securities_created,
        errors,
        warnings,
    })
}

/// Extract raw text from PDF for debugging/custom parsing
#[command]
pub fn extract_pdf_raw_text(pdf_path: String) -> Result<String, String> {
    extract_pdf_text(&pdf_path)
}

/// Parse PDF content that was already extracted
#[command]
pub fn parse_pdf_text(content: String) -> Result<ParseResult, String> {
    parse_pdf_content(&content)
}

/// Detect which bank a PDF is from
#[command]
pub fn detect_pdf_bank(pdf_path: String) -> Result<Option<String>, String> {
    let content = extract_pdf_text(&pdf_path)?;

    let parsers = crate::pdf_import::get_parsers();
    for parser in parsers {
        if parser.detect(&content) {
            return Ok(Some(parser.bank_name().to_string()));
        }
    }

    Ok(None)
}
