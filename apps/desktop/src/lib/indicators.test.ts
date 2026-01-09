/**
 * Unit Tests for Technical Analysis Indicators
 */

import { describe, it, expect } from 'vitest';
import {
  calculateSMA,
  calculateEMA,
  calculateRSI,
  calculateMACD,
  calculateBollinger,
  calculateATR,
  convertToOHLC,
  type OHLCData,
} from './indicators';

// Helper to generate test OHLC data
function generateOHLCData(prices: number[]): OHLCData[] {
  return prices.map((close, i) => ({
    time: `2024-01-${String(i + 1).padStart(2, '0')}`,
    open: close - 0.5,
    high: close + 1,
    low: close - 1,
    close,
    volume: 1000000,
  }));
}

describe('calculateSMA', () => {
  it('should return null for insufficient data', () => {
    const data = generateOHLCData([100, 101, 102]);
    const result = calculateSMA(data, 5);

    expect(result[0].value).toBeNull();
    expect(result[1].value).toBeNull();
    expect(result[2].value).toBeNull();
  });

  it('should calculate correct SMA values', () => {
    const data = generateOHLCData([100, 102, 104, 106, 108]);
    const result = calculateSMA(data, 3);

    // SMA starts at index 2 (period - 1)
    expect(result[0].value).toBeNull();
    expect(result[1].value).toBeNull();
    expect(result[2].value).toBeCloseTo(102, 4); // (100+102+104)/3 = 102
    expect(result[3].value).toBeCloseTo(104, 4); // (102+104+106)/3 = 104
    expect(result[4].value).toBeCloseTo(106, 4); // (104+106+108)/3 = 106
  });

  it('should preserve time values', () => {
    const data = generateOHLCData([100, 102, 104]);
    const result = calculateSMA(data, 2);

    expect(result[0].time).toBe('2024-01-01');
    expect(result[1].time).toBe('2024-01-02');
    expect(result[2].time).toBe('2024-01-03');
  });
});

describe('calculateEMA', () => {
  it('should return null for insufficient data', () => {
    const data = generateOHLCData([100, 101, 102]);
    const result = calculateEMA(data, 5);

    expect(result[0].value).toBeNull();
    expect(result[1].value).toBeNull();
    expect(result[2].value).toBeNull();
  });

  it('should start with SMA for first EMA value', () => {
    const data = generateOHLCData([100, 102, 104, 106, 108]);
    const result = calculateEMA(data, 3);

    // First EMA at index 2 should equal SMA
    expect(result[2].value).toBeCloseTo(102, 4); // SMA = (100+102+104)/3 = 102
  });

  it('should calculate subsequent EMA values correctly', () => {
    const data = generateOHLCData([100, 102, 104, 106, 108, 110]);
    const result = calculateEMA(data, 3);

    // EMA multiplier = 2 / (3 + 1) = 0.5
    const firstEma = 102; // SMA
    const secondEma = (106 - firstEma) * 0.5 + firstEma; // 104
    const thirdEma = (108 - secondEma) * 0.5 + secondEma; // 106
    const fourthEma = (110 - thirdEma) * 0.5 + thirdEma; // 108

    expect(result[3].value).toBeCloseTo(secondEma, 4);
    expect(result[4].value).toBeCloseTo(thirdEma, 4);
    expect(result[5].value).toBeCloseTo(fourthEma, 4);
  });
});

describe('calculateRSI', () => {
  it('should return null for insufficient data', () => {
    const data = generateOHLCData([100, 101, 102, 103, 104]);
    const result = calculateRSI(data, 14);

    result.forEach((point) => {
      expect(point.value).toBeNull();
    });
  });

  it('should return RSI between 0 and 100', () => {
    // Generate more data for RSI calculation
    const prices = Array.from({ length: 30 }, (_, i) => 100 + Math.sin(i * 0.5) * 10);
    const data = generateOHLCData(prices);
    const result = calculateRSI(data, 14);

    result.forEach((point) => {
      if (point.value !== null) {
        expect(point.value).toBeGreaterThanOrEqual(0);
        expect(point.value).toBeLessThanOrEqual(100);
      }
    });
  });

  it('should return ~100 for all gains', () => {
    // Steadily increasing prices
    const prices = Array.from({ length: 20 }, (_, i) => 100 + i * 2);
    const data = generateOHLCData(prices);
    const result = calculateRSI(data, 14);

    const lastValue = result[result.length - 1].value;
    expect(lastValue).not.toBeNull();
    expect(lastValue!).toBeGreaterThan(95); // Should be near 100
  });

  it('should return ~0 for all losses', () => {
    // Steadily decreasing prices
    const prices = Array.from({ length: 20 }, (_, i) => 100 - i * 2);
    const data = generateOHLCData(prices);
    const result = calculateRSI(data, 14);

    const lastValue = result[result.length - 1].value;
    expect(lastValue).not.toBeNull();
    expect(lastValue!).toBeLessThan(5); // Should be near 0
  });
});

describe('calculateMACD', () => {
  it('should return three arrays with same length as input', () => {
    const data = generateOHLCData(Array.from({ length: 50 }, (_, i) => 100 + i));
    const result = calculateMACD(data);

    expect(result.macd.length).toBe(data.length);
    expect(result.signal.length).toBe(data.length);
    expect(result.histogram.length).toBe(data.length);
  });

  it('should return null for insufficient data', () => {
    const data = generateOHLCData([100, 101, 102]);
    const result = calculateMACD(data, 12, 26, 9);

    expect(result.macd[0].value).toBeNull();
    expect(result.signal[0].value).toBeNull();
    expect(result.histogram[0].value).toBeNull();
  });

  it('should calculate histogram as MACD - Signal', () => {
    const prices = Array.from({ length: 50 }, (_, i) => 100 + Math.sin(i * 0.3) * 10);
    const data = generateOHLCData(prices);
    const result = calculateMACD(data);

    for (let i = 0; i < result.histogram.length; i++) {
      const macd = result.macd[i].value;
      const signal = result.signal[i].value;
      const histogram = result.histogram[i].value;

      if (macd !== null && signal !== null) {
        expect(histogram).toBeCloseTo(macd - signal, 6);
      }
    }
  });

  it('should have green/red colors for histogram', () => {
    const prices = Array.from({ length: 50 }, (_, i) => 100 + Math.sin(i * 0.3) * 10);
    const data = generateOHLCData(prices);
    const result = calculateMACD(data);

    result.histogram.forEach((point) => {
      if (point.value !== null) {
        if (point.value >= 0) {
          expect(point.color).toBe('#26a69a');
        } else {
          expect(point.color).toBe('#ef5350');
        }
      }
    });
  });
});

describe('calculateBollinger', () => {
  it('should return three bands with same length as input', () => {
    const data = generateOHLCData(Array.from({ length: 30 }, (_, i) => 100 + i));
    const result = calculateBollinger(data);

    expect(result.upper.length).toBe(data.length);
    expect(result.middle.length).toBe(data.length);
    expect(result.lower.length).toBe(data.length);
  });

  it('should have middle band equal to SMA', () => {
    const prices = Array.from({ length: 30 }, () => 100 + Math.random() * 10);
    const data = generateOHLCData(prices);
    const period = 20;

    const bollinger = calculateBollinger(data, period);
    const sma = calculateSMA(data, period);

    for (let i = 0; i < data.length; i++) {
      expect(bollinger.middle[i].value).toBe(sma[i].value);
    }
  });

  it('should have upper > middle > lower when data is present', () => {
    const prices = Array.from({ length: 30 }, (_, i) => 100 + Math.sin(i) * 5);
    const data = generateOHLCData(prices);
    const result = calculateBollinger(data, 20, 2);

    for (let i = 0; i < data.length; i++) {
      const upper = result.upper[i].value;
      const middle = result.middle[i].value;
      const lower = result.lower[i].value;

      if (upper !== null && middle !== null && lower !== null) {
        expect(upper).toBeGreaterThan(middle);
        expect(middle).toBeGreaterThan(lower);
      }
    }
  });

  it('should increase band width with higher stdDev', () => {
    const prices = Array.from({ length: 30 }, (_, i) => 100 + Math.sin(i) * 5);
    const data = generateOHLCData(prices);

    const narrow = calculateBollinger(data, 20, 1);
    const wide = calculateBollinger(data, 20, 3);

    const lastIndex = data.length - 1;
    const narrowWidth =
      narrow.upper[lastIndex].value! - narrow.lower[lastIndex].value!;
    const wideWidth = wide.upper[lastIndex].value! - wide.lower[lastIndex].value!;

    expect(wideWidth).toBeGreaterThan(narrowWidth);
  });
});

describe('calculateATR', () => {
  it('should return null for insufficient data', () => {
    const data = generateOHLCData([100, 101, 102]);
    const result = calculateATR(data, 14);

    result.forEach((point) => {
      expect(point.value).toBeNull();
    });
  });

  it('should return positive values', () => {
    const prices = Array.from({ length: 30 }, (_, i) => 100 + Math.sin(i) * 5);
    const data = generateOHLCData(prices);
    const result = calculateATR(data);

    result.forEach((point) => {
      if (point.value !== null) {
        expect(point.value).toBeGreaterThan(0);
      }
    });
  });

  it('should increase with higher volatility', () => {
    // Low volatility data
    const lowVolData: OHLCData[] = Array.from({ length: 20 }, (_, i) => ({
      time: `2024-01-${String(i + 1).padStart(2, '0')}`,
      open: 100,
      high: 101,
      low: 99,
      close: 100,
    }));

    // High volatility data
    const highVolData: OHLCData[] = Array.from({ length: 20 }, (_, i) => ({
      time: `2024-01-${String(i + 1).padStart(2, '0')}`,
      open: 100,
      high: 110,
      low: 90,
      close: 100,
    }));

    const lowATR = calculateATR(lowVolData, 14);
    const highATR = calculateATR(highVolData, 14);

    const lastIndex = lowVolData.length - 1;
    expect(highATR[lastIndex].value!).toBeGreaterThan(lowATR[lastIndex].value!);
  });
});

describe('convertToOHLC', () => {
  it('should convert price data to OHLC format', () => {
    const priceData = [
      { date: '2024-01-01', value: 100 },
      { date: '2024-01-02', value: 105 },
      { date: '2024-01-03', value: 102 },
    ];

    const result = convertToOHLC(priceData);

    expect(result.length).toBe(3);
    expect(result[0].time).toBe('2024-01-01');
    expect(result[0].close).toBe(100);
    expect(result[1].time).toBe('2024-01-02');
    expect(result[1].close).toBe(105);
  });

  it('should set open to previous close', () => {
    const priceData = [
      { date: '2024-01-01', value: 100 },
      { date: '2024-01-02', value: 105 },
      { date: '2024-01-03', value: 102 },
    ];

    const result = convertToOHLC(priceData);

    expect(result[0].open).toBe(100); // First open equals close
    expect(result[1].open).toBe(100); // Previous close
    expect(result[2].open).toBe(105); // Previous close
  });

  it('should generate high >= max(open, close)', () => {
    const priceData = [
      { date: '2024-01-01', value: 100 },
      { date: '2024-01-02', value: 105 },
    ];

    const result = convertToOHLC(priceData, 0); // No extra variance

    result.forEach((candle) => {
      expect(candle.high).toBeGreaterThanOrEqual(Math.max(candle.open, candle.close));
    });
  });

  it('should generate low <= min(open, close)', () => {
    const priceData = [
      { date: '2024-01-01', value: 100 },
      { date: '2024-01-02', value: 95 },
    ];

    const result = convertToOHLC(priceData, 0); // No extra variance

    result.forEach((candle) => {
      expect(candle.low).toBeLessThanOrEqual(Math.min(candle.open, candle.close));
    });
  });

  it('should generate synthetic volume', () => {
    const priceData = [
      { date: '2024-01-01', value: 100 },
      { date: '2024-01-02', value: 105 },
    ];

    const result = convertToOHLC(priceData);

    for (const candle of result) {
      expect(candle.volume).toBeDefined();
      expect(candle.volume).toBeGreaterThan(0);
    }
  });
});
