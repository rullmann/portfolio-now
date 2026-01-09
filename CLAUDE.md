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
│   ├── components/         # UI (layout/, common/, modals/, charts/)
│   ├── views/              # View-Komponenten pro Route
│   └── lib/                # API, Types, Hooks
└── src-tauri/              # Rust Backend
    └── src/
        ├── commands/       # Tauri IPC Commands (20 Module)
        ├── db/             # SQLite (rusqlite)
        ├── pp/             # Portfolio Performance Datenmodelle
        ├── protobuf/       # .portfolio Parser
        ├── quotes/         # Kursquellen (Yahoo, Finnhub, EZB, etc.)
        └── fifo/           # FIFO Cost Basis
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

---

## Quote Provider

| Provider | API Key | Beschreibung |
|----------|---------|--------------|
| **Yahoo** | Nein | Kostenlos, aktuell + historisch |
| **Finnhub** | Ja | US-Aktien, Premium für Historie |
| **AlphaVantage** | Ja | 25 Calls/Tag free |
| **CoinGecko** | Nein | Kryptowährungen |
| **EZB** | Nein | Wechselkurse |

---

## SQLite Schema (Kerntabellen)

```sql
-- Securities
pp_security (id, uuid, name, currency, isin, wkn, ticker, feed, is_retired, custom_logo)

-- Accounts & Portfolios
pp_account (id, uuid, name, currency, is_retired)
pp_portfolio (id, uuid, name, reference_account_id, is_retired)

-- Transactions
pp_txn (id, uuid, owner_type, owner_id, security_id, txn_type, date, amount, currency, shares, note)
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
useUIStore: { currentView, sidebarCollapsed, setCurrentView, toggleSidebar }

// App State
useAppStore: { isLoading, error, setLoading, setError, clearError }

// Settings (LocalStorage)
useSettingsStore: {
  language: 'de' | 'en',
  theme: 'light' | 'dark' | 'system',
  baseCurrency: string,
  brandfetchApiKey, finnhubApiKey
}

// Toast
toast.success(msg), toast.error(msg), toast.info(msg), toast.warning(msg)
```

---

## Views

| View | Status | Beschreibung |
|------|--------|--------------|
| Dashboard | ✅ | Depotwert, Holdings, Mini-Charts |
| Portfolio | ✅ | CRUD, History Chart |
| Securities | ✅ | CRUD, Logos, Sync-Button |
| Accounts | ✅ | CRUD, Balance-Tracking |
| Transactions | ✅ | Filter, Pagination |
| Holdings | ✅ | Donut-Chart mit Logos |
| Watchlist | ✅ | Multiple Listen, Mini-Charts |
| Taxonomies | ✅ | Hierarchischer Baum |
| Benchmark | ✅ | Performance-Vergleich |
| Charts | ✅ | Candlestick, RSI, MACD, Bollinger |
| Plans | ✅ | Sparpläne |
| Reports | 🔄 | Backend fertig, UI in Arbeit |
| Rebalancing | 🔄 | Backend fertig, UI in Arbeit |
| Settings | ✅ | Sprache, Theme, API Keys |

---

## Bekannte Fallen

1. **Holdings vs FIFO:** Niemals FIFO-Lots für Stückzahlen verwenden
2. **TRANSFER vs DELIVERY:** TRANSFER hat CrossEntry, DELIVERY nicht
3. **SECURITY_TRANSFER:** Erzeugt zwei Transaktionen
4. **Retired Portfolios:** Holdings trotzdem anzeigen wenn > 0
5. **ISIN-Aggregation:** Securities mit gleicher ISIN zusammenfassen
6. **Yahoo-Symbole:** Internationale haben Suffix (.DE, .WA), US nicht

---

## Datenformat (.portfolio)

- **Container:** ZIP-Archiv mit `data.portfolio`
- **Header:** `PPPBV1` (6 Bytes)
- **Body:** Protocol Buffers (prost)
- **Referenzen:** Index-basiert → UUID-Auflösung

---

## UI Design

**Kompaktes Layout:** `p-4` für Cards, `space-y-4` zwischen Sektionen
**Farben:** `text-green-600` (positiv), `text-red-600` (negativ), `text-muted-foreground`
**Icons:** Lucide React

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
