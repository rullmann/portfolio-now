/**
 * Dashboard - Full-width Bento Grid Layout
 * Modern, minimal design with cost basis visualization
 */

import { useEffect, useState, useMemo, useCallback, useRef } from 'react';
import {
  TrendingUp,
  Database,
  RefreshCw,
  ArrowUpRight,
  ArrowDownRight,
  Sparkles,
  Loader2,
  Brain,
  X,
  ChevronRight,
} from 'lucide-react';
import { createChart, AreaSeries, LineSeries } from 'lightweight-charts';
import type { IChartApi, ISeriesApi, LineData, AreaData, Time } from 'lightweight-charts';
import {
  useSettingsStore,
  useUIStore,
  toast,
  type AutoUpdateInterval,
  type ChartTimeRange,
} from '../../store';
import {
  usePortfolioAnalysisStore,
  getTrendColorClass,
  type AnalysisStatus,
} from '../../store/portfolioAnalysis';
import type { AggregatedHolding, PortfolioData } from '../types';
import { formatNumber } from '../utils';
import { getBaseCurrency, calculatePerformance } from '../../lib/api';
import { useQuoteSync } from '../../hooks/useQuoteSync';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { useCachedLogos } from '../../lib/hooks';
import { SecurityLogo } from '../../components/common/SecurityLogo';
import type { PerformanceResult } from '../../lib/types';
import { PortfolioInsightsModal } from '../../components/modals/PortfolioInsightsModal';

// Portfolio Insights Card Component
interface PortfolioInsightsCardProps {
  onOpenInsights: () => void;
}

function PortfolioInsightsCard({ onOpenInsights }: PortfolioInsightsCardProps) {
  const { setCurrentView } = useUIStore();
  const {
    aiEnabled,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
    openrouterApiKey,
  } = useSettingsStore();

  // Check if AI is configured (has at least one API key)
  const hasAnyAiApiKey = !!(anthropicApiKey || openaiApiKey || geminiApiKey || perplexityApiKey || openrouterApiKey);
  const aiConfigured = aiEnabled && hasAnyAiApiKey;

  if (!aiConfigured) {
    return (
      <button
        onClick={() => setCurrentView('settings')}
        className="card-interactive p-3 min-w-[140px] flex flex-col items-center justify-center gap-2"
        title="KI-Funktionen konfigurieren"
      >
        <div className="p-2 rounded-full bg-muted">
          <Sparkles size={16} className="text-muted-foreground" />
        </div>
        <span className="text-[10px] text-muted-foreground text-center">
          KI nicht konfiguriert
        </span>
        <span className="text-[9px] text-primary flex items-center gap-0.5">
          Einrichten <ChevronRight size={10} />
        </span>
      </button>
    );
  }

  return (
    <button
      onClick={onOpenInsights}
      className="card-interactive p-3 min-w-[140px] flex flex-col items-center justify-center gap-2 group"
      title="Portfolio Insights anzeigen"
    >
      <div className="p-2 rounded-full bg-primary/10 group-hover:bg-primary/20 transition-colors">
        <Sparkles size={16} className="text-primary" />
      </div>
      <span className="text-[11px] font-medium text-center">
        Portfolio Insights
      </span>
      <span className="text-[9px] text-muted-foreground flex items-center gap-0.5">
        Analyse starten <ChevronRight size={10} />
      </span>
    </button>
  );
}

interface DashboardViewProps {
  dbHoldings: AggregatedHolding[];
  dbPortfolios: PortfolioData[];
  dbPortfolioHistory: Array<{ date: string; value: number }>;
  dbInvestedCapitalHistory: Array<{ date: string; value: number }>;
  onImportPP: () => void;
  onRefresh?: () => void;
}

// Trend Indicator component for AI analysis status
function TrendIndicator({
  status,
  summary,
}: {
  status: AnalysisStatus | undefined;
  summary?: string;
}) {
  if (!status) return null;

  const colorClass = getTrendColorClass(status);
  const title =
    status === 'bullish'
      ? `Bullish${summary ? `: ${summary}` : ''}`
      : status === 'bearish'
      ? `Bearish${summary ? `: ${summary}` : ''}`
      : status === 'neutral'
      ? `Neutral${summary ? `: ${summary}` : ''}`
      : status === 'pending'
      ? 'Analyse läuft...'
      : status === 'error'
      ? 'Analyse fehlgeschlagen'
      : '';

  return (
    <div
      className={`w-2.5 h-2.5 rounded-full shrink-0 ${colorClass}`}
      title={title}
    />
  );
}

// Chart mode toggle type
type ChartMode = 'current' | 'history';

// Main Chart with Cost Basis Line (TradingView style)
function PortfolioChart({
  portfolioData,
  investedCapitalData,
  timeRange,
  onTimeRangeChange,
  currency,
  currentCostBasis,
}: {
  portfolioData: Array<{ date: string; value: number }>;
  investedCapitalData: Array<{ date: string; value: number }>;
  timeRange: string;
  onTimeRangeChange: (range: '1W' | '1M' | '3M' | '6M' | 'YTD' | '1Y' | '3Y' | '5Y' | 'MAX') => void;
  currency: string;
  currentCostBasis?: number;
}) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const valueSeriesRef = useRef<ISeriesApi<'Area'> | null>(null);
  const costBasisSeriesRef = useRef<ISeriesApi<'Line'> | ISeriesApi<'Area'> | null>(null);

  // Chart mode: 'current' = current cost basis line, 'history' = cost basis over time
  const [chartMode, setChartMode] = useState<ChartMode>('current');

  // Use actual current cost basis if provided
  const displayCostBasis = currentCostBasis ?? 0;

  // Initialize lightweight-charts
  useEffect(() => {
    if (!chartContainerRef.current || portfolioData.length === 0) {
      return;
    }

    // Clean up existing chart
    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
    }

    const container = chartContainerRef.current;

    const isDark = document.documentElement.classList.contains('dark');

    // Create chart with theme-aware colors
    const chart = createChart(container, {
      layout: {
        background: { color: 'transparent' },
        textColor: isDark ? '#9ca3af' : '#6b7280',
        fontFamily: 'Geist Mono, ui-monospace, monospace',
      },
      grid: {
        vertLines: { color: 'rgba(128, 128, 128, 0.08)' },
        horzLines: { color: 'rgba(128, 128, 128, 0.08)' },
      },
      width: container.clientWidth,
      height: container.clientHeight,
      rightPriceScale: {
        borderColor: 'rgba(128, 128, 128, 0.15)',
        scaleMargins: {
          top: 0.1,
          bottom: 0,
        },
      },
      timeScale: {
        borderColor: 'rgba(128, 128, 128, 0.15)',
        timeVisible: true,
        rightOffset: 12,
      },
      crosshair: {
        mode: 1,
      },
    });

    chartRef.current = chart;

    // Portfolio value area series (green gradient)
    const valueSeries = chart.addSeries(AreaSeries, {
      lineColor: '#22c55e',
      topColor: 'rgba(34, 197, 94, 0.4)',
      bottomColor: 'rgba(34, 197, 94, 0.0)',
      lineWidth: 2,
      autoscaleInfoProvider: (original: () => { priceRange: { minValue: number; maxValue: number } } | null) => {
        const res = original();
        if (res !== null) {
          res.priceRange.minValue = 0;
        }
        return res;
      },
    });
    valueSeriesRef.current = valueSeries;

    // Prepare portfolio value data
    const valueData: AreaData<Time>[] = portfolioData.map((point) => ({
      time: point.date as Time,
      value: point.value,
    }));
    valueSeries.setData(valueData);

    if (chartMode === 'current') {
      // Mode 1: Current cost basis as horizontal dashed line
      const costBasisSeries = chart.addSeries(LineSeries, {
        color: '#f97316',
        lineWidth: 2,
        lineStyle: 2, // Dashed
      });
      costBasisSeriesRef.current = costBasisSeries;

      if (portfolioData.length > 0 && displayCostBasis > 0) {
        const costBasisData: LineData<Time>[] = [
          { time: portfolioData[0].date as Time, value: displayCostBasis },
          { time: portfolioData[portfolioData.length - 1].date as Time, value: displayCostBasis },
        ];
        costBasisSeries.setData(costBasisData);
      }
    } else {
      // Mode 2: Cost basis over time as area chart
      const costBasisSeries = chart.addSeries(AreaSeries, {
        lineColor: '#f97316',
        topColor: 'rgba(249, 115, 22, 0.3)',
        bottomColor: 'rgba(249, 115, 22, 0.0)',
        lineWidth: 2,
      });
      costBasisSeriesRef.current = costBasisSeries;

      // Merge invested capital data with portfolio dates
      // Create a map of invested capital by date
      const investedMap = new Map(investedCapitalData.map(d => [d.date, d.value]));

      // For each portfolio date, find the corresponding or last known invested capital
      let lastInvestedValue = 0;
      const costBasisHistoryData: AreaData<Time>[] = portfolioData.map((point) => {
        if (investedMap.has(point.date)) {
          lastInvestedValue = investedMap.get(point.date)!;
        }
        return {
          time: point.date as Time,
          value: lastInvestedValue,
        };
      });

      costBasisSeries.setData(costBasisHistoryData);
    }

    // Fit content
    chart.timeScale().fitContent();

    // Handle resize
    const handleResize = () => {
      if (chartRef.current && container) {
        chartRef.current.applyOptions({
          width: container.clientWidth,
          height: container.clientHeight,
        });
      }
    };

    const resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
      }
    };
  }, [portfolioData, investedCapitalData, displayCostBasis, chartMode]);

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-4 text-[10px] text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <span className="w-3 h-[3px] rounded-full bg-positive" />
            Marktwert
          </span>
          {chartMode === 'current' ? (
            <span className="flex items-center gap-1.5">
              <span className="w-3 h-0 border-t-2 border-dashed border-orange-500" />
              Einstandswert ({formatNumber(displayCostBasis)} {currency})
            </span>
          ) : (
            <span className="flex items-center gap-1.5">
              <span className="w-3 h-[3px] rounded-full bg-orange-500" />
              Einstandswert (Verlauf)
            </span>
          )}
          {/* Toggle Button */}
          <button
            onClick={() => setChartMode(chartMode === 'current' ? 'history' : 'current')}
            className="ml-2 px-2 py-0.5 text-[9px] font-medium rounded border border-muted-foreground/30 hover:bg-muted transition-colors"
            title={chartMode === 'current' ? 'Einstandsverlauf anzeigen' : 'Aktuellen Einstand anzeigen'}
          >
            {chartMode === 'current' ? 'Verlauf' : 'Aktuell'}
          </button>
        </div>
        <div className="flex gap-1">
          {(['1W', '1M', '3M', '6M', 'YTD', '1Y', '3Y', '5Y', 'MAX'] as const).map((range) => (
            <button
              key={range}
              onClick={() => onTimeRangeChange(range)}
              className={`px-2 py-1 text-[10px] font-medium rounded-md transition-all font-mono ${
                timeRange === range
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted'
              }`}
            >
              {range}
            </button>
          ))}
        </div>
      </div>

      {/* Chart */}
      <div className="flex-1 min-h-0">
        {portfolioData.length > 0 ? (
          <div ref={chartContainerRef} className="h-full w-full" />
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
  dbHoldings,
  dbPortfolioHistory,
  dbInvestedCapitalHistory,
  onImportPP,
  onRefresh,
}: DashboardViewProps) {
  const { keys: secureKeys } = useSecureApiKeys();
  const syncOnlyHeldSecurities = useSettingsStore((state) => state.syncOnlyHeldSecurities);
  const autoUpdateInterval = useSettingsStore((state) => state.autoUpdateInterval);
  const setAutoUpdateInterval = useSettingsStore((state) => state.setAutoUpdateInterval);

  // Portfolio AI Analysis Store
  const {
    analyses,
    isAnalyzing: isBatchAnalyzing,
    progress: batchProgress,
    lastBatchRun,
    clearAllAnalyses,
  } = usePortfolioAnalysisStore();

  const [baseCurrency, setBaseCurrency] = useState<string>('EUR');
  const [performance, setPerformance] = useState<PerformanceResult | null>(null);
  const defaultChartTimeRange = useSettingsStore((state) => state.defaultChartTimeRange);
  const [chartTimeRange, setChartTimeRange] = useState<ChartTimeRange>(defaultChartTimeRange);
  const [isSyncing, setIsSyncing] = useState(false);
  const [showInsightsModal, setShowInsightsModal] = useState(false);
  const lastSyncTime = useSettingsStore((state) => state.lastSyncTime);
  const setLastSyncTime = useSettingsStore((state) => state.setLastSyncTime);
  const [nextSyncSeconds, setNextSyncSeconds] = useState<number | null>(null);
  const [syncStatus, setSyncStatus] = useState<string | null>(null);
  const { startSync } = useQuoteSync();
  const handleSyncQuotes = useCallback(async () => {
    if (isSyncing) return;
    setIsSyncing(true);
    setSyncStatus('Sync...');
    try {
      const apiKeys = {
        finnhub: secureKeys.finnhubApiKey || undefined,
        coingecko: secureKeys.coingeckoApiKey || undefined,
        alphaVantage: secureKeys.alphaVantageApiKey || undefined,
        twelveData: secureKeys.twelveDataApiKey || undefined,
      };
      await startSync(syncOnlyHeldSecurities, apiKeys);
      setLastSyncTime(new Date());
      onRefresh?.();
      setSyncStatus(null);
    } catch (err) {
      setSyncStatus(`Sync fehlgeschlagen: ${err}`);
      setTimeout(() => setSyncStatus(null), 5000);
    } finally {
      setIsSyncing(false);
    }
  }, [
    isSyncing,
    secureKeys,
    syncOnlyHeldSecurities,
    onRefresh,
    setLastSyncTime,
    startSync,
  ]);


  // Auto-sync timer: triggers sync when countdown reaches 0
  useEffect(() => {
    if (autoUpdateInterval === 0) {
      setNextSyncSeconds(null);
      return;
    }

    // If no lastSyncTime yet, set it to now (first run)
    if (!lastSyncTime) {
      setLastSyncTime(new Date());
      return;
    }

    const calculateRemaining = () => {
      const lastSync = new Date(lastSyncTime).getTime();
      const nextSync = lastSync + autoUpdateInterval * 60 * 1000;
      const now = Date.now();
      const remaining = Math.max(0, Math.floor((nextSync - now) / 1000));
      return remaining;
    };

    // Update countdown every second
    const countdownInterval = setInterval(() => {
      const remaining = calculateRemaining();
      setNextSyncSeconds(remaining);

      // Trigger sync when countdown reaches 0
      if (remaining === 0 && !isSyncing) {
        handleSyncQuotes();
      }
    }, 1000);

    // Initial calculation
    setNextSyncSeconds(calculateRemaining());

    return () => clearInterval(countdownInterval);
  }, [autoUpdateInterval, lastSyncTime, isSyncing, handleSyncQuotes, setLastSyncTime]);

  const securitiesForLogos = useMemo(
    () =>
      dbHoldings.map((h) => ({
        id: h.securityIds[0],
        ticker: undefined,
        name: h.name || '',
      })),
    [dbHoldings]
  );

  const { logos: cachedLogos } = useCachedLogos(securitiesForLogos, secureKeys.brandfetchApiKey);

  useEffect(() => {
    getBaseCurrency()
      .then(setBaseCurrency)
      .catch(() => setBaseCurrency('EUR'));
  }, []);

  // Calculate start date based on time range for performance metrics
  const performanceStartDate = useMemo(() => {
    const now = new Date();
    switch (chartTimeRange) {
      case '1W':
        return new Date(new Date().setDate(now.getDate() - 7)).toISOString().split('T')[0];
      case '1M':
        return new Date(new Date().setMonth(now.getMonth() - 1)).toISOString().split('T')[0];
      case '3M':
        return new Date(new Date().setMonth(now.getMonth() - 3)).toISOString().split('T')[0];
      case '6M':
        return new Date(new Date().setMonth(now.getMonth() - 6)).toISOString().split('T')[0];
      case 'YTD':
        return new Date(now.getFullYear(), 0, 1).toISOString().split('T')[0];
      case '1Y':
        return new Date(new Date().setFullYear(now.getFullYear() - 1)).toISOString().split('T')[0];
      case '3Y':
        return new Date(new Date().setFullYear(now.getFullYear() - 3)).toISOString().split('T')[0];
      case '5Y':
        return new Date(new Date().setFullYear(now.getFullYear() - 5)).toISOString().split('T')[0];
      case 'MAX':
      default:
        return undefined; // Use default (first transaction date)
    }
  }, [chartTimeRange]);

  useEffect(() => {
    if (dbHoldings.length > 0) {
      calculatePerformance({ startDate: performanceStartDate })
        .then(setPerformance)
        .catch(() => setPerformance(null));
    }
  }, [dbHoldings, performanceStartDate]);

  const { filteredChartData, filteredInvestedData } = useMemo(() => {
    if (dbPortfolioHistory.length === 0) {
      return { filteredChartData: [], filteredInvestedData: [] };
    }

    const now = new Date();
    let cutoffDate: Date;
    switch (chartTimeRange) {
      case '1W':
        cutoffDate = new Date(new Date().setDate(now.getDate() - 7));
        break;
      case '1M':
        cutoffDate = new Date(new Date().setMonth(now.getMonth() - 1));
        break;
      case '3M':
        cutoffDate = new Date(new Date().setMonth(now.getMonth() - 3));
        break;
      case '6M':
        cutoffDate = new Date(new Date().setMonth(now.getMonth() - 6));
        break;
      case 'YTD':
        cutoffDate = new Date(now.getFullYear(), 0, 1); // January 1st of current year
        break;
      case '1Y':
        cutoffDate = new Date(new Date().setFullYear(now.getFullYear() - 1));
        break;
      case '3Y':
        cutoffDate = new Date(new Date().setFullYear(now.getFullYear() - 3));
        break;
      case '5Y':
        cutoffDate = new Date(new Date().setFullYear(now.getFullYear() - 5));
        break;
      case 'MAX':
      default:
        // Start from first investment date
        if (dbInvestedCapitalHistory.length > 0) {
          const firstInvestmentDate = new Date(dbInvestedCapitalHistory[0].date);
          return {
            filteredChartData: dbPortfolioHistory.filter((d) => new Date(d.date) >= firstInvestmentDate),
            filteredInvestedData: dbInvestedCapitalHistory,
          };
        }
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

  // Calculate derived values for widgets
  const totalValue = dbHoldings.reduce((sum, h) => sum + (h.currentValue || 0), 0);
  const totalCostBasis = dbHoldings.reduce((sum, h) => sum + h.costBasis, 0);
  const totalGainLoss = totalValue - totalCostBasis;
  const totalGainLossPercent = totalCostBasis > 0 ? (totalGainLoss / totalCostBasis) * 100 : 0;

  // Main dashboard with holdings
  if (dbHoldings.length > 0) {
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
        {/* Top Row - Hero + Metrics */}
        <div className="flex gap-2 flex-shrink-0">
          {/* Portfolio Value - Hero Card */}
          <div
            className="card-elevated p-5 flex-[2] min-w-[280px] cursor-default animate-fade-up"
            style={{ background: 'var(--gradient-subtle)' }}
            title="Gesamtwert deines Portfolios

Der aktuelle Marktwert aller deiner Wertpapiere basierend auf den letzten verfügbaren Kursen.

Berechnung:
• Summe aller Positionen × aktuelle Kurse
• Ohne Kontoguthaben

Der Gewinn/Verlust zeigt die Differenz zum Einstand (Anschaffungskosten)."
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-label text-muted-foreground">
                Portfolio
              </span>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleSyncQuotes()}
                  disabled={isSyncing}
                  className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-muted/50 hover:bg-muted transition-colors text-muted-foreground hover:text-foreground"
                  title="Kurse aktualisieren"
                >
                  <RefreshCw
                    size={12}
                    className={isSyncing ? 'animate-spin text-primary' : ''}
                  />
                  <span className="text-[10px] font-medium font-mono tabular-nums">
                    {isSyncing ? 'Sync...' : lastSyncTime ? new Date(lastSyncTime).toLocaleTimeString('de-DE', {
                      hour: '2-digit',
                      minute: '2-digit',
                    }) : 'Sync'}
                  </span>
                </button>
              </div>
            </div>
            <div className="text-display font-mono tabular-nums">
              {formatNumber(totalValue)}
              <span className="text-base text-muted-foreground ml-1 font-sans">{baseCurrency}</span>
            </div>
            <div className="flex items-center gap-2 mt-1">
              <span
                className={`inline-flex items-center gap-0.5 text-sm font-medium font-mono tabular-nums ${
                  totalGainLoss >= 0 ? 'text-positive' : 'text-negative'
                }`}
              >
                {totalGainLoss >= 0 ? <ArrowUpRight size={14} /> : <ArrowDownRight size={14} />}
                {totalGainLoss >= 0 ? '+' : ''}
                {formatNumber(totalGainLoss)}
              </span>
              <span
                className={`text-xs px-1.5 py-0.5 rounded font-mono tabular-nums ${
                  totalGainLossPercent >= 0
                    ? 'bg-positive-muted text-positive'
                    : 'bg-negative-muted text-negative'
                }`}
              >
                {totalGainLossPercent >= 0 ? '+' : ''}
                {totalGainLossPercent.toFixed(2)}%
              </span>
            </div>
          </div>

          {/* Metric Cards — staggered entrance */}
          <div
            className="card-surface p-3 px-4 min-w-[100px] cursor-default animate-fade-up-delay-1"
            title="Tagesperformance

Zeigt die Wertänderung deines Portfolios seit dem letzten Handelstag.

Berechnung:
• Aktueller Wert minus Wert am Vortag
• Prozentuale Änderung zum Vortag

Hinweis: Berücksichtigt nur Kursänderungen, keine Ein-/Auszahlungen am selben Tag."
          >
            <span className="text-label text-muted-foreground block mb-1">
              Heute
            </span>
            <div
              className={`text-headline font-mono tabular-nums ${
                dailyChange >= 0 ? 'text-positive' : 'text-negative'
              }`}
            >
              {dailyChangePercent >= 0 ? '+' : ''}
              {dailyChangePercent.toFixed(2)}%
            </div>
            <div
              className={`text-[10px] font-mono tabular-nums ${dailyChange >= 0 ? 'text-positive/70' : 'text-negative/70'}`}
            >
              {dailyChange >= 0 ? '+' : ''}
              {formatNumber(dailyChange)}
            </div>
          </div>

          <div
            className="card-surface p-3 px-4 min-w-[100px] cursor-default animate-fade-up-delay-2"
            title="TTWROR (True Time-Weighted Rate of Return)

Misst die reine Anlageperformance unabhängig von Ein- und Auszahlungen.

Gut geeignet um:
• Deine Anlageentscheidungen zu bewerten
• Mit Benchmarks (z.B. MSCI World) zu vergleichen
• Fondsmanager zu vergleichen

Beispiel: Wenn du 1.000€ investierst und der Markt um 10% steigt, ist der TTWROR +10% - egal wann du das Geld eingezahlt hast."
          >
            <span className="text-label text-muted-foreground block mb-1">
              TTWROR
            </span>
            <div
              className={`text-headline font-mono tabular-nums ${
                (performance?.ttwror ?? 0) >= 0 ? 'text-positive' : 'text-negative'
              }`}
            >
              {performance?.ttwror != null
                ? `${performance.ttwror >= 0 ? '+' : ''}${performance.ttwror.toFixed(1)}%`
                : '—'}
            </div>
            <div className="text-[10px] text-muted-foreground">Zeitgewichtet</div>
          </div>

          <div
            className="card-surface p-3 px-4 min-w-[100px] cursor-default animate-fade-up-delay-3"
            title="IRR (Internal Rate of Return / Interner Zinsfuß)

Misst deine persönliche Rendite unter Berücksichtigung WANN du Geld ein- oder ausgezahlt hast.

Gut geeignet um:
• Deine tatsächliche Vermögensentwicklung zu sehen
• Mit Festgeld/Tagesgeld zu vergleichen
• Den Effekt von Market Timing zu erkennen

Beispiel: Wenn du vor einem Crash mehr investiert hast, ist dein IRR niedriger als der TTWROR - und umgekehrt bei gutem Timing."
          >
            <span className="text-label text-muted-foreground block mb-1">
              IRR
            </span>
            <div
              className={`text-headline font-mono tabular-nums ${
                (performance?.irr ?? 0) >= 0 ? 'text-positive' : 'text-negative'
              }`}
            >
              {performance?.irr != null
                ? `${performance.irr >= 0 ? '+' : ''}${performance.irr.toFixed(1)}%`
                : '—'}
            </div>
            <div className="text-[10px] text-muted-foreground">Kapitalgewichtet</div>
          </div>

          <div
            className="card-surface p-3 px-4 min-w-[100px] cursor-default animate-fade-up-delay-4"
            title="Einstand (Cost Basis)

Deine gesamten Anschaffungskosten nach der FIFO-Methode (First In, First Out).

Beinhaltet:
• Kaufpreise aller Positionen
• Transaktionsgebühren
• Steuern beim Kauf

Verwendung:
• Gewinn/Verlust = Depotwert − Einstand
• Basis für steuerliche Berechnung bei Verkauf"
          >
            <span className="text-label text-muted-foreground block mb-1">
              Einstand
            </span>
            <div className="text-headline font-mono tabular-nums">{formatNumber(totalCostBasis)}</div>
            <div className="text-[10px] text-muted-foreground">{baseCurrency}</div>
          </div>

          {/* Top 3 Performer */}
          <div
            className="card-surface p-3 px-4 min-w-[180px] cursor-default animate-fade-up-delay-4"
            title="Top 3 Performer

Die drei Positionen mit der besten prozentualen Performance (unrealisierter Gewinn/Verlust).

Berechnung:
• (Aktueller Wert − Einstand) / Einstand × 100%
• Basierend auf FIFO-Einstandskursen"
          >
            <span className="text-label text-muted-foreground block mb-2">
              Top Performer
            </span>
            <div className="space-y-1.5">
              {[...dbHoldings]
                .filter((h) => h.gainLossPercent != null)
                .sort((a, b) => (b.gainLossPercent || 0) - (a.gainLossPercent || 0))
                .slice(0, 3)
                .map((holding, index) => {
                  const gainPercent = holding.gainLossPercent || 0;
                  return (
                    <div key={holding.securityIds[0]} className="flex items-center gap-2">
                      <span className="text-[10px] text-muted-foreground w-3 font-mono">{index + 1}.</span>
                      <SecurityLogo securityId={holding.securityIds[0]} logos={cachedLogos} size={20} customLogoUrl={holding.customLogo} />
                      <span className="text-[11px] font-medium truncate flex-1 max-w-[80px]">
                        {holding.name}
                      </span>
                      <span className={`text-[11px] font-semibold font-mono tabular-nums ${gainPercent >= 0 ? 'text-positive' : 'text-negative'}`}>
                        {gainPercent >= 0 ? '+' : ''}{gainPercent.toFixed(1)}%
                      </span>
                    </div>
                  );
                })}
            </div>
          </div>

          {/* Portfolio Insights Card */}
          <PortfolioInsightsCard
            onOpenInsights={() => setShowInsightsModal(true)}
          />

          {/* Auto-Update */}
          <div
            className="card-surface p-3 px-4 flex flex-col justify-between min-w-[110px] cursor-default animate-fade-up-delay-4"
            title="Automatische Kursaktualisierung

Lädt aktuelle Kurse für deine Wertpapiere automatisch im gewählten Intervall.

Quellen:
• Yahoo Finance (kostenlos)
• Finnhub (mit API-Key)
• CoinGecko (Krypto)
• EZB (Wechselkurse)

Tipp: API-Keys in den Einstellungen hinterlegen für bessere Abdeckung."
          >
            <div className="flex items-center gap-1.5">
              {isSyncing ? (
                <Loader2 size={12} className="text-primary animate-spin" />
              ) : (
                <Sparkles size={12} className="text-muted-foreground" />
              )}
              <span className="text-[10px] text-muted-foreground">Auto-Sync</span>
            </div>
            <select
              value={autoUpdateInterval}
              onChange={(e) => setAutoUpdateInterval(Number(e.target.value) as AutoUpdateInterval)}
              className="bg-transparent border-none text-sm font-medium text-foreground focus:outline-none cursor-pointer -ml-1"
              disabled={isSyncing}
            >
              <option value={0}>Aus</option>
              <option value={15}>15 Min</option>
              <option value={30}>30 Min</option>
              <option value={60}>1 Std</option>
            </select>
            {isSyncing ? (
              <div className="text-[10px] text-primary font-medium">
                Synchronisiere...
              </div>
            ) : nextSyncSeconds !== null && nextSyncSeconds > 0 ? (
              <div className="text-[10px] text-muted-foreground font-mono tabular-nums">
                Nächste: {Math.floor(nextSyncSeconds / 60)}:{(nextSyncSeconds % 60).toString().padStart(2, '0')}
              </div>
            ) : null}
          </div>
        </div>

        {/* Sync Status Banner */}
        {syncStatus && (
          <div className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            syncStatus.includes('Fehler') || syncStatus.includes('fehlgeschlagen')
              ? 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border border-amber-500/20'
              : syncStatus.includes('Lade')
              ? 'bg-primary/10 text-primary border border-primary/20'
              : 'bg-positive-muted text-positive border border-positive/20'
          }`}>
            {syncStatus.includes('Lade') && (
              <Loader2 size={12} className="animate-spin" />
            )}
            {syncStatus}
          </div>
        )}

        {/* Main Content - Chart + Holdings */}
        <div className="flex-1 flex gap-2 min-h-0 overflow-hidden">
          {/* Chart */}
          <div className="flex-1 card-elevated p-5 min-w-0">
            <PortfolioChart
              portfolioData={filteredChartData}
              investedCapitalData={filteredInvestedData}
              timeRange={chartTimeRange}
              onTimeRangeChange={setChartTimeRange}
              currency={baseCurrency}
              currentCostBasis={totalCostBasis}
            />
          </div>

          {/* Holdings Sidebar */}
          <div className="w-[360px] card-surface p-4 flex flex-col flex-shrink-0">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-label text-muted-foreground">
                  Positionen
                </span>
                {/* AI Analysis Button */}
                <button
                  onClick={() => {
                    toast.info('Öffne die Technische Analyse (Charts), um KI-Analysen für einzelne Wertpapiere zu erstellen. Die Ergebnisse werden hier als Trend-Ampel angezeigt.');
                  }}
                  disabled={isBatchAnalyzing}
                  className="p-1 rounded hover:bg-muted/50 transition-colors group/brain"
                  title={
                    isBatchAnalyzing
                      ? `Analysiere ${batchProgress.current}/${batchProgress.total}...`
                      : lastBatchRun
                      ? `Letzte Analyse: ${new Date(lastBatchRun).toLocaleString('de-DE')}`
                      : 'KI-Trend-Analyse'
                  }
                >
                  {isBatchAnalyzing ? (
                    <Loader2 size={12} className="animate-spin text-primary" />
                  ) : (
                    <Brain size={12} className="text-muted-foreground group-hover/brain:text-primary transition-colors" />
                  )}
                </button>
                {/* Clear analyses button */}
                {Object.keys(analyses).length > 0 && !isBatchAnalyzing && (
                  <button
                    onClick={() => {
                      clearAllAnalyses();
                      toast.info('Alle Trend-Analysen zurückgesetzt');
                    }}
                    className="p-1 rounded hover:bg-muted/50 transition-colors"
                    title="Alle Analysen löschen"
                  >
                    <X size={10} className="text-muted-foreground hover:text-destructive" />
                  </button>
                )}
              </div>
              <div className="flex items-center gap-2">
                {/* Batch progress indicator */}
                {isBatchAnalyzing && (
                  <span className="text-[9px] text-primary font-mono tabular-nums">
                    {batchProgress.current}/{batchProgress.total}
                  </span>
                )}
                <span className="text-[10px] text-muted-foreground font-mono tabular-nums">{dbHoldings.length} Titel</span>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto -mx-4 px-4 space-y-0.5">
              {holdingsByValue.map((holding) => {
                const percent =
                  totalValue > 0 ? ((holding.currentValue || 0) / totalValue) * 100 : 0;
                const gainPercent = holding.gainLossPercent || 0;
                const isPositive = gainPercent >= 0;

                // Get AI analysis for this holding
                const analysis = analyses[holding.securityIds[0]];

                return (
                  <div
                    key={holding.securityIds[0]}
                    className="flex items-center gap-2 py-1.5 px-2 -mx-2 rounded-lg hover:bg-muted/30 transition-all duration-150 cursor-pointer group"
                  >
                    <SecurityLogo securityId={holding.securityIds[0]} logos={cachedLogos} size={28} customLogoUrl={holding.customLogo} />
                    <div className="flex-1 min-w-0">
                      <div className="text-xs font-medium truncate group-hover:text-primary transition-colors">
                        {holding.name}
                      </div>
                      <div className="text-[10px] text-muted-foreground font-mono tabular-nums">{percent.toFixed(1)}%</div>
                    </div>
                    {/* AI Trend Indicator */}
                    <TrendIndicator status={analysis?.trend} summary={analysis?.summary} />
                    <div className="text-right min-w-[65px]">
                      <div className="text-xs font-medium font-mono tabular-nums">
                        {formatNumber(holding.currentValue || 0)}
                      </div>
                      <div
                        className={`text-[10px] font-mono tabular-nums ${isPositive ? 'text-positive' : 'text-negative'}`}
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

        {/* Portfolio Insights Modal */}
        <PortfolioInsightsModal
          isOpen={showInsightsModal}
          onClose={() => setShowInsightsModal(false)}
        />
      </div>
    );
  }

  // Welcome screen (no holdings yet)
  return (
    <div className="h-full flex items-center justify-center">
      <div className="text-center max-w-sm animate-fade-up">
        <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center mx-auto mb-6 backdrop-blur-sm border border-primary/10">
          <TrendingUp className="w-8 h-8 text-primary" />
        </div>
        <h2 className="text-xl font-light mb-2 tracking-tight">Portfolio Now</h2>
        <p className="text-sm text-muted-foreground mb-8">
          Importiere deine Portfolio Performance Datei, um zu starten
        </p>
        <button
          onClick={onImportPP}
          className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors mx-auto dark:glow-primary"
        >
          <Database size={16} />
          PP-Datei importieren
        </button>
      </div>
    </div>
  );
}
