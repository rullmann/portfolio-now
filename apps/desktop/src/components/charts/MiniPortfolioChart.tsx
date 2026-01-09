import { useMemo } from 'react';
import {
  AreaChart,
  Area,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts';

interface PortfolioValuePoint {
  date: string;
  value: number;
}

interface MiniPortfolioChartProps {
  data: PortfolioValuePoint[];
  height?: number;
  showAxis?: boolean;
}

export function MiniPortfolioChart({ data, height = 80, showAxis = false }: MiniPortfolioChartProps) {
  const chartData = useMemo(() => {
    if (!data || data.length === 0) return [];
    return data;
  }, [data]);

  if (chartData.length < 2) {
    return (
      <div className="flex items-center justify-center text-muted-foreground text-sm" style={{ height }}>
        Keine Verlaufsdaten
      </div>
    );
  }

  const firstValue = chartData[0]?.value || 0;
  const lastValue = chartData[chartData.length - 1]?.value || 0;
  const change = firstValue > 0 ? ((lastValue - firstValue) / firstValue) * 100 : 0;
  const isPositive = change >= 0;

  const strokeColor = isPositive ? '#22c55e' : '#ef4444';

  // Use unique gradient ID to avoid conflicts with multiple charts
  const gradientId = showAxis ? 'dashboardChartGradient' : 'miniChartGradient';

  return (
    <div className="w-full" style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart
          data={chartData}
          margin={showAxis
            ? { top: 10, right: 10, left: 0, bottom: 0 }
            : { top: 2, right: 2, left: 2, bottom: 2 }
          }
        >
          <defs>
            <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={strokeColor} stopOpacity={0.2} />
              <stop offset="95%" stopColor={strokeColor} stopOpacity={0} />
            </linearGradient>
          </defs>
          {showAxis && (
            <XAxis
              dataKey="date"
              axisLine={false}
              tickLine={false}
              tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
              tickFormatter={(value) => {
                const date = new Date(value);
                return date.toLocaleDateString('de-DE', { month: 'short' });
              }}
              interval="preserveStartEnd"
              minTickGap={40}
            />
          )}
          {showAxis && (
            <YAxis
              axisLine={false}
              tickLine={false}
              tick={{ fontSize: 10, fill: 'hsl(var(--muted-foreground))' }}
              tickFormatter={(value) => `${(value / 1000).toFixed(0)}k`}
              width={40}
              domain={['dataMin - 1000', 'dataMax + 1000']}
            />
          )}
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '6px',
              fontSize: '12px',
              padding: '4px 8px',
            }}
            labelStyle={{ color: 'hsl(var(--foreground))', fontSize: '10px' }}
            formatter={(value) => [
              `${(value as number).toLocaleString('de-DE', { minimumFractionDigits: 0, maximumFractionDigits: 0 })} EUR`,
              '',
            ]}
            labelFormatter={(label) => new Date(label).toLocaleDateString('de-DE')}
          />
          <Area
            type="monotone"
            dataKey="value"
            stroke={strokeColor}
            strokeWidth={showAxis ? 2 : 1.5}
            fill={`url(#${gradientId})`}
            dot={false}
            activeDot={{ r: showAxis ? 4 : 3, fill: strokeColor }}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
