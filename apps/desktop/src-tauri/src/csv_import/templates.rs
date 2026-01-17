//! Broker templates for CSV import.
//!
//! Contains predefined column mappings for various German and international brokers.

use crate::commands::csv::CsvColumnMapping;
use serde::{Deserialize, Serialize};

/// A broker template with detection patterns and column mapping.
#[derive(Debug, Clone)]
pub struct BrokerTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Headers that must be present for detection (case-insensitive)
    pub detection_headers: &'static [&'static str],
    pub delimiter: char,
    pub date_format: &'static str,
    pub decimal_separator: char,
    pub mapping: CsvColumnMapping,
    /// Maps broker-specific transaction types to internal types
    pub type_mapping: &'static [(&'static str, &'static str)],
}

/// Result of broker detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokerDetectionResult {
    pub template_id: Option<String>,
    pub broker_name: String,
    pub confidence: f32,
    pub detected_headers: Vec<String>,
}

/// Summary of a broker template for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrokerTemplateSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

// ============================================================================
// Broker Templates
// ============================================================================

/// Trade Republic CSV template
const TRADE_REPUBLIC: BrokerTemplate = BrokerTemplate {
    id: "trade-republic",
    name: "Trade Republic",
    description: "Trade Republic Transaktionsexport",
    detection_headers: &["Datum", "Typ", "ISIN", "Stück", "Preis je Aktie"],
    delimiter: ';',
    date_format: "%d.%m.%Y",
    decimal_separator: ',',
    mapping: CsvColumnMapping {
        date: Some(0),
        txn_type: Some(1),
        isin: Some(2),
        shares: Some(3),
        amount: Some(5),
        currency: Some(6),
        security_name: None,
        fees: None,
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("Kauf", "BUY"),
        ("Verkauf", "SELL"),
        ("Dividende", "DIVIDENDS"),
        ("Einzahlung", "DEPOSIT"),
        ("Auszahlung", "REMOVAL"),
        ("Zinsen", "INTEREST"),
    ],
};

/// Scalable Capital CSV template
const SCALABLE_CAPITAL: BrokerTemplate = BrokerTemplate {
    id: "scalable-capital",
    name: "Scalable Capital",
    description: "Scalable Capital / Baader Bank Export",
    detection_headers: &["Buchungsdatum", "Typ", "Wertpapier", "ISIN", "Anzahl", "Kurs"],
    delimiter: ';',
    date_format: "%d.%m.%Y",
    decimal_separator: ',',
    mapping: CsvColumnMapping {
        date: Some(0),
        txn_type: Some(1),
        security_name: Some(2),
        isin: Some(3),
        shares: Some(4),
        amount: Some(6),
        currency: Some(7),
        fees: None,
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("Kauf", "BUY"),
        ("Verkauf", "SELL"),
        ("Ausschüttung", "DIVIDENDS"),
        ("Einlage", "DEPOSIT"),
        ("Entnahme", "REMOVAL"),
    ],
};

/// ING-DiBa CSV template
const ING_DIBA: BrokerTemplate = BrokerTemplate {
    id: "ing-diba",
    name: "ING-DiBa",
    description: "ING-DiBa Depotauszug",
    detection_headers: &["Buchung", "Wertpapier", "ISIN", "Stück", "Kurs"],
    delimiter: ';',
    date_format: "%d.%m.%Y",
    decimal_separator: ',',
    mapping: CsvColumnMapping {
        date: Some(0),
        security_name: Some(1),
        isin: Some(2),
        shares: Some(3),
        amount: Some(5),
        currency: Some(6),
        txn_type: None,
        fees: None,
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("Kauf", "BUY"),
        ("Verkauf", "SELL"),
        ("Dividende", "DIVIDENDS"),
        ("Ertragsgutschrift", "DIVIDENDS"),
    ],
};

/// DKB CSV template
const DKB: BrokerTemplate = BrokerTemplate {
    id: "dkb",
    name: "DKB",
    description: "DKB Broker Export",
    detection_headers: &["Buchungstag", "Wertstellung", "Buchungstext", "Betrag"],
    delimiter: ';',
    date_format: "%d.%m.%Y",
    decimal_separator: ',',
    mapping: CsvColumnMapping {
        date: Some(0),
        txn_type: Some(2),
        amount: Some(3),
        currency: Some(4),
        security_name: None,
        isin: None,
        shares: None,
        fees: None,
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("Wertpapierkauf", "BUY"),
        ("Wertpapierverkauf", "SELL"),
        ("Dividende", "DIVIDENDS"),
        ("Gutschrift", "DEPOSIT"),
        ("Lastschrift", "REMOVAL"),
    ],
};

/// DEGIRO CSV template
const DEGIRO: BrokerTemplate = BrokerTemplate {
    id: "degiro",
    name: "DEGIRO",
    description: "DEGIRO Transaktionsübersicht",
    detection_headers: &["Datum", "Produkt", "ISIN", "Anzahl", "Kurs"],
    delimiter: ',',
    date_format: "%d-%m-%Y",
    decimal_separator: ',',
    mapping: CsvColumnMapping {
        date: Some(0),
        security_name: Some(2),
        isin: Some(3),
        shares: Some(5),
        amount: Some(7),
        currency: Some(8),
        txn_type: None,
        fees: Some(10),
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("Kauf", "BUY"),
        ("Verkauf", "SELL"),
        ("Dividende", "DIVIDENDS"),
    ],
};

/// Comdirect CSV template
const COMDIRECT: BrokerTemplate = BrokerTemplate {
    id: "comdirect",
    name: "Comdirect",
    description: "Comdirect Depotauszug",
    detection_headers: &["Buchungstag", "Geschäftsart", "WKN", "ISIN", "Stück"],
    delimiter: ';',
    date_format: "%d.%m.%Y",
    decimal_separator: ',',
    mapping: CsvColumnMapping {
        date: Some(0),
        txn_type: Some(1),
        isin: Some(3),
        shares: Some(4),
        amount: Some(6),
        currency: Some(7),
        security_name: None,
        fees: None,
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("Kauf", "BUY"),
        ("Verkauf", "SELL"),
        ("Dividende", "DIVIDENDS"),
        ("Ertragsgutschrift", "DIVIDENDS"),
    ],
};

/// Consorsbank CSV template
const CONSORSBANK: BrokerTemplate = BrokerTemplate {
    id: "consorsbank",
    name: "Consorsbank",
    description: "Consorsbank Export",
    detection_headers: &["Datum", "Umsatzart", "ISIN", "Stück", "Kurs"],
    delimiter: ';',
    date_format: "%d.%m.%Y",
    decimal_separator: ',',
    mapping: CsvColumnMapping {
        date: Some(0),
        txn_type: Some(1),
        isin: Some(2),
        shares: Some(3),
        amount: Some(5),
        currency: Some(6),
        security_name: None,
        fees: None,
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("Kauf", "BUY"),
        ("Verkauf", "SELL"),
        ("Dividende", "DIVIDENDS"),
    ],
};

/// Interactive Brokers CSV template
const INTERACTIVE_BROKERS: BrokerTemplate = BrokerTemplate {
    id: "interactive-brokers",
    name: "Interactive Brokers",
    description: "IBKR Activity Statement",
    detection_headers: &["Symbol", "Date/Time", "Quantity", "T. Price", "Proceeds"],
    delimiter: ',',
    date_format: "%Y-%m-%d",
    decimal_separator: '.',
    mapping: CsvColumnMapping {
        date: Some(1),
        security_name: Some(0),
        shares: Some(2),
        amount: Some(4),
        currency: Some(5),
        txn_type: None,
        isin: None,
        fees: Some(6),
        taxes: None,
        note: None,
    },
    type_mapping: &[
        ("BUY", "BUY"),
        ("SELL", "SELL"),
        ("DIV", "DIVIDENDS"),
    ],
};

// ============================================================================
// Template Registry
// ============================================================================

/// Get all available broker templates
pub fn get_all_templates() -> Vec<&'static BrokerTemplate> {
    vec![
        &TRADE_REPUBLIC,
        &SCALABLE_CAPITAL,
        &ING_DIBA,
        &DKB,
        &DEGIRO,
        &COMDIRECT,
        &CONSORSBANK,
        &INTERACTIVE_BROKERS,
    ]
}

/// Get a specific template by ID
pub fn get_template(id: &str) -> Option<&'static BrokerTemplate> {
    get_all_templates().into_iter().find(|t| t.id == id)
}

/// Detect broker from CSV headers
pub fn detect_broker(headers: &[String]) -> BrokerDetectionResult {
    let headers_lower: Vec<String> = headers.iter().map(|h| h.to_lowercase()).collect();

    let mut best_match: Option<(&BrokerTemplate, f32)> = None;

    for template in get_all_templates() {
        let matched_count = template
            .detection_headers
            .iter()
            .filter(|dh| headers_lower.iter().any(|h| h.contains(&dh.to_lowercase())))
            .count();

        let confidence = matched_count as f32 / template.detection_headers.len() as f32;

        if confidence > 0.0 {
            if best_match.is_none() || confidence > best_match.unwrap().1 {
                best_match = Some((template, confidence));
            }
        }
    }

    match best_match {
        Some((template, confidence)) => BrokerDetectionResult {
            template_id: Some(template.id.to_string()),
            broker_name: template.name.to_string(),
            confidence,
            detected_headers: headers.to_vec(),
        },
        None => BrokerDetectionResult {
            template_id: None,
            broker_name: "Unbekannt".to_string(),
            confidence: 0.0,
            detected_headers: headers.to_vec(),
        },
    }
}

