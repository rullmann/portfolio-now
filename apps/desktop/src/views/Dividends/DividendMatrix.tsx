/**
 * DividendMatrix - Security x Year heatmap table.
 */

import { useMemo } from 'react';
import type { DividendReport } from '../../lib/types';
import { SecurityLogo } from '../../components/common/SecurityLogo';
import type { CachedLogo } from '../../lib/hooks';

interface Props {
  yearReports: Map<number, DividendReport>;
  logos: Map<number, CachedLogo>;
  currency: string;
  isLoading: boolean;
}

const formatAmount = (amount: number) =>
  amount > 0 ? amount.toLocaleString('de-DE', { minimumFractionDigits: 0, maximumFractionDigits: 0 }) : '';

export function DividendMatrix({ yearReports, logos, currency, isLoading }: Props) {
  const years = useMemo(() => Array.from(yearReports.keys()).sort(), [yearReports]);

  // Collect all securities across all years
  const { securityRows, maxAmount } = useMemo(() => {
    const secMap = new Map<number, { name: string; isin?: string; byYear: Map<number, number> }>();
    let max = 0;

    for (const [year, report] of yearReports) {
      for (const sec of report.bySecurity) {
        let entry = secMap.get(sec.securityId);
        if (!entry) {
          entry = { name: sec.securityName, isin: sec.securityIsin, byYear: new Map() };
          secMap.set(sec.securityId, entry);
        }
        entry.byYear.set(year, sec.totalNet);
        if (sec.totalNet > max) max = sec.totalNet;
      }
    }

    // Sort by total descending
    const rows = Array.from(secMap.entries())
      .map(([id, data]) => ({
        securityId: id,
        ...data,
        total: Array.from(data.byYear.values()).reduce((s, v) => s + v, 0),
      }))
      .sort((a, b) => b.total - a.total);

    return { securityRows: rows, maxAmount: max };
  }, [yearReports]);

  // Year totals
  const yearTotals = useMemo(() => {
    const totals = new Map<number, number>();
    for (const year of years) {
      totals.set(year, yearReports.get(year)?.totalNet || 0);
    }
    return totals;
  }, [yearReports, years]);

  if (isLoading) {
    return (
      <div className="bg-card rounded-xl border border-border p-5">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
          Dividendenmatrix
        </h3>
        <div className="h-32 flex items-center justify-center text-muted-foreground animate-pulse">
          Lade Daten...
        </div>
      </div>
    );
  }

  if (securityRows.length === 0) return null;

  const getHeatColor = (amount: number): string => {
    if (amount <= 0 || maxAmount <= 0) return '';
    const intensity = Math.min(amount / maxAmount, 1);
    // Green shades from very light to strong
    const alpha = Math.round(intensity * 40 + 8);
    return `rgba(16, 185, 129, ${alpha / 100})`;
  };

  return (
    <div className="bg-card rounded-xl border border-border overflow-hidden">
      <div className="p-5 pb-3">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground">
          Dividendenmatrix
        </h3>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/30">
              <th className="text-left py-2.5 px-4 font-medium text-muted-foreground sticky left-0 bg-muted/30 z-10 min-w-[180px]">
                Wertpapier
              </th>
              {years.map((year) => (
                <th key={year} className="text-right py-2.5 px-3 font-medium text-muted-foreground min-w-[80px]">
                  {year}
                </th>
              ))}
              <th className="text-right py-2.5 px-4 font-medium text-muted-foreground min-w-[90px]">
                Gesamt
              </th>
            </tr>
          </thead>
          <tbody>
            {securityRows.map((row) => (
              <tr key={row.securityId} className="border-b border-border/50 hover:bg-muted/20 transition-colors">
                <td className="py-2 px-4 sticky left-0 bg-card z-10">
                  <div className="flex items-center gap-2">
                    <SecurityLogo securityId={row.securityId} logos={logos} size={22} />
                    <span className="font-medium truncate max-w-[140px]">{row.name}</span>
                  </div>
                </td>
                {years.map((year) => {
                  const amount = row.byYear.get(year) || 0;
                  return (
                    <td
                      key={year}
                      className="py-2 px-3 text-right tabular-nums"
                      style={{ backgroundColor: getHeatColor(amount) }}
                    >
                      <span className={amount > 0 ? 'text-foreground' : 'text-muted-foreground/30'}>
                        {amount > 0 ? formatAmount(amount) : '-'}
                      </span>
                    </td>
                  );
                })}
                <td className="py-2 px-4 text-right font-semibold tabular-nums text-green-600">
                  {formatAmount(row.total)}
                </td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr className="bg-muted/40 font-semibold">
              <td className="py-2.5 px-4 sticky left-0 bg-muted/40 z-10">Gesamt</td>
              {years.map((year) => (
                <td key={year} className="py-2.5 px-3 text-right tabular-nums text-green-600">
                  {formatAmount(yearTotals.get(year) || 0)}
                </td>
              ))}
              <td className="py-2.5 px-4 text-right tabular-nums text-green-600">
                {formatAmount(Array.from(yearTotals.values()).reduce((s, v) => s + v, 0))}
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}
