# Code Review Summary: portfolio-modern

## Code Review: 14. Januar 2026 (aktueller Stand)

### Findings (nach Schweregrad)

1. **Theme-Fehler in Charts (Medium):** `theme === 'system'` wird in `Charts` immer auf `dark` gesetzt, wodurch System-Theme-Light ignoriert wird. Das kann Kontrast- und Lesbarkeitsprobleme verursachen.  
   Datei: `apps/desktop/src/views/Charts/index.tsx:490`

2. **UI-Status inkonsistent beim Logo-Upload (Medium):** `setUploadingLogoId(null)` wird im `finally` ausgeführt, bevor `FileReader`/Upload abgeschlossen sind. Das UI signalisiert "fertig", obwohl der Upload noch läuft; Fehler nach `reader.onload` sind schwer nachvollziehbar.  
   Datei: `apps/desktop/src/views/Securities/index.tsx:339-377`

3. **Welcome-Modal Race bei Persist-Hydration (Low):** Die Entscheidung, ob das Welcome-Modal gezeigt wird, passiert nur einmal. Wenn `userName` später rehydriert wird, kann das Modal fälschlich erscheinen.  
   Datei: `apps/desktop/src/App.tsx:83-97`

4. **Stale Data bei Watchlist-Price-Histories (Low):** Preisverläufe werden mit Snapshot von `securities` geladen; bei schnellem Watchlist-Wechsel können veraltete Histories gesetzt werden.  
   Datei: `apps/desktop/src/views/Watchlist/index.tsx:63-85`

5. **Retry-Timer ohne Unmount-Cleanup (Low):** Der AI-Panel Retry-Timer wird nicht beim Unmount gecleart; kann zu State-Updates nach Unmount führen.  
   Datei: `apps/desktop/src/components/charts/AIAnalysisPanel.tsx:153-177`
