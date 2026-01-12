# Plan: Einheitliche Datum-Behandlung

## Problem-Analyse

Das Datum wird in der Datenbank im Format `YYYY-MM-DD HH:MM:SS` gespeichert (z.B. "2024-01-15 09:30:00").

**Hauptproblem:**
- Das HTML `<input type="date">` erwartet `YYYY-MM-DD` - mit Uhrzeit bleibt das Feld **leer**
- TransactionFormModal setzt `date: transaction.date` direkt → Input zeigt nichts an

## Bestandsaufnahme

### Zentrale Funktionen existieren bereits in `src/lib/types.ts`:
- `formatDate(dateStr)` - Zeile 276 - Für Anzeige (dd.mm.yyyy) ✓
- `formatDateTime(dateStr)` - Zeile 285 - Mit Uhrzeit ✓

### Lokale Duplikate (sollten zentrale Funktion verwenden):
- `Reports/index.tsx:127` - lokales `formatDate`
- `InvestmentPlans/index.tsx:124` - lokales `formatDate`
- `Dividends/index.tsx:227` - lokales `formatDate`
- `PdfImportModal.tsx:290` - lokales `formatDate`

### Fehlendes:
- `formatDateForInput(dateStr)` - Für HTML date input → "YYYY-MM-DD"

## Lösung

### 1. Neue Funktion in `src/lib/types.ts` hinzufügen:

```typescript
/**
 * Extrahiert das Datum (YYYY-MM-DD) aus einem Datetime-String.
 * Für HTML <input type="date">.
 */
export function formatDateForInput(dateStr: string | null | undefined): string {
  if (!dateStr) return '';
  // "2024-01-15 09:30:00" → "2024-01-15"
  // "2024-01-15T09:30:00" → "2024-01-15"
  return dateStr.split(' ')[0].split('T')[0];
}
```

### 2. TransactionFormModal.tsx anpassen:

```typescript
import { formatDateForInput } from '../../lib/types';

// Zeile 135: Edit-Mode
date: formatDateForInput(transaction.date),
```

### 3. Transactions/index.tsx - Zentrale formatDate verwenden:

```typescript
import { formatDate } from '../../lib/types';

// Zeile 265
<td className="py-2 whitespace-nowrap">{formatDate(tx.date)}</td>
```

### 4. Lokale Duplikate entfernen und zentrale Funktion verwenden

## Implementierungs-Schritte

1. [ ] `src/lib/types.ts` - `formatDateForInput()` hinzufügen
2. [ ] `TransactionFormModal.tsx` - Import + Verwendung
3. [ ] `Transactions/index.tsx` - Zentrale `formatDate` verwenden
4. [ ] Lokale Duplikate durch zentrale Imports ersetzen (4 Dateien)
5. [ ] TypeScript Check
6. [ ] Testen
