/**
 * Benchmark comparison chart using TradingView Lightweight Charts.
 * Shows portfolio vs benchmark performance as normalized percentage returns.
 */

import { useEffect, useRef } from 'react';
import {
  createChart,
  ColorType,
  LineSeries,
  type IChartApi,
  type LineData,
  type Time,
} from 'lightweight-charts';

interface BenchmarkDataPoint {
  date: string;
  portfolioReturn: number;
  benchmarkReturn: number;
}

interface TradingViewBenchmarkChartProps {
  data: BenchmarkDataPoint[];
  height?: number;
  portfolioName?: string;
  benchmarkName?: string;
}

export function TradingViewBenchmarkChart({
  data,
  height = 320,
  portfolioName = 'Portfolio',
  benchmarkName = 'Benchmark',
}: TradingViewBenchmarkChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);

  useEffect(() => {
    if (!chartContainerRef.current || data.length === 0) {
      return;
    }

    // Clean up existing chart
    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
    }

    // Detect dark mode
    const isDark = document.documentElement.classList.contains('dark');

    // Create chart
    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: isDark ? '#a1a1aa' : '#71717a',
        attributionLogo: false,
      },
      grid: {
        vertLines: { color: isDark ? '#27272a' : '#e4e4e7' },
        horzLines: { color: isDark ? '#27272a' : '#e4e4e7' },
      },
      width: chartContainerRef.current.clientWidth,
      height,
      rightPriceScale: {
        borderColor: isDark ? '#27272a' : '#e4e4e7',
      },
      timeScale: {
        borderColor: isDark ? '#27272a' : '#e4e4e7',
        timeVisible: false,
      },
      crosshair: {
        vertLine: {
          labelBackgroundColor: isDark ? '#27272a' : '#e4e4e7',
        },
        horzLine: {
          labelBackgroundColor: isDark ? '#27272a' : '#e4e4e7',
        },
      },
    });

    chartRef.current = chart;

    // Prepare data for both series
    const sortedData = [...data].sort((a, b) => a.date.localeCompare(b.date));

    const portfolioData: LineData<Time>[] = sortedData.map((d) => ({
      time: d.date as Time,
      value: d.portfolioReturn,
    }));

    const benchmarkData: LineData<Time>[] = sortedData.map((d) => ({
      time: d.date as Time,
      value: d.benchmarkReturn,
    }));

    // Create portfolio series (blue)
    const portfolioSeries = chart.addSeries(LineSeries, {
      color: '#3b82f6',
      lineWidth: 2,
      crosshairMarkerVisible: true,
      priceLineVisible: false,
      lastValueVisible: true,
      title: portfolioName,
    });
    portfolioSeries.setData(portfolioData);

    // Create benchmark series (green)
    const benchmarkSeries = chart.addSeries(LineSeries, {
      color: '#10b981',
      lineWidth: 2,
      crosshairMarkerVisible: true,
      priceLineVisible: false,
      lastValueVisible: true,
      title: benchmarkName,
    });
    benchmarkSeries.setData(benchmarkData);

    // Format price scale as percentage
    chart.priceScale('right').applyOptions({
      ticksVisible: true,
    });

    chart.timeScale().fitContent();

    // Handle resize
    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
      }
    };
  }, [data, height, portfolioName, benchmarkName]);

  if (data.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-muted-foreground"
        style={{ height }}
      >
        Keine Vergleichsdaten verfügbar
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {/* Legend */}
      <div className="flex gap-4 text-sm">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-blue-500" />
          <span>{portfolioName}</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-emerald-500" />
          <span>{benchmarkName}</span>
        </div>
      </div>

      {/* Chart container */}
      <div ref={chartContainerRef} />
    </div>
  );
}
