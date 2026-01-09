# Portfolio Performance Protobuf Implementation Progress

**Stand:** 2026-01-04
**Ziel:** 100% Kompatibilität mit PP Binary Format

---

## Abgeschlossen

### Phase 1: Schema-Korrekturen (DONE)

#### 1.1 Enum-Korrekturen
- [x] **Transaction Type Enum** korrigiert
  - `PURCHASE=0, SALE=1, INBOUND_DELIVERY=2, OUTBOUND_DELIVERY=3, SECURITY_TRANSFER=4, CASH_TRANSFER=5, DEPOSIT=6, REMOVAL=7, DIVIDEND=8, INTEREST=9, INTEREST_CHARGE=10, TAX=11, TAX_REFUND=12, FEE=13, FEE_REFUND=14`
- [x] **TransactionUnit Type Enum** korrigiert
  - `GROSS_VALUE=0, TAX=1, FEE=2` (war falsch: FEE=0!)

#### 1.2 Neue Helper-Messages
- [x] `PAnyValue` - Flexibler Wert-Container (oneof: Null, String, Int32, Int64, Double, Bool, Map)
- [x] `PKeyValue` - Key-Value-Paar für Attributes/Properties
- [x] `PMap` - Map von Key-Value-Paaren
- [x] `PDecimalValue` - Präzise Dezimalwerte (scale, precision, bytes)
- [x] `PTimestamp` - Timestamp (seconds, nanos)

#### 1.3 Schema-Updates
- [x] `PSecurity`: events, attributes, properties, isRetired, updatedAt hinzugefügt
- [x] `PAccount`: note, isRetired, attributes, updatedAt hinzugefügt
- [x] `PPortfolio`: note, isRetired, attributes, updatedAt hinzugefügt
- [x] `PTransaction`: Offizielle Feldnamen (portfolio, other_portfolio, other_account, security, note, etc.)
- [x] `PTransactionUnit`: fxAmount, fxCurrencyCode, fxRateToBase hinzugefügt
- [x] `PClient`: plans (tag 6), watchlists (tag 7) hinzugefügt
- [x] `PWatchlist`: securities als Vec<String> (UUIDs)
- [x] `PInvestmentPlan`: Vollständige Struktur
- [x] `PSettings`: configurationSets hinzugefügt

#### 1.4 Parser-Anpassungen
- [x] `convert_client`: Neue Feldnamen (portfolio, other_portfolio)
- [x] `convert_transaction`: Neue Feldnamen, korrekte Enum-Konstanten
- [x] `convert_security`: Optional currency_code
- [x] `convert_portfolio`: Optional reference_account

#### 1.5 Tests
- [x] Alle 21 Tests bestanden
- [x] Echte Portfolio.portfolio Datei: 92 Securities, 1 Account, 10 Portfolios

---

## Ausstehend

### Phase 2: Neue Features implementieren

#### 2.1 pp:: Model Erweiterungen
- [ ] `Watchlist` struct in `src/pp/watchlist.rs`
- [ ] `InvestmentPlan` struct in `src/pp/investment_plan.rs`
- [ ] `SecurityEvent` struct in `src/pp/security.rs`
- [ ] `Client` um watchlists, investment_plans erweitern

#### 2.2 Parser-Erweiterungen
- [ ] `convert_watchlist` Funktion
- [ ] `convert_investment_plan` Funktion
- [ ] `convert_security_event` Funktion
- [ ] Watchlists in `convert_client` laden
- [ ] Investment Plans in `convert_client` laden
- [ ] Security Events in `convert_security` laden

### Phase 3: Classification Tree
- [ ] `build_classification_tree` Funktion (parentId → Baum)
- [ ] `Classification` mit children Vec
- [ ] Rekursive Tree-Struktur

### Phase 4: Erweiterte Felder nutzen
- [ ] `isRetired` Felder in UI nutzen
- [ ] `note` Felder anzeigen
- [ ] `attributes` / `properties` (PKeyValue) parsen
- [ ] `updatedAt` Timestamps konvertieren

### Phase 5: Settings
- [ ] `Bookmark` struct
- [ ] `AttributeType` struct
- [ ] `ConfigurationSet` struct
- [ ] Settings in Client integrieren

---

## Referenzen

- **Offizielles Schema:** https://github.com/portfolio-performance/portfolio/blob/master/name.abuchen.portfolio/src/name/abuchen/portfolio/model/client.proto
- **Detaillierter Plan:** `/Users/ricoullmann/.claude/plans/generic-sleeping-dewdrop.md`

---

## Dateien

| Datei | Status |
|-------|--------|
| `src/protobuf/schema.rs` | Phase 1 komplett |
| `src/protobuf/parser.rs` | Phase 1 komplett |
| `src/pp/watchlist.rs` | Noch nicht erstellt |
| `src/pp/investment_plan.rs` | Noch nicht erstellt |

---

## Nächster Schritt

**Phase 2 starten:** Neue pp:: Structs für Watchlist, InvestmentPlan, SecurityEvent erstellen und Parser erweitern.
