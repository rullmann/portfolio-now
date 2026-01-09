/**
 * Mini sparkline chart using TradingView Lightweight Charts.
 * Used for inline price visualization in tables and cards.
 */

import { useEffect, useRef, useMemo } from 'react';
import { createChart, ColorType, AreaSeries, type IChartApi, type AreaData, type Time } from 'lightweight-charts';

interface TradingViewMiniChartProps {
  data: Array<{ date: string; value: number }>;
  width?: number;
  height?: number;
  showTooltip?: boolean;
  className?: string;
}

export function TradingViewMiniChart({
  data,
  width = 120,
  height = 40,
  showTooltip = false,
  className = '',
}: TradingViewMiniChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);

  // Process and sort data
  const chartData = useMemo(() => {
    if (!data || data.length === 0) return [];
    return data
      .map((d) => ({
        time: d.date as Time,
        value: d.value,
      }))
      .sort((a, b) => (a.time as string).localeCompare(b.time as string));
  }, [data]);

  // Calculate if trend is positive
  const isPositive = useMemo(() => {
    if (chartData.length < 2) return true;
    const first = chartData[0]?.value || 0;
    const last = chartData[chartData.length - 1]?.value || 0;
    return last >= first;
  }, [chartData]);

  useEffect(() => {
    if (!chartContainerRef.current || chartData.length === 0) {
      return;
    }

    // Clean up existing chart
    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
    }

    // Create minimal chart
    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: 'transparent',
        attributionLogo: false,
      },
      grid: {
        vertLines: { visible: false },
        horzLines: { visible: false },
      },
      width,
      height,
      rightPriceScale: {
        visible: false,
      },
      leftPriceScale: {
        visible: false,
      },
      timeScale: {
        visible: false,
      },
      crosshair: {
        vertLine: { visible: showTooltip },
        horzLine: { visible: showTooltip },
      },
      handleScroll: false,
      handleScale: false,
    });

    chartRef.current = chart;

    // Create area series
    const color = isPositive ? '#22c55e' : '#ef4444';
    const series = chart.addSeries(AreaSeries, {
      lineColor: color,
      lineWidth: 2,
      topColor: isPositive ? 'rgba(34, 197, 94, 0.2)' : 'rgba(239, 68, 68, 0.2)',
      bottomColor: 'transparent',
      crosshairMarkerVisible: showTooltip,
      priceLineVisible: false,
      lastValueVisible: false,
    });

    series.setData(chartData as AreaData<Time>[]);
    chart.timeScale().fitContent();

    return () => {
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
      }
    };
  }, [chartData, width, height, isPositive, showTooltip]);

  if (chartData.length === 0) {
    return (
      <div
        className={`flex items-center justify-center text-muted-foreground text-xs ${className}`}
        style={{ width, height }}
      >
        -
      </div>
    );
  }

  return <div ref={chartContainerRef} className={className} />;
}
