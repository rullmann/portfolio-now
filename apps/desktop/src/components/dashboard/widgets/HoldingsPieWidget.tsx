/**
 * Holdings Pie Widget - Pie chart showing portfolio allocation
 */

import { useMemo } from 'react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Legend } from 'recharts';
import type { WidgetProps } from '../types';

interface Holding {
  name: string;
  value: number;
  weight: number;
  gainLossPercent: number;
}

interface HoldingsPieWidgetProps extends WidgetProps {
  holdings?: Holding[];
  currency?: string;
}

const COLORS = [
  '#22c55e', // green
  '#3b82f6', // blue
  '#f59e0b', // amber
  '#8b5cf6', // violet
  '#ec4899', // pink
  '#06b6d4', // cyan
  '#f97316', // orange
  '#6366f1', // indigo
  '#84cc16', // lime
  '#14b8a6', // teal
];

export function HoldingsPieWidget({
  config,
  holdings = [],
  currency = 'EUR',
}: HoldingsPieWidgetProps) {
  const limit = (config.settings.limit as number) || 10;

  const chartData = useMemo(() => {
    if (holdings.length === 0) return [];

    // Sort by value and take top N
    const sorted = [...holdings].sort((a, b) => b.value - a.value);
    const top = sorted.slice(0, limit);

    // If there are more, add "Sonstige"
    if (sorted.length > limit) {
      const otherValue = sorted.slice(limit).reduce((sum, h) => sum + h.value, 0);
      const otherWeight = sorted.slice(limit).reduce((sum, h) => sum + h.weight, 0);
      top.push({
        name: 'Sonstige',
        value: otherValue,
        weight: otherWeight,
        gainLossPercent: 0,
      });
    }

    return top.map((h, index) => ({
      name: h.name,
      value: h.value,
      weight: h.weight,
      color: COLORS[index % COLORS.length],
    }));
  }, [holdings, limit]);

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(value);
  };

  const formatPercent = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'percent',
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }).format(value / 100);
  };

  if (chartData.length === 0) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <p className="text-muted-foreground text-sm">Keine Bestände vorhanden</p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-2">
        Allokation
      </div>
      <div className="flex-1 min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie
              data={chartData}
              cx="50%"
              cy="50%"
              innerRadius="40%"
              outerRadius="70%"
              paddingAngle={2}
              dataKey="value"
            >
              {chartData.map((entry, index) => (
                <Cell key={`cell-${index}`} fill={entry.color} />
              ))}
            </Pie>
            <Tooltip
              formatter={(value, name) => [formatCurrency(value as number), name]}
              contentStyle={{
                backgroundColor: 'hsl(var(--background))',
                border: '1px solid hsl(var(--border))',
                borderRadius: '6px',
              }}
            />
            <Legend
              layout="vertical"
              align="right"
              verticalAlign="middle"
              formatter={(value) => {
                const data = chartData.find((d) => d.name === value);
                return (
                  <span className="text-xs">
                    {value} ({data ? formatPercent(data.weight) : ''})
                  </span>
                );
              }}
              wrapperStyle={{ fontSize: '10px', paddingLeft: '10px' }}
            />
          </PieChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
