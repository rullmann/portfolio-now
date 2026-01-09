import { useMemo } from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  ReferenceLine,
} from 'recharts';

interface PriceEntry {
  date: string;  // NaiveDate serialized as string
  value: number;  // price * 10^8
}

interface SecurityPriceChartProps {
  prices: PriceEntry[];
  currency: string;
  name?: string;
}

// PP stores prices as value * 10^8 (like shares), convert to actual price
const convertPrice = (value: number): number => value / 100000000;

export function SecurityPriceChart({ prices, currency, name }: SecurityPriceChartProps) {
  const chartData = useMemo(() => {
    if (!prices || prices.length === 0) return [];

    return prices
      .map((p) => ({
        date: p.date,
        price: convertPrice(p.value),
      }))
      .sort((a, b) => a.date.localeCompare(b.date));
  }, [prices]);

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        Keine Kursdaten verfügbar
      </div>
    );
  }

  const minPrice = Math.min(...chartData.map((d) => d.price));
  const maxPrice = Math.max(...chartData.map((d) => d.price));
  const latestPrice = chartData[chartData.length - 1]?.price || 0;
  const firstPrice = chartData[0]?.price || 0;
  const priceChange = firstPrice > 0 ? ((latestPrice - firstPrice) / firstPrice) * 100 : 0;
  const isPositive = priceChange >= 0;

  return (
    <div className="space-y-2">
      {name && (
        <div className="flex items-center justify-between">
          <h3 className="font-medium">{name}</h3>
          <div className="text-right">
            <div className="text-lg font-semibold">
              {latestPrice.toFixed(2)} {currency}
            </div>
            <div className={`text-sm ${isPositive ? 'text-green-500' : 'text-red-500'}`}>
              {isPositive ? '+' : ''}{priceChange.toFixed(2)}%
            </div>
          </div>
        </div>
      )}
      <ResponsiveContainer width="100%" height={250}>
        <LineChart data={chartData} margin={{ top: 5, right: 20, left: 10, bottom: 5 }}>
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
            domain={[minPrice * 0.95, maxPrice * 1.05]}
            tick={{ fontSize: 11 }}
            tickFormatter={(value) => value.toFixed(0)}
            className="text-muted-foreground"
          />
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '6px',
            }}
            labelStyle={{ color: 'hsl(var(--foreground))' }}
            formatter={(value) => [`${(value as number ?? 0).toFixed(2)} ${currency}`, 'Kurs']}
            labelFormatter={(label) => new Date(label).toLocaleDateString('de-DE')}
          />
          <ReferenceLine y={firstPrice} stroke="hsl(var(--muted-foreground))" strokeDasharray="3 3" />
          <Line
            type="monotone"
            dataKey="price"
            stroke={isPositive ? 'hsl(142, 76%, 36%)' : 'hsl(0, 84%, 60%)'}
            strokeWidth={2}
            dot={false}
            activeDot={{ r: 4 }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
