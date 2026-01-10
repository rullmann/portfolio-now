# PDF Import Feature - Status & Dokumentation

**Datum:** 2026-01-10
**Status:** Funktioniert im Dev-Modus, Release-Modus noch nicht getestet

---

## Übersicht

PDF-Import-Funktion zum Einlesen von Bank-Abrechnungen (Kauf/Verkauf, Dividenden, etc.) aus PDF-Dateien.

### Unterstützte Banken

| Bank | Parser | Beschreibung |
|------|--------|--------------|
| DKB | `dkb.rs` | Deutsche Kreditbank AG |
| ING | `ing.rs` | ING-DiBa AG |
| Comdirect | `comdirect.rs` | Comdirect Bank AG |
| Trade Republic | `trade_republic.rs` | Trade Republic Bank GmbH |
| Scalable Capital | `scalable.rs` | Scalable Capital GmbH (via Baader Bank) |

---

## Implementierte Funktionen

### Backend (Rust)

| Datei | Funktion | Beschreibung |
|-------|----------|--------------|
| `pdf_import/mod.rs` | `extract_pdf_text()` | PDF-Text-Extraktion mit Thread-Isolation |
| `pdf_import/mod.rs` | `parse_pdf()` | Auto-Detection der Bank + Parsing |
| `pdf_import/mod.rs` | `validate_pdf()` | PDF-Header und Größen-Validierung |
| `commands/pdf_import.rs` | `preview_pdf_import()` | Vorschau ohne DB-Änderungen |
| `commands/pdf_import.rs` | `import_pdf_transactions()` | Import in Datenbank |
| `commands/pdf_import.rs` | `get_supported_banks()` | Liste unterstützter Banken |

### Frontend (React/TypeScript)

| Datei | Komponente | Beschreibung |
|-------|------------|--------------|
| `components/modals/PdfImportModal.tsx` | `PdfImportModal` | Upload-Dialog mit Vorschau |
| `views/Transactions/index.tsx` | Button in Toolbar | "PDF Import" Button hinzugefügt |

---

## Bekanntes Problem: Crashes in Release-Modus

### Symptom
App zeigt schwarzen Bildschirm beim PDF-Import im Release-Build.

### Ursache
Die `pdf-extract` Crate (v0.7) verursacht Panics bei bestimmten PDFs, die auch mit `catch_unwind` und Thread-Isolation im Release-Modus nicht abgefangen werden können.

### Bisherige Lösungsversuche

| Versuch | Ergebnis |
|---------|----------|
| `panic = "unwind"` in Cargo.toml | Notwendig, aber nicht ausreichend |
| `catch_unwind` wrapper | Funktioniert im Dev-Modus |
| Thread-basierte Isolation | Funktioniert im Dev-Modus |
| Kombination aller Maßnahmen | Dev: OK, Release: Crash |

### Aktuelle Lösung

Thread-basierte Extraktion mit `catch_unwind`:

```rust
pub fn extract_pdf_text(pdf_path: &str) -> Result<String, String> {
    let bytes = std::fs::read(pdf_path)?;
    validate_pdf(&bytes)?;

    let handle = thread::spawn(move || {
        catch_unwind(AssertUnwindSafe(|| {
            pdf_extract::extract_text_from_mem(&bytes)
        }))
    });

    match handle.join() {
        Ok(Ok(Ok(text))) => Ok(text),
        Ok(Ok(Err(e))) => Err(format!("Extraktion fehlgeschlagen: {}", e)),
        Ok(Err(_)) => Err("PDF-Parsing fehlgeschlagen (Panic caught)"),
        Err(_) => Err("Thread panicked"),
    }
}
```

### Mögliche zukünftige Lösungen

1. **Alternative PDF-Bibliothek**
   - `lopdf` - Niedriger Level, stabiler
   - `pdf-rs` - Neuere Implementierung
   - `pdfium-render` - Google's PDFium Bindings

2. **Separater Prozess**
   - PDF-Extraktion als Child-Process auslagern
   - Crash isoliert vom Hauptprozess

3. **WASM-basierte Lösung**
   - PDF.js via WebAssembly
   - Läuft isoliert im Frontend

---

## Cargo.toml Änderungen

```toml
[profile.release]
panic = "unwind"  # Geändert von "abort" für catch_unwind Support
```

**Wichtig:** Diese Änderung ist notwendig für `catch_unwind`, erhöht aber die Binary-Größe leicht.

---

## Test-Ergebnisse

### Dev-Modus (10:00:27)

```
PDF Extract: Reading file /Users/.../Abrechnungsausführung4.pdf
PDF Extract: File read, 15577 bytes
PDF Extract: Validation passed, starting text extraction in thread
PDF Extract: Thread completed
PDF Extract: Success, extracted 1110 chars
PDF Import: Successfully parsed PDF, found 1 transactions
```

**Ergebnis:** 1 Transaktion erfolgreich aus Scalable Capital PDF extrahiert.

### Release-Modus

**Status:** Noch nicht mit aktueller Implementierung getestet.

---

## Dateien

### Geänderte Dateien

| Datei | Änderung |
|-------|----------|
| `src-tauri/Cargo.toml` | `panic = "unwind"` |
| `src-tauri/src/pdf_import/mod.rs` | Thread-basierte Extraktion |
| `src-tauri/src/commands/pdf_import.rs` | Logging hinzugefügt |
| `src/views/Transactions/index.tsx` | PDF Import Button |

### Neue Abhängigkeiten

Keine neuen Abhängigkeiten. `pdf-extract` war bereits im Projekt.

---

## UI Integration

Der PDF-Import Button befindet sich in der Transaktions-Ansicht:

```
Transaktionen Toolbar:
[+ Transaktion] [PDF Import] [Löschen] [Aktualisieren]
```

### Workflow

1. User klickt "PDF Import"
2. Modal öffnet sich mit Datei-Upload
3. Nach Upload: Vorschau der erkannten Transaktionen
4. User wählt Portfolio und Konto
5. Optional: Fehlende Wertpapiere automatisch anlegen
6. Import durchführen

---

## Nächste Schritte

1. [ ] Release-Build testen
2. [ ] Bei Crash: Alternative PDF-Bibliothek evaluieren
3. [ ] Parser für weitere Banken erweitern
4. [ ] Bessere Fehlerbehandlung für nicht erkannte PDFs
