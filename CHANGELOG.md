# Changelog

All notable changes to Portfolio Now will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.8] - 2026-02-01

### Changed
- **Dynamic SQL System**: ChatBot now uses dynamic SQL generation via `sql_executor.rs` instead of fixed templates
- SQL queries generated as ```sql``` code blocks by the LLM
- Session-based SQL pattern approval (`sqlApprovalMode: 'always' | 'session' | 'never'`)
- Configurable error handling (`sqlErrorHandling: 'auto_retry' | 'show_error'`)
- **AI Models Update**: o3/o4-mini jetzt mit Vision + Web-Suche, gpt-4.1 nur noch Coding-Modell (kein Vision)
- OpenAI Fallback-Chain: gpt-5-mini → o3 → gpt-4o → gpt-4o-mini

### Added
- **Query Approval Commands**: `approve_query_type_for_session`, `execute_pending_query`, `get_session_approved_query_types`, `revoke_all_query_approvals` in `commands/chat.rs`
- **E2E Test**: AI-Chat Playwright Test (`ai-chat.spec.ts`)

### Removed
- **Query Templates** (~4,800 lines deleted):
  - `query_templates.rs` - 13 fixed query templates
  - `structured_query.rs` - Structured query JSON parser
  - `user_templates.rs` - User-defined template system
- "Eigene Abfragen" settings section and UI components
- `UserTemplatesSettings.tsx`, `UserTemplateModal.tsx`
- `gpt-4.1` aus Vision-Modell-Registry entfernt (Coding-fokussiert, kein Vision-Support)

### Fixed
- **Tag-Leak Fix**: LLM-generated tags with single closing bracket `]` now normalized to `]]`
  - Backend: `fix_single_close_bracket()` in `normalizer.rs`
  - Frontend: `sanitizeCommandTags()` in `SafeMarkdown.tsx`
- **Version Sync**: `tauri.conf.json` auf 0.1.8 aktualisiert (war auf 0.1.6 stehen geblieben)

### Security & Stability Audit (2026-02-09)

#### CRITICAL Fixes
- **IRR Doppelzählung**: DELIVERY-Cashflows wurden in `get_cash_flows_with_fallback()` doppelt gezählt (bereits in `get_cash_flows()` enthalten)
- **Debug-Datei entfernt**: `/tmp/irr-debug-output.txt` wurde im Release-Build geschrieben (`performance/mod.rs`)
- **SQL-Injection Härtung**: Comment-Stripping (`-- / /* */`), Semicolon-Schutz, CTE-Blocking, neue verbotene Keywords (`LOAD_EXTENSION`, `SAVEPOINT`, `RELEASE`) in `sql_executor.rs`
- **DB Mutex Absturz**: `DB.lock().unwrap()` → sicheres Error-Handling in `db/mod.rs`

#### HIGH Fixes
- **Rate Limiting aktiviert**: `check_rate_limit()` jetzt aktiv für `sync_all_prices`, `analyze_chart_with_ai`, `chat_with_portfolio_assistant`
- **FIFO Error-Logging**: Stille `.filter_map(|r| r.ok())` durch explizites `log::warn!` ersetzt
- **FIFO Rundungsfehler**: Letzter Lot-Verbrauch nutzt Rest statt erneute Rundung (verhindert Cent-Differenzen)
- **Performance unwrap()**: `.first().unwrap()` / `.last().unwrap()` durch Pattern-Matching ersetzt
- **TTWROR Fallback-Warnung**: Warnt wenn Cashflows bei einfacher Berechnung ignoriert werden

#### MEDIUM Fixes
- **CSP gehärtet**: `connect-src` von `https:` auf 15 explizite API-Domains eingeschränkt
- **Input-Sanitization aktiviert**: `sanitize_string()` jetzt in `create_security` und `create_account` aktiv
- **TRANSFER_IN Fallback**: Nutzt Transaktionsbetrag statt 0 für Cost Basis
- **Stale Exchange Rates**: Wechselkurse > 180 Tage werden abgelehnt
- **SQL format!() dokumentiert**: Tech-Debt-Kommentar für i64-sichere Werte

#### LOW Fixes
- **Profilbild MIME-Validierung**: Nur `data:image/` wird akzeptiert in ChatMessage
- **Externe Links**: Via Tauri Shell API statt `target="_blank"` (SafeMarkdown, DivvyDiaryExportModal)
- **userName Limit**: Eingabe auf 50 Zeichen begrenzt in Settings

#### ChatBot Fixes
- **Fees/Taxes verloren**: `execute_confirmed_transaction` erstellt jetzt FEE/TAX-Units
- **Wechselkurs-Richtung**: `compute_dividend_gross_amount` und `normalize_extracted_fees` korrigiert (`gross / rate` statt `gross * rate`)
- **SSOT**: Doppelte `normalize_extracted_txn_type` entfernt (jetzt nur in `command_parser.rs`)
- **Input-Wiederherstellung**: Chat-Input wird bei Fehler wiederhergestellt (nicht nur Retry-Button)
- **Suggestions bei DB-Fehler**: Werden trotzdem in der UI angezeigt
- **Context-Logging**: `load_portfolio_context` loggt jetzt DB-Fehler statt sie zu schlucken
- **Datum-Validierung**: Minimum von 2000 auf 1970 geändert (historische Transaktionen)

#### Test Fixes
- **AiQuoteSuggestion**: `#[serde(alias = "feed_url")]` — akzeptiert jetzt snake_case und camelCase
- **Name-Similarity**: Trailing-Punctuation-Stripping + Prefix-Matching für Abkürzungen (z.B. "Corp" ↔ "Corporation")
- 6 neue SQL-Validierungs-Tests in `sql_executor.rs`
- **Ergebnis**: 313 Tests bestanden, 0 Fehler (vorher 311 bestanden, 2 Fehler)

## [0.1.7] - 2026-01-29

> **Hinweis:** Manifest-Versionen (`Cargo.toml`, `package.json`) wurden bei diesem Release versehentlich nicht aktualisiert (blieben auf 0.1.6). Ab 0.1.8 gelten strikte Automatisierungsregeln (siehe CLAUDE.md).

### Added
- **Logarithmic Scale**: Chart toggle button for logarithmic/linear Y-axis scale (normal + fullscreen mode)
- **Historical Quotes Batch**: New modal for batch loading historical prices with progress tracking, spike detection, and cancel support
- **Inline Quote Assistant**: AI-powered quote configuration suggestions integrated directly in SecurityFormModal
- **Quote Error Tracking**: Quote fetch errors are now saved to database for debugging
- **Widget Dashboard**: Drag & drop Widget-Dashboard als alternative Dashboard-Ansicht

### Changed
- **Quote UI Refactor**: Consolidated 4 separate quote modals into inline assistant and batch loader
- Quote Assistant now configurable as separate AI feature (`quoteAssistant`)

### Removed
- QuoteAssistantModal, QuoteAuditModal, QuoteManagerModal, QuoteSuggestionModal (functionality moved to inline assistant)

## [0.1.6] - 2025-01-23

### Added
- **Query Templates**: User-defined query templates for ChatBot with 13 built-in templates
- **Image Drag & Drop**: Support for dragging images directly into chat
- **Inline Suggestions**: ChatBot now shows inline action suggestions
- **PDF Drag & Drop**: Drag & drop support for PDF import
- **Quote Assistant**: AI-powered quote assistant with symbol validation
- **Settings Redesign**: New sidebar navigation with free input for chat context
- **Transaction Creation**: Create transactions directly via ChatBot
- **Speech-to-Text**: Voice input for ChatBot using OpenAI Whisper API (only when OpenAI provider selected)
- **Cost Basis History Chart**: Dashboard chart now supports toggling between current cost basis line and historical cost basis over time

### Fixed
- Unified confirmation UI across ChatBot
- Fees bug for DEPOSIT/REMOVAL transactions
- Repository name in README.md
- Waveform animation not showing during speech-to-text recording (race condition fix)
- Portfolio history chart now shows today's value using latest prices (matches Dashboard total)

## [0.1.5] - 2025-01-20

### Added
- **AI Activity Indicator**: Visual feedback during PDF OCR processing
- **Extended AI Dropdown**: Header dropdown now shows all 5 AI features
- **Feature-specific AI Config**: Each AI feature can have its own provider/model
- **29 New Bank Parsers**: Extended PDF import support
- **Consortium View**: Neue View für Konzern-/Gruppenübersicht

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
- **Screener View**: Neue View für Wertpapier-Screening
- **GPT-5 Mini**: OpenAI gpt-5-mini Modell hinzugefügt

### Fixed
- PDF import duplicate detection
- UI optimizations for import flow

## [0.1.0] - 2025-01-05

### Added
- **Initial Release**: Complete portfolio tracking application
- **.portfolio Import/Export**: Full support for Portfolio Performance file format
- **Quote Providers**: Yahoo Finance, Finnhub, Alpha Vantage, CoinGecko, EZB, TradingView, Portfolio Report
- **AI Providers**: Claude (Sonnet/Haiku 4.5), OpenAI (o3, o4-mini, GPT-5 Mini, GPT-4o), Gemini (2.5 Flash/Pro, 3 Flash/Pro Preview), Perplexity (Sonar Pro/Sonar) with vision support
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
- **Asset Statement View**: Vermögensaufstellung-Ansicht

### Technical
- Tauri 2.9 with Rust backend
- React 18 + TypeScript frontend
- SQLite database with prost Protobuf
- pnpm workspaces with Turbo build system

[0.1.8]: https://github.com/rullmann/portfolio-now/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/rullmann/portfolio-now/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/rullmann/portfolio-now/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/rullmann/portfolio-now/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/rullmann/portfolio-now/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/rullmann/portfolio-now/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/rullmann/portfolio-now/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/rullmann/portfolio-now/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/rullmann/portfolio-now/releases/tag/v0.1.0
