# Portfolio Now

Cross-Platform Desktop-App zur Portfolio-Verwaltung. Neuimplementierung von [Portfolio Performance](https://github.com/portfolio-performance/portfolio) mit Tauri (Rust + React/TypeScript).

| Eigenschaft | Wert | SSOT |
|-------------|------|------|
| **Bundle ID** | `com.portfolio-now.app` | `tauri.conf.json` |
| **Version** | 0.1.8 | `Cargo.toml` + `package.json` |
| **Jahr** | 2026 | - |

> **Versions-Pflege:** Bei jedem Release müssen `Cargo.toml`, `package.json`, `tauri.conf.json` und `CHANGELOG.md` synchron aktualisiert werden.

## Build-Hinweise

- **KEINE Mac DMG bauen** - nur Development-Builds verwenden
- Für Release-Builds: `pnpm tauri build --bundles app`

---

## AI Agents

Für komplexe Aufgaben sollen spezialisierte Agenten eingesetzt werden:

| Agent | Rolle | Aufgaben |
|-------|-------|----------|
| **Product / Requirements** | Anforderungsanalyse | User Stories verstehen, Akzeptanzkriterien definieren, Scope klären |
| **Architect / Design** | Systemdesign | Architekturentscheidungen, Datenmodelle, API-Design, Technologie-Auswahl |
| **Coding / Implementation** | Entwicklung | Code schreiben, Refactoring, Bug-Fixes, Feature-Implementierung |
| **Test / QA** | Qualitätssicherung | Tests schreiben, Edge Cases finden, Testabdeckung prüfen |
| **Review / Critic** | Code-Review | Code-Qualität bewerten, Verbesserungen vorschlagen, Best Practices |
| **DevOps / Deployment** | Build & Deploy | CI/CD, Build-Prozesse, Release-Management, Performance |
| **Security** | Sicherheit | Schwachstellen finden, Security-Best-Practices, Audit |
| **Documentation** | Dokumentation | README, API-Docs, Code-Kommentare, User-Guides |

**Einsatz:** Bei komplexen Tasks mehrere Agenten parallel oder sequentiell nutzen. Der Architect plant, Coding implementiert, Test validiert, Review prüft.

### Automatisierung nach Änderungen

**PFLICHT nach jeder signifikanten Änderung:**

| Schritt | Aktion | Verantwortlich |
|---------|--------|----------------|
| 1. **Code-Änderung** | Feature/Fix implementieren | Coding Agent |
| 2. **Tests** | Unit/Integration Tests anpassen | Test Agent |
| 3. **Review** | Code-Qualität prüfen | Review Agent |
| 4. **CHANGELOG.md** | Version + Änderungen dokumentieren | Documentation Agent |
| 5. **CLAUDE.md** | Architektur/SSOT/Commands aktualisieren | Documentation Agent |

**Bei größeren Änderungen (>100 Zeilen, neue Module, Architektur):**
- **Alle 8 Agenten einbeziehen** (parallel oder sequentiell)
- Product → Architect → Coding → Test → Review → Security → DevOps → Documentation
- Keine Änderung ist abgeschlossen ohne CHANGELOG.md und CLAUDE.md Update

**Checkliste vor Abschluss:**
- [ ] Kompiliert fehlerfrei (`cargo build --release` + `pnpm build`)
- [ ] Tests laufen (`cargo test --release`)
- [ ] CHANGELOG.md hat neuen Eintrag (Version + Datum + Kategorien)
- [ ] CLAUDE.md reflektiert aktuelle Architektur
- [ ] Keine TODO-Kommentare im neuen Code

---

## Performance-Berechnungen (TTWROR, IRR) - KORRIGIERT

**SSOT-Funktionen:**
- TTWROR/Risk: `get_cash_flows()` - nur DEPOSIT/REMOVAL
- IRR: `get_cash_flows_with_fallback()` - mit Fallback + Start-Wert als Cashflow
- Wert: `get_portfolio_value_at_date_with_currency()` - inkl. Cash + FX

**IRR-Berechnung:** Portfolio-Wert am Periodenstart als Cashflow + DEPOSIT/REMOVAL + DELIVERY_INBOUND/OUTBOUND + finaler Wert.

**Dateien:** `src-tauri/src/performance/mod.rs`, `src-tauri/src/commands/performance.rs`

## Architektur

```
apps/desktop/
├── src/                    # React Frontend (TypeScript)
│   ├── store/              # Zustand State Management
│   ├── components/         # UI (11 Unterordner)
│   │   ├── alerts/         # Alert-Anzeige
│   │   ├── attributes/     # Custom-Attribute
│   │   ├── charts/         # TradingViewChart, AIAnalysisPanel, DrawingTools, SignalsPanel
│   │   ├── chat/           # ChatPanel, ChatMessage, ChatButton
│   │   ├── common/         # Shared (Skeleton, DropdownMenu, AIProviderLogo, SafeMarkdown, ...)
│   │   ├── dashboard/      # Dashboard-Cards, Widgets
│   │   ├── layout/         # Layout-Wrapper
│   │   ├── metrics/        # Kennzahlen-Anzeige
│   │   ├── modals/         # PortfolioInsightsModal, TransactionFormModal, etc.
│   │   ├── quote-assistant/ # Kursquellen-Konfiguration
│   │   └── settings/       # Einstellungs-Komponenten
│   ├── views/              # View-Komponenten pro Route
│   └── lib/                # API, Types, Hooks
│       ├── indicators.ts       # Types, Konstanten, convertToOHLC() (keine Berechnungen!)
│       ├── indicators-rust.ts  # Async Tauri invoke-Wrapper für Rust-Indikatoren
│       ├── patterns.ts         # Re-Export von Pattern-Types aus indicators.ts
│       ├── signals.ts          # Re-Export von Signal-Types aus indicators.ts
│       └── screener.ts         # Screener-Types, Presets, Labels, Filter-Helpers
└── src-tauri/              # Rust Backend
    └── src/
        ├── commands/       # Tauri IPC Commands (33 Module)
        ├── db/             # SQLite (rusqlite)
        ├── pp/             # Portfolio Performance Datenmodelle
        ├── protobuf/       # .portfolio Parser
        ├── quotes/         # Kursquellen (Yahoo, Finnhub, EZB, etc.)
        ├── fifo/           # FIFO Cost Basis
        ├── pdf_import/     # PDF Import mit OCR (Vision API)
        ├── csv_import/     # CSV Import (Broker-Templates)
        ├── ai/             # KI-Analyse, Chat, Portfolio Insights, Models Registry, Dynamic SQL
        ├── currency/       # Währungsumrechnung, Wechselkurse
        ├── performance/    # TTWROR, IRR Berechnungen
        ├── optimization/   # Portfolio-Optimierung (Markowitz, Efficient Frontier)
        ├── tax/            # Steuerberechnungen (DE: Anlage KAP)
        ├── indicators/     # Technische Analyse (Rust-native, 20-25x schneller als TS)
        │   ├── calculations.rs  # SMA, EMA, RSI, MACD, Bollinger, ATR, Stochastic, ADX, Ichimoku, Pivot, Fibonacci
        │   ├── patterns.rs      # 22 Candlestick-Patterns
        │   ├── signals.rs       # Signal-Erkennung + Divergenzen
        │   ├── screener.rs      # Screener-Engine mit allen Filtern
        │   ├── regime.rs        # Markt-Regime-Erkennung, Setup-Scoring, Risk-Engine
        │   └── types.rs         # Shared Types
        ├── security/       # Pfadvalidierung, Rate Limiting
        ├── validation/     # Input-Validierung, AI-Fallback
        ├── models/         # Datenstrukturen
        └── events.rs       # Tauri Event-Emitter (data_changed, etc.)
```

## Tech Stack

**Frontend:** React 19, TypeScript, Vite, Tailwind CSS 4, Zustand, Recharts, Lightweight Charts v5, Lucide Icons
**Backend:** Tauri 2.9, Rust, SQLite, prost (Protobuf), Tokio, reqwest
**Build:** pnpm Workspaces, Turbo

```bash
pnpm install && pnpm desktop              # Dev Server mit Hot Reload
pnpm desktop:build                        # Release Build
cd apps/desktop/src-tauri && cargo test --release  # Rust Tests
```

---

## SSOT: Single Source of Truth

**Jede Datenberechnung hat genau EINE autoritative Quelle. Niemals Logik duplizieren!**

| Daten | SSOT-Modul | Zentrale Funktion(en) | VERBOTEN |
|-------|------------|----------------------|----------|
| **Holdings (Stückzahlen)** | `pp/common.rs` | `HOLDINGS_SUM_SQL`, `HOLDINGS_ADD_TYPES`, `HOLDINGS_REMOVE_TYPES` | FIFO-Lots für Stückzahlen |
| **Cost Basis (Einstandswert)** | `fifo/mod.rs` | `get_total_cost_basis_converted()`, `get_cost_basis_by_security_*()` | GROUP BY auf FIFO-Lots |
| **Datum-Parsing** | `pp/common.rs` | `parse_date_flexible()` | Eigene Date-Parser |
| **Währungsumrechnung** | `currency/mod.rs` | `convert()`, `get_exchange_rate()` | Eigene Kurs-Lookups |
| **AI-Modelle** | `ai/models.rs` | `get_model()`, `get_model_upgrade()`, `get_fallback()` | Hardcodierte Modell-IDs |
| **Kurse abrufen** | `quotes/mod.rs` | `fetch_all_quotes()`, Provider-spezifische Funktionen | Direkte API-Calls |
| **Performance (TTWROR/IRR)** | `performance/mod.rs` | `calculate_ttwror()`, `calculate_irr()` | Eigene Berechnungen |
| **Cashflows (TTWROR/Risk)** | `performance/mod.rs` | `get_cash_flows()` - nur DEPOSIT/REMOVAL | BUY/SELL für TTWROR |
| **Cashflows (IRR)** | `performance/mod.rs` | `get_cash_flows_with_fallback()` - mit BUY/SELL Fallback | Mischen von BUY/SELL + DEPOSIT/REMOVAL |
| **Portfolio-Wert** | `performance/mod.rs` | `get_portfolio_value_at_date_with_currency()` | latest_price ohne FX/Cash |
| **Technische Indikatoren** | `indicators/calculations.rs` | `calculate_sma()`, `calculate_rsi()`, `calculate_macd()` etc. | TS-Berechnungen für Indikatoren |
| **Candlestick-Muster** | `indicators/patterns.rs` | `detect_candlestick_patterns()` | TS Pattern-Erkennung |
| **Handelssignale** | `indicators/signals.rs` | `detect_signals()`, `get_all_signals()` | TS Signal-Erkennung |
| **Screener** | `indicators/screener.rs` | `run_screener()` | TS Screener-Engine |
| **Regime-Erkennung** | `indicators/regime.rs` | `detect_regime()`, `score_setup()`, `calculate_risk()` | Eigene Regime/Scoring-Logik |
| **Datumsformatierung** | `lib/types.ts` | `formatDate()`, `formatDateTime()`, `formatDateShort()` | Eigene Date-Formatierung |
| **ChatBot DB-Abfragen** | `ai/sql_executor.rs` | `execute_sql()`, `validate_sql()` | Hardcodierte SQL im ChatBot |
| **Txn-Type-Normalisierung** | `ai/command_parser.rs` | `normalize_extracted_txn_type()` | Eigene Txn-Type-Mappings |

**Neue Funktion?** 1. Prüfen ob SSOT existiert → 2. Falls ja: verwenden → 3. Falls nein: Im passenden Modul hinzufügen

---

## Code-first, AI-fallback

**KI ist Fallback, kein Ersatz für regelbasierte Logik.**

```
1. Code-Lösung versuchen (deterministisch, schnell, kostenlos)
   ↓ Falls erfolgreich → Fertig
   ↓ Falls fehlgeschlagen oder < 80% Konfidenz
2. KI-Unterstützung anbieten (optional, User muss aktivieren)
3. User bestätigt KI-Vorschläge manuell
```

| Feature | Code-Lösung | KI-Fallback |
|---------|-------------|-------------|
| **CSV-Import** | Broker-Templates + Header-Pattern-Matching | KI analysiert unbekannte Formate |
| **PDF-Import** | Regex + Bank-spezifische Parser | OCR mit Vision-API |
| **Watchlist** | Direkte CRUD-Operationen | ChatBot schlägt vor (User bestätigt) |
| **Chart-Analyse** | Technische Indikatoren (SMA, RSI, MACD) | KI interpretiert Chart-Bild |

---

## Skalierungsfaktoren (KRITISCH!)

| Wert | Faktor | Beispiel |
|------|--------|----------|
| **Shares** | 10^8 | 1.5 Stück = 150_000_000 |
| **Amount** | 10^2 | 100.50 EUR = 10050 |
| **Prices** | 10^8 | 150.25 EUR = 15_025_000_000 |

---

## Holdings-Berechnung (KRITISCH!)

**Holdings ≠ FIFO-Lots!** Niemals FIFO-Lots für Stückzahlen verwenden.

```sql
SELECT SUM(CASE
    WHEN txn_type IN ('BUY', 'TRANSFER_IN', 'DELIVERY_INBOUND') THEN shares
    WHEN txn_type IN ('SELL', 'TRANSFER_OUT', 'DELIVERY_OUTBOUND') THEN -shares
END) / 100000000.0 as shares
FROM pp_txn WHERE owner_type = 'portfolio' GROUP BY security_id, owner_id
```

## Cost Basis (SSOT!)

**NIEMALS** eigene Cost-Basis-Berechnung! Securities können FIFO-Lots in **verschiedenen Währungen** haben (z.B. NESTLE mit CHF + EUR Lots). Immer `fifo/mod.rs`:

```rust
fifo::get_total_cost_basis_converted(conn, portfolio_id, base_currency) -> f64
fifo::get_cost_basis_by_security_converted(conn, base_currency) -> HashMap<String, f64>
fifo::get_cost_basis_by_security_id_converted(conn, base_currency) -> HashMap<i64, f64>
```

---

## Transaktionstypen

**PortfolioTransaction:** `BUY`, `SELL`, `TRANSFER_IN`, `TRANSFER_OUT`, `DELIVERY_INBOUND`, `DELIVERY_OUTBOUND`
**AccountTransaction:** `DEPOSIT`, `REMOVAL`, `INTEREST`, `INTEREST_CHARGE`, `DIVIDENDS`, `FEES`, `FEES_REFUND`, `TAXES`, `TAX_REFUND`, `BUY`, `SELL`, `TRANSFER_IN`, `TRANSFER_OUT`

---

## Tauri Commands

### File & Import
`import_pp_file(path)`, `export_database_to_portfolio(path)`, `rebuild_fifo_lots()`, `read_file_as_base64(path)`, `read_image_as_base64(path)`

### Data
`get_securities()`, `get_accounts()`, `get_pp_portfolios()`, `get_transactions(owner_type?, owner_id?, security_id?, limit?, offset?)`, `get_holdings(portfolio_id)`, `get_all_holdings()`, `get_portfolio_summary()`, `get_portfolio_history()`, `get_price_history(security_id, start_date?, end_date?)`, `get_fifo_cost_basis_history(security_id)`

### CRUD
`create/update/delete/retire_security`, analog für `account`, `portfolio`, `transaction`

### Quotes
`sync_all_prices(only_held?, api_keys?)`, `sync_security_prices(security_ids, api_keys?)`, `fetch_historical_prices(security_id, from, to, api_keys?)`, `search_external_securities(query)`, `fetch_exchange_rates()`, `fetch_exchange_rate(base, target)`

### Performance & Reports
`calculate_performance(portfolio_id?, from?, to?)`, `calculate_benchmark_comparison(benchmark_id, from?, to?)`, `get_dividend_report()`, `get_realized_gains_report()`, `get_tax_report(year)`

### PDF Export
`export_portfolio_summary_pdf`, `export_holdings_pdf`, `export_performance_pdf`, `export_dividend_pdf`, `export_tax_report_pdf`

### Features
`get_watchlists()`, `add_to_watchlist()`, `remove_from_watchlist()`, `get_taxonomies()`, `get_taxonomy_allocations()`, `get_investment_plans()`, `execute_investment_plan()`, `preview_rebalance()`, `execute_rebalance()`

### Corporate Actions
`preview/apply/undo_stock_split`, `apply_spin_off`, `preview/apply_merger`

### Portfolio Optimization (Markowitz)
`calculate_correlation_matrix(portfolio_id?, start?, end?)`, `calculate_efficient_frontier(portfolio_id?, start?, end?, risk_free_rate?)`, `get_optimal_weights(target_return, portfolio_id?, start?, end?)`

### German Tax (DE)
`get/save_tax_settings(year)`, `generate_german_tax_report(year)`, `get_freistellung_status(year)`

### AI Features
`analyze_chart_with_ai/with_annotations/enhanced`, `analyze_portfolio_with_ai`, `chat_with_portfolio_assistant`, `research_security_news(provider, model, api_key, security_name, ...)`, `analyze_trading_setup_with_ai(provider, model, api_key, securityName, regimeLabel, setupScore, ...)`, `get_ai_models(provider, api_key)`, `get_vision_models(provider)`

### AI Helper (ChatBot)
`ai_search_security(query, api_key?)`, `ai_add/remove_from_watchlist(watchlist, security, api_key?)`, `ai_list_watchlists()`, `ai_query_transactions(security?, year?, type?, limit?)`

### Chart Drawings & Pattern
`save/get/delete/clear_chart_drawing(s)`, `save_pattern_detection`, `evaluate_pattern_outcomes`, `get_pattern_statistics`, `get_pattern_history`

### Technical Analysis (Rust-native)
`calculate_sma`, `calculate_ema`, `calculate_rsi`, `calculate_macd`, `calculate_bollinger`, `calculate_atr`, `calculate_vwap`, `calculate_stochastic`, `calculate_obv`, `calculate_adx`, `calculate_ichimoku`, `calculate_pivot_points`, `calculate_fibonacci`, `convert_to_heikin_ashi`, `calculate_all_indicators`, `detect_candlestick_patterns`, `detect_signals`, `get_all_signals`, `detect_all_divergences`, `run_screener`, `detect_regime`, `score_setup`, `calculate_risk`, `full_trading_analysis`

---

## Quote Provider

| Provider | API Key | Beschreibung |
|----------|---------|--------------|
| **Yahoo** | Nein | Kostenlos, aktuell + historisch |
| **TradingView** | Nein | Globale Märkte (EXCHANGE:SYMBOL Format) |
| **Portfolio Report** | Nein | ISIN/WKN-Lookup, Kurse (wie PP) |
| **Finnhub** | Ja | US-Aktien, Premium für Historie |
| **AlphaVantage** | Ja | 25 Calls/Tag free |
| **CoinGecko** | Optional | Krypto, alle Währungen (BTC→bitcoin, ETH→ethereum) |
| **Kraken** | Nein | Krypto-Börsenpreise (BTC→XBT intern) |
| **EZB** | Nein | Wechselkurse |

---

## AI Provider

| Provider | Modelle | Besonderheiten |
|----------|---------|----------------|
| **Claude** | claude-sonnet-4-5-20250514, claude-haiku-4-5-20251015 | Vision + **direkter PDF-Upload** |
| **OpenAI** | o3, o4-mini, gpt-5-mini, gpt-4.1, gpt-4o, gpt-4o-mini | o3/o4: Vision + **Web-Suche**, gpt-4.1 nur Coding (kein Vision) |
| **Gemini** | gemini-2.5-flash, gemini-2.5-pro, gemini-3-flash-preview, gemini-3-pro-preview | Vision + **direkter PDF-Upload** |
| **Perplexity** | sonar-pro, sonar | Vision + Web-Suche |

**PDF OCR:** Claude/Gemini = direkter Upload, OpenAI/Perplexity = Poppler nötig (`brew install poppler`)

**Defaults:** Claude → claude-sonnet-4-5-20250514, OpenAI → gpt-5-mini, Gemini → gemini-2.5-flash, Perplexity → sonar-pro

### AI-Kostenanzeige

Jede AI-Anfrage zeigt nach Abschluss die tatsächlichen Kosten in der Basiswährung an.

**Backend:** Alle Response-Structs haben `input_tokens` + `output_tokens` (getrennt, für korrekte Kostenberechnung).
**Frontend:** `lib/ai-cost.ts` → `calculateAiCost()` berechnet Kosten aus Token-Counts × Model-Pricing, konvertiert via `getLatestExchangeRate()`.

| Komponente | Anzeige-Format |
|-----------|---------------|
| AIAnalysisPanel | `model \| 1.697 Tokens · €0,03` |
| SecurityPriceModal | `model · 1.697 Tokens · €0,03` |
| PortfolioInsightsModal | `(1.697 Tokens · €0,03)` |
| NewsResearchModal | `\| 1.697 Tokens · €0,03` |
| ChatPanel | Subtile Zeile über Eingabefeld |

**Pricing-Daten:** Statisch in `AI_MODELS_FALLBACK` (USD/1M Tokens) + dynamisch von Provider-APIs (OpenRouter). AIModelSelector zeigt Pricing-Badge in Basiswährung.

### Deprecated Model Lifecycle

Veraltete Modelle werden automatisch erkannt und migriert:

| Schicht | Mechanismus | Datei |
|---------|-------------|-------|
| **Rust** | `DEPRECATED_MODELS` Array (old→new Mapping) | `ai/models.rs` |
| **Rust** | `is_deprecated_model()` filtert Provider-API-Ergebnisse | `ai/mod.rs` |
| **Frontend** | `DEPRECATED_MODELS` Map + `getUpgradedModel()` | `store/index.ts` |
| **Frontend** | Zustand `merge` migriert `aiModel` + alle `aiFeatureSettings` | `store/index.ts` |

**Ablauf bei neuem Deprecated Model:**
1. Eintrag in `DEPRECATED_MODELS` in Rust (`ai/models.rs`) UND Frontend (`store/index.ts`)
2. `list_*_models()` filtert automatisch via `is_deprecated_model()`
3. Beim App-Start migriert Zustand `merge` gespeicherte Settings automatisch

**GPT-4.1 Sonderfall:** Kein Vision-Support → `supports_vision` explizit `false` in `openai.rs`

### Unterstützte Banken (PDF Import)

**DE (24):** Baader Bank, Comdirect, Commerzbank, Consorsbank, DAB, Deutsche Bank, DKB, DZ Bank, ebase, flatex, GenoBroker, ING-DiBa, MLP Bank, OLB, OnVista, Postbank, Quirion, S Broker, Santander, Scalable Capital, Targobank, Trade Republic, 1822direkt
**CH (6):** Credit Suisse, LGT, PostFinance, Swissquote, UBS, ZKB
**AT (2):** Erste Bank, Raiffeisen
**International (4):** DEGIRO, Merkur, Revolut, Saxo Bank

### AI Feature Matrix

Jedes Feature kann eigenen Provider/Model haben in `aiFeatureSettings`:

| Feature | ID | Vision nötig? | Web-Search nötig? |
|---------|-----|---------------|-------------------|
| Chart-Analyse | `chartAnalysis` | Ja | Nein |
| Portfolio Insights | `portfolioInsights` | Nein | Nein |
| Chat-Assistent | `chatAssistant` | Nein | Nein |
| PDF OCR | `pdfOcr` | Ja | Nein |
| CSV-Import | `csvImport` | Nein | Nein |
| Kursquellen-Assistent | `quoteAssistant` | Nein | Nein |
| Nachrichten-Recherche | `newsResearch` | Nein | Ja |

**Logos:** `AIProviderLogo` in `src/components/common/AIProviderLogo.tsx`

---

## SQLite Schema (Kerntabellen)

```sql
pp_security (id, uuid, name, currency, isin, wkn, ticker, feed, is_retired, custom_logo, attributes)
pp_account (id, uuid, name, currency, is_retired, attributes)
pp_portfolio (id, uuid, name, reference_account_id, is_retired, attributes)
pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note, other_account_id, other_portfolio_id)
pp_txn_unit (id, txn_id, unit_type, amount, currency, forex_amount, forex_currency, exchange_rate)
pp_cross_entry (id, entry_type, from_txn_id, to_txn_id, portfolio_txn_id, account_txn_id)
pp_price (security_id, date, value, volume PRIMARY KEY)
pp_latest_price (security_id PRIMARY KEY, date, value, high, low, volume)
pp_exchange_rate (base_currency, target_currency, date, rate PRIMARY KEY)
pp_fifo_lot (id, security_id, portfolio_id, purchase_txn_id, purchase_date, original_shares, remaining_shares, gross_amount, net_amount, currency)
pp_fifo_consumption (id, lot_id, sale_txn_id, shares_consumed, gross_amount, net_amount)
pp_investment_plan (id, uuid, name, security_id, portfolio_id, account_id, amount, fees, taxes, interval, start_date, auto_generate, plan_type, note, attributes)
pp_dashboard, pp_settings, pp_client_properties, pp_chart_drawing, pp_pattern_history
```

## FIFO Cost Basis

| Begriff | Feld | Beschreibung |
|---------|------|--------------|
| **Einstandswert** | `gross_amount` | Kaufpreis MIT Gebühren/Steuern |
| **Netto-Kaufpreis** | `net_amount` | OHNE Gebühren/Steuern |
| **Einstandskurs** | `gross_amount / shares` | Pro Aktie |

---

## Zustand Stores

```typescript
useUIStore: { currentView, sidebarCollapsed, scrollTarget, pdfImportModalOpen, pdfImportInitialPath }
useAppStore: { isLoading, error }
useSettingsStore: {
  language: 'de' | 'en', theme: 'light' | 'dark' | 'system', baseCurrency,
  // Quote Provider Keys: brandfetch, finnhub, coingecko, alphaVantage, twelveData
  // AI: aiProvider, aiModel, anthropic/openai/gemini/perplexityApiKey
  aiFeatureSettings: { chartAnalysis, portfolioInsights, chatAssistant, pdfOcr, csvImport, quoteAssistant }
}
// toast.success/error/info/warning(msg)
```

---

## Views

Dashboard, WidgetDashboard, Portfolio, Securities, Accounts, Transactions, Holdings, Dividends, Watchlist, Taxonomies, Benchmark, Charts, Plans, Reports, Rebalancing, Optimization, AssetStatement, Consortium, Screener, Settings - alle ✅ implementiert.

---

## Security (WICHTIG!)

| Maßnahme | Modul | Beschreibung |
|----------|-------|--------------|
| **CSP aktiviert** | `tauri.conf.json` | Content Security Policy verhindert XSS |
| **Permissions** | `capabilities/default.json` | Keine direkten FS/Shell-Permissions |
| **Pfadvalidierung** | `security/mod.rs` | `validate_file_path()`, `validate_file_path_with_extension()` |
| **AI-Commands** | `ai/command_parser.rs` | Nur Suggestions, User-Bestätigung erforderlich |
| **PDF-OCR Consent** | `PdfImportModal.tsx` | Explizite Zustimmung für KI-Upload |
| **ZIP-Bomb-Schutz** | `protobuf/parser.rs` | `MAX_UNCOMPRESSED_SIZE` (500 MB) |
| **Rate Limiting** | `security/mod.rs` | `check_rate_limit()` — aktiv für Quotes, AI-Chat, Chart-Analyse |
| **SQL-Härtung** | `ai/sql_executor.rs` | Comment-Stripping, Semicolon-Schutz, CTE-Support (WITH+SELECT erlaubt) |
| **Input-Sanitization** | `commands/crud.rs` | `sanitize_string()` für Security/Account-Namen |
| **Stale-Rate-Schutz** | `currency/mod.rs` | Wechselkurse > 180 Tage werden abgelehnt |
| **API-Keys** | `secureStorage.ts` | `tauri-plugin-store`, nie localStorage |
| **D&D Schutz** | `App.tsx` | `preventDefault()` verhindert Browser-Default |

### Secure Storage (KRITISCH!)

**ALLE API-Keys MÜSSEN ausschließlich über den sicheren Speicher gespeichert werden. Keine Ausnahme!**

API-Keys liegen in `app_data_dir/secure-keys.json` via `tauri-plugin-store`. Hook: `useSecureApiKeys()`

```typescript
const { keys, setApiKey, isSecureStorageAvailable } = useSecureApiKeys();
await setApiKey('divvyDiary', 'mein-key');  // Speichert in secure-keys.json + synct zu Zustand
```

**VERBOTEN:**
- `useSettingsStore.getState().setXxxApiKey(key)` — Speichert NUR in Zustand (flüchtig!), Key ist nach Neustart weg
- Zustand `partialize()` schließt bewusst ALLE API-Keys von localStorage aus
- Einziger korrekter Weg: `useSecureApiKeys().setApiKey(keyType, value)`

**Architektur:**
1. `secureStorage.ts` → Low-level Read/Write in `secure-keys.json`
2. `useSecureApiKeys()` → Hook: lädt Keys beim Mount, synct zu Zustand für UI-Zugriff
3. Zustand Store → Nur Lese-Cache für andere Komponenten, NIE direkt beschreiben!

**Lesen von API-Keys:**
- Komponenten die Keys NUTZEN (nicht editieren) → `useSecureApiKeys().keys.xxxApiKey`
- NICHT aus Zustand lesen (`useSettingsStore().xxxApiKey`) — Zustand startet leer, Sync ist async!

**Einheitliches Vorgehen für Features mit API-Keys:**
- API-Key-Eingabe/Änderung gehört **NUR in Settings** (`views/Settings/index.tsx`)
- Feature-Dialoge (Export-Modals, etc.) dürfen Keys **nur lesen**, nie editieren
- Wenn kein Key hinterlegt → auf Einstellungen verweisen (Redirect oder Link)
- Beispiel: DivvyDiary Export → liest Key aus `useSecureApiKeys()`, bei fehlendem Key → "Zu den Einstellungen"
- **API-Keys NIEMALS in der UI anzeigen** — auch nicht teilweise (z.B. `key.slice(0,8)...`). Key-Anzeige gehört ausschließlich in Settings (mit Eye-Toggle).
- **Kein Verbindungsstatus-Banner** in Feature-Dialogen. Verbindung zeigt sich implizit: Daten laden = funktioniert, Fehler = Fehlermeldung. Kein extra "Verbunden"-Badge.

**Aktuell registrierte Key-Typen:** `brandfetch`, `finnhub`, `coingecko`, `alphaVantage`, `twelveData`, `anthropic`, `openai`, `gemini`, `perplexity`, `divvyDiary`, `twitterClientId`

### Bei neuen Commands IMMER

1. Pfade: `security::validate_file_path()` verwenden
2. Input: `security::sanitize_string()` für Dateinamen
3. AI: Nur Suggestions zurückgeben, nie auto-ausführen
4. Externe Uploads: Consent-Flag erforderlich
5. Kein `.unwrap()` bei User-Input
6. API-Keys nie loggen
7. **API-Keys: NUR über `useSecureApiKeys().setApiKey()` speichern — niemals direkt über Zustand-Setter!**

---

## Bekannte Fallen

1. **Holdings ≠ FIFO-Lots** - Niemals FIFO-Lots für Stückzahlen
2. **TRANSFER vs DELIVERY** - TRANSFER hat CrossEntry, DELIVERY nicht
3. **SECURITY_TRANSFER** - Erzeugt zwei Transaktionen
4. **Retired Portfolios** - Holdings trotzdem anzeigen wenn > 0
5. **ISIN-Aggregation** - Securities mit gleicher ISIN zusammenfassen
6. **Yahoo-Symbole** - Internationale haben Suffix (.DE, .WA), US nicht
7. **AI Raw Strings** - `r#"..."#` nicht mit `"#` im Inhalt (benutze `r##"..."##`)
8. **GBX/GBp Währung** - Durch 100 teilen für GBP-Wert
9. **DELIVERY_INBOUND/OUTBOUND** - Im ChatBot als "BUY (Einlieferung)" / "SELL (Auslieferung)"
10. **Transaktionsänderungen** - IMMER: `fifo::build_fifo_lots()` + `emit_data_changed()`
11. **PDF Import Duplikate** - `get_duplicate_check_types()` nutzen (BUY kann als DELIVERY_INBOUND in DB)
12. **Merger/Fusion** - DELIVERY_OUTBOUND + DELIVERY_INBOUND + optional DIVIDENDS
13. **Portfolio-Optimierung** - Monte Carlo 10.000 Sim., min. 30 Datenpunkte pro Security
14. **Running Balance** - INFLOWS vor OUTFLOWS am gleichen Tag
15. **D&D** - Kein `stopPropagation()` - blockiert Tauri's `onDragDropEvent`
16. **AI-Markdown** - `<SafeMarkdown>` statt `<ReactMarkdown>` (XSS-Schutz)
17. **PDF Parser** - `strict_mode: true` Default, `parse_date_strict()` verwenden
18. **Wechselkurse X/EUR** - NIEMALS direkte X/EUR Kurse (z.B. USD/EUR) in `pp_exchange_rate` speichern! EZB liefert nur EUR/X Kurse. Der Code invertiert automatisch: `get_exchange_rate()` sucht erst direkt, dann invers (1/rate). Falsche direkte Einträge (z.B. USD/EUR=1.16 statt 0.85) führen zu massiv falschen Portfoliowerten!
19. **ChatBot Bild-Erkennung** - 🚧 OFFEN: LLM ignoriert `[[EXTRACTED_TRANSACTIONS:...]]` Command-Anweisung im System-Prompt (`prompts.rs`). Gibt nur Text-Zusammenfassung aus statt Command → keine Transaktion-Vorschau erscheint. **Lösungsansätze:** (a) Prompt aggressiver formulieren, (b) Few-shot Examples, (c) Post-Processing: Text→Command extrahieren, (d) Function Calling/Tool Use statt Text-Commands, (e) Anderes LLM-Modell testen
20. **Wechselkurs-Richtung (ChatBot)** - `exchangeRate` = EUR/Foreign (z.B. 1.1939 = 1 EUR = 1.1939 USD). Umrechnung Foreign→EUR: `amount_eur = foreign_amount / rate`. NIEMALS `* rate`!
21. **AiQuoteSuggestion JSON** - Struct nutzt `camelCase`, aber AI-Prompt gibt `feed_url` (snake_case) aus → `#[serde(alias = "feed_url")]` nötig
22. **API-Keys NUR über Secure Storage** - NIEMALS `useSettingsStore.getState().setXxxApiKey()` zum Speichern verwenden! Zustand schließt API-Keys von localStorage aus (`partialize`). Immer `useSecureApiKeys().setApiKey(keyType, value)` nutzen, sonst ist der Key nach App-Neustart weg!

---

## Tauri Events

```rust
emit_data_changed(&app, DataChangedPayload::transaction("created", security_id));
emit_data_changed(&app, DataChangedPayload::import(affected_security_ids));
emit_data_changed(&app, DataChangedPayload::rebalance(affected_security_ids));
```

Frontend: `listen('data_changed', ...)` → `invalidateAllQueries()` + `loadDbData()`

| Command | Event |
|---------|-------|
| `create/update/delete_transaction` | `transaction(...)` |
| `import_pdf_transactions`, `import_transactions_csv` | `import(...)` |
| `execute_rebalance` | `rebalance(...)` |
| `apply_stock_split`, `apply_merger` | `transaction(...)` |

---

## Datenformat (.portfolio)

- **Container:** ZIP-Archiv mit `data.portfolio`
- **Header:** `PPPBV1` (6 Bytes)
- **Body:** Protocol Buffers (prost)
- **Referenzen:** Index-basiert → UUID-Auflösung

**Round-Trip:** Securities, Accounts, Portfolios, Transactions, Investment Plans, Dashboards, Settings, Properties. Details: `PP_IMPORT_EXPORT.md`

---

## UI Design

**Layout:** `p-4` Cards, `space-y-4` Sektionen
**Farben:** `text-green-600` positiv, `text-red-600` negativ, `text-muted-foreground`
**Icons:** Lucide React
**Sprache:** Durchgehend informelles Deutsch ("du"-Form), kein "Sie"

### Modal-Footer-Pattern (VERBINDLICH)

Alle Modals verwenden denselben Footer-Aufbau (Referenz: `PdfExportModal.tsx`):

```tsx
<div className="flex justify-end gap-2 p-4 border-t border-border">
  <button className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors">
    Abbrechen
  </button>
  <button className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed">
    <Icon size={16} />
    Aktion
  </button>
</div>
```

**Regeln:**
- `flex justify-end gap-2 p-4 border-t border-border` als Footer-Container
- Sekundär-Button: `border border-border`, Text "Abbrechen"
- Primär-Button: `bg-primary text-primary-foreground`, Icon (16px) + kurzer Text
- Immer `disabled:opacity-50 disabled:cursor-not-allowed` bei disabled-Buttons
- **Keine hardcodierten Farben** (`#ff8a4c` etc.) — immer Theme-Variablen nutzen

### ChatBot Bestätigungen (UI-Konsistenz)

Alle Aktionen mit Bestätigung (Watchlist, Transaktionen, Transfers) nutzen:
- Amber Container (`border-amber-500/50`, `bg-amber-500/5`)
- AlertTriangle/Receipt Icon
- Vollbreite Buttons: grün=Bestätigen, muted=Abbrechen
- `p-4` Padding
- **NIEMALS** Icon-only Buttons für Bestätigungen!

### ChatBot Commands

- `[[WATCHLIST:{"action":"add","name":"...","security":"..."}]]`
- SQL-Abfragen: LLM generiert ```sql``` Code-Blöcke (dynamisches SQL via `sql_executor.rs`)

### ChatBot SQL-System

Das LLM generiert dynamisch SQL-Abfragen als ```sql``` Code-Blöcke. Validierung:
- Nur `SELECT` erlaubt
- Nur `pp_*` Tabellen
- Session-basierte Pattern-Approval (`sqlApprovalMode: 'always' | 'session' | 'never'`)

**Skalierungsfaktoren in SQL:** `shares / 100000000.0`, `amount / 100.0`, `prices / 100000000.0`

### Speech-to-Text (Whisper)

**Nur verfügbar wenn ChatBot-Provider auf OpenAI eingestellt ist.**

- Mikrofon-Button erscheint neben dem Bild-Button
- Nutzt OpenAI Whisper API für Transkription
- Sprache: Deutsch (hardcoded)
- Audio-Format: WebM

Tauri Command: `transcribe_audio(audio_base64, api_key, language?)`

### Chart Features

- **Zeichenwerkzeuge:** Trendlinien, Horizontal, Fibonacci (0%-100%)
- **Pattern-Erkennung:** 22 Candlestick-Patterns mit Trend-Kontext
- **Erweiterte Analyse:** Indikator-Werte, Volumen, Alerts, Risk/Reward

---

## Validierung nach Import

```sql
-- Holdings pro Portfolio
SELECT p.name, s.name, SUM(CASE
    WHEN t.txn_type IN ('BUY','TRANSFER_IN','DELIVERY_INBOUND') THEN t.shares
    WHEN t.txn_type IN ('SELL','TRANSFER_OUT','DELIVERY_OUTBOUND') THEN -t.shares
END) / 100000000.0 as shares
FROM pp_txn t JOIN pp_portfolio p ON p.id = t.owner_id JOIN pp_security s ON s.id = t.security_id
WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
GROUP BY p.id, s.id HAVING shares > 0;

-- FIFO Lots
SELECT s.name, l.remaining_shares / 100000000.0, l.gross_amount / 100.0 as cost_basis
FROM pp_fifo_lot l JOIN pp_security s ON s.id = l.security_id WHERE l.remaining_shares > 0;
```
