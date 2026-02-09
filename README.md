# Portfolio Now

Modern cross-platform desktop application for tracking and analyzing investment portfolios. A reimplementation of [Portfolio Performance](https://github.com/portfolio-performance/portfolio) using Tauri (Rust + React/TypeScript).

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)

## Features

- **Import Portfolio Performance files** (.portfolio format via Protobuf)
- **Real-time quotes** from 8 providers (Yahoo Finance, TradingView, Portfolio Report, Finnhub, Alpha Vantage, CoinGecko, Kraken, EZB)
- **FIFO cost basis tracking** with realized gains calculation
- **Performance metrics** (TTWROR, IRR, benchmark comparison with Alpha/Beta/Sharpe)
- **Technical analysis** charts (Candlestick, 9 indicators, 22 candlestick patterns, drawing tools, signals)
- **AI-powered analysis** with Claude, GPT-5, Gemini, or Perplexity for chart and portfolio insights
- **AI Chat Assistant** with dynamic SQL queries, watchlist management, and transaction creation
- **Portfolio Insights & Buy Opportunities** via AI analysis
- **Dividend tracking** with calendar view, payment history, and logos
- **Taxonomies & classifications** for asset allocation analysis
- **Investment plans** with interval scheduling and auto-execution
- **Rebalancing** preview and execution
- **Portfolio optimization** (Markowitz, Efficient Frontier, Monte Carlo)
- **Screener** with customizable filters and presets
- **Widget Dashboard** with drag & drop customization
- **CSV import** with broker template detection (Trade Republic, Scalable, ING, DKB, DEGIRO, and more)
- **PDF import** with AI-powered OCR for 36 banks (DE, CH, AT, International)
- **PDF export** for portfolio reports
- **DivvyDiary export** for dividend calendar sync
- **Share to X (Twitter)** with chart screenshots and AI analysis threads
- **German tax reports** (Anlage KAP) with Freistellungsauftrag tracking
- **Corporate actions** (stock splits, mergers, spin-offs)
- **Multi-currency support** with ECB exchange rates
- **Speech-to-text** via OpenAI Whisper for chat input

## Screenshots

*Coming soon*

## Tech Stack

| Layer | Technologies |
|-------|-------------|
| **Frontend** | React 18, TypeScript, Vite, TailwindCSS, Zustand, Recharts, Lightweight Charts |
| **Backend** | Tauri 2.9, Rust, SQLite, prost (Protobuf), Tokio, reqwest |
| **Build** | pnpm Workspaces, Turbo |

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/) 8+
- [Rust](https://rustup.rs/) 1.70+

### Installation

```bash
# Clone the repository
git clone https://github.com/rullmann/portfolio-now.git
cd portfolio-now

# Install dependencies
pnpm install

# Start development server
pnpm desktop
```

### Build

```bash
# Build for production
pnpm desktop:build
```

## Project Structure

```
portfolio-now/
├── apps/desktop/              # Tauri Desktop App
│   ├── src/                  # React Frontend (TypeScript)
│   │   ├── components/       # UI Components (11 subdirectories)
│   │   ├── views/            # 20 Page Views
│   │   ├── store/            # Zustand State Management
│   │   ├── hooks/            # Custom React Hooks
│   │   └── lib/              # API, Types, Utilities, Indicators, Signals
│   └── src-tauri/            # Rust Backend
│       └── src/
│           ├── commands/     # 33 Tauri IPC Command Modules
│           ├── db/           # SQLite (rusqlite)
│           ├── quotes/       # 8 Quote Providers
│           ├── ai/           # AI Analysis, Chat, SQL Executor, Models
│           ├── fifo/         # FIFO Cost Basis
│           ├── performance/  # TTWROR, IRR Calculations
│           ├── pdf_import/   # PDF Import with OCR
│           ├── csv_import/   # CSV Import (Broker Templates)
│           ├── currency/     # Currency Conversion
│           ├── tax/          # German Tax (Anlage KAP)
│           ├── optimization/ # Portfolio Optimization (Markowitz)
│           └── security/     # Path Validation, Rate Limiting
```

## Views

| View | Description |
|------|-------------|
| Dashboard | Portfolio overview with KPIs, mini-charts, and auto-refresh |
| Widget Dashboard | Customizable drag & drop dashboard with widgets |
| Securities | Manage securities with logos, attributes, and price sync |
| Accounts | Track cash accounts and running balances |
| Transactions | Filter, paginate, and bulk-manage all transactions |
| Holdings | Donut chart visualization with allocation breakdown |
| Dividends | Dividend payments with calendar view and forecasts |
| Watchlist | Track securities without owning them |
| Taxonomies | Classify assets by custom categories with pie charts |
| Benchmark | Compare portfolio against benchmarks (Alpha, Beta, Sharpe) |
| Charts | Technical analysis with 9 indicators, 22 patterns, AI analysis, drawing tools |
| Reports | Dividend, realized gains, and tax reports with PDF export |
| Rebalancing | Calculate trades to reach target allocation |
| Optimization | Markowitz portfolio optimization with efficient frontier |
| Screener | Filter securities by performance, dividends, and risk metrics |
| Investment Plans | Manage recurring buy plans with auto-execution |
| Asset Statement | Stichtagsvergleich (point-in-time comparison) |
| Consortium | Multi-portfolio consortium analysis |
| Settings | API keys, AI configuration, sharing, and preferences |

## Quote Providers

| Provider | API Key Required | Features |
|----------|-----------------|----------|
| Yahoo Finance | No | Real-time & historical quotes |
| TradingView | No | Global markets (EXCHANGE:SYMBOL) |
| Portfolio Report | No | ISIN/WKN lookup, prices |
| Finnhub | Yes | US stocks, premium history |
| Alpha Vantage | Yes | Global stocks (25 calls/day free) |
| CoinGecko | Optional | Cryptocurrencies |
| Kraken | No | Crypto exchange prices |
| ECB | No | Exchange rates |

## AI Providers

| Provider | Models | Features |
|----------|--------|----------|
| Claude (Anthropic) | claude-sonnet-4-5, claude-haiku-4-5 | Vision, direct PDF upload |
| OpenAI | gpt-5-mini, o3, o4-mini, gpt-4.1, gpt-4o | Vision, Web Search (o3/o4) |
| Gemini (Google) | gemini-2.5-flash/pro, gemini-3-flash/pro | Vision, direct PDF upload |
| Perplexity | sonar-pro, sonar | Vision, Web Search |

Each AI feature (Chart Analysis, Portfolio Insights, Chat, PDF OCR, CSV Import, Quote Assistant) can use a different provider/model.

## Acknowledgments

- [Portfolio Performance](https://github.com/portfolio-performance/portfolio) - The original inspiration
- [Tauri](https://tauri.app/) - For the amazing cross-platform framework
- [Lightweight Charts](https://github.com/tradingview/lightweight-charts) - For beautiful financial charts
