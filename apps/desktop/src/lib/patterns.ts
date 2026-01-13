/**
 * Candlestick Pattern Recognition
 * Detects common candlestick patterns for technical analysis
 */

import type { OHLCData } from './indicators';

// ============================================================================
// Types
// ============================================================================

export type CandlestickPattern =
  // Single Candle Patterns
  | 'doji'
  | 'hammer'
  | 'inverted_hammer'
  | 'hanging_man'
  | 'shooting_star'
  | 'spinning_top'
  | 'marubozu_bullish'
  | 'marubozu_bearish'
  // Two Candle Patterns
  | 'engulfing_bullish'
  | 'engulfing_bearish'
  | 'harami_bullish'
  | 'harami_bearish'
  | 'piercing_line'
  | 'dark_cloud_cover'
  | 'tweezer_top'
  | 'tweezer_bottom'
  // Three Candle Patterns
  | 'morning_star'
  | 'evening_star'
  | 'three_white_soldiers'
  | 'three_black_crows'
  | 'three_inside_up'
  | 'three_inside_down';

export type PatternDirection = 'bullish' | 'bearish' | 'neutral';
export type PatternReliability = 'high' | 'medium' | 'low';

export interface PatternMatch {
  pattern: CandlestickPattern;
  name: string;
  startIndex: number;
  endIndex: number;
  direction: PatternDirection;
  reliability: PatternReliability;
  description: string;
}

// ============================================================================
// Helper Functions
// ============================================================================

function getBodySize(candle: OHLCData): number {
  return Math.abs(candle.close - candle.open);
}

function getCandleRange(candle: OHLCData): number {
  return candle.high - candle.low;
}

function getUpperWick(candle: OHLCData): number {
  return candle.high - Math.max(candle.open, candle.close);
}

function getLowerWick(candle: OHLCData): number {
  return Math.min(candle.open, candle.close) - candle.low;
}

function isBullish(candle: OHLCData): boolean {
  return candle.close > candle.open;
}

function isBearish(candle: OHLCData): boolean {
  return candle.close < candle.open;
}

function getAverageBody(data: OHLCData[], index: number, period: number = 10): number {
  let sum = 0;
  let count = 0;
  for (let i = Math.max(0, index - period); i < index; i++) {
    sum += getBodySize(data[i]);
    count++;
  }
  return count > 0 ? sum / count : getBodySize(data[index]);
}

function isInDowntrend(data: OHLCData[], index: number, period: number = 5): boolean {
  if (index < period) return false;
  const startPrice = data[index - period].close;
  const endPrice = data[index].close;
  return endPrice < startPrice * 0.98; // At least 2% decline
}

function isInUptrend(data: OHLCData[], index: number, period: number = 5): boolean {
  if (index < period) return false;
  const startPrice = data[index - period].close;
  const endPrice = data[index].close;
  return endPrice > startPrice * 1.02; // At least 2% increase
}

// ============================================================================
// Single Candle Patterns
// ============================================================================

function detectDoji(candle: OHLCData, avgBody: number): boolean {
  const body = getBodySize(candle);
  const range = getCandleRange(candle);
  return body < avgBody * 0.1 && range > avgBody * 0.5;
}

function detectHammer(candle: OHLCData, avgBody: number, isDowntrend: boolean): boolean {
  const body = getBodySize(candle);
  const lowerWick = getLowerWick(candle);
  const upperWick = getUpperWick(candle);

  return isDowntrend &&
    lowerWick >= body * 2 &&
    upperWick <= body * 0.3 &&
    body >= avgBody * 0.3;
}

function detectInvertedHammer(candle: OHLCData, avgBody: number, isDowntrend: boolean): boolean {
  const body = getBodySize(candle);
  const lowerWick = getLowerWick(candle);
  const upperWick = getUpperWick(candle);

  return isDowntrend &&
    upperWick >= body * 2 &&
    lowerWick <= body * 0.3 &&
    body >= avgBody * 0.3;
}

function detectHangingMan(candle: OHLCData, avgBody: number, isUptrend: boolean): boolean {
  const body = getBodySize(candle);
  const lowerWick = getLowerWick(candle);
  const upperWick = getUpperWick(candle);

  return isUptrend &&
    lowerWick >= body * 2 &&
    upperWick <= body * 0.3 &&
    body >= avgBody * 0.3;
}

function detectShootingStar(candle: OHLCData, avgBody: number, isUptrend: boolean): boolean {
  const body = getBodySize(candle);
  const lowerWick = getLowerWick(candle);
  const upperWick = getUpperWick(candle);

  return isUptrend &&
    upperWick >= body * 2 &&
    lowerWick <= body * 0.3 &&
    body >= avgBody * 0.3;
}

function detectSpinningTop(candle: OHLCData, avgBody: number): boolean {
  const body = getBodySize(candle);
  const upperWick = getUpperWick(candle);
  const lowerWick = getLowerWick(candle);

  return body < avgBody * 0.5 &&
    upperWick > body &&
    lowerWick > body;
}

function detectMarubozu(candle: OHLCData, avgBody: number): 'bullish' | 'bearish' | null {
  const body = getBodySize(candle);
  const upperWick = getUpperWick(candle);
  const lowerWick = getLowerWick(candle);
  const range = getCandleRange(candle);

  if (body > avgBody * 1.5 && upperWick < range * 0.05 && lowerWick < range * 0.05) {
    return isBullish(candle) ? 'bullish' : 'bearish';
  }
  return null;
}

// ============================================================================
// Two Candle Patterns
// ============================================================================

function detectEngulfingBullish(prev: OHLCData, curr: OHLCData, isDowntrend: boolean): boolean {
  return isDowntrend &&
    isBearish(prev) &&
    isBullish(curr) &&
    curr.open < prev.close &&
    curr.close > prev.open;
}

function detectEngulfingBearish(prev: OHLCData, curr: OHLCData, isUptrend: boolean): boolean {
  return isUptrend &&
    isBullish(prev) &&
    isBearish(curr) &&
    curr.open > prev.close &&
    curr.close < prev.open;
}

function detectHaramiBullish(prev: OHLCData, curr: OHLCData, isDowntrend: boolean): boolean {
  return isDowntrend &&
    isBearish(prev) &&
    isBullish(curr) &&
    curr.open > prev.close &&
    curr.close < prev.open &&
    getBodySize(curr) < getBodySize(prev) * 0.5;
}

function detectHaramiBearish(prev: OHLCData, curr: OHLCData, isUptrend: boolean): boolean {
  return isUptrend &&
    isBullish(prev) &&
    isBearish(curr) &&
    curr.open < prev.close &&
    curr.close > prev.open &&
    getBodySize(curr) < getBodySize(prev) * 0.5;
}

function detectPiercingLine(prev: OHLCData, curr: OHLCData, isDowntrend: boolean): boolean {
  const prevMidpoint = (prev.open + prev.close) / 2;

  return isDowntrend &&
    isBearish(prev) &&
    isBullish(curr) &&
    curr.open < prev.low &&
    curr.close > prevMidpoint &&
    curr.close < prev.open;
}

function detectDarkCloudCover(prev: OHLCData, curr: OHLCData, isUptrend: boolean): boolean {
  const prevMidpoint = (prev.open + prev.close) / 2;

  return isUptrend &&
    isBullish(prev) &&
    isBearish(curr) &&
    curr.open > prev.high &&
    curr.close < prevMidpoint &&
    curr.close > prev.open;
}

function detectTweezerTop(prev: OHLCData, curr: OHLCData, isUptrend: boolean): boolean {
  const tolerance = getCandleRange(prev) * 0.05;

  return isUptrend &&
    isBullish(prev) &&
    isBearish(curr) &&
    Math.abs(prev.high - curr.high) < tolerance;
}

function detectTweezerBottom(prev: OHLCData, curr: OHLCData, isDowntrend: boolean): boolean {
  const tolerance = getCandleRange(prev) * 0.05;

  return isDowntrend &&
    isBearish(prev) &&
    isBullish(curr) &&
    Math.abs(prev.low - curr.low) < tolerance;
}

// ============================================================================
// Three Candle Patterns
// ============================================================================

function detectMorningStar(
  first: OHLCData,
  second: OHLCData,
  third: OHLCData,
  avgBody: number,
  isDowntrend: boolean
): boolean {
  const firstBody = getBodySize(first);
  const secondBody = getBodySize(second);
  const thirdBody = getBodySize(third);

  return isDowntrend &&
    isBearish(first) &&
    firstBody > avgBody &&
    secondBody < avgBody * 0.5 &&
    second.close < first.close &&
    isBullish(third) &&
    thirdBody > avgBody &&
    third.close > (first.open + first.close) / 2;
}

function detectEveningStar(
  first: OHLCData,
  second: OHLCData,
  third: OHLCData,
  avgBody: number,
  isUptrend: boolean
): boolean {
  const firstBody = getBodySize(first);
  const secondBody = getBodySize(second);
  const thirdBody = getBodySize(third);

  return isUptrend &&
    isBullish(first) &&
    firstBody > avgBody &&
    secondBody < avgBody * 0.5 &&
    second.close > first.close &&
    isBearish(third) &&
    thirdBody > avgBody &&
    third.close < (first.open + first.close) / 2;
}

function detectThreeWhiteSoldiers(
  first: OHLCData,
  second: OHLCData,
  third: OHLCData,
  avgBody: number
): boolean {
  return isBullish(first) && isBullish(second) && isBullish(third) &&
    second.open > first.open && second.close > first.close &&
    third.open > second.open && third.close > second.close &&
    getBodySize(first) > avgBody * 0.7 &&
    getBodySize(second) > avgBody * 0.7 &&
    getBodySize(third) > avgBody * 0.7 &&
    getUpperWick(first) < getBodySize(first) * 0.3 &&
    getUpperWick(second) < getBodySize(second) * 0.3 &&
    getUpperWick(third) < getBodySize(third) * 0.3;
}

function detectThreeBlackCrows(
  first: OHLCData,
  second: OHLCData,
  third: OHLCData,
  avgBody: number
): boolean {
  return isBearish(first) && isBearish(second) && isBearish(third) &&
    second.open < first.open && second.close < first.close &&
    third.open < second.open && third.close < second.close &&
    getBodySize(first) > avgBody * 0.7 &&
    getBodySize(second) > avgBody * 0.7 &&
    getBodySize(third) > avgBody * 0.7 &&
    getLowerWick(first) < getBodySize(first) * 0.3 &&
    getLowerWick(second) < getBodySize(second) * 0.3 &&
    getLowerWick(third) < getBodySize(third) * 0.3;
}

function detectThreeInsideUp(
  first: OHLCData,
  second: OHLCData,
  third: OHLCData,
  isDowntrend: boolean
): boolean {
  return isDowntrend &&
    isBearish(first) &&
    isBullish(second) &&
    second.open > first.close &&
    second.close < first.open &&
    getBodySize(second) < getBodySize(first) * 0.5 &&
    isBullish(third) &&
    third.close > first.open;
}

function detectThreeInsideDown(
  first: OHLCData,
  second: OHLCData,
  third: OHLCData,
  isUptrend: boolean
): boolean {
  return isUptrend &&
    isBullish(first) &&
    isBearish(second) &&
    second.open < first.close &&
    second.close > first.open &&
    getBodySize(second) < getBodySize(first) * 0.5 &&
    isBearish(third) &&
    third.close < first.open;
}

// ============================================================================
// Pattern Names and Descriptions
// ============================================================================

const patternInfo: Record<CandlestickPattern, { name: string; description: string }> = {
  doji: { name: 'Doji', description: 'Unentschlossenheit, mögliche Trendwende' },
  hammer: { name: 'Hammer', description: 'Bullisches Umkehrmuster nach Abwärtstrend' },
  inverted_hammer: { name: 'Umgekehrter Hammer', description: 'Mögliche bullische Umkehr' },
  hanging_man: { name: 'Hanging Man', description: 'Bärisches Warnsignal nach Aufwärtstrend' },
  shooting_star: { name: 'Shooting Star', description: 'Bärisches Umkehrmuster nach Aufwärtstrend' },
  spinning_top: { name: 'Spinning Top', description: 'Unentschlossenheit im Markt' },
  marubozu_bullish: { name: 'Bullish Marubozu', description: 'Starke bullische Kerze ohne Dochte' },
  marubozu_bearish: { name: 'Bearish Marubozu', description: 'Starke bärische Kerze ohne Dochte' },
  engulfing_bullish: { name: 'Bullish Engulfing', description: 'Starkes bullisches Umkehrmuster' },
  engulfing_bearish: { name: 'Bearish Engulfing', description: 'Starkes bärisches Umkehrmuster' },
  harami_bullish: { name: 'Bullish Harami', description: 'Mögliche bullische Umkehr' },
  harami_bearish: { name: 'Bearish Harami', description: 'Mögliche bärische Umkehr' },
  piercing_line: { name: 'Piercing Line', description: 'Bullisches Umkehrmuster' },
  dark_cloud_cover: { name: 'Dark Cloud Cover', description: 'Bärisches Umkehrmuster' },
  tweezer_top: { name: 'Tweezer Top', description: 'Bärisches Umkehrmuster mit gleichem Hoch' },
  tweezer_bottom: { name: 'Tweezer Bottom', description: 'Bullisches Umkehrmuster mit gleichem Tief' },
  morning_star: { name: 'Morning Star', description: 'Starkes bullisches Umkehrmuster' },
  evening_star: { name: 'Evening Star', description: 'Starkes bärisches Umkehrmuster' },
  three_white_soldiers: { name: 'Three White Soldiers', description: 'Starkes bullisches Fortsetzungsmuster' },
  three_black_crows: { name: 'Three Black Crows', description: 'Starkes bärisches Fortsetzungsmuster' },
  three_inside_up: { name: 'Three Inside Up', description: 'Bullisches Bestätigungsmuster' },
  three_inside_down: { name: 'Three Inside Down', description: 'Bärisches Bestätigungsmuster' },
};

// ============================================================================
// Main Pattern Detection Function
// ============================================================================

export function detectCandlestickPatterns(data: OHLCData[]): PatternMatch[] {
  const patterns: PatternMatch[] = [];

  if (data.length < 10) return patterns;

  // Only check the last N candles for performance
  const lookback = Math.min(20, data.length - 5);
  const startIndex = data.length - lookback;

  for (let i = startIndex; i < data.length; i++) {
    const candle = data[i];
    const avgBody = getAverageBody(data, i);
    const inDowntrend = isInDowntrend(data, i);
    const inUptrend = isInUptrend(data, i);

    // Single candle patterns
    if (detectDoji(candle, avgBody)) {
      patterns.push({
        pattern: 'doji',
        name: patternInfo.doji.name,
        startIndex: i,
        endIndex: i,
        direction: 'neutral',
        reliability: 'medium',
        description: patternInfo.doji.description,
      });
    }

    if (detectHammer(candle, avgBody, inDowntrend)) {
      patterns.push({
        pattern: 'hammer',
        name: patternInfo.hammer.name,
        startIndex: i,
        endIndex: i,
        direction: 'bullish',
        reliability: 'high',
        description: patternInfo.hammer.description,
      });
    }

    if (detectInvertedHammer(candle, avgBody, inDowntrend)) {
      patterns.push({
        pattern: 'inverted_hammer',
        name: patternInfo.inverted_hammer.name,
        startIndex: i,
        endIndex: i,
        direction: 'bullish',
        reliability: 'medium',
        description: patternInfo.inverted_hammer.description,
      });
    }

    if (detectHangingMan(candle, avgBody, inUptrend)) {
      patterns.push({
        pattern: 'hanging_man',
        name: patternInfo.hanging_man.name,
        startIndex: i,
        endIndex: i,
        direction: 'bearish',
        reliability: 'medium',
        description: patternInfo.hanging_man.description,
      });
    }

    if (detectShootingStar(candle, avgBody, inUptrend)) {
      patterns.push({
        pattern: 'shooting_star',
        name: patternInfo.shooting_star.name,
        startIndex: i,
        endIndex: i,
        direction: 'bearish',
        reliability: 'high',
        description: patternInfo.shooting_star.description,
      });
    }

    if (detectSpinningTop(candle, avgBody)) {
      patterns.push({
        pattern: 'spinning_top',
        name: patternInfo.spinning_top.name,
        startIndex: i,
        endIndex: i,
        direction: 'neutral',
        reliability: 'low',
        description: patternInfo.spinning_top.description,
      });
    }

    const marubozu = detectMarubozu(candle, avgBody);
    if (marubozu) {
      const pattern = marubozu === 'bullish' ? 'marubozu_bullish' : 'marubozu_bearish';
      patterns.push({
        pattern,
        name: patternInfo[pattern].name,
        startIndex: i,
        endIndex: i,
        direction: marubozu,
        reliability: 'high',
        description: patternInfo[pattern].description,
      });
    }

    // Two candle patterns (need previous candle)
    if (i > 0) {
      const prev = data[i - 1];

      if (detectEngulfingBullish(prev, candle, inDowntrend)) {
        patterns.push({
          pattern: 'engulfing_bullish',
          name: patternInfo.engulfing_bullish.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bullish',
          reliability: 'high',
          description: patternInfo.engulfing_bullish.description,
        });
      }

      if (detectEngulfingBearish(prev, candle, inUptrend)) {
        patterns.push({
          pattern: 'engulfing_bearish',
          name: patternInfo.engulfing_bearish.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bearish',
          reliability: 'high',
          description: patternInfo.engulfing_bearish.description,
        });
      }

      if (detectHaramiBullish(prev, candle, inDowntrend)) {
        patterns.push({
          pattern: 'harami_bullish',
          name: patternInfo.harami_bullish.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bullish',
          reliability: 'medium',
          description: patternInfo.harami_bullish.description,
        });
      }

      if (detectHaramiBearish(prev, candle, inUptrend)) {
        patterns.push({
          pattern: 'harami_bearish',
          name: patternInfo.harami_bearish.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bearish',
          reliability: 'medium',
          description: patternInfo.harami_bearish.description,
        });
      }

      if (detectPiercingLine(prev, candle, inDowntrend)) {
        patterns.push({
          pattern: 'piercing_line',
          name: patternInfo.piercing_line.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bullish',
          reliability: 'high',
          description: patternInfo.piercing_line.description,
        });
      }

      if (detectDarkCloudCover(prev, candle, inUptrend)) {
        patterns.push({
          pattern: 'dark_cloud_cover',
          name: patternInfo.dark_cloud_cover.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bearish',
          reliability: 'high',
          description: patternInfo.dark_cloud_cover.description,
        });
      }

      if (detectTweezerTop(prev, candle, inUptrend)) {
        patterns.push({
          pattern: 'tweezer_top',
          name: patternInfo.tweezer_top.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bearish',
          reliability: 'medium',
          description: patternInfo.tweezer_top.description,
        });
      }

      if (detectTweezerBottom(prev, candle, inDowntrend)) {
        patterns.push({
          pattern: 'tweezer_bottom',
          name: patternInfo.tweezer_bottom.name,
          startIndex: i - 1,
          endIndex: i,
          direction: 'bullish',
          reliability: 'medium',
          description: patternInfo.tweezer_bottom.description,
        });
      }
    }

    // Three candle patterns
    if (i > 1) {
      const first = data[i - 2];
      const second = data[i - 1];
      const third = candle;
      const inDowntrendFirst = isInDowntrend(data, i - 2);
      const inUptrendFirst = isInUptrend(data, i - 2);

      if (detectMorningStar(first, second, third, avgBody, inDowntrendFirst)) {
        patterns.push({
          pattern: 'morning_star',
          name: patternInfo.morning_star.name,
          startIndex: i - 2,
          endIndex: i,
          direction: 'bullish',
          reliability: 'high',
          description: patternInfo.morning_star.description,
        });
      }

      if (detectEveningStar(first, second, third, avgBody, inUptrendFirst)) {
        patterns.push({
          pattern: 'evening_star',
          name: patternInfo.evening_star.name,
          startIndex: i - 2,
          endIndex: i,
          direction: 'bearish',
          reliability: 'high',
          description: patternInfo.evening_star.description,
        });
      }

      if (detectThreeWhiteSoldiers(first, second, third, avgBody)) {
        patterns.push({
          pattern: 'three_white_soldiers',
          name: patternInfo.three_white_soldiers.name,
          startIndex: i - 2,
          endIndex: i,
          direction: 'bullish',
          reliability: 'high',
          description: patternInfo.three_white_soldiers.description,
        });
      }

      if (detectThreeBlackCrows(first, second, third, avgBody)) {
        patterns.push({
          pattern: 'three_black_crows',
          name: patternInfo.three_black_crows.name,
          startIndex: i - 2,
          endIndex: i,
          direction: 'bearish',
          reliability: 'high',
          description: patternInfo.three_black_crows.description,
        });
      }

      if (detectThreeInsideUp(first, second, third, inDowntrendFirst)) {
        patterns.push({
          pattern: 'three_inside_up',
          name: patternInfo.three_inside_up.name,
          startIndex: i - 2,
          endIndex: i,
          direction: 'bullish',
          reliability: 'high',
          description: patternInfo.three_inside_up.description,
        });
      }

      if (detectThreeInsideDown(first, second, third, inUptrendFirst)) {
        patterns.push({
          pattern: 'three_inside_down',
          name: patternInfo.three_inside_down.name,
          startIndex: i - 2,
          endIndex: i,
          direction: 'bearish',
          reliability: 'high',
          description: patternInfo.three_inside_down.description,
        });
      }
    }
  }

  // Remove duplicates (keep highest reliability for same end index)
  const uniquePatterns = new Map<number, PatternMatch>();
  for (const pattern of patterns) {
    const key = pattern.endIndex;
    const existing = uniquePatterns.get(key);
    if (!existing ||
      (pattern.reliability === 'high' && existing.reliability !== 'high') ||
      (pattern.reliability === 'medium' && existing.reliability === 'low')) {
      uniquePatterns.set(key, pattern);
    }
  }

  // Sort by end index descending (most recent first)
  return Array.from(uniquePatterns.values())
    .sort((a, b) => b.endIndex - a.endIndex);
}

// ============================================================================
// Get Pattern at specific index
// ============================================================================

export function getPatternAtIndex(data: OHLCData[], index: number): PatternMatch | null {
  const patterns = detectCandlestickPatterns(data);
  return patterns.find(p => p.endIndex === index) || null;
}

// ============================================================================
// Get latest patterns (most recent N)
// ============================================================================

export function getLatestPatterns(data: OHLCData[], count: number = 5): PatternMatch[] {
  const patterns = detectCandlestickPatterns(data);
  return patterns.slice(0, count);
}
