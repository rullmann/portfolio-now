# Portfolio Now

Cross-Platform Desktop-App zur Portfolio-Verwaltung. Neuimplementierung von [Portfolio Performance](https://github.com/portfolio-performance/portfolio) mit Tauri (Rust + React/TypeScript).

| Eigenschaft | Wert |
|-------------|------|
| **Bundle ID** | `com.portfolio-now.app` |
| **Version** | 0.1.0 |

## Architektur

```
apps/desktop/
├── src/                    # React Frontend (TypeScript)
│   ├── store/              # Zustand State Management
│   ├── components/         # UI (layout/, common/, modals/, charts/, chat/)
│   │   ├── common/         # Shared (Skeleton, DropdownMenu, AIProviderLogo, ...)
│   │   ├── charts/         # TradingViewChart, AIAnalysisPanel
│   │   ├── chat/           # ChatPanel, ChatMessage, ChatButton
│   │   └── modals/         # PortfolioInsightsModal, TransactionFormModal, etc.
│   ├── views/              # View-Komponenten pro Route
│   └── lib/                # API, Types, Hooks
└── src-tauri/              # Rust Backend
    └── src/
        ├── commands/       # Tauri IPC Commands (24 Module)
        ├── db/             # SQLite (rusqlite)
        ├── pp/             # Portfolio Performance Datenmodelle
        ├── protobuf/       # .portfolio Parser
        ├── quotes/         # Kursquellen (Yahoo, Finnhub, EZB, etc.)
        ├── fifo/           # FIFO Cost Basis
        ├── pdf_import/     # PDF Import mit OCR (Vision API)
        └── ai/             # KI-Analyse, Chat, Portfolio Insights, Models Registry
```

## Tech Stack

**Frontend:** React 18, TypeScript, Vite, TailwindCSS, Zustand, Recharts, Lightweight Charts v5, Lucide Icons
**Backend:** Tauri 2.9, Rust, SQLite, prost (Protobuf), Tokio, reqwest
**Build:** pnpm Workspaces, Turbo

## Entwicklung

```bash
pnpm install              # Installation
pnpm desktop              # Dev Server mit Hot Reload
pnpm desktop:build        # Release Build
pnpm lint                 # Linting

# Rust Tests
cd apps/desktop/src-tauri && cargo test --release
```

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
FROM pp_txn WHERE owner_type = 'portfolio'
GROUP BY security_id, owner_id
```

---

## Transaktionstypen

**PortfolioTransaction:** `BUY`, `SELL`, `TRANSFER_IN`, `TRANSFER_OUT`, `DELIVERY_INBOUND`, `DELIVERY_OUTBOUND`

**AccountTransaction:** `DEPOSIT`, `REMOVAL`, `INTEREST`, `INTEREST_CHARGE`, `DIVIDENDS`, `FEES`, `FEES_REFUND`, `TAXES`, `TAX_REFUND`, `BUY`, `SELL`, `TRANSFER_IN`, `TRANSFER_OUT`

---

## Tauri Commands (Kurzübersicht)

### File & Import
- `import_pp_file(path)` - PP-Datei Import mit Progress
- `export_database_to_portfolio(path)` - Export Round-Trip
- `rebuild_fifo_lots()` - FIFO-Lots neu berechnen

### Data
- `get_securities()`, `get_accounts()`, `get_pp_portfolios()`
- `get_transactions(owner_type?, owner_id?, security_id?, limit?, offset?)`
- `get_holdings(portfolio_id)`, `get_all_holdings()` - ISIN-aggregiert
- `get_portfolio_summary()`, `get_portfolio_history()`
- `get_price_history(security_id, start_date?, end_date?)`
- `get_fifo_cost_basis_history(security_id)` - Cost Basis für Chart

### CRUD
- `create_security(data)` - Mit ISIN-Validierung (Luhn-Check)
- `update_security(id, data)`, `delete_security(id)`, `retire_security(id)`
- Analog für: `account`, `portfolio`, `transaction`

### Quotes
- `sync_all_prices(only_held?, api_keys?)` - Alle Kurse synchronisieren
- `sync_security_prices(security_ids, api_keys?)` - Einzelne Securities
- `fetch_historical_prices(security_id, from, to, api_keys?)`
- `search_external_securities(query)` - Yahoo + Alpha Vantage Suche
- `fetch_exchange_rates()`, `fetch_exchange_rate(base, target)`

### Performance & Reports
- `calculate_performance(portfolio_id?, from?, to?)` - TTWROR, IRR
- `calculate_benchmark_comparison(benchmark_id, from?, to?)` - Alpha, Beta, Sharpe
- `get_dividend_report()`, `get_realized_gains_report()`, `get_tax_report(year)`

### Features
- `get_watchlists()`, `add_to_watchlist()`, `remove_from_watchlist()`
- `get_taxonomies()`, `get_taxonomy_allocations()`
- `get_investment_plans()`, `execute_investment_plan()`
- `preview_rebalance()`, `execute_rebalance()`
- `record_stock_split()`, `record_spinoff()`, `record_merger()`

### AI Features
- `analyze_chart_with_ai(request)` - Chart-Bild mit KI analysieren
- `analyze_chart_with_annotations(request)` - Chart-Analyse mit strukturierten Markern
- `analyze_portfolio_with_ai(request)` - Portfolio Insights (Stärken, Risiken, Empfehlungen)
- `chat_with_portfolio_assistant(request)` - KI-Chat über Portfolio-Daten
- `get_ai_models(provider, api_key)` - Verfügbare Modelle von Provider-API laden
- `get_vision_models(provider)` - Vision-fähige Modelle aus Registry

### AI Helper Commands (ChatBot Actions)
- `ai_search_security(query, api_key?)` - Security in DB + extern suchen
- `ai_add_to_watchlist(watchlist, security, api_key?)` - Security zur Watchlist (mit Enrichment)
- `ai_remove_from_watchlist(watchlist, security)` - Security von Watchlist entfernen
- `ai_list_watchlists()` - Alle Watchlists mit Securities auflisten
- `ai_query_transactions(security?, year?, type?, limit?)` - Transaktionen filtern

---

## Quote Provider

| Provider | API Key | Beschreibung |
|----------|---------|--------------|
| **Yahoo** | Nein | Kostenlos, aktuell + historisch |
| **Portfolio Report** | Nein | ISIN/WKN-Lookup, Kurse (wie PP) |
| **Finnhub** | Ja | US-Aktien, Premium für Historie |
| **AlphaVantage** | Ja | 25 Calls/Tag free |
| **CoinGecko** | Nein | Kryptowährungen |
| **EZB** | Nein | Wechselkurse |

---

## AI Provider

| Provider | API Key | Modelle | Besonderheiten |
|----------|---------|---------|----------------|
| **Claude** | Ja | claude-sonnet-4-5, claude-haiku-4-5 | Vision + **direkter PDF-Upload** |
| **OpenAI** | Ja | o3, o4-mini, gpt-4.1, gpt-4o, gpt-4o-mini | o3/o4: Vision + **Web-Suche** |
| **Gemini** | Ja | gemini-3-flash, gemini-3-pro | Vision + **direkter PDF-Upload** |
| **Perplexity** | Ja | sonar-pro, sonar | Vision + Web-Suche |

### PDF OCR Support

| Provider | Methode | Poppler nötig? |
|----------|---------|----------------|
| **Claude** | Direkter PDF-Upload | Nein |
| **Gemini** | Direkter PDF-Upload | Nein |
| **OpenAI** | PDF → Bilder → Vision | Ja (`brew install poppler`) |
| **Perplexity** | PDF → Bilder → Vision | Ja |

### Web-Suche

OpenAI o3 und o4-mini Modelle unterstützen `web_search_preview` Tool für aktuelle Informationen.

### Dynamische Modell-Erkennung

Modelle werden beim Öffnen der Einstellungen automatisch von den Provider-APIs geladen:
- **Deprecated Models**: Automatische Migration auf empfohlenes Modell + Toast-Warnung beim App-Start
- **Neue Modelle**: Info-Toast wenn neue Modelle verfügbar sind
- **Refresh-Button**: Manuelle Aktualisierung der Modell-Liste

### Markdown-Normalisierung

Alle AI-Antworten werden durch `normalize_markdown_response()` nachbearbeitet:
- Konvertiert Plain-Text-Überschriften (z.B. "Trend:") zu Markdown ("## Trend")
- Entfernt Perplexity-Zitierungen wie [1], [2]
- Stellt konsistente Formatierung über alle Provider sicher

### AI Provider Logos

Offizielle SVG-Logos in `src/components/common/AIProviderLogo.tsx`:
```tsx
import { AIProviderLogo, ClaudeLogo, OpenAILogo, GeminiLogo, PerplexityLogo } from '../common/AIProviderLogo';

// Dynamisch nach Provider
<AIProviderLogo provider="claude" size={24} />
<AIProviderLogo provider="perplexity" size={24} />

// Oder einzeln
<ClaudeLogo size={20} />
<OpenAILogo size={20} />
<GeminiLogo size={20} />
<PerplexityLogo size={20} />
```

---

## SQLite Schema (Kerntabellen)

```sql
-- Securities (mit Attributes JSON)
pp_security (id, uuid, name, currency, isin, wkn, ticker, feed, is_retired, custom_logo, attributes)

-- Accounts & Portfolios (mit Attributes JSON)
pp_account (id, uuid, name, currency, is_retired, attributes)
pp_portfolio (id, uuid, name, reference_account_id, is_retired, attributes)

-- Transactions (mit Transfer-Tracking)
pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note,
        other_account_id, other_portfolio_id)
pp_txn_unit (id, txn_id, unit_type, amount, currency, forex_amount, forex_currency, exchange_rate)
pp_cross_entry (id, entry_type, from_txn_id, to_txn_id, portfolio_txn_id, account_txn_id)

-- Prices
pp_price (security_id, date, value PRIMARY KEY)
pp_latest_price (security_id PRIMARY KEY, date, value, high, low, volume)
pp_exchange_rate (base_currency, target_currency, date, rate PRIMARY KEY)

-- FIFO Cost Basis
pp_fifo_lot (id, security_id, portfolio_id, purchase_txn_id, purchase_date,
             original_shares, remaining_shares, gross_amount, net_amount, currency)
pp_fifo_consumption (id, lot_id, sale_txn_id, shares_consumed, gross_amount, net_amount)

-- Investment Plans (erweitert)
pp_investment_plan (id, uuid, name, security_id, portfolio_id, account_id, amount, fees, taxes,
                    interval, start_date, auto_generate, plan_type, note, attributes)

-- Dashboards & Settings
pp_dashboard (id, import_id, dashboard_id, name, columns_json, configuration_json)
pp_settings (id, import_id, settings_json)
pp_client_properties (id, import_id, key, value)
```

---

## FIFO Cost Basis

| Begriff | Feld | Beschreibung |
|---------|------|--------------|
| **Einstandswert** | `gross_amount` | Kaufpreis MIT Gebühren/Steuern |
| **Netto-Kaufpreis** | `net_amount` | Kaufpreis OHNE Gebühren/Steuern |
| **Einstandskurs** | `gross_amount / shares` | Pro Aktie |

---

## Zustand Stores

```typescript
// UI State (LocalStorage)
useUIStore: { currentView, sidebarCollapsed, scrollTarget, setCurrentView, toggleSidebar, setScrollTarget }

// App State
useAppStore: { isLoading, error, setLoading, setError, clearError }

// Settings (LocalStorage, Version 4)
useSettingsStore: {
  language: 'de' | 'en',
  theme: 'light' | 'dark' | 'system',
  baseCurrency: string,
  // Quote Provider Keys
  brandfetchApiKey, finnhubApiKey, coingeckoApiKey, alphaVantageApiKey, twelveDataApiKey,
  // AI Provider
  aiProvider: 'claude' | 'openai' | 'gemini' | 'perplexity',
  aiModel: string,  // z.B. 'claude-sonnet-4-5-20250514'
  anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey,
  // Transient (nicht persistiert)
  pendingModelMigration: { from, to, provider } | null
}

// AI_MODELS Konstante (Fallback wenn API nicht erreichbar)
AI_MODELS: { claude: [...], openai: [...], gemini: [...], perplexity: [...] }

// DEPRECATED_MODELS Mapping für Auto-Upgrade (inkl. non-vision Modelle)
DEPRECATED_MODELS: { 'sonar-reasoning-pro': 'sonar-pro', 'o3': 'gpt-4.1', ... }

// Toast
toast.success(msg), toast.error(msg), toast.info(msg), toast.warning(msg)
```

---

## Views

| View | Status | Beschreibung |
|------|--------|--------------|
| Dashboard | ✅ | Depotwert, Holdings, Mini-Charts, KI Insights, Sync-Button |
| Portfolio | ✅ | CRUD, History Chart |
| Securities | ✅ | CRUD, Logos, Sync-Button |
| Accounts | ✅ | CRUD, Balance-Tracking |
| Transactions | ✅ | Filter, Pagination |
| Holdings | ✅ | Donut-Chart mit Logos |
| Dividends | ✅ | Dividenden-Übersicht mit Logos |
| Watchlist | ✅ | Multiple Listen, Mini-Charts, ChatBot-Integration |
| Taxonomies | ✅ | Hierarchischer Baum |
| Benchmark | ✅ | Performance-Vergleich |
| Charts | ✅ | Candlestick, RSI, MACD, Bollinger, KI-Analyse + Marker |
| Plans | ✅ | Sparpläne |
| Reports | ✅ | Performance, Dividenden, Gewinne, Steuer mit Charts |
| Rebalancing | ✅ | Zielgewichtung, Vorschau, Ausführung |
| Settings | ✅ | Sprache, Theme, API Keys, KI-Provider (4 Provider) |

---

## Bekannte Fallen

1. **Holdings vs FIFO:** Niemals FIFO-Lots für Stückzahlen verwenden
2. **TRANSFER vs DELIVERY:** TRANSFER hat CrossEntry, DELIVERY nicht
3. **SECURITY_TRANSFER:** Erzeugt zwei Transaktionen
4. **Retired Portfolios:** Holdings trotzdem anzeigen wenn > 0
5. **ISIN-Aggregation:** Securities mit gleicher ISIN zusammenfassen
6. **Yahoo-Symbole:** Internationale haben Suffix (.DE, .WA), US nicht
7. **AI Raw Strings:** In Rust `r#"..."#` nicht mit `"#` im Inhalt verwenden (benutze `r##"..."##`)
8. **GBX/GBp Währung:** British Pence durch 100 teilen für GBP-Wert
9. **AI Portfolio-Kontext:** Währungsumrechnung in Basiswährung beachten
10. **TwelveData Warnings:** Ungenutzte Felder in `quotes/twelvedata.rs` (harmlos, für API-Kompatibilität)
11. **DELIVERY_INBOUND/OUTBOUND:** Werden im ChatBot als "BUY (Einlieferung)" / "SELL (Auslieferung)" angezeigt

---

## Datenformat (.portfolio)

- **Container:** ZIP-Archiv mit `data.portfolio`
- **Header:** `PPPBV1` (6 Bytes)
- **Body:** Protocol Buffers (prost)
- **Referenzen:** Index-basiert → UUID-Auflösung

### Round-Trip Support (Import → Export)

Folgende Daten überleben einen vollständigen Import/Export-Zyklus:

| Entität | Felder |
|---------|--------|
| **Securities** | attributes, note, updated_at, latest_feed, latest_feed_url |
| **Accounts** | attributes, updated_at |
| **Portfolios** | attributes |
| **Transactions** | other_account_uuid, other_portfolio_uuid (Transfer-Tracking) |
| **Investment Plans** | fees, taxes, plan_type, note, attributes |
| **Dashboards** | name, id, columns (mit widgets) |
| **Settings** | bookmarks, attribute_types, configuration_sets |
| **Properties** | key-value Paare |

Siehe `apps/desktop/src-tauri/PP_IMPORT_EXPORT.md` für Details.

---

## UI Design

**Kompaktes Layout:** `p-4` für Cards, `space-y-4` zwischen Sektionen
**Farben:** `text-green-600` (positiv), `text-red-600` (negativ), `text-muted-foreground`
**Icons:** Lucide React

### Header
- **View-Titel** links
- **AI-Indikator** (wenn konfiguriert): Provider-Logo + Name + Modell
- **Aktionen** rechts: Importieren, Refresh, Neue Buchung

### AI Features
- **Portfolio Insights Modal**: KI-Analyse mit farbcodierten Karten (grün=Stärken, orange=Risiken, blau=Empfehlungen)
- **Chat Panel**: Floating Button unten rechts, Slide-in Chat für Portfolio-Fragen
  - Resizable (links ziehen, 320-800px)
  - Farbcodierte Nachrichten (blau=User, orange=Bot)
  - Einzelne Nachrichten löschbar (X-Button bei Hover)
  - Watchlist-Integration: "Füge Apple zur Watchlist hinzu"
  - Transaktions-Abfragen: "Zeige alle Käufe 2024"
  - Historische Daten: Verkaufte Positionen, Jahresübersicht
- **Chart Marker**: Support/Resistance-Linien werden direkt im Chart angezeigt

### ChatBot Commands (intern)
Der ChatBot kann folgende Aktionen ausführen:
- `[[WATCHLIST:{"action":"add","name":"...","security":"..."}]]` - Security zur Watchlist hinzufügen
- `[[QUERY_TRANSACTIONS:{"security":"...","year":2024,"type":"BUY"}]]` - Transaktionen abfragen

### Watchlist Security Enrichment
Beim Hinzufügen via ChatBot werden automatisch:
1. **ISIN/WKN** von Portfolio Report ermittelt
2. **Aktueller Kurs** von Yahoo Finance geladen
3. **3 Monate Historie** für Mini-Charts abgerufen

---

## Validierung nach Import

```sql
-- Holdings pro Portfolio
SELECT p.name, s.name, SUM(CASE
    WHEN t.txn_type IN ('BUY','TRANSFER_IN','DELIVERY_INBOUND') THEN t.shares
    WHEN t.txn_type IN ('SELL','TRANSFER_OUT','DELIVERY_OUTBOUND') THEN -t.shares
END) / 100000000.0 as shares
FROM pp_txn t
JOIN pp_portfolio p ON p.id = t.owner_id
JOIN pp_security s ON s.id = t.security_id
WHERE t.owner_type = 'portfolio' AND t.shares IS NOT NULL
GROUP BY p.id, s.id HAVING shares > 0;

-- FIFO Lots
SELECT s.name, l.remaining_shares / 100000000.0, l.gross_amount / 100.0 as cost_basis
FROM pp_fifo_lot l JOIN pp_security s ON s.id = l.security_id
WHERE l.remaining_shares > 0;
```
