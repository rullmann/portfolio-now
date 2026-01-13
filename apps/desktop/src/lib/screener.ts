/**
 * Screener - Filter-based security screening
 * Scan securities for technical conditions
 */

import type { OHLCData, LineData } from './indicators';
import {
  calculateRSI,
  calculateMACD,
  calculateBollinger,
  calculateStochastic,
  calculateADX,
  calculateOBV,
} from './indicators';

// ============================================================================
// Types
// ============================================================================

export type ScreenerIndicator =
  | 'price'
  | 'volume'
  | 'rsi'
  | 'macd'
  | 'macd_signal'
  | 'macd_histogram'
  | 'bollinger_upper'
  | 'bollinger_lower'
  | 'bollinger_width'
  | 'stochastic_k'
  | 'stochastic_d'
  | 'adx'
  | 'di_plus'
  | 'di_minus'
  | 'obv'
  | 'sma_20'
  | 'sma_50'
  | 'sma_200'
  | 'change_1d'
  | 'change_5d'
  | 'change_20d';

export type ScreenerCondition =
  | 'above'
  | 'below'
  | 'crosses_above'
  | 'crosses_below'
  | 'between'
  | 'increasing'
  | 'decreasing';

export interface ScreenerFilter {
  id: string;
  indicator: ScreenerIndicator;
  condition: ScreenerCondition;
  value: number;
  value2?: number; // For 'between' condition
  enabled: boolean;
}

export interface ScreenerPreset {
  id: string;
  name: string;
  description: string;
  filters: Omit<ScreenerFilter, 'id' | 'enabled'>[];
}

export interface SecurityData {
  securityId: number;
  name: string;
  ticker?: string;
  isin?: string;
  currency?: string;
  ohlcData: OHLCData[];
}

export interface ScreenerResult {
  securityId: number;
  securityName: string;
  ticker?: string;
  isin?: string;
  currency?: string;
  matchedFilters: string[];
  currentValues: Record<string, number | undefined>;
  lastPrice: number;
  change1d?: number;
  change5d?: number;
  change20d?: number;
}

// ============================================================================
// Preset Definitions
// ============================================================================

export const screenerPresets: ScreenerPreset[] = [
  {
    id: 'oversold',
    name: 'Überverkauft',
    description: 'RSI unter 30, potenzielle Kaufgelegenheit',
    filters: [
      { indicator: 'rsi', condition: 'below', value: 30 },
    ],
  },
  {
    id: 'overbought',
    name: 'Überkauft',
    description: 'RSI über 70, potenzielle Verkaufssituation',
    filters: [
      { indicator: 'rsi', condition: 'above', value: 70 },
    ],
  },
  {
    id: 'strong_uptrend',
    name: 'Starker Aufwärtstrend',
    description: 'ADX > 25 mit +DI > -DI',
    filters: [
      { indicator: 'adx', condition: 'above', value: 25 },
      { indicator: 'di_plus', condition: 'above', value: 0 }, // Will compare di_plus > di_minus
    ],
  },
  {
    id: 'strong_downtrend',
    name: 'Starker Abwärtstrend',
    description: 'ADX > 25 mit -DI > +DI',
    filters: [
      { indicator: 'adx', condition: 'above', value: 25 },
      { indicator: 'di_minus', condition: 'above', value: 0 }, // Will compare di_minus > di_plus
    ],
  },
  {
    id: 'bollinger_squeeze',
    name: 'Bollinger Squeeze',
    description: 'Niedrige Volatilität, Ausbruch erwartet',
    filters: [
      { indicator: 'bollinger_width', condition: 'below', value: 5 },
    ],
  },
  {
    id: 'golden_cross_setup',
    name: 'Golden Cross Setup',
    description: 'SMA 50 nahe SMA 200 (innerhalb 2%)',
    filters: [
      { indicator: 'sma_50', condition: 'above', value: 0 }, // Will use special comparison
    ],
  },
  {
    id: 'volume_spike',
    name: 'Volumen-Spike',
    description: 'Volumen > 200% des Durchschnitts',
    filters: [
      { indicator: 'volume', condition: 'above', value: 200 },
    ],
  },
  {
    id: 'momentum_bullish',
    name: 'Bullish Momentum',
    description: 'Positiver MACD mit steigendem Histogramm',
    filters: [
      { indicator: 'macd_histogram', condition: 'above', value: 0 },
      { indicator: 'macd_histogram', condition: 'increasing', value: 0 },
    ],
  },
  {
    id: 'stochastic_oversold',
    name: 'Stochastic Überverkauft',
    description: 'K und D unter 20',
    filters: [
      { indicator: 'stochastic_k', condition: 'below', value: 20 },
      { indicator: 'stochastic_d', condition: 'below', value: 20 },
    ],
  },
  {
    id: 'breakout_candidate',
    name: 'Ausbruchs-Kandidat',
    description: 'Preis nahe Bollinger Upper Band mit hohem Volumen',
    filters: [
      { indicator: 'bollinger_upper', condition: 'above', value: 95 }, // Price at 95% of upper band
      { indicator: 'volume', condition: 'above', value: 150 },
    ],
  },
];

// ============================================================================
// Indicator Labels
// ============================================================================

export const indicatorLabels: Record<ScreenerIndicator, string> = {
  price: 'Preis',
  volume: 'Volumen (%)',
  rsi: 'RSI (14)',
  macd: 'MACD',
  macd_signal: 'MACD Signal',
  macd_histogram: 'MACD Histogramm',
  bollinger_upper: 'Bollinger Upper (%)',
  bollinger_lower: 'Bollinger Lower (%)',
  bollinger_width: 'Bollinger Breite (%)',
  stochastic_k: 'Stochastic %K',
  stochastic_d: 'Stochastic %D',
  adx: 'ADX (14)',
  di_plus: '+DI',
  di_minus: '-DI',
  obv: 'OBV',
  sma_20: 'SMA 20',
  sma_50: 'SMA 50',
  sma_200: 'SMA 200',
  change_1d: 'Änderung 1T (%)',
  change_5d: 'Änderung 5T (%)',
  change_20d: 'Änderung 20T (%)',
};

export const conditionLabels: Record<ScreenerCondition, string> = {
  above: 'über',
  below: 'unter',
  crosses_above: 'kreuzt über',
  crosses_below: 'kreuzt unter',
  between: 'zwischen',
  increasing: 'steigend',
  decreasing: 'fallend',
};

// ============================================================================
// Screener Logic
// ============================================================================

interface CalculatedIndicators {
  price: number;
  volume: number;
  volumeAvg20: number;
  rsi?: number;
  macd?: number;
  macdSignal?: number;
  macdHistogram?: number;
  macdHistogramPrev?: number;
  bollingerUpper?: number;
  bollingerLower?: number;
  bollingerMiddle?: number;
  bollingerWidth?: number;
  stochasticK?: number;
  stochasticD?: number;
  stochasticKPrev?: number;
  stochasticDPrev?: number;
  adx?: number;
  diPlus?: number;
  diMinus?: number;
  obv?: number;
  sma20?: number;
  sma50?: number;
  sma200?: number;
  change1d?: number;
  change5d?: number;
  change20d?: number;
}

function getLastValue(data: LineData[]): number | undefined {
  if (data.length === 0) return undefined;
  const value = data[data.length - 1].value;
  return value === null ? undefined : value;
}

function getPrevValue(data: LineData[], offset: number = 1): number | undefined {
  if (data.length <= offset) return undefined;
  const value = data[data.length - 1 - offset].value;
  return value === null ? undefined : value;
}

function calculateSMA(data: OHLCData[], period: number): number | undefined {
  if (data.length < period) return undefined;
  const slice = data.slice(-period);
  const sum = slice.reduce((acc, d) => acc + d.close, 0);
  return sum / period;
}

function calculateIndicators(ohlcData: OHLCData[]): CalculatedIndicators | null {
  if (ohlcData.length < 2) return null;

  const lastCandle = ohlcData[ohlcData.length - 1];
  const prevCandle = ohlcData[ohlcData.length - 2];

  // Volume average
  const volumeSlice = ohlcData.slice(-20);
  const volumeAvg = volumeSlice.reduce((acc, d) => acc + (d.volume || 0), 0) / volumeSlice.length;

  // Price changes
  const price5dAgo = ohlcData.length >= 5 ? ohlcData[ohlcData.length - 5].close : undefined;
  const price20dAgo = ohlcData.length >= 20 ? ohlcData[ohlcData.length - 20].close : undefined;

  // Calculate indicators
  const rsiData = calculateRSI(ohlcData, 14);
  const macdData = calculateMACD(ohlcData);
  const bollingerData = calculateBollinger(ohlcData);
  const stochasticData = calculateStochastic(ohlcData);
  const adxData = calculateADX(ohlcData);
  const obvData = calculateOBV(ohlcData);

  return {
    price: lastCandle.close,
    volume: lastCandle.volume || 0,
    volumeAvg20: volumeAvg,
    rsi: getLastValue(rsiData),
    macd: macdData ? getLastValue(macdData.macd) : undefined,
    macdSignal: macdData ? getLastValue(macdData.signal) : undefined,
    macdHistogram: macdData ? getLastValue(macdData.histogram) : undefined,
    macdHistogramPrev: macdData ? getPrevValue(macdData.histogram) : undefined,
    bollingerUpper: bollingerData ? getLastValue(bollingerData.upper) : undefined,
    bollingerLower: bollingerData ? getLastValue(bollingerData.lower) : undefined,
    bollingerMiddle: bollingerData ? getLastValue(bollingerData.middle) : undefined,
    bollingerWidth: bollingerData && bollingerData.middle.length > 0
      ? ((getLastValue(bollingerData.upper)! - getLastValue(bollingerData.lower)!) / getLastValue(bollingerData.middle)!) * 100
      : undefined,
    stochasticK: stochasticData ? getLastValue(stochasticData.k) : undefined,
    stochasticD: stochasticData ? getLastValue(stochasticData.d) : undefined,
    stochasticKPrev: stochasticData ? getPrevValue(stochasticData.k) : undefined,
    stochasticDPrev: stochasticData ? getPrevValue(stochasticData.d) : undefined,
    adx: adxData ? getLastValue(adxData.adx) : undefined,
    diPlus: adxData ? getLastValue(adxData.diPlus) : undefined,
    diMinus: adxData ? getLastValue(adxData.diMinus) : undefined,
    obv: getLastValue(obvData),
    sma20: calculateSMA(ohlcData, 20),
    sma50: calculateSMA(ohlcData, 50),
    sma200: calculateSMA(ohlcData, 200),
    change1d: prevCandle ? ((lastCandle.close - prevCandle.close) / prevCandle.close) * 100 : undefined,
    change5d: price5dAgo ? ((lastCandle.close - price5dAgo) / price5dAgo) * 100 : undefined,
    change20d: price20dAgo ? ((lastCandle.close - price20dAgo) / price20dAgo) * 100 : undefined,
  };
}

function getIndicatorValue(
  indicator: ScreenerIndicator,
  values: CalculatedIndicators
): number | undefined {
  switch (indicator) {
    case 'price':
      return values.price;
    case 'volume':
      return values.volumeAvg20 > 0 ? (values.volume / values.volumeAvg20) * 100 : undefined;
    case 'rsi':
      return values.rsi;
    case 'macd':
      return values.macd;
    case 'macd_signal':
      return values.macdSignal;
    case 'macd_histogram':
      return values.macdHistogram;
    case 'bollinger_upper':
      return values.bollingerUpper && values.price
        ? (values.price / values.bollingerUpper) * 100
        : undefined;
    case 'bollinger_lower':
      return values.bollingerLower && values.price
        ? (values.price / values.bollingerLower) * 100
        : undefined;
    case 'bollinger_width':
      return values.bollingerWidth;
    case 'stochastic_k':
      return values.stochasticK;
    case 'stochastic_d':
      return values.stochasticD;
    case 'adx':
      return values.adx;
    case 'di_plus':
      return values.diPlus;
    case 'di_minus':
      return values.diMinus;
    case 'obv':
      return values.obv;
    case 'sma_20':
      return values.sma20;
    case 'sma_50':
      return values.sma50;
    case 'sma_200':
      return values.sma200;
    case 'change_1d':
      return values.change1d;
    case 'change_5d':
      return values.change5d;
    case 'change_20d':
      return values.change20d;
    default:
      return undefined;
  }
}

function checkCondition(
  filter: ScreenerFilter,
  values: CalculatedIndicators
): boolean {
  const value = getIndicatorValue(filter.indicator, values);
  if (value === undefined) return false;

  switch (filter.condition) {
    case 'above':
      // Special handling for DI comparisons
      if (filter.indicator === 'di_plus' && values.diMinus !== undefined) {
        return values.diPlus !== undefined && values.diPlus > values.diMinus;
      }
      if (filter.indicator === 'di_minus' && values.diPlus !== undefined) {
        return values.diMinus !== undefined && values.diMinus > values.diPlus;
      }
      return value > filter.value;

    case 'below':
      return value < filter.value;

    case 'crosses_above':
      // Need previous value for cross detection
      if (filter.indicator === 'stochastic_k' && values.stochasticKPrev !== undefined) {
        return values.stochasticKPrev <= filter.value && value > filter.value;
      }
      if (filter.indicator === 'stochastic_d' && values.stochasticDPrev !== undefined) {
        return values.stochasticDPrev <= filter.value && value > filter.value;
      }
      return false;

    case 'crosses_below':
      if (filter.indicator === 'stochastic_k' && values.stochasticKPrev !== undefined) {
        return values.stochasticKPrev >= filter.value && value < filter.value;
      }
      if (filter.indicator === 'stochastic_d' && values.stochasticDPrev !== undefined) {
        return values.stochasticDPrev >= filter.value && value < filter.value;
      }
      return false;

    case 'between':
      return filter.value2 !== undefined && value >= filter.value && value <= filter.value2;

    case 'increasing':
      if (filter.indicator === 'macd_histogram' && values.macdHistogramPrev !== undefined) {
        return values.macdHistogram !== undefined && values.macdHistogram > values.macdHistogramPrev;
      }
      return false;

    case 'decreasing':
      if (filter.indicator === 'macd_histogram' && values.macdHistogramPrev !== undefined) {
        return values.macdHistogram !== undefined && values.macdHistogram < values.macdHistogramPrev;
      }
      return false;

    default:
      return false;
  }
}

export function runScreener(
  securities: SecurityData[],
  filters: ScreenerFilter[]
): ScreenerResult[] {
  const activeFilters = filters.filter(f => f.enabled);
  if (activeFilters.length === 0) return [];

  const results: ScreenerResult[] = [];

  for (const security of securities) {
    if (security.ohlcData.length < 20) continue; // Need minimum data

    const indicators = calculateIndicators(security.ohlcData);
    if (!indicators) continue;

    const matchedFilters: string[] = [];
    let allMatch = true;

    for (const filter of activeFilters) {
      if (checkCondition(filter, indicators)) {
        matchedFilters.push(
          `${indicatorLabels[filter.indicator]} ${conditionLabels[filter.condition]} ${filter.value}${filter.value2 !== undefined ? ` und ${filter.value2}` : ''}`
        );
      } else {
        allMatch = false;
        break;
      }
    }

    if (allMatch && matchedFilters.length > 0) {
      results.push({
        securityId: security.securityId,
        securityName: security.name,
        ticker: security.ticker,
        isin: security.isin,
        currency: security.currency,
        matchedFilters,
        currentValues: {
          price: indicators.price,
          rsi: indicators.rsi,
          macd: indicators.macd,
          adx: indicators.adx,
          volume: indicators.volumeAvg20 > 0 ? (indicators.volume / indicators.volumeAvg20) * 100 : undefined,
          change1d: indicators.change1d,
          change5d: indicators.change5d,
          change20d: indicators.change20d,
        },
        lastPrice: indicators.price,
        change1d: indicators.change1d,
        change5d: indicators.change5d,
        change20d: indicators.change20d,
      });
    }
  }

  // Sort by absolute 1-day change (most volatile first)
  results.sort((a, b) => {
    const aChange = Math.abs(a.change1d || 0);
    const bChange = Math.abs(b.change1d || 0);
    return bChange - aChange;
  });

  return results;
}

// ============================================================================
// Filter Helpers
// ============================================================================

export function createFilter(
  indicator: ScreenerIndicator,
  condition: ScreenerCondition,
  value: number,
  value2?: number
): ScreenerFilter {
  return {
    id: `${indicator}-${condition}-${value}-${Date.now()}`,
    indicator,
    condition,
    value,
    value2,
    enabled: true,
  };
}

export function applyPreset(preset: ScreenerPreset): ScreenerFilter[] {
  return preset.filters.map((f, i) => ({
    ...f,
    id: `${preset.id}-${i}-${Date.now()}`,
    enabled: true,
  }));
}
