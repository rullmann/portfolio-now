# Portfolio Now

Cross-Platform Desktop-App zur Portfolio-Verwaltung. Moderne Neuimplementierung von [Portfolio Performance](https://github.com/portfolio-performance/portfolio) mit Tauri (Rust + React/TypeScript).

## Features

### Portfolio-Management
- Import/Export von Portfolio Performance `.portfolio` Dateien
- Mehrere Portfolios und Konten verwalten
- FIFO-basierte Einstandsberechnung
- Automatische Kursaktualisierung (Yahoo, Finnhub, CoinGecko, EZB)
- Dividenden-Tracking und Steuerreports

### Dashboard
- Depotwert mit Gewinn/Verlust-Anzeige
- Portfolio-Entwicklung vs. investiertes Kapital
- Performance-Kennzahlen (TTWROR, IRR)
- Sync-Button für manuelle Kursaktualisierung
- Auto-Sync (15 Min, 30 Min, 1 Std)

### KI-Integration
- **Portfolio Insights**: Automatische Analyse mit Stärken, Risiken, Empfehlungen
- **ChatBot**: Fragen zum Portfolio stellen
  - "Wie war meine Rendite dieses Jahr?"
  - "Zeige alle Käufe 2024"
  - "Füge Apple zur Watchlist hinzu"
- **Chart-Analyse**: Technische Analyse mit Support/Resistance-Markern
- Unterstützte Provider: Claude, OpenAI, Gemini, Perplexity

### Watchlist
- Mehrere Watchlists verwalten
- Mini-Charts mit Kursentwicklung
- ChatBot-Integration zum Hinzufügen
- Automatische ISIN/WKN-Ermittlung via Portfolio Report

### Charts & Analyse
- Candlestick-Charts mit TradingView
- Technische Indikatoren: RSI, MACD, Bollinger Bands
- KI-gestützte Trendanalyse

### Reports
- Performance-Übersicht
- Dividenden-Report
- Realisierte Gewinne/Verluste
- Steuer-Report nach Jahr

## Tech Stack

- **Frontend**: React 18, TypeScript, Vite, TailwindCSS, Zustand
- **Backend**: Tauri 2.9, Rust, SQLite
- **Charts**: Recharts, Lightweight Charts v5
- **AI**: Claude, OpenAI, Gemini, Perplexity APIs

## Installation

```bash
# Dependencies installieren
pnpm install

# Development Server starten
pnpm desktop

# Release Build erstellen
pnpm desktop:build
```

## Konfiguration

API-Keys werden in den Einstellungen konfiguriert:

| Provider | Verwendung |
|----------|------------|
| **Finnhub** | US-Aktienkurse |
| **Alpha Vantage** | Aktiensuche |
| **CoinGecko** | Kryptowährungen |
| **Brandfetch** | Firmenlogos |
| **Claude/OpenAI/Gemini/Perplexity** | KI-Features |

## Entwicklung

```bash
# Linting
pnpm lint

# Rust Tests
cd apps/desktop/src-tauri && cargo test --release

# Type Check
pnpm desktop:build
```

## Lizenz

Noch nicht festgelegt.
