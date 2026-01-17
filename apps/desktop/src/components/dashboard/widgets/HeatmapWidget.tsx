/**
 * Heatmap Widget - Monthly returns visualization
 */

import { useMemo } from 'react';
import type { WidgetProps } from '../types';

interface MonthlyReturn {
  year: number;
  month: number;
  returnPercent: number;
}

interface HeatmapWidgetProps extends WidgetProps {
  monthlyReturns?: MonthlyReturn[];
}

const MONTHS = ['Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez'];

export function HeatmapWidget({ monthlyReturns = [] }: HeatmapWidgetProps) {
  // Group returns by year
  const returnsByYear = useMemo(() => {
    const grouped: Record<number, Record<number, number>> = {};

    for (const ret of monthlyReturns) {
      if (!grouped[ret.year]) {
        grouped[ret.year] = {};
      }
      grouped[ret.year][ret.month] = ret.returnPercent;
    }

    return grouped;
  }, [monthlyReturns]);

  const years = Object.keys(returnsByYear).map(Number).sort((a, b) => b - a);

  // Color scale for returns
  const getColor = (value: number | undefined) => {
    if (value === undefined) return 'bg-muted/30';

    if (value >= 5) return 'bg-green-600 text-white';
    if (value >= 2) return 'bg-green-500 text-white';
    if (value >= 0) return 'bg-green-400/50';
    if (value >= -2) return 'bg-red-400/50';
    if (value >= -5) return 'bg-red-500 text-white';
    return 'bg-red-600 text-white';
  };

  if (years.length === 0) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <p className="text-muted-foreground text-sm">
          Keine Renditedaten vorhanden
        </p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
        Monatsrenditen
      </div>
      <div className="flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead>
            <tr>
              <th className="text-left p-1 text-muted-foreground">Jahr</th>
              {MONTHS.map((month) => (
                <th key={month} className="text-center p-1 text-muted-foreground">
                  {month}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {years.map((year) => (
              <tr key={year}>
                <td className="p-1 font-medium">{year}</td>
                {MONTHS.map((_, monthIndex) => {
                  const value = returnsByYear[year]?.[monthIndex + 1];
                  return (
                    <td
                      key={monthIndex}
                      data-testid={`heatmap-${year}-${String(monthIndex + 1).padStart(2, '0')}`}
                      className={`p-1 text-center ${getColor(value)}`}
                      title={value !== undefined ? `${value.toFixed(1)}%` : 'Keine Daten'}
                    >
                      {value !== undefined ? `${value.toFixed(0)}` : '-'}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
