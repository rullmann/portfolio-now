/**
 * DividendAllocation - Donut chart + legend showing dividend distribution by security.
 */

import { useMemo } from 'react';
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip,
} from 'recharts';
import type { DividendBySecurity } from '../../lib/types';
import { SecurityLogo } from '../../components/common/SecurityLogo';
import type { CachedLogo } from '../../lib/hooks';

interface Props {
  bySecurity: DividendBySecurity[];
  totalNet: number;
  currency: string;
  logos: Map<number, CachedLogo>;
  isLoading: boolean;
}

const COLORS = [
  '#10b981', '#3b82f6', '#8b5cf6', '#f59e0b', '#ef4444',
  '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1',
];

const formatCurrency = (amount: number, currency: string) =>
  `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;

export function DividendAllocation({ bySecurity, totalNet, currency, logos, isLoading }: Props) {
  const sorted = useMemo(() => {
    return [...bySecurity].sort((a, b) => b.totalNet - a.totalNet);
  }, [bySecurity]);

  const pieData = useMemo(() => {
    // Show top 8, rest as "Sonstige"
    const top = sorted.slice(0, 8);
    const rest = sorted.slice(8);
    const data = top.map((s) => ({
      name: s.securityName,
      value: s.totalNet,
      securityId: s.securityId,
    }));
    if (rest.length > 0) {
      data.push({
        name: 'Sonstige',
        value: rest.reduce((sum, s) => sum + s.totalNet, 0),
        securityId: -1,
      });
    }
    return data;
  }, [sorted]);

  if (isLoading) {
    return (
      <div className="bg-card rounded-xl border border-border p-5">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
          Verteilung
        </h3>
        <div className="h-64 flex items-center justify-center text-muted-foreground animate-pulse">
          Lade Daten...
        </div>
      </div>
    );
  }

  if (pieData.length === 0) {
    return (
      <div className="bg-card rounded-xl border border-border p-5">
        <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
          Verteilung
        </h3>
        <div className="h-64 flex items-center justify-center text-muted-foreground">
          Keine Daten vorhanden
        </div>
      </div>
    );
  }

  return (
    <div className="bg-card rounded-xl border border-border p-5">
      <h3 className="font-semibold text-sm uppercase tracking-wide text-muted-foreground mb-4">
        Verteilung
      </h3>
      <div className="space-y-4">
        {/* Donut */}
        <div className="h-52">
          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                data={pieData}
                dataKey="value"
                nameKey="name"
                cx="50%"
                cy="50%"
                innerRadius={55}
                outerRadius={95}
                paddingAngle={2}
                strokeWidth={0}
              >
                {pieData.map((_, idx) => (
                  <Cell key={idx} fill={COLORS[idx % COLORS.length]} />
                ))}
              </Pie>
              <Tooltip
                content={({ active, payload }) => {
                  if (!active || !payload?.length) return null;
                  const data = payload[0];
                  const pct = totalNet > 0 ? ((data.value as number) / totalNet * 100).toFixed(1) : '0';
                  return (
                    <div className="bg-popover border border-border rounded-lg shadow-lg p-2 text-xs">
                      <div className="font-medium">{data.name}</div>
                      <div className="text-green-600">{formatCurrency(data.value as number, currency)} ({pct}%)</div>
                    </div>
                  );
                }}
              />
            </PieChart>
          </ResponsiveContainer>
        </div>

        {/* Legend */}
        <div className="space-y-2">
          {sorted.slice(0, 10).map((sec, idx) => {
            const pct = totalNet > 0 ? (sec.totalNet / totalNet * 100) : 0;
            return (
              <div key={sec.securityId} className="flex items-center gap-2 text-sm">
                <div
                  className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                  style={{ backgroundColor: COLORS[idx % COLORS.length] }}
                />
                <SecurityLogo securityId={sec.securityId} logos={logos} size={18} />
                <span className="truncate flex-1 text-foreground">{sec.securityName}</span>
                <span className="text-muted-foreground tabular-nums flex-shrink-0">
                  {pct.toFixed(1)}%
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
