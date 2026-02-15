/**
 * Technical Analysis - Types, Constants & Utilities
 *
 * All calculation functions have been migrated to Rust (src-tauri/src/indicators/).
 * Use indicators-rust.ts for async Tauri invoke wrappers.
 * This file retains only types, configuration constants, and convertToOHLC().
 */

// ============================================================================
// Data Types
// ============================================================================

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

// ============================================================================
// Indicator Result Types
// ============================================================================

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

export interface AllIndicatorsResult {
  sma?: LineData[];
  ema?: LineData[];
  rsi?: LineData[];
  macd?: MACDResult;
  bollinger?: BollingerResult;
  atr?: LineData[];
  vwap?: LineData[];
  stochastic?: StochasticResult;
  obv?: LineData[];
  adx?: ADXResult;
  ichimoku?: IchimokuResult;
  pivot?: PivotPointsResult;
  fibonacci?: FibonacciResult;
  heikinAshi?: OHLCData[];
}

// ============================================================================
// Indicator Configuration
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
  stochastic: { type: 'stochastic', enabled: false, params: { kPeriod: 14, kSlowPeriod: 3, dPeriod: 3 } },
  obv: { type: 'obv', enabled: false, params: {} },
  adx: { type: 'adx', enabled: false, params: { period: 14 } },
  ichimoku: { type: 'ichimoku', enabled: false, params: { tenkan: 9, kijun: 26, senkouB: 52 }, color: '#00bcd4' },
  pivot: { type: 'pivot', enabled: false, params: {}, pivotType: 'standard' },
  fibonacci: { type: 'fibonacci', enabled: false, params: { lookback: 50 } },
};

// ============================================================================
// Signal Types (migrated from signals.ts)
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
  rsiOversold?: number;
  rsiOverbought?: number;
  adxTrendThreshold?: number;
  adxStrongThreshold?: number;
  bollingerSqueezePeriod?: number;
  divergenceLookback?: number;
}

// ============================================================================
// Pattern Types (migrated from patterns.ts)
// ============================================================================

export type CandlestickPattern =
  | 'doji' | 'hammer' | 'inverted_hammer' | 'hanging_man'
  | 'shooting_star' | 'spinning_top' | 'marubozu_bullish' | 'marubozu_bearish'
  | 'engulfing_bullish' | 'engulfing_bearish' | 'harami_bullish' | 'harami_bearish'
  | 'piercing_line' | 'dark_cloud_cover' | 'tweezer_top' | 'tweezer_bottom'
  | 'morning_star' | 'evening_star' | 'three_white_soldiers' | 'three_black_crows'
  | 'three_inside_up' | 'three_inside_down';

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
// Trading Analysis Types (Regime, Setup Scoring, Risk)
// ============================================================================

export type RegimeType =
  | 'strong_uptrend' | 'uptrend' | 'sideways'
  | 'downtrend' | 'strong_downtrend' | 'squeeze';

export type VolatilityLevel = 'low' | 'normal' | 'high' | 'extreme';

export type SetupType =
  | 'trend_following' | 'breakout' | 'reversal'
  | 'mean_reversion' | 'squeeze' | 'no_setup';

export type TradeDirection = 'long' | 'short' | 'neutral';

export interface RegimeAnalysis {
  regime: RegimeType;
  confidence: number;
  trendStrength: number;
  volatilityLevel: VolatilityLevel;
  momentum: number;
  supportingEvidence: string[];
}

export interface SetupFactorDetail {
  name: string;
  score: number;
  weight: number;
  description: string;
}

export interface SetupScore {
  totalScore: number;
  regimeScore: number;
  momentumScore: number;
  patternScore: number;
  volumeScore: number;
  riskScore: number;
  factors: SetupFactorDetail[];
  setupType: SetupType;
  setupLabel: string;
  direction: TradeDirection;
}

export interface RiskAnalysis {
  atrValue: number;
  atrStopPrice: number;
  atrStopPercent: number;
  suggestedPositionSize: number;
  suggestedPositionValue: number;
  riskPerTrade: number;
  riskRewardRatio: number;
  portfolioImpactPercent: number;
  maxLossPercent: number;
  entryPrice: number;
  stopPrice: number;
  targetPrice: number;
}

export interface TradingAnalysis {
  regime: RegimeAnalysis;
  setup: SetupScore;
  risk: RiskAnalysis | null;
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
      volume: Math.floor(Math.random() * 1000000) + 100000,
    };
  });
}
