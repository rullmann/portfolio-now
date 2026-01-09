/**
 * Portfolio value chart using TradingView Lightweight Charts.
 * Used for displaying overall portfolio performance.
 */

import { useEffect, useRef, useMemo, useState } from 'react';
import {
  createChart,
  ColorType,
  AreaSeries,
  type IChartApi,
  type AreaData,
  type Time,
} from 'lightweight-charts';

interface TradingViewPortfolioChartProps {
  data: Array<{ date: string; value: number }>;
  height?: number;
  currency?: string;
}

type TimePeriod = '1M' | '3M' | '6M' | 'YTD' | '1Y' | '2Y' | 'MAX';

export function TradingViewPortfolioChart({
  data,
  height = 300,
  currency = 'EUR',
}: TradingViewPortfolioChartProps) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const [selectedPeriod, setSelectedPeriod] = useState<TimePeriod>('1Y');

  // Process and sort data
  const allData = useMemo(() => {
    if (!data || data.length === 0) return [];
    return data
      .map((d) => ({
        time: d.date as Time,
        value: d.value,
      }))
      .sort((a, b) => (a.time as string).localeCompare(b.time as string));
  }, [data]);

  // Filter data based on selected period
  const chartData = useMemo(() => {
    if (allData.length === 0) return [];
    if (selectedPeriod === 'MAX') return allData;

    const now = new Date();
    let startDate: Date;

    switch (selectedPeriod) {
      case '1M':
        startDate = new Date(now.getFullYear(), now.getMonth() - 1, now.getDate());
        break;
      case '3M':
        startDate = new Date(now.getFullYear(), now.getMonth() - 3, now.getDate());
        break;
      case '6M':
        startDate = new Date(now.getFullYear(), now.getMonth() - 6, now.getDate());
        break;
      case 'YTD':
        startDate = new Date(now.getFullYear(), 0, 1);
        break;
      case '1Y':
        startDate = new Date(now.getFullYear() - 1, now.getMonth(), now.getDate());
        break;
      case '2Y':
        startDate = new Date(now.getFullYear() - 2, now.getMonth(), now.getDate());
        break;
      default:
        return allData;
    }

    const startDateStr = startDate.toISOString().split('T')[0];
    return allData.filter((d) => (d.time as string) >= startDateStr);
  }, [allData, selectedPeriod]);

  // Calculate statistics
  const stats = useMemo(() => {
    if (chartData.length < 2) return { change: 0, changePercent: 0, isPositive: true };
    const first = chartData[0]?.value || 0;
    const last = chartData[chartData.length - 1]?.value || 0;
    const change = last - first;
    const changePercent = first > 0 ? (change / first) * 100 : 0;
    return { change, changePercent, isPositive: change >= 0, first, last };
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

    // Create area series
    const color = stats.isPositive ? '#22c55e' : '#ef4444';
    const series = chart.addSeries(AreaSeries, {
      lineColor: color,
      lineWidth: 2,
      topColor: stats.isPositive ? 'rgba(34, 197, 94, 0.3)' : 'rgba(239, 68, 68, 0.3)',
      bottomColor: stats.isPositive ? 'rgba(34, 197, 94, 0.05)' : 'rgba(239, 68, 68, 0.05)',
      crosshairMarkerVisible: true,
      priceLineVisible: false,
      lastValueVisible: true,
    });

    series.setData(chartData as AreaData<Time>[]);
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
  }, [chartData, height, stats.isPositive]);

  const periods: TimePeriod[] = ['1M', '3M', '6M', 'YTD', '1Y', '2Y', 'MAX'];

  if (allData.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-muted-foreground"
        style={{ height }}
      >
        Keine historischen Daten verfügbar
      </div>
    );
  }

  const formatNumber = (num: number) =>
    num.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

  return (
    <div className="space-y-4">
      {/* Header with stats */}
      <div className="flex items-center justify-between">
        <div>
          <div className="text-sm text-muted-foreground">Gesamtportfolio</div>
          <div className="text-2xl font-bold">
            {formatNumber(stats.last || 0)} {currency}
          </div>
        </div>
        <div className={`text-right ${stats.isPositive ? 'text-green-600' : 'text-red-600'}`}>
          <div className="text-lg font-semibold">
            {stats.isPositive ? '+' : ''}
            {stats.changePercent.toFixed(2)}%
          </div>
          <div className="text-sm">
            {stats.isPositive ? '+' : ''}
            {formatNumber(stats.change)} {currency}
          </div>
        </div>
      </div>

      {/* Period selector */}
      <div className="flex gap-1">
        {periods.map((period) => (
          <button
            key={period}
            onClick={() => setSelectedPeriod(period)}
            className={`px-3 py-1 text-xs font-medium rounded transition-colors ${
              selectedPeriod === period
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted hover:bg-muted/80 text-muted-foreground'
            }`}
          >
            {period}
          </button>
        ))}
      </div>

      {/* Chart container */}
      <div ref={chartContainerRef} />
    </div>
  );
}
