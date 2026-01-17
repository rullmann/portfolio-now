/**
 * Dividend Forecast - Annual dividend projection based on historical patterns.
 */

import { useState, useEffect, useMemo } from 'react';
import { TrendingUp, TrendingDown, Calendar, Building2, CheckCircle2, Clock, AlertCircle } from 'lucide-react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { estimateAnnualDividends, getCachedLogoData, fetchLogosBatch, getSecurities } from '../../lib/api';
import type { DividendForecast as ForecastData } from '../../lib/api';
import type { SecurityData } from '../../lib/types';
import { useSettingsStore } from '../../store';

interface Props {
  selectedYear: number;
}

const MONTH_NAMES_SHORT = ['Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez'];

export function DividendForecast({ selectedYear }: Props) {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [forecast, setForecast] = useState<ForecastData | null>(null);
  const [logos, setLogos] = useState<Map<number, string>>(new Map());

  const { brandfetchApiKey } = useSettingsStore();

  // Load forecast data
  useEffect(() => {
    async function loadData() {
      setIsLoading(true);
      setError(null);
      try {
        const data = await estimateAnnualDividends(selectedYear);
        setForecast(data);

        // Load logos
        const securityIds = new Set(data.bySecurity.map(s => s.securityId));
        if (securityIds.size > 0) {
          const securities = await getSecurities();
          const secMap = new Map<number, SecurityData>();
          securities.forEach(s => secMap.set(s.id, s));

          const toFetch = Array.from(securityIds)
            .map(id => secMap.get(id))
            .filter((s): s is SecurityData => !!s)
            .map(s => ({ id: s.id, ticker: s.ticker || undefined, name: s.name }));

          if (toFetch.length > 0) {
            const results = await fetchLogosBatch(brandfetchApiKey || '', toFetch);
            const newLogos = new Map<number, string>();

            for (const result of results) {
              if (result.domain) {
                const cached = await getCachedLogoData(result.domain);
                if (cached) {
                  newLogos.set(result.securityId, cached);
                } else if (result.logoUrl) {
                  newLogos.set(result.securityId, result.logoUrl);
                }
              }
            }
            setLogos(newLogos);
          }
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoading(false);
      }
    }
    loadData();
  }, [selectedYear, brandfetchApiKey]);

  // Chart data
  const chartData = useMemo(() => {
    if (!forecast) return [];
    return forecast.byMonth.map(m => ({
      month: MONTH_NAMES_SHORT[m.month - 1],
      Erhalten: m.received,
      Erwartet: m.estimated,
      isPast: m.isPast,
    }));
  }, [forecast]);

  const formatCurrency = (amount: number, currency: string = 'EUR') => {
    return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
  };

  const formatFrequency = (freq: string) => {
    switch (freq) {
      case 'MONTHLY': return 'Monatlich';
      case 'QUARTERLY': return 'Quartalsweise';
      case 'SEMI_ANNUAL': return 'Halbjährlich';
      case 'ANNUAL': return 'Jährlich';
      case 'IRREGULAR': return 'Unregelmäßig';
      default: return freq;
    }
  };

  const SecurityLogo = ({ securityId, size = 24 }: { securityId: number; size?: number }) => {
    const logoUrl = logos.get(securityId);
    if (logoUrl) {
      return (
        <img
          src={logoUrl}
          alt=""
          className="rounded-sm object-contain bg-white flex-shrink-0"
          style={{ width: size, height: size }}
        />
      );
    }
    return (
      <div
        className="rounded-sm bg-muted flex items-center justify-center flex-shrink-0"
        style={{ width: size, height: size }}
      >
        <Building2 size={size * 0.6} className="text-muted-foreground" />
      </div>
    );
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        Lade Prognose...
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 p-4 bg-destructive/10 border border-destructive/20 rounded-md text-destructive">
        <AlertCircle size={20} />
        {error}
      </div>
    );
  }

  if (!forecast) {
    return (
      <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
        <Calendar className="w-12 h-12 mx-auto mb-3 opacity-50" />
        <p>Keine Dividenden-Daten für {selectedYear} verfügbar.</p>
      </div>
    );
  }

  const totalExpected = forecast.totalEstimated + forecast.totalReceived;
  const completionPercent = totalExpected > 0 ? (forecast.totalReceived / totalExpected) * 100 : 0;

  return (
    <div className="space-y-6">
      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
            <CheckCircle2 size={14} className="text-green-600" />
            Bereits erhalten
          </div>
          <div className="text-2xl font-bold text-green-600">
            {formatCurrency(forecast.totalReceived, forecast.currency)}
          </div>
        </div>

        <div className="bg-card rounded-lg border border-border p-4">
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
            <Clock size={14} className="text-blue-600" />
            Noch erwartet
          </div>
          <div className="text-2xl font-bold text-blue-600">
            {formatCurrency(forecast.totalEstimated, forecast.currency)}
          </div>
        </div>

        <div className="bg-card rounded-lg border border-border p-4">
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
            <TrendingUp size={14} />
            Jahresprognose {selectedYear}
          </div>
          <div className="text-2xl font-bold">
            {formatCurrency(totalExpected, forecast.currency)}
          </div>
          <div className="mt-2">
            <div className="flex items-center justify-between text-xs text-muted-foreground mb-1">
              <span>Fortschritt</span>
              <span>{completionPercent.toFixed(0)}%</span>
            </div>
            <div className="w-full bg-muted rounded-full h-2">
              <div
                className="bg-green-600 h-2 rounded-full transition-all"
                style={{ width: `${completionPercent}%` }}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Monthly Chart */}
      <div className="bg-card rounded-lg border border-border p-4">
        <h3 className="font-semibold mb-4">Monatliche Prognose {selectedYear}</h3>
        <div className="h-64">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="month" tick={{ fontSize: 12 }} />
              <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => `${v.toFixed(0)}€`} />
              <Tooltip
                formatter={(value, name) => [
                  typeof value === 'number' ? formatCurrency(value, forecast.currency) : '-',
                  name,
                ]}
              />
              <Legend />
              <Bar dataKey="Erhalten" stackId="a" fill="#10b981" />
              <Bar dataKey="Erwartet" stackId="a" fill="#3b82f6" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Per Security Forecast */}
      {forecast.bySecurity.length > 0 && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-4 border-b border-border">
            <h3 className="font-semibold">Dividenden nach Wertpapier</h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50">
                  <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
                  <th className="text-center py-3 px-4 font-medium">Frequenz</th>
                  <th className="text-right py-3 px-4 font-medium">Stück</th>
                  <th className="text-right py-3 px-4 font-medium">Pro Aktie</th>
                  <th className="text-right py-3 px-4 font-medium">Wachstum</th>
                  <th className="text-right py-3 px-4 font-medium">Prognose {selectedYear}</th>
                </tr>
              </thead>
              <tbody>
                {forecast.bySecurity.map(sec => {
                  const hasGrowth = sec.pattern.growthRate != null;
                  const isPositiveGrowth = hasGrowth && sec.pattern.growthRate! > 0;

                  return (
                    <tr key={sec.securityId} className="border-b border-border last:border-0 hover:bg-muted/30">
                      <td className="py-3 px-4">
                        <div className="flex items-center gap-3">
                          <SecurityLogo securityId={sec.securityId} size={32} />
                          <div>
                            <div className="font-medium">{sec.securityName}</div>
                            {sec.securityIsin && (
                              <div className="text-xs text-muted-foreground">{sec.securityIsin}</div>
                            )}
                          </div>
                        </div>
                      </td>
                      <td className="py-3 px-4 text-center">
                        <span className="px-2 py-1 bg-muted rounded text-xs">
                          {formatFrequency(sec.pattern.frequency)}
                        </span>
                      </td>
                      <td className="py-3 px-4 text-right">
                        {sec.sharesHeld.toLocaleString('de-DE', { maximumFractionDigits: 4 })}
                      </td>
                      <td className="py-3 px-4 text-right">
                        {formatCurrency(sec.pattern.avgPerShare, sec.pattern.currency)}
                      </td>
                      <td className="py-3 px-4 text-right">
                        {hasGrowth ? (
                          <span className={`flex items-center justify-end gap-1 ${
                            isPositiveGrowth ? 'text-green-600' : 'text-red-600'
                          }`}>
                            {isPositiveGrowth ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
                            {isPositiveGrowth ? '+' : ''}{sec.pattern.growthRate!.toFixed(1)}%
                          </span>
                        ) : (
                          <span className="text-muted-foreground">-</span>
                        )}
                      </td>
                      <td className="py-3 px-4 text-right font-medium text-green-600">
                        {formatCurrency(sec.estimatedAnnual, sec.pattern.currency)}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
              <tfoot>
                <tr className="bg-muted/50 font-medium">
                  <td colSpan={5} className="py-3 px-4">Gesamt Prognose</td>
                  <td className="py-3 px-4 text-right text-green-600">
                    {formatCurrency(totalExpected, forecast.currency)}
                  </td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>
      )}

      {/* Payment Schedule */}
      {forecast.bySecurity.some(s => s.expectedPayments.length > 0) && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-4 border-b border-border">
            <h3 className="font-semibold">Erwartete Zahlungstermine</h3>
          </div>
          <div className="p-4 grid grid-cols-2 md:grid-cols-4 gap-3">
            {forecast.byMonth.map(m => {
              const paymentsThisMonth = forecast.bySecurity
                .flatMap(s => s.expectedPayments.filter(p => p.month === m.month))
                .filter(p => !p.isReceived || p.actualAmount);

              if (paymentsThisMonth.length === 0 && m.estimated === 0 && m.received === 0) return null;

              return (
                <div
                  key={m.month}
                  className={`p-3 rounded-lg border ${
                    m.isPast
                      ? 'border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-900/20'
                      : 'border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-900/20'
                  }`}
                >
                  <div className="text-sm font-medium mb-1">{m.monthName}</div>
                  <div className={`text-lg font-bold ${m.isPast ? 'text-green-600' : 'text-blue-600'}`}>
                    {formatCurrency(m.isPast ? m.received : m.estimated, forecast.currency)}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {m.isPast ? (
                      <span className="flex items-center gap-1">
                        <CheckCircle2 size={12} className="text-green-600" />
                        Erhalten
                      </span>
                    ) : (
                      <span className="flex items-center gap-1">
                        <Clock size={12} className="text-blue-600" />
                        Erwartet
                      </span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Empty State */}
      {forecast.bySecurity.length === 0 && (
        <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
          <Calendar className="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p>Keine Wertpapiere mit Dividenden-Historie gefunden.</p>
          <p className="text-sm mt-2">Dividenden-Prognosen basieren auf historischen Zahlungsmustern.</p>
        </div>
      )}
    </div>
  );
}
