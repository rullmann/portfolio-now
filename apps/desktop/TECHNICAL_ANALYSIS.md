# Technische Analyse - Dokumentation

Dieses Dokument beschreibt die technischen Analyse-Features in Portfolio Now.

## Inhaltsverzeichnis

1. [Indikatoren](#indikatoren)
2. [Candlestick-Pattern-Erkennung](#candlestick-pattern-erkennung)
3. [Signal-System](#signal-system)
4. [Zeichenwerkzeuge](#zeichenwerkzeuge)
5. [KI-Integration](#ki-integration)
6. [Pattern-Tracking](#pattern-tracking)
7. [API-Referenz](#api-referenz)

---

## Indikatoren

### Übersicht

Alle Indikatoren sind in `src/lib/indicators.ts` implementiert und können im Chart aktiviert werden.

### Verfügbare Indikatoren

#### Trend-Indikatoren

| Indikator | Funktion | Parameter | Beschreibung |
|-----------|----------|-----------|--------------|
| **SMA** | `calculateSMA()` | period (default: 20) | Simple Moving Average |
| **EMA** | `calculateEMA()` | period (default: 20) | Exponential Moving Average |
| **VWAP** | `calculateVWAP()` | - | Volume Weighted Average Price |

#### Momentum-Indikatoren

| Indikator | Funktion | Parameter | Beschreibung |
|-----------|----------|-----------|--------------|
| **RSI** | `calculateRSI()` | period (default: 14) | Relative Strength Index (0-100) |
| **Stochastic** | `calculateStochastic()` | kPeriod (14), kSlow (3), dPeriod (3) | %K und %D Linien |
| **MACD** | `calculateMACD()` | fast (12), slow (26), signal (9) | MACD, Signal, Histogram |

#### Volatilitäts-Indikatoren

| Indikator | Funktion | Parameter | Beschreibung |
|-----------|----------|-----------|--------------|
| **Bollinger Bands** | `calculateBollingerBands()` | period (20), stdDev (2) | Upper, Middle, Lower Band |
| **ATR** | `calculateATR()` | period (14) | Average True Range |

#### Volumen-Indikatoren

| Indikator | Funktion | Parameter | Beschreibung |
|-----------|----------|-----------|--------------|
| **OBV** | `calculateOBV()` | - | On-Balance Volume |
| **ADX** | `calculateADX()` | period (14) | Average Directional Index + DI+/DI- |

### Verwendung im Code

```typescript
import {
  calculateSMA,
  calculateRSI,
  calculateMACD,
  calculateBollingerBands,
  type OHLCData,
  type LineData
} from '@/lib/indicators';

// Beispiel: RSI berechnen
const rsiData: LineData[] = calculateRSI(ohlcData, 14);

// Beispiel: MACD berechnen
const macdResult = calculateMACD(ohlcData, 12, 26, 9);
// macdResult.macd - MACD Linie
// macdResult.signal - Signal Linie
// macdResult.histogram - Histogram

// Beispiel: Bollinger Bands
const bbResult = calculateBollingerBands(ohlcData, 20, 2);
// bbResult.upper - Oberes Band
// bbResult.middle - SMA
// bbResult.lower - Unteres Band
```

### Signalinterpretation

#### RSI
- **< 30**: Überverkauft (potenzielles Kaufsignal)
- **> 70**: Überkauft (potenzielles Verkaufssignal)
- **50**: Neutraler Bereich

#### MACD
- **MACD kreuzt Signal von unten**: Bullish Crossover
- **MACD kreuzt Signal von oben**: Bearish Crossover
- **Histogram wechselt Vorzeichen**: Momentum-Wechsel

#### Bollinger Bands
- **Kurs berührt oberes Band**: Mögliche Überkauft-Situation
- **Kurs berührt unteres Band**: Mögliche Überverkauft-Situation
- **Bands kontrahieren (Squeeze)**: Volatilität nimmt ab, Ausbruch erwartet

#### ADX
- **< 20**: Schwacher/kein Trend
- **20-40**: Moderater Trend
- **> 40**: Starker Trend
- **+DI > -DI**: Aufwärtstrend
- **-DI > +DI**: Abwärtstrend

---

## Candlestick-Pattern-Erkennung

### Übersicht

Die Pattern-Erkennung ist in `src/lib/patterns.ts` implementiert und erkennt automatisch Candlestick-Muster.

### Erkannte Muster

#### Single Candle Patterns

| Pattern | Richtung | Zuverlässigkeit | Beschreibung |
|---------|----------|-----------------|--------------|
| **Doji** | Neutral | Medium | Open ≈ Close, zeigt Unentschlossenheit |
| **Hammer** | Bullish | High | Langer unterer Schatten, kleiner Körper oben (nur im Abwärtstrend) |
| **Inverted Hammer** | Bullish | Medium | Langer oberer Schatten, kleiner Körper unten (nur im Abwärtstrend) |
| **Hanging Man** | Bearish | Medium | Wie Hammer, aber im Aufwärtstrend |
| **Shooting Star** | Bearish | High | Langer oberer Schatten (nur im Aufwärtstrend) |
| **Spinning Top** | Neutral | Low | Kleiner Körper, lange Schatten beidseitig |
| **Marubozu** | Bullish/Bearish | High | Großer Körper, keine/minimale Schatten |

#### Two Candle Patterns

| Pattern | Richtung | Zuverlässigkeit | Beschreibung |
|---------|----------|-----------------|--------------|
| **Bullish Engulfing** | Bullish | High | Grüne Kerze umschließt vorherige rote |
| **Bearish Engulfing** | Bearish | High | Rote Kerze umschließt vorherige grüne |
| **Bullish Harami** | Bullish | Medium | Kleine grüne Kerze innerhalb großer roter |
| **Bearish Harami** | Bearish | Medium | Kleine rote Kerze innerhalb großer grüner |
| **Piercing Line** | Bullish | Medium | Grüne Kerze schließt über 50% der vorherigen roten |
| **Dark Cloud Cover** | Bearish | Medium | Rote Kerze schließt unter 50% der vorherigen grünen |
| **Tweezer Top** | Bearish | Medium | Zwei Kerzen mit gleichem Hoch |
| **Tweezer Bottom** | Bullish | Medium | Zwei Kerzen mit gleichem Tief |

#### Three Candle Patterns

| Pattern | Richtung | Zuverlässigkeit | Beschreibung |
|---------|----------|-----------------|--------------|
| **Morning Star** | Bullish | High | Große rote, kleine, große grüne Kerze |
| **Evening Star** | Bearish | High | Große grüne, kleine, große rote Kerze |
| **Three White Soldiers** | Bullish | High | Drei aufeinanderfolgende grüne Kerzen |
| **Three Black Crows** | Bearish | High | Drei aufeinanderfolgende rote Kerzen |
| **Three Inside Up** | Bullish | Medium | Harami + Bestätigungskerze nach oben |
| **Three Inside Down** | Bearish | Medium | Harami + Bestätigungskerze nach unten |

### Verwendung im Code

```typescript
import {
  detectCandlestickPatterns,
  getLatestPatterns,
  type PatternMatch,
  type CandlestickPattern
} from '@/lib/patterns';

// Alle Patterns erkennen
const patterns: PatternMatch[] = detectCandlestickPatterns(ohlcData);

// Nur die letzten N Patterns
const latestPatterns = getLatestPatterns(ohlcData, 5);

// PatternMatch Struktur
interface PatternMatch {
  pattern: CandlestickPattern;  // z.B. 'hammer', 'engulfing_bullish'
  name: string;                  // z.B. 'Hammer', 'Bullish Engulfing'
  startIndex: number;            // Index der ersten Kerze
  endIndex: number;              // Index der letzten Kerze
  direction: 'bullish' | 'bearish' | 'neutral';
  reliability: 'high' | 'medium' | 'low';
  description: string;           // Deutsche Beschreibung
}
```

### Trend-Erkennung

Die Pattern-Erkennung berücksichtigt den aktuellen Trend:

```typescript
// Intern verwendet:
function isInUptrend(data: OHLCData[], index: number, period = 5): boolean {
  // Prüft ob Preis mind. 2% gestiegen ist
  return data[index].close > data[index - period].close * 1.02;
}

function isInDowntrend(data: OHLCData[], index: number, period = 5): boolean {
  // Prüft ob Preis mind. 2% gefallen ist
  return data[index].close < data[index - period].close * 0.98;
}
```

---

## Signal-System

### Übersicht

Das Signal-System in `src/lib/signals.ts` erkennt automatisch Trading-Signale basierend auf Indikatoren.

### Signal-Typen

| Signal | Bedingung | Richtung |
|--------|-----------|----------|
| `rsi_oversold` | RSI < 30 | Bullish |
| `rsi_overbought` | RSI > 70 | Bearish |
| `macd_crossover_bullish` | MACD kreuzt Signal von unten | Bullish |
| `macd_crossover_bearish` | MACD kreuzt Signal von oben | Bearish |
| `bollinger_squeeze` | Bandbreite < 20-Tage-Min | Neutral |
| `stochastic_oversold` | %K < 20 und kreuzt %D | Bullish |
| `stochastic_overbought` | %K > 80 und kreuzt %D | Bearish |
| `adx_trend_start` | ADX steigt über 25 | Neutral |

### Divergenz-Erkennung

```typescript
import { detectDivergence, type DivergenceSignal } from '@/lib/signals';

// Erkennt bullische/bärische Divergenzen
const divergences: DivergenceSignal[] = detectDivergence(
  priceData,
  rsiData,
  'rsi',
  20  // Lookback Periode
);

// Bullish Divergence: Preis macht tieferes Tief, Indikator macht höheres Tief
// Bearish Divergence: Preis macht höheres Hoch, Indikator macht tieferes Hoch
```

---

## Zeichenwerkzeuge

### Übersicht

Die Zeichenwerkzeuge sind in `src/components/charts/DrawingTools.tsx` implementiert.

### Verfügbare Werkzeuge

| Werkzeug | Beschreibung | Punkte |
|----------|--------------|--------|
| **Trendlinie** | Linie von Punkt A zu B | 2 |
| **Horizontale Linie** | Level auf bestimmtem Preis | 1 |
| **Fibonacci** | Retracement-Level | 2 |

### Fibonacci-Level

| Level | Farbe | Bedeutung |
|-------|-------|-----------|
| 0% | Rot | Startpunkt |
| 23.6% | Orange | Erstes Retracement |
| 38.2% | Gelb | Goldener Schnitt |
| 50% | Grün | Halbierung |
| 61.8% | Cyan | Goldener Schnitt (wichtigster) |
| 78.6% | Violett | Tiefes Retracement |
| 100% | Rot | Vollständiges Retracement |

### Persistenz

Zeichnungen werden in der SQLite-Datenbank gespeichert:

```sql
CREATE TABLE pp_chart_drawing (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    security_id INTEGER NOT NULL,
    drawing_type TEXT NOT NULL,  -- 'trendline', 'horizontal', 'fibonacci'
    points_json TEXT NOT NULL,   -- JSON Array von Punkten
    color TEXT NOT NULL DEFAULT '#2563eb',
    line_width INTEGER NOT NULL DEFAULT 2,
    fib_levels_json TEXT,        -- Nur für Fibonacci
    is_visible INTEGER NOT NULL DEFAULT 1,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### Tauri Commands

```rust
// Zeichnung speichern
save_chart_drawing(drawing: ChartDrawing) -> ChartDrawingResponse

// Alle Zeichnungen für Security laden
get_chart_drawings(security_id: i64) -> Vec<ChartDrawingResponse>

// Einzelne Zeichnung löschen
delete_chart_drawing(drawing_id: i64)

// Alle Zeichnungen für Security löschen
clear_chart_drawings(security_id: i64)
```

---

## KI-Integration

### Erweiterte Chart-Analyse

Die KI-Analyse (`analyze_chart_enhanced`) erhält umfangreichen Kontext:

```typescript
interface EnhancedChartContext {
  securityName: string;
  currentPrice: number;
  priceChange24h: number;
  timeframe: string;

  // Indikator-Werte
  indicators: {
    rsi?: { value: number; signal: string };
    macd?: { macd: number; signal: number; histogram: number; crossover?: string };
    bollingerBands?: { upper: number; middle: number; lower: number; position: string };
    sma?: Record<string, number>;
    ema?: Record<string, number>;
    atr?: number;
    volume?: { current: number; average20d: number; ratio: number };
  };

  // OHLC-Daten
  ohlcData: OHLCData[];

  // Erkannte Patterns
  patterns?: PatternMatch[];

  // Web-Kontext (nur Perplexity)
  includeWebContext?: boolean;
}
```

### Web-Suche (Perplexity)

Bei aktivierter Web-Suche recherchiert die KI:

1. **Aktuelle Nachrichten** zu der Security
2. **Earnings-Termine** (bevorstehend/kürzlich)
3. **Analysteneinschätzungen** (Ratings, Kursziele)
4. **Sektor-Entwicklung**

```typescript
// Im Frontend
const supportsWebSearch = getModelCapabilities(provider, model).webSearch;

// Aktivierung
<AIAnalysisPanel
  includeWebContext={true}  // Nur wenn supportsWebSearch
/>
```

### Model Capabilities

```typescript
interface ModelCapabilities {
  vision: boolean;      // Kann Bilder analysieren
  webSearch: boolean;   // Kann Web durchsuchen (Perplexity)
  pdfUpload: boolean;   // Kann PDFs direkt verarbeiten (Claude, Gemini)
}

// Web-fähige Modelle
const webSearchModels = ['sonar', 'sonar-pro'];
```

---

## Pattern-Tracking

### Übersicht

Das Pattern-Tracking speichert erkannte Muster und evaluiert deren Erfolg.

### Datenbank-Schema

```sql
CREATE TABLE pp_pattern_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    security_id INTEGER NOT NULL,
    pattern_type TEXT NOT NULL,
    detected_at TEXT NOT NULL,
    price_at_detection REAL NOT NULL,
    predicted_direction TEXT NOT NULL,  -- 'bullish', 'bearish'
    actual_outcome TEXT,                 -- 'success', 'failure', 'pending'
    price_after_5d REAL,
    price_after_10d REAL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
```

### Tauri Commands

```rust
// Pattern speichern
save_pattern_detection(pattern: PatternDetection) -> i64

// Outcomes evaluieren (5/10 Tage)
evaluate_pattern_outcomes() -> PatternEvaluationResult

// Statistiken abrufen
get_pattern_statistics() -> Vec<PatternStatistics>

// Historie für Security
get_pattern_history(security_id: i64) -> Vec<PatternHistory>
```

### Erfolgsberechnung

```typescript
// Nach 5 Tagen
if (predicted_direction === 'bullish') {
  outcome = price_after_5d > price_at_detection ? 'success' : 'failure';
} else {
  outcome = price_after_5d < price_at_detection ? 'success' : 'failure';
}

// Statistik
success_rate = successful_patterns / total_patterns * 100;
```

---

## API-Referenz

### TypeScript Types

```typescript
// Indikator-Daten
interface LineData {
  time: string;
  value: number;
}

// OHLC-Kerze
interface OHLCData {
  time: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

// Pattern-Match
interface PatternMatch {
  pattern: CandlestickPattern;
  name: string;
  startIndex: number;
  endIndex: number;
  direction: PatternDirection;
  reliability: PatternReliability;
  description: string;
}

// Zeichnung
interface Drawing {
  id: string;
  type: DrawingTool;
  points: Point[];
  color: string;
  lineWidth: number;
  fibLevels?: number[];
}

// Signal
interface TechnicalSignal {
  type: string;
  direction: 'bullish' | 'bearish' | 'neutral';
  date: string;
  price: number;
  description: string;
  strength: 'strong' | 'moderate' | 'weak';
}
```

### Rust Types

```rust
// Pattern Detection
pub struct PatternDetection {
    pub security_id: i64,
    pub pattern_type: String,
    pub detected_at: String,
    pub price_at_detection: f64,
    pub predicted_direction: String,
}

// Pattern Statistics
pub struct PatternStatistics {
    pub pattern_type: String,
    pub total_detections: i64,
    pub successful: i64,
    pub failed: i64,
    pub pending: i64,
    pub success_rate: f64,
}

// Chart Drawing
pub struct ChartDrawing {
    pub id: Option<String>,
    pub security_id: i64,
    pub drawing_type: String,
    pub points: Vec<Point>,
    pub color: String,
    pub line_width: i32,
    pub fib_levels: Option<Vec<f64>>,
}
```

---

## Tests

### Unit Tests

```bash
# Alle Tests ausführen
cd apps/desktop && pnpm test

# Nur Pattern-Tests
pnpm test -- patterns.test.ts

# Nur Indikator-Tests
pnpm test -- indicators.test.ts
```

### Test-Abdeckung

- `indicators.test.ts` - Alle Indikator-Berechnungen
- `patterns.test.ts` - Candlestick-Pattern-Erkennung (150+ Tests)
- `signals.test.ts` - Signal-Erkennung

### Beispiel-Test

```typescript
describe('Hammer Pattern', () => {
  it('should detect hammer in downtrend with long lower shadow', () => {
    const data = generateTrendingData(120, 15, 'down');
    data.push(createCandle('2024-01-16', 92, 93, 80, 93));

    const patterns = detectCandlestickPatterns(data);
    const hammer = patterns.find((p) => p.pattern === 'hammer');

    expect(hammer).toBeDefined();
    expect(hammer?.direction).toBe('bullish');
    expect(hammer?.reliability).toBe('high');
  });
});
```
