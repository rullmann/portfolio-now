/**
 * Dividends view - comprehensive overview of dividend income.
 */

import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { Coins, TrendingUp, TrendingDown, Calendar, RefreshCw, AlertCircle, Building2 } from 'lucide-react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
  PieChart,
  Pie,
  Cell,
} from 'recharts';
import {
  generateDividendReport,
  getSecurities,
  fetchLogosBatch,
  getCachedLogoData,
  saveLogoToCache,
} from '../../lib/api';
import type { DividendReport, SecurityData } from '../../lib/types';
import { formatDate } from '../../lib/types';
import { useSettingsStore } from '../../store';

const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

interface LogoData {
  url: string;
  domain: string;
  isFresh: boolean;
}

export function DividendsView() {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentYearData, setCurrentYearData] = useState<DividendReport | null>(null);
  const [lastYearData, setLastYearData] = useState<DividendReport | null>(null);
  const [selectedYear, setSelectedYear] = useState<number>(new Date().getFullYear());
  const [logos, setLogos] = useState<Map<number, LogoData>>(new Map());
  const logosToCache = useRef<Map<number, { url: string; domain: string }>>(new Map());

  const { brandfetchApiKey } = useSettingsStore();

  // Load dividend data for current and previous year
  const loadData = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const currentYear = selectedYear;
      const lastYear = currentYear - 1;

      // Load dividend data and securities in parallel
      const [currentData, previousData, allSecurities] = await Promise.all([
        generateDividendReport(`${currentYear}-01-01`, `${currentYear}-12-31`, undefined),
        generateDividendReport(`${lastYear}-01-01`, `${lastYear}-12-31`, undefined),
        getSecurities(),
      ]);

      setCurrentYearData(currentData);
      setLastYearData(previousData);

      // Create securities map for quick lookup
      const secMap = new Map<number, SecurityData>();
      allSecurities.forEach((s) => secMap.set(s.id, s));

      // Get unique security IDs from dividends
      const securityIds = new Set<number>();
      currentData.bySecurity.forEach((s) => securityIds.add(s.securityId));
      currentData.entries.forEach((e) => securityIds.add(e.securityId));

      // Fetch logos for these securities
      const securitiesToFetch: { id: number; ticker?: string; name: string }[] = [];
      for (const id of securityIds) {
        const sec = secMap.get(id);
        if (sec) {
          securitiesToFetch.push({
            id: sec.id,
            ticker: sec.ticker || undefined,
            name: sec.name || '',
          });
        }
      }

      if (securitiesToFetch.length > 0) {
        const results = await fetchLogosBatch(brandfetchApiKey || '', securitiesToFetch);
        const newLogos = new Map<number, LogoData>();
        const toCacheMap = new Map<number, { url: string; domain: string }>();

        for (const result of results) {
          if (result.domain) {
            const cachedData = await getCachedLogoData(result.domain);

            if (cachedData) {
              newLogos.set(result.securityId, {
                url: cachedData,
                domain: result.domain,
                isFresh: false,
              });
            } else if (result.logoUrl) {
              newLogos.set(result.securityId, {
                url: result.logoUrl,
                domain: result.domain,
                isFresh: true,
              });
              toCacheMap.set(result.securityId, {
                url: result.logoUrl,
                domain: result.domain,
              });
            }
          }
        }

        setLogos(newLogos);
        logosToCache.current = toCacheMap;
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [selectedYear, brandfetchApiKey]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Handle logo load - cache fresh logos
  const handleLogoLoad = useCallback(async (securityId: number, imgElement: HTMLImageElement) => {
    const toCache = logosToCache.current.get(securityId);
    if (!toCache) return;

    try {
      const canvas = document.createElement('canvas');
      canvas.width = imgElement.naturalWidth || 64;
      canvas.height = imgElement.naturalHeight || 64;

      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      ctx.drawImage(imgElement, 0, 0);
      const base64Data = canvas.toDataURL('image/png');

      await saveLogoToCache(toCache.domain, base64Data);

      // Update logo state to show as cached
      setLogos((prev) => {
        const next = new Map(prev);
        const current = next.get(securityId);
        if (current) {
          next.set(securityId, { ...current, url: base64Data, isFresh: false });
        }
        return next;
      });

      logosToCache.current.delete(securityId);
    } catch (err) {
      console.error('Failed to cache logo:', err);
    }
  }, []);

  // Calculate YoY growth
  const yoyGrowth = useMemo(() => {
    if (!currentYearData || !lastYearData || lastYearData.totalNet === 0) return null;
    return ((currentYearData.totalNet - lastYearData.totalNet) / lastYearData.totalNet) * 100;
  }, [currentYearData, lastYearData]);

  // Monthly chart data with comparison
  const monthlyChartData = useMemo(() => {
    const months = ['Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez'];
    const monthMap: Record<string, { current: number; previous: number }> = {};

    months.forEach((_, idx) => {
      const key = String(idx + 1).padStart(2, '0');
      monthMap[key] = { current: 0, previous: 0 };
    });

    currentYearData?.byMonth.forEach((m) => {
      const monthKey = m.month.split('-')[1];
      if (monthMap[monthKey]) {
        monthMap[monthKey].current = m.totalNet;
      }
    });

    lastYearData?.byMonth.forEach((m) => {
      const monthKey = m.month.split('-')[1];
      if (monthMap[monthKey]) {
        monthMap[monthKey].previous = m.totalNet;
      }
    });

    return months.map((month, idx) => {
      const key = String(idx + 1).padStart(2, '0');
      return {
        month,
        [selectedYear]: monthMap[key].current,
        [selectedYear - 1]: monthMap[key].previous,
      };
    });
  }, [currentYearData, lastYearData, selectedYear]);

  // Top dividend payers
  const topPayers = useMemo(() => {
    if (!currentYearData) return [];
    return [...currentYearData.bySecurity]
      .sort((a, b) => b.totalNet - a.totalNet)
      .slice(0, 8);
  }, [currentYearData]);

  // Pie chart data
  const pieData = useMemo(() => {
    return topPayers.map((s) => ({
      name: s.securityName,
      value: s.totalNet,
    }));
  }, [topPayers]);

  const formatCurrency = (amount: number, currency: string = 'EUR') => {
    return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
  };

  const years = useMemo(() => {
    const currentYear = new Date().getFullYear();
    return Array.from({ length: 10 }, (_, i) => currentYear - i);
  }, []);

  // Logo component
  const SecurityLogo = ({ securityId, size = 32 }: { securityId: number; size?: number }) => {
    const logoData = logos.get(securityId);

    if (logoData) {
      return (
        <img
          src={logoData.url}
          alt=""
          className="rounded-md object-contain bg-white flex-shrink-0"
          style={{ width: size, height: size }}
          crossOrigin="anonymous"
          onLoad={(e) => {
            if (logoData.isFresh) {
              handleLogoLoad(securityId, e.currentTarget);
            }
          }}
          onError={(e) => {
            e.currentTarget.style.display = 'none';
          }}
        />
      );
    }

    return (
      <div
        className="rounded-md bg-muted flex items-center justify-center flex-shrink-0"
        style={{ width: size, height: size }}
      >
        <Building2 size={size * 0.5} className="text-muted-foreground" />
      </div>
    );
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Coins className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Dividenden</h1>
        </div>
        <div className="flex items-center gap-3">
          <select
            value={selectedYear}
            onChange={(e) => setSelectedYear(Number(e.target.value))}
            className="px-3 py-2 border border-border rounded-md bg-background"
          >
            {years.map((y) => (
              <option key={y} value={y}>
                {y}
              </option>
            ))}
          </select>
          <button
            onClick={loadData}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-2 border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="flex items-center gap-2 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          <AlertCircle size={16} />
          {error}
        </div>
      )}

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
            <Calendar size={14} />
            Dividenden {selectedYear}
          </div>
          <div className="text-2xl font-bold text-green-600">
            {isLoading ? (
              <span className="text-muted-foreground">...</span>
            ) : (
              formatCurrency(currentYearData?.totalNet || 0, currentYearData?.currency || 'EUR')
            )}
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            {currentYearData?.entries.length || 0} Zahlungen
          </div>
        </div>

        <div className="bg-card rounded-lg border border-border p-4">
          <div className="flex items-center gap-2 text-sm text-muted-foreground mb-1">
            <Calendar size={14} />
            Dividenden {selectedYear - 1}
          </div>
          <div className="text-2xl font-bold">
            {isLoading ? (
              <span className="text-muted-foreground">...</span>
            ) : (
              formatCurrency(lastYearData?.totalNet || 0, lastYearData?.currency || 'EUR')
            )}
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            {lastYearData?.entries.length || 0} Zahlungen
          </div>
        </div>

        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground mb-1">Veränderung zum Vorjahr</div>
          {isLoading ? (
            <div className="text-2xl font-bold text-muted-foreground">...</div>
          ) : yoyGrowth !== null ? (
            <div className={`text-2xl font-bold flex items-center gap-2 ${yoyGrowth >= 0 ? 'text-green-600' : 'text-red-600'}`}>
              {yoyGrowth >= 0 ? <TrendingUp size={20} /> : <TrendingDown size={20} />}
              {yoyGrowth >= 0 ? '+' : ''}{yoyGrowth.toFixed(1)}%
            </div>
          ) : (
            <div className="text-2xl font-bold text-muted-foreground">-</div>
          )}
          <div className="text-xs text-muted-foreground mt-1">
            {yoyGrowth !== null && lastYearData && currentYearData
              ? formatCurrency(currentYearData.totalNet - lastYearData.totalNet)
              : 'Keine Vergleichsdaten'}
          </div>
        </div>

        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground mb-1">Durchschnitt pro Monat</div>
          <div className="text-2xl font-bold">
            {isLoading ? (
              <span className="text-muted-foreground">...</span>
            ) : (
              formatCurrency(
                (currentYearData?.totalNet || 0) / (new Date().getMonth() + 1),
                currentYearData?.currency || 'EUR'
              )
            )}
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            {currentYearData?.bySecurity.length || 0} Wertpapiere
          </div>
        </div>
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2 bg-card rounded-lg border border-border p-4">
          <h3 className="font-semibold mb-4">Monatliche Dividenden</h3>
          {isLoading ? (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Lade Daten...
            </div>
          ) : monthlyChartData.length > 0 ? (
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={monthlyChartData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                  <XAxis dataKey="month" tick={{ fontSize: 12 }} />
                  <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => `${v.toFixed(0)}€`} />
                  <Tooltip
                    formatter={(value) => [
                      typeof value === 'number' ? formatCurrency(value) : '-',
                    ]}
                    labelFormatter={(label) => `${label}`}
                  />
                  <Legend />
                  <Bar dataKey={selectedYear} name={String(selectedYear)} fill={COLORS[1]} />
                  <Bar dataKey={selectedYear - 1} name={String(selectedYear - 1)} fill={COLORS[0]} opacity={0.5} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Keine Daten vorhanden
            </div>
          )}
        </div>

        <div className="bg-card rounded-lg border border-border p-4">
          <h3 className="font-semibold mb-4">Nach Wertpapier</h3>
          {isLoading ? (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Lade Daten...
            </div>
          ) : pieData.length > 0 ? (
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={pieData}
                    dataKey="value"
                    nameKey="name"
                    cx="50%"
                    cy="50%"
                    outerRadius={80}
                    label={({ percent }) => `${((percent ?? 0) * 100).toFixed(0)}%`}
                    labelLine={false}
                  >
                    {pieData.map((_, idx) => (
                      <Cell key={`cell-${idx}`} fill={COLORS[idx % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => [typeof value === 'number' ? formatCurrency(value) : '-']} />
                </PieChart>
              </ResponsiveContainer>
            </div>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Keine Daten vorhanden
            </div>
          )}
        </div>
      </div>

      {/* Top Payers Table */}
      {!isLoading && topPayers.length > 0 && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-4 border-b border-border">
            <h3 className="font-semibold">Top Dividendenzahler {selectedYear}</h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50">
                  <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
                  <th className="text-right py-3 px-4 font-medium">Zahlungen</th>
                  <th className="text-right py-3 px-4 font-medium">Brutto</th>
                  <th className="text-right py-3 px-4 font-medium">Steuer</th>
                  <th className="text-right py-3 px-4 font-medium">Netto</th>
                  <th className="text-right py-3 px-4 font-medium">Anteil</th>
                </tr>
              </thead>
              <tbody>
                {topPayers.map((security, idx) => {
                  const totalNet = currentYearData?.totalNet || 1;
                  const share = (security.totalNet / totalNet) * 100;
                  return (
                    <tr key={security.securityId} className="border-b border-border last:border-0 hover:bg-muted/30">
                      <td className="py-3 px-4">
                        <div className="flex items-center gap-3">
                          <SecurityLogo securityId={security.securityId} size={32} />
                          <div
                            className="w-3 h-3 rounded-full flex-shrink-0"
                            style={{ backgroundColor: COLORS[idx % COLORS.length] }}
                          />
                          <div>
                            <div className="font-medium">{security.securityName}</div>
                            {security.securityIsin && (
                              <div className="text-xs text-muted-foreground">{security.securityIsin}</div>
                            )}
                          </div>
                        </div>
                      </td>
                      <td className="py-3 px-4 text-right">{security.paymentCount}</td>
                      <td className="py-3 px-4 text-right">
                        {formatCurrency(security.totalGross)}
                      </td>
                      <td className="py-3 px-4 text-right text-red-600">
                        -{formatCurrency(security.totalTaxes)}
                      </td>
                      <td className="py-3 px-4 text-right font-medium text-green-600">
                        {formatCurrency(security.totalNet)}
                      </td>
                      <td className="py-3 px-4 text-right">
                        <div className="flex items-center justify-end gap-2">
                          <div className="w-16 bg-muted rounded-full h-2 overflow-hidden">
                            <div
                              className="h-full bg-primary"
                              style={{ width: `${share}%` }}
                            />
                          </div>
                          <span className="text-muted-foreground w-12 text-right">
                            {share.toFixed(1)}%
                          </span>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
              <tfoot>
                <tr className="bg-muted/50 font-medium">
                  <td className="py-3 px-4">Gesamt</td>
                  <td className="py-3 px-4 text-right">
                    {currentYearData?.entries.length || 0}
                  </td>
                  <td className="py-3 px-4 text-right">
                    {formatCurrency(currentYearData?.totalGross || 0)}
                  </td>
                  <td className="py-3 px-4 text-right text-red-600">
                    -{formatCurrency(currentYearData?.totalTaxes || 0)}
                  </td>
                  <td className="py-3 px-4 text-right text-green-600">
                    {formatCurrency(currentYearData?.totalNet || 0)}
                  </td>
                  <td className="py-3 px-4 text-right">100%</td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>
      )}

      {/* Recent Payments Table */}
      {!isLoading && currentYearData && currentYearData.entries.length > 0 && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-4 border-b border-border">
            <h3 className="font-semibold">Letzte Dividendenzahlungen</h3>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50">
                  <th className="text-left py-3 px-4 font-medium">Datum</th>
                  <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
                  <th className="text-right py-3 px-4 font-medium">Brutto</th>
                  <th className="text-right py-3 px-4 font-medium">Steuer</th>
                  <th className="text-right py-3 px-4 font-medium">Netto</th>
                  <th className="text-right py-3 px-4 font-medium">Pro Stück</th>
                </tr>
              </thead>
              <tbody>
                {currentYearData.entries.map((entry, idx) => (
                  <tr key={idx} className="border-b border-border last:border-0 hover:bg-muted/30">
                    <td className="py-3 px-4">{formatDate(entry.date)}</td>
                    <td className="py-3 px-4">
                      <div className="flex items-center gap-3">
                        <SecurityLogo securityId={entry.securityId} size={28} />
                        <div>
                          <div className="font-medium">{entry.securityName}</div>
                          {entry.shares && (
                            <div className="text-xs text-muted-foreground">
                              {entry.shares.toLocaleString('de-DE', { maximumFractionDigits: 4 })} Stück
                            </div>
                          )}
                        </div>
                      </div>
                    </td>
                    <td className="py-3 px-4 text-right">
                      {formatCurrency(entry.grossAmount, entry.currency)}
                    </td>
                    <td className="py-3 px-4 text-right text-red-600">
                      -{formatCurrency(entry.taxes, entry.currency)}
                    </td>
                    <td className="py-3 px-4 text-right font-medium text-green-600">
                      {formatCurrency(entry.netAmount, entry.currency)}
                    </td>
                    <td className="py-3 px-4 text-right text-muted-foreground">
                      {entry.perShare
                        ? formatCurrency(entry.perShare, entry.currency)
                        : '-'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Empty State */}
      {!isLoading && (!currentYearData || currentYearData.entries.length === 0) && (
        <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
          <Coins className="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p>Keine Dividenden für {selectedYear} gefunden.</p>
        </div>
      )}
    </div>
  );
}
