/**
 * TradingView-like Chart Component using lightweight-charts v5
 *
 * Features:
 * - Candlestick chart with volume
 * - Overlay indicators (SMA, EMA, Bollinger)
 * - Crosshair, zoom, pan
 * - Responsive resize
 */

import { useEffect, useRef, useState, useMemo } from 'react';
import {
  createChart,
  ColorType,
  CrosshairMode,
  LineStyle,
  CandlestickSeries,
  HistogramSeries,
  LineSeries,
} from 'lightweight-charts';
import type {
  IChartApi,
  ISeriesApi,
  CandlestickData,
  HistogramData,
  LineData,
  Time,
} from 'lightweight-charts';
import type {
  OHLCData,
  IndicatorConfig,
  LineData as IndicatorLineData,
  HistogramData as IndicatorHistogramData,
  StochasticResult,
  ADXResult,
  IchimokuResult,
  PivotPointsResult,
  FibonacciResult,
} from '../../lib/indicators';
import type { ChartAnnotationWithId } from '../../lib/types';
import {
  calculateSMA,
  calculateEMA,
  calculateRSI,
  calculateMACD,
  calculateBollinger,
  calculateATR,
  calculateStochastic,
  calculateOBV,
  calculateADX,
  calculateIchimoku,
  calculatePivotPoints,
  calculateFibonacci,
} from '../../lib/indicators';

// ============================================================================
// Types
// ============================================================================

export interface TradingViewChartProps {
  data: OHLCData[];
  indicators: IndicatorConfig[];
  height?: number;
  theme?: 'light' | 'dark';
  showVolume?: boolean;
  symbol?: string;
  /** AI-generated chart annotations */
  annotations?: ChartAnnotationWithId[];
  /** Callback when an annotation is clicked */
  onAnnotationClick?: (annotation: ChartAnnotationWithId) => void;
}

// ============================================================================
// Helper Functions
// ============================================================================

const convertToLineData = (data: IndicatorLineData[]): LineData<Time>[] => {
  return data
    .filter(d => d.value !== null)
    .map(d => ({
      time: d.time as Time,
      value: d.value as number,
    }));
};

const convertToHistogramData = (data: IndicatorHistogramData[]): HistogramData<Time>[] => {
  return data
    .filter(d => d.value !== null)
    .map(d => ({
      time: d.time as Time,
      value: d.value as number,
      color: d.color,
    }));
};

// ============================================================================
// Main Component
// ============================================================================

export function TradingViewChart({
  data,
  indicators,
  height = 500,
  theme = 'dark',
  showVolume = true,
  symbol,
  annotations = [],
  onAnnotationClick,
}: TradingViewChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const [legendData, setLegendData] = useState<{
    open?: number;
    high?: number;
    low?: number;
    close?: number;
    volume?: number;
    time?: string;
  }>({});

  // Hover state for annotation tooltips
  const [hoveredAnnotation, setHoveredAnnotation] = useState<ChartAnnotationWithId | null>(null);
  const [tooltipPosition, setTooltipPosition] = useState<{ x: number; y: number } | null>(null);

  // Memoize chart data conversion
  const chartData = useMemo(() => {
    if (!data || data.length < 2) return { candles: [], volume: [] };

    const candles: CandlestickData<Time>[] = data.map(d => ({
      time: d.time as Time,
      open: d.open,
      high: d.high,
      low: d.low,
      close: d.close,
    }));

    const volume: HistogramData<Time>[] = data.map(d => ({
      time: d.time as Time,
      value: d.volume || 0,
      color: d.close >= d.open ? 'rgba(38, 166, 154, 0.5)' : 'rgba(239, 83, 80, 0.5)',
    }));

    return { candles, volume };
  }, [data]);

  // Calculate indicator data
  const indicatorData = useMemo(() => {
    if (!data || data.length < 2) return {};

    const result: Record<string, unknown> = {};

    for (const indicator of indicators) {
      if (!indicator.enabled) continue;

      try {
        switch (indicator.type) {
          case 'sma':
            result[indicator.id] = calculateSMA(data, indicator.params.period);
            break;
          case 'ema':
            result[indicator.id] = calculateEMA(data, indicator.params.period);
            break;
          case 'rsi':
            result[indicator.id] = calculateRSI(data, indicator.params.period);
            break;
          case 'macd':
            result[indicator.id] = calculateMACD(
              data,
              indicator.params.fast,
              indicator.params.slow,
              indicator.params.signal
            );
            break;
          case 'bollinger':
            result[indicator.id] = calculateBollinger(
              data,
              indicator.params.period,
              indicator.params.stdDev
            );
            break;
          case 'atr':
            result[indicator.id] = calculateATR(data, indicator.params.period as number);
            break;
          case 'stochastic':
            result[indicator.id] = calculateStochastic(
              data,
              indicator.params.kPeriod as number,
              indicator.params.kSlowPeriod as number,
              indicator.params.dPeriod as number
            );
            break;
          case 'obv':
            result[indicator.id] = calculateOBV(data);
            break;
          case 'adx':
            result[indicator.id] = calculateADX(data, indicator.params.period as number);
            break;
          case 'ichimoku':
            result[indicator.id] = calculateIchimoku(
              data,
              indicator.params.tenkan as number,
              indicator.params.kijun as number,
              indicator.params.senkouB as number
            );
            break;
          case 'pivot':
            result[indicator.id] = calculatePivotPoints(
              data,
              indicator.pivotType || 'standard'
            );
            break;
          case 'fibonacci':
            result[indicator.id] = calculateFibonacci(data, indicator.params.lookback as number);
            break;
        }
      } catch (e) {
        console.warn(`Failed to calculate ${indicator.type}:`, e);
      }
    }

    return result;
  }, [data, indicators]);

  // Create and manage chart
  useEffect(() => {
    if (!containerRef.current) return;
    if (chartData.candles.length < 2) return;

    // Cleanup existing chart
    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
    }

    const isDark = theme === 'dark';

    // Use container height if available, otherwise fall back to prop
    const containerHeight = containerRef.current.clientHeight;
    const chartHeight = containerHeight > 100 ? containerHeight : height;

    // Create chart
    const chart = createChart(containerRef.current, {
      width: containerRef.current.clientWidth,
      height: chartHeight,
      layout: {
        background: { type: ColorType.Solid, color: isDark ? '#1a1a2e' : '#ffffff' },
        textColor: isDark ? '#d1d4dc' : '#333333',
      },
      grid: {
        vertLines: { color: isDark ? '#2B2B43' : '#e1e1e1' },
        horzLines: { color: isDark ? '#2B2B43' : '#e1e1e1' },
      },
      crosshair: {
        mode: CrosshairMode.Normal,
      },
      rightPriceScale: {
        borderColor: isDark ? '#2B2B43' : '#e1e1e1',
      },
      timeScale: {
        borderColor: isDark ? '#2B2B43' : '#e1e1e1',
        timeVisible: true,
        secondsVisible: false,
      },
    });

    chartRef.current = chart;

    // Add candlestick series
    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      upColor: '#26a69a',
      downColor: '#ef5350',
      borderVisible: false,
      wickUpColor: '#26a69a',
      wickDownColor: '#ef5350',
    });
    candlestickSeries.setData(chartData.candles);

    // Add volume series
    let volumeSeries: ISeriesApi<'Histogram'> | null = null;
    if (showVolume && chartData.volume.length > 0) {
      volumeSeries = chart.addSeries(HistogramSeries, {
        priceFormat: { type: 'volume' },
        priceScaleId: 'volume',
      });
      volumeSeries.setData(chartData.volume);

      chart.priceScale('volume').applyOptions({
        scaleMargins: { top: 0.8, bottom: 0 },
      });
    }

    // Add overlay indicators (SMA, EMA, Bollinger)
    for (const indicator of indicators) {
      if (!indicator.enabled) continue;
      const indData = indicatorData[indicator.id];
      if (!indData) continue;

      try {
        if (indicator.type === 'sma' || indicator.type === 'ema') {
          const series = chart.addSeries(LineSeries, {
            color: indicator.color || '#2196f3',
            lineWidth: 1,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          series.setData(convertToLineData(indData as IndicatorLineData[]));
        }

        if (indicator.type === 'bollinger') {
          const bb = indData as { upper: IndicatorLineData[]; middle: IndicatorLineData[]; lower: IndicatorLineData[] };
          const color = indicator.color || '#9c27b0';

          const upperSeries = chart.addSeries(LineSeries, {
            color,
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          upperSeries.setData(convertToLineData(bb.upper));

          const middleSeries = chart.addSeries(LineSeries, {
            color,
            lineWidth: 1,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          middleSeries.setData(convertToLineData(bb.middle));

          const lowerSeries = chart.addSeries(LineSeries, {
            color,
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          lowerSeries.setData(convertToLineData(bb.lower));
        }

        // RSI on separate scale (shown at bottom)
        if (indicator.type === 'rsi') {
          const rsiSeries = chart.addSeries(LineSeries, {
            color: '#7e57c2',
            lineWidth: 2,
            priceScaleId: 'rsi',
            priceLineVisible: false,
            lastValueVisible: true,
          });
          rsiSeries.setData(convertToLineData(indData as IndicatorLineData[]));

          chart.priceScale('rsi').applyOptions({
            scaleMargins: { top: 0.8, bottom: 0.02 },
          });

          // RSI reference lines
          rsiSeries.createPriceLine({ price: 70, color: '#ef5350', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: false });
          rsiSeries.createPriceLine({ price: 30, color: '#26a69a', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: false });
        }

        // MACD
        if (indicator.type === 'macd') {
          const macd = indData as { macd: IndicatorLineData[]; signal: IndicatorLineData[]; histogram: IndicatorHistogramData[] };

          const histSeries = chart.addSeries(HistogramSeries, {
            priceScaleId: 'macd',
            priceLineVisible: false,
          });
          histSeries.setData(convertToHistogramData(macd.histogram));

          const macdSeries = chart.addSeries(LineSeries, {
            color: '#2196f3',
            lineWidth: 1,
            priceScaleId: 'macd',
            priceLineVisible: false,
            lastValueVisible: false,
          });
          macdSeries.setData(convertToLineData(macd.macd));

          const signalSeries = chart.addSeries(LineSeries, {
            color: '#ff9800',
            lineWidth: 1,
            priceScaleId: 'macd',
            priceLineVisible: false,
            lastValueVisible: false,
          });
          signalSeries.setData(convertToLineData(macd.signal));

          chart.priceScale('macd').applyOptions({
            scaleMargins: { top: 0.9, bottom: 0.02 },
          });
        }

        // ATR
        if (indicator.type === 'atr') {
          const atrSeries = chart.addSeries(LineSeries, {
            color: '#00bcd4',
            lineWidth: 2,
            priceScaleId: 'atr',
            priceLineVisible: false,
            lastValueVisible: true,
          });
          atrSeries.setData(convertToLineData(indData as IndicatorLineData[]));

          chart.priceScale('atr').applyOptions({
            scaleMargins: { top: 0.9, bottom: 0.02 },
          });
        }

        // Stochastic Oscillator
        if (indicator.type === 'stochastic') {
          const stoch = indData as StochasticResult;

          const kSeries = chart.addSeries(LineSeries, {
            color: '#2196f3',
            lineWidth: 2,
            priceScaleId: 'stochastic',
            priceLineVisible: false,
            lastValueVisible: true,
          });
          kSeries.setData(convertToLineData(stoch.k));

          const dSeries = chart.addSeries(LineSeries, {
            color: '#ff9800',
            lineWidth: 1,
            priceScaleId: 'stochastic',
            priceLineVisible: false,
            lastValueVisible: false,
          });
          dSeries.setData(convertToLineData(stoch.d));

          chart.priceScale('stochastic').applyOptions({
            scaleMargins: { top: 0.85, bottom: 0.02 },
          });

          // Overbought/Oversold lines
          kSeries.createPriceLine({ price: 80, color: '#ef5350', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: false });
          kSeries.createPriceLine({ price: 20, color: '#26a69a', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: false });
        }

        // OBV (On-Balance Volume)
        if (indicator.type === 'obv') {
          const obvSeries = chart.addSeries(LineSeries, {
            color: '#9c27b0',
            lineWidth: 2,
            priceScaleId: 'obv',
            priceLineVisible: false,
            lastValueVisible: true,
          });
          obvSeries.setData(convertToLineData(indData as IndicatorLineData[]));

          chart.priceScale('obv').applyOptions({
            scaleMargins: { top: 0.85, bottom: 0.02 },
          });
        }

        // ADX with +DI/-DI
        if (indicator.type === 'adx') {
          const adxData = indData as ADXResult;

          const adxSeries = chart.addSeries(LineSeries, {
            color: '#ffffff',
            lineWidth: 2,
            priceScaleId: 'adx',
            priceLineVisible: false,
            lastValueVisible: true,
          });
          adxSeries.setData(convertToLineData(adxData.adx));

          const diPlusSeries = chart.addSeries(LineSeries, {
            color: '#26a69a',
            lineWidth: 1,
            priceScaleId: 'adx',
            priceLineVisible: false,
            lastValueVisible: false,
          });
          diPlusSeries.setData(convertToLineData(adxData.diPlus));

          const diMinusSeries = chart.addSeries(LineSeries, {
            color: '#ef5350',
            lineWidth: 1,
            priceScaleId: 'adx',
            priceLineVisible: false,
            lastValueVisible: false,
          });
          diMinusSeries.setData(convertToLineData(adxData.diMinus));

          chart.priceScale('adx').applyOptions({
            scaleMargins: { top: 0.85, bottom: 0.02 },
          });

          // Trend strength threshold line
          adxSeries.createPriceLine({ price: 25, color: '#ffeb3b', lineWidth: 1, lineStyle: LineStyle.Dashed, axisLabelVisible: false });
        }

        // Ichimoku Cloud
        if (indicator.type === 'ichimoku') {
          const ichi = indData as IchimokuResult;
          const baseColor = indicator.color || '#00bcd4';

          // Tenkan-sen (Conversion Line)
          const tenkanSeries = chart.addSeries(LineSeries, {
            color: '#2196f3',
            lineWidth: 1,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          tenkanSeries.setData(convertToLineData(ichi.tenkan));

          // Kijun-sen (Base Line)
          const kijunSeries = chart.addSeries(LineSeries, {
            color: '#ef5350',
            lineWidth: 1,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          kijunSeries.setData(convertToLineData(ichi.kijun));

          // Senkou Span A (cloud upper)
          const senkouASeries = chart.addSeries(LineSeries, {
            color: 'rgba(38, 166, 154, 0.8)',
            lineWidth: 1,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          senkouASeries.setData(convertToLineData(ichi.senkouA));

          // Senkou Span B (cloud lower)
          const senkouBSeries = chart.addSeries(LineSeries, {
            color: 'rgba(239, 83, 80, 0.8)',
            lineWidth: 1,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          senkouBSeries.setData(convertToLineData(ichi.senkouB));

          // Chikou Span (Lagging)
          const chikouSeries = chart.addSeries(LineSeries, {
            color: baseColor,
            lineWidth: 1,
            lineStyle: LineStyle.Dotted,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          chikouSeries.setData(convertToLineData(ichi.chikou));
        }

        // Pivot Points
        if (indicator.type === 'pivot') {
          const pivots = indData as PivotPointsResult;

          // Pivot line (main)
          const pivotSeries = chart.addSeries(LineSeries, {
            color: '#ffeb3b',
            lineWidth: 2,
            priceLineVisible: false,
            lastValueVisible: true,
          });
          pivotSeries.setData(convertToLineData(pivots.pivot));

          // Resistance lines (R1, R2, R3)
          const r1Series = chart.addSeries(LineSeries, {
            color: 'rgba(239, 83, 80, 0.8)',
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          r1Series.setData(convertToLineData(pivots.r1));

          const r2Series = chart.addSeries(LineSeries, {
            color: 'rgba(239, 83, 80, 0.6)',
            lineWidth: 1,
            lineStyle: LineStyle.Dotted,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          r2Series.setData(convertToLineData(pivots.r2));

          // Support lines (S1, S2, S3)
          const s1Series = chart.addSeries(LineSeries, {
            color: 'rgba(38, 166, 154, 0.8)',
            lineWidth: 1,
            lineStyle: LineStyle.Dashed,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          s1Series.setData(convertToLineData(pivots.s1));

          const s2Series = chart.addSeries(LineSeries, {
            color: 'rgba(38, 166, 154, 0.6)',
            lineWidth: 1,
            lineStyle: LineStyle.Dotted,
            priceLineVisible: false,
            lastValueVisible: false,
          });
          s2Series.setData(convertToLineData(pivots.s2));
        }

        // Fibonacci Retracements
        if (indicator.type === 'fibonacci') {
          const fib = indData as FibonacciResult;

          // Draw horizontal lines for each level
          fib.levels.forEach((level, idx) => {
            const colors = ['#ef5350', '#ff9800', '#ffeb3b', '#4caf50', '#2196f3', '#9c27b0', '#26a69a'];
            candlestickSeries.createPriceLine({
              price: level.price,
              color: colors[idx % colors.length],
              lineWidth: 1,
              lineStyle: idx === 0 || idx === fib.levels.length - 1 ? LineStyle.Solid : LineStyle.Dashed,
              axisLabelVisible: true,
              title: level.label,
            });
          });
        }
      } catch (e) {
        console.warn(`Failed to render indicator ${indicator.type}:`, e);
      }
    }

    // ========================================================================
    // Render AI Annotations
    // ========================================================================
    if (annotations && annotations.length > 0) {
      // Support and Resistance lines (horizontal price lines)
      annotations
        .filter(a => a.type === 'support' || a.type === 'resistance')
        .forEach(annotation => {
          try {
            candlestickSeries.createPriceLine({
              price: annotation.price,
              color: annotation.type === 'support' ? '#26a69a' : '#ef5350',
              lineWidth: 2,
              lineStyle: LineStyle.Dashed,
              axisLabelVisible: true,
              title: annotation.title,
            });
          } catch (e) {
            console.warn(`Failed to render annotation ${annotation.title}:`, e);
          }
        });

      // Target and StopLoss lines
      annotations
        .filter(a => a.type === 'target' || a.type === 'stoploss')
        .forEach(annotation => {
          try {
            candlestickSeries.createPriceLine({
              price: annotation.price,
              color: annotation.type === 'target' ? '#2196f3' : '#ff5722',
              lineWidth: 1,
              lineStyle: LineStyle.Dotted,
              axisLabelVisible: true,
              title: annotation.title,
            });
          } catch (e) {
            console.warn(`Failed to render annotation ${annotation.title}:`, e);
          }
        });

      // Note: Pattern/Signal markers are not supported in lightweight-charts v5
      // They are shown in the annotations list below the chart instead
    }

    // Crosshair handler
    chart.subscribeCrosshairMove(param => {
      if (!param || !param.time) {
        setLegendData({});
        setHoveredAnnotation(null);
        setTooltipPosition(null);
        return;
      }

      const candlePrice = param.seriesData.get(candlestickSeries);
      if (candlePrice && 'open' in candlePrice) {
        const volValue = volumeSeries ? param.seriesData.get(volumeSeries) : null;
        setLegendData({
          time: param.time as string,
          open: candlePrice.open,
          high: candlePrice.high,
          low: candlePrice.low,
          close: candlePrice.close,
          volume: volValue && 'value' in volValue ? (volValue as { value: number }).value : undefined,
        });
      }

      // Check for annotation hover (support/resistance lines)
      if (annotations && annotations.length > 0 && param.point) {
        const { x, y } = param.point;

        // Find annotation near cursor (20px tolerance)
        // Use series.priceToCoordinate instead of priceScale
        const nearbyAnnotation = annotations.find(ann => {
          if (ann.type !== 'support' && ann.type !== 'resistance' && ann.type !== 'target' && ann.type !== 'stoploss') {
            return false;
          }
          const annotationY = candlestickSeries.priceToCoordinate(ann.price);
          if (annotationY === null) return false;
          return Math.abs(y - annotationY) < 20;
        });

        if (nearbyAnnotation) {
          setHoveredAnnotation(nearbyAnnotation);
          setTooltipPosition({ x, y });
        } else {
          setHoveredAnnotation(null);
          setTooltipPosition(null);
        }
      }
    });

    chart.timeScale().fitContent();

    // Resize observer - handle both width and height
    const resizeObserver = new ResizeObserver(entries => {
      for (const entry of entries) {
        const { width, height: newHeight } = entry.contentRect;
        if (chart && width > 0 && newHeight > 100) {
          chart.applyOptions({ width, height: newHeight });
        }
      }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      resizeObserver.disconnect();
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
      }
    };
  }, [chartData, indicatorData, indicators, height, theme, showVolume, annotations]);

  // Early return for no data
  if (!data || data.length < 2) {
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Keine ausreichenden Daten für Chart
      </div>
    );
  }

  return (
    <div className="relative w-full h-full">
      {/* Legend */}
      <div className="absolute top-2 left-2 z-10 bg-background/80 backdrop-blur-sm rounded-lg px-3 py-2 text-xs font-mono">
        {symbol && <span className="font-bold text-foreground mr-4">{symbol}</span>}
        {legendData.time && (
          <>
            <span className="text-muted-foreground mr-2">{legendData.time}</span>
            <span className="text-muted-foreground">O:</span>
            <span className="text-foreground ml-1 mr-2">{legendData.open?.toFixed(2)}</span>
            <span className="text-muted-foreground">H:</span>
            <span className="text-foreground ml-1 mr-2">{legendData.high?.toFixed(2)}</span>
            <span className="text-muted-foreground">L:</span>
            <span className="text-foreground ml-1 mr-2">{legendData.low?.toFixed(2)}</span>
            <span className="text-muted-foreground">C:</span>
            <span
              className={`ml-1 mr-2 ${
                legendData.close && legendData.open
                  ? legendData.close >= legendData.open
                    ? 'text-emerald-500'
                    : 'text-red-500'
                  : 'text-foreground'
              }`}
            >
              {legendData.close?.toFixed(2)}
            </span>
            {legendData.volume !== undefined && (
              <>
                <span className="text-muted-foreground">V:</span>
                <span className="text-foreground ml-1">
                  {legendData.volume.toLocaleString('de-DE')}
                </span>
              </>
            )}
          </>
        )}
      </div>

      {/* Annotation Tooltip Overlay */}
      {hoveredAnnotation && tooltipPosition && (
        <div
          className="absolute z-20 pointer-events-none"
          style={{
            left: Math.min(tooltipPosition.x + 12, containerRef.current?.clientWidth ? containerRef.current.clientWidth - 260 : tooltipPosition.x),
            top: Math.max(tooltipPosition.y - 80, 10),
          }}
        >
          <div className="bg-popover/95 backdrop-blur-sm border border-border rounded-lg shadow-xl p-3 max-w-[250px] pointer-events-auto">
            <div className="flex items-center gap-2 mb-2">
              <span
                className={`w-2.5 h-2.5 rounded-full shrink-0 ${
                  hoveredAnnotation.signal === 'bullish'
                    ? 'bg-emerald-500'
                    : hoveredAnnotation.signal === 'bearish'
                    ? 'bg-red-500'
                    : 'bg-amber-500'
                }`}
              />
              <span className="font-medium text-sm truncate">{hoveredAnnotation.title}</span>
              <span className="text-xs text-muted-foreground ml-auto shrink-0">
                {Math.round(hoveredAnnotation.confidence * 100)}%
              </span>
            </div>
            <p className="text-xs text-muted-foreground line-clamp-3">{hoveredAnnotation.description}</p>
            <div className="flex items-center justify-between mt-2 pt-2 border-t border-border">
              <span className="text-xs font-mono">@ {hoveredAnnotation.price.toFixed(2)}</span>
              {onAnnotationClick && (
                <button
                  onClick={() => onAnnotationClick(hoveredAnnotation)}
                  className="text-xs px-2 py-0.5 bg-muted hover:bg-muted/80 rounded transition-colors"
                >
                  Details
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Chart Container - absolute positioning ensures it fills the parent */}
      <div ref={containerRef} className="absolute inset-0" />
    </div>
  );
}

export default TradingViewChart;
