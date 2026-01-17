/**
 * Portfolio Value Widget - Shows current portfolio value with sparkline
 */

import { TrendingUp, TrendingDown } from 'lucide-react';
import type { WidgetProps } from '../types';

interface PortfolioValueWidgetProps extends WidgetProps {
  portfolioValue?: number;
  costBasis?: number;
  gainLoss?: number;
  gainLossPercent?: number;
  currency?: string;
}

export function PortfolioValueWidget({
  portfolioValue = 0,
  gainLoss = 0,
  gainLossPercent = 0,
  currency = 'EUR',
}: PortfolioValueWidgetProps) {
  const isPositive = gainLoss >= 0;

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value);
  };

  const formatPercent = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'percent',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      signDisplay: 'exceptZero',
    }).format(value / 100);
  };

  return (
    <div className="h-full flex flex-col justify-center p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-1">
        Depotwert
      </div>
      <div className="text-2xl font-bold">
        {formatCurrency(portfolioValue)}
      </div>
      <div className={`flex items-center gap-1 text-sm ${isPositive ? 'text-green-600' : 'text-red-600'}`}>
        {isPositive ? (
          <TrendingUp className="h-4 w-4" />
        ) : (
          <TrendingDown className="h-4 w-4" />
        )}
        <span>{formatCurrency(gainLoss)}</span>
        <span className="text-muted-foreground">({formatPercent(gainLossPercent)})</span>
      </div>
    </div>
  );
}
