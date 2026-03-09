/**
 * DividendTimeline - Month-grouped payment timeline for the selected year.
 */

import { useMemo } from 'react';
import { CheckCircle2, Clock } from 'lucide-react';
import type { DividendReport, DividendEntry } from '../../lib/types';
import { SecurityLogo } from '../../components/common/SecurityLogo';
import type { CachedLogo } from '../../lib/hooks';

interface Props {
  report: DividendReport | null;
  selectedYear: number;
  logos: Map<number, CachedLogo>;
  currency: string;
  isLoading: boolean;
}

const MONTH_NAMES = [
  'Januar', 'Februar', 'März', 'April', 'Mai', 'Juni',
  'Juli', 'August', 'September', 'Oktober', 'November', 'Dezember',
];

const formatCurrency = (amount: number, currency: string) =>
  `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;

const formatDateShort = (dateStr: string) => {
  const parts = dateStr.split('-');
  return `${parts[2]}.${parts[1]}.`;
};

export function DividendTimeline({ report, selectedYear, logos, currency, isLoading }: Props) {
  const monthGroups = useMemo(() => {
    if (!report) return [];
    const now = new Date();
    const currentMonth = now.getFullYear() === selectedYear ? now.getMonth() + 1 : 13;

    // Group entries by month
    const groups = new Map<number, { entries: DividendEntry[]; total: number }>();
    for (const entry of report.entries) {
      const month = parseInt(entry.date.split('-')[1], 10);
      const existing = groups.get(month);
      if (existing) {
        existing.entries.push(entry);
        existing.total += entry.netAmount;
      } else {
        groups.set(month, { entries: [entry], total: entry.netAmount });
      }
    }

    return Array.from({ length: 12 }, (_, i) => {
      const month = i + 1;
      const data = groups.get(month);
      return {
        month,
        name: MONTH_NAMES[i],
        isPast: month < currentMonth,
        entries: data?.entries || [],
        total: data?.total || 0,
      };
    }).filter((m) => m.entries.length > 0 || m.isPast);
  }, [report, selectedYear]);

  if (isLoading) {
    return (
      <div className="bg-card rounded-xl border border-border p-5">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
          Zahlungsverlauf {selectedYear}
        </h3>
        <div className="h-32 flex items-center justify-center text-muted-foreground animate-pulse">
          Lade Daten...
        </div>
      </div>
    );
  }

  if (monthGroups.length === 0) {
    return (
      <div className="bg-card rounded-xl border border-border p-5">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
          Zahlungsverlauf {selectedYear}
        </h3>
        <div className="h-32 flex items-center justify-center text-muted-foreground">
          Keine Zahlungen in {selectedYear}
        </div>
      </div>
    );
  }

  return (
    <div className="bg-card rounded-xl border border-border p-5">
      <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
        Zahlungsverlauf {selectedYear}
      </h3>
      <div className="space-y-3">
        {monthGroups.map((group) => (
          <div key={group.month} className="relative">
            {/* Month header */}
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                {group.isPast && group.entries.length > 0 ? (
                  <CheckCircle2 size={14} className="text-emerald-500" />
                ) : (
                  <Clock size={14} className="text-muted-foreground" />
                )}
                <span className="text-sm font-semibold">{group.name}</span>
                {group.entries.length > 0 && (
                  <span className="text-xs text-muted-foreground">
                    ({group.entries.length} {group.entries.length === 1 ? 'Zahlung' : 'Zahlungen'})
                  </span>
                )}
              </div>
              {group.total > 0 && (
                <span className="text-sm font-semibold text-green-600 tabular-nums">
                  {formatCurrency(group.total, currency)}
                </span>
              )}
            </div>

            {/* Entries */}
            {group.entries.length > 0 && (
              <div className="ml-5 space-y-1">
                {group.entries.map((entry, idx) => (
                  <div
                    key={idx}
                    className="flex items-center justify-between py-1.5 px-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex items-center gap-2.5 min-w-0">
                      <SecurityLogo securityId={entry.securityId} logos={logos} size={20} />
                      <span className="text-sm truncate">{entry.securityName}</span>
                      <span className="text-xs text-muted-foreground flex-shrink-0">
                        {formatDateShort(entry.date)}
                      </span>
                    </div>
                    <span className="text-sm font-medium text-green-600 tabular-nums flex-shrink-0 ml-2">
                      {formatCurrency(entry.netAmount, entry.currency)}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
