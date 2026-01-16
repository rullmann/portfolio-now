# Portfolio Now - Release Notes

## Version 0.1.0 (Initial Release)

### Überblick

Portfolio Now ist eine Cross-Platform Desktop-App zur Portfolio-Verwaltung, inspiriert von [Portfolio Performance](https://github.com/portfolio-performance/portfolio). Die Anwendung wurde komplett neu in Rust (Backend) und TypeScript/React (Frontend) implementiert.

### Lizenz

MIT License - siehe [LICENSE](./LICENSE)

### Technologie

- **Backend:** Tauri 2.9, Rust, SQLite
- **Frontend:** React 18, TypeScript, Vite, TailwindCSS
- **Build:** pnpm Workspaces, Turbo

---

## Aktive Features (v0.1.0)

### Übersicht
- **Dashboard** - Portfolioübersicht mit Mini-Charts, Kennzahlen, KI-Insights
- **Portfolios** - CRUD-Operationen, Portfolio-History
- **Wertpapiere** - Verwaltung mit Kursquellen, Logos, Kapitalmaßnahmen (Split, Merger)
- **Konten** - Kontoverwaltung mit Salden
- **Buchungen** - Transaktionsübersicht, CSV/PDF-Import
- **Bestand** - Holdings-Ansicht mit Donut-Chart
- **Dividenden** - Übersicht, Kalender, Prognose
- **Watchlist** - Multiple Listen mit Mini-Charts

### Analyse
- **Vermögensaufstellung** - Detaillierte Aufstellung mit Export (CSV/PDF)
- **Klassifizierung** - Hierarchische Taxonomien

### Werkzeuge
- **Optimierung** - Portfolio-Optimierung (Markowitz, Efficient Frontier)
- **Technische Analyse** - Candlestick-Charts mit 6+ Indikatoren, Zeichenwerkzeuge, Pattern-Erkennung

### KI-Integration
- 4 Provider: Claude, OpenAI, Gemini, Perplexity
- Chart-Analyse mit Vision-Modellen
- Portfolio-Insights
- Chat-Assistent

---

## Versteckte Features (v0.1.0)

Die folgenden Features sind implementiert, aber in v0.1.0 ausgeblendet:

| Feature | Grund | Datei |
|---------|-------|-------|
| **Sparpläne** | Weitere Tests erforderlich | `src/views/InvestmentPlans/` |
| **Rebalancing** | Weitere Tests erforderlich | `src/views/Rebalancing/` |
| **Berichte** | Weitere Tests erforderlich | `src/views/Reports/` |
| **Benchmark** | Weitere Tests erforderlich | `src/views/Benchmark/` |
| **Portfolio-Gruppen** | Weitere Tests erforderlich | `src/views/Consortium/` |
| **Screener** | Weitere Tests erforderlich | `src/views/Screener/` |

### Reaktivierung

Um diese Features zu aktivieren, entkommentieren Sie die entsprechenden Zeilen in:
`src/store/index.ts` (navItems Array)

```typescript
// --- HIDDEN FOR v0.1.0 RELEASE (see RELEASE_NOTES.md) ---
// { id: 'benchmark', label: 'Benchmark', icon: 'Target', section: 'analysis' },
// { id: 'consortium', label: 'Portfolio-Gruppen', icon: 'FolderKanban', section: 'analysis' },
// { id: 'reports', label: 'Berichte', icon: 'BarChart3', section: 'analysis' },
// { id: 'screener', label: 'Screener', icon: 'Search', section: 'tools' },
// { id: 'plans', label: 'Sparpläne', icon: 'CalendarClock', section: 'tools' },
// { id: 'rebalancing', label: 'Rebalancing', icon: 'Scale', section: 'tools' },
```

---

## Code-Herkunft

Diese Anwendung ist eine **Clean-Room Reimplementierung** von Portfolio Performance:

| Komponente | Status |
|------------|--------|
| Protobuf Parser | Reverse-engineered (nicht kopiert) |
| Frontend (React/TS) | 100% original |
| FIFO-Logik | Algorithmus-Adaptation |
| Performance-Formeln | Standard-Finanzformeln (TTWROR, IRR) |
| Technische Indikatoren | Eigenimplementierung |

**Kein Java-Code wurde kopiert.** Die Quellenangaben im Code (z.B. "Based on TradeCollector.java") sind Inspirations-Verweise, keine Code-Kopien.

---

## Bekannte Einschränkungen

1. **macOS Only (v0.1.0)** - Windows/Linux-Builds sind möglich, aber nicht getestet
2. **Poppler für PDF-OCR** - Erforderlich für OpenAI/Perplexity OCR (optional, Claude/Gemini funktionieren ohne)
3. **Deutsche Sprache primär** - Englische Übersetzung teilweise vorhanden

---

## Nächste Schritte (v0.2.0)

- [ ] Sparpläne aktivieren
- [ ] Rebalancing aktivieren
- [ ] Berichte aktivieren
- [ ] Benchmark aktivieren
- [ ] Portfolio-Gruppen aktivieren
- [ ] Windows/Linux-Builds testen
- [ ] Englische Lokalisierung vervollständigen
