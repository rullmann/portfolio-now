import { useMemo } from 'react';
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip,
  Legend,
} from 'recharts';

interface Security {
  uuid: string;
  name: string;
  currency: string;
  latest?: { date?: string | null; value?: number | null } | null;
}

interface AssetAllocationChartProps {
  securities: Security[];
  groupBy?: 'currency' | 'name';
}

const COLORS = [
  'hsl(221, 83%, 53%)',  // Blue
  'hsl(142, 76%, 36%)',  // Green
  'hsl(262, 83%, 58%)',  // Purple
  'hsl(25, 95%, 53%)',   // Orange
  'hsl(0, 84%, 60%)',    // Red
  'hsl(174, 72%, 40%)',  // Teal
  'hsl(47, 96%, 53%)',   // Yellow
  'hsl(336, 80%, 58%)',  // Pink
  'hsl(199, 89%, 48%)',  // Cyan
  'hsl(45, 93%, 47%)',   // Amber
];

export function AssetAllocationChart({ securities, groupBy = 'currency' }: AssetAllocationChartProps) {
  const chartData = useMemo(() => {
    if (!securities || securities.length === 0) return [];

    if (groupBy === 'currency') {
      // Group by currency
      const currencyGroups = new Map<string, number>();

      for (const security of securities) {
        const currency = security.currency || 'Unbekannt';
        const count = currencyGroups.get(currency) || 0;
        currencyGroups.set(currency, count + 1);
      }

      return Array.from(currencyGroups.entries())
        .map(([name, value]) => ({ name, value }))
        .sort((a, b) => b.value - a.value);
    } else {
      // Show top securities by count (placeholder for actual value calculation)
      return securities.slice(0, 10).map((s) => ({
        name: s.name?.slice(0, 20) || 'Unbekannt',
        value: 1,
      }));
    }
  }, [securities, groupBy]);

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        Keine Daten verfügbar
      </div>
    );
  }

  const total = chartData.reduce((sum, item) => sum + item.value, 0);

  return (
    <div className="space-y-2">
      <ResponsiveContainer width="100%" height={280}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            innerRadius={60}
            outerRadius={100}
            paddingAngle={2}
            dataKey="value"
            label={({ name, percent }) =>
              (percent ?? 0) > 0.05 ? `${name} (${((percent ?? 0) * 100).toFixed(0)}%)` : ''
            }
            labelLine={false}
          >
            {chartData.map((_, index) => (
              <Cell
                key={`cell-${index}`}
                fill={COLORS[index % COLORS.length]}
                stroke="hsl(var(--background))"
                strokeWidth={2}
              />
            ))}
          </Pie>
          <Tooltip
            contentStyle={{
              backgroundColor: 'hsl(var(--card))',
              border: '1px solid hsl(var(--border))',
              borderRadius: '6px',
            }}
            formatter={(value, name) => [
              `${value ?? 0} (${(((value ?? 0) as number / total) * 100).toFixed(1)}%)`,
              name as string,
            ]}
          />
          <Legend
            layout="horizontal"
            align="center"
            verticalAlign="bottom"
            wrapperStyle={{ paddingTop: '20px' }}
          />
        </PieChart>
      </ResponsiveContainer>
    </div>
  );
}
