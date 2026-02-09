//! Currency conversion module
//!
//! Provides currency conversion with:
//! - Historical rate lookup from database
//! - Forward-fill for missing dates (weekend/holiday)
//! - Cross-rate triangulation through EUR
//! - Caching for performance

use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Normalize GBX/GBp to GBP (1 GBP = 100 GBX/GBp)
fn normalize_gbx(currency: &str) -> (&str, f64) {
    match currency {
        "GBX" | "GBx" | "GBp" => ("GBP", 0.01),
        _ => (currency, 1.0),
    }
}

/// Convert an amount from one currency to another on a specific date
pub fn convert(
    conn: &Connection,
    amount: f64,
    from: &str,
    to: &str,
    date: NaiveDate,
) -> Result<f64> {
    if from == to {
        return Ok(amount);
    }

    // Handle GBX/GBp → GBP (1 GBP = 100 GBX)
    let (from_norm, from_factor) = normalize_gbx(from);
    let (to_norm, to_factor) = normalize_gbx(to);

    if from_norm == to_norm {
        // e.g. GBX → GBP or GBP → GBX
        return Ok(amount * from_factor / to_factor);
    }

    let rate = get_exchange_rate(conn, from_norm, to_norm, date)?;
    Ok(amount * from_factor * rate / to_factor)
}

/// Convert an amount (in cents) from one currency to another
pub fn convert_cents(
    conn: &Connection,
    amount: i64,
    from: &str,
    to: &str,
    date: NaiveDate,
) -> Result<i64> {
    if from == to {
        return Ok(amount);
    }

    let (from_norm, from_factor) = normalize_gbx(from);
    let (to_norm, to_factor) = normalize_gbx(to);

    if from_norm == to_norm {
        return Ok((amount as f64 * from_factor / to_factor).round() as i64);
    }

    let rate = get_exchange_rate(conn, from_norm, to_norm, date)?;
    Ok((amount as f64 * from_factor * rate / to_factor).round() as i64)
}

/// Get exchange rate for a currency pair on a specific date
/// Uses forward-fill: if no rate on date, uses most recent rate before
/// Note: GBX/GBp normalization should be done by callers (convert/convert_cents)
pub fn get_exchange_rate(
    conn: &Connection,
    base: &str,
    target: &str,
    date: NaiveDate,
) -> Result<f64> {
    if base == target {
        return Ok(1.0);
    }

    // Try direct rate first
    if let Some(rate) = lookup_rate(conn, base, target, date)? {
        return Ok(rate);
    }

    // Try inverse rate
    if let Some(rate) = lookup_rate(conn, target, base, date)? {
        if rate == 0.0 {
            return Err(anyhow!("Exchange rate {}/{} is 0.0 on {} – cannot invert", target, base, date));
        }
        return Ok(1.0 / rate);
    }

    // Triangulate through EUR
    if base != "EUR" && target != "EUR" {
        let base_to_eur = get_exchange_rate(conn, base, "EUR", date)?;
        let eur_to_target = get_exchange_rate(conn, "EUR", target, date)?;
        return Ok(base_to_eur * eur_to_target);
    }

    Err(anyhow!(
        "No exchange rate found for {}/{} on {} or before",
        base, target, date
    ))
}

/// Look up rate from database with forward-fill
fn lookup_rate(
    conn: &Connection,
    base: &str,
    target: &str,
    date: NaiveDate,
) -> Result<Option<f64>> {
    // Get rate on or before the date (forward-fill)
    // Note: Database uses term_currency, rate stored as TEXT
    let sql = r#"
        SELECT rate, date FROM pp_exchange_rate
        WHERE base_currency = ?1 AND term_currency = ?2 AND date <= ?3
        ORDER BY date DESC
        LIMIT 1
    "#;

    let result: Option<(String, String)> = conn
        .query_row(sql, params![base, target, date.to_string()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .ok();

    match result {
        Some((rate_str, rate_date_str)) => {
            let rate = rate_str.parse::<f64>().ok();
            if rate.is_none() {
                log::warn!("Failed to parse exchange rate '{}' for {}/{}", rate_str, base, target);
            }
            // Reject rates older than 180 days as unreliable
            if let Ok(rate_date) = NaiveDate::parse_from_str(&rate_date_str, "%Y-%m-%d") {
                let age_days = (date - rate_date).num_days();
                if age_days > 180 {
                    log::warn!(
                        "Rejecting stale exchange rate {}/{}: {} days old (rate date: {}, requested: {}). Sync exchange rates.",
                        base, target, age_days, rate_date_str, date
                    );
                    return Ok(None);
                }
                if age_days > 30 {
                    log::warn!(
                        "Using stale exchange rate {}/{}: {} days old (rate date: {}, requested: {})",
                        base, target, age_days, rate_date_str, date
                    );
                }
            }
            Ok(rate)
        }
        None => Ok(None),
    }
}

/// Get all available exchange rates for a date (forward-filled)
pub fn get_all_rates_for_date(
    conn: &Connection,
    date: NaiveDate,
) -> Result<HashMap<(String, String), f64>> {
    let mut rates = HashMap::new();

    // Get all unique currency pairs
    let sql = r#"
        SELECT DISTINCT base_currency, term_currency FROM pp_exchange_rate
    "#;

    let pairs: Vec<(String, String)> = {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    for (base, target) in pairs {
        if let Ok(rate) = get_exchange_rate(conn, &base, &target, date) {
            rates.insert((base, target), rate);
        }
    }

    Ok(rates)
}

/// Store exchange rate in database
///
/// WICHTIG: EZB liefert nur EUR/X Kurse. Der Code invertiert automatisch bei Bedarf.
/// X/EUR Kurse sollten NICHT direkt gespeichert werden - das führt zu Berechnungsfehlern!
/// Siehe CLAUDE.md "Bekannte Fallen" #18
pub fn store_rate(
    conn: &Connection,
    base: &str,
    target: &str,
    date: NaiveDate,
    rate: f64,
) -> Result<()> {
    // Warnung bei verdächtigen inversen Kursen (X/EUR wo X != EUR)
    // Diese sollten nicht direkt gespeichert werden - der Code invertiert EUR/X automatisch
    if target == "EUR" && base != "EUR" {
        log::warn!(
            "Storing suspicious inverse rate {}/EUR = {}. \
             EZB only provides EUR/X rates. The code auto-inverts EUR/{} when needed. \
             Direct {}/EUR entries may cause calculation errors!",
            base, rate, base, base
        );
    }

    conn.execute(
        r#"
        INSERT OR REPLACE INTO pp_exchange_rate (base_currency, term_currency, date, rate)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![base, target, date.to_string(), rate.to_string()],
    )?;

    Ok(())
}

/// Get the latest available rate for a currency pair
pub fn get_latest_rate(conn: &Connection, base: &str, target: &str) -> Result<(NaiveDate, f64)> {
    if base == target {
        return Ok((chrono::Utc::now().date_naive(), 1.0));
    }

    // Try direct rate
    let sql = r#"
        SELECT date, rate FROM pp_exchange_rate
        WHERE base_currency = ?1 AND term_currency = ?2
        ORDER BY date DESC
        LIMIT 1
    "#;

    if let Ok((date_str, rate_str)) = conn.query_row(sql, params![base, target], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        if let (Ok(date), Ok(rate)) = (
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d"),
            rate_str.parse::<f64>(),
        ) {
            return Ok((date, rate));
        }
    }

    // Try inverse rate
    if let Ok((date_str, rate_str)) = conn.query_row(sql, params![target, base], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) {
        if let (Ok(date), Ok(rate)) = (
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d"),
            rate_str.parse::<f64>(),
        ) {
            if rate == 0.0 {
                return Err(anyhow!("Exchange rate {}/{} is 0.0 – cannot invert", target, base));
            }
            return Ok((date, 1.0 / rate));
        }
    }

    Err(anyhow!("No exchange rate found for {}/{}", base, target))
}

/// Convert holdings value to base currency
pub fn convert_holdings_to_base_currency(
    conn: &Connection,
    holdings: &[(i64, String, f64)], // (security_id, currency, value)
    base_currency: &str,
    date: NaiveDate,
) -> Result<f64> {
    let mut total = 0.0;

    for (_, currency, value) in holdings {
        let converted = convert(conn, *value, currency, base_currency, date)?;
        total += converted;
    }

    Ok(total)
}

/// Get base currency from client settings (stored in import)
pub fn get_base_currency(conn: &Connection) -> Result<String> {
    let sql = "SELECT base_currency FROM pp_import ORDER BY id DESC LIMIT 1";

    conn.query_row(sql, [], |row| row.get(0))
        .map_err(|_| anyhow!("No base currency configured"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_currency() {
        // Same currency should return 1.0
        assert_eq!(
            get_exchange_rate(
                &Connection::open_in_memory().unwrap(),
                "EUR",
                "EUR",
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
            )
            .unwrap(),
            1.0
        );
    }
}
