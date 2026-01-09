import { useMemo } from 'react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts';

interface Security {
  name: string;
  currencyCode: string;
  prices?: { price: Array<{ "@t": string; "@v": number }> } | null;
  latest?: { "@t"?: string | null; "@v"?: number | null } | null;
}

interface PortfolioValueChartProps {
  securities: Security[];
  baseCurrency: string;
}

// PP stores prices as value * 10^8
const convertPrice = (value: number): number => value / 100000000;

export function PortfolioValueChart({ securities, baseCurrency }: PortfolioValueChartProps) {
  const chartData = useMemo(() => {
    if (!securities || securities.length === 0) return [];

    // Collect all prices from all securities
    const pricesByDate = new Map<string, { total: number; count: number }>();

    for (const security of securities) {
      const prices = security.prices?.price || [];
      for (const price of prices) {
        const existing = pricesByDate.get(price["@t"]) || { total: 0, count: 0 };
        pricesByDate.set(price["@t"], {
          total: existing.total + convertPrice(price["@v"]),
          count: existing.count + 1,
        });
      }
    }

    // Convert to chart data and sort by date
    return Array.from(pricesByDate.entries())
      .map(([date, { total, count }]) => ({
        date,
        value: total,
        avgValue: total / count,
        count,
      }))
      .sort((a, b) => a.date.localeCompare(b.date))
      .slice(-365); // Last 365 data points
  }, [securities]);

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        Keine historischen Daten verfügbar
      </div>
    );
  }

  const latestValue = chartData[chartData.length - 1]?.value || 0;
  const firstValue = chartData[0]?.value || 0;
  const change = firstValue > 0 ? ((latestValue - firstValue) / firstValue) * 100 : 0;
  const isPositive = change >= 0;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <div className="text-sm text-muted-foreground">Portfolio-Entwicklung</div>
          <div className="text-2xl font-semibold">
            {latestValue.toLocaleString('de-DE', { maximumFractionDigits: 0 })} {baseCurrency}
          </div>
        </div>
        <div className={`text-right ${isPositive ? 'text-green-500' : 'text-red-500'}`}>
          <div className="text-lg font-semibold">
            {isPositive ? '+' : ''}{change.toFixed(2)}%
          </div>
          <div className="text-sm">
            {isPositive ? '+' : ''}{(latestValue - firstValue).toLocaleString('de-DE', { maximumFractionDigits: 0 })} {baseCurrency}
          </div>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={300}>
        <AreaChart data={chartData} margin={{ top: 10, right: 20, left: 10, bottom: 5 }}>
          <defs>
            <linearGradient id="colorValue" x1="0" y1="0" x2="0" y2="1">
              <stop
                offset="5%"
                stopColor={isPositive ? 'hsl(142, 76%, 36%)' : 'hsl(0, 84%, 60%)'}
                stopOpacity={0.3}
              />
              <stop
                offset="95%"
                stopColor={isPositive ? 'hsl(142, 76%, 36%)' : 'hsl(0, 84%, 60%)'}
                stopOpacity={0}
              />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
          <XAxis
            dataKey="date"
            tick={{ fontSize: 11 }}
            tickFormatter={(value) => {
              const date = new Date(value);
              return `${date.getMonth() + 1}/${date.getFullYear().toString().slice(2)}`;
            }}
            interval="preserveStartEnd"
            className="text-muted-foreground"
          />
          <YAxis
            tick={{ fontSize: 11 }}
            tickFormatter={(value) => `${(value / 1000).toFixed(0)}k`}
            className="text-muted-foreground"
          />
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '6px',
            }}
            labelStyle={{ color: 'hsl(var(--foreground))' }}
            formatter={(value) => [
              `${(value as number ?? 0).toLocaleString('de-DE', { maximumFractionDigits: 2 })} ${baseCurrency}`,
              'Wert',
            ]}
            labelFormatter={(label) => new Date(label).toLocaleDateString('de-DE')}
          />
          <Area
            type="monotone"
            dataKey="value"
            stroke={isPositive ? 'hsl(142, 76%, 36%)' : 'hsl(0, 84%, 60%)'}
            strokeWidth={2}
            fillOpacity={1}
            fill="url(#colorValue)"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
