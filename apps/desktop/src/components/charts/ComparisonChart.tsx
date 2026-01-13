/**
 * Comparison Chart Component
 * Compare performance of multiple securities on a single chart
 * Uses normalized percentage performance from a starting point
 */

import { useEffect, useRef, useState } from 'react';
import {
  createChart,
  ColorType,
  CrosshairMode,
  LineSeries,
} from 'lightweight-charts';
import type {
  IChartApi,
  ISeriesApi,
  LineData,
  Time,
} from 'lightweight-charts';

// ============================================================================
// Types
// ============================================================================

export interface ComparisonSecurity {
  id: number;
  name: string;
  ticker?: string;
  color: string;
  data: { date: string; close: number }[];
}

export interface ComparisonChartProps {
  securities: ComparisonSecurity[];
  height?: number;
  theme?: 'light' | 'dark';
  normalize?: boolean; // If true, show percentage performance; if false, show raw prices
}

// Predefined colors for comparison lines
export const COMPARISON_COLORS = [
  '#2563eb', // Blue
  '#dc2626', // Red
  '#16a34a', // Green
  '#ca8a04', // Yellow
  '#9333ea', // Purple
  '#ea580c', // Orange
  '#0891b2', // Cyan
  '#be185d', // Pink
];

// ============================================================================
// Helper Functions
// ============================================================================

function normalizeData(
  data: { date: string; close: number }[],
  normalize: boolean
): LineData<Time>[] {
  if (data.length === 0) return [];

  const basePrice = data[0].close;

  return data.map(d => ({
    time: d.date as Time,
    value: normalize
      ? ((d.close - basePrice) / basePrice) * 100 // Percentage change
      : d.close,
  }));
}

// ============================================================================
// Main Component
// ============================================================================

export function ComparisonChart({
  securities,
  height = 400,
  theme = 'dark',
  normalize = true,
}: ComparisonChartProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<Map<number, ISeriesApi<'Line'>>>(new Map());
  const [hoveredSecurity, setHoveredSecurity] = useState<number | null>(null);

  // Theme colors
  const isDark = theme === 'dark';
  const backgroundColor = isDark ? '#1e1e2e' : '#ffffff';
  const textColor = isDark ? '#9ca3af' : '#374151';
  const gridColor = isDark ? '#2d2d3d' : '#e5e7eb';

  // Create chart
  useEffect(() => {
    if (!containerRef.current) return;

    // Create chart instance
    const chart = createChart(containerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: backgroundColor },
        textColor,
      },
      width: containerRef.current.clientWidth,
      height,
      crosshair: {
        mode: CrosshairMode.Normal,
        vertLine: {
          width: 1,
          color: isDark ? '#4b5563' : '#9ca3af',
          style: 2,
        },
        horzLine: {
          width: 1,
          color: isDark ? '#4b5563' : '#9ca3af',
          style: 2,
        },
      },
      grid: {
        vertLines: { color: gridColor },
        horzLines: { color: gridColor },
      },
      rightPriceScale: {
        borderColor: gridColor,
        scaleMargins: { top: 0.1, bottom: 0.1 },
      },
      timeScale: {
        borderColor: gridColor,
        timeVisible: true,
        secondsVisible: false,
      },
    });

    chartRef.current = chart;

    // Handle resize
    const handleResize = () => {
      if (containerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: containerRef.current.clientWidth,
        });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
      chartRef.current = null;
      seriesRef.current.clear();
    };
  }, [height, theme, isDark, backgroundColor, textColor, gridColor]);

  // Update series when securities change
  useEffect(() => {
    const chart = chartRef.current;
    if (!chart) return;

    // Remove old series that are no longer in securities
    const currentIds = new Set(securities.map(s => s.id));
    seriesRef.current.forEach((series, id) => {
      if (!currentIds.has(id)) {
        chart.removeSeries(series);
        seriesRef.current.delete(id);
      }
    });

    // Add or update series for each security
    securities.forEach(security => {
      let series = seriesRef.current.get(security.id);

      if (!series) {
        // Create new series
        series = chart.addSeries(LineSeries, {
          color: security.color,
          lineWidth: 2,
          priceFormat: {
            type: 'custom',
            formatter: (price: number) => normalize ? `${price.toFixed(1)}%` : price.toFixed(2),
          },
          crosshairMarkerVisible: true,
          crosshairMarkerRadius: 4,
          lastValueVisible: true,
          priceLineVisible: false,
        });
        seriesRef.current.set(security.id, series);
      } else {
        // Update series color if changed
        series.applyOptions({ color: security.color });
      }

      // Set data
      const lineData = normalizeData(security.data, normalize);
      series.setData(lineData);
    });

    // Fit content
    chart.timeScale().fitContent();
  }, [securities, normalize]);

  // Update series opacity on hover
  useEffect(() => {
    seriesRef.current.forEach((series, id) => {
      const isHovered = hoveredSecurity === null || hoveredSecurity === id;
      const security = securities.find(s => s.id === id);
      if (security) {
        series.applyOptions({
          lineWidth: isHovered ? 2 : 1,
          color: isHovered ? security.color : `${security.color}80`, // Add transparency
        });
      }
    });
  }, [hoveredSecurity, securities]);

  if (securities.length === 0) {
    return (
      <div
        className="flex items-center justify-center text-muted-foreground"
        style={{ height }}
      >
        Wählen Sie Wertpapiere zum Vergleichen aus
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {/* Chart */}
      <div ref={containerRef} className="w-full rounded-lg overflow-hidden" />

      {/* Legend */}
      <div className="flex flex-wrap gap-3 px-2">
        {securities.map(security => {
          const lastValue = security.data.length > 0
            ? security.data[security.data.length - 1].close
            : null;
          const firstValue = security.data.length > 0
            ? security.data[0].close
            : null;
          const percentChange = firstValue && lastValue
            ? ((lastValue - firstValue) / firstValue) * 100
            : null;

          return (
            <div
              key={security.id}
              className={`flex items-center gap-2 px-2 py-1 rounded cursor-pointer transition-opacity ${
                hoveredSecurity !== null && hoveredSecurity !== security.id
                  ? 'opacity-50'
                  : ''
              }`}
              onMouseEnter={() => setHoveredSecurity(security.id)}
              onMouseLeave={() => setHoveredSecurity(null)}
            >
              <div
                className="w-3 h-3 rounded-full"
                style={{ backgroundColor: security.color }}
              />
              <span className="text-sm font-medium">
                {security.ticker || security.name}
              </span>
              {percentChange !== null && (
                <span
                  className={`text-xs font-mono ${
                    percentChange >= 0 ? 'text-green-500' : 'text-red-500'
                  }`}
                >
                  {percentChange >= 0 ? '+' : ''}{percentChange.toFixed(1)}%
                </span>
              )}
            </div>
          );
        })}
      </div>

      {/* Y-Axis Label */}
      <div className="text-xs text-muted-foreground text-center">
        {normalize ? 'Performance (%)' : 'Kurs'}
      </div>
    </div>
  );
}

export default ComparisonChart;
