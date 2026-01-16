//! PDF Export Commands
//!
//! Generate PDF reports for portfolios, holdings, and performance.

use crate::db;
use crate::performance;
use chrono::{NaiveDate, NaiveDateTime};
use printpdf::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use tauri::command;

#[allow(dead_code)]
const MM_TO_PT: f32 = 2.834645669;  // 1mm = 2.834645669 points

/// PDF export result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfExportResult {
    pub success: bool,
    pub path: String,
    pub pages: i32,
}

/// Create a new PDF document
fn create_pdf(title: &str) -> (PdfDocumentReference, PdfPageIndex, PdfLayerIndex) {
    let (doc, page1, layer1) = PdfDocument::new(
        title,
        Mm(210.0),  // A4 width
        Mm(297.0),  // A4 height
        "Layer 1",
    );
    (doc, page1, layer1)
}

/// Get a built-in font
fn get_font(doc: &PdfDocumentReference) -> IndirectFontRef {
    doc.add_builtin_font(BuiltinFont::Helvetica).unwrap()
}

fn get_font_bold(doc: &PdfDocumentReference) -> IndirectFontRef {
    doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap()
}

/// Add text to a layer
fn add_text(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    x: Mm,
    y: Mm,
    size: f32,
    text: &str,
) {
    layer.use_text(text, size, x, y, font);
}

/// Parse date string flexibly - handles both "YYYY-MM-DD" and "YYYY-MM-DD HH:MM:SS" formats
fn parse_date_flexible(date_str: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .ok()
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
        .or_else(|| {
            NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|dt| dt.date())
        })
}

/// Export portfolio summary as PDF
#[command]
pub fn export_portfolio_summary_pdf(
    portfolio_id: Option<i64>,
    path: String,
) -> Result<PdfExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (doc, page1, layer1) = create_pdf("Portfolio Zusammenfassung");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Title
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(277.0), 18.0, "Portfolio Zusammenfassung");

    // Date
    let date_str = chrono::Local::now().format("%d.%m.%Y").to_string();
    add_text(&current_layer, &font, Mm(20.0), Mm(267.0), 10.0, &format!("Stand: {}", date_str));

    // Get portfolio info
    let portfolio_name = match portfolio_id {
        Some(id) => {
            conn.query_row(
                "SELECT name FROM pp_portfolio WHERE id = ?1",
                [id],
                |row| row.get::<_, String>(0)
            ).unwrap_or_else(|_| "Alle Portfolios".to_string())
        }
        None => "Alle Portfolios".to_string(),
    };

    add_text(&current_layer, &font, Mm(20.0), Mm(257.0), 12.0, &portfolio_name);

    // Get holdings
    let portfolio_filter = match portfolio_id {
        Some(id) => format!("AND t.owner_id = {}", id),
        None => String::new(),
    };

    let sql = format!(
        "SELECT
            s.name,
            s.isin,
            COALESCE(SUM(CASE
                WHEN t.txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN t.shares
                WHEN t.txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -t.shares
                ELSE 0
            END), 0) / 100000000.0 as shares,
            COALESCE(lp.value, 0) / 100000000.0 as price,
            s.currency
         FROM pp_security s
         LEFT JOIN pp_txn t ON t.security_id = s.id
            AND t.owner_type = 'portfolio'
            AND t.shares IS NOT NULL
            {}
         LEFT JOIN pp_latest_price lp ON lp.security_id = s.id
         GROUP BY s.id
         HAVING shares > 0
         ORDER BY (shares * price) DESC",
        portfolio_filter
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let holdings: Vec<(String, Option<String>, f64, f64, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Table header
    let mut y = 240.0;
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(y), 10.0, "Wertpapier");
    add_text(&current_layer, &font_bold, Mm(100.0), Mm(y), 10.0, "Stück");
    add_text(&current_layer, &font_bold, Mm(130.0), Mm(y), 10.0, "Kurs");
    add_text(&current_layer, &font_bold, Mm(160.0), Mm(y), 10.0, "Wert");

    y -= 5.0;

    // Draw line
    let line = Line {
        points: vec![
            (Point::new(Mm(20.0), Mm(y)), false),
            (Point::new(Mm(190.0), Mm(y)), false),
        ],
        is_closed: false,
    };
    current_layer.add_line(line);

    y -= 8.0;

    // Holdings
    let mut total_value = 0.0;
    for (name, _isin, shares, price, currency) in &holdings {
        if y < 30.0 {
            break;  // Would need new page
        }

        let value = shares * price;
        total_value += value;

        let display_name = if name.len() > 35 {
            format!("{}...", &name[..32])
        } else {
            name.clone()
        };

        add_text(&current_layer, &font, Mm(20.0), Mm(y), 9.0, &display_name);
        add_text(&current_layer, &font, Mm(100.0), Mm(y), 9.0, &format!("{:.2}", shares));
        add_text(&current_layer, &font, Mm(130.0), Mm(y), 9.0, &format!("{:.2} {}", price, currency));
        add_text(&current_layer, &font, Mm(160.0), Mm(y), 9.0, &format!("{:.2} {}", value, currency));

        y -= 6.0;
    }

    // Total
    y -= 5.0;
    let line2 = Line {
        points: vec![
            (Point::new(Mm(130.0), Mm(y + 3.0)), false),
            (Point::new(Mm(190.0), Mm(y + 3.0)), false),
        ],
        is_closed: false,
    };
    current_layer.add_line(line2);

    add_text(&current_layer, &font_bold, Mm(130.0), Mm(y - 3.0), 10.0, "Gesamt:");
    add_text(&current_layer, &font_bold, Mm(160.0), Mm(y - 3.0), 10.0, &format!("{:.2} EUR", total_value));

    // Footer
    add_text(
        &current_layer,
        &font,
        Mm(20.0),
        Mm(15.0),
        8.0,
        &format!("Erstellt mit Portfolio Performance Modern - {}", date_str),
    );

    // Save
    let file = File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path,
        pages: 1,
    })
}

/// Export holdings as PDF
#[command]
pub fn export_holdings_pdf(
    portfolio_id: Option<i64>,
    _date: Option<String>,
    path: String,
) -> Result<PdfExportResult, String> {
    // Similar to portfolio summary but with more detail
    export_portfolio_summary_pdf(portfolio_id, path)
}

/// Export performance report as PDF
#[command]
pub fn export_performance_pdf(
    portfolio_id: Option<i64>,
    start_date: String,
    end_date: String,
    path: String,
) -> Result<PdfExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let start = parse_date_flexible(&start_date)
        .ok_or_else(|| "Invalid start_date".to_string())?;
    let end = parse_date_flexible(&end_date)
        .ok_or_else(|| "Invalid end_date".to_string())?;

    let (doc, page1, layer1) = create_pdf("Performance Bericht");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Title
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(277.0), 18.0, "Performance Bericht");

    // Period
    add_text(
        &current_layer,
        &font,
        Mm(20.0),
        Mm(267.0),
        10.0,
        &format!("Zeitraum: {} bis {}", start_date, end_date),
    );

    // Get performance data from SSOT
    let ttwror_result = performance::calculate_ttwror(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    let cash_flows = performance::get_cash_flows_with_fallback(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;

    let current_value = performance::get_portfolio_value_at_date_with_currency(conn, portfolio_id, end)
        .map_err(|e| e.to_string())?;

    let irr_result = performance::calculate_irr(&cash_flows, current_value, end)
        .map_err(|e| e.to_string())?;

    let risk_metrics = performance::calculate_risk_metrics(conn, portfolio_id, start, end, None, None).ok();
    let mut y = 250.0;

    add_text(&current_layer, &font_bold, Mm(20.0), Mm(y), 12.0, "Performance Kennzahlen");
    y -= 10.0;

    let ttwror = ttwror_result.total_return * 100.0;
    let ttwror_annualized = ttwror_result.annualized_return * 100.0;
    let irr = irr_result.irr * 100.0;

    let max_drawdown = risk_metrics
        .as_ref()
        .map(|m| format!("-{:.2}%", m.max_drawdown * 100.0))
        .unwrap_or_else(|| "n/a".to_string());
    let volatility = risk_metrics
        .as_ref()
        .map(|m| format!("{:.2}%", m.volatility * 100.0))
        .unwrap_or_else(|| "n/a".to_string());

    let metrics = vec![
        ("Gesamtrendite (TTWROR)", format!("{:.2}%", ttwror)),
        ("Annualisierte Rendite", format!("{:.2}%", ttwror_annualized)),
        ("Interner Zinsfuß (IRR)", format!("{:.2}%", irr)),
        ("Max. Drawdown", max_drawdown),
        ("Volatilität (ann.)", volatility),
    ];

    for (label, value) in metrics {
        add_text(&current_layer, &font, Mm(20.0), Mm(y), 10.0, label);
        add_text(&current_layer, &font, Mm(120.0), Mm(y), 10.0, &value);
        y -= 6.0;
    }

    // Footer
    let date_str = chrono::Local::now().format("%d.%m.%Y").to_string();
    add_text(
        &current_layer,
        &font,
        Mm(20.0),
        Mm(15.0),
        8.0,
        &format!("Erstellt mit Portfolio Performance Modern - {}", date_str),
    );

    // Save
    let file = File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path,
        pages: 1,
    })
}

/// Export dividend report as PDF
#[command]
pub fn export_dividend_pdf(
    year: i32,
    _portfolio_id: Option<i64>,
    path: String,
) -> Result<PdfExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (doc, page1, layer1) = create_pdf("Dividenden Übersicht");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Title
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(277.0), 18.0, &format!("Dividenden Übersicht {}", year));

    // Get dividend data
    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-12-31", year);

    let mut stmt = conn.prepare(
        "SELECT
            s.name,
            t.date,
            t.amount / 100.0 as amount,
            COALESCE((SELECT SUM(u.amount) FROM pp_txn_unit u WHERE u.txn_id = t.id AND u.unit_type = 'TAX'), 0) / 100.0 as taxes
         FROM pp_txn t
         JOIN pp_security s ON s.id = t.security_id
         WHERE t.txn_type = 'DIVIDENDS'
           AND t.date >= ?1 AND t.date <= ?2
         ORDER BY t.date"
    ).map_err(|e| e.to_string())?;

    let dividends: Vec<(String, String, f64, f64)> = stmt
        .query_map(rusqlite::params![start_date, end_date], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Table header
    let mut y = 260.0;
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(y), 10.0, "Wertpapier");
    add_text(&current_layer, &font_bold, Mm(100.0), Mm(y), 10.0, "Datum");
    add_text(&current_layer, &font_bold, Mm(130.0), Mm(y), 10.0, "Brutto");
    add_text(&current_layer, &font_bold, Mm(160.0), Mm(y), 10.0, "Steuern");

    y -= 5.0;
    let line = Line {
        points: vec![
            (Point::new(Mm(20.0), Mm(y)), false),
            (Point::new(Mm(190.0), Mm(y)), false),
        ],
        is_closed: false,
    };
    current_layer.add_line(line);

    y -= 8.0;

    let mut total_gross = 0.0;
    let mut total_tax = 0.0;

    for (name, date, amount, taxes) in &dividends {
        if y < 30.0 {
            break;
        }

        total_gross += amount;
        total_tax += taxes;

        let display_name = if name.len() > 35 {
            format!("{}...", &name[..32])
        } else {
            name.clone()
        };

        add_text(&current_layer, &font, Mm(20.0), Mm(y), 9.0, &display_name);
        add_text(&current_layer, &font, Mm(100.0), Mm(y), 9.0, date);
        add_text(&current_layer, &font, Mm(130.0), Mm(y), 9.0, &format!("{:.2} €", amount));
        add_text(&current_layer, &font, Mm(160.0), Mm(y), 9.0, &format!("{:.2} €", taxes));

        y -= 6.0;
    }

    // Totals
    y -= 5.0;
    let line2 = Line {
        points: vec![
            (Point::new(Mm(100.0), Mm(y + 3.0)), false),
            (Point::new(Mm(190.0), Mm(y + 3.0)), false),
        ],
        is_closed: false,
    };
    current_layer.add_line(line2);

    add_text(&current_layer, &font_bold, Mm(100.0), Mm(y - 3.0), 10.0, "Gesamt:");
    add_text(&current_layer, &font_bold, Mm(130.0), Mm(y - 3.0), 10.0, &format!("{:.2} €", total_gross));
    add_text(&current_layer, &font_bold, Mm(160.0), Mm(y - 3.0), 10.0, &format!("{:.2} €", total_tax));

    y -= 12.0;
    add_text(&current_layer, &font_bold, Mm(100.0), Mm(y), 10.0, "Netto:");
    add_text(&current_layer, &font_bold, Mm(130.0), Mm(y), 10.0, &format!("{:.2} €", total_gross - total_tax));

    // Footer
    let date_str = chrono::Local::now().format("%d.%m.%Y").to_string();
    add_text(
        &current_layer,
        &font,
        Mm(20.0),
        Mm(15.0),
        8.0,
        &format!("Erstellt mit Portfolio Performance Modern - {}", date_str),
    );

    // Save
    let file = File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path,
        pages: 1,
    })
}

/// Export tax report as PDF
#[command]
pub fn export_tax_report_pdf(
    year: i32,
    path: String,
) -> Result<PdfExportResult, String> {
    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (doc, page1, layer1) = create_pdf("Steuerbericht");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Title
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(277.0), 18.0, &format!("Steuerbericht {}", year));

    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-12-31", year);

    // Calculate totals
    let dividend_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) / 100.0 FROM pp_txn WHERE txn_type = 'DIVIDENDS' AND date >= ?1 AND date <= ?2",
        rusqlite::params![start_date, end_date],
        |row| row.get(0)
    ).unwrap_or(0.0);

    let interest_total: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) / 100.0 FROM pp_txn WHERE txn_type = 'INTEREST' AND date >= ?1 AND date <= ?2",
        rusqlite::params![start_date, end_date],
        |row| row.get(0)
    ).unwrap_or(0.0);

    // Summary section
    let mut y = 257.0;
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(y), 12.0, "Zusammenfassung");
    y -= 10.0;

    let items = vec![
        ("Dividenden (brutto)", dividend_total),
        ("Zinsen", interest_total),
        ("Kapitalerträge gesamt", dividend_total + interest_total),
    ];

    for (label, value) in items {
        add_text(&current_layer, &font, Mm(20.0), Mm(y), 10.0, label);
        add_text(&current_layer, &font, Mm(140.0), Mm(y), 10.0, &format!("{:.2} €", value));
        y -= 6.0;
    }

    y -= 10.0;

    // Tax summary
    add_text(&current_layer, &font_bold, Mm(20.0), Mm(y), 12.0, "Einbehaltene Steuern");
    y -= 10.0;

    // Get tax totals
    let tax_withheld: f64 = conn.query_row(
        "SELECT COALESCE(SUM(u.amount), 0) / 100.0
         FROM pp_txn_unit u
         JOIN pp_txn t ON t.id = u.txn_id
         WHERE u.unit_type = 'TAX' AND t.date >= ?1 AND t.date <= ?2",
        rusqlite::params![start_date, end_date],
        |row| row.get(0)
    ).unwrap_or(0.0);

    add_text(&current_layer, &font, Mm(20.0), Mm(y), 10.0, "Einbehaltene Steuern");
    add_text(&current_layer, &font, Mm(140.0), Mm(y), 10.0, &format!("{:.2} €", tax_withheld));

    // Disclaimer
    y = 40.0;
    add_text(
        &current_layer,
        &font,
        Mm(20.0),
        Mm(y),
        8.0,
        "Hinweis: Dieser Bericht dient nur der Information und ersetzt keine Steuerberatung.",
    );

    // Footer
    let date_str = chrono::Local::now().format("%d.%m.%Y").to_string();
    add_text(
        &current_layer,
        &font,
        Mm(20.0),
        Mm(15.0),
        8.0,
        &format!("Erstellt mit Portfolio Performance Modern - {}", date_str),
    );

    // Save
    let file = File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path,
        pages: 1,
    })
}
