/**
 * Year Returns Widget - Annual returns table
 */

import type { WidgetProps } from '../types';

interface YearlyReturn {
  year: number;
  ttwror: number;
  irr: number;
  absoluteGain: number;
}

interface YearReturnsWidgetProps extends WidgetProps {
  yearlyReturns?: YearlyReturn[];
  currency?: string;
}

export function YearReturnsWidget({
  yearlyReturns = [],
  currency = 'EUR',
}: YearReturnsWidgetProps) {
  const formatPercent = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'percent',
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
      signDisplay: 'exceptZero',
    }).format(value / 100);
  };

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
      signDisplay: 'exceptZero',
    }).format(value);
  };

  const getColorClass = (value: number) => {
    if (value > 0) return 'text-green-600';
    if (value < 0) return 'text-red-600';
    return '';
  };

  // Sort by year descending
  const sortedReturns = [...yearlyReturns].sort((a, b) => b.year - a.year);

  if (sortedReturns.length === 0) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <p className="text-muted-foreground text-sm">
          Keine Jahresrenditen vorhanden
        </p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
        Jahresrenditen
      </div>
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-xs text-muted-foreground border-b">
              <th className="text-left py-1">Jahr</th>
              <th className="text-right py-1">TTWROR</th>
              <th className="text-right py-1">IRR</th>
              <th className="text-right py-1">Gewinn</th>
            </tr>
          </thead>
          <tbody>
            {sortedReturns.map((ret) => (
              <tr key={ret.year} className="border-b border-muted/50 last:border-0">
                <td className="py-1.5 font-medium">{ret.year}</td>
                <td className={`py-1.5 text-right ${getColorClass(ret.ttwror)}`}>
                  {formatPercent(ret.ttwror)}
                </td>
                <td className={`py-1.5 text-right ${getColorClass(ret.irr)}`}>
                  {formatPercent(ret.irr)}
                </td>
                <td className={`py-1.5 text-right ${getColorClass(ret.absoluteGain)}`}>
                  {formatCurrency(ret.absoluteGain)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
