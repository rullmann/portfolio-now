/**
 * DividendYearChart - Multi-year bar chart showing dividend income progression.
 */

import { useMemo } from 'react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts';
import type { DividendReport } from '../../lib/types';

interface Props {
  yearReports: Map<number, DividendReport>;
  selectedYear: number;
  currency: string;
  isLoading: boolean;
}

const formatCurrency = (amount: number, currency: string) =>
  `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;

export function DividendYearChart({ yearReports, selectedYear, currency, isLoading }: Props) {
  const chartData = useMemo(() => {
    const years = Array.from(yearReports.keys()).sort();
    return years.map((year) => {
      const report = yearReports.get(year)!;
      return {
        year: String(year),
        netto: report.totalNet,
        brutto: report.totalGross,
        steuern: report.totalTaxes,
        isSelected: year === selectedYear,
      };
    });
  }, [yearReports, selectedYear]);

  if (isLoading) {
    return (
      <div className="bg-card rounded-xl border border-border p-5">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
          Dividendenentwicklung
        </h3>
        <div className="h-56 flex items-center justify-center text-muted-foreground animate-pulse">
          Lade Daten...
        </div>
      </div>
    );
  }

  if (chartData.length === 0) {
    return (
      <div className="bg-card rounded-xl border border-border p-5">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
          Dividendenentwicklung
        </h3>
        <div className="h-56 flex items-center justify-center text-muted-foreground">
          Keine Daten vorhanden
        </div>
      </div>
    );
  }

  return (
    <div className="bg-card rounded-xl border border-border p-5">
      <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
        Dividendenentwicklung
      </h3>
      <div className="h-56">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={chartData} barCategoryGap="20%">
            <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" vertical={false} />
            <XAxis
              dataKey="year"
              tick={{ fontSize: 12 }}
              axisLine={false}
              tickLine={false}
            />
            <YAxis
              tick={{ fontSize: 11 }}
              tickFormatter={(v) => `${(v / 1).toFixed(0)}`}
              axisLine={false}
              tickLine={false}
              width={60}
            />
            <Tooltip
              content={({ active, payload, label }) => {
                if (!active || !payload?.length) return null;
                const data = payload[0]?.payload;
                return (
                  <div className="bg-popover border border-border rounded-lg shadow-lg p-3 text-sm">
                    <div className="font-semibold mb-1">{label}</div>
                    <div className="text-green-600">Netto: {formatCurrency(data.netto, currency)}</div>
                    <div className="text-muted-foreground">Brutto: {formatCurrency(data.brutto, currency)}</div>
                    <div className="text-red-500">Steuern: {formatCurrency(data.steuern, currency)}</div>
                  </div>
                );
              }}
            />
            <Bar dataKey="netto" radius={[6, 6, 0, 0]} maxBarSize={48}>
              {chartData.map((entry, idx) => (
                <Cell
                  key={idx}
                  fill={entry.isSelected ? '#10b981' : '#10b98166'}
                  className="transition-colors"
                />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
