/**
 * Rust-native Technical Analysis - Tauri invoke wrappers
 *
 * All technical analysis runs in Rust (src-tauri/src/indicators/).
 * This file provides typed async wrappers around Tauri invoke calls.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  OHLCData,
  LineData,
  MACDResult,
  BollingerResult,
  StochasticResult,
  ADXResult,
  IchimokuResult,
  PivotPointsResult,
  FibonacciResult,
  IndicatorConfig,
  AllIndicatorsResult,
  PatternMatch,
  TechnicalSignal,
  SignalDetectionConfig,
  RegimeAnalysis,
  SetupScore,
  RiskAnalysis,
  TradingAnalysis,
} from './indicators';
import type {
  ScreenerFilter,
  ScreenerResult,
  SecurityData,
} from './screener';

// ============================================================================
// Individual Indicators
// ============================================================================

export async function calculateSMA(data: OHLCData[], period: number): Promise<LineData[]> {
  return invoke('calculate_sma', { data, period });
}

export async function calculateEMA(data: OHLCData[], period: number): Promise<LineData[]> {
  return invoke('calculate_ema', { data, period });
}

export async function calculateRSI(data: OHLCData[], period: number = 14): Promise<LineData[]> {
  return invoke('calculate_rsi', { data, period });
}

export async function calculateMACD(
  data: OHLCData[],
  fast: number = 12,
  slow: number = 26,
  signal: number = 9,
): Promise<MACDResult> {
  return invoke('calculate_macd', { data, fast, slow, signal });
}

export async function calculateBollinger(
  data: OHLCData[],
  period: number = 20,
  stdDev: number = 2,
): Promise<BollingerResult> {
  return invoke('calculate_bollinger', { data, period, stdDev });
}

export async function calculateATR(data: OHLCData[], period: number = 14): Promise<LineData[]> {
  return invoke('calculate_atr', { data, period });
}

export async function calculateVWAP(data: OHLCData[]): Promise<LineData[]> {
  return invoke('calculate_vwap', { data });
}

export async function calculateStochastic(
  data: OHLCData[],
  kPeriod: number = 14,
  kSlow: number = 3,
  dPeriod: number = 3,
): Promise<StochasticResult> {
  return invoke('calculate_stochastic', { data, kPeriod, kSlow, dPeriod });
}

export async function calculateOBV(data: OHLCData[]): Promise<LineData[]> {
  return invoke('calculate_obv', { data });
}

export async function calculateADX(data: OHLCData[], period: number = 14): Promise<ADXResult> {
  return invoke('calculate_adx', { data, period });
}

export async function calculateIchimoku(
  data: OHLCData[],
  tenkan: number = 9,
  kijun: number = 26,
  senkouB: number = 52,
): Promise<IchimokuResult> {
  return invoke('calculate_ichimoku', { data, tenkan, kijun, senkouB });
}

export async function calculatePivotPoints(
  data: OHLCData[],
  pivotType: string = 'standard',
): Promise<PivotPointsResult> {
  return invoke('calculate_pivot_points', { data, pivotType });
}

export async function calculateFibonacci(data: OHLCData[], lookback: number = 50): Promise<FibonacciResult> {
  return invoke('calculate_fibonacci', { data, lookback });
}

export async function convertToHeikinAshi(data: OHLCData[]): Promise<OHLCData[]> {
  return invoke('convert_to_heikin_ashi', { data });
}

// ============================================================================
// Batch: All indicators in one Rust call
// ============================================================================

interface IndicatorRequest {
  indicatorType: string;
  params: Record<string, number | string>;
}

/** Build request array from active IndicatorConfigs */
export function buildIndicatorRequests(configs: IndicatorConfig[]): IndicatorRequest[] {
  return configs
    .filter(c => c.enabled)
    .map(c => ({
      indicatorType: c.type,
      params: { ...c.params, ...(c.pivotType ? { pivotType: c.pivotType } : {}) },
    }));
}

export async function calculateAllIndicators(
  data: OHLCData[],
  requests: IndicatorRequest[],
): Promise<AllIndicatorsResult> {
  return invoke('calculate_all_indicators', { data, requests });
}

// ============================================================================
// Pattern Detection
// ============================================================================

export async function detectCandlestickPatterns(data: OHLCData[]): Promise<PatternMatch[]> {
  return invoke('detect_candlestick_patterns', { data });
}

// ============================================================================
// Signal Detection
// ============================================================================

export async function getAllSignals(
  data: OHLCData[],
  config?: SignalDetectionConfig,
): Promise<TechnicalSignal[]> {
  return invoke('get_all_signals', { data, config: config ?? null });
}

// ============================================================================
// Screener
// ============================================================================

export async function runScreener(
  securities: SecurityData[],
  filters: ScreenerFilter[],
): Promise<ScreenerResult[]> {
  return invoke('run_screener', {
    securities: securities.map(s => ({
      securityId: s.securityId,
      name: s.name,
      ticker: s.ticker,
      isin: s.isin,
      currency: s.currency,
      ohlcData: s.ohlcData,
    })),
    filters,
  });
}

// ============================================================================
// Trading Analysis (Regime, Setup Scoring, Risk)
// ============================================================================

export async function detectRegime(data: OHLCData[]): Promise<RegimeAnalysis> {
  return invoke('detect_regime', { data });
}

export async function scoreSetup(data: OHLCData[]): Promise<SetupScore> {
  return invoke('score_setup', { data });
}

export async function calculateRisk(
  data: OHLCData[],
  accountSize: number,
  riskPercent?: number,
  entryPrice?: number,
  atrMultiplier?: number,
): Promise<RiskAnalysis> {
  return invoke('calculate_risk', {
    data,
    accountSize,
    riskPercent: riskPercent ?? null,
    entryPrice: entryPrice ?? null,
    atrMultiplier: atrMultiplier ?? null,
  });
}

export async function fullTradingAnalysis(
  data: OHLCData[],
  accountSize?: number,
  riskPercent?: number,
  entryPrice?: number,
): Promise<TradingAnalysis> {
  return invoke('full_trading_analysis', {
    data,
    accountSize: accountSize ?? null,
    riskPercent: riskPercent ?? null,
    entryPrice: entryPrice ?? null,
  });
}
