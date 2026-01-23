# Changelog

Alle nennenswerten Änderungen an diesem Projekt werden in dieser Datei dokumentiert.

Das Format basiert auf [Keep a Changelog](https://keepachangelog.com/de/1.0.0/),
und dieses Projekt hält sich an [Semantic Versioning](https://semver.org/lang/de/).

## [0.1.5] - 2026-01-23

### Hinzugefügt

#### ChatBot Transaktionserstellung
Der Portfolio-ChatBot kann jetzt Transaktionen über natürliche Sprache erstellen:

**Unterstützte Transaktionstypen:**
- **Kauf/Verkauf (BUY/SELL)** - Mit Depot, Wertpapier, Stückzahl, Betrag
- **Einlieferung/Auslieferung (DELIVERY_INBOUND/OUTBOUND)** - Depotüberträge ohne Gegenwert
- **Dividende (DIVIDENDS)** - Konto, Wertpapier, Betrag
- **Einlage/Entnahme (DEPOSIT/REMOVAL)** - Kontobewegungen
- **Depotwechsel (PORTFOLIO_TRANSFER)** - Aktien von Depot A nach Depot B

**Multi-Step Konversation:**
Der ChatBot sammelt alle benötigten Daten durch natürlichen Dialog:
```
User: "Buche einen Kauf von Apple"
AI: "In welchem Depot? - Hauptdepot (ID: 1) - Zweitdepot (ID: 2)"
User: "Hauptdepot, 10 Stück zu 180 Euro am 15.01.2026"
AI: "Sollen Gebühren erfasst werden?"
User: "1 Euro Gebühren"
AI: [Zeigt Transaktions-Vorschau zur Bestätigung]
```

**Sicherheit:**
- Transaktionen werden IMMER als Vorschläge zurückgegeben
- Detaillierte Transaktionsvorschau mit allen Feldern
- Explizite Benutzerbestätigung erforderlich (Bestätigen/Abbrechen)
- Doppelte Validierung (Frontend + Backend)
- FIFO-Lots werden automatisch neu berechnet

**Neue Command-Patterns:**
- `[[TRANSACTION_CREATE:{...}]]` - Einzelne Transaktion
- `[[PORTFOLIO_TRANSFER:{...}]]` - Depotwechsel (erzeugt 2 Transaktionen)

**Neue Tauri Commands:**
- `execute_confirmed_transaction` - Führt bestätigte Transaktion aus
- `execute_confirmed_portfolio_transfer` - Führt Depotwechsel aus

**Neue UI-Komponente:**
- `TransactionConfirmation` in ChatPanel.tsx - Detaillierte Vorschau mit Bestätigungs-Buttons

**Dateien:**
- `src-tauri/src/ai/types.rs` - `TransactionCreateCommand`, `PortfolioTransferCommand`
- `src-tauri/src/ai/command_parser.rs` - Transaction-Command-Parsing
- `src-tauri/src/ai/prompts.rs` - Erweiterte System-Prompts
- `src-tauri/src/commands/ai.rs` - Neue Execute-Commands
- `src/lib/types.ts` - Frontend-Types
- `src/components/chat/ChatPanel.tsx` - TransactionConfirmation-Komponente

### Behoben

#### Fees-Bug bei DEPOSIT/REMOVAL
- AI fügte bei Einlagen/Entnahmen ungefragt Gebühren hinzu
- Neue explizite Regel in System-Prompt: "DEPOSIT/REMOVAL: NIEMALS Gebühren oder Steuern hinzufügen!"
- Klare Beispiele für korrektes Verhalten im Prompt

### Geändert

#### Einheitliche Bestätigungs-UI im ChatBot
- **Vorher**: Watchlist-Aktionen hatten kleine Icon-only Buttons (✓/✗)
- **Nachher**: Alle Bestätigungen (Watchlist, Transaktionen, Transfers) nutzen das gleiche UI-Pattern:
  - Amber Container mit Header-Icon
  - Beschreibung der Aktion
  - Zwei vollbreite Buttons: "Bestätigen" (grün) / "Abbrechen" (muted)
- Entfernt: `declineAllSuggestions()` Funktion (unbenutzt nach UI-Refactoring)

---

## [0.1.4] - 2026-01-20

### Hinzugefügt

#### ChatBot Datenbank-Integration
Der Portfolio-ChatBot hat jetzt vollständigen Zugriff auf die SQLite-Datenbank mit 13 spezialisierten Query-Templates:

**Basis-Abfragen:**
- `security_transactions` - Alle Transaktionen für ein Wertpapier (nach Name/ISIN/Ticker)
- `dividends_by_security` - Dividenden für ein Wertpapier
- `all_dividends` - Alle Dividenden gruppiert (mit Jahr-Filter)
- `transactions_by_date` - Transaktionen in Zeitraum
- `security_cost_basis` - FIFO-Lots und Einstandskurse
- `sold_securities` - Verkaufte/geschlossene Positionen

**Erweiterte Abfragen:**
- `holding_period_analysis` - Haltefrist-Analyse für Krypto/Gold (§ 23 EStG)
- `fifo_lot_details` - Detaillierte FIFO-Lots mit Haltetagen und Tax-Status
- `account_transactions` - Kontobewegungen (Einzahlungen, Auszahlungen, Dividenden)
- `investment_plans` - Alle Sparpläne
- `portfolio_accounts` - Konten mit aktuellen Salden
- `tax_relevant_sales` - Verkäufe mit Haltefrist und Steuerstatus

**Account Balance Analysis:**
- `account_balance_analysis` - **NEU**: Analysiert woher ein Kontostand kommt
- Running Balance Berechnung mit Window Functions
- Korrekte Reihenfolge: INFLOWS vor OUTFLOWS am gleichen Tag
- Ausgabe mit `[AKTUELLER SALDO]` Marker

**Beispiel-Frage:** "Woher kommen die 25 Cent auf dem Referenzkonto?"
```
→ • 02.10.2025 Dividende +0,25 EUR → Saldo: 0,25 EUR | NVIDIA [AKTUELLER SALDO: 0,25 EUR]
  • 03.07.2025 Auszahlung -0,22 EUR → Saldo: 0,00 EUR
  • 03.07.2025 Dividende +0,22 EUR → Saldo: 0,22 EUR | NVIDIA
```

**Dateien:**
- `src-tauri/src/ai/query_templates.rs` - Query-Templates und Formatierung
- `src-tauri/src/ai/command_parser.rs` - `[[QUERY_DB:...]]` Command-Parsing
- `src-tauri/src/ai/prompts.rs` - Erweiterte System-Prompts

#### 29 neue Bank-Parser für PDF-Import
Erweiterte Unterstützung für Bank-Dokumente aus Deutschland, Schweiz, Österreich und international.

**Deutschland (16):**
- Baader Bank, Commerzbank, DAB, Deutsche Bank, DZ Bank
- ebase, flatex, GenoBroker, MLP Bank, OLB
- OnVista, Postbank, Quirion, S Broker, Santander, Targobank

**Schweiz (6):**
- Credit Suisse, LGT, PostFinance, Swissquote, UBS, ZKB

**Österreich (2):**
- Erste Bank, Raiffeisen

**International (5):**
- DEGIRO, Merkur, Revolut, Saxo Bank, 1822direkt

#### AI Feature Matrix
- **Feature-spezifische KI-Konfiguration**: Jedes KI-Feature kann einen eigenen Provider und Modell haben
- Konfigurierbare Features: Chart-Analyse, Portfolio Insights, Chat, PDF OCR, CSV-Import
- Neue Komponente: `AiFeatureMatrix.tsx` in Settings
- Store-Erweiterung: `aiFeatureSettings` mit per-Feature Provider/Model

#### AI Migration Modal
- **Automatische Modell-Migration**: Erkennt deprecated Modelle beim App-Start
- User-Benachrichtigung mit altem und neuem Modell
- Manuelles Bestätigen oder Ablehnen der Migration
- Info-Toast bei neuen verfügbaren Modellen

#### Header KI-Dropdown erweitert
Alle 5 KI-Features jetzt im Header-Dropdown verfügbar:
- Portfolio Insights
- Nachkauf-Chancen
- Chat öffnen
- **Chart-Analyse** (navigiert zur Charts-View)
- **PDF OCR** (öffnet Modal)
- **CSV-Import** (öffnet Modal)

#### PDF OCR Aktivitätsanzeige
Verbesserte visuelle Rückmeldung während KI-OCR:
- **Provider-Logo** mit pulsierendem Indikator
- Anzeige von Provider-Name und Modell
- **Fortschrittsbalken** bei mehreren Dateien (X von Y)
- Provider-spezifischer Hinweis:
  - Claude/Gemini: "Direkter PDF-Upload (schneller)"
  - OpenAI/Perplexity: "PDF → Bilder → Vision API"

#### E2E-Tests
- **Playwright-Konfiguration** für Tauri App Testing
- **WebDriverIO-Setup** als Alternative
- 20+ E2E-Test-Specs für alle Views
- Page Object Models für Dashboard
- Test-Utilities und Tauri-Mocks

### Behoben

- **Dashboard AiFeaturesCard**: Nur 3 von 5 Features sichtbar
  - CSS-Fix: `max-h-[140px] overflow-y-auto` für scrollbare Feature-Liste

### Geändert

- **Secure API Keys Hook**: Refactored für besseres Error Handling
- **AIAnalysisPanel**: Verbesserte Analyse-Darstellung
- **ChatPanel**: Optimiertes Message Handling
- **CsvImportModal**: Bessere AI-Integration
- **PortfolioInsightsModal**: Verfeinerte Insights-Anzeige

### Dokumentation

- CHANGELOG.md aktualisiert mit Version 0.1.4
- CLAUDE.md erweitert um:
  - AI Feature Matrix Dokumentation
  - Erweiterte Bank-Parser Liste
  - Header KI-Dropdown Beschreibung

---

## [0.1.3] - 2026-01-17

### Hinzugefügt

#### PDF-Export Verbesserungen
- **Professionelles Design**: Komplette Neugestaltung der PDF-Dokumente
  - Farbige Header mit Akzentlinie
  - Fußzeile mit App-Name und Seitenzahlen
  - Zebra-Streifen für bessere Lesbarkeit
  - Farbcodierte Werte (grün=positiv, rot=negativ)
  - Summary-Boxen mit Hintergrund
- **Deutsche Zahlenformatierung**: Tausender-Trennzeichen mit Punkt, Dezimal mit Komma
- **Pfadvalidierung**: Alle PDF-Export-Funktionen nutzen jetzt `validate_file_path_with_extension()`

#### Datumsformat-Standardisierung
- **dd.MM.yyyy Format**: Einheitliches deutsches Datumsformat in der gesamten App
- Neue Funktionen: `formatDate()`, `formatDateTime()`, `formatDateShort()`
- Uhrzeiten nur wo relevant (Alerts, Pattern-Trigger)

### Behoben

- **PDF-Export Dividenden**: Falscher Command-Name (`export_dividend_report_pdf` → `export_dividend_pdf`) und Parameter (`startDate/endDate` → `year`) korrigiert
- **Analyse löschen**: Button löscht jetzt auch Analysetext, Trendinfo, Alerts und Risk/Reward (nicht nur Marker)
- **Button-Beschriftung**: "Marker löschen" → "Analyse löschen" umbenannt

### Geändert

- **AIAnalysisPanel**: `clearAllAnnotations()` → `clearAnalysis()` refactored
- **pdf_export.rs**: Komplett neu geschrieben mit Farbkonstanten, Layout-Helpern
- **types.ts**: Erweiterte Datums-Formatierungsfunktionen
- Dateien mit aktualisiertem Datumsformat: Securities, Transactions, Benchmark, SignalsPanel, AlertsPanel, MergerModal, StockSplitModal, SecurityPriceModal

### Entfernt

- Unnötige Dateien: `.DS_Store`, leere `portfolio.db` Dateien, `react.svg`

---

## [0.1.2] - 2026-01-16

### Hinzugefügt

#### CSV-Import mit Broker-Templates
- **Broker-Erkennung**: Automatische Erkennung des CSV-Formats anhand der Header
- **8 Broker-Templates**: Trade Republic, Scalable Capital, ING-DiBa, DKB, Comdirect, Consorsbank, DEGIRO, Interactive Brokers
- **AI-Fallback**: KI-Analyse für unbekannte CSV-Formate (Code-first, AI-fallback Prinzip)
- **Import-Wizard**: Mehrstufiger Dialog mit Vorschau und Spalten-Mapping
- Neues Modul: `src-tauri/src/csv_import/`

#### AI-Assistent im Header
- **Klickbares AI-Badge**: Provider-Logo und Modell im Header sind jetzt klickbar
- **Dropdown-Menü** mit:
  - Portfolio Insights (startet Analyse direkt)
  - Nachkauf-Chancen (startet Opportunity-Analyse direkt)
  - Chat öffnen
  - View-spezifische Aktionen (z.B. "Diversifikation prüfen" bei Holdings)
- `initialMode` Prop für PortfolioInsightsModal zum direkten Start

### Behoben

- **GPT-5 Responses API**: `content_type` Filter korrigiert ("text" statt "output_text")
- Portfolio Insights mit GPT-5 zeigen jetzt korrekt Ergebnisse an

### Geändert

- Header zeigt Chevron-Icon am AI-Badge für bessere Affordance
- PortfolioInsightsModal unterstützt jetzt `initialMode` Prop

---

## [0.1.1] - 2026-01-14

### Sicherheit

#### Secure Storage für API-Keys
- **Tauri Plugin Store**: API-Keys werden jetzt sicher im App-Datenverzeichnis gespeichert statt im localStorage
- **Automatische Migration**: Bestehende Keys werden beim ersten Start migriert
- **useSecureApiKeys Hook**: React Hook für sichere Key-Verwaltung im Frontend
- Shield-Icon in Einstellungen zeigt sichere Speicherung an

#### Security-Modul (Backend)
- **Pfadvalidierung**: `validate_file_path()` verhindert Directory Traversal Angriffe
- **Rate Limiting**: `check_rate_limit()` für API-Aufrufe (vorbereitet)
- **Input Sanitization**: `sanitize_string()`, `sanitize_filename()` für sichere Eingaben
- Neues Modul: `src-tauri/src/security/mod.rs`

#### AI Command Security
- **Suggestions statt Auto-Execution**: Watchlist-Aktionen vom ChatBot erfordern jetzt User-Bestätigung
- Gelber Hinweisbereich zeigt ausstehende Aktionen
- Einzelne Bestätigung/Ablehnung pro Aktion
- `parse_response_with_suggestions()` ersetzt `execute_watchlist_commands()`

#### PDF-Import Consent
- **OCR Consent Dialog**: Explizite Zustimmung vor Upload an KI-Provider
- Informiert über Datenübertragung und Ziel-Service
- `ocrConsentGiven` Flag muss gesetzt sein

#### Capabilities & Permissions
- `store:default` Permission für Secure Storage hinzugefügt
- Dokumentierte Security-Hinweise in `capabilities/default.json`

### Hinzugefügt

#### Bulk Delete für Transaktionen
- Mehrfachauswahl mit Checkboxen
- `BulkDeleteConfirmModal` mit Bestätigungsdialog
- Anzeige der zu löschenden Transaktionen vor Ausführung

### Geändert

- **CLAUDE.md**: Security-First Regeln und Code-Hygiene Pflichten hinzugefügt
- **Store**: API-Keys werden nicht mehr in localStorage persistiert
- **ChatPanel**: Suggestions-UI für Watchlist-Aktionen

### Entfernt

- `execute_watchlist_commands()` - ersetzt durch Suggestions-System
- Ungenutzte Imports und Variablen (Code-Hygiene)
- Module-Level `#![allow(dead_code)]` - ersetzt durch gezielte Annotationen

---

## [0.1.0] - 2026-01-13

### Hinzugefügt

#### Technische Analyse - Indikatoren
- **Stochastic Oscillator**: %K und %D Linien mit konfigurierbaren Perioden
- **On-Balance Volume (OBV)**: Volumen-basierter Momentum-Indikator
- **ADX (Average Directional Index)**: Trendstärke mit +DI/-DI Linien
- **ATR (Average True Range)**: Volatilitäts-Messung
- **VWAP (Volume Weighted Average Price)**: Volumengewichteter Durchschnittspreis

#### Candlestick-Pattern-Erkennung
- Neue `patterns.ts` Bibliothek mit umfassender Pattern-Erkennung
- **Single Candle**: Doji, Hammer, Inverted Hammer, Hanging Man, Shooting Star, Spinning Top, Marubozu
- **Two Candle**: Bullish/Bearish Engulfing, Harami, Piercing Line, Dark Cloud Cover, Tweezer Top/Bottom
- **Three Candle**: Morning Star, Evening Star, Three White Soldiers, Three Black Crows, Three Inside Up/Down
- Automatische Trend-Erkennung für kontextabhängige Patterns
- Pattern-Anzeige im SignalsPanel

#### Zeichenwerkzeuge (Drawing Tools)
- Neue `DrawingTools.tsx` Komponente mit Canvas-Overlay
- **Trendlinien**: Zwei-Punkt-Linien für Trend-Analyse
- **Horizontale Linien**: Support/Resistance-Level markieren
- **Fibonacci Retracements**: Automatische Level (0%, 23.6%, 38.2%, 50%, 61.8%, 78.6%, 100%)
- Farbcodierte Fibonacci-Level
- Toolbar mit Werkzeug-Auswahl
- "Zeichnen" Toggle-Button im Chart-Header
- Persistente Speicherung in SQLite (`pp_chart_drawing` Tabelle)

#### Pattern-Tracking
- Neue `pp_pattern_history` Datenbank-Tabelle
- Speicherung erkannter Patterns mit Preis und Zeitstempel
- Automatische Evaluierung nach 5 und 10 Tagen
- Erfolgsquoten-Statistiken pro Pattern-Typ
- Tauri Commands: `save_pattern_detection`, `evaluate_pattern_outcomes`, `get_pattern_statistics`, `get_pattern_history`

#### KI-Verbesserungen
- **Web-Kontext**: News-Integration für Perplexity-Modelle
- "📰 News" Toggle-Button im AIAnalysisPanel
- Automatische Capability-Erkennung via `getModelCapabilities()`
- Erweiterte Prompts mit aktuellen Nachrichten, Earnings, Analysteneinschätzungen

#### Dokumentation
- Neue `README.md` mit vollständiger Feature-Übersicht
- Neue `TECHNICAL_ANALYSIS.md` mit detaillierter TA-Dokumentation
- Neue `CHANGELOG.md` (diese Datei)

#### Tests
- Neue `patterns.test.ts` mit 150+ Unit Tests
- Umfassende Tests für alle Candlestick-Patterns
- Edge-Case-Tests (flacher Markt, extreme Werte, Lücken)
- Alle Tests bestanden

### Geändert

- `indicators.ts`: Erweitert um neue Indikatoren
- `TradingViewChart.tsx`: Integration der Zeichenwerkzeuge
- `Charts/index.tsx`: "Zeichnen" Button und Drawing-State
- `AIAnalysisPanel.tsx`: Web-Kontext Toggle und Capability-Check
- `src-tauri/src/ai/mod.rs`: Erweiterte Prompts mit Web-Recherche
- `src-tauri/src/db/mod.rs`: Neue Tabellen für Drawings und Pattern-History

### Rust Backend

#### Neue Commands
```rust
// Zeichnungen
commands::drawings::save_chart_drawing
commands::drawings::get_chart_drawings
commands::drawings::delete_chart_drawing
commands::drawings::clear_chart_drawings

// Pattern-Tracking
commands::patterns::save_pattern_detection
commands::patterns::evaluate_pattern_outcomes
commands::patterns::get_pattern_statistics
commands::patterns::get_pattern_history
```

#### Neue Module
- `src-tauri/src/commands/drawings.rs`
- `src-tauri/src/commands/patterns.rs`

### Datenbank-Schema

#### Neue Tabellen

```sql
-- Zeichnungen
CREATE TABLE pp_chart_drawing (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    security_id INTEGER NOT NULL,
    drawing_type TEXT NOT NULL,
    points_json TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#2563eb',
    line_width INTEGER NOT NULL DEFAULT 2,
    fib_levels_json TEXT,
    is_visible INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

-- Pattern-Historie
CREATE TABLE pp_pattern_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    security_id INTEGER NOT NULL,
    pattern_type TEXT NOT NULL,
    detected_at TEXT NOT NULL,
    price_at_detection REAL NOT NULL,
    predicted_direction TEXT NOT NULL,
    actual_outcome TEXT,
    price_after_5d REAL,
    price_after_10d REAL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### Behoben

- TypeScript-Fehler in `DrawingTools.tsx` (unbenutzte Imports)
- Pattern-Test-Daten angepasst für korrekte Trend-Erkennung
- Shooting Star Test mit korrektem Close-Preis für Aufwärtstrend

---

## [0.0.x] - Frühere Versionen

### Basis-Features
- Portfolio Performance Import/Export
- Dashboard mit Performance-Übersicht
- Holdings-Verwaltung
- Transaktions-Tracking
- FIFO-Kostenbasis
- Dividenden-Reports
- Steuer-Reports
- Watchlists
- Taxonomien
- Investment-Pläne
- Rebalancing
- Benchmark-Vergleich
- ChatBot
- Portfolio Insights
- Chart-Analyse mit KI
- PDF-Import mit OCR
- Corporate Actions (Splits, Spin-Offs)
