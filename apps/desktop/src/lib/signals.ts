/**
 * Technical Signal Detection System
 * Automatic detection of trading signals from technical indicators
 */

import type { OHLCData, LineData } from './indicators';
import { calculateRSI, calculateMACD, calculateBollinger, calculateStochastic, calculateADX, calculateOBV } from './indicators';

// ============================================================================
// Types
// ============================================================================

export type SignalType =
  | 'rsi_oversold'
  | 'rsi_overbought'
  | 'macd_bullish_cross'
  | 'macd_bearish_cross'
  | 'bollinger_squeeze'
  | 'bollinger_breakout_up'
  | 'bollinger_breakout_down'
  | 'stochastic_oversold'
  | 'stochastic_overbought'
  | 'stochastic_bullish_cross'
  | 'stochastic_bearish_cross'
  | 'adx_trend_start'
  | 'adx_trend_strong'
  | 'golden_cross'
  | 'death_cross'
  | 'divergence_bullish'
  | 'divergence_bearish';

export type SignalDirection = 'bullish' | 'bearish' | 'neutral';
export type SignalStrength = 'strong' | 'moderate' | 'weak';

export interface TechnicalSignal {
  type: SignalType;
  direction: SignalDirection;
  strength: SignalStrength;
  date: string;
  price: number;
  indicator: string;
  value?: number;
  description: string;
}

export interface DivergenceSignal {
  type: 'bullish' | 'bearish';
  indicator: 'rsi' | 'macd' | 'obv' | 'stochastic';
  startDate: string;
  endDate: string;
  priceStart: number;
  priceEnd: number;
  indicatorStart: number;
  indicatorEnd: number;
  confidence: number;
}

export interface SignalDetectionConfig {
  rsiOversold?: number;      // Default: 30
  rsiOverbought?: number;    // Default: 70
  adxTrendThreshold?: number; // Default: 25
  adxStrongThreshold?: number; // Default: 40
  bollingerSqueezePeriod?: number; // Default: 20
  divergenceLookback?: number; // Default: 20
}

// ============================================================================
// Main Signal Detection
// ============================================================================

export function detectSignals(
  data: OHLCData[],
  config: SignalDetectionConfig = {}
): TechnicalSignal[] {
  const signals: TechnicalSignal[] = [];

  if (data.length < 30) return signals;

  const {
    rsiOversold = 30,
    rsiOverbought = 70,
    adxTrendThreshold = 25,
    adxStrongThreshold = 40,
    bollingerSqueezePeriod = 20,
  } = config;

  // Calculate indicators
  const rsi = calculateRSI(data, 14);
  const macd = calculateMACD(data, 12, 26, 9);
  const bollinger = calculateBollinger(data, 20, 2);
  const stochastic = calculateStochastic(data, 14, 3, 3);
  const adx = calculateADX(data, 14);

  // Detect signals for the most recent data points (last 5 bars)
  const lookback = Math.min(5, data.length - 30);

  for (let i = data.length - lookback; i < data.length; i++) {
    const bar = data[i];
    const prevBar = data[i - 1];

    // RSI Signals
    const rsiValue = rsi[i]?.value;
    const prevRsiValue = rsi[i - 1]?.value;

    if (rsiValue !== null && prevRsiValue !== null) {
      // RSI crosses below oversold
      if (rsiValue <= rsiOversold && prevRsiValue > rsiOversold) {
        signals.push({
          type: 'rsi_oversold',
          direction: 'bullish',
          strength: rsiValue < 20 ? 'strong' : 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'RSI',
          value: rsiValue,
          description: `RSI ist überverkauft (${rsiValue.toFixed(1)})`,
        });
      }

      // RSI crosses above overbought
      if (rsiValue >= rsiOverbought && prevRsiValue < rsiOverbought) {
        signals.push({
          type: 'rsi_overbought',
          direction: 'bearish',
          strength: rsiValue > 80 ? 'strong' : 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'RSI',
          value: rsiValue,
          description: `RSI ist überkauft (${rsiValue.toFixed(1)})`,
        });
      }
    }

    // MACD Signals
    const macdValue = macd.macd[i]?.value;
    const signalValue = macd.signal[i]?.value;
    const prevMacdValue = macd.macd[i - 1]?.value;
    const prevSignalValue = macd.signal[i - 1]?.value;

    if (macdValue !== null && signalValue !== null && prevMacdValue !== null && prevSignalValue !== null) {
      // Bullish crossover
      if (macdValue > signalValue && prevMacdValue <= prevSignalValue) {
        signals.push({
          type: 'macd_bullish_cross',
          direction: 'bullish',
          strength: macdValue < 0 ? 'strong' : 'moderate', // Stronger if crossing in negative territory
          date: bar.time,
          price: bar.close,
          indicator: 'MACD',
          description: 'MACD kreuzt Signal-Linie von unten (Kaufsignal)',
        });
      }

      // Bearish crossover
      if (macdValue < signalValue && prevMacdValue >= prevSignalValue) {
        signals.push({
          type: 'macd_bearish_cross',
          direction: 'bearish',
          strength: macdValue > 0 ? 'strong' : 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'MACD',
          description: 'MACD kreuzt Signal-Linie von oben (Verkaufssignal)',
        });
      }
    }

    // Bollinger Band Signals
    const upperBand = bollinger.upper[i]?.value;
    const lowerBand = bollinger.lower[i]?.value;
    const middleBand = bollinger.middle[i]?.value;

    if (upperBand !== null && lowerBand !== null && middleBand !== null) {
      // Calculate bandwidth for squeeze detection
      const bandwidth = (upperBand - lowerBand) / middleBand;

      // Check for squeeze (lowest bandwidth in last N periods)
      let isLowestBandwidth = true;
      for (let j = Math.max(0, i - bollingerSqueezePeriod); j < i; j++) {
        const prevUpper = bollinger.upper[j]?.value;
        const prevLower = bollinger.lower[j]?.value;
        const prevMiddle = bollinger.middle[j]?.value;
        if (prevUpper !== null && prevLower !== null && prevMiddle !== null) {
          const prevBandwidth = (prevUpper - prevLower) / prevMiddle;
          if (prevBandwidth < bandwidth) {
            isLowestBandwidth = false;
            break;
          }
        }
      }

      if (isLowestBandwidth && bandwidth < 0.1) {
        signals.push({
          type: 'bollinger_squeeze',
          direction: 'neutral',
          strength: bandwidth < 0.05 ? 'strong' : 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'Bollinger',
          description: 'Bollinger Squeeze - Volatilitätsausbruch erwartet',
        });
      }

      // Breakout signals
      if (bar.close > upperBand && prevBar.close <= bollinger.upper[i - 1]?.value!) {
        signals.push({
          type: 'bollinger_breakout_up',
          direction: 'bullish',
          strength: 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'Bollinger',
          description: 'Kurs bricht über oberes Bollinger Band aus',
        });
      }

      if (bar.close < lowerBand && prevBar.close >= bollinger.lower[i - 1]?.value!) {
        signals.push({
          type: 'bollinger_breakout_down',
          direction: 'bearish',
          strength: 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'Bollinger',
          description: 'Kurs bricht unter unteres Bollinger Band aus',
        });
      }
    }

    // Stochastic Signals
    const kValue = stochastic.k[i]?.value;
    const dValue = stochastic.d[i]?.value;
    const prevKValue = stochastic.k[i - 1]?.value;
    const prevDValue = stochastic.d[i - 1]?.value;

    if (kValue !== null && dValue !== null) {
      // Oversold/Overbought
      if (kValue <= 20 && prevKValue !== null && prevKValue > 20) {
        signals.push({
          type: 'stochastic_oversold',
          direction: 'bullish',
          strength: kValue < 10 ? 'strong' : 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'Stochastic',
          value: kValue,
          description: `Stochastic überverkauft (%K: ${kValue.toFixed(1)})`,
        });
      }

      if (kValue >= 80 && prevKValue !== null && prevKValue < 80) {
        signals.push({
          type: 'stochastic_overbought',
          direction: 'bearish',
          strength: kValue > 90 ? 'strong' : 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'Stochastic',
          value: kValue,
          description: `Stochastic überkauft (%K: ${kValue.toFixed(1)})`,
        });
      }

      // Crossovers
      if (prevKValue !== null && prevDValue !== null) {
        if (kValue > dValue && prevKValue <= prevDValue && kValue < 50) {
          signals.push({
            type: 'stochastic_bullish_cross',
            direction: 'bullish',
            strength: kValue < 20 ? 'strong' : 'moderate',
            date: bar.time,
            price: bar.close,
            indicator: 'Stochastic',
            description: '%K kreuzt %D von unten (Kaufsignal)',
          });
        }

        if (kValue < dValue && prevKValue >= prevDValue && kValue > 50) {
          signals.push({
            type: 'stochastic_bearish_cross',
            direction: 'bearish',
            strength: kValue > 80 ? 'strong' : 'moderate',
            date: bar.time,
            price: bar.close,
            indicator: 'Stochastic',
            description: '%K kreuzt %D von oben (Verkaufssignal)',
          });
        }
      }
    }

    // ADX Signals
    const adxValue = adx.adx[i]?.value;
    const prevAdxValue = adx.adx[i - 1]?.value;
    const diPlus = adx.diPlus[i]?.value;
    const diMinus = adx.diMinus[i]?.value;

    if (adxValue !== null && prevAdxValue !== null) {
      // Trend starting
      if (adxValue >= adxTrendThreshold && prevAdxValue < adxTrendThreshold) {
        const direction: SignalDirection = diPlus !== null && diMinus !== null
          ? (diPlus > diMinus ? 'bullish' : 'bearish')
          : 'neutral';
        signals.push({
          type: 'adx_trend_start',
          direction,
          strength: 'moderate',
          date: bar.time,
          price: bar.close,
          indicator: 'ADX',
          value: adxValue,
          description: `Trend beginnt (ADX: ${adxValue.toFixed(1)}, ${direction === 'bullish' ? 'Aufwärts' : direction === 'bearish' ? 'Abwärts' : 'Unbestimmt'})`,
        });
      }

      // Strong trend
      if (adxValue >= adxStrongThreshold && prevAdxValue < adxStrongThreshold) {
        const direction: SignalDirection = diPlus !== null && diMinus !== null
          ? (diPlus > diMinus ? 'bullish' : 'bearish')
          : 'neutral';
        signals.push({
          type: 'adx_trend_strong',
          direction,
          strength: 'strong',
          date: bar.time,
          price: bar.close,
          indicator: 'ADX',
          value: adxValue,
          description: `Starker Trend (ADX: ${adxValue.toFixed(1)})`,
        });
      }
    }
  }

  // Sort by date descending (newest first)
  signals.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());

  return signals;
}

// ============================================================================
// Divergence Detection
// ============================================================================

interface PivotPoint {
  index: number;
  price: number;
  indicatorValue: number;
  type: 'high' | 'low';
}

function findPivotPoints(
  prices: number[],
  indicatorValues: (number | null)[],
  lookback: number = 5
): PivotPoint[] {
  const pivots: PivotPoint[] = [];

  for (let i = lookback; i < prices.length - lookback; i++) {
    const indicatorValue = indicatorValues[i];
    if (indicatorValue === null) continue;

    // Check for pivot high
    let isPivotHigh = true;
    let isPivotLow = true;

    for (let j = 1; j <= lookback; j++) {
      const leftValue = indicatorValues[i - j];
      const rightValue = indicatorValues[i + j];

      if (leftValue === null || rightValue === null) {
        isPivotHigh = false;
        isPivotLow = false;
        break;
      }

      if (indicatorValue <= leftValue || indicatorValue <= rightValue) {
        isPivotHigh = false;
      }
      if (indicatorValue >= leftValue || indicatorValue >= rightValue) {
        isPivotLow = false;
      }
    }

    if (isPivotHigh) {
      pivots.push({
        index: i,
        price: prices[i],
        indicatorValue,
        type: 'high',
      });
    }

    if (isPivotLow) {
      pivots.push({
        index: i,
        price: prices[i],
        indicatorValue,
        type: 'low',
      });
    }
  }

  return pivots;
}

export function detectDivergence(
  data: OHLCData[],
  indicatorData: LineData[],
  indicatorName: 'rsi' | 'macd' | 'obv' | 'stochastic',
  lookback: number = 20
): DivergenceSignal[] {
  const divergences: DivergenceSignal[] = [];

  if (data.length < lookback * 2) return divergences;

  const prices = data.map(d => d.close);
  const indicatorValues = indicatorData.map(d => d.value);

  // Find pivot points in the last portion of data
  const startIndex = Math.max(0, data.length - lookback * 3);
  const pivots = findPivotPoints(
    prices.slice(startIndex),
    indicatorValues.slice(startIndex),
    3
  ).map(p => ({
    ...p,
    index: p.index + startIndex,
  }));

  // Find divergences by comparing consecutive pivot points
  const pivotHighs = pivots.filter(p => p.type === 'high');
  const pivotLows = pivots.filter(p => p.type === 'low');

  // Bearish divergence: Higher high in price, lower high in indicator
  for (let i = 1; i < pivotHighs.length; i++) {
    const prev = pivotHighs[i - 1];
    const curr = pivotHighs[i];

    if (curr.index - prev.index > lookback) continue;

    // Price makes higher high, indicator makes lower high
    if (curr.price > prev.price && curr.indicatorValue < prev.indicatorValue) {
      const priceDiff = (curr.price - prev.price) / prev.price;
      const indicatorDiff = (prev.indicatorValue - curr.indicatorValue) / prev.indicatorValue;
      const confidence = Math.min(1, (priceDiff + indicatorDiff) * 5);

      if (confidence > 0.3) {
        divergences.push({
          type: 'bearish',
          indicator: indicatorName,
          startDate: data[prev.index].time,
          endDate: data[curr.index].time,
          priceStart: prev.price,
          priceEnd: curr.price,
          indicatorStart: prev.indicatorValue,
          indicatorEnd: curr.indicatorValue,
          confidence,
        });
      }
    }
  }

  // Bullish divergence: Lower low in price, higher low in indicator
  for (let i = 1; i < pivotLows.length; i++) {
    const prev = pivotLows[i - 1];
    const curr = pivotLows[i];

    if (curr.index - prev.index > lookback) continue;

    // Price makes lower low, indicator makes higher low
    if (curr.price < prev.price && curr.indicatorValue > prev.indicatorValue) {
      const priceDiff = (prev.price - curr.price) / prev.price;
      const indicatorDiff = (curr.indicatorValue - prev.indicatorValue) / Math.abs(prev.indicatorValue || 1);
      const confidence = Math.min(1, (priceDiff + indicatorDiff) * 5);

      if (confidence > 0.3) {
        divergences.push({
          type: 'bullish',
          indicator: indicatorName,
          startDate: data[prev.index].time,
          endDate: data[curr.index].time,
          priceStart: prev.price,
          priceEnd: curr.price,
          indicatorStart: prev.indicatorValue,
          indicatorEnd: curr.indicatorValue,
          confidence,
        });
      }
    }
  }

  return divergences;
}

// ============================================================================
// Helper: Detect all divergences across indicators
// ============================================================================

export function detectAllDivergences(
  data: OHLCData[],
  lookback: number = 20
): DivergenceSignal[] {
  const allDivergences: DivergenceSignal[] = [];

  if (data.length < 30) return allDivergences;

  // RSI divergences
  const rsi = calculateRSI(data, 14);
  allDivergences.push(...detectDivergence(data, rsi, 'rsi', lookback));

  // MACD divergences (using MACD line)
  const macd = calculateMACD(data, 12, 26, 9);
  allDivergences.push(...detectDivergence(data, macd.macd, 'macd', lookback));

  // OBV divergences
  const obv = calculateOBV(data);
  allDivergences.push(...detectDivergence(data, obv, 'obv', lookback));

  // Stochastic divergences
  const stochastic = calculateStochastic(data, 14, 3, 3);
  allDivergences.push(...detectDivergence(data, stochastic.k, 'stochastic', lookback));

  // Sort by end date descending
  allDivergences.sort((a, b) => new Date(b.endDate).getTime() - new Date(a.endDate).getTime());

  return allDivergences;
}

// ============================================================================
// Convert signals to TechnicalSignal format for UI
// ============================================================================

export function divergenceToSignal(div: DivergenceSignal, price: number): TechnicalSignal {
  const indicatorNames: Record<string, string> = {
    rsi: 'RSI',
    macd: 'MACD',
    obv: 'OBV',
    stochastic: 'Stochastic',
  };

  return {
    type: div.type === 'bullish' ? 'divergence_bullish' : 'divergence_bearish',
    direction: div.type,
    strength: div.confidence > 0.7 ? 'strong' : div.confidence > 0.5 ? 'moderate' : 'weak',
    date: div.endDate,
    price,
    indicator: indicatorNames[div.indicator] || div.indicator,
    description: div.type === 'bullish'
      ? `Bullische Divergenz bei ${indicatorNames[div.indicator]} - Preis fällt, Indikator steigt`
      : `Bärische Divergenz bei ${indicatorNames[div.indicator]} - Preis steigt, Indikator fällt`,
  };
}

// ============================================================================
// Get all signals including divergences
// ============================================================================

export function getAllSignals(
  data: OHLCData[],
  config: SignalDetectionConfig = {}
): TechnicalSignal[] {
  const signals = detectSignals(data, config);

  // Add divergences
  const divergences = detectAllDivergences(data, config.divergenceLookback || 20);
  for (const div of divergences) {
    const lastBar = data.find(d => d.time === div.endDate);
    if (lastBar) {
      signals.push(divergenceToSignal(div, lastBar.close));
    }
  }

  // Sort by date descending
  signals.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime());

  return signals;
}
