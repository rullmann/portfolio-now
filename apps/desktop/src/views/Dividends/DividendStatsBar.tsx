/**
 * DividendStatsBar - 3 key metrics: Dividend Yield, YTD Dividends, FWD Annual.
 */

import { TrendingUp, Percent, Calendar } from 'lucide-react';

interface Props {
  dividendYield: number | null;
  ytdNet: number;
  fwdAnnual: number;
  currency: string;
  isLoading: boolean;
}

const formatCurrency = (amount: number, currency: string) =>
  `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;

export function DividendStatsBar({ dividendYield, ytdNet, fwdAnnual, currency, isLoading }: Props) {
  const stats = [
    {
      label: 'Dividendenrendite',
      value: dividendYield != null ? `${dividendYield.toFixed(2)}%` : '-',
      icon: Percent,
      color: 'text-emerald-600 dark:text-emerald-400',
      bgColor: 'bg-emerald-50 dark:bg-emerald-900/20',
      iconBg: 'bg-emerald-100 dark:bg-emerald-900/40',
    },
    {
      label: 'YTD Dividenden (netto)',
      value: formatCurrency(ytdNet, currency),
      icon: Calendar,
      color: 'text-blue-600 dark:text-blue-400',
      bgColor: 'bg-blue-50 dark:bg-blue-900/20',
      iconBg: 'bg-blue-100 dark:bg-blue-900/40',
    },
    {
      label: 'FWD Bruttodividende',
      value: formatCurrency(fwdAnnual, currency),
      icon: TrendingUp,
      color: 'text-violet-600 dark:text-violet-400',
      bgColor: 'bg-violet-50 dark:bg-violet-900/20',
      iconBg: 'bg-violet-100 dark:bg-violet-900/40',
    },
  ];

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
      {stats.map((stat) => {
        const Icon = stat.icon;
        return (
          <div
            key={stat.label}
            className={`${stat.bgColor} rounded-xl p-5 border border-border/50`}
          >
            <div className="flex items-center gap-3">
              <div className={`${stat.iconBg} p-2.5 rounded-lg`}>
                <Icon size={20} className={stat.color} />
              </div>
              <div className="min-w-0">
                <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                  {stat.label}
                </div>
                <div className={`text-2xl font-bold ${stat.color} mt-0.5`}>
                  {isLoading ? (
                    <span className="text-muted-foreground animate-pulse">...</span>
                  ) : (
                    stat.value
                  )}
                </div>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
