//! German tax calculation module
//!
//! Implements German capital gains tax (Abgeltungssteuer) calculations:
//! - Base rate: 25% + 5.5% Solidaritätszuschlag = 26.375%
//! - With Kirchensteuer (8%): 24.51% + Soli + KiSt
//! - With Kirchensteuer (9%): 24.45% + Soli + KiSt
//!
//! Freistellungsauftrag (tax-free allowance):
//! - Since 2023: 1000€ (single) / 2000€ (married)
//! - Before 2023: 801€ (single) / 1602€ (married)

use crate::db;
use serde::{Deserialize, Serialize};
use tauri::command;

// ============================================================================
// Constants
// ============================================================================

/// Base Abgeltungssteuer rate
const ABGELTUNGSSTEUER_RATE: f64 = 0.25;

/// Solidaritätszuschlag rate (on Abgeltungssteuer)
const SOLI_RATE: f64 = 0.055;

/// Maximum creditable foreign withholding tax rate
const MAX_CREDITABLE_WHT: f64 = 0.15;

/// Freistellungsauftrag limits by year
fn get_freistellung_limit(year: i32, married: bool) -> f64 {
    let single = if year >= 2023 { 1000.0 } else { 801.0 };
    if married { single * 2.0 } else { single }
}

// ============================================================================
// Types
// ============================================================================

/// Tax settings for a year
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxSettings {
    pub year: i32,
    pub is_married: bool,
    pub kirchensteuer_rate: Option<f64>, // 0.08 or 0.09 (Bayern/BW) or None
    pub bundesland: Option<String>,
    pub freistellung_limit: f64,
    pub freistellung_used: f64,
}

/// Detailed tax calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GermanTaxReport {
    pub year: i32,
    pub currency: String,
    pub settings: TaxSettings,

    // Income
    pub dividend_income_gross: f64,
    pub interest_income_gross: f64,
    pub realized_gains: f64,
    pub realized_losses: f64,
    pub total_taxable_income: f64,

    // Deductions
    pub freistellung_available: f64,
    pub freistellung_used: f64,
    pub loss_carryforward: f64,

    // After deductions
    pub taxable_after_deductions: f64,

    // Foreign taxes
    pub foreign_withholding_tax: f64,
    pub creditable_foreign_tax: f64,

    // German tax calculation
    pub abgeltungssteuer: f64,
    pub solidaritaetszuschlag: f64,
    pub kirchensteuer: f64,
    pub total_german_tax: f64,

    // Already paid
    pub tax_already_paid: f64,
    pub remaining_tax_liability: f64,

    // Breakdown by category
    pub dividend_details: Vec<TaxableItem>,
    pub gains_details: Vec<TaxableItem>,
    pub losses_details: Vec<TaxableItem>,

    // Anlage KAP data
    pub anlage_kap: AnlageKapData,
}

/// Individual taxable item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxableItem {
    pub date: String,
    pub security_name: String,
    pub security_isin: Option<String>,
    pub gross_amount: f64,
    pub withholding_tax: f64,
    pub net_amount: f64,
    pub item_type: String, // DIVIDEND, INTEREST, GAIN, LOSS
}

/// Data for German tax form "Anlage KAP"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnlageKapData {
    /// Line 7: Kapitalerträge inländischer Dividenden
    pub zeile_7_inland_dividenden: f64,
    /// Line 8: Kapitalerträge ausländischer Dividenden
    pub zeile_8_ausland_dividenden: f64,
    /// Line 14: Zinsen
    pub zeile_14_zinsen: f64,
    /// Line 15: Gewinne aus Veräußerungen
    pub zeile_15_veraeusserungsgewinne: f64,
    /// Line 16: Verluste aus Veräußerungen
    pub zeile_16_veraeusserungsverluste: f64,
    /// Line 47: Angerechnete ausländische Steuern
    pub zeile_47_auslaendische_steuern: f64,
    /// Line 48: Gezahlte Kapitalertragsteuer
    pub zeile_48_kapest: f64,
    /// Line 49: Gezahlter Solidaritätszuschlag
    pub zeile_49_soli: f64,
    /// Line 50: Gezahlte Kirchensteuer
    pub zeile_50_kist: f64,
}

/// Freistellung status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreistellungStatus {
    pub year: i32,
    pub limit: f64,
    pub used: f64,
    pub remaining: f64,
    pub is_married: bool,
    pub usage_percent: f64,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate Abgeltungssteuer with optional Kirchensteuer
fn calculate_abgeltungssteuer(
    taxable_amount: f64,
    kirchensteuer_rate: Option<f64>,
) -> (f64, f64, f64) {
    if taxable_amount <= 0.0 {
        return (0.0, 0.0, 0.0);
    }

    // With Kirchensteuer, the base rate is reduced
    let effective_rate = match kirchensteuer_rate {
        Some(rate) if rate > 0.0 => {
            // Formula: effective = 0.25 / (1 + rate)
            ABGELTUNGSSTEUER_RATE / (1.0 + rate)
        }
        _ => ABGELTUNGSSTEUER_RATE,
    };

    let abgeltungssteuer = taxable_amount * effective_rate;
    let soli = abgeltungssteuer * SOLI_RATE;
    let kirchensteuer = kirchensteuer_rate
        .filter(|&r| r > 0.0)
        .map(|r| abgeltungssteuer * r)
        .unwrap_or(0.0);

    (abgeltungssteuer, soli, kirchensteuer)
}

/// Calculate creditable foreign withholding tax
fn calculate_creditable_wht(foreign_tax: f64, gross_income: f64) -> f64 {
    let max_credit = gross_income * MAX_CREDITABLE_WHT;
    foreign_tax.min(max_credit)
}

// ============================================================================
// Commands
// ============================================================================

/// Get or create tax settings for a year
#[command]
pub fn get_tax_settings(year: i32) -> Result<TaxSettings, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    // Try to get from database
    let result: Option<(bool, Option<f64>, Option<String>, f64)> = conn
        .query_row(
            "SELECT is_married, kirchensteuer_rate, bundesland, freistellung_used FROM pp_tax_settings WHERE year = ?",
            [year],
            |row| {
                Ok((
                    row.get::<_, i32>(0)? == 1,
                    row.get::<_, Option<f64>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, f64>(3)?,
                ))
            },
        )
        .ok();

    match result {
        Some((is_married, kirchensteuer_rate, bundesland, freistellung_used)) => {
            Ok(TaxSettings {
                year,
                is_married,
                kirchensteuer_rate,
                bundesland,
                freistellung_limit: get_freistellung_limit(year, is_married),
                freistellung_used,
            })
        }
        None => {
            // Return default settings
            Ok(TaxSettings {
                year,
                is_married: false,
                kirchensteuer_rate: None,
                bundesland: None,
                freistellung_limit: get_freistellung_limit(year, false),
                freistellung_used: 0.0,
            })
        }
    }
}

/// Save tax settings for a year
#[command]
pub fn save_tax_settings(settings: TaxSettings) -> Result<(), String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    conn.execute(
        r#"
        INSERT OR REPLACE INTO pp_tax_settings (year, is_married, kirchensteuer_rate, bundesland, freistellung_used)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
        rusqlite::params![
            settings.year,
            if settings.is_married { 1 } else { 0 },
            settings.kirchensteuer_rate,
            settings.bundesland,
            settings.freistellung_used,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Generate detailed German tax report
#[command]
pub fn generate_german_tax_report(year: i32) -> Result<GermanTaxReport, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let settings = get_tax_settings(year)?;

    let base_currency: String = conn
        .query_row(
            "SELECT base_currency FROM pp_import ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "EUR".to_string());

    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-12-31", year);

    // Get dividends
    let mut dividend_details: Vec<TaxableItem> = Vec::new();
    let mut total_dividend_gross = 0.0;
    let mut total_dividend_wht = 0.0;

    let mut div_stmt = conn
        .prepare(
            r#"
            SELECT
                t.date,
                COALESCE(s.name, 'Unbekannt') as name,
                s.isin,
                t.amount / 100.0 as gross,
                COALESCE((SELECT SUM(amount) FROM pp_txn_unit WHERE txn_id = t.id AND unit_type = 'TAX'), 0) / 100.0 as tax
            FROM pp_txn t
            LEFT JOIN pp_security s ON s.id = t.security_id
            WHERE t.txn_type = 'DIVIDENDS'
              AND t.date >= ?1 AND t.date <= ?2
            ORDER BY t.date
            "#,
        )
        .map_err(|e| e.to_string())?;

    let div_rows = div_stmt
        .query_map([&start_date, &end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in div_rows.flatten() {
        let (date, name, isin, gross, tax) = row;
        total_dividend_gross += gross;
        total_dividend_wht += tax;
        dividend_details.push(TaxableItem {
            date,
            security_name: name,
            security_isin: isin,
            gross_amount: gross,
            withholding_tax: tax,
            net_amount: gross - tax,
            item_type: "DIVIDEND".to_string(),
        });
    }

    // Get interest
    let total_interest: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(amount), 0) / 100.0
            FROM pp_txn
            WHERE txn_type = 'INTEREST' AND date >= ?1 AND date <= ?2
            "#,
            [&start_date, &end_date],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Get realized gains from FIFO consumption
    let mut gains_details: Vec<TaxableItem> = Vec::new();
    let mut losses_details: Vec<TaxableItem> = Vec::new();
    let mut total_gains = 0.0;
    let mut total_losses = 0.0;

    let mut gains_stmt = conn
        .prepare(
            r#"
            SELECT
                t.date,
                COALESCE(s.name, 'Unbekannt') as name,
                s.isin,
                t.amount / 100.0 as proceeds,
                COALESCE(SUM(fc.gross_amount), 0) / 100.0 as cost_basis
            FROM pp_txn t
            LEFT JOIN pp_security s ON s.id = t.security_id
            LEFT JOIN pp_fifo_consumption fc ON fc.sale_txn_id = t.id
            WHERE t.txn_type = 'SELL'
              AND t.owner_type = 'portfolio'
              AND t.date >= ?1 AND t.date <= ?2
            GROUP BY t.id
            ORDER BY t.date
            "#,
        )
        .map_err(|e| e.to_string())?;

    let gains_rows = gains_stmt
        .query_map([&start_date, &end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in gains_rows.flatten() {
        let (date, name, isin, proceeds, cost_basis) = row;
        let gain = proceeds - cost_basis;

        let item = TaxableItem {
            date: date.clone(),
            security_name: name,
            security_isin: isin,
            gross_amount: proceeds,
            withholding_tax: 0.0,
            net_amount: gain,
            item_type: if gain >= 0.0 { "GAIN" } else { "LOSS" }.to_string(),
        };

        if gain >= 0.0 {
            total_gains += gain;
            gains_details.push(item);
        } else {
            total_losses += gain.abs();
            losses_details.push(item);
        }
    }

    // Calculate totals
    let total_taxable_income = total_dividend_gross + total_interest + total_gains;

    // Apply Freistellung
    let freistellung_available = settings.freistellung_limit - settings.freistellung_used;
    let freistellung_used_now = freistellung_available.min(total_taxable_income).max(0.0);

    // Apply losses (against gains only in Germany)
    let net_gains_after_losses = (total_gains - total_losses).max(0.0);

    // Taxable amount after deductions
    let taxable_after_deductions =
        (total_dividend_gross + total_interest + net_gains_after_losses - freistellung_used_now).max(0.0);

    // Foreign tax credit
    let creditable_foreign_tax = calculate_creditable_wht(total_dividend_wht, total_dividend_gross);

    // Calculate German taxes
    let (abgeltungssteuer, soli, kirchensteuer) =
        calculate_abgeltungssteuer(taxable_after_deductions, settings.kirchensteuer_rate);

    // Apply foreign tax credit
    let total_german_tax_before_credit = abgeltungssteuer + soli + kirchensteuer;
    let total_german_tax = (total_german_tax_before_credit - creditable_foreign_tax).max(0.0);

    // Already paid (approximation from dividend withholding + any recorded tax transactions)
    let tax_already_paid: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(SUM(amount), 0) / 100.0
            FROM pp_txn
            WHERE txn_type = 'TAXES' AND date >= ?1 AND date <= ?2
            "#,
            [&start_date, &end_date],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Anlage KAP data
    // Separate inland vs. ausland dividends (simplified: assume all are foreign for now)
    let anlage_kap = AnlageKapData {
        zeile_7_inland_dividenden: 0.0, // Would need ISIN prefix check for German stocks
        zeile_8_ausland_dividenden: total_dividend_gross,
        zeile_14_zinsen: total_interest,
        zeile_15_veraeusserungsgewinne: total_gains,
        zeile_16_veraeusserungsverluste: total_losses,
        zeile_47_auslaendische_steuern: creditable_foreign_tax,
        zeile_48_kapest: total_dividend_wht + tax_already_paid, // Approximation
        zeile_49_soli: soli,
        zeile_50_kist: kirchensteuer,
    };

    Ok(GermanTaxReport {
        year,
        currency: base_currency,
        settings,
        dividend_income_gross: total_dividend_gross,
        interest_income_gross: total_interest,
        realized_gains: total_gains,
        realized_losses: total_losses,
        total_taxable_income,
        freistellung_available,
        freistellung_used: freistellung_used_now,
        loss_carryforward: 0.0, // Would need tracking across years
        taxable_after_deductions,
        foreign_withholding_tax: total_dividend_wht,
        creditable_foreign_tax,
        abgeltungssteuer,
        solidaritaetszuschlag: soli,
        kirchensteuer,
        total_german_tax,
        tax_already_paid,
        remaining_tax_liability: (total_german_tax - tax_already_paid).max(0.0),
        dividend_details,
        gains_details,
        losses_details,
        anlage_kap,
    })
}

/// Get Freistellung status for a year
#[command]
pub fn get_freistellung_status(year: i32) -> Result<FreistellungStatus, String> {
    let settings = get_tax_settings(year)?;

    let remaining = (settings.freistellung_limit - settings.freistellung_used).max(0.0);
    let usage_percent = if settings.freistellung_limit > 0.0 {
        (settings.freistellung_used / settings.freistellung_limit) * 100.0
    } else {
        0.0
    };

    Ok(FreistellungStatus {
        year,
        limit: settings.freistellung_limit,
        used: settings.freistellung_used,
        remaining,
        is_married: settings.is_married,
        usage_percent,
    })
}

/// Update Freistellung used amount
#[command]
pub fn update_freistellung_used(year: i32, amount: f64) -> Result<(), String> {
    let mut settings = get_tax_settings(year)?;
    settings.freistellung_used = amount.max(0.0).min(settings.freistellung_limit);
    save_tax_settings(settings)
}
