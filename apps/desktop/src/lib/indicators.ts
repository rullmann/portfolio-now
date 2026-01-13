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

export interface StochasticResult {
  k: LineData[];  // %K (fast line)
  d: LineData[];  // %D (signal line)
}

export interface ADXResult {
  adx: LineData[];      // Trend strength (0-100)
  diPlus: LineData[];   // +DI
  diMinus: LineData[];  // -DI
}

export interface IchimokuResult {
  tenkan: LineData[];   // Conversion Line (9)
  kijun: LineData[];    // Base Line (26)
  senkouA: LineData[];  // Leading Span A (26 shifted forward)
  senkouB: LineData[];  // Leading Span B (52 shifted forward)
  chikou: LineData[];   // Lagging Span (26 shifted back)
}

export interface PivotPointsResult {
  pivot: LineData[];
  r1: LineData[];
  r2: LineData[];
  r3: LineData[];
  s1: LineData[];
  s2: LineData[];
  s3: LineData[];
}

export interface FibonacciLevel {
  level: number;
  price: number;
  label: string;
}

export interface FibonacciResult {
  levels: FibonacciLevel[];
  swingHigh: { price: number; time: string };
  swingLow: { price: number; time: string };
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
// Stochastic Oscillator
// ============================================================================

export function calculateStochastic(
  data: OHLCData[],
  kPeriod: number = 14,
  kSlowPeriod: number = 3,
  dPeriod: number = 3
): StochasticResult {
  const rawK: (number | null)[] = [];

  // Calculate raw %K: 100 * (Close - Lowest Low) / (Highest High - Lowest Low)
  for (let i = 0; i < data.length; i++) {
    if (i < kPeriod - 1) {
      rawK.push(null);
    } else {
      let lowestLow = Infinity;
      let highestHigh = -Infinity;
      for (let j = 0; j < kPeriod; j++) {
        lowestLow = Math.min(lowestLow, data[i - j].low);
        highestHigh = Math.max(highestHigh, data[i - j].high);
      }
      const range = highestHigh - lowestLow;
      rawK.push(range === 0 ? 50 : 100 * (data[i].close - lowestLow) / range);
    }
  }

  // Smooth %K with SMA (kSlowPeriod)
  const k: LineData[] = [];
  for (let i = 0; i < data.length; i++) {
    if (rawK[i] === null || i < kPeriod - 1 + kSlowPeriod - 1) {
      k.push({ time: data[i].time, value: null });
    } else {
      let sum = 0;
      let count = 0;
      for (let j = 0; j < kSlowPeriod; j++) {
        if (rawK[i - j] !== null) {
          sum += rawK[i - j]!;
          count++;
        }
      }
      k.push({ time: data[i].time, value: count > 0 ? sum / count : null });
    }
  }

  // Calculate %D as SMA of %K (dPeriod)
  const d: LineData[] = [];
  for (let i = 0; i < data.length; i++) {
    if (k[i].value === null || i < kPeriod - 1 + kSlowPeriod - 1 + dPeriod - 1) {
      d.push({ time: data[i].time, value: null });
    } else {
      let sum = 0;
      let count = 0;
      for (let j = 0; j < dPeriod; j++) {
        if (k[i - j].value !== null) {
          sum += k[i - j].value!;
          count++;
        }
      }
      d.push({ time: data[i].time, value: count > 0 ? sum / count : null });
    }
  }

  return { k, d };
}

// ============================================================================
// On-Balance Volume (OBV)
// ============================================================================

export function calculateOBV(data: OHLCData[]): LineData[] {
  const result: LineData[] = [];
  let obv = 0;

  for (let i = 0; i < data.length; i++) {
    const volume = data[i].volume ?? 0;

    if (i === 0) {
      obv = volume;
    } else {
      if (data[i].close > data[i - 1].close) {
        obv += volume;
      } else if (data[i].close < data[i - 1].close) {
        obv -= volume;
      }
      // If close equals previous close, OBV stays the same
    }

    result.push({ time: data[i].time, value: obv });
  }

  return result;
}

// ============================================================================
// Average Directional Index (ADX) with +DI and -DI
// ============================================================================

export function calculateADX(data: OHLCData[], period: number = 14): ADXResult {
  const diPlus: LineData[] = [];
  const diMinus: LineData[] = [];
  const adx: LineData[] = [];

  if (data.length < period + 1) {
    return {
      adx: data.map(d => ({ time: d.time, value: null })),
      diPlus: data.map(d => ({ time: d.time, value: null })),
      diMinus: data.map(d => ({ time: d.time, value: null })),
    };
  }

  // Calculate True Range, +DM, -DM
  const tr: number[] = [];
  const plusDM: number[] = [];
  const minusDM: number[] = [];

  for (let i = 1; i < data.length; i++) {
    const high = data[i].high;
    const low = data[i].low;
    const prevHigh = data[i - 1].high;
    const prevLow = data[i - 1].low;
    const prevClose = data[i - 1].close;

    // True Range
    tr.push(Math.max(high - low, Math.abs(high - prevClose), Math.abs(low - prevClose)));

    // Directional Movement
    const upMove = high - prevHigh;
    const downMove = prevLow - low;

    if (upMove > downMove && upMove > 0) {
      plusDM.push(upMove);
    } else {
      plusDM.push(0);
    }

    if (downMove > upMove && downMove > 0) {
      minusDM.push(downMove);
    } else {
      minusDM.push(0);
    }
  }

  // First value is null
  diPlus.push({ time: data[0].time, value: null });
  diMinus.push({ time: data[0].time, value: null });
  adx.push({ time: data[0].time, value: null });

  // Calculate smoothed TR, +DM, -DM using Wilder's smoothing
  let smoothedTR = tr.slice(0, period).reduce((a, b) => a + b, 0);
  let smoothedPlusDM = plusDM.slice(0, period).reduce((a, b) => a + b, 0);
  let smoothedMinusDM = minusDM.slice(0, period).reduce((a, b) => a + b, 0);

  const dx: number[] = [];

  for (let i = 1; i < data.length; i++) {
    if (i < period) {
      diPlus.push({ time: data[i].time, value: null });
      diMinus.push({ time: data[i].time, value: null });
      adx.push({ time: data[i].time, value: null });
    } else if (i === period) {
      // First smoothed values
      const plusDI = smoothedTR === 0 ? 0 : 100 * (smoothedPlusDM / smoothedTR);
      const minusDI = smoothedTR === 0 ? 0 : 100 * (smoothedMinusDM / smoothedTR);

      diPlus.push({ time: data[i].time, value: plusDI });
      diMinus.push({ time: data[i].time, value: minusDI });

      const diSum = plusDI + minusDI;
      dx.push(diSum === 0 ? 0 : 100 * Math.abs(plusDI - minusDI) / diSum);

      adx.push({ time: data[i].time, value: null }); // ADX needs more data
    } else {
      // Wilder's smoothing
      smoothedTR = smoothedTR - (smoothedTR / period) + tr[i - 1];
      smoothedPlusDM = smoothedPlusDM - (smoothedPlusDM / period) + plusDM[i - 1];
      smoothedMinusDM = smoothedMinusDM - (smoothedMinusDM / period) + minusDM[i - 1];

      const plusDI = smoothedTR === 0 ? 0 : 100 * (smoothedPlusDM / smoothedTR);
      const minusDI = smoothedTR === 0 ? 0 : 100 * (smoothedMinusDM / smoothedTR);

      diPlus.push({ time: data[i].time, value: plusDI });
      diMinus.push({ time: data[i].time, value: minusDI });

      const diSum = plusDI + minusDI;
      dx.push(diSum === 0 ? 0 : 100 * Math.abs(plusDI - minusDI) / diSum);

      // ADX is smoothed average of DX
      if (dx.length >= period) {
        if (dx.length === period) {
          // First ADX is SMA of first 'period' DX values
          const adxValue = dx.reduce((a, b) => a + b, 0) / period;
          adx.push({ time: data[i].time, value: adxValue });
        } else {
          // Wilder's smoothing for ADX
          const prevADX = adx[adx.length - 1].value!;
          const adxValue = (prevADX * (period - 1) + dx[dx.length - 1]) / period;
          adx.push({ time: data[i].time, value: adxValue });
        }
      } else {
        adx.push({ time: data[i].time, value: null });
      }
    }
  }

  return { adx, diPlus, diMinus };
}

// ============================================================================
// Ichimoku Cloud
// ============================================================================

export function calculateIchimoku(
  data: OHLCData[],
  tenkanPeriod: number = 9,
  kijunPeriod: number = 26,
  senkouBPeriod: number = 52
): IchimokuResult {
  const tenkan: LineData[] = [];
  const kijun: LineData[] = [];
  const senkouA: LineData[] = [];
  const senkouB: LineData[] = [];
  const chikou: LineData[] = [];

  // Helper to calculate (highest high + lowest low) / 2 for period
  const calcMidpoint = (endIdx: number, period: number): number | null => {
    if (endIdx < period - 1) return null;
    let high = -Infinity;
    let low = Infinity;
    for (let i = 0; i < period; i++) {
      high = Math.max(high, data[endIdx - i].high);
      low = Math.min(low, data[endIdx - i].low);
    }
    return (high + low) / 2;
  };

  for (let i = 0; i < data.length; i++) {
    // Tenkan-sen (Conversion Line)
    tenkan.push({ time: data[i].time, value: calcMidpoint(i, tenkanPeriod) });

    // Kijun-sen (Base Line)
    kijun.push({ time: data[i].time, value: calcMidpoint(i, kijunPeriod) });

    // Chikou Span (Lagging Span) - current close plotted 26 periods back
    // For index i, chikou at i represents the close that will be plotted at i - kijunPeriod
    chikou.push({ time: data[i].time, value: data[i].close });
  }

  // Senkou Span A and B are shifted forward by kijunPeriod
  // We calculate them for current data but they represent future positions
  for (let i = 0; i < data.length; i++) {
    const tenkanVal = tenkan[i].value;
    const kijunVal = kijun[i].value;

    // Senkou A = (Tenkan + Kijun) / 2
    senkouA.push({
      time: data[i].time,
      value: tenkanVal !== null && kijunVal !== null ? (tenkanVal + kijunVal) / 2 : null,
    });

    // Senkou B = midpoint of senkouBPeriod
    senkouB.push({ time: data[i].time, value: calcMidpoint(i, senkouBPeriod) });
  }

  return { tenkan, kijun, senkouA, senkouB, chikou };
}

// ============================================================================
// Pivot Points (Standard/Floor Method)
// ============================================================================

export function calculatePivotPoints(
  data: OHLCData[],
  pivotType: 'standard' | 'fibonacci' | 'woodie' = 'standard'
): PivotPointsResult {
  const pivot: LineData[] = [];
  const r1: LineData[] = [];
  const r2: LineData[] = [];
  const r3: LineData[] = [];
  const s1: LineData[] = [];
  const s2: LineData[] = [];
  const s3: LineData[] = [];

  for (let i = 0; i < data.length; i++) {
    // Use previous day's data for pivot calculation
    if (i === 0) {
      pivot.push({ time: data[i].time, value: null });
      r1.push({ time: data[i].time, value: null });
      r2.push({ time: data[i].time, value: null });
      r3.push({ time: data[i].time, value: null });
      s1.push({ time: data[i].time, value: null });
      s2.push({ time: data[i].time, value: null });
      s3.push({ time: data[i].time, value: null });
      continue;
    }

    const prev = data[i - 1];
    const H = prev.high;
    const L = prev.low;
    const C = prev.close;

    let P: number, R1: number, R2: number, R3: number, S1: number, S2: number, S3: number;

    if (pivotType === 'standard') {
      P = (H + L + C) / 3;
      R1 = 2 * P - L;
      R2 = P + (H - L);
      R3 = H + 2 * (P - L);
      S1 = 2 * P - H;
      S2 = P - (H - L);
      S3 = L - 2 * (H - P);
    } else if (pivotType === 'fibonacci') {
      P = (H + L + C) / 3;
      const range = H - L;
      R1 = P + 0.382 * range;
      R2 = P + 0.618 * range;
      R3 = P + range;
      S1 = P - 0.382 * range;
      S2 = P - 0.618 * range;
      S3 = P - range;
    } else {
      // Woodie
      P = (H + L + 2 * C) / 4;
      R1 = 2 * P - L;
      R2 = P + (H - L);
      R3 = R1 + (H - L);
      S1 = 2 * P - H;
      S2 = P - (H - L);
      S3 = S1 - (H - L);
    }

    pivot.push({ time: data[i].time, value: P });
    r1.push({ time: data[i].time, value: R1 });
    r2.push({ time: data[i].time, value: R2 });
    r3.push({ time: data[i].time, value: R3 });
    s1.push({ time: data[i].time, value: S1 });
    s2.push({ time: data[i].time, value: S2 });
    s3.push({ time: data[i].time, value: S3 });
  }

  return { pivot, r1, r2, r3, s1, s2, s3 };
}

// ============================================================================
// Fibonacci Retracements (Auto-detect Swing High/Low)
// ============================================================================

export function calculateFibonacci(
  data: OHLCData[],
  lookbackPeriod: number = 50
): FibonacciResult {
  const lookback = Math.min(lookbackPeriod, data.length);
  const recentData = data.slice(-lookback);

  // Find swing high and swing low
  let swingHigh = { price: -Infinity, time: '' };
  let swingLow = { price: Infinity, time: '' };

  for (const d of recentData) {
    if (d.high > swingHigh.price) {
      swingHigh = { price: d.high, time: d.time };
    }
    if (d.low < swingLow.price) {
      swingLow = { price: d.low, time: d.time };
    }
  }

  // Determine direction (uptrend or downtrend based on which came first)
  const highIdx = recentData.findIndex(d => d.time === swingHigh.time);
  const lowIdx = recentData.findIndex(d => d.time === swingLow.time);

  const isUptrend = lowIdx < highIdx;
  const range = swingHigh.price - swingLow.price;

  // Standard Fibonacci levels
  const fibLevels = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1];
  const levels: FibonacciLevel[] = fibLevels.map(level => {
    const price = isUptrend
      ? swingHigh.price - level * range  // Retracement from high
      : swingLow.price + level * range;  // Retracement from low
    return {
      level: level * 100,
      price,
      label: `${(level * 100).toFixed(1)}%`,
    };
  });

  return { levels, swingHigh, swingLow };
}

// ============================================================================
// Heikin-Ashi Conversion
// ============================================================================

export function convertToHeikinAshi(data: OHLCData[]): OHLCData[] {
  const result: OHLCData[] = [];

  for (let i = 0; i < data.length; i++) {
    const current = data[i];

    // HA Close = (Open + High + Low + Close) / 4
    const haClose = (current.open + current.high + current.low + current.close) / 4;

    let haOpen: number;
    if (i === 0) {
      // First HA Open = (Open + Close) / 2
      haOpen = (current.open + current.close) / 2;
    } else {
      // HA Open = (Previous HA Open + Previous HA Close) / 2
      const prev = result[i - 1];
      haOpen = (prev.open + prev.close) / 2;
    }

    // HA High = max(High, HA Open, HA Close)
    const haHigh = Math.max(current.high, haOpen, haClose);

    // HA Low = min(Low, HA Open, HA Close)
    const haLow = Math.min(current.low, haOpen, haClose);

    result.push({
      time: current.time,
      open: haOpen,
      high: haHigh,
      low: haLow,
      close: haClose,
      volume: current.volume,
    });
  }

  return result;
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

export type IndicatorType =
  | 'sma' | 'ema' | 'rsi' | 'macd' | 'bollinger' | 'atr' | 'vwap'
  | 'stochastic' | 'obv' | 'adx' | 'ichimoku' | 'pivot' | 'fibonacci';

export interface IndicatorConfig {
  id: string;
  type: IndicatorType;
  enabled: boolean;
  params: Record<string, number>;
  color?: string;
  /** For pivot points: 'standard' | 'fibonacci' | 'woodie' */
  pivotType?: 'standard' | 'fibonacci' | 'woodie';
}

export const defaultIndicatorConfigs: Record<IndicatorType, Omit<IndicatorConfig, 'id'>> = {
  sma: { type: 'sma', enabled: false, params: { period: 20 }, color: '#2196f3' },
  ema: { type: 'ema', enabled: false, params: { period: 20 }, color: '#ff9800' },
  rsi: { type: 'rsi', enabled: false, params: { period: 14 } },
  macd: { type: 'macd', enabled: false, params: { fast: 12, slow: 26, signal: 9 } },
  bollinger: { type: 'bollinger', enabled: false, params: { period: 20, stdDev: 2 }, color: '#9c27b0' },
  atr: { type: 'atr', enabled: false, params: { period: 14 } },
  vwap: { type: 'vwap', enabled: false, params: {}, color: '#e91e63' },
  // New indicators
  stochastic: { type: 'stochastic', enabled: false, params: { kPeriod: 14, kSlowPeriod: 3, dPeriod: 3 } },
  obv: { type: 'obv', enabled: false, params: {} },
  adx: { type: 'adx', enabled: false, params: { period: 14 } },
  ichimoku: { type: 'ichimoku', enabled: false, params: { tenkan: 9, kijun: 26, senkouB: 52 }, color: '#00bcd4' },
  pivot: { type: 'pivot', enabled: false, params: {}, pivotType: 'standard' },
  fibonacci: { type: 'fibonacci', enabled: false, params: { lookback: 50 } },
};
