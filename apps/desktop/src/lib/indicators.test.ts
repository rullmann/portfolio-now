/**
 * Unit Tests for indicators.ts utilities
 *
 * Calculation function tests (SMA, EMA, RSI, MACD, Bollinger, ATR) have been removed
 * since those functions are now in Rust with tests in src-tauri/src/indicators/.
 */

import { describe, it, expect } from 'vitest';
import { convertToOHLC } from './indicators';

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
