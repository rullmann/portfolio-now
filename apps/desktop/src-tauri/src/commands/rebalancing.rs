//! Rebalancing Tool Commands
//!
//! Calculate and execute portfolio rebalancing based on target allocations.

use crate::db;
use serde::{Deserialize, Serialize};
use tauri::command;

/// Target allocation for rebalancing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceTarget {
    pub security_id: Option<i64>,
    pub classification_id: Option<i64>,
    pub target_weight: f64,  // 0.0 - 100.0
}

/// Calculated rebalancing action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceAction {
    pub security_id: i64,
    pub security_name: String,
    pub isin: Option<String>,
    pub action: String,  // "BUY" or "SELL"
    pub shares: f64,
    pub amount: f64,  // in currency
    pub current_weight: f64,
    pub target_weight: f64,
    pub current_value: f64,
    pub target_value: f64,
}

/// Rebalancing preview result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancePreview {
    pub total_value: f64,
    pub new_cash: Option<f64>,
    pub targets: Vec<RebalanceTargetWithCurrent>,
    pub actions: Vec<RebalanceAction>,
    pub deviation_before: f64,
    pub deviation_after: f64,
}

/// Target with current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceTargetWithCurrent {
    pub security_id: Option<i64>,
    pub security_name: Option<String>,
    pub classification_id: Option<i64>,
    pub classification_name: Option<String>,
    pub target_weight: f64,
    pub current_weight: f64,
    pub current_value: f64,
    pub target_value: f64,
    pub difference: f64,
}

/// Rebalancing execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceResult {
    pub success: bool,
    pub transactions_created: i32,
    pub total_bought: f64,
    pub total_sold: f64,
}

/// Preview rebalancing actions
#[command]
pub fn preview_rebalance(
    portfolio_id: i64,
    targets: Vec<RebalanceTarget>,
    new_cash: Option<f64>,
) -> Result<RebalancePreview, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get current holdings with values
    let mut stmt = conn.prepare(
        "SELECT
            s.id, s.name, s.isin, s.currency,
            COALESCE(SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END), 0) / 100000000.0 as shares,
            COALESCE(lp.value, 0) / 100000000.0 as price
         FROM pp_security s
         LEFT JOIN pp_txn t ON t.security_id = s.id
            AND t.owner_type = 'portfolio'
            AND t.owner_id = ?1
            AND t.shares IS NOT NULL
         LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
         GROUP BY s.id
         HAVING shares > 0"
    ).map_err(|e| e.to_string())?;

    let holdings: Vec<(i64, String, Option<String>, String, f64, f64)> = stmt
        .query_map([portfolio_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Calculate total value
    let mut total_value: f64 = holdings.iter().map(|(_, _, _, _, shares, price)| shares * price).sum();

    // Add new cash
    if let Some(cash) = new_cash {
        total_value += cash;
    }

    if total_value <= 0.0 {
        return Err("Portfolio has no value".to_string());
    }

    // Map targets by security
    let mut target_map: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
    for target in &targets {
        if let Some(security_id) = target.security_id {
            target_map.insert(security_id, target.target_weight);
        }
    }

    // Calculate current weights and deviations
    let mut targets_with_current = Vec::new();
    let mut actions = Vec::new();
    let mut deviation_before = 0.0;
    let mut deviation_after = 0.0;

    for (security_id, name, isin, _currency, shares, price) in &holdings {
        let current_value = shares * price;
        let current_weight = (current_value / total_value) * 100.0;

        let target_weight = target_map.get(security_id).copied().unwrap_or(current_weight);
        let target_value = (target_weight / 100.0) * total_value;
        let difference = target_value - current_value;

        deviation_before += (current_weight - target_weight).abs();

        targets_with_current.push(RebalanceTargetWithCurrent {
            security_id: Some(*security_id),
            security_name: Some(name.clone()),
            classification_id: None,
            classification_name: None,
            target_weight,
            current_weight,
            current_value,
            target_value,
            difference,
        });

        // Calculate action
        if difference.abs() > 1.0 {  // Ignore tiny differences
            let shares_diff = if *price > 0.0 { difference / price } else { 0.0 };

            if difference > 0.0 {
                actions.push(RebalanceAction {
                    security_id: *security_id,
                    security_name: name.clone(),
                    isin: isin.clone(),
                    action: "BUY".to_string(),
                    shares: shares_diff,
                    amount: difference,
                    current_weight,
                    target_weight,
                    current_value,
                    target_value,
                });
            } else {
                actions.push(RebalanceAction {
                    security_id: *security_id,
                    security_name: name.clone(),
                    isin: isin.clone(),
                    action: "SELL".to_string(),
                    shares: shares_diff.abs(),
                    amount: difference.abs(),
                    current_weight,
                    target_weight,
                    current_value,
                    target_value,
                });
            }
        }

        // Calculate post-rebalance deviation (should be ~0 for matched targets)
        let new_weight = target_weight;
        deviation_after += (new_weight - target_weight).abs();
    }

    // Add targets for securities not currently held
    for target in &targets {
        if let Some(security_id) = target.security_id {
            if !holdings.iter().any(|(id, _, _, _, _, _)| *id == security_id) {
                // New security to buy
                let target_value = (target.target_weight / 100.0) * total_value;

                if target_value > 1.0 {
                    // Get security info
                    let sec_info: Result<(String, Option<String>, f64), _> = conn.query_row(
                        "SELECT s.name, s.isin, COALESCE(lp.value, 0) / 100000000.0
                         FROM pp_security s
                         LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
                         WHERE s.id = ?1",
                        [security_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    );

                    if let Ok((name, isin, price)) = sec_info {
                        let shares = if price > 0.0 { target_value / price } else { 0.0 };

                        targets_with_current.push(RebalanceTargetWithCurrent {
                            security_id: Some(security_id),
                            security_name: Some(name.clone()),
                            classification_id: None,
                            classification_name: None,
                            target_weight: target.target_weight,
                            current_weight: 0.0,
                            current_value: 0.0,
                            target_value,
                            difference: target_value,
                        });

                        actions.push(RebalanceAction {
                            security_id,
                            security_name: name,
                            isin,
                            action: "BUY".to_string(),
                            shares,
                            amount: target_value,
                            current_weight: 0.0,
                            target_weight: target.target_weight,
                            current_value: 0.0,
                            target_value,
                        });
                    }
                }
            }
        }
    }

    // Sort actions: sells first, then buys
    actions.sort_by(|a, b| {
        match (&a.action[..], &b.action[..]) {
            ("SELL", "BUY") => std::cmp::Ordering::Less,
            ("BUY", "SELL") => std::cmp::Ordering::Greater,
            _ => b.amount.partial_cmp(&a.amount).unwrap_or(std::cmp::Ordering::Equal),
        }
    });

    Ok(RebalancePreview {
        total_value,
        new_cash,
        targets: targets_with_current,
        actions,
        deviation_before,
        deviation_after,
    })
}

/// Execute rebalancing by creating transactions
#[command]
pub fn execute_rebalance(
    portfolio_id: i64,
    account_id: i64,
    actions: Vec<RebalanceAction>,
    date: Option<String>,
) -> Result<RebalanceResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let exec_date = date.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

    // Get import_id
    let import_id: i64 = conn.query_row(
        "SELECT import_id FROM pp_portfolio WHERE id = ?1",
        [portfolio_id],
        |row| row.get(0)
    ).map_err(|e| format!("Portfolio not found: {}", e))?;

    // Get account currency
    let currency: String = conn.query_row(
        "SELECT currency FROM pp_account WHERE id = ?1",
        [account_id],
        |row| row.get(0)
    ).map_err(|e| format!("Account not found: {}", e))?;

    let mut transactions_created = 0;
    let mut total_bought = 0.0;
    let mut total_sold = 0.0;

    for action in &actions {
        let amount_cents = (action.amount * 100.0) as i64;
        let shares_scaled = (action.shares * 100_000_000.0) as i64;

        // Create portfolio transaction
        let txn_uuid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (?1, ?2, 'portfolio', ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Rebalancing')",
            rusqlite::params![
                import_id,
                txn_uuid,
                portfolio_id,
                action.security_id,
                action.action,
                exec_date,
                amount_cents,
                currency,
                shares_scaled,
            ],
        ).map_err(|e| e.to_string())?;

        let portfolio_txn_id = conn.last_insert_rowid();

        // Create account transaction
        let account_uuid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO pp_txn (import_id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
             VALUES (?1, ?2, 'account', ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Rebalancing')",
            rusqlite::params![
                import_id,
                account_uuid,
                account_id,
                action.security_id,
                action.action,
                exec_date,
                amount_cents,
                currency,
                shares_scaled,
            ],
        ).map_err(|e| e.to_string())?;

        let account_txn_id = conn.last_insert_rowid();

        // Create cross entry
        conn.execute(
            "INSERT INTO pp_cross_entry (entry_type, portfolio_txn_id, account_txn_id)
             VALUES ('BUY_SELL', ?1, ?2)",
            [portfolio_txn_id, account_txn_id],
        ).map_err(|e| e.to_string())?;

        transactions_created += 1;

        if action.action == "BUY" {
            total_bought += action.amount;
        } else {
            total_sold += action.amount;
        }
    }

    Ok(RebalanceResult {
        success: true,
        transactions_created,
        total_bought,
        total_sold,
    })
}

/// Calculate current deviation from targets
#[command]
pub fn calculate_deviation(
    portfolio_id: i64,
    targets: Vec<RebalanceTarget>,
) -> Result<f64, String> {
    let preview = preview_rebalance(portfolio_id, targets, None)?;
    Ok(preview.deviation_before)
}

/// Get suggested rebalance based on taxonomy
#[command]
pub fn suggest_rebalance_by_taxonomy(
    _portfolio_id: i64,
    taxonomy_id: i64,
) -> Result<Vec<RebalanceTarget>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Get classification weights
    let mut stmt = conn.prepare(
        "SELECT
            ca.vehicle_uuid,
            c.name,
            ca.weight / 100.0 as weight_percent
         FROM pp_classification_assignment ca
         JOIN pp_classification c ON c.id = ca.classification_id
         WHERE c.taxonomy_id = ?1
           AND ca.vehicle_type = 'security'"
    ).map_err(|e| e.to_string())?;

    let assignments: Vec<(String, String, f64)> = stmt
        .query_map([taxonomy_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Convert to targets
    let mut targets = Vec::new();
    for (uuid, _name, weight) in assignments {
        // Get security_id from uuid
        let security_id: Option<i64> = conn.query_row(
            "SELECT id FROM pp_security WHERE uuid = ?1",
            [&uuid],
            |row| row.get(0)
        ).ok();

        if let Some(id) = security_id {
            targets.push(RebalanceTarget {
                security_id: Some(id),
                classification_id: None,
                target_weight: weight,
            });
        }
    }

    Ok(targets)
}
