/**
 * Screener - Types, Constants & Filter Helpers
 *
 * The runScreener() engine has been migrated to Rust (src-tauri/src/indicators/screener.rs).
 * Use indicators-rust.ts for the async Tauri invoke wrapper.
 * This file retains types, preset definitions, labels, and filter helper functions.
 */

import type { OHLCData } from './indicators';
import type { BreakoutScore } from './types';

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
  breakoutScore?: BreakoutScore;
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
      { indicator: 'di_plus', condition: 'above', value: 0 },
    ],
  },
  {
    id: 'strong_downtrend',
    name: 'Starker Abwärtstrend',
    description: 'ADX > 25 mit -DI > +DI',
    filters: [
      { indicator: 'adx', condition: 'above', value: 25 },
      { indicator: 'di_minus', condition: 'above', value: 0 },
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
      { indicator: 'sma_50', condition: 'above', value: 0 },
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
      { indicator: 'bollinger_upper', condition: 'above', value: 95 },
      { indicator: 'volume', condition: 'above', value: 150 },
    ],
  },
];

// ============================================================================
// Labels
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
