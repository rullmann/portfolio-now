/**
 * Chart Widget - Portfolio performance chart
 */

import { useMemo } from 'react';
import {
  AreaChart,
  Area,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';
import type { WidgetProps } from '../types';

interface ChartWidgetProps extends WidgetProps {
  portfolioHistory?: Array<{ date: string; value: number }>;
  currency?: string;
}

export function ChartWidget({
  portfolioHistory = [],
  currency = 'EUR',
}: ChartWidgetProps) {
  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  const chartData = useMemo(() => {
    return portfolioHistory.map((d) => ({
      date: d.date,
      value: d.value,
      label: new Date(d.date).toLocaleDateString('de-DE', {
        day: '2-digit',
        month: '2-digit',
      }),
    }));
  }, [portfolioHistory]);

  const minValue = useMemo(() => {
    if (chartData.length === 0) return 0;
    return Math.min(...chartData.map((d) => d.value)) * 0.95;
  }, [chartData]);

  const maxValue = useMemo(() => {
    if (chartData.length === 0) return 100;
    return Math.max(...chartData.map((d) => d.value)) * 1.05;
  }, [chartData]);

  if (chartData.length === 0) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <p className="text-muted-foreground text-sm">Keine Daten vorhanden</p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-2">
        Portfolio-Entwicklung
      </div>
      <div className="flex-1 min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={chartData} margin={{ top: 5, right: 5, left: 5, bottom: 5 }}>
            <defs>
              <linearGradient id="chartGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#10b981" stopOpacity={0.3} />
                <stop offset="100%" stopColor="#10b981" stopOpacity={0} />
              </linearGradient>
            </defs>
            <XAxis
              dataKey="label"
              tick={{ fontSize: 10 }}
              tickLine={false}
              axisLine={false}
              interval="preserveStartEnd"
            />
            <YAxis
              domain={[minValue, maxValue]}
              tick={{ fontSize: 10 }}
              tickLine={false}
              axisLine={false}
              tickFormatter={(v) => `${Math.round(v / 1000)}k`}
              width={35}
            />
            <Tooltip
              formatter={(value) => [formatCurrency(value as number), 'Wert']}
              labelFormatter={(label) => `Datum: ${label}`}
              contentStyle={{
                backgroundColor: 'hsl(var(--background))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '6px',
              }}
            />
            <Area
              type="monotone"
              dataKey="value"
              stroke="#10b981"
              strokeWidth={2}
              fill="url(#chartGradient)"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
