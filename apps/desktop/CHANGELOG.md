# Changelog

Alle nennenswerten Änderungen an diesem Projekt werden in dieser Datei dokumentiert.

Das Format basiert auf [Keep a Changelog](https://keepachangelog.com/de/1.0.0/),
und dieses Projekt hält sich an [Semantic Versioning](https://semver.org/lang/de/).

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
