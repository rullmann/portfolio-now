//! Corporate Actions commands for Tauri
//!
//! Handles stock splits, mergers, spin-offs, and other corporate events
//! that affect share counts and cost basis.

use crate::db;
use crate::pp::common::shares;
use serde::{Deserialize, Serialize};
use tauri::command;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CorporateActionType {
    /// Stock split (e.g., 2:1 means 2 new shares for each old share)
    StockSplit,
    /// Reverse stock split (e.g., 1:4 means 1 new share for 4 old shares)
    ReverseSplit,
    /// Spin-off: new shares in a different company
    SpinOff,
    /// Merger: shares converted to shares in acquiring company
    Merger,
    /// Stock dividend (bonus shares)
    StockDividend,
    /// Rights issue
    RightsIssue,
    /// Name/ticker change
    SymbolChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorporateAction {
    pub id: i64,
    pub security_id: i64,
    pub security_name: String,
    pub action_type: CorporateActionType,
    pub effective_date: String,
    /// For splits: numerator (e.g., 3 in 3:1 split)
    pub ratio_from: i32,
    /// For splits: denominator (e.g., 1 in 3:1 split)
    pub ratio_to: i32,
    /// For spin-offs/mergers: target security ID
    pub target_security_id: Option<i64>,
    /// For spin-offs/mergers: target security name
    pub target_security_name: Option<String>,
    /// Additional notes
    pub note: Option<String>,
    /// Whether the action has been applied
    pub is_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyStockSplitRequest {
    pub security_id: i64,
    pub effective_date: String,
    /// New shares per old share (e.g., 3 for 3:1 split)
    pub ratio_from: i32,
    /// Old shares (usually 1)
    pub ratio_to: i32,
    /// Whether to adjust historical prices
    pub adjust_prices: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplySpinOffRequest {
    pub source_security_id: i64,
    pub target_security_id: i64,
    pub effective_date: String,
    /// Cost basis allocation to new security (0.0 - 1.0)
    pub cost_allocation: f64,
    /// Shares of new security per source share
    pub share_ratio: f64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockSplitPreview {
    pub security_name: String,
    pub effective_date: String,
    pub ratio_display: String,
    /// Portfolios affected
    pub affected_portfolios: Vec<AffectedPortfolio>,
    /// Total shares before
    pub total_shares_before: f64,
    /// Total shares after
    pub total_shares_after: f64,
    /// Number of FIFO lots to adjust
    pub fifo_lots_count: i64,
    /// Number of prices to adjust (if adjust_prices is true)
    pub prices_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedPortfolio {
    pub portfolio_id: i64,
    pub portfolio_name: String,
    pub shares_before: f64,
    pub shares_after: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorporateActionResult {
    pub success: bool,
    pub message: String,
    /// Number of transactions adjusted
    pub transactions_adjusted: i64,
    /// Number of FIFO lots adjusted
    pub fifo_lots_adjusted: i64,
    /// Number of prices adjusted
    pub prices_adjusted: i64,
}

// ============================================================================
// Commands
// ============================================================================

/// Preview the effect of a stock split
#[command]
pub fn preview_stock_split(
    security_id: i64,
    effective_date: String,
    ratio_from: i32,
    ratio_to: i32,
) -> Result<StockSplitPreview, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get security name
    let security_name: String = conn
        .query_row(
            "SELECT name FROM pp_security WHERE id = ?",
            [security_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let ratio = ratio_from as f64 / ratio_to as f64;
    let ratio_display = format!("{}:{}", ratio_from, ratio_to);

    // Get affected portfolios
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                p.id, p.name,
                SUM(CASE
                    WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                    WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                    ELSE 0
                END) as net_shares
            FROM pp_txn t
            JOIN pp_portfolio p ON p.id = t.owner_id
            WHERE t.security_id = ? AND t.owner_type = 'portfolio' AND t.date <= ?
            GROUP BY p.id
            HAVING net_shares > 0
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(rusqlite::params![security_id, effective_date], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
        })
        .map_err(|e| e.to_string())?;

    let mut affected_portfolios = Vec::new();
    let mut total_shares_before = 0.0;

    for row in rows.flatten() {
        let (portfolio_id, portfolio_name, shares_raw) = row;
        let shares_before = shares::to_decimal(shares_raw);
        let shares_after = shares_before * ratio;
        total_shares_before += shares_before;

        affected_portfolios.push(AffectedPortfolio {
            portfolio_id,
            portfolio_name,
            shares_before,
            shares_after,
        });
    }

    let total_shares_after = total_shares_before * ratio;

    // Count FIFO lots
    let fifo_lots_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_fifo_lot WHERE security_id = ? AND shares > 0",
            [security_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Count historical prices before effective date
    let prices_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pp_price WHERE security_id = ? AND date < ?",
            rusqlite::params![security_id, effective_date],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(StockSplitPreview {
        security_name,
        effective_date,
        ratio_display,
        affected_portfolios,
        total_shares_before,
        total_shares_after,
        fifo_lots_count,
        prices_count,
    })
}

/// Apply a stock split
#[command]
pub fn apply_stock_split(request: ApplyStockSplitRequest) -> Result<CorporateActionResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let ratio = request.ratio_from as f64 / request.ratio_to as f64;
    let inverse_ratio = request.ratio_to as f64 / request.ratio_from as f64;

    let mut transactions_adjusted = 0i64;
    let mut fifo_lots_adjusted = 0i64;
    let mut prices_adjusted = 0i64;

    // 1. Adjust shares in transactions before effective date
    let result = conn.execute(
        r#"
        UPDATE pp_txn
        SET shares = CAST(shares * ? AS INTEGER)
        WHERE security_id = ? AND date < ? AND shares IS NOT NULL
        "#,
        rusqlite::params![ratio, request.security_id, request.effective_date],
    );
    if let Ok(count) = result {
        transactions_adjusted = count as i64;
    }

    // 2. Adjust FIFO lots
    // Adjust remaining shares
    let result = conn.execute(
        r#"
        UPDATE pp_fifo_lot
        SET shares = CAST(shares * ? AS INTEGER),
            cost_per_share = CAST(cost_per_share * ? AS INTEGER)
        WHERE security_id = ? AND date < ?
        "#,
        rusqlite::params![ratio, inverse_ratio, request.security_id, request.effective_date],
    );
    if let Ok(count) = result {
        fifo_lots_adjusted = count as i64;
    }

    // 3. Adjust historical prices if requested
    if request.adjust_prices {
        let result = conn.execute(
            r#"
            UPDATE pp_price
            SET value = CAST(value * ? AS INTEGER)
            WHERE security_id = ? AND date < ?
            "#,
            rusqlite::params![inverse_ratio, request.security_id, request.effective_date],
        );
        if let Ok(count) = result {
            prices_adjusted = count as i64;
        }

        // Also adjust latest price if before effective date
        let _ = conn.execute(
            r#"
            UPDATE pp_latest_price
            SET value = CAST(value * ? AS INTEGER),
                high = CAST(high * ? AS INTEGER),
                low = CAST(low * ? AS INTEGER)
            WHERE security_id = ? AND date < ?
            "#,
            rusqlite::params![inverse_ratio, inverse_ratio, inverse_ratio, request.security_id, request.effective_date],
        );
    }

    // 4. Log the corporate action (store in security events table if exists)
    // For now, we'll just add a note to the security
    if let Some(note) = &request.note {
        let existing_note: Option<String> = conn
            .query_row(
                "SELECT note FROM pp_security WHERE id = ?",
                [request.security_id],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let new_note = match existing_note {
            Some(existing) => format!("{}\n[{}] Stock Split {}: {}", existing, request.effective_date, format!("{}:{}", request.ratio_from, request.ratio_to), note),
            None => format!("[{}] Stock Split {}: {}", request.effective_date, format!("{}:{}", request.ratio_from, request.ratio_to), note),
        };

        let _ = conn.execute(
            "UPDATE pp_security SET note = ? WHERE id = ?",
            rusqlite::params![new_note, request.security_id],
        );
    }

    Ok(CorporateActionResult {
        success: true,
        message: format!(
            "Stock split {}:{} applied successfully",
            request.ratio_from, request.ratio_to
        ),
        transactions_adjusted,
        fifo_lots_adjusted,
        prices_adjusted,
    })
}

/// Undo a stock split (reverse the adjustments)
#[command]
pub fn undo_stock_split(
    security_id: i64,
    effective_date: String,
    ratio_from: i32,
    ratio_to: i32,
    adjust_prices: bool,
) -> Result<CorporateActionResult, String> {
    // Simply apply the inverse split
    apply_stock_split(ApplyStockSplitRequest {
        security_id,
        effective_date,
        ratio_from: ratio_to,
        ratio_to: ratio_from,
        adjust_prices,
        note: Some("Undo stock split".to_string()),
    })
}

/// Apply a spin-off (create holdings in new security from existing holdings)
#[command]
pub fn apply_spin_off(request: ApplySpinOffRequest) -> Result<CorporateActionResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get current holdings in source security
    let mut stmt = conn
        .prepare(
            r#"
            SELECT owner_id, SUM(CASE
                WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
                WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
                ELSE 0
            END) as net_shares
            FROM pp_txn
            WHERE security_id = ? AND owner_type = 'portfolio' AND date <= ?
            GROUP BY owner_id
            HAVING net_shares > 0
            "#,
        )
        .map_err(|e| e.to_string())?;

    let holdings: Vec<(i64, i64)> = stmt
        .query_map(
            rusqlite::params![request.source_security_id, request.effective_date],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let import_id: i64 = conn
        .query_row("SELECT id FROM pp_import ORDER BY id DESC LIMIT 1", [], |r| r.get(0))
        .unwrap_or(1);

    let mut transactions_created = 0i64;

    for (portfolio_id, source_shares) in holdings {
        // Calculate new shares based on ratio
        let new_shares = (shares::to_decimal(source_shares) * request.share_ratio * 100_000_000.0) as i64;

        if new_shares > 0 {
            // Create DELIVERY_INBOUND transaction for new security
            let uuid = uuid::Uuid::new_v4().to_string();
            let _ = conn.execute(
                r#"
                INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
                VALUES (?, ?, 'portfolio', ?, ?, 'DELIVERY_INBOUND', ?, 0, 'EUR', ?, ?)
                "#,
                rusqlite::params![
                    import_id,
                    uuid,
                    portfolio_id,
                    request.target_security_id,
                    request.effective_date,
                    new_shares,
                    request.note.as_deref().unwrap_or("Spin-off")
                ],
            );
            transactions_created += 1;

            // Optionally adjust cost basis (create FIFO lot with allocated cost)
            if request.cost_allocation > 0.0 {
                // Get original cost basis from source security FIFO lots
                let source_cost: i64 = conn
                    .query_row(
                        "SELECT COALESCE(SUM(total_cost), 0) FROM pp_fifo_lot WHERE security_id = ? AND portfolio_id = ?",
                        rusqlite::params![request.source_security_id, portfolio_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                let allocated_cost = (source_cost as f64 * request.cost_allocation) as i64;
                let cost_per_share = if new_shares > 0 {
                    ((allocated_cost as f64 / shares::to_decimal(new_shares)) * 100_000_000.0) as i64
                } else {
                    0
                };

                let lot_uuid = uuid::Uuid::new_v4().to_string();
                let _ = conn.execute(
                    r#"
                    INSERT INTO pp_fifo_lot (security_id, portfolio_id, txn_id, date, shares, cost_per_share, total_cost)
                    VALUES (?, ?, (SELECT id FROM pp_txn WHERE uuid = ?), ?, ?, ?, ?)
                    "#,
                    rusqlite::params![
                        request.target_security_id,
                        portfolio_id,
                        lot_uuid,
                        request.effective_date,
                        new_shares,
                        cost_per_share,
                        allocated_cost
                    ],
                );

                // Reduce cost basis in source security FIFO lots proportionally
                let _ = conn.execute(
                    r#"
                    UPDATE pp_fifo_lot
                    SET total_cost = CAST(total_cost * ? AS INTEGER),
                        cost_per_share = CAST(cost_per_share * ? AS INTEGER)
                    WHERE security_id = ? AND portfolio_id = ?
                    "#,
                    rusqlite::params![
                        1.0 - request.cost_allocation,
                        1.0 - request.cost_allocation,
                        request.source_security_id,
                        portfolio_id
                    ],
                );
            }
        }
    }

    Ok(CorporateActionResult {
        success: true,
        message: "Spin-off applied successfully".to_string(),
        transactions_adjusted: transactions_created,
        fifo_lots_adjusted: transactions_created, // Same count for lots created
        prices_adjusted: 0,
    })
}

/// Get stock split history for a security
#[command]
pub fn get_split_history(security_id: i64) -> Result<Vec<CorporateAction>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Check if security_events table exists and has split data
    // For now, parse from security notes
    let _security_name: String = conn
        .query_row(
            "SELECT name FROM pp_security WHERE id = ?",
            [security_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Return empty for now - would need to implement event storage
    Ok(vec![])
}

/// Calculate the adjusted price after splits
#[command]
pub fn get_split_adjusted_price(
    _security_id: i64,
    original_price: f64,
    _original_date: String,
) -> Result<f64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let _conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // For now, just return the original price
    // Would need split history to properly adjust
    Ok(original_price)
}
