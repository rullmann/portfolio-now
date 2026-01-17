/**
 * Holdings Table Widget - Shows top holdings in a table
 */

import type { WidgetProps } from '../types';

interface Holding {
  name: string;
  value: number;
  weight: number;
  gainLossPercent: number;
}

interface HoldingsTableWidgetProps extends WidgetProps {
  holdings?: Holding[];
  currency?: string;
}

export function HoldingsTableWidget({
  config,
  holdings = [],
  currency = 'EUR',
}: HoldingsTableWidgetProps) {
  const limit = (config.settings.limit as number) || 10;
  const displayedHoldings = holdings.slice(0, limit);

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
      signDisplay: 'exceptZero',
    }).format(value / 100);
  };

  return (
    <div className="h-full flex flex-col p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
        Bestände
      </div>
      <div className="flex-1 overflow-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-xs text-muted-foreground border-b">
              <th className="text-left py-1">Name</th>
              <th className="text-right py-1">Wert</th>
              <th className="text-right py-1">%</th>
              <th className="text-right py-1">G/V</th>
            </tr>
          </thead>
          <tbody>
            {displayedHoldings.map((holding, index) => (
              <tr key={index} className="border-b border-muted/50 last:border-0">
                <td className="py-1.5 truncate max-w-[120px]" title={holding.name}>
                  {holding.name}
                </td>
                <td className="py-1.5 text-right">{formatCurrency(holding.value)}</td>
                <td className="py-1.5 text-right text-muted-foreground">
                  {formatPercent(holding.weight)}
                </td>
                <td className={`py-1.5 text-right ${holding.gainLossPercent >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                  {formatPercent(holding.gainLossPercent)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {holdings.length === 0 && (
          <div className="text-center text-muted-foreground py-4">
            Keine Bestände vorhanden
          </div>
        )}
      </div>
    </div>
  );
}
