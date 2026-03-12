/// Cache management for persistent watchlist/security data.
///
/// Securities' price, earnings, and dividend data is retained even after
/// removal from watchlists. This module provides commands to inspect
/// and clean up cached data from the Settings UI.

use rusqlite::params;
use serde::Serialize;
use tauri::command;

use crate::db;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedSecurityInfo {
    pub security_id: i64,
    pub name: String,
    pub ticker: Option<String>,
    pub isin: Option<String>,
    pub is_in_watchlist: bool,
    pub is_held: bool,
    pub price_count: i64,
    pub price_date_min: Option<String>,
    pub price_date_max: Option<String>,
    pub earnings_count: i64,
    pub dividend_count: i64,
    pub has_latest_price: bool,
    pub total_data_points: i64,
    pub last_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheDeleteResult {
    pub prices_deleted: i64,
    pub latest_prices_deleted: i64,
    pub earnings_deleted: i64,
    pub dividends_deleted: i64,
    pub events_deleted: i64,
}

/// Get all securities that have any cached data, with statistics.
#[command]
pub fn get_cached_securities() -> Result<Vec<CachedSecurityInfo>, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("No database connection")?;

    // First, ensure cache meta is up to date
    rebuild_cache_meta_internal(conn).map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                m.security_id,
                m.security_name,
                m.ticker,
                m.isin,
                m.price_count,
                m.price_date_min,
                m.price_date_max,
                m.earnings_count,
                m.dividend_count,
                m.has_latest_price,
                m.last_updated_at,
                EXISTS(SELECT 1 FROM pp_watchlist_security ws WHERE ws.security_id = m.security_id) as in_watchlist,
                EXISTS(
                    SELECT 1 FROM pp_txn t
                    WHERE t.security_id = m.security_id AND t.owner_type = 'portfolio'
                    GROUP BY t.security_id
                    HAVING SUM(CASE
                        WHEN t.txn_type IN ('BUY','TRANSFER_IN','DELIVERY_INBOUND') THEN t.shares
                        WHEN t.txn_type IN ('SELL','TRANSFER_OUT','DELIVERY_OUTBOUND') THEN -t.shares
                        ELSE 0
                    END) > 0
                ) as is_held
            FROM pp_data_cache_meta m
            ORDER BY
                -- Orphaned first, then by name
                CASE WHEN EXISTS(SELECT 1 FROM pp_watchlist_security ws WHERE ws.security_id = m.security_id) THEN 1
                     WHEN EXISTS(SELECT 1 FROM pp_txn t WHERE t.security_id = m.security_id AND t.owner_type = 'portfolio') THEN 1
                     ELSE 0
                END ASC,
                m.security_name ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let results = stmt
        .query_map([], |row| {
            let price_count: i64 = row.get(4)?;
            let earnings_count: i64 = row.get(7)?;
            let dividend_count: i64 = row.get(8)?;
            let has_latest: bool = row.get(9)?;
            Ok(CachedSecurityInfo {
                security_id: row.get(0)?,
                name: row.get(1)?,
                ticker: row.get(2)?,
                isin: row.get(3)?,
                price_count,
                price_date_min: row.get(5)?,
                price_date_max: row.get(6)?,
                earnings_count,
                dividend_count,
                has_latest_price: has_latest,
                last_updated_at: row.get(10)?,
                is_in_watchlist: row.get(11)?,
                is_held: row.get(12)?,
                total_data_points: price_count + earnings_count + dividend_count + if has_latest { 1 } else { 0 },
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(results)
}

/// Delete all cached data for a single security (prices, earnings, dividends, events).
/// Does NOT delete the pp_security row itself.
#[command]
pub fn delete_cached_data_for_security(security_id: i64) -> Result<CacheDeleteResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("No database connection")?;

    // Get ticker for earnings cache (keyed by ticker)
    let ticker: Option<String> = conn
        .query_row(
            "SELECT ticker FROM pp_security WHERE id = ?1",
            params![security_id],
            |row| row.get(0),
        )
        .ok();

    let prices = conn
        .execute("DELETE FROM pp_price WHERE security_id = ?1", params![security_id])
        .unwrap_or(0) as i64;

    let latest = conn
        .execute("DELETE FROM pp_latest_price WHERE security_id = ?1", params![security_id])
        .unwrap_or(0) as i64;

    let earnings = if let Some(ref t) = ticker {
        conn.execute("DELETE FROM pp_earnings_cache WHERE ticker = ?1", params![t])
            .unwrap_or(0) as i64
    } else {
        conn.execute("DELETE FROM pp_earnings_cache WHERE security_id = ?1", params![security_id])
            .unwrap_or(0) as i64
    };

    let dividends = conn
        .execute("DELETE FROM pp_ex_dividend WHERE security_id = ?1", params![security_id])
        .unwrap_or(0) as i64;

    let events = conn
        .execute("DELETE FROM pp_security_event WHERE security_id = ?1", params![security_id])
        .unwrap_or(0) as i64;

    // Remove from cache meta
    let _ = conn.execute("DELETE FROM pp_data_cache_meta WHERE security_id = ?1", params![security_id]);

    log::info!(
        "Deleted cached data for security {}: {} prices, {} latest, {} earnings, {} dividends, {} events",
        security_id, prices, latest, earnings, dividends, events
    );

    Ok(CacheDeleteResult {
        prices_deleted: prices,
        latest_prices_deleted: latest,
        earnings_deleted: earnings,
        dividends_deleted: dividends,
        events_deleted: events,
    })
}

/// Find security IDs that have cached data but are not in any watchlist or portfolio.
fn get_orphaned_security_ids() -> anyhow::Result<Vec<i64>> {
    let conn_guard = db::get_connection()?;
    let conn = conn_guard.as_ref().ok_or(anyhow::anyhow!("No database connection"))?;

    let mut stmt = conn.prepare(
        r#"
        SELECT DISTINCT m.security_id
        FROM pp_data_cache_meta m
        WHERE NOT EXISTS (SELECT 1 FROM pp_watchlist_security ws WHERE ws.security_id = m.security_id)
          AND NOT EXISTS (
            SELECT 1 FROM pp_txn t
            WHERE t.security_id = m.security_id AND t.owner_type = 'portfolio'
            GROUP BY t.security_id
            HAVING SUM(CASE
                WHEN t.txn_type IN ('BUY','TRANSFER_IN','DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL','TRANSFER_OUT','DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END) > 0
          )
        "#,
    )?;

    let ids = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

/// Delete all cached data for securities not in any watchlist or portfolio.
#[command]
pub fn delete_all_orphaned_cache() -> Result<CacheDeleteResult, String> {
    // Collect orphan IDs first, then drop connection before deleting
    let orphan_ids = get_orphaned_security_ids().map_err(|e| e.to_string())?;

    let mut total = CacheDeleteResult {
        prices_deleted: 0,
        latest_prices_deleted: 0,
        earnings_deleted: 0,
        dividends_deleted: 0,
        events_deleted: 0,
    };

    for id in &orphan_ids {
        match delete_cached_data_for_security(*id) {
            Ok(r) => {
                total.prices_deleted += r.prices_deleted;
                total.latest_prices_deleted += r.latest_prices_deleted;
                total.earnings_deleted += r.earnings_deleted;
                total.dividends_deleted += r.dividends_deleted;
                total.events_deleted += r.events_deleted;
            }
            Err(e) => log::warn!("Failed to delete cache for security {}: {}", id, e),
        }
    }

    log::info!("Deleted orphaned cache for {} securities", orphan_ids.len());
    Ok(total)
}

/// Rebuild the pp_data_cache_meta table by scanning all data tables.
#[command]
pub fn rebuild_cache_meta() -> Result<i64, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard.as_ref().ok_or("No database connection")?;
    rebuild_cache_meta_internal(conn).map_err(|e| e.to_string())
}

/// Internal helper to rebuild cache meta (callable without Tauri command context).
fn rebuild_cache_meta_internal(conn: &rusqlite::Connection) -> anyhow::Result<i64> {
    conn.execute_batch(
        r#"
        DELETE FROM pp_data_cache_meta;
        INSERT OR REPLACE INTO pp_data_cache_meta (security_id, security_name, ticker, isin, price_count, price_date_min, price_date_max, earnings_count, dividend_count, has_latest_price, last_updated_at)
        SELECT
            s.id,
            s.name,
            s.ticker,
            s.isin,
            COALESCE((SELECT COUNT(*) FROM pp_price p WHERE p.security_id = s.id), 0),
            (SELECT MIN(date) FROM pp_price p WHERE p.security_id = s.id),
            (SELECT MAX(date) FROM pp_price p WHERE p.security_id = s.id),
            COALESCE((SELECT COUNT(*) FROM pp_earnings_cache e WHERE e.ticker = s.ticker AND s.ticker IS NOT NULL), 0),
            COALESCE((SELECT COUNT(*) FROM pp_ex_dividend d WHERE d.security_id = s.id), 0),
            CASE WHEN EXISTS(SELECT 1 FROM pp_latest_price lp WHERE lp.security_id = s.id) THEN 1 ELSE 0 END,
            CURRENT_TIMESTAMP
        FROM pp_security s
        WHERE EXISTS(SELECT 1 FROM pp_price p WHERE p.security_id = s.id)
           OR EXISTS(SELECT 1 FROM pp_latest_price lp WHERE lp.security_id = s.id)
           OR EXISTS(SELECT 1 FROM pp_earnings_cache e WHERE e.ticker = s.ticker AND s.ticker IS NOT NULL)
           OR EXISTS(SELECT 1 FROM pp_ex_dividend d WHERE d.security_id = s.id);
        "#,
    )?;

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM pp_data_cache_meta", [], |row| row.get(0))?;
    log::info!("Rebuilt cache meta: {} securities with cached data", count);
    Ok(count)
}

/// Update cache meta for a single security after data changes.
pub fn update_cache_meta(conn: &rusqlite::Connection, security_id: i64) -> anyhow::Result<()> {
    conn.execute(
        r#"
        INSERT OR REPLACE INTO pp_data_cache_meta (security_id, security_name, ticker, isin, price_count, price_date_min, price_date_max, earnings_count, dividend_count, has_latest_price, last_updated_at)
        SELECT
            s.id,
            s.name,
            s.ticker,
            s.isin,
            COALESCE((SELECT COUNT(*) FROM pp_price p WHERE p.security_id = s.id), 0),
            (SELECT MIN(date) FROM pp_price p WHERE p.security_id = s.id),
            (SELECT MAX(date) FROM pp_price p WHERE p.security_id = s.id),
            COALESCE((SELECT COUNT(*) FROM pp_earnings_cache e WHERE e.ticker = s.ticker AND s.ticker IS NOT NULL), 0),
            COALESCE((SELECT COUNT(*) FROM pp_ex_dividend d WHERE d.security_id = s.id), 0),
            CASE WHEN EXISTS(SELECT 1 FROM pp_latest_price lp WHERE lp.security_id = s.id) THEN 1 ELSE 0 END,
            CURRENT_TIMESTAMP
        FROM pp_security s
        WHERE s.id = ?1
        "#,
        params![security_id],
    )?;
    Ok(())
}
