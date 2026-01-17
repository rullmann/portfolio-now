//! PDF Export Commands
//!
//! Generate professional PDF reports for portfolios, holdings, and performance.

use crate::db;
use crate::performance;
use crate::pp::parse_date_flexible;
use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use tauri::command;

// Colors
const COLOR_PRIMARY: (f32, f32, f32) = (0.15, 0.35, 0.60);      // Dark blue
const COLOR_HEADER_BG: (f32, f32, f32) = (0.93, 0.95, 0.98);    // Light blue-gray
const COLOR_ROW_ALT: (f32, f32, f32) = (0.97, 0.97, 0.97);      // Light gray for alternating rows
const COLOR_POSITIVE: (f32, f32, f32) = (0.13, 0.55, 0.13);     // Green
const COLOR_NEGATIVE: (f32, f32, f32) = (0.80, 0.20, 0.20);     // Red
const COLOR_TEXT: (f32, f32, f32) = (0.20, 0.20, 0.20);         // Dark gray
const COLOR_MUTED: (f32, f32, f32) = (0.50, 0.50, 0.50);        // Medium gray

// Layout constants
const PAGE_WIDTH: f32 = 210.0;
const PAGE_HEIGHT: f32 = 297.0;
const MARGIN_LEFT: f32 = 20.0;
const MARGIN_RIGHT: f32 = 20.0;
const CONTENT_WIDTH: f32 = PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT;

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
        Mm(PAGE_WIDTH),
        Mm(PAGE_HEIGHT),
        "Layer 1",
    );
    (doc, page1, layer1)
}

/// Get fonts
fn get_font(doc: &PdfDocumentReference) -> IndirectFontRef {
    doc.add_builtin_font(BuiltinFont::Helvetica).unwrap()
}

fn get_font_bold(doc: &PdfDocumentReference) -> IndirectFontRef {
    doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap()
}

/// Add text with color
fn add_text_colored(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    x: Mm,
    y: Mm,
    size: f32,
    text: &str,
    color: (f32, f32, f32),
) {
    layer.set_fill_color(Color::Rgb(Rgb::new(color.0, color.1, color.2, None)));
    layer.use_text(text, size, x, y, font);
}

/// Add text (default color)
fn add_text(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    x: Mm,
    y: Mm,
    size: f32,
    text: &str,
) {
    add_text_colored(layer, font, x, y, size, text, COLOR_TEXT);
}

/// Draw a filled rectangle
fn draw_rect(
    layer: &PdfLayerReference,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: (f32, f32, f32),
) {
    layer.set_fill_color(Color::Rgb(Rgb::new(color.0, color.1, color.2, None)));
    let points = vec![
        (Point::new(Mm(x), Mm(y)), false),
        (Point::new(Mm(x + width), Mm(y)), false),
        (Point::new(Mm(x + width), Mm(y + height)), false),
        (Point::new(Mm(x), Mm(y + height)), false),
    ];
    let polygon = Polygon {
        rings: vec![points],
        mode: PaintMode::Fill,
        winding_order: WindingOrder::NonZero,
    };
    layer.add_polygon(polygon);
}

/// Draw a horizontal line
fn draw_line(
    layer: &PdfLayerReference,
    x1: f32,
    y: f32,
    x2: f32,
    color: (f32, f32, f32),
    thickness: f32,
) {
    layer.set_outline_color(Color::Rgb(Rgb::new(color.0, color.1, color.2, None)));
    layer.set_outline_thickness(thickness);
    let line = Line {
        points: vec![
            (Point::new(Mm(x1), Mm(y)), false),
            (Point::new(Mm(x2), Mm(y)), false),
        ],
        is_closed: false,
    };
    layer.add_line(line);
}

/// Draw page header with title
fn draw_header(
    layer: &PdfLayerReference,
    font_bold: &IndirectFontRef,
    font: &IndirectFontRef,
    title: &str,
    subtitle: Option<&str>,
) -> f32 {
    // Header background
    draw_rect(layer, 0.0, PAGE_HEIGHT - 35.0, PAGE_WIDTH, 35.0, COLOR_HEADER_BG);

    // Title
    add_text_colored(layer, font_bold, Mm(MARGIN_LEFT), Mm(PAGE_HEIGHT - 18.0), 16.0, title, COLOR_PRIMARY);

    // Date on the right
    let date_str = chrono::Local::now().format("%d.%m.%Y").to_string();
    add_text_colored(layer, font, Mm(PAGE_WIDTH - MARGIN_RIGHT - 25.0), Mm(PAGE_HEIGHT - 18.0), 9.0, &date_str, COLOR_MUTED);

    // Subtitle if provided
    if let Some(sub) = subtitle {
        add_text_colored(layer, font, Mm(MARGIN_LEFT), Mm(PAGE_HEIGHT - 28.0), 10.0, sub, COLOR_MUTED);
    }

    // Accent line under header
    draw_line(layer, 0.0, PAGE_HEIGHT - 35.0, PAGE_WIDTH, COLOR_PRIMARY, 1.5);

    PAGE_HEIGHT - 50.0 // Return Y position for content start
}

/// Draw page footer
fn draw_footer(layer: &PdfLayerReference, font: &IndirectFontRef, page_num: i32, total_pages: i32) {
    let date_str = chrono::Local::now().format("%d.%m.%Y").to_string();

    // Footer line
    draw_line(layer, MARGIN_LEFT, 18.0, PAGE_WIDTH - MARGIN_RIGHT, COLOR_HEADER_BG, 0.5);

    // App name on left
    add_text_colored(layer, font, Mm(MARGIN_LEFT), Mm(12.0), 7.0, "Portfolio Now", COLOR_MUTED);

    // Date in center
    add_text_colored(layer, font, Mm(PAGE_WIDTH / 2.0 - 10.0), Mm(12.0), 7.0, &date_str, COLOR_MUTED);

    // Page number on right
    let page_text = format!("Seite {} von {}", page_num, total_pages);
    add_text_colored(layer, font, Mm(PAGE_WIDTH - MARGIN_RIGHT - 20.0), Mm(12.0), 7.0, &page_text, COLOR_MUTED);
}

/// Draw a section header
fn draw_section_header(
    layer: &PdfLayerReference,
    font_bold: &IndirectFontRef,
    y: f32,
    title: &str,
) -> f32 {
    add_text_colored(layer, font_bold, Mm(MARGIN_LEFT), Mm(y), 11.0, title, COLOR_PRIMARY);
    draw_line(layer, MARGIN_LEFT, y - 3.0, PAGE_WIDTH - MARGIN_RIGHT, COLOR_PRIMARY, 0.5);
    y - 12.0
}

/// Draw table header row
fn draw_table_header(
    layer: &PdfLayerReference,
    font_bold: &IndirectFontRef,
    y: f32,
    columns: &[(&str, f32)], // (label, x position)
) -> f32 {
    // Header background
    draw_rect(layer, MARGIN_LEFT, y - 5.0, CONTENT_WIDTH, 7.0, COLOR_HEADER_BG);

    for (label, x) in columns {
        add_text_colored(layer, font_bold, Mm(*x), Mm(y - 3.5), 8.0, label, COLOR_TEXT);
    }

    y - 10.0
}

/// Format number with German locale
fn format_number(value: f64, decimals: usize) -> String {
    let formatted = format!("{:.prec$}", value, prec = decimals);
    // Replace . with , for German format
    formatted.replace('.', ",")
}

/// Format currency value
fn format_currency(value: f64, currency: &str) -> String {
    format!("{} {}", format_number(value, 2), currency)
}

/// Truncate text if too long
fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() > max_chars {
        format!("{}...", &text[..max_chars.saturating_sub(3)])
    } else {
        text.to_string()
    }
}

/// Export portfolio summary as PDF
#[command]
pub fn export_portfolio_summary_pdf(
    portfolio_id: Option<i64>,
    path: String,
) -> Result<PdfExportResult, String> {
    let validated_path = crate::security::validate_file_path_with_extension(&path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (doc, page1, layer1) = create_pdf("Portfolio Zusammenfassung");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Get portfolio name
    let portfolio_name = match portfolio_id {
        Some(id) => conn.query_row(
            "SELECT name FROM pp_portfolio WHERE id = ?1",
            [id],
            |row| row.get::<_, String>(0)
        ).unwrap_or_else(|_| "Alle Portfolios".to_string()),
        None => "Alle Portfolios".to_string(),
    };

    // Header
    let mut y = draw_header(&current_layer, &font_bold, &font, "Portfolio Zusammenfassung", Some(&portfolio_name));

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
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Section: Vermögensaufstellung
    y = draw_section_header(&current_layer, &font_bold, y, "Vermögensaufstellung");

    // Table header
    let columns = [
        ("Wertpapier", MARGIN_LEFT),
        ("ISIN", 85.0),
        ("Stück", 125.0),
        ("Kurs", 145.0),
        ("Wert", 170.0),
    ];
    y = draw_table_header(&current_layer, &font_bold, y, &columns);

    // Holdings rows
    let mut total_by_currency: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

    for (i, (name, isin, shares, price, currency)) in holdings.iter().enumerate() {
        if y < 45.0 {
            break; // Would need new page
        }

        let value = shares * price;
        *total_by_currency.entry(currency.clone()).or_insert(0.0) += value;

        // Alternating row background
        if i % 2 == 1 {
            draw_rect(&current_layer, MARGIN_LEFT, y - 4.0, CONTENT_WIDTH, 6.0, COLOR_ROW_ALT);
        }

        add_text(&current_layer, &font, Mm(MARGIN_LEFT), Mm(y - 3.0), 8.0, &truncate_text(name, 30));
        add_text_colored(&current_layer, &font, Mm(85.0), Mm(y - 3.0), 7.0,
            &isin.as_ref().map(|s| truncate_text(s, 12)).unwrap_or_else(|| "-".to_string()), COLOR_MUTED);
        add_text(&current_layer, &font, Mm(125.0), Mm(y - 3.0), 8.0, &format_number(*shares, 2));
        add_text(&current_layer, &font, Mm(145.0), Mm(y - 3.0), 8.0, &format_currency(*price, currency));
        add_text_colored(&current_layer, &font_bold, Mm(170.0), Mm(y - 3.0), 8.0,
            &format_currency(value, currency), COLOR_TEXT);

        y -= 6.0;
    }

    // Totals section
    y -= 5.0;
    draw_line(&current_layer, 145.0, y + 2.0, PAGE_WIDTH - MARGIN_RIGHT, COLOR_PRIMARY, 1.0);
    y -= 5.0;

    if total_by_currency.len() == 1 {
        let (currency, total) = total_by_currency.iter().next().unwrap();
        add_text_colored(&current_layer, &font_bold, Mm(145.0), Mm(y), 10.0, "Gesamt:", COLOR_PRIMARY);
        add_text_colored(&current_layer, &font_bold, Mm(170.0), Mm(y), 10.0,
            &format_currency(*total, currency), COLOR_PRIMARY);
    } else {
        add_text_colored(&current_layer, &font_bold, Mm(145.0), Mm(y), 9.0, "Gesamt:", COLOR_PRIMARY);
        for (currency, total) in &total_by_currency {
            y -= 5.0;
            add_text_colored(&current_layer, &font_bold, Mm(170.0), Mm(y), 9.0,
                &format_currency(*total, currency), COLOR_TEXT);
        }
    }

    // Footer
    draw_footer(&current_layer, &font, 1, 1);

    // Save
    let file = File::create(&validated_path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path: validated_path.to_string_lossy().to_string(),
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
    let validated_path = crate::security::validate_file_path_with_extension(&path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let start = parse_date_flexible(&start_date).ok_or_else(|| "Invalid start_date".to_string())?;
    let end = parse_date_flexible(&end_date).ok_or_else(|| "Invalid end_date".to_string())?;

    let (doc, page1, layer1) = create_pdf("Performance Bericht");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Format dates for display
    let start_formatted = start.format("%d.%m.%Y").to_string();
    let end_formatted = end.format("%d.%m.%Y").to_string();
    let period = format!("{} - {}", start_formatted, end_formatted);

    // Header
    let mut y = draw_header(&current_layer, &font_bold, &font, "Performance Bericht", Some(&period));

    // Get performance data
    let ttwror_result = performance::calculate_ttwror(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;
    let cash_flows = performance::get_cash_flows_with_fallback(conn, portfolio_id, start, end)
        .map_err(|e| e.to_string())?;
    let current_value = performance::get_portfolio_value_at_date_with_currency(conn, portfolio_id, end)
        .map_err(|e| e.to_string())?;
    let irr_result = performance::calculate_irr(&cash_flows, current_value, end)
        .map_err(|e| e.to_string())?;
    let risk_metrics = performance::calculate_risk_metrics(conn, portfolio_id, start, end, None, None).ok();

    // Section: Performance Kennzahlen
    y = draw_section_header(&current_layer, &font_bold, y, "Performance Kennzahlen");

    let ttwror = ttwror_result.total_return * 100.0;
    let ttwror_annualized = ttwror_result.annualized_return * 100.0;
    let irr = irr_result.irr * 100.0;

    let metrics = [
        ("Gesamtrendite (TTWROR)", format!("{}%", format_number(ttwror, 2)), ttwror >= 0.0),
        ("Annualisierte Rendite", format!("{}%", format_number(ttwror_annualized, 2)), ttwror_annualized >= 0.0),
        ("Interner Zinsfuß (IRR)", format!("{}%", format_number(irr, 2)), irr >= 0.0),
    ];

    for (i, (label, value, is_positive)) in metrics.iter().enumerate() {
        if i % 2 == 1 {
            draw_rect(&current_layer, MARGIN_LEFT, y - 4.0, CONTENT_WIDTH, 8.0, COLOR_ROW_ALT);
        }
        add_text(&current_layer, &font, Mm(MARGIN_LEFT), Mm(y - 2.0), 9.0, label);
        let color = if *is_positive { COLOR_POSITIVE } else { COLOR_NEGATIVE };
        add_text_colored(&current_layer, &font_bold, Mm(130.0), Mm(y - 2.0), 9.0, value, color);
        y -= 8.0;
    }

    // Section: Risikokennzahlen
    y -= 10.0;
    y = draw_section_header(&current_layer, &font_bold, y, "Risikokennzahlen");

    let max_drawdown = risk_metrics.as_ref()
        .map(|m| format!("-{}%", format_number(m.max_drawdown * 100.0, 2)))
        .unwrap_or_else(|| "n/a".to_string());
    let volatility = risk_metrics.as_ref()
        .map(|m| format!("{}%", format_number(m.volatility * 100.0, 2)))
        .unwrap_or_else(|| "n/a".to_string());
    let sharpe = risk_metrics.as_ref()
        .map(|m| format_number(m.sharpe_ratio, 2))
        .unwrap_or_else(|| "n/a".to_string());

    let risk_items = [
        ("Max. Drawdown", max_drawdown),
        ("Volatilität (ann.)", volatility),
        ("Sharpe Ratio", sharpe),
    ];

    for (i, (label, value)) in risk_items.iter().enumerate() {
        if i % 2 == 1 {
            draw_rect(&current_layer, MARGIN_LEFT, y - 4.0, CONTENT_WIDTH, 8.0, COLOR_ROW_ALT);
        }
        add_text(&current_layer, &font, Mm(MARGIN_LEFT), Mm(y - 2.0), 9.0, label);
        add_text(&current_layer, &font_bold, Mm(130.0), Mm(y - 2.0), 9.0, value);
        y -= 8.0;
    }

    // Footer
    draw_footer(&current_layer, &font, 1, 1);

    // Save
    let file = File::create(&validated_path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path: validated_path.to_string_lossy().to_string(),
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
    let validated_path = crate::security::validate_file_path_with_extension(&path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (doc, page1, layer1) = create_pdf("Dividenden Übersicht");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Header
    let mut y = draw_header(&current_layer, &font_bold, &font,
        &format!("Dividenden Übersicht {}", year), None);

    // Get dividend data
    let start_date = format!("{}-01-01", year);
    let end_date = format!("{}-12-31", year);

    let mut stmt = conn.prepare(
        "SELECT
            s.name,
            t.date,
            t.amount / 100.0 as amount,
            t.currency,
            COALESCE((SELECT SUM(u.amount) FROM pp_txn_unit u WHERE u.txn_id = t.id AND u.unit_type = 'TAX'), 0) / 100.0 as taxes
         FROM pp_txn t
         JOIN pp_security s ON s.id = t.security_id
         WHERE t.txn_type = 'DIVIDENDS'
           AND t.date >= ?1 AND t.date <= ?2
         ORDER BY t.date"
    ).map_err(|e| e.to_string())?;

    let dividends: Vec<(String, String, f64, String, f64)> = stmt
        .query_map(rusqlite::params![start_date, end_date], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Section: Dividenden
    y = draw_section_header(&current_layer, &font_bold, y, "Dividendenzahlungen");

    // Table header
    let columns = [
        ("Wertpapier", MARGIN_LEFT),
        ("Datum", 90.0),
        ("Brutto", 125.0),
        ("Steuern", 155.0),
        ("Netto", 180.0),
    ];
    y = draw_table_header(&current_layer, &font_bold, y, &columns);

    let mut total_gross = 0.0;
    let mut total_tax = 0.0;

    for (i, (name, date, amount, currency, taxes)) in dividends.iter().enumerate() {
        if y < 50.0 {
            break;
        }

        total_gross += amount;
        total_tax += taxes;
        let net = amount - taxes;

        // Parse and format date
        let date_formatted = if let Some(d) = parse_date_flexible(date) {
            d.format("%d.%m.%Y").to_string()
        } else {
            date.clone()
        };

        // Alternating row background
        if i % 2 == 1 {
            draw_rect(&current_layer, MARGIN_LEFT, y - 4.0, CONTENT_WIDTH, 6.0, COLOR_ROW_ALT);
        }

        add_text(&current_layer, &font, Mm(MARGIN_LEFT), Mm(y - 3.0), 8.0, &truncate_text(name, 32));
        add_text_colored(&current_layer, &font, Mm(90.0), Mm(y - 3.0), 8.0, &date_formatted, COLOR_MUTED);
        add_text(&current_layer, &font, Mm(125.0), Mm(y - 3.0), 8.0, &format_currency(*amount, currency));
        add_text_colored(&current_layer, &font, Mm(155.0), Mm(y - 3.0), 8.0,
            &format_currency(*taxes, currency), COLOR_NEGATIVE);
        add_text_colored(&current_layer, &font_bold, Mm(180.0), Mm(y - 3.0), 8.0,
            &format_currency(net, currency), COLOR_POSITIVE);

        y -= 6.0;
    }

    // Totals
    y -= 5.0;
    draw_line(&current_layer, 125.0, y + 2.0, PAGE_WIDTH - MARGIN_RIGHT, COLOR_PRIMARY, 1.0);
    y -= 8.0;

    // Summary box
    draw_rect(&current_layer, MARGIN_LEFT, y - 25.0, CONTENT_WIDTH, 30.0, COLOR_HEADER_BG);

    add_text_colored(&current_layer, &font_bold, Mm(MARGIN_LEFT + 5.0), Mm(y - 2.0), 9.0, "Zusammenfassung", COLOR_PRIMARY);
    y -= 10.0;

    add_text(&current_layer, &font, Mm(MARGIN_LEFT + 5.0), Mm(y - 2.0), 8.0, "Brutto-Dividenden:");
    add_text(&current_layer, &font_bold, Mm(125.0), Mm(y - 2.0), 8.0, &format!("{} EUR", format_number(total_gross, 2)));
    y -= 6.0;

    add_text(&current_layer, &font, Mm(MARGIN_LEFT + 5.0), Mm(y - 2.0), 8.0, "Einbehaltene Steuern:");
    add_text_colored(&current_layer, &font_bold, Mm(125.0), Mm(y - 2.0), 8.0,
        &format!("-{} EUR", format_number(total_tax, 2)), COLOR_NEGATIVE);
    y -= 6.0;

    add_text_colored(&current_layer, &font_bold, Mm(MARGIN_LEFT + 5.0), Mm(y - 2.0), 9.0, "Netto-Dividenden:", COLOR_PRIMARY);
    add_text_colored(&current_layer, &font_bold, Mm(125.0), Mm(y - 2.0), 9.0,
        &format!("{} EUR", format_number(total_gross - total_tax, 2)), COLOR_POSITIVE);

    // Footer
    draw_footer(&current_layer, &font, 1, 1);

    // Save
    let file = File::create(&validated_path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path: validated_path.to_string_lossy().to_string(),
        pages: 1,
    })
}

/// Export tax report as PDF
#[command]
pub fn export_tax_report_pdf(
    year: i32,
    path: String,
) -> Result<PdfExportResult, String> {
    let validated_path = crate::security::validate_file_path_with_extension(&path, Some(&["pdf"]))
        .map_err(|e| format!("Invalid file path: {}", e))?;

    let conn_guard = db::get_connection().map_err(|e| e.to_string())?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| "Database not initialized".to_string())?;

    let (doc, page1, layer1) = create_pdf("Steuerbericht");
    let font = get_font(&doc);
    let font_bold = get_font_bold(&doc);
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Header
    let mut y = draw_header(&current_layer, &font_bold, &font,
        &format!("Steuerbericht {}", year), Some("Kapitalerträge und Steuern"));

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

    let tax_withheld: f64 = conn.query_row(
        "SELECT COALESCE(SUM(u.amount), 0) / 100.0
         FROM pp_txn_unit u
         JOIN pp_txn t ON t.id = u.txn_id
         WHERE u.unit_type = 'TAX' AND t.date >= ?1 AND t.date <= ?2",
        rusqlite::params![start_date, end_date],
        |row| row.get(0)
    ).unwrap_or(0.0);

    // Section: Kapitalerträge
    y = draw_section_header(&current_layer, &font_bold, y, "Kapitalerträge");

    let income_items = [
        ("Dividenden (brutto)", dividend_total),
        ("Zinserträge", interest_total),
    ];

    for (i, (label, value)) in income_items.iter().enumerate() {
        if i % 2 == 1 {
            draw_rect(&current_layer, MARGIN_LEFT, y - 4.0, CONTENT_WIDTH, 8.0, COLOR_ROW_ALT);
        }
        add_text(&current_layer, &font, Mm(MARGIN_LEFT), Mm(y - 2.0), 9.0, label);
        add_text(&current_layer, &font_bold, Mm(140.0), Mm(y - 2.0), 9.0, &format!("{} EUR", format_number(*value, 2)));
        y -= 8.0;
    }

    // Total income
    y -= 2.0;
    draw_line(&current_layer, 100.0, y + 2.0, PAGE_WIDTH - MARGIN_RIGHT, COLOR_PRIMARY, 0.5);
    y -= 6.0;
    add_text_colored(&current_layer, &font_bold, Mm(MARGIN_LEFT), Mm(y), 10.0, "Kapitalerträge gesamt:", COLOR_PRIMARY);
    add_text_colored(&current_layer, &font_bold, Mm(140.0), Mm(y), 10.0,
        &format!("{} EUR", format_number(dividend_total + interest_total, 2)), COLOR_PRIMARY);

    // Section: Einbehaltene Steuern
    y -= 20.0;
    y = draw_section_header(&current_layer, &font_bold, y, "Einbehaltene Steuern");

    draw_rect(&current_layer, MARGIN_LEFT, y - 4.0, CONTENT_WIDTH, 8.0, COLOR_ROW_ALT);
    add_text(&current_layer, &font, Mm(MARGIN_LEFT), Mm(y - 2.0), 9.0, "Quellensteuer / Kapitalertragsteuer");
    add_text_colored(&current_layer, &font_bold, Mm(140.0), Mm(y - 2.0), 9.0,
        &format!("{} EUR", format_number(tax_withheld, 2)), COLOR_NEGATIVE);

    // Summary box
    y -= 25.0;
    draw_rect(&current_layer, MARGIN_LEFT, y - 20.0, CONTENT_WIDTH, 25.0, COLOR_HEADER_BG);

    add_text_colored(&current_layer, &font_bold, Mm(MARGIN_LEFT + 5.0), Mm(y - 2.0), 10.0,
        "Netto-Kapitalerträge:", COLOR_PRIMARY);
    add_text_colored(&current_layer, &font_bold, Mm(140.0), Mm(y - 2.0), 10.0,
        &format!("{} EUR", format_number(dividend_total + interest_total - tax_withheld, 2)), COLOR_POSITIVE);

    // Disclaimer
    y -= 40.0;
    draw_rect(&current_layer, MARGIN_LEFT, y - 15.0, CONTENT_WIDTH, 20.0, (0.99, 0.95, 0.90)); // Light orange
    add_text_colored(&current_layer, &font_bold, Mm(MARGIN_LEFT + 5.0), Mm(y - 2.0), 8.0, "Hinweis:", (0.8, 0.5, 0.0));
    add_text_colored(&current_layer, &font, Mm(MARGIN_LEFT + 5.0), Mm(y - 10.0), 7.5,
        "Dieser Bericht dient nur der Information und ersetzt keine Steuerberatung.", COLOR_TEXT);

    // Footer
    draw_footer(&current_layer, &font, 1, 1);

    // Save
    let file = File::create(&validated_path).map_err(|e| format!("Failed to create file: {}", e))?;
    doc.save(&mut BufWriter::new(file)).map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(PdfExportResult {
        success: true,
        path: validated_path.to_string_lossy().to_string(),
        pages: 1,
    })
}
