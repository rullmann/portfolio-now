# Refactoring-Guide: Einstandspreis als Single Source of Truth

## Ziel
Einstandswerte (Cost Basis) muessen im gesamten Code exakt gleich berechnet werden.
Die einzige Quelle der Wahrheit ist die FIFO-Logik in `src-tauri/src/fifo/mod.rs`.
Alle anderen Berechnungen sind entweder zu entfernen oder auf die SSOT-Funktionen umzubauen.

## Begriffsklaerung
- Einstandswert: Summe der FIFO-Lots, inkl. Fees/Taxes, pro Lot in Basiswaehrung umgerechnet.
- Einstandskurs (Einstandspreis pro Anteil): `Einstandswert / verbleibende Anteile`.
- SSOT: `get_total_cost_basis_converted`, `get_cost_basis_by_security_converted`,
  `get_cost_basis_by_security_id_converted` in `src-tauri/src/fifo/mod.rs`.

## Warum SSOT wichtig ist
- FIFO-Lots koennen unterschiedliche Waehrungen haben; Summen muessen pro Lot konvertiert werden.
- `GROUP BY` in SQL ueber FIFO-Lots ist falsch, sobald Waehrungen gemischt sind.
- Der Einstandswert muss identisch zu dem Wert sein, der im "Einstand"-UI angezeigt wird.

## Aktuelle doppelte Implementationen (Refactor-Kandidaten)
1) `src-tauri/src/commands/rebalancing.rs`
   - Inline SQL auf `pp_fifo_lot` mit `GROUP BY` und ohne Waehrungskonvertierung.
   - Ergebnis weicht vom Einstandswert ab, sobald Lots unterschiedliche Waehrungen haben.

2) `packages/core/src/calculations/holdings.ts`
   - Eigene Cost-Basis-Logik aus Transaktionen (kein FIFO, Fees/Taxes nur teilweise).
   - Sollte nicht fuer produktive Einstandswerte genutzt werden.

3) Weitere Direktabfragen auf `pp_fifo_lot`
   - Erlaubt nur fuer technische Operationen (z.B. Rebuild/Integrity),
     aber nicht fuer Einstandswerte.

## Zielbild (Architektur)
Alle Einstandswerte kommen aus `fifo::get_*_cost_basis_*`.
Kein UI- oder Command-Code berechnet Einstandswerte selbst.

## Konkreter Refactoring-Plan
1) SSOT erweitern
   - Neue Funktion: `get_cost_basis_by_security_id_converted(conn, base_currency, portfolio_id: Option<i64>)`
     oder ein Wrapper, der optional nach Portfolio filtert.
   - Begruendung: Rebalancing braucht Portfolio-spezifische Einstandswerte.

2) Rebalancing umbauen
   - Ersetze die SQL-Map in `src-tauri/src/commands/rebalancing.rs` durch SSOT-Aufruf.
   - Waehle `base_currency` aus Request und konvertiere pro Lot.

3) Core-Holdings deprecaten
   - `packages/core/src/calculations/holdings.ts` als "approx" markieren oder entfernen,
     falls nicht mehr genutzt.
   - Falls weiterhin benoetigt: Backend-API nutzen und Einstandswert liefern lassen.

4) Guardrails
   - Kommentar in `fifo/mod.rs` ist SSOT. Zusaetzlich: kurzer Hinweis in `commands/`-Modulen,
     dass direkte Cost-Basis-SQL nicht erlaubt ist.
   - Optional: kleine Test-Hilfe, die `pp_fifo_lot`-SQL in Commands als Anti-Pattern flaggt.

## Definition der einzigen Quelle (Einstandspreis)
Die einzige korrekte Quelle fuer Einstandswerte ist:
- `src-tauri/src/fifo/mod.rs`:
  - `get_total_cost_basis_converted`
  - `get_cost_basis_by_security_converted`
  - `get_cost_basis_by_security_id_converted`

Alles andere ist nur Anzeige, Aggregation oder UI-Berechnung auf Basis dieser Werte.

## Validierung nach Refactor
- Vergleich: Einstandswert im Dashboard vs. Einstandswert in Reports/AI/Rebalancing.
- Testfall mit gemischten Waehrungen in FIFO-Lots (z.B. CHF + EUR fuer gleiche ISIN).
- Sicherstellen, dass `GROUP BY` auf FIFO-Lots nirgendwo zur Cost-Basis genutzt wird.
