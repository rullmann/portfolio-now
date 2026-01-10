# Portfolio Performance Import/Export

Detaillierte Dokumentation des PP-Dateiformats und der Round-Trip-Kompatibilität.

## Dateiformat (.portfolio)

```
┌─────────────────────────────────────┐
│  ZIP-Archiv (.portfolio)            │
│  └── data.portfolio                 │
│      ├── Header: "PPPBV1" (6 Bytes) │
│      └── Body: Protocol Buffers     │
└─────────────────────────────────────┘
```

### Protobuf Schema (PClient)

```protobuf
message PClient {
  int32 version = 1;              // z.B. 68
  repeated PSecurity securities = 2;
  repeated PAccount accounts = 3;
  repeated PPortfolio portfolios = 4;
  repeated PTransaction transactions = 5;
  repeated PInvestmentPlan plans = 6;
  repeated PWatchlist watchlists = 7;
  repeated PTaxonomy taxonomies = 8;
  repeated PDashboard dashboards = 9;
  repeated PProperty properties = 10;
  PSettings settings = 11;
  string base_currency = 12;
}
```

---

## Skalierungsfaktoren

| Wert | Faktor | Rust-Typ | Beispiel |
|------|--------|----------|----------|
| **Shares** | 10^8 | `i64` | 1.5 Stück = `150_000_000` |
| **Amount** | 10^2 | `i64` | 100.50 EUR = `10050` |
| **Prices** | 10^8 | `i64` | 150.25 = `15_025_000_000` |
| **Weights** | 10^4 | `i32` | 100% = `10000` |

### Konvertierungsfunktionen

```rust
// pp/common.rs
pub mod shares {
    pub fn to_decimal(scaled: i64) -> f64 { scaled as f64 / 100_000_000.0 }
    pub fn from_decimal(decimal: f64) -> i64 { (decimal * 100_000_000.0).round() as i64 }
}

pub mod prices {
    pub fn to_decimal(scaled: i64) -> f64 { scaled as f64 / 100_000_000.0 }
    pub fn from_decimal(decimal: f64) -> i64 { (decimal * 100_000_000.0).round() as i64 }
}
```

---

## Import-Prozess

### Ablauf (`commands/import.rs`)

```
1. parse_portfolio_file(path)     → Client (Rust-Modell)
2. save_client_to_db(client)      → SQLite mit Progress-Events
   ├── insert_securities()
   ├── insert_accounts()
   ├── insert_portfolios()
   ├── insert_transactions()      → mit CrossEntry-Verknüpfung
   ├── insert_investment_plans()
   ├── insert_watchlists()
   ├── insert_taxonomies()
   ├── insert_dashboards()
   └── insert_settings()
3. process_fifo_lots()            → FIFO Cost Basis berechnen
```

### Progress-Events

```typescript
// Frontend empfängt Events via Tauri
listen('import-progress', (event) => {
  const { step, progress, message } = event.payload;
  // step: 'parsing' | 'securities' | 'accounts' | 'transactions' | 'fifo'
  // progress: 0-100
});
```

---

## Datenbank-Schema

### Kerntabellen

```sql
-- Securities mit Attributes
CREATE TABLE pp_security (
    id INTEGER PRIMARY KEY,
    import_id INTEGER,
    uuid TEXT UNIQUE,
    name TEXT,
    currency TEXT,
    isin TEXT,
    wkn TEXT,
    ticker TEXT,
    feed TEXT,
    feed_url TEXT,
    latest_feed TEXT,
    latest_feed_url TEXT,
    is_retired INTEGER DEFAULT 0,
    note TEXT,
    custom_logo TEXT,           -- Base64-encoded
    attributes TEXT,            -- JSON: {"key": "value", ...}
    updated_at TEXT
);

-- Accounts mit Attributes
CREATE TABLE pp_account (
    id INTEGER PRIMARY KEY,
    import_id INTEGER,
    uuid TEXT UNIQUE,
    name TEXT,
    currency TEXT,
    note TEXT,
    is_retired INTEGER DEFAULT 0,
    attributes TEXT,            -- JSON
    updated_at TEXT
);

-- Portfolios mit Attributes
CREATE TABLE pp_portfolio (
    id INTEGER PRIMARY KEY,
    import_id INTEGER,
    uuid TEXT UNIQUE,
    name TEXT,
    reference_account_id INTEGER,
    note TEXT,
    is_retired INTEGER DEFAULT 0,
    attributes TEXT,            -- JSON
    updated_at TEXT
);

-- Transactions mit Transfer-Tracking
CREATE TABLE pp_txn (
    id INTEGER PRIMARY KEY,
    import_id INTEGER,
    uuid TEXT UNIQUE,
    owner_type TEXT,            -- 'account' | 'portfolio'
    owner_id INTEGER,
    security_id INTEGER,
    txn_type TEXT,
    date TEXT,
    amount INTEGER,             -- × 10^2
    currency TEXT,
    shares INTEGER,             -- × 10^8
    note TEXT,
    source TEXT,
    other_account_id INTEGER,   -- Ziel-Account bei Transfers
    other_portfolio_id INTEGER, -- Ziel-Portfolio bei Transfers
    updated_at TEXT
);

-- Investment Plans (erweitert)
CREATE TABLE pp_investment_plan (
    id INTEGER PRIMARY KEY,
    import_id INTEGER,
    uuid TEXT,
    name TEXT,
    security_id INTEGER,
    portfolio_id INTEGER,
    account_id INTEGER,
    amount INTEGER,             -- × 10^2
    fees INTEGER,               -- × 10^2
    taxes INTEGER,              -- × 10^2
    interval INTEGER,           -- <100 = Monate, >100 = Wochen
    start_date TEXT,
    auto_generate INTEGER,
    plan_type INTEGER,          -- 0=PURCHASE, 1=DEPOSIT, 2=REMOVAL, 3=INTEREST
    note TEXT,
    attributes TEXT             -- JSON
);

-- Dashboards
CREATE TABLE pp_dashboard (
    id INTEGER PRIMARY KEY,
    import_id INTEGER,
    dashboard_id TEXT,
    name TEXT NOT NULL,
    columns_json TEXT,          -- JSON: [{weight, widgets: [...]}]
    configuration_json TEXT
);

-- Settings
CREATE TABLE pp_settings (
    id INTEGER PRIMARY KEY,
    import_id INTEGER UNIQUE,
    settings_json TEXT          -- JSON: {bookmarks, attributeTypes, configurationSets}
);

-- Client Properties
CREATE TABLE pp_client_properties (
    id INTEGER PRIMARY KEY,
    import_id INTEGER,
    key TEXT,
    value TEXT
);
```

---

## Export-Prozess

### Ablauf (`protobuf/writer.rs`)

```
1. convert_to_protobuf(client)    → PClient (Protobuf-Modell)
   ├── convert_security()         → mit attributes, events
   ├── convert_account()          → mit attributes
   ├── convert_portfolio()        → mit attributes
   ├── convert_*_transaction()    → mit other_*_uuid
   ├── convert_investment_plan()  → mit fees, taxes, plan_type
   ├── convert_taxonomy()
   ├── convert_dashboard()
   └── convert_settings_to_protobuf()
2. serialize_client()             → Bytes mit Header
3. write_portfolio_file()         → ZIP-Archiv
```

### Attributes-Konvertierung

```rust
/// HashMap → Protobuf PKeyValue
fn convert_attributes_to_protobuf(
    attrs: &HashMap<String, String>,
) -> Vec<PKeyValue> {
    attrs.iter().map(|(key, value)| PKeyValue {
        key: key.clone(),
        value: Some(PAnyValue {
            kind: Some(PAnyValueKind::String(value.clone())),
        }),
    }).collect()
}
```

### Settings-Konvertierung

```rust
/// JSON → Protobuf PSettings
fn convert_settings_to_protobuf(settings: &serde_json::Value) -> Option<PSettings> {
    // Konvertiert: bookmarks, attribute_types, configuration_sets
}
```

---

## Transaktionstypen

### Portfolio-Transaktionen

| Protobuf | Rust | SQL | Beschreibung |
|----------|------|-----|--------------|
| `PURCHASE` (0) | `Buy` | `BUY` | Kauf über Account |
| `SALE` (1) | `Sell` | `SELL` | Verkauf über Account |
| `INBOUND_DELIVERY` (2) | `DeliveryInbound` | `DELIVERY_INBOUND` | Einlieferung |
| `OUTBOUND_DELIVERY` (3) | `DeliveryOutbound` | `DELIVERY_OUTBOUND` | Auslieferung |
| `SECURITY_TRANSFER` (4) | `TransferIn/Out` | `TRANSFER_IN/OUT` | Portfolio-Transfer |

### Account-Transaktionen

| Protobuf | Rust | SQL | Beschreibung |
|----------|------|-----|--------------|
| `DEPOSIT` (6) | `Deposit` | `DEPOSIT` | Einzahlung |
| `REMOVAL` (7) | `Removal` | `REMOVAL` | Auszahlung |
| `DIVIDEND` (8) | `Dividends` | `DIVIDENDS` | Dividende |
| `INTEREST` (9) | `Interest` | `INTEREST` | Zinsen |
| `INTEREST_CHARGE` (10) | `InterestCharge` | `INTEREST_CHARGE` | Zinsbelastung |
| `TAX` (11) | `Taxes` | `TAXES` | Steuer |
| `TAX_REFUND` (12) | `TaxRefund` | `TAX_REFUND` | Steuerrückerstattung |
| `FEE` (13) | `Fees` | `FEES` | Gebühr |
| `FEE_REFUND` (14) | `FeesRefund` | `FEES_REFUND` | Gebührenrückerstattung |
| `CASH_TRANSFER` (5) | `TransferIn/Out` | `TRANSFER_IN/OUT` | Konto-Transfer |

---

## CrossEntry-Mechanismus

Verknüpft zusammengehörige Transaktionen.

```sql
CREATE TABLE pp_cross_entry (
    id INTEGER PRIMARY KEY,
    entry_type TEXT,          -- 'BUY_SELL' | 'PORTFOLIO_TRANSFER' | 'ACCOUNT_TRANSFER'
    from_txn_id INTEGER,      -- Quell-Transaktion
    to_txn_id INTEGER,        -- Ziel-Transaktion
    portfolio_txn_id INTEGER, -- Portfolio-Seite (bei BUY_SELL)
    account_txn_id INTEGER    -- Account-Seite (bei BUY_SELL)
);
```

| entry_type | Verknüpfung |
|------------|-------------|
| `BUY_SELL` | Portfolio-BUY ↔ Account-BUY |
| `PORTFOLIO_TRANSFER` | TRANSFER_OUT ↔ TRANSFER_IN |
| `ACCOUNT_TRANSFER` | TRANSFER_OUT ↔ TRANSFER_IN |

---

## Transfer-Tracking

Transaktionen speichern das Ziel direkt:

```rust
// pp/transaction.rs
pub struct AccountTransaction {
    // ...
    pub other_account_uuid: Option<String>,  // Ziel bei TRANSFER_OUT
}

pub struct PortfolioTransaction {
    // ...
    pub other_portfolio_uuid: Option<String>,  // Ziel bei TRANSFER_OUT
}
```

Import-Auflösung:

```rust
// commands/import.rs
if let Some(ref other_uuid) = tx.other_account_uuid {
    let other_id = tx_conn.query_row(
        "SELECT id FROM pp_account WHERE uuid = ?1",
        [other_uuid],
        |row| row.get(0)
    ).ok();
    // Speichern in pp_txn.other_account_id
}
```

---

## Investment Plans

### Intervall-Kodierung

| Wert | Bedeutung |
|------|-----------|
| `1` | Monatlich |
| `3` | Quartalsweise |
| `6` | Halbjährlich |
| `12` | Jährlich |
| `101` | Wöchentlich |
| `102` | Zweiwöchentlich |

### Plan-Typen

| Wert | Typ | Beschreibung |
|------|-----|--------------|
| `0` | `PURCHASE_OR_DELIVERY` | Kauf/Einlieferung |
| `1` | `DEPOSIT` | Einzahlung |
| `2` | `REMOVAL` | Auszahlung |
| `3` | `INTEREST` | Zinsen |

---

## Dashboards & Settings

### Dashboard-Struktur

```json
{
  "name": "Übersicht",
  "id": "dashboard-uuid",
  "columns": [
    {
      "weight": 50,
      "widgets": [
        {
          "widget_type": "chart.pie",
          "label": "Asset Allocation",
          "configuration": {}
        }
      ]
    }
  ]
}
```

### Settings-Struktur

```json
{
  "bookmarks": [
    {"label": "Google", "pattern": "https://google.com/search?q={ticker}"}
  ],
  "attributeTypes": [
    {
      "id": "attr-uuid",
      "name": "Sector",
      "columnLabel": "Sektor",
      "source": "SECURITY",
      "target": "STRING",
      "type": "STRING"
    }
  ],
  "configurationSets": [
    {"key": "views", "uuid": "...", "name": "Default", "data": "..."}
  ]
}
```

---

## Validierung nach Import

```sql
-- Holdings pro Portfolio (KRITISCH: nicht FIFO-Lots verwenden!)
SELECT p.name, s.name, SUM(CASE
    WHEN t.txn_type IN ('BUY','TRANSFER_IN','DELIVERY_INBOUND') THEN t.shares
    WHEN t.txn_type IN ('SELL','TRANSFER_OUT','DELIVERY_OUTBOUND') THEN -t.shares
END) / 100000000.0 as shares
FROM pp_txn t
JOIN pp_portfolio p ON p.id = t.owner_id
JOIN pp_security s ON s.id = t.security_id
WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
GROUP BY p.id, s.id HAVING shares > 0;

-- CrossEntry-Verknüpfungen
SELECT entry_type, COUNT(*) FROM pp_cross_entry GROUP BY entry_type;

-- Attributes prüfen
SELECT name, json_extract(attributes, '$') as attrs
FROM pp_security WHERE attributes IS NOT NULL LIMIT 5;

-- Investment Plans mit erweiterten Feldern
SELECT name, fees/100.0, taxes/100.0, plan_type, note
FROM pp_investment_plan;

-- Dashboards
SELECT name, dashboard_id, json_array_length(columns_json) as cols
FROM pp_dashboard;

-- Settings
SELECT json_extract(settings_json, '$.bookmarks') FROM pp_settings;
```

---

## Bekannte Einschränkungen

1. **Security Events**: Werden importiert aber nicht vollständig exportiert (Dividend-Details)
2. **Widget Configuration**: Wird als Bytes gespeichert, nicht vollständig interpretiert
3. **Properties**: Nur String-Werte werden exportiert
4. **Taxonomy Data**: Verschachtelte Classification-Data wird vereinfacht

---

## Fehlerbehebung

### FIFO-Lots neu berechnen

```typescript
import { invoke } from '@tauri-apps/api/core';
const result = await invoke('rebuild_fifo_lots');
console.log(`${result.securitiesProcessed} Securities, ${result.lotsCreated} Lots`);
```

### Import-Fehler debuggen

```rust
// In parser.rs aktivieren für Raw-Protobuf-Dump
#[test]
fn test_dump_raw_protobuf_values() {
    let path = "/path/to/file.portfolio";
    // Gibt alle Protobuf-Felder aus
}
```
