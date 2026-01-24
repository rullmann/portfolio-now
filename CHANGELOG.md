# Changelog

All notable changes to Portfolio Now will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2025-01-23

### Added
- **Query Templates**: User-defined query templates for ChatBot with 13 built-in templates
- **Image Drag & Drop**: Support for dragging images directly into chat
- **Inline Suggestions**: ChatBot now shows inline action suggestions
- **PDF Drag & Drop**: Drag & drop support for PDF import
- **Quote Assistant**: AI-powered quote assistant with symbol validation
- **Settings Redesign**: New sidebar navigation with free input for chat context
- **Transaction Creation**: Create transactions directly via ChatBot

### Fixed
- Unified confirmation UI across ChatBot
- Fees bug for DEPOSIT/REMOVAL transactions
- Repository name in README.md

## [0.1.5] - 2025-01-20

### Added
- **AI Activity Indicator**: Visual feedback during PDF OCR processing
- **Extended AI Dropdown**: Header dropdown now shows all 5 AI features
- **Feature-specific AI Config**: Each AI feature can have its own provider/model
- **29 New Bank Parsers**: Extended PDF import support

### Fixed
- Dashboard AiFeaturesCard now scrollable to show all 5 features

### Changed
- Removed unused `externalBin` (pp-import) from bundle config

### CI/CD
- Added Linux builds with free GitHub runners
- Auto-publish releases (no draft mode)
- Added GitHub Actions release workflow

## [0.1.4] - 2025-01-15

### Added
- **Playwright & WebDriverIO**: E2E test setup for automated testing
- **Rust Unit Tests**: Added proper unit tests, removed `|| true` anti-pattern

### Changed
- Minor improvements to AI components

## [0.1.3] - 2025-01-12

### Added
- **PDF Export Redesign**: Improved PDF export with consistent styling
- **Date Format Standardization**: Unified date formatting across the app

## [0.1.2] - 2025-01-10

### Added
- **CSV Import**: Broker template detection for 20+ brokers (Trade Republic, Scalable, ING, DKB, DEGIRO, etc.)
- **AI Dropdown in Header**: Quick access to AI features from main navigation
- **GPT-5 Support**: Added OpenAI GPT-5 model support

### Fixed
- IRR calculation with proper cashflow handling
- DivvyDiary export compatibility

### Added
- **Portfolio Optimization**: Markowitz efficient frontier calculation
- Enhanced documentation

## [0.1.1] - 2025-01-08

### Security
- **Security Hardening**: Comprehensive security review and fixes
- **Secure API Key Storage**: Migrated from localStorage to `tauri-plugin-store`
- **Code Cleanup**: Removed dead code and unused dependencies

### Added
- AI module extraction for better maintainability
- Improved crypto provider (CoinGecko, Kraken)

### Fixed
- PDF import duplicate detection
- UI optimizations for import flow

## [0.1.0] - 2025-01-05

### Added
- **Initial Release**: Complete portfolio tracking application
- **.portfolio Import/Export**: Full support for Portfolio Performance file format
- **Quote Providers**: Yahoo Finance, Finnhub, Alpha Vantage, CoinGecko, EZB, TradingView, Portfolio Report
- **AI Providers**: Claude, OpenAI (GPT-4/5), Gemini, Perplexity with vision support
- **FIFO Cost Basis**: Automatic lot tracking with realized gains calculation
- **Performance Metrics**: TTWROR, IRR, benchmark comparison (Alpha, Beta, Sharpe)
- **Technical Analysis**: Candlestick charts with RSI, MACD, Bollinger Bands, SMA/EMA
- **AI Chart Analysis**: Vision-based chart interpretation
- **Portfolio Insights**: AI-powered portfolio analysis and recommendations
- **Chat Assistant**: Natural language portfolio queries with action suggestions
- **Dividend Tracking**: Payment history with security logos
- **Taxonomies**: Custom classification system for asset allocation
- **Investment Plans**: Interval-based investment scheduling
- **Rebalancing**: Preview and execute trades to reach target allocation
- **PDF Import**: AI-powered OCR for 36 supported banks
- **Corporate Actions**: Stock splits, spin-offs, and mergers
- **German Tax Report**: Anlage KAP generation
- **Multi-currency**: ECB exchange rates with automatic conversion

### Technical
- Tauri 2.9 with Rust backend
- React 18 + TypeScript frontend
- SQLite database with prost Protobuf
- pnpm workspaces with Turbo build system

[0.1.6]: https://github.com/rullmann/portfolio-now/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/rullmann/portfolio-now/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/rullmann/portfolio-now/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/rullmann/portfolio-now/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/rullmann/portfolio-now/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/rullmann/portfolio-now/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/rullmann/portfolio-now/releases/tag/v0.1.0
