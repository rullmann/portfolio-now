//! FIFO (First-In-First-Out) cost basis calculation
//!
//! Implements Portfolio Performance's FIFO method exactly:
//! - BUY/DELIVERY_INBOUND: Create new lots
//! - SELL/DELIVERY_OUTBOUND: Consume lots in FIFO order
//! - TRANSFER_IN: Move lots from source portfolio (via cross-entry)
//! - TRANSFER_OUT: Ignored (handled by TRANSFER_IN)
//!
//! Based on: https://github.com/portfolio-performance/portfolio
//! See: TradeCollector.java, CostCalculation.java

use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Scale factors matching Portfolio Performance
pub const SHARES_SCALE: i64 = 100_000_000; // 10^8
pub const AMOUNT_SCALE: i64 = 100;         // 10^2 (cents)

/// A FIFO lot representing a purchase
#[derive(Debug, Clone)]
pub struct FifoLot {
    pub id: i64,
    pub security_id: i64,
    pub portfolio_id: i64,
    pub purchase_txn_id: i64,
    pub purchase_date: String,
    pub original_shares: i64,
    pub remaining_shares: i64,
    pub gross_amount: i64,
    pub net_amount: i64,
    pub currency: String,
}

impl FifoLot {
    /// Calculate remaining cost basis proportionally
    /// Uses gross_amount (INCLUDING fees and taxes) per PP convention for Purchase Value
    pub fn remaining_cost_basis(&self) -> i64 {
        if self.original_shares == 0 {
            return 0;
        }
        ((self.remaining_shares as i128 * self.gross_amount as i128) /
         self.original_shares as i128) as i64
    }
}

/// Record of a lot consumption (for tracking realized gains)
#[derive(Debug, Clone)]
pub struct FifoConsumption {
    pub lot_id: i64,
    pub sale_txn_id: i64,
    pub shares_consumed: i64,
    pub gross_amount: i64, // Proportional cost basis (with fees/taxes)
    pub net_amount: i64,   // Proportional cost basis (without fees/taxes)
}

/// Transaction data for FIFO processing
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TxnData {
    id: i64,
    uuid: String,
    portfolio_id: i64,
    txn_type: String,
    date: String,
    amount: i64,
    currency: String,
    shares: i64,
    fees: i64,
    taxes: i64,
    cross_entry_id: Option<i64>,
}

/// Build FIFO lots from all portfolio transactions for a security
/// This implements Portfolio Performance's TradeCollector logic
pub fn build_fifo_lots(conn: &Connection, security_id: i64) -> Result<()> {
    // Clear existing lots for this security
    conn.execute(
        "DELETE FROM pp_fifo_consumption WHERE lot_id IN (
            SELECT id FROM pp_fifo_lot WHERE security_id = ?
        )",
        [security_id],
    )?;
    conn.execute(
        "DELETE FROM pp_fifo_lot WHERE security_id = ?",
        [security_id],
    )?;

    // Get all portfolio transactions for this security, sorted by PP rules:
    // 1. By date
    // 2. Same date: BUY/DELIVERY_INBOUND first, then TRANSFER, then SELL/DELIVERY_OUTBOUND
    let mut stmt = conn.prepare(r#"
        SELECT
            t.id, t.uuid, t.owner_id, t.txn_type, t.date,
            t.amount, t.currency, t.shares, t.cross_entry_id,
            COALESCE(SUM(CASE WHEN u.unit_type = 'FEE' THEN u.amount ELSE 0 END), 0) as fees,
            COALESCE(SUM(CASE WHEN u.unit_type = 'TAX' THEN u.amount ELSE 0 END), 0) as taxes
        FROM pp_txn t
        LEFT JOIN pp_txn_unit u ON u.txn_id = t.id
        WHERE t.security_id = ? AND t.owner_type = 'portfolio' AND t.shares IS NOT NULL
        GROUP BY t.id
        ORDER BY
            date(t.date),
            CASE t.txn_type
                WHEN 'BUY' THEN 1
                WHEN 'DELIVERY_INBOUND' THEN 1
                WHEN 'TRANSFER_IN' THEN 2
                WHEN 'TRANSFER_OUT' THEN 3
                WHEN 'SELL' THEN 4
                WHEN 'DELIVERY_OUTBOUND' THEN 4
                ELSE 5
            END,
            t.id
    "#)?;

    let transactions: Vec<TxnData> = stmt
        .query_map([security_id], |row| {
            Ok(TxnData {
                id: row.get(0)?,
                uuid: row.get(1)?,
                portfolio_id: row.get(2)?,
                txn_type: row.get(3)?,
                date: row.get(4)?,
                amount: row.get(5)?,
                currency: row.get(6)?,
                shares: row.get(7)?,
                cross_entry_id: row.get(8)?,
                fees: row.get(9)?,
                taxes: row.get(10)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Build cross-entry map to find source portfolio for transfers
    let cross_entry_map = build_cross_entry_map(conn)?;

    // FIFO lots per portfolio: portfolio_id -> Vec<FifoLot>
    let mut lots_by_portfolio: HashMap<i64, Vec<FifoLot>> = HashMap::new();
    let mut consumptions: Vec<FifoConsumption> = Vec::new();
    let mut next_lot_id: i64 = 1;

    for txn in transactions {
        match txn.txn_type.as_str() {
            "BUY" | "DELIVERY_INBOUND" => {
                // Create new lot
                // PP CostCalculation.java: gross_amount INCLUDES fees and taxes (= Purchase Value)
                // net_amount EXCLUDES fees and taxes
                let gross_amount = txn.amount;
                let net_amount = txn.amount - txn.fees - txn.taxes;

                let lot = FifoLot {
                    id: next_lot_id,
                    security_id,
                    portfolio_id: txn.portfolio_id,
                    purchase_txn_id: txn.id,
                    purchase_date: txn.date,
                    original_shares: txn.shares,
                    remaining_shares: txn.shares,
                    gross_amount,
                    net_amount,
                    currency: txn.currency,
                };
                next_lot_id += 1;

                lots_by_portfolio
                    .entry(txn.portfolio_id)
                    .or_default()
                    .push(lot);
            }

            "SELL" | "DELIVERY_OUTBOUND" => {
                // Consume lots in FIFO order and track consumptions
                let lots = lots_by_portfolio.entry(txn.portfolio_id).or_default();
                let mut shares_to_consume = txn.shares;

                for lot in lots.iter_mut() {
                    if shares_to_consume <= 0 {
                        break;
                    }
                    if lot.remaining_shares <= 0 {
                        continue;
                    }

                    let consumed = std::cmp::min(lot.remaining_shares, shares_to_consume);

                    // Calculate proportional cost basis for consumed shares
                    // PP CostCalculation.java: proportion = consumed / original_shares
                    let proportion = consumed as f64 / lot.original_shares as f64;
                    let consumed_gross = (lot.gross_amount as f64 * proportion).round() as i64;
                    let consumed_net = (lot.net_amount as f64 * proportion).round() as i64;

                    // Record the consumption for realized gains tracking
                    consumptions.push(FifoConsumption {
                        lot_id: lot.id,
                        sale_txn_id: txn.id,
                        shares_consumed: consumed,
                        gross_amount: consumed_gross,
                        net_amount: consumed_net,
                    });

                    lot.remaining_shares -= consumed;
                    shares_to_consume -= consumed;
                }

                if shares_to_consume > 0 {
                    log::warn!(
                        "FIFO: Could not consume all shares for txn {}: {} remaining",
                        txn.id, shares_to_consume
                    );
                }
            }

            "TRANSFER_IN" => {
                // Find source portfolio via cross-entry and move lots
                if let Some(cross_entry_id) = txn.cross_entry_id {
                    if let Some(source_portfolio_id) = cross_entry_map.get(&cross_entry_id) {
                        // Move lots from source to destination
                        move_lots_between_portfolios(
                            &mut lots_by_portfolio,
                            *source_portfolio_id,
                            txn.portfolio_id,
                            txn.shares,
                            &mut next_lot_id,
                            security_id,
                            &txn,
                        );
                    } else {
                        // No cross-entry found - create new lot as fallback
                        log::warn!("TRANSFER_IN without valid cross-entry, creating new lot");
                        create_lot_for_transfer(&mut lots_by_portfolio, &txn, security_id, &mut next_lot_id);
                    }
                } else {
                    // No cross-entry - create new lot
                    log::warn!("TRANSFER_IN without cross_entry_id, creating new lot");
                    create_lot_for_transfer(&mut lots_by_portfolio, &txn, security_id, &mut next_lot_id);
                }
            }

            "TRANSFER_OUT" => {
                // Ignored - handled by TRANSFER_IN
            }

            _ => {
                log::warn!("Unknown transaction type: {}", txn.txn_type);
            }
        }
    }

    // Insert all lots into database and track the mapping from temp ID to actual DB ID
    let mut lot_id_map: HashMap<i64, i64> = HashMap::new();

    for (_portfolio_id, lots) in lots_by_portfolio {
        for lot in lots {
            if lot.remaining_shares > 0 || lot.original_shares > 0 {
                conn.execute(
                    r#"INSERT INTO pp_fifo_lot
                       (security_id, portfolio_id, purchase_txn_id, purchase_date,
                        original_shares, remaining_shares, gross_amount, net_amount, currency)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
                    params![
                        lot.security_id, lot.portfolio_id, lot.purchase_txn_id, lot.purchase_date,
                        lot.original_shares, lot.remaining_shares, lot.gross_amount, lot.net_amount, lot.currency
                    ],
                )?;

                // Map the temporary lot ID to the actual database ID
                let db_lot_id = conn.last_insert_rowid();
                lot_id_map.insert(lot.id, db_lot_id);
            }
        }
    }

    // Insert all consumption records
    for consumption in consumptions {
        // Look up the actual database lot ID
        if let Some(&db_lot_id) = lot_id_map.get(&consumption.lot_id) {
            conn.execute(
                r#"INSERT INTO pp_fifo_consumption
                   (lot_id, sale_txn_id, shares_consumed, gross_amount, net_amount)
                   VALUES (?, ?, ?, ?, ?)"#,
                params![
                    db_lot_id,
                    consumption.sale_txn_id,
                    consumption.shares_consumed,
                    consumption.gross_amount,
                    consumption.net_amount
                ],
            )?;
        } else {
            log::warn!(
                "FIFO: Consumption references unknown lot_id {} for sale_txn {}",
                consumption.lot_id, consumption.sale_txn_id
            );
        }
    }

    Ok(())
}

/// Build a map of cross_entry_id -> source_portfolio_id for transfers
fn build_cross_entry_map(conn: &Connection) -> Result<HashMap<i64, i64>> {
    let mut map = HashMap::new();

    // For PORTFOLIO_TRANSFER entries, from_txn_id points to TRANSFER_OUT
    // We need to find the portfolio of from_txn_id
    let mut stmt = conn.prepare(r#"
        SELECT ce.id, t.owner_id
        FROM pp_cross_entry ce
        JOIN pp_txn t ON t.id = ce.from_txn_id
        WHERE ce.entry_type = 'PORTFOLIO_TRANSFER'
    "#)?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;

    for row in rows {
        if let Ok((cross_entry_id, source_portfolio_id)) = row {
            map.insert(cross_entry_id, source_portfolio_id);
        }
    }

    Ok(map)
}

/// Move lots from source portfolio to destination portfolio
fn move_lots_between_portfolios(
    lots_by_portfolio: &mut HashMap<i64, Vec<FifoLot>>,
    source_portfolio_id: i64,
    dest_portfolio_id: i64,
    shares_to_move: i64,
    next_lot_id: &mut i64,
    security_id: i64,
    txn: &TxnData,
) {
    let source_lots = lots_by_portfolio.entry(source_portfolio_id).or_default();
    let mut shares_remaining = shares_to_move;
    let mut lots_to_add: Vec<FifoLot> = Vec::new();

    // Move lots in FIFO order
    for lot in source_lots.iter_mut() {
        if shares_remaining <= 0 {
            break;
        }
        if lot.remaining_shares <= 0 {
            continue;
        }

        let shares_from_lot = std::cmp::min(lot.remaining_shares, shares_remaining);

        // Calculate proportional cost for moved shares
        let proportion = shares_from_lot as f64 / lot.original_shares as f64;
        let moved_gross = (lot.gross_amount as f64 * proportion) as i64;
        let moved_net = (lot.net_amount as f64 * proportion) as i64;

        // Create new lot in destination portfolio
        let new_lot = FifoLot {
            id: *next_lot_id,
            security_id,
            portfolio_id: dest_portfolio_id,
            purchase_txn_id: lot.purchase_txn_id, // Keep original purchase txn
            purchase_date: lot.purchase_date.clone(),
            original_shares: shares_from_lot,
            remaining_shares: shares_from_lot,
            gross_amount: moved_gross,
            net_amount: moved_net,
            currency: lot.currency.clone(),
        };
        *next_lot_id += 1;
        lots_to_add.push(new_lot);

        // Reduce source lot
        lot.remaining_shares -= shares_from_lot;
        shares_remaining -= shares_from_lot;
    }

    if shares_remaining > 0 {
        log::warn!(
            "TRANSFER_IN: Could not find enough lots to transfer {} shares, {} remaining",
            shares_to_move, shares_remaining
        );
        // Create a new lot for the remaining shares as fallback
        let new_lot = FifoLot {
            id: *next_lot_id,
            security_id,
            portfolio_id: dest_portfolio_id,
            purchase_txn_id: txn.id,
            purchase_date: txn.date.clone(),
            original_shares: shares_remaining,
            remaining_shares: shares_remaining,
            gross_amount: 0, // Unknown cost
            net_amount: 0,
            currency: txn.currency.clone(),
        };
        *next_lot_id += 1;
        lots_to_add.push(new_lot);
    }

    // Add moved lots to destination
    lots_by_portfolio
        .entry(dest_portfolio_id)
        .or_default()
        .extend(lots_to_add);
}

/// Create a lot for a transfer when no cross-entry exists
fn create_lot_for_transfer(
    lots_by_portfolio: &mut HashMap<i64, Vec<FifoLot>>,
    txn: &TxnData,
    security_id: i64,
    next_lot_id: &mut i64,
) {
    // PP CostCalculation.java: gross_amount INCLUDES fees and taxes (= Purchase Value)
    // net_amount EXCLUDES fees and taxes
    let lot = FifoLot {
        id: *next_lot_id,
        security_id,
        portfolio_id: txn.portfolio_id,
        purchase_txn_id: txn.id,
        purchase_date: txn.date.clone(),
        original_shares: txn.shares,
        remaining_shares: txn.shares,
        gross_amount: txn.amount,
        net_amount: txn.amount - txn.fees - txn.taxes,
        currency: txn.currency.clone(),
    };
    *next_lot_id += 1;

    lots_by_portfolio
        .entry(txn.portfolio_id)
        .or_default()
        .push(lot);
}

/// Build FIFO lots for all securities in the database
pub fn build_all_fifo_lots(conn: &Connection) -> Result<()> {
    let security_ids: Vec<i64> = conn
        .prepare("SELECT DISTINCT id FROM pp_security")?
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for security_id in security_ids {
        if let Err(e) = build_fifo_lots(conn, security_id) {
            log::error!("Failed to build FIFO lots for security {}: {}", security_id, e);
        }
    }

    Ok(())
}

/// Get FIFO cost basis for a security (aggregated across all portfolios)
/// Returns (total_remaining_shares, total_cost_basis)
/// Cost basis uses gross_amount (INCLUDING fees and taxes) per PP convention
pub fn get_fifo_cost_basis(conn: &Connection, security_id: i64) -> Result<(i64, i64)> {
    let mut stmt = conn.prepare(r#"
        SELECT
            COALESCE(SUM(remaining_shares), 0),
            COALESCE(SUM(
                CASE WHEN original_shares > 0 THEN
                    (remaining_shares * gross_amount / original_shares)
                ELSE 0 END
            ), 0)
        FROM pp_fifo_lot
        WHERE security_id = ? AND remaining_shares > 0
    "#)?;

    let result: (i64, i64) = stmt.query_row([security_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remaining_cost_basis() {
        let lot = FifoLot {
            id: 1,
            security_id: 1,
            portfolio_id: 1,
            purchase_txn_id: 1,
            purchase_date: "2024-01-01".to_string(),
            original_shares: 100 * SHARES_SCALE,
            remaining_shares: 50 * SHARES_SCALE,
            gross_amount: 1000 * AMOUNT_SCALE,
            net_amount: 1000 * AMOUNT_SCALE,
            currency: "EUR".to_string(),
        };

        let cost_basis = lot.remaining_cost_basis();
        assert_eq!(cost_basis, 500 * AMOUNT_SCALE);
    }
}
