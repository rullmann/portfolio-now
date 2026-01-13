# Changelog

Alle nennenswerten Änderungen an diesem Projekt werden in dieser Datei dokumentiert.

Das Format basiert auf [Keep a Changelog](https://keepachangelog.com/de/1.0.0/),
und dieses Projekt hält sich an [Semantic Versioning](https://semver.org/lang/de/).

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
