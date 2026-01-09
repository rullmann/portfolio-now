# Portfolio Now

Modern cross-platform desktop application for tracking and analyzing investment portfolios. A reimplementation of [Portfolio Performance](https://github.com/portfolio-performance/portfolio) using Tauri (Rust + React/TypeScript).

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)

## Features

- **Import Portfolio Performance files** (.portfolio format via Protobuf)
- **Real-time quotes** from 7 providers (Yahoo Finance, Finnhub, Alpha Vantage, CoinGecko, EZB, and more)
- **FIFO cost basis tracking** with realized gains calculation
- **Performance metrics** (TTWROR, IRR, benchmark comparison with Alpha/Beta/Sharpe)
- **Technical analysis** charts (Candlestick, RSI, MACD, Bollinger Bands, SMA/EMA)
- **Taxonomies & classifications** for asset allocation analysis
- **Investment plans** with interval scheduling
- **Rebalancing** preview and execution
- **PDF/CSV import & export**
- **Multi-currency support** with ECB exchange rates

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
git clone https://github.com/rullmann/portfolio_now.git
cd portfolio_now

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
├── apps/desktop/           # Tauri Desktop App
│   ├── src/               # React Frontend
│   │   ├── components/    # UI Components
│   │   ├── views/         # Page Views
│   │   ├── store/         # Zustand State
│   │   └── lib/           # Utilities & API
│   └── src-tauri/         # Rust Backend
│       ├── src/commands/  # Tauri IPC Commands
│       ├── src/db/        # SQLite Database
│       ├── src/quotes/    # Quote Providers
│       └── src/fifo/      # FIFO Cost Basis
└── packages/
    ├── core/              # Business Logic
    ├── ui/                # Shared UI Components
    └── i18n/              # Internationalization (DE/EN)
```

## Views

| View | Description |
|------|-------------|
| Dashboard | Portfolio overview with holdings table and mini-charts |
| Securities | Manage securities with logos and price sync |
| Accounts | Track cash accounts and balances |
| Transactions | Filter and paginate all transactions |
| Holdings | Donut chart visualization of positions |
| Watchlist | Track securities without owning them |
| Taxonomies | Classify assets by custom categories |
| Benchmark | Compare portfolio against benchmarks |
| Charts | Technical analysis with indicators |
| Reports | Dividend, gains, and tax reports |
| Rebalancing | Calculate trades to reach target allocation |

## Quote Providers

| Provider | API Key Required | Features |
|----------|-----------------|----------|
| Yahoo Finance | No | Real-time & historical quotes |
| Finnhub | Yes | US stocks |
| Alpha Vantage | Yes | Global stocks (25 calls/day free) |
| CoinGecko | No | Cryptocurrencies |
| ECB | No | Exchange rates |

## Acknowledgments

- [Portfolio Performance](https://github.com/portfolio-performance/portfolio) - The original inspiration
- [Tauri](https://tauri.app/) - For the amazing cross-platform framework
- [Lightweight Charts](https://github.com/tradingview/lightweight-charts) - For beautiful financial charts
