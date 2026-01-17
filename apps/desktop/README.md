# Portfolio Now

Cross-Platform Desktop-App zur Portfolio-Verwaltung. Moderne Neuimplementierung von [Portfolio Performance](https://github.com/portfolio-performance/portfolio) mit Tauri (Rust + React/TypeScript).

![Version](https://img.shields.io/badge/version-0.1.3-blue)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)
![License](https://img.shields.io/badge/license-TBD-orange)

## Features

### Portfolio-Management
- Import/Export von Portfolio Performance `.portfolio` Dateien (vollständiger Round-Trip)
- Mehrere Portfolios und Konten verwalten
- FIFO-basierte Einstandsberechnung mit Cost-Basis-Historie
- Automatische Kursaktualisierung (Yahoo, Finnhub, CoinGecko, EZB, Portfolio Report)
- Dividenden-Tracking und Steuerreports
- Taxonomien zur Kategorisierung (Regionen, Sektoren, Asset-Klassen)
- Investment-Pläne (Sparpläne) mit automatischer Ausführung
- Rebalancing-Vorschläge basierend auf Zielgewichtung

### Dashboard
- Depotwert mit Gewinn/Verlust-Anzeige
- Portfolio-Entwicklung vs. investiertes Kapital (Dual-Axis-Chart)
- Performance-Kennzahlen (TTWROR, IRR)
- Holdings-Übersicht mit Donut-Chart
- Top-Performer und größte Positionen
- Sync-Button für manuelle Kursaktualisierung
- Auto-Sync (15 Min, 30 Min, 1 Std)

### KI-Integration
- **Portfolio Insights**: Automatische Analyse mit farbcodierten Karten
  - Grün: Stärken
  - Orange: Risiken
  - Blau: Empfehlungen
- **ChatBot**: Natürlichsprachliche Fragen zum Portfolio
  - "Wie war meine Rendite dieses Jahr?"
  - "Zeige alle Käufe 2024"
  - "Füge Apple zur Watchlist hinzu"
  - "Liste alle Dividenden der letzten 12 Monate"
- **Chart-Analyse**: Technische Analyse mit KI
  - Support/Resistance-Marker direkt im Chart
  - Trend-Erkennung und Prognosen
  - Indikator-Interpretation
- **Web-Suche**: Aktuelle Nachrichten einbeziehen (Perplexity)
- Unterstützte Provider: Claude, OpenAI, Gemini, Perplexity

### Watchlist
- Mehrere Watchlists verwalten
- Mini-Charts mit 3-Monats-Kursentwicklung
- ChatBot-Integration zum Hinzufügen von Securities
- Automatische ISIN/WKN-Ermittlung via Portfolio Report
- Drag & Drop zwischen Watchlists

### Technische Analyse

#### Indikatoren
| Indikator | Beschreibung |
|-----------|--------------|
| **SMA** | Simple Moving Average (10, 20, 50, 200 Tage) |
| **EMA** | Exponential Moving Average |
| **RSI** | Relative Strength Index (Überkauft/Überverkauft) |
| **MACD** | Moving Average Convergence Divergence |
| **Bollinger Bands** | Volatilitätsbänder |
| **Stochastic** | Stochastic Oscillator (%K, %D) |
| **OBV** | On-Balance Volume |
| **ADX** | Average Directional Index (Trendstärke) |
| **ATR** | Average True Range (Volatilität) |
| **VWAP** | Volume Weighted Average Price |

#### Candlestick-Pattern-Erkennung
Automatische Erkennung von Candlestick-Mustern:

**Single Candle:**
- Doji, Hammer, Inverted Hammer, Hanging Man
- Shooting Star, Spinning Top, Marubozu

**Two Candle:**
- Bullish/Bearish Engulfing
- Bullish/Bearish Harami
- Piercing Line, Dark Cloud Cover
- Tweezer Top/Bottom

**Three Candle:**
- Morning Star, Evening Star
- Three White Soldiers, Three Black Crows
- Three Inside Up/Down

#### Zeichenwerkzeuge
- **Trendlinien**: Linie zwischen zwei Punkten
- **Horizontale Linien**: Support/Resistance-Level
- **Fibonacci Retracements**: Automatische Level (0%, 23.6%, 38.2%, 50%, 61.8%, 78.6%, 100%)
- Persistente Speicherung in der Datenbank
- Farbauswahl und Linienstärke

#### Signale & Alerts
- RSI Überkauft/Überverkauft-Signale
- MACD Crossover-Erkennung
- Bollinger Band Squeeze
- Preis-Alerts mit Benachrichtigung

#### Pattern-Tracking
- Erfolgsquoten-Tracking für erkannte Muster
- Evaluierung nach 5 und 10 Tagen
- Statistiken pro Pattern-Typ

### Charts
- Candlestick-Charts mit Lightweight Charts v5
- Heikin-Ashi-Darstellung (optional)
- Mehrere Zeitrahmen (1D, 1W, 1M, 3M, 6M, 1Y, 5Y, Max)
- Volumen-Overlay
- Split-View für Indikatoren
- KI-Annotations direkt im Chart
- Vergleichsmodus (bis zu 5 Securities)

### Reports
- **Performance**: TTWROR, IRR, Volatilität, Sharpe Ratio
- **Dividenden**: Monatliche/jährliche Übersicht mit Charts
- **Realisierte Gewinne**: Nach Security und Jahr
- **Steuer-Report**: Aufbereitung für Steuererklärung
- **PDF-Export**: Alle Reports als PDF exportierbar

### Corporate Actions
- Stock Splits mit automatischer Anpassung
- Spin-Offs mit Kostenbasis-Aufteilung
- Split-Historie und adjustierte Kurse

### Import/Export
- Portfolio Performance `.portfolio` Dateien
- CSV-Import für Transaktionen und Kurse
- CSV-Export für alle Daten
- PDF-Import mit OCR (Bank-Abrechnungen)
  - Unterstützte Banken: Trade Republic, Scalable Capital, comdirect, ING, DKB
  - OCR via Claude/Gemini (direkter PDF-Upload) oder OpenAI/Perplexity (mit Poppler)

## Tech Stack

| Bereich | Technologie |
|---------|-------------|
| **Frontend** | React 18, TypeScript, Vite, TailwindCSS |
| **State** | Zustand |
| **Charts** | Recharts, Lightweight Charts v5 |
| **Backend** | Tauri 2.9, Rust |
| **Datenbank** | SQLite (rusqlite) |
| **AI** | Claude, OpenAI, Gemini, Perplexity APIs |
| **Build** | pnpm Workspaces, Turbo |

## Installation

### Voraussetzungen
- Node.js 18+
- pnpm 8+
- Rust (für Entwicklung)
- Poppler (optional, für PDF-OCR mit OpenAI/Perplexity)

### Entwicklung

```bash
# Repository klonen
git clone https://github.com/your-repo/portfolio-modern.git
cd portfolio-modern

# Dependencies installieren
pnpm install

# Development Server starten
pnpm desktop

# Release Build erstellen
pnpm desktop:build

# Tests ausführen
cd apps/desktop && pnpm test
```

### Release-Builds

Nach `pnpm desktop:build` befinden sich die Builds unter:
- **macOS**: `src-tauri/target/release/bundle/macos/Portfolio Now.app`
- **DMG**: `src-tauri/target/release/bundle/dmg/Portfolio Now_0.1.0_aarch64.dmg`

## Konfiguration

### Quote Provider

| Provider | API Key | Beschreibung |
|----------|---------|--------------|
| **Yahoo** | Nein | Kostenlos, aktuell + historisch |
| **Portfolio Report** | Nein | ISIN/WKN-Lookup (wie PP) |
| **Finnhub** | Ja | US-Aktien, Premium für Historie |
| **Alpha Vantage** | Ja | 25 Calls/Tag free |
| **CoinGecko** | Nein | Kryptowährungen |
| **EZB** | Nein | Wechselkurse |

### AI Provider

| Provider | API Key | Modelle | Besonderheiten |
|----------|---------|---------|----------------|
| **Claude** | Ja | claude-sonnet-4-5, claude-haiku-4-5 | Vision + PDF-Upload |
| **OpenAI** | Ja | gpt-4.1, gpt-4o, gpt-4o-mini | Vision |
| **Gemini** | Ja | gemini-3-flash, gemini-3-pro | Vision + PDF-Upload |
| **Perplexity** | Ja | sonar-pro, sonar | Vision + Web-Suche |

## Projektstruktur

```
apps/desktop/
├── src/                    # React Frontend
│   ├── components/         # UI-Komponenten
│   │   ├── charts/         # TradingViewChart, AIAnalysisPanel, DrawingTools
│   │   ├── chat/           # ChatPanel, ChatMessage
│   │   ├── common/         # Shared Components
│   │   ├── layout/         # Header, Sidebar
│   │   └── modals/         # Dialoge
│   ├── views/              # Seiten (Dashboard, Charts, Reports, etc.)
│   ├── store/              # Zustand Stores
│   └── lib/                # Utilities, Types, Hooks
│       ├── indicators.ts   # Technische Indikatoren
│       ├── patterns.ts     # Candlestick-Pattern-Erkennung
│       └── signals.ts      # Signal-Erkennung
├── src-tauri/              # Rust Backend
│   └── src/
│       ├── commands/       # Tauri IPC Commands
│       ├── db/             # SQLite Schema & Queries
│       ├── ai/             # KI-Integration
│       ├── quotes/         # Kursquellen
│       ├── fifo/           # FIFO Cost Basis
│       └── pdf_import/     # PDF-Parsing
└── tests/                  # Unit Tests
```

## Entwicklung

```bash
# Linting
pnpm lint

# Type Check
cd apps/desktop && pnpm tsc --noEmit

# Unit Tests
cd apps/desktop && pnpm test

# Rust Tests
cd apps/desktop/src-tauri && cargo test --release

# Rust Formatting
cd apps/desktop/src-tauri && cargo fmt

# Rust Linting
cd apps/desktop/src-tauri && cargo clippy
```

## Bekannte Einschränkungen

- Holdings werden aus Transaktionen berechnet (nicht aus FIFO-Lots)
- ISIN-Aggregation: Securities mit gleicher ISIN werden zusammengefasst
- Yahoo-Symbole: Internationale haben Suffix (.DE, .WA), US nicht
- GBX/GBp Währung: British Pence werden durch 100 geteilt

## Changelog

Siehe [CHANGELOG.md](./CHANGELOG.md) für die Versionshistorie.

## Dokumentation

- [Technische Analyse](./TECHNICAL_ANALYSIS.md) - Detaillierte Dokumentation der TA-Features
- [PP Import/Export](./src-tauri/PP_IMPORT_EXPORT.md) - Portfolio Performance Dateiformat
- [PDF Import](./src-tauri/PDF_IMPORT_STATUS.md) - Status der Bank-PDF-Parser

## Lizenz

Noch nicht festgelegt.

## Credits

- Inspiriert von [Portfolio Performance](https://github.com/portfolio-performance/portfolio)
- Charts: [Lightweight Charts](https://github.com/nickvdyck/lightweight-charts)
- Icons: [Lucide](https://lucide.dev/)
