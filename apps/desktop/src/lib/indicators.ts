/**
 * Technical Analysis Indicators
 * Pure, testable functions for calculating trading indicators
 * All functions return arrays with same length as input, using null for insufficient data
 */

export interface OHLCData {
  time: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

export interface LineData {
  time: string;
  value: number | null;
}

export interface HistogramData {
  time: string;
  value: number | null;
  color?: string;
}

export interface MACDResult {
  macd: LineData[];
  signal: LineData[];
  histogram: HistogramData[];
}

export interface BollingerResult {
  upper: LineData[];
  middle: LineData[];
  lower: LineData[];
}

// ============================================================================
// Simple Moving Average (SMA)
// ============================================================================

export function calculateSMA(data: OHLCData[], period: number): LineData[] {
  const result: LineData[] = [];

  for (let i = 0; i < data.length; i++) {
    if (i < period - 1) {
      result.push({ time: data[i].time, value: null });
    } else {
      let sum = 0;
      for (let j = 0; j < period; j++) {
        sum += data[i - j].close;
      }
      result.push({ time: data[i].time, value: sum / period });
    }
  }

  return result;
}

// ============================================================================
// Exponential Moving Average (EMA)
// ============================================================================

export function calculateEMA(data: OHLCData[], period: number): LineData[] {
  const result: LineData[] = [];
  const multiplier = 2 / (period + 1);

  // Calculate initial SMA for first EMA value
  let ema: number | null = null;

  for (let i = 0; i < data.length; i++) {
    if (i < period - 1) {
      result.push({ time: data[i].time, value: null });
    } else if (i === period - 1) {
      // First EMA is SMA
      let sum = 0;
      for (let j = 0; j < period; j++) {
        sum += data[i - j].close;
      }
      ema = sum / period;
      result.push({ time: data[i].time, value: ema });
    } else {
      // EMA = (Close - Previous EMA) * multiplier + Previous EMA
      ema = (data[i].close - ema!) * multiplier + ema!;
      result.push({ time: data[i].time, value: ema });
    }
  }

  return result;
}

// ============================================================================
// Relative Strength Index (RSI) - Wilder's smoothing
// ============================================================================

export function calculateRSI(data: OHLCData[], period: number = 14): LineData[] {
  const result: LineData[] = [];

  if (data.length < period + 1) {
    return data.map(d => ({ time: d.time, value: null }));
  }

  // Calculate price changes
  const changes: number[] = [];
  for (let i = 1; i < data.length; i++) {
    changes.push(data[i].close - data[i - 1].close);
  }

  // Separate gains and losses
  const gains: number[] = changes.map(c => c > 0 ? c : 0);
  const losses: number[] = changes.map(c => c < 0 ? -c : 0);

  // First RSI value uses SMA
  let avgGain = gains.slice(0, period).reduce((a, b) => a + b, 0) / period;
  let avgLoss = losses.slice(0, period).reduce((a, b) => a + b, 0) / period;

  // Fill initial nulls
  for (let i = 0; i <= period; i++) {
    result.push({ time: data[i].time, value: null });
  }

  // Calculate first RSI
  let rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
  let rsi = 100 - (100 / (1 + rs));
  result[period] = { time: data[period].time, value: rsi };

  // Calculate subsequent RSI using Wilder's smoothing
  for (let i = period + 1; i < data.length; i++) {
    const changeIdx = i - 1;
    avgGain = (avgGain * (period - 1) + gains[changeIdx]) / period;
    avgLoss = (avgLoss * (period - 1) + losses[changeIdx]) / period;

    rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
    rsi = 100 - (100 / (1 + rs));
    result.push({ time: data[i].time, value: rsi });
  }

  return result;
}

// ============================================================================
// MACD (Moving Average Convergence Divergence)
// ============================================================================

export function calculateMACD(
  data: OHLCData[],
  fastPeriod: number = 12,
  slowPeriod: number = 26,
  signalPeriod: number = 9
): MACDResult {
  const fastEMA = calculateEMA(data, fastPeriod);
  const slowEMA = calculateEMA(data, slowPeriod);

  // MACD Line = Fast EMA - Slow EMA
  const macdLine: LineData[] = data.map((d, i) => {
    const fast = fastEMA[i].value;
    const slow = slowEMA[i].value;
    return {
      time: d.time,
      value: fast !== null && slow !== null ? fast - slow : null,
    };
  });

  // Signal Line = EMA of MACD Line
  const signalLine: LineData[] = [];
  const multiplier = 2 / (signalPeriod + 1);
  let signalEma: number | null = null;
  let validCount = 0;

  for (let i = 0; i < macdLine.length; i++) {
    const macdValue = macdLine[i].value;

    if (macdValue === null) {
      signalLine.push({ time: data[i].time, value: null });
      continue;
    }

    validCount++;

    if (validCount < signalPeriod) {
      signalLine.push({ time: data[i].time, value: null });
    } else if (validCount === signalPeriod) {
      // First signal is SMA of MACD values
      let sum = 0;
      let count = 0;
      for (let j = i; j >= 0 && count < signalPeriod; j--) {
        if (macdLine[j].value !== null) {
          sum += macdLine[j].value!;
          count++;
        }
      }
      signalEma = sum / signalPeriod;
      signalLine.push({ time: data[i].time, value: signalEma });
    } else {
      signalEma = (macdValue - signalEma!) * multiplier + signalEma!;
      signalLine.push({ time: data[i].time, value: signalEma });
    }
  }

  // Histogram = MACD Line - Signal Line
  const histogram: HistogramData[] = data.map((d, i) => {
    const macd = macdLine[i].value;
    const signal = signalLine[i].value;
    const value = macd !== null && signal !== null ? macd - signal : null;
    return {
      time: d.time,
      value,
      color: value !== null ? (value >= 0 ? '#26a69a' : '#ef5350') : undefined,
    };
  });

  return { macd: macdLine, signal: signalLine, histogram };
}

// ============================================================================
// Bollinger Bands
// ============================================================================

export function calculateBollinger(
  data: OHLCData[],
  period: number = 20,
  stdDev: number = 2
): BollingerResult {
  const middle = calculateSMA(data, period);
  const upper: LineData[] = [];
  const lower: LineData[] = [];

  for (let i = 0; i < data.length; i++) {
    if (i < period - 1 || middle[i].value === null) {
      upper.push({ time: data[i].time, value: null });
      lower.push({ time: data[i].time, value: null });
    } else {
      // Calculate standard deviation
      let sum = 0;
      for (let j = 0; j < period; j++) {
        const diff = data[i - j].close - middle[i].value!;
        sum += diff * diff;
      }
      const std = Math.sqrt(sum / period);

      upper.push({ time: data[i].time, value: middle[i].value! + stdDev * std });
      lower.push({ time: data[i].time, value: middle[i].value! - stdDev * std });
    }
  }

  return { upper, middle, lower };
}

// ============================================================================
// Average True Range (ATR) - Wilder's smoothing
// ============================================================================

export function calculateATR(data: OHLCData[], period: number = 14): LineData[] {
  const result: LineData[] = [];

  if (data.length < 2) {
    return data.map(d => ({ time: d.time, value: null }));
  }

  // Calculate True Range
  const tr: number[] = [data[0].high - data[0].low]; // First TR is just high - low

  for (let i = 1; i < data.length; i++) {
    const high = data[i].high;
    const low = data[i].low;
    const prevClose = data[i - 1].close;

    const tr1 = high - low;
    const tr2 = Math.abs(high - prevClose);
    const tr3 = Math.abs(low - prevClose);

    tr.push(Math.max(tr1, tr2, tr3));
  }

  // First ATR is SMA of TR
  result.push({ time: data[0].time, value: null });

  for (let i = 1; i < data.length; i++) {
    if (i < period) {
      result.push({ time: data[i].time, value: null });
    } else if (i === period) {
      const sum = tr.slice(1, period + 1).reduce((a, b) => a + b, 0);
      result.push({ time: data[i].time, value: sum / period });
    } else {
      // Wilder's smoothing: ATR = ((Previous ATR * (period - 1)) + Current TR) / period
      const prevATR = result[i - 1].value!;
      const atr = (prevATR * (period - 1) + tr[i]) / period;
      result.push({ time: data[i].time, value: atr });
    }
  }

  return result;
}

// ============================================================================
// Volume Weighted Average Price (VWAP) - for intraday
// ============================================================================

export function calculateVWAP(data: OHLCData[]): LineData[] {
  let cumulativeTPV = 0; // Typical Price * Volume
  let cumulativeVolume = 0;

  return data.map(d => {
    if (d.volume === undefined || d.volume === 0) {
      return { time: d.time, value: null };
    }

    const typicalPrice = (d.high + d.low + d.close) / 3;
    cumulativeTPV += typicalPrice * d.volume;
    cumulativeVolume += d.volume;

    return {
      time: d.time,
      value: cumulativeVolume > 0 ? cumulativeTPV / cumulativeVolume : null,
    };
  });
}

// ============================================================================
// Utility: Convert price-only data to synthetic OHLC
// ============================================================================

export function convertToOHLC(
  priceData: Array<{ date: string; value: number }>,
  volatilityPercent: number = 1.5
): OHLCData[] {
  return priceData.map((d, i) => {
    const close = d.value;
    // Create realistic OHLC from close price with some variance
    const variance = close * (volatilityPercent / 100);
    const open = i > 0 ? priceData[i - 1].value : close;
    const high = Math.max(open, close) + Math.random() * variance;
    const low = Math.min(open, close) - Math.random() * variance;

    return {
      time: d.date,
      open,
      high,
      low,
      close,
      volume: Math.floor(Math.random() * 1000000) + 100000, // Synthetic volume
    };
  });
}

// ============================================================================
// Indicator Configuration Types
// ============================================================================

export type IndicatorType = 'sma' | 'ema' | 'rsi' | 'macd' | 'bollinger' | 'atr' | 'vwap';

export interface IndicatorConfig {
  id: string;
  type: IndicatorType;
  enabled: boolean;
  params: Record<string, number>;
  color?: string;
}

export const defaultIndicatorConfigs: Record<IndicatorType, Omit<IndicatorConfig, 'id'>> = {
  sma: { type: 'sma', enabled: false, params: { period: 20 }, color: '#2196f3' },
  ema: { type: 'ema', enabled: false, params: { period: 20 }, color: '#ff9800' },
  rsi: { type: 'rsi', enabled: false, params: { period: 14 } },
  macd: { type: 'macd', enabled: false, params: { fast: 12, slow: 26, signal: 9 } },
  bollinger: { type: 'bollinger', enabled: false, params: { period: 20, stdDev: 2 }, color: '#9c27b0' },
  atr: { type: 'atr', enabled: false, params: { period: 14 } },
  vwap: { type: 'vwap', enabled: false, params: {}, color: '#e91e63' },
};
