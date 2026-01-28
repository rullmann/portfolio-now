/**
 * Dividends Widget - Shows dividend summary
 */

import { useEffect, useState, useMemo } from 'react';
import { RefreshCw, TrendingUp } from 'lucide-react';
import { generateDividendReport } from '../../../lib/api';
import type { DividendReport } from '../../../lib/types';
import type { WidgetProps } from '../types';

interface DividendsWidgetProps extends WidgetProps {
  currency?: string;
}

export function DividendsWidget({ currency = 'EUR' }: DividendsWidgetProps) {
  const [report, setReport] = useState<DividendReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Calculate date range (default: current year)
  const { startDate, endDate } = useMemo(() => {
    const now = new Date();
    const year = now.getFullYear();
    return {
      startDate: `${year}-01-01`,
      endDate: `${year}-12-31`,
    };
  }, []);

  const loadDividends = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await generateDividendReport(startDate, endDate);
      setReport(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadDividends();
  }, [startDate, endDate]);

  const formatCurrency = (value: number, curr: string) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency: curr,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value);
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col p-4">
        <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
          Dividenden
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center text-sm text-muted-foreground">
            <p>{error}</p>
            <button
              onClick={loadDividends}
              className="mt-2 text-primary hover:underline"
            >
              Erneut versuchen
            </button>
          </div>
        </div>
      </div>
    );
  }

  const totalNet = report?.totalNet ?? 0;
  const totalGross = report?.totalGross ?? 0;
  const totalTaxes = report?.totalTaxes ?? 0;
  const entriesCount = report?.entries?.length ?? 0;
  const reportCurrency = report?.currency ?? currency;

  // Calculate monthly average
  const currentMonth = new Date().getMonth() + 1;
  const monthlyAverage = currentMonth > 0 ? totalNet / currentMonth : 0;

  return (
    <div className="h-full flex flex-col p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="text-xs text-muted-foreground uppercase tracking-wide">
          Dividenden {new Date().getFullYear()}
        </div>
        <div className="text-xs text-muted-foreground">
          {entriesCount} Zahlungen
        </div>
      </div>

      <div className="flex-1 flex flex-col justify-center">
        {/* Main Value */}
        <div className="text-center mb-4">
          <div className="text-2xl font-bold text-green-600">
            {formatCurrency(totalNet, reportCurrency)}
          </div>
          <div className="text-xs text-muted-foreground">Netto</div>
        </div>

        {/* Details Grid */}
        <div className="grid grid-cols-2 gap-3 text-center">
          <div>
            <div className="text-sm font-medium">
              {formatCurrency(totalGross, reportCurrency)}
            </div>
            <div className="text-[10px] text-muted-foreground">Brutto</div>
          </div>
          <div>
            <div className="text-sm font-medium text-red-600">
              -{formatCurrency(totalTaxes, reportCurrency)}
            </div>
            <div className="text-[10px] text-muted-foreground">Steuern</div>
          </div>
        </div>

        {/* Monthly Average */}
        {monthlyAverage > 0 && (
          <div className="mt-4 pt-3 border-t">
            <div className="flex items-center justify-center gap-2 text-xs text-muted-foreground">
              <TrendingUp className="h-3 w-3" />
              <span>
                {formatCurrency(monthlyAverage, reportCurrency)} / Monat
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
