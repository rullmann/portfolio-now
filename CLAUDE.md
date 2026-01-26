# Portfolio Now

Cross-Platform Desktop-App zur Portfolio-Verwaltung. Neuimplementierung von [Portfolio Performance](https://github.com/portfolio-performance/portfolio) mit Tauri (Rust + React/TypeScript).

| Eigenschaft | Wert |
|-------------|------|
| **Bundle ID** | `com.portfolio-now.app` |
| **Version** | 0.1.6 |
| **Jahr** | 2026 |

## Build-Hinweise

- **KEINE Mac DMG bauen** - nur Development-Builds verwenden
- Für Release-Builds: `pnpm tauri build --bundles app`

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
│   ├── components/         # UI (layout/, common/, modals/, charts/, chat/)
│   │   ├── common/         # Shared (Skeleton, DropdownMenu, AIProviderLogo, SafeMarkdown, ...)
│   │   ├── charts/         # TradingViewChart, AIAnalysisPanel, DrawingTools, SignalsPanel
│   │   ├── chat/           # ChatPanel, ChatMessage, ChatButton
│   │   └── modals/         # PortfolioInsightsModal, TransactionFormModal, etc.
│   ├── views/              # View-Komponenten pro Route
│   └── lib/                # API, Types, Hooks
│       ├── indicators.ts   # Technische Indikatoren (SMA, EMA, RSI, MACD, BB, Stochastic, OBV, ADX, ATR)
│       ├── patterns.ts     # Candlestick-Pattern-Erkennung (22 Patterns)
│       └── signals.ts      # Signal-Erkennung und Divergenzen
└── src-tauri/              # Rust Backend
    └── src/
        ├── commands/       # Tauri IPC Commands (26 Module)
        ├── db/             # SQLite (rusqlite)
        ├── pp/             # Portfolio Performance Datenmodelle
        ├── protobuf/       # .portfolio Parser
        ├── quotes/         # Kursquellen (Yahoo, Finnhub, EZB, etc.)
        ├── fifo/           # FIFO Cost Basis
        ├── pdf_import/     # PDF Import mit OCR (Vision API)
        ├── ai/             # KI-Analyse, Chat, Portfolio Insights, Models Registry, Query Templates
        ├── optimization/   # Portfolio-Optimierung (Markowitz, Efficient Frontier)
        └── tax/            # Steuerberechnungen (DE: Anlage KAP)
```

## Tech Stack

**Frontend:** React 18, TypeScript, Vite, TailwindCSS, Zustand, Recharts, Lightweight Charts v5, Lucide Icons
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
| **Datumsformatierung** | `lib/types.ts` | `formatDate()`, `formatDateTime()`, `formatDateShort()` | Eigene Date-Formatierung |
| **ChatBot DB-Abfragen** | `ai/query_templates.rs` | `execute_template()`, `get_all_templates()` | Eigene SQL im ChatBot |
| **Account Running Balance** | `ai/query_templates.rs` | `account_balance_analysis` Template | Eigene Saldo-Berechnung |

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
`analyze_chart_with_ai/with_annotations/enhanced`, `analyze_portfolio_with_ai`, `chat_with_portfolio_assistant`, `get_ai_models(provider, api_key)`, `get_vision_models(provider)`

### AI Helper (ChatBot)
`ai_search_security(query, api_key?)`, `ai_add/remove_from_watchlist(watchlist, security, api_key?)`, `ai_list_watchlists()`, `ai_query_transactions(security?, year?, type?, limit?)`

### Chart Drawings & Pattern
`save/get/delete/clear_chart_drawing(s)`, `save_pattern_detection`, `evaluate_pattern_outcomes`, `get_pattern_statistics`, `get_pattern_history`

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
| **Claude** | claude-sonnet-4-5, claude-haiku-4-5 | Vision + **direkter PDF-Upload** |
| **OpenAI** | o3, o4-mini, gpt-4.1, gpt-4o, gpt-4o-mini | o3/o4: Vision + **Web-Suche** |
| **Gemini** | gemini-3-flash, gemini-3-pro | Vision + **direkter PDF-Upload** |
| **Perplexity** | sonar-pro, sonar | Vision + Web-Suche |

**PDF OCR:** Claude/Gemini = direkter Upload, OpenAI/Perplexity = Poppler nötig (`brew install poppler`)

### Unterstützte Banken (PDF Import)

**DE (24):** Baader Bank, Comdirect, Commerzbank, Consorsbank, DAB, Deutsche Bank, DKB, DZ Bank, ebase, flatex, GenoBroker, ING-DiBa, MLP Bank, OLB, OnVista, Postbank, Quirion, S Broker, Santander, Scalable Capital, Targobank, Trade Republic, 1822direkt
**CH (6):** Credit Suisse, LGT, PostFinance, Swissquote, UBS, ZKB
**AT (2):** Erste Bank, Raiffeisen
**International (4):** DEGIRO, Merkur, Revolut, Saxo Bank

### AI Feature Matrix

Jedes Feature kann eigenen Provider/Model haben in `aiFeatureSettings`:

| Feature | ID | Vision nötig? |
|---------|-----|---------------|
| Chart-Analyse | `chartAnalysis` | Ja |
| Portfolio Insights | `portfolioInsights` | Nein |
| Chat-Assistent | `chatAssistant` | Nein |
| PDF OCR | `pdfOcr` | Ja |
| CSV-Import | `csvImport` | Nein |

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
pp_price (security_id, date, value PRIMARY KEY)
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
  aiFeatureSettings: { chartAnalysis, portfolioInsights, chatAssistant, pdfOcr, csvImport }
}
// toast.success/error/info/warning(msg)
```

---

## Views

Dashboard, Portfolio, Securities, Accounts, Transactions, Holdings, Dividends, Watchlist, Taxonomies, Benchmark, Charts, Plans, Reports, Rebalancing, Optimization, Settings - alle ✅ implementiert.

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
| **Rate Limiting** | `security/mod.rs` | `check_rate_limit()` |
| **API-Keys** | `secureStorage.ts` | `tauri-plugin-store`, nie localStorage |
| **D&D Schutz** | `App.tsx` | `preventDefault()` verhindert Browser-Default |

### Secure Storage

API-Keys in `app_data_dir/secure-keys.json` via `tauri-plugin-store`. Hook: `useSecureApiKeys()`

```typescript
const { keys, setApiKey, isSecureStorageAvailable } = useSecureApiKeys();
await setApiKey('anthropic', 'sk-ant-...');
```

### Bei neuen Commands IMMER

1. Pfade: `security::validate_file_path()` verwenden
2. Input: `security::sanitize_string()` für Dateinamen
3. AI: Nur Suggestions zurückgeben, nie auto-ausführen
4. Externe Uploads: Consent-Flag erforderlich
5. Kein `.unwrap()` bei User-Input
6. API-Keys nie loggen

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

### ChatBot Bestätigungen (UI-Konsistenz)

Alle Aktionen mit Bestätigung (Watchlist, Transaktionen, Transfers) nutzen:
- Amber Container (`border-amber-500/50`, `bg-amber-500/5`)
- AlertTriangle/Receipt Icon
- Vollbreite Buttons: grün=Bestätigen, muted=Abbrechen
- `p-4` Padding
- **NIEMALS** Icon-only Buttons für Bestätigungen!

### ChatBot Commands

- `[[WATCHLIST:{"action":"add","name":"...","security":"..."}]]`
- `[[QUERY_DB:{"query":"template_id","params":{...}}]]`

### Speech-to-Text (Whisper)

**Nur verfügbar wenn ChatBot-Provider auf OpenAI eingestellt ist.**

- Mikrofon-Button erscheint neben dem Bild-Button
- Nutzt OpenAI Whisper API für Transkription
- Sprache: Deutsch (hardcoded)
- Audio-Format: WebM

Tauri Command: `transcribe_audio(audio_base64, api_key, language?)`

### Query Templates (13)

`security_transactions`, `dividends_by_security`, `all_dividends`, `transactions_by_date`, `security_cost_basis`, `sold_securities`, `holding_period_analysis`, `fifo_lot_details`, `account_transactions`, `investment_plans`, `portfolio_accounts`, `tax_relevant_sales`, `account_balance_analysis`

**account_balance_analysis:** Running Balance mit INFLOWS vor OUTFLOWS am gleichen Tag.

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
