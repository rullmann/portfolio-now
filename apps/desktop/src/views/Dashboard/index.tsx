/**
 * Dashboard - Full-width Bento Grid Layout
 * Modern, minimal design with cost basis visualization
 */

import { useEffect, useState, useMemo, useCallback, useRef } from 'react';
import {
  TrendingUp,
  FolderOpen,
  Database,
  Building2,
  RefreshCw,
  ArrowUpRight,
  ArrowDownRight,
  Sparkles,
} from 'lucide-react';
import { useDataModeStore, useSettingsStore, toast, type AutoUpdateInterval } from '../../store';
import type { PortfolioFile, AggregatedHolding, PortfolioData } from '../types';
import { formatNumber } from '../utils';
import { getBaseCurrency, calculatePerformance, syncAllPrices } from '../../lib/api';
import { useCachedLogos } from '../../lib/hooks';
import type { PerformanceResult } from '../../lib/types';
import {
  AreaChart,
  Area,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  ComposedChart,
  Line,
} from 'recharts';

interface DashboardViewProps {
  portfolioFile: PortfolioFile | null;
  dbHoldings: AggregatedHolding[];
  dbPortfolios: PortfolioData[];
  dbPortfolioHistory: Array<{ date: string; value: number }>;
  dbInvestedCapitalHistory: Array<{ date: string; value: number }>;
  onOpenFile: () => void;
  onImportToDb: () => void;
  onRefreshHoldings?: () => void;
}

// Sparkline component
function Sparkline({ data, positive }: { data: number[]; positive: boolean }) {
  const chartData = data.map((value, i) => ({ value, i }));
  const color = positive ? '#10b981' : '#ef4444';

  return (
    <div className="w-14 h-5">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ top: 0, right: 0, left: 0, bottom: 0 }}>
          <defs>
            <linearGradient id={`spark-${positive}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={color} stopOpacity={0.3} />
              <stop offset="100%" stopColor={color} stopOpacity={0} />
            </linearGradient>
          </defs>
          <Area
            type="monotone"
            dataKey="value"
            stroke={color}
            strokeWidth={1.5}
            fill={`url(#spark-${positive})`}
            dot={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}

// Main Chart with Invested Capital Line
function PortfolioChart({
  portfolioData,
  investedData,
  timeRange,
  onTimeRangeChange,
  currency,
}: {
  portfolioData: Array<{ date: string; value: number }>;
  investedData: Array<{ date: string; value: number }>;
  timeRange: string;
  onTimeRangeChange: (range: '1M' | '3M' | '6M' | '1Y' | 'MAX') => void;
  currency: string;
}) {
  // Merge portfolio and invested data by date
  // investedData has monthly dates (YYYY-MM-DD), portfolioData has daily dates
  // We need to find the invested value that applies to each portfolio date
  const mergedData = useMemo(() => {
    // Sort invested data by date
    const sortedInvested = [...investedData].sort((a, b) => a.date.localeCompare(b.date));

    // Find the last invested value that is <= each portfolio date
    const findInvestedValue = (portfolioDate: string): number => {
      let result = 0;
      for (const inv of sortedInvested) {
        if (inv.date <= portfolioDate) {
          result = inv.value;
        } else {
          break;
        }
      }
      return result;
    };

    return portfolioData.map((d) => ({
      date: d.date,
      value: d.value,
      invested: findInvestedValue(d.date),
    }));
  }, [portfolioData, investedData]);

  const firstValue = mergedData[0]?.value || 0;
  const lastValue = mergedData[mergedData.length - 1]?.value || 0;
  const lastInvested = mergedData[mergedData.length - 1]?.invested || 0;
  const isPositive = lastValue >= firstValue;
  const valueColor = isPositive ? '#10b981' : '#ef4444';

  // Calculate Y axis domain - start at 0
  const allValues = mergedData.flatMap((d) => [d.value, d.invested]);
  const dataMax = Math.max(...allValues);
  const yMax = dataMax * 1.05;

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-4">
          <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
            Portfolio-Entwicklung
          </span>
          <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <span className="w-4 h-0.5 rounded" style={{ backgroundColor: valueColor }} />
              Depotwert
            </span>
            <span className="flex items-center gap-1.5">
              <span className="w-4 h-0.5 rounded bg-blue-400" />
              Investiert ({formatNumber(lastInvested)} {currency})
            </span>
          </div>
        </div>
        <div className="flex gap-1">
          {(['1M', '3M', '6M', '1Y', 'MAX'] as const).map((range) => (
            <button
              key={range}
              onClick={() => onTimeRangeChange(range)}
              className={`px-2 py-0.5 text-[10px] font-medium rounded-full transition-all ${
                timeRange === range
                  ? 'bg-foreground text-background'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
              }`}
            >
              {range}
            </button>
          ))}
        </div>
      </div>

      {/* Chart */}
      <div className="flex-1 min-h-0">
        {mergedData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <ComposedChart data={mergedData} margin={{ top: 8, right: 8, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="portfolioGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={valueColor} stopOpacity={0.15} />
                  <stop offset="100%" stopColor={valueColor} stopOpacity={0} />
                </linearGradient>
              </defs>
              <XAxis
                dataKey="date"
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 9, fill: 'hsl(var(--muted-foreground))' }}
                tickFormatter={(value) => {
                  const d = new Date(value);
                  return d.toLocaleDateString('de-DE', { day: 'numeric', month: 'short' });
                }}
                interval="preserveStartEnd"
                minTickGap={60}
              />
              <YAxis
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 9, fill: 'hsl(var(--muted-foreground))' }}
                tickFormatter={(value) => `${(value / 1000).toFixed(0)}k`}
                width={45}
                domain={[0, yMax]}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: 'hsl(var(--background))',
                  border: '1px solid hsl(var(--border))',
                  borderRadius: '8px',
                  fontSize: '11px',
                  padding: '8px 12px',
                  boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
                }}
                labelFormatter={(label) =>
                  new Date(label).toLocaleDateString('de-DE', {
                    weekday: 'short',
                    day: 'numeric',
                    month: 'short',
                    year: 'numeric',
                  })
                }
                formatter={(value, name) => {
                  const numValue = typeof value === 'number' ? value : 0;
                  const label = name === 'invested' ? 'Investiert' : 'Depotwert';
                  return [
                    `${numValue.toLocaleString('de-DE', {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 2,
                    })} ${currency}`,
                    label,
                  ];
                }}
              />
              {/* Invested Capital Line - blue smooth line (monthly aggregated) */}
              <Line
                type="monotone"
                dataKey="invested"
                stroke="#60a5fa"
                strokeWidth={2}
                dot={false}
                activeDot={{ r: 3, fill: '#60a5fa' }}
              />
              {/* Portfolio Value Area */}
              <Area
                type="monotone"
                dataKey="value"
                stroke={valueColor}
                strokeWidth={2}
                fill="url(#portfolioGradient)"
                dot={false}
                activeDot={{ r: 4, fill: valueColor, strokeWidth: 0 }}
              />
            </ComposedChart>
          </ResponsiveContainer>
        ) : (
          <div className="h-full flex items-center justify-center text-muted-foreground/50 text-xs">
            Keine Daten
          </div>
        )}
      </div>
    </div>
  );
}

export function DashboardView({
  portfolioFile,
  dbHoldings,
  dbPortfolioHistory,
  dbInvestedCapitalHistory,
  onOpenFile,
  onImportToDb,
  onRefreshHoldings,
}: DashboardViewProps) {
  const { useDbData } = useDataModeStore();
  const brandfetchApiKey = useSettingsStore((state) => state.brandfetchApiKey);
  const finnhubApiKey = useSettingsStore((state) => state.finnhubApiKey);
  const coingeckoApiKey = useSettingsStore((state) => state.coingeckoApiKey);
  const alphaVantageApiKey = useSettingsStore((state) => state.alphaVantageApiKey);
  const twelveDataApiKey = useSettingsStore((state) => state.twelveDataApiKey);
  const syncOnlyHeldSecurities = useSettingsStore((state) => state.syncOnlyHeldSecurities);
  const autoUpdateInterval = useSettingsStore((state) => state.autoUpdateInterval);
  const setAutoUpdateInterval = useSettingsStore((state) => state.setAutoUpdateInterval);

  const [baseCurrency, setBaseCurrency] = useState<string>('EUR');
  const [performance, setPerformance] = useState<PerformanceResult | null>(null);
  const [chartTimeRange, setChartTimeRange] = useState<'1M' | '3M' | '6M' | '1Y' | 'MAX'>('1Y');
  const [isSyncing, setIsSyncing] = useState(false);
  const lastSyncTime = useSettingsStore((state) => state.lastSyncTime);
  const setLastSyncTime = useSettingsStore((state) => state.setLastSyncTime);
  const autoUpdateTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const handleSyncQuotes = useCallback(async () => {
    if (isSyncing) return;
    setIsSyncing(true);
    try {
      const apiKeys = {
        finnhub: finnhubApiKey || undefined,
        coingecko: coingeckoApiKey || undefined,
        alphaVantage: alphaVantageApiKey || undefined,
        twelveData: twelveDataApiKey || undefined,
      };
      const result = await syncAllPrices(syncOnlyHeldSecurities, apiKeys);
      setLastSyncTime(new Date());
      if (result.errors > 0) {
        toast.warning(`${result.success} aktualisiert, ${result.errors} Fehler`);
      } else {
        toast.success(`${result.success} Kurse aktualisiert`);
      }
      onRefreshHoldings?.();
    } catch (err) {
      toast.error(`Sync fehlgeschlagen: ${err}`);
    } finally {
      setIsSyncing(false);
    }
  }, [
    isSyncing,
    finnhubApiKey,
    coingeckoApiKey,
    alphaVantageApiKey,
    twelveDataApiKey,
    syncOnlyHeldSecurities,
    onRefreshHoldings,
    setLastSyncTime,
  ]);

  useEffect(() => {
    if (autoUpdateTimerRef.current) {
      clearInterval(autoUpdateTimerRef.current);
      autoUpdateTimerRef.current = null;
    }
    if (autoUpdateInterval > 0 && useDbData) {
      const intervalMs = autoUpdateInterval * 60 * 1000;
      autoUpdateTimerRef.current = setInterval(() => {
        handleSyncQuotes();
      }, intervalMs);
    }
    return () => {
      if (autoUpdateTimerRef.current) {
        clearInterval(autoUpdateTimerRef.current);
      }
    };
  }, [autoUpdateInterval, useDbData, handleSyncQuotes]);

  const securitiesForLogos = useMemo(
    () =>
      dbHoldings.map((h) => ({
        id: h.securityId,
        ticker: undefined,
        name: h.name || '',
      })),
    [dbHoldings]
  );

  const { logos: cachedLogos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

  useEffect(() => {
    if (useDbData) {
      getBaseCurrency()
        .then(setBaseCurrency)
        .catch(() => setBaseCurrency('EUR'));
    }
  }, [useDbData]);

  useEffect(() => {
    if (useDbData && dbHoldings.length > 0) {
      calculatePerformance()
        .then(setPerformance)
        .catch(() => setPerformance(null));
    }
  }, [useDbData, dbHoldings]);

  const { filteredChartData, filteredInvestedData } = useMemo(() => {
    if (dbPortfolioHistory.length === 0) {
      return { filteredChartData: [], filteredInvestedData: [] };
    }

    const now = new Date();
    let cutoffDate: Date;
    switch (chartTimeRange) {
      case '1M':
        cutoffDate = new Date(new Date().setMonth(now.getMonth() - 1));
        break;
      case '3M':
        cutoffDate = new Date(new Date().setMonth(now.getMonth() - 3));
        break;
      case '6M':
        cutoffDate = new Date(new Date().setMonth(now.getMonth() - 6));
        break;
      case '1Y':
        cutoffDate = new Date(new Date().setFullYear(now.getFullYear() - 1));
        break;
      case 'MAX':
      default:
        return {
          filteredChartData: dbPortfolioHistory,
          filteredInvestedData: dbInvestedCapitalHistory,
        };
    }

    return {
      filteredChartData: dbPortfolioHistory.filter((d) => new Date(d.date) >= cutoffDate),
      // Keep ALL invested data so we can find the historical value before the filtered period
      filteredInvestedData: dbInvestedCapitalHistory,
    };
  }, [dbPortfolioHistory, dbInvestedCapitalHistory, chartTimeRange]);

  // DB-based dashboard
  if (useDbData && dbHoldings.length > 0) {
    const totalValue = dbHoldings.reduce((sum, h) => sum + (h.currentValue || 0), 0);
    const totalCostBasis = dbHoldings.reduce((sum, h) => sum + h.costBasis, 0);
    const totalGainLoss = totalValue - totalCostBasis;
    const totalGainLossPercent = totalCostBasis > 0 ? (totalGainLoss / totalCostBasis) * 100 : 0;

    const dailyChange =
      filteredChartData.length >= 2
        ? filteredChartData[filteredChartData.length - 1].value -
          filteredChartData[filteredChartData.length - 2].value
        : 0;
    const dailyChangePercent =
      filteredChartData.length >= 2 && filteredChartData[filteredChartData.length - 2].value > 0
        ? (dailyChange / filteredChartData[filteredChartData.length - 2].value) * 100
        : 0;

    const holdingsByValue = [...dbHoldings].sort(
      (a, b) => (b.currentValue || 0) - (a.currentValue || 0)
    );

    return (
      <div className="h-full flex flex-col p-3 gap-2 overflow-hidden">
        {/* Top Row - Metrics */}
        <div className="flex gap-2 flex-shrink-0">
          {/* Portfolio Value - Hero */}
          <div className="glass-card p-4 flex-1 min-w-[280px]">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
                Portfolio
              </span>
              <div className="flex items-center gap-2">
                {lastSyncTime && (
                  <span className="text-[9px] text-muted-foreground/60">
                    {new Date(lastSyncTime).toLocaleTimeString('de-DE', {
                      hour: '2-digit',
                      minute: '2-digit',
                    })}
                  </span>
                )}
                <button
                  onClick={handleSyncQuotes}
                  disabled={isSyncing}
                  className="p-1 rounded-full hover:bg-muted/50 transition-colors"
                  title="Kurse aktualisieren"
                >
                  <RefreshCw
                    size={12}
                    className={isSyncing ? 'animate-spin text-primary' : 'text-muted-foreground'}
                  />
                </button>
              </div>
            </div>
            <div className="text-3xl font-light tracking-tight">
              {formatNumber(totalValue)}
              <span className="text-base text-muted-foreground ml-1">{baseCurrency}</span>
            </div>
            <div className="flex items-center gap-2 mt-1">
              <span
                className={`inline-flex items-center gap-0.5 text-sm font-medium ${
                  totalGainLoss >= 0 ? 'text-emerald-500' : 'text-red-500'
                }`}
              >
                {totalGainLoss >= 0 ? <ArrowUpRight size={14} /> : <ArrowDownRight size={14} />}
                {totalGainLoss >= 0 ? '+' : ''}
                {formatNumber(totalGainLoss)}
              </span>
              <span
                className={`text-xs px-1.5 py-0.5 rounded ${
                  totalGainLossPercent >= 0
                    ? 'bg-emerald-500/10 text-emerald-500'
                    : 'bg-red-500/10 text-red-500'
                }`}
              >
                {totalGainLossPercent >= 0 ? '+' : ''}
                {totalGainLossPercent.toFixed(2)}%
              </span>
            </div>
          </div>

          {/* Metric Cards */}
          <div className="glass-card p-3 min-w-[100px]">
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground block mb-1">
              Heute
            </span>
            <div
              className={`text-xl font-medium ${
                dailyChange >= 0 ? 'text-emerald-500' : 'text-red-500'
              }`}
            >
              {dailyChangePercent >= 0 ? '+' : ''}
              {dailyChangePercent.toFixed(2)}%
            </div>
            <div
              className={`text-[10px] ${dailyChange >= 0 ? 'text-emerald-500/70' : 'text-red-500/70'}`}
            >
              {dailyChange >= 0 ? '+' : ''}
              {formatNumber(dailyChange)}
            </div>
          </div>

          <div className="glass-card p-3 min-w-[100px]">
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground block mb-1">
              TTWROR
            </span>
            <div
              className={`text-xl font-medium ${
                (performance?.ttwror ?? 0) >= 0 ? 'text-emerald-500' : 'text-red-500'
              }`}
            >
              {performance?.ttwror != null
                ? `${performance.ttwror >= 0 ? '+' : ''}${performance.ttwror.toFixed(1)}%`
                : '—'}
            </div>
            <div className="text-[10px] text-muted-foreground">Zeitgewichtet</div>
          </div>

          <div className="glass-card p-3 min-w-[100px]">
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground block mb-1">
              IRR
            </span>
            <div
              className={`text-xl font-medium ${
                (performance?.irr ?? 0) >= 0 ? 'text-emerald-500' : 'text-red-500'
              }`}
            >
              {performance?.irr != null
                ? `${performance.irr >= 0 ? '+' : ''}${performance.irr.toFixed(1)}%`
                : '—'}
            </div>
            <div className="text-[10px] text-muted-foreground">Kapitalgewichtet</div>
          </div>

          <div className="glass-card p-3 min-w-[100px]">
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground block mb-1">
              Einstand
            </span>
            <div className="text-xl font-medium">{formatNumber(totalCostBasis)}</div>
            <div className="text-[10px] text-muted-foreground">{baseCurrency}</div>
          </div>

          {/* Auto-Update */}
          <div className="glass-card p-3 flex flex-col justify-between min-w-[110px]">
            <div className="flex items-center gap-1.5">
              <Sparkles size={12} className="text-muted-foreground" />
              <span className="text-[10px] text-muted-foreground">Auto-Sync</span>
            </div>
            <select
              value={autoUpdateInterval}
              onChange={(e) => setAutoUpdateInterval(Number(e.target.value) as AutoUpdateInterval)}
              className="bg-transparent border-none text-sm font-medium text-foreground focus:outline-none cursor-pointer -ml-1"
            >
              <option value={0}>Aus</option>
              <option value={15}>15 Min</option>
              <option value={30}>30 Min</option>
              <option value={60}>1 Std</option>
            </select>
          </div>
        </div>

        {/* Main Content - Chart + Holdings */}
        <div className="flex-1 flex gap-2 min-h-0 overflow-hidden">
          {/* Chart */}
          <div className="flex-1 glass-card p-4 min-w-0">
            <PortfolioChart
              portfolioData={filteredChartData}
              investedData={filteredInvestedData}
              timeRange={chartTimeRange}
              onTimeRangeChange={setChartTimeRange}
              currency={baseCurrency}
            />
          </div>

          {/* Holdings Sidebar */}
          <div className="w-[340px] glass-card p-3 flex flex-col flex-shrink-0">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
                Positionen
              </span>
              <span className="text-[10px] text-muted-foreground">{dbHoldings.length} Titel</span>
            </div>
            <div className="flex-1 overflow-y-auto -mx-3 px-3 space-y-0.5">
              {holdingsByValue.map((holding) => {
                const cachedLogo = cachedLogos.get(holding.securityId);
                const logoUrl = holding.customLogo || cachedLogo?.url;
                const percent =
                  totalValue > 0 ? ((holding.currentValue || 0) / totalValue) * 100 : 0;
                const gainPercent = holding.gainLossPercent || 0;
                const isPositive = gainPercent >= 0;

                const sparkData = Array.from({ length: 10 }, (_, i) => {
                  const base = 100;
                  const trend = isPositive ? 1 : -1;
                  return base + trend * i * 2 + Math.random() * 8;
                });

                return (
                  <div
                    key={holding.securityId}
                    className="flex items-center gap-2 py-1.5 px-2 -mx-2 rounded-lg hover:bg-muted/30 transition-colors cursor-pointer group"
                  >
                    <div className="w-7 h-7 rounded-md bg-muted/50 flex items-center justify-center overflow-hidden flex-shrink-0">
                      {logoUrl ? (
                        <img src={logoUrl} alt="" className="w-full h-full object-contain" />
                      ) : (
                        <Building2 size={12} className="text-muted-foreground" />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-xs font-medium truncate group-hover:text-primary transition-colors">
                        {holding.name}
                      </div>
                      <div className="text-[10px] text-muted-foreground">{percent.toFixed(1)}%</div>
                    </div>
                    <Sparkline data={sparkData} positive={isPositive} />
                    <div className="text-right min-w-[65px]">
                      <div className="text-xs font-medium">
                        {formatNumber(holding.currentValue || 0)}
                      </div>
                      <div
                        className={`text-[10px] ${isPositive ? 'text-emerald-500' : 'text-red-500'}`}
                      >
                        {isPositive ? '+' : ''}
                        {gainPercent.toFixed(1)}%
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Welcome screen
  if (!portfolioFile) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center max-w-sm">
          <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center mx-auto mb-6 backdrop-blur-sm border border-primary/10">
            <TrendingUp className="w-8 h-8 text-primary" />
          </div>
          <h2 className="text-xl font-light mb-2">Portfolio Now</h2>
          <p className="text-sm text-muted-foreground mb-8">
            Importieren Sie Ihre Portfolio Performance Datei
          </p>
          <div className="flex gap-3 justify-center">
            <button
              onClick={onOpenFile}
              className="flex items-center gap-2 px-4 py-2 text-sm border border-border rounded-xl hover:bg-muted/50 transition-colors"
            >
              <FolderOpen size={16} />
              Öffnen
            </button>
            <button
              onClick={onImportToDb}
              className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors"
            >
              <Database size={16} />
              Importieren
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-4 text-center text-muted-foreground text-sm">
      Portfolio geladen. Importieren Sie in die Datenbank für die vollständige Ansicht.
    </div>
  );
}
