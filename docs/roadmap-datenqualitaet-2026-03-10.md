# Roadmap: Datenqualität & Synergie (Stand 10. März 2026)

## Ist-Zustand
- 92 Wertpapiere, 283.372 Preiszeilen in Portfolio.db, neuester Kurs vom 30.12.2025
- 90/92 Wertpapiere >30 Tage alt, 20 >90 Tage, 2 ohne Preise
- Alle Preiszeilen `source='import'`
- Alt-DB nur Schlusskurse; neue App (`pp_price`) hat echtes OHLC-Schema
- Zufalls-OHLC in `indicators.ts:294` wird auch im Trading-Briefing verwendet (`TradingBriefingWidget.tsx:59`)
- AI inkonsistent: Chart-Panel baut strukturierten Kontext (`AIAnalysisPanel.tsx:337`), SecurityPriceModal schickt nur Bild+`indicators:[]` (`SecurityPriceModal.tsx:202`)
- Portfolio-Insights ohne technische Signale (`ai.rs:277`, `prompts.rs:338`)
- 13 Wertpapiere auf PP/GENERIC_HTML_TABLE Feed-Typen, 6 doppelte ISINs

## Abarbeitungsreihenfolge

### 1) Random-OHLC aus Scoring-Pfaden entfernen
- `convertToOHLC()` in `indicators.ts` erzeugt Zufallswerte wenn kein echtes OHLC
- Diese dürfen NIE in Regime-, Setup-, Screener- oder Risk-Logik laufen
- Stattdessen "nicht belastbar" / "Keine OHLC-Daten" anzeigen
- Für reine Visualisierung (Charts) tolerierbar, für Scoring nicht

### 2) Import + Quote-Audit + Backfill koppeln
- `data_quality_score` pro Wertpapier einführen: Frische, OHLCV-Anteil, Volumenabdeckung, Split-Abdeckung, Provider-Vertrauen
- TA und AI müssen diesen Score kennen
- Automatischer Import-Audit mit Mapping auf Yahoo/TwelveData/Kraken
- Veraltete-Quote-Info aus `ai/context.rs:801` auch in TA/UI nutzen

### 3) AI-Pfade vereinheitlichen
- SecurityPriceModal soll denselben strukturierten Kontext wie AIAnalysisPanel nutzen (nicht nur Screenshot)
- Portfolio-Insights sollen technische Zustände je Holding mitbekommen: Regime, Setup-Score, SMA200-Distanz, RSI/MACD, ATR-Risiko, Datenfrische

### 4) Provider-Routing einführen
- **Yahoo**: Default-Backbone (breit, kostenlos, inoffiziell)
- **Twelve Data**: Gezielt für EU/CH-Reparatur (800 Req/Tag free, echte OHLCV, international nur Trial-Symbole)
- **Kraken**: Primärquelle Krypto-OHLCV (besser als CoinGecko, aber nur letzte 720 Kerzen via REST)
- **CoinGecko**: Krypto-Fallback (Public kostenlos, aber nur Close+Volume, kein echtes OHLC)
- **Alpha Vantage**: Nur manuelle Reparatur/kleine Queues (25 Req/Tag free)
- **EODHD**: Lohnt sich kostenlos kaum (20 Calls/Tag)
- **TradingView**: Nur Search/Quote, Historie fällt auf 1 Tageskerze zurück (`tradingview.rs:162`)

## Kostenlose Kursquellen-Bewertung
| Provider | Free-Limit | Stärke | Schwäche |
|---|---|---|---|
| Yahoo | unbegrenzt (inoffiziell) | Breiteste Abdeckung | Inoffiziell, kein OHLCV-Guarantee |
| Twelve Data | 800 Req/Tag | Echte OHLCV | International nur Trial-Symbole free |
| Alpha Vantage | 25 Req/Tag | Offiziell | Zu wenig für Portfolio-Sync |
| EODHD | 20 Calls/Tag | - | Zu wenig |
| Kraken | Unbegrenzt | Offizielle OHLC | Nur Krypto, nur 720 Kerzen |
| CoinGecko | Unbegrenzt | Breit Krypto | Nur Close+Volume |

## Synergien
- Wichtigster Schritt: `data_quality_score` pro Wertpapier, den TA und AI kennen
- Zufalls-OHLC nie in Scoring/Signale — lieber "nicht belastbar" anzeigen
- Starke AI-Analyse aus Chart-Panel überall wiederverwenden (SecurityPriceModal, Portfolio-Insights)
- Portfolio-Insights mit technischen Zuständen je Holding anreichern → AI wird "Priorisierer"
- Datenbasis fast da: AI-Kontext kennt veraltete Quotes (`ai/context.rs:801`) — auch in TA/UI nutzen
- Import-Audit: 13 alte Feed-Typen + 6 doppelte ISINs automatisch auf Yahoo/TwelveData/Kraken mappen
