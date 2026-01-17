//! PDF Import Commands
//!
//! Tauri commands for importing bank statements from PDF files.

use crate::db;
use crate::events::{emit_data_changed, DataChangedPayload};
use crate::pdf_import::{
    extract_pdf_text, parse_pdf, parse_pdf_content, ParsedTransaction, ParsedTransactionType,
    ParseResult,
};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle};

/// Format date with optional time for database storage
fn format_datetime(txn: &ParsedTransaction) -> String {
    if let Some(time) = txn.time {
        format!("{} {}", txn.date, time)
    } else {
        format!("{}", txn.date)
    }
}

/// Get possible DB transaction types for duplicate detection.
/// Returns all types that could match (original + delivery mode variant).
/// This is needed because a PDF "Buy" could have been imported as "DELIVERY_INBOUND"
/// if deliveryMode was active during the original import.
fn get_duplicate_check_types(txn_type: ParsedTransactionType) -> Vec<&'static str> {
    match txn_type {
        ParsedTransactionType::Buy => vec!["BUY", "DELIVERY_INBOUND"],
        ParsedTransactionType::Sell => vec!["SELL", "DELIVERY_OUTBOUND"],
        ParsedTransactionType::TransferIn => vec!["DELIVERY_INBOUND"],
        ParsedTransactionType::TransferOut => vec!["DELIVERY_OUTBOUND"],
        ParsedTransactionType::Dividend => vec!["DIVIDENDS"],
        ParsedTransactionType::Interest => vec!["INTEREST"],
        _ => vec![],
    }
}

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
            id: "consorsbank".to_string(),
            name: "Consorsbank".to_string(),
            description: "Consorsbank (BNP Paribas)".to_string(),
        },
        SupportedBank {
            id: "trade_republic".to_string(),
            name: "Trade Republic".to_string(),
            description: "Trade Republic Bank GmbH".to_string(),
        },
        SupportedBank {
            id: "scalable".to_string(),
            name: "Scalable Capital".to_string(),
            description: "Scalable Capital GmbH".to_string(),
        },
    ]
}

/// Preview PDF import without making changes
#[command]
pub async fn preview_pdf_import(pdf_path: String) -> Result<PdfImportPreview, String> {
    // SECURITY: Validate path (defense-in-depth)
    let validated_path = crate::security::validate_file_path_with_extension(&pdf_path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;
    let validated_path_str = validated_path.to_string_lossy().to_string();

    log::info!("PDF Import: Starting preview for {}", validated_path_str);

    // Run blocking PDF parsing in a separate thread to not block the main thread
    let result = tokio::task::spawn_blocking(move || {
        preview_pdf_import_sync(&validated_path_str)
    })
    .await
    .map_err(|e| format!("PDF preview task failed: {}", e))?;

    result
}

/// Synchronous PDF preview implementation (runs in blocking thread)
fn preview_pdf_import_sync(pdf_path: &str) -> Result<PdfImportPreview, String> {
    log::info!("PDF Import: Starting sync preview for {}", pdf_path);

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
                let amount_cents = (txn.net_amount * 100.0).round() as i64;
                // Get all possible DB types (original + delivery mode variant)
                let txn_types = get_duplicate_check_types(txn.txn_type);
                if txn_types.is_empty() {
                    continue; // Skip non-portfolio transactions (Dividend, etc.)
                }

                // Check if similar transaction exists (same security, date, type, amount within 1 cent)
                // Use LIKE for date comparison to handle seconds mismatch
                // Check for multiple type variants (BUY and DELIVERY_INBOUND, etc.)
                let date_pattern = format!("{}%", format_datetime(txn));
                let type_placeholders = txn_types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 4))
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    r#"
                    SELECT id FROM pp_txn
                    WHERE security_id = ?1
                      AND date LIKE ?2
                      AND ABS(amount - ?3) <= 1
                      AND txn_type IN ({})
                    LIMIT 1
                    "#,
                    type_placeholders
                );

                let mut params: Vec<&dyn rusqlite::ToSql> =
                    vec![&sec_id, &date_pattern, &amount_cents];
                for t in &txn_types {
                    params.push(t);
                }

                let existing: Option<i64> = conn
                    .query_row(&sql, params.as_slice(), |row| row.get(0))
                    .ok();

                if let Some(existing_id) = existing {
                    potential_duplicates.push(PotentialDuplicate {
                        transaction_index: idx,
                        existing_txn_id: existing_id,
                        date: format_datetime(txn),
                        amount: txn.net_amount,
                        security_name: txn.security_name.clone(),
                        txn_type: txn_types[0].to_string(), // Use first type for display
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
pub async fn import_pdf_transactions(
    app: AppHandle,
    pdf_path: String,
    portfolio_id: i64,
    account_id: i64,
    create_missing_securities: bool,
    skip_duplicates: Option<bool>,
    type_overrides: Option<std::collections::HashMap<usize, String>>,
    fee_overrides: Option<std::collections::HashMap<usize, f64>>,
) -> Result<PdfImportResult, String> {
    // SECURITY: Validate path (defense-in-depth)
    let validated_path = crate::security::validate_file_path_with_extension(&pdf_path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;
    let validated_path_str = validated_path.to_string_lossy().to_string();

    let skip_duplicates = skip_duplicates.unwrap_or(true);
    let type_overrides = type_overrides.unwrap_or_default();
    let fee_overrides = fee_overrides.unwrap_or_default();

    // Run blocking import in a separate thread
    let result = tokio::task::spawn_blocking(move || {
        import_pdf_transactions_sync(
            &validated_path_str,
            portfolio_id,
            account_id,
            create_missing_securities,
            skip_duplicates,
            type_overrides,
            fee_overrides,
        )
    })
    .await
    .map_err(|e| format!("PDF import task failed: {}", e))?;

    // Emit data changed event if import was successful
    if let Ok(ref import_result) = result {
        if import_result.transactions_imported > 0 {
            emit_data_changed(&app, DataChangedPayload::import(vec![]));
        }
    }

    result
}

/// Synchronous PDF import implementation (runs in blocking thread)
fn import_pdf_transactions_sync(
    pdf_path: &str,
    portfolio_id: i64,
    account_id: i64,
    create_missing_securities: bool,
    skip_duplicates: bool,
    type_overrides: std::collections::HashMap<usize, String>,
    fee_overrides: std::collections::HashMap<usize, f64>,
) -> Result<PdfImportResult, String> {
    let result = parse_pdf(pdf_path)?;

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let mut transactions_imported = 0;
    let mut transactions_skipped = 0;
    let mut securities_created = 0;
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    // Track affected securities for FIFO rebuild
    let mut affected_security_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

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
                let amount_cents = (txn.net_amount * 100.0).round() as i64;
                // Get all possible DB types (original + delivery mode variant)
                // This handles the case where the same PDF was previously imported with deliveryMode
                let txn_types = get_duplicate_check_types(effective_type);

                if !txn_types.is_empty() {
                    // Use LIKE for date comparison to handle seconds mismatch
                    // PDF might have "2026-01-07 09:30" but DB has "2026-01-07 09:30:55"
                    let date_pattern = format!("{}%", format_datetime(txn));
                    let type_placeholders = txn_types
                        .iter()
                        .enumerate()
                        .map(|(i, _)| format!("?{}", i + 4))
                        .collect::<Vec<_>>()
                        .join(", ");

                    let sql = format!(
                        r#"
                        SELECT 1 FROM pp_txn
                        WHERE security_id = ?1
                          AND date LIKE ?2
                          AND ABS(amount - ?3) <= 1
                          AND txn_type IN ({})
                        LIMIT 1
                        "#,
                        type_placeholders
                    );

                    let mut params: Vec<&dyn rusqlite::ToSql> =
                        vec![&sec_id, &date_pattern, &amount_cents];
                    for t in &txn_types {
                        params.push(t);
                    }

                    let is_duplicate: bool = conn
                        .query_row(&sql, params.as_slice(), |_| Ok(true))
                        .unwrap_or(false);

                    if is_duplicate {
                        warnings.push(format!(
                            "Transaktion vom {} übersprungen (Duplikat: {} {})",
                            txn.date,
                            txn_types[0],
                            txn.security_name.as_deref().unwrap_or("Unbekannt")
                        ));
                        transactions_skipped += 1;
                        continue;
                    }
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
        let amount_cents = (txn.net_amount * 100.0).round() as i64;
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
                    format_datetime(txn),
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

            // Track security for FIFO rebuild
            if let Some(sec_id) = security_id {
                affected_security_ids.insert(sec_id);
            }

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
                        format_datetime(txn),
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
                let fee_cents = (effective_fee * 100.0).round() as i64;
                conn.execute(
                    "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency)
                     VALUES (?1, 'FEE', ?2, ?3)",
                    rusqlite::params![portfolio_txn_id, fee_cents, txn.currency],
                ).map_err(|e| e.to_string())?;
            }

            // Add tax unit if present
            if txn.taxes > 0.0 {
                let tax_cents = (txn.taxes * 100.0).round() as i64;
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
                    format_datetime(txn),
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
                let tax_cents = (txn.taxes * 100.0).round() as i64;
                conn.execute(
                    "INSERT INTO pp_txn_unit (txn_id, unit_type, amount, currency)
                     VALUES (?1, 'TAX', ?2, ?3)",
                    rusqlite::params![txn_id, tax_cents, txn.currency],
                ).map_err(|e| e.to_string())?;
            }
        }

        transactions_imported += 1;
    }

    // Rebuild FIFO lots for all affected securities
    // WICHTIG: Ohne FIFO-Rebuild werden neue Transaktionen nicht im Einstandswert berücksichtigt!
    for sec_id in &affected_security_ids {
        if let Err(e) = crate::fifo::build_fifo_lots(conn, *sec_id) {
            log::warn!("Failed to rebuild FIFO lots for security {}: {}", sec_id, e);
        }
    }
    if !affected_security_ids.is_empty() {
        log::info!(
            "PDF Import: Rebuilt FIFO lots for {} securities",
            affected_security_ids.len()
        );
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
    // SECURITY: Validate path (defense-in-depth)
    let validated_path = crate::security::validate_file_path_with_extension(&pdf_path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;
    extract_pdf_text(&validated_path.to_string_lossy())
}

/// Parse PDF content that was already extracted
#[command]
pub fn parse_pdf_text(content: String) -> Result<ParseResult, String> {
    parse_pdf_content(&content)
}

/// Detect which bank a PDF is from
#[command]
pub fn detect_pdf_bank(pdf_path: String) -> Result<Option<String>, String> {
    // SECURITY: Validate path (defense-in-depth)
    let validated_path = crate::security::validate_file_path_with_extension(&pdf_path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;
    let content = extract_pdf_text(&validated_path.to_string_lossy())?;

    let parsers = crate::pdf_import::get_parsers();
    for parser in parsers {
        if parser.detect(&content) {
            return Ok(Some(parser.bank_name().to_string()));
        }
    }

    Ok(None)
}

// ============================================================================
// OCR Commands
// ============================================================================

/// OCR options for PDF text extraction
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrExtractRequest {
    pub pdf_path: String,
    pub provider: String,
    pub model: String,
    pub api_key: String,
}

/// OCR extraction result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrExtractResult {
    pub text: String,
    pub pages_processed: usize,
    pub provider: String,
    pub model: String,
}

/// Check if OCR tools (poppler-utils) are available
#[command]
pub fn is_ocr_available() -> bool {
    crate::pdf_import::ocr::is_pdftoppm_available()
}

/// Extract text from PDF using OCR (Vision API)
///
/// This is slower than regular extraction but works for scanned PDFs.
/// Requires poppler-utils (pdftoppm) to be installed.
#[command]
pub async fn extract_pdf_with_ocr(request: OcrExtractRequest) -> Result<OcrExtractResult, String> {
    use crate::pdf_import::ocr::{ocr_pdf, OcrOptions};

    // SECURITY: Validate path (defense-in-depth)
    let validated_path = crate::security::validate_file_path_with_extension(&request.pdf_path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;
    let validated_path_str = validated_path.to_string_lossy().to_string();

    let options = OcrOptions {
        provider: request.provider.clone(),
        model: request.model.clone(),
        api_key: request.api_key,
    };

    let result = ocr_pdf(&validated_path_str, options, None)
        .await
        .map_err(|e| e)?;

    Ok(OcrExtractResult {
        text: result.full_text,
        pages_processed: result.pages.len(),
        provider: result.provider,
        model: result.model,
    })
}

/// Preview PDF import with optional OCR fallback
///
/// If use_ocr is true and regular text extraction yields too little content,
/// OCR will be used as a fallback.
///
/// # Security
/// When `use_ocr` is true, the PDF content will be uploaded to an external AI service.
/// The `ocr_consent_given` flag MUST be set to true to confirm the user has consented
/// to this external data transfer. Without explicit consent, OCR will be refused.
///
/// The frontend MUST show a consent dialog informing the user:
/// - Which AI provider will receive the data (Claude, OpenAI, Gemini, Perplexity)
/// - That the PDF may contain sensitive financial information
/// - That the data will be processed by a third-party service
#[command]
pub async fn preview_pdf_import_with_ocr(
    pdf_path: String,
    use_ocr: bool,
    ocr_provider: Option<String>,
    ocr_model: Option<String>,
    ocr_api_key: Option<String>,
    ocr_consent_given: Option<bool>,
) -> Result<PdfImportPreview, String> {
    use crate::pdf_import::ocr::{ocr_pdf, should_use_ocr_fallback, OcrOptions};

    // SECURITY: Validate path (defense-in-depth)
    let validated_path = crate::security::validate_file_path_with_extension(&pdf_path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;
    let validated_path_str = validated_path.to_string_lossy().to_string();

    log::info!("PDF Import: Starting preview for {} (OCR: {})", validated_path_str, use_ocr);

    // First try regular text extraction
    let extracted_text = extract_pdf_text(&validated_path_str)?;

    // Check if we should use OCR fallback
    let content = if use_ocr && should_use_ocr_fallback(&extracted_text, 100) {
        log::info!("PDF Import: Text extraction yielded too little content, using OCR fallback");

        // SECURITY: Require explicit consent for external data upload
        if ocr_consent_given != Some(true) {
            return Err(
                "OCR erfordert explizite Zustimmung zum Upload an externe KI-Services. \
                 Das PDF enthält möglicherweise sensible Finanzdaten. \
                 Bitte bestätige im Dialog, dass du dem Upload zustimmst."
                    .to_string(),
            );
        }

        // Require OCR options
        let provider = ocr_provider.ok_or("OCR Provider ist erforderlich")?;
        let model = ocr_model.ok_or("OCR Modell ist erforderlich")?;
        let api_key = ocr_api_key.ok_or("OCR API-Key ist erforderlich")?;

        log::info!(
            "PDF Import: User consented to OCR upload to provider: {}",
            provider
        );

        let options = OcrOptions {
            provider,
            model,
            api_key,
        };

        let ocr_result = ocr_pdf(&validated_path_str, options, None).await?;
        ocr_result.full_text
    } else {
        extracted_text
    };

    // Parse the content
    let result = parse_pdf_content(&content)?;

    // Rest of the preview logic (same as preview_pdf_import)
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

    let mut potential_duplicates = Vec::new();
    for (idx, txn) in result.transactions.iter().enumerate() {
        if let Some(isin) = &txn.isin {
            let security_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM pp_security WHERE isin = ?1 LIMIT 1",
                    [isin],
                    |row| row.get(0),
                )
                .ok();

            if let Some(sec_id) = security_id {
                let amount_cents = (txn.net_amount * 100.0).round() as i64;
                // Get all possible DB types (original + delivery mode variant)
                let txn_types = get_duplicate_check_types(txn.txn_type);
                if txn_types.is_empty() {
                    continue; // Skip non-portfolio transactions (Dividend, etc.)
                }

                // Use LIKE for date comparison to handle seconds mismatch
                // Check for multiple type variants (BUY and DELIVERY_INBOUND, etc.)
                let date_pattern = format!("{}%", format_datetime(txn));
                let type_placeholders = txn_types
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 4))
                    .collect::<Vec<_>>()
                    .join(", ");

                let sql = format!(
                    r#"
                    SELECT id FROM pp_txn
                    WHERE security_id = ?1
                      AND date LIKE ?2
                      AND ABS(amount - ?3) <= 1
                      AND txn_type IN ({})
                    LIMIT 1
                    "#,
                    type_placeholders
                );

                let mut params: Vec<&dyn rusqlite::ToSql> =
                    vec![&sec_id, &date_pattern, &amount_cents];
                for t in &txn_types {
                    params.push(t);
                }

                let existing: Option<i64> = conn
                    .query_row(&sql, params.as_slice(), |row| row.get(0))
                    .ok();

                if let Some(existing_id) = existing {
                    potential_duplicates.push(PotentialDuplicate {
                        transaction_index: idx,
                        existing_txn_id: existing_id,
                        date: format_datetime(txn),
                        amount: txn.net_amount,
                        security_name: txn.security_name.clone(),
                        txn_type: txn_types[0].to_string(), // Use first type for display
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
