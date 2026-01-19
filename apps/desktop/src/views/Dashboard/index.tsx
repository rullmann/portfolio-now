/**
 * Dashboard - Full-width Bento Grid Layout
 * Modern, minimal design with cost basis visualization
 */

import { useEffect, useState, useMemo, useCallback } from 'react';
import {
  TrendingUp,
  Database,
  Building2,
  RefreshCw,
  ArrowUpRight,
  ArrowDownRight,
  Sparkles,
  Loader2,
  Brain,
  X,
  BarChart3,
  Lightbulb,
  MessageSquare,
  FileText,
  FileSpreadsheet,
  ChevronRight,
  Settings,
} from 'lucide-react';
import { AIProviderLogo } from '../../components/common/AIProviderLogo';
import {
  useSettingsStore,
  useUIStore,
  toast,
  type AutoUpdateInterval,
  AI_FEATURES,
  AI_MODELS,
  type AiProvider,
} from '../../store';
import {
  usePortfolioAnalysisStore,
  getTrendColorClass,
  type AnalysisStatus,
} from '../../store/portfolioAnalysis';
import type { AggregatedHolding, PortfolioData } from '../types';
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
  CartesianGrid,
} from 'recharts';
import { PortfolioInsightsModal } from '../../components/modals/PortfolioInsightsModal';
import { PdfImportModal } from '../../components/modals/PdfImportModal';
import { CsvImportModal } from '../../components/modals/CsvImportModal';

// Feature icon mapping
const FEATURE_ICONS: Record<string, typeof BarChart3> = {
  BarChart3,
  Lightbulb,
  MessageSquare,
  FileText,
  FileSpreadsheet,
};

// AI Features Card Component
interface AiFeaturesCardProps {
  onOpenInsights: () => void;
  onOpenChat: () => void;
  onOpenPdfImport: () => void;
  onOpenCsvImport: () => void;
}

function AiFeaturesCard({ onOpenInsights, onOpenChat, onOpenPdfImport, onOpenCsvImport }: AiFeaturesCardProps) {
  const { setCurrentView } = useUIStore();
  const {
    aiEnabled,
    aiFeatureSettings,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
  } = useSettingsStore();

  // Check if AI is configured (has at least one API key)
  const hasAnyAiApiKey = !!(anthropicApiKey || openaiApiKey || geminiApiKey || perplexityApiKey);
  const aiConfigured = aiEnabled && hasAnyAiApiKey;

  // Get available providers
  const availableProviders: AiProvider[] = [];
  if (anthropicApiKey) availableProviders.push('claude');
  if (openaiApiKey) availableProviders.push('openai');
  if (geminiApiKey) availableProviders.push('gemini');
  if (perplexityApiKey) availableProviders.push('perplexity');

  // Handle feature click - navigate to appropriate view/action
  const handleFeatureClick = (featureId: string) => {
    switch (featureId) {
      case 'chartAnalysis':
        setCurrentView('charts');
        break;
      case 'portfolioInsights':
        onOpenInsights();
        break;
      case 'chatAssistant':
        onOpenChat();
        break;
      case 'pdfOcr':
        onOpenPdfImport();
        break;
      case 'csvImport':
        onOpenCsvImport();
        break;
      default:
        break;
    }
  };

  // Get model display name
  const getModelName = (provider: AiProvider, modelId: string) => {
    const models = AI_MODELS[provider] || [];
    const model = models.find((m) => m.id === modelId);
    return model?.name || modelId.split('-').slice(0, 2).join(' ');
  };

  if (!aiConfigured) {
    return (
      <button
        onClick={() => setCurrentView('settings')}
        className="glass-card p-3 min-w-[140px] flex flex-col items-center justify-center gap-2 hover:bg-muted/50 transition-colors cursor-pointer"
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

  // Show all features
  return (
    <div className="glass-card p-3 min-w-[220px] flex flex-col">
      <div className="flex items-center justify-between mb-2">
        <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
          KI-Funktionen
        </span>
        <button
          onClick={() => setCurrentView('settings')}
          className="p-1 rounded hover:bg-muted/50 transition-colors"
          title="KI-Einstellungen"
        >
          <Settings size={10} className="text-muted-foreground" />
        </button>
      </div>
      <div className="space-y-1 max-h-[140px] overflow-y-auto">
        {AI_FEATURES.map((feature) => {
          const config = aiFeatureSettings[feature.id];
          const isAvailable = availableProviders.includes(config?.provider);
          const Icon = FEATURE_ICONS[feature.icon] || Sparkles;

          return (
            <button
              key={feature.id}
              onClick={() => handleFeatureClick(feature.id)}
              disabled={!isAvailable}
              className={`w-full flex items-center gap-2 p-1.5 rounded-md transition-colors text-left ${
                isAvailable
                  ? 'hover:bg-muted/50 cursor-pointer'
                  : 'opacity-50 cursor-not-allowed'
              }`}
              title={isAvailable ? `${feature.name} öffnen` : 'Provider nicht konfiguriert'}
            >
              <Icon size={12} className="text-muted-foreground shrink-0" />
              <div className="flex-1 min-w-0">
                <div className="text-[11px] font-medium truncate">{feature.name}</div>
                {isAvailable && config && (
                  <div className="flex items-center gap-1">
                    <AIProviderLogo provider={config.provider} size={10} />
                    <span className="text-[9px] text-muted-foreground truncate">
                      {getModelName(config.provider, config.model)}
                    </span>
                  </div>
                )}
              </div>
              <ChevronRight size={10} className="text-muted-foreground shrink-0" />
            </button>
          );
        })}
      </div>
    </div>
  );
}

interface DashboardViewProps {
  dbHoldings: AggregatedHolding[];
  dbPortfolios: PortfolioData[];
  dbPortfolioHistory: Array<{ date: string; value: number }>;
  dbInvestedCapitalHistory: Array<{ date: string; value: number }>;
  onImportPP: () => void;
  onRefresh?: () => void;
  onOpenChat?: () => void;
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

// Main Chart with Invested Capital Line
function PortfolioChart({
  portfolioData,
  investedData,
  timeRange,
  onTimeRangeChange,
  currency,
  currentTotalValue,
  currentCostBasis,
}: {
  portfolioData: Array<{ date: string; value: number }>;
  investedData: Array<{ date: string; value: number }>;
  timeRange: string;
  onTimeRangeChange: (range: '1W' | '1M' | '3M' | '6M' | 'YTD' | '1Y' | '3Y' | '5Y' | 'MAX') => void;
  currency: string;
  currentTotalValue?: number;
  currentCostBasis?: number;
}) {
  // Merge portfolio and invested data by date
  const mergedData = useMemo(() => {
    const sortedInvested = [...investedData].sort((a, b) => a.date.localeCompare(b.date));

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

  // Use actual current values if provided, otherwise fall back to last chart values
  const displayValue = currentTotalValue ?? mergedData[mergedData.length - 1]?.value ?? 0;
  const displayCostBasis = currentCostBasis ?? mergedData[mergedData.length - 1]?.invested ?? 0;
  const isPositive = displayValue >= displayCostBasis;
  const gainLoss = displayValue - displayCostBasis;
  const gainLossPercent = displayCostBasis > 0 ? (gainLoss / displayCostBasis) * 100 : 0;

  // Calculate Y axis domain
  const allValues = mergedData.flatMap((d) => [d.value, d.invested]);
  const dataMin = Math.min(...allValues);
  const dataMax = Math.max(...allValues);
  const padding = (dataMax - dataMin) * 0.1;
  const yMin = Math.max(0, dataMin - padding);
  const yMax = dataMax + padding;

  // Check if data spans multiple years
  const spansMultipleYears = useMemo(() => {
    if (mergedData.length < 2) return false;
    const firstYear = new Date(mergedData[0].date).getFullYear();
    const lastYear = new Date(mergedData[mergedData.length - 1].date).getFullYear();
    return lastYear - firstYear >= 1;
  }, [mergedData]);

  // Custom tooltip
  const CustomTooltip = ({ active, payload, label }: { active?: boolean; payload?: Array<{ value: number; dataKey: string }>; label?: string }) => {
    if (!active || !payload || !label) return null;

    const portfolioValue = payload.find(p => p.dataKey === 'value')?.value || 0;
    const investedValue = payload.find(p => p.dataKey === 'invested')?.value || 0;
    const diff = portfolioValue - investedValue;
    const diffPercent = investedValue > 0 ? (diff / investedValue) * 100 : 0;
    const isProfit = diff >= 0;

    return (
      <div className="bg-popover/95 backdrop-blur-sm border border-border rounded-lg shadow-xl p-3 min-w-[180px]">
        <div className="text-[10px] text-muted-foreground mb-2">
          {new Date(label).toLocaleDateString('de-DE', {
            weekday: 'short',
            day: 'numeric',
            month: 'short',
            year: 'numeric',
          })}
        </div>
        <div className="space-y-1.5">
          <div className="flex items-center justify-between gap-4">
            <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className="w-2 h-2 rounded-full bg-emerald-500" />
              Depotwert
            </span>
            <span className="text-xs font-medium">
              {formatNumber(portfolioValue)} {currency}
            </span>
          </div>
          <div className="flex items-center justify-between gap-4">
            <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className="w-2 h-2 rounded-full bg-blue-400" />
              Investiert
            </span>
            <span className="text-xs font-medium">
              {formatNumber(investedValue)} {currency}
            </span>
          </div>
          <div className="border-t border-border pt-1.5 mt-1.5">
            <div className="flex items-center justify-between gap-4">
              <span className="text-xs text-muted-foreground">Gewinn/Verlust</span>
              <span className={`text-xs font-semibold ${isProfit ? 'text-emerald-500' : 'text-red-500'}`}>
                {isProfit ? '+' : ''}{formatNumber(diff)} {currency}
                <span className="ml-1 text-[10px] opacity-80">
                  ({isProfit ? '+' : ''}{diffPercent.toFixed(1)}%)
                </span>
              </span>
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-4">
          <div>
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
              Portfolio-Entwicklung
            </span>
            <div className="flex items-center gap-2 mt-0.5">
              <span className="text-lg font-semibold">{formatNumber(displayValue)} {currency}</span>
              <span className={`text-xs font-medium px-1.5 py-0.5 rounded ${
                isPositive ? 'bg-emerald-500/10 text-emerald-600' : 'bg-red-500/10 text-red-600'
              }`}>
                {isPositive ? '+' : ''}{formatNumber(gainLoss)} ({isPositive ? '+' : ''}{gainLossPercent.toFixed(1)}%)
              </span>
            </div>
          </div>
        </div>
        <div className="flex flex-col items-end gap-2">
          <div className="flex gap-1">
            {(['1W', '1M', '3M', '6M', 'YTD', '1Y', '3Y', '5Y', 'MAX'] as const).map((range) => (
              <button
                key={range}
                onClick={() => onTimeRangeChange(range)}
                className={`px-2 py-1 text-[10px] font-medium rounded-md transition-all ${
                  timeRange === range
                    ? 'bg-primary text-primary-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground hover:bg-muted'
                }`}
              >
                {range}
              </button>
            ))}
          </div>
          <div className="flex items-center gap-4 text-[10px] text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <span className="w-3 h-[3px] rounded-full bg-emerald-500" />
              Depotwert
            </span>
            <span className="flex items-center gap-1.5">
              <span className="w-3 h-[3px] rounded-full bg-blue-400 opacity-60" />
              Investiert
            </span>
          </div>
        </div>
      </div>

      {/* Chart */}
      <div className="flex-1 min-h-0">
        {mergedData.length > 0 ? (
          <ResponsiveContainer width="100%" height="100%">
            <ComposedChart data={mergedData} margin={{ top: 8, right: 12, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="portfolioGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#10b981" stopOpacity={0.25} />
                  <stop offset="50%" stopColor="#10b981" stopOpacity={0.08} />
                  <stop offset="100%" stopColor="#10b981" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="investedGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#60a5fa" stopOpacity={0.15} />
                  <stop offset="100%" stopColor="#60a5fa" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid
                strokeDasharray="3 3"
                stroke="hsl(var(--border))"
                strokeOpacity={0.5}
                vertical={false}
              />
              <XAxis
                dataKey="date"
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 9, fill: 'hsl(var(--muted-foreground))' }}
                tickFormatter={(value) => {
                  const d = new Date(value);
                  if (spansMultipleYears) {
                    return d.toLocaleDateString('de-DE', { month: 'short', year: '2-digit' });
                  }
                  return d.toLocaleDateString('de-DE', { day: 'numeric', month: 'short' });
                }}
                interval="preserveStartEnd"
                minTickGap={60}
                dy={8}
              />
              <YAxis
                axisLine={false}
                tickLine={false}
                tick={{ fontSize: 9, fill: 'hsl(var(--muted-foreground))' }}
                tickFormatter={(value) => {
                  if (value >= 1000000) return `${(value / 1000000).toFixed(1)}M`;
                  if (value >= 1000) return `${(value / 1000).toFixed(0)}k`;
                  return value.toFixed(0);
                }}
                width={48}
                domain={[yMin, yMax]}
              />
              <Tooltip content={<CustomTooltip />} cursor={{ stroke: 'hsl(var(--muted-foreground))', strokeWidth: 1, strokeDasharray: '4 4' }} />
              {/* Invested Capital Area - subtle blue */}
              <Area
                type="monotone"
                dataKey="invested"
                stroke="#60a5fa"
                strokeWidth={2}
                strokeOpacity={0.6}
                fill="url(#investedGradient)"
                dot={false}
                activeDot={{ r: 4, fill: '#60a5fa', stroke: '#fff', strokeWidth: 2 }}
              />
              {/* Portfolio Value Area - prominent green */}
              <Area
                type="monotone"
                dataKey="value"
                stroke="#10b981"
                strokeWidth={2.5}
                fill="url(#portfolioGradient)"
                dot={false}
                activeDot={{ r: 5, fill: '#10b981', stroke: '#fff', strokeWidth: 2 }}
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
  dbHoldings,
  dbPortfolioHistory,
  dbInvestedCapitalHistory,
  onImportPP,
  onRefresh,
  onOpenChat,
}: DashboardViewProps) {
  const brandfetchApiKey = useSettingsStore((state) => state.brandfetchApiKey);
  const finnhubApiKey = useSettingsStore((state) => state.finnhubApiKey);
  const coingeckoApiKey = useSettingsStore((state) => state.coingeckoApiKey);
  const alphaVantageApiKey = useSettingsStore((state) => state.alphaVantageApiKey);
  const twelveDataApiKey = useSettingsStore((state) => state.twelveDataApiKey);
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
  const [chartTimeRange, setChartTimeRange] = useState<'1W' | '1M' | '3M' | '6M' | 'YTD' | '1Y' | '3Y' | '5Y' | 'MAX'>('1Y');
  const [isSyncing, setIsSyncing] = useState(false);
  const [showInsightsModal, setShowInsightsModal] = useState(false);
  const [showPdfImportModal, setShowPdfImportModal] = useState(false);
  const [showCsvImportModal, setShowCsvImportModal] = useState(false);
  const lastSyncTime = useSettingsStore((state) => state.lastSyncTime);
  const setLastSyncTime = useSettingsStore((state) => state.setLastSyncTime);
  const [nextSyncSeconds, setNextSyncSeconds] = useState<number | null>(null);
  const [syncStatus, setSyncStatus] = useState<string | null>(null);
  const handleSyncQuotes = useCallback(async () => {
    if (isSyncing) return;
    setIsSyncing(true);
    setSyncStatus('Lade Kurse...');
    try {
      const apiKeys = {
        finnhub: finnhubApiKey || undefined,
        coingecko: coingeckoApiKey || undefined,
        alphaVantage: alphaVantageApiKey || undefined,
        twelveData: twelveDataApiKey || undefined,
      };
      const result = await syncAllPrices(syncOnlyHeldSecurities, apiKeys);
      setLastSyncTime(new Date());

      // Build status message
      let statusMsg = `${result.success} Kurse aktualisiert`;
      if (result.errors > 0) {
        statusMsg += `, ${result.errors} Fehler`;
      }
      setSyncStatus(statusMsg);

      // Show toast notification
      if (result.errors > 0) {
        toast.warning(statusMsg);
      } else {
        toast.success(statusMsg);
      }

      onRefresh?.();

      // Clear status after 3 seconds
      setTimeout(() => setSyncStatus(null), 3000);
    } catch (err) {
      const errorMsg = `Sync fehlgeschlagen: ${err}`;
      setSyncStatus(errorMsg);
      toast.error(errorMsg);
      setTimeout(() => setSyncStatus(null), 5000);
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
    onRefresh,
    setLastSyncTime,
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
        id: h.securityId,
        ticker: undefined,
        name: h.name || '',
      })),
    [dbHoldings]
  );

  const { logos: cachedLogos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

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

  // Main dashboard with holdings
  if (dbHoldings.length > 0) {
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
          <div
            className="glass-card p-4 flex-1 min-w-[280px] cursor-help"
            title="Gesamtwert Ihres Portfolios

Der aktuelle Marktwert aller Ihrer Wertpapiere basierend auf den letzten verfügbaren Kursen.

Berechnung:
• Summe aller Positionen × aktuelle Kurse
• Ohne Kontoguthaben

Der Gewinn/Verlust zeigt die Differenz zum Einstand (Anschaffungskosten)."
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
                Portfolio
              </span>
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
                <span className="text-[10px] font-medium">
                  {isSyncing ? 'Sync...' : lastSyncTime ? new Date(lastSyncTime).toLocaleTimeString('de-DE', {
                    hour: '2-digit',
                    minute: '2-digit',
                  }) : 'Sync'}
                </span>
              </button>
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
          <div
            className="glass-card p-3 min-w-[100px] cursor-help"
            title="Tagesperformance

Zeigt die Wertänderung Ihres Portfolios seit dem letzten Handelstag.

Berechnung:
• Aktueller Wert minus Wert am Vortag
• Prozentuale Änderung zum Vortag

Hinweis: Berücksichtigt nur Kursänderungen, keine Ein-/Auszahlungen am selben Tag."
          >
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

          <div
            className="glass-card p-3 min-w-[100px] cursor-help"
            title="TTWROR (True Time-Weighted Rate of Return)

Misst die reine Anlageperformance unabhängig von Ein- und Auszahlungen.

Gut geeignet um:
• Ihre Anlageentscheidungen zu bewerten
• Mit Benchmarks (z.B. MSCI World) zu vergleichen
• Fondsmanager zu vergleichen

Beispiel: Wenn Sie 1.000€ investieren und der Markt um 10% steigt, ist der TTWROR +10% - egal wann Sie das Geld eingezahlt haben."
          >
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

          <div
            className="glass-card p-3 min-w-[100px] cursor-help"
            title="IRR (Internal Rate of Return / Interner Zinsfuß)

Misst Ihre persönliche Rendite unter Berücksichtigung WANN Sie Geld ein- oder ausgezahlt haben.

Gut geeignet um:
• Ihre tatsächliche Vermögensentwicklung zu sehen
• Mit Festgeld/Tagesgeld zu vergleichen
• Den Effekt von Market Timing zu erkennen

Beispiel: Wenn Sie vor einem Crash mehr investiert haben, ist Ihr IRR niedriger als der TTWROR - und umgekehrt bei gutem Timing."
          >
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

          <div
            className="glass-card p-3 min-w-[100px] cursor-help"
            title="Einstand (Cost Basis)

Ihre gesamten Anschaffungskosten nach der FIFO-Methode (First In, First Out).

Beinhaltet:
• Kaufpreise aller Positionen
• Transaktionsgebühren
• Steuern beim Kauf

Verwendung:
• Gewinn/Verlust = Depotwert − Einstand
• Basis für steuerliche Berechnung bei Verkauf"
          >
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground block mb-1">
              Einstand
            </span>
            <div className="text-xl font-medium">{formatNumber(totalCostBasis)}</div>
            <div className="text-[10px] text-muted-foreground">{baseCurrency}</div>
          </div>

          {/* Top 3 Performer */}
          <div
            className="glass-card p-3 min-w-[180px] cursor-help"
            title="Top 3 Performer

Die drei Positionen mit der besten prozentualen Performance (unrealisierter Gewinn/Verlust).

Berechnung:
• (Aktueller Wert − Einstand) / Einstand × 100%
• Basierend auf FIFO-Einstandskursen"
          >
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground block mb-2">
              Top Performer
            </span>
            <div className="space-y-1.5">
              {[...dbHoldings]
                .filter((h) => h.gainLossPercent != null)
                .sort((a, b) => (b.gainLossPercent || 0) - (a.gainLossPercent || 0))
                .slice(0, 3)
                .map((holding, index) => {
                  const cachedLogo = cachedLogos.get(holding.securityId);
                  const logoUrl = holding.customLogo || cachedLogo?.url;
                  const gainPercent = holding.gainLossPercent || 0;
                  return (
                    <div key={holding.securityId} className="flex items-center gap-2">
                      <span className="text-[10px] text-muted-foreground w-3">{index + 1}.</span>
                      <div className="w-5 h-5 rounded bg-muted/50 flex items-center justify-center overflow-hidden flex-shrink-0">
                        {logoUrl ? (
                          <img src={logoUrl} alt="" className="w-full h-full object-contain" />
                        ) : (
                          <Building2 size={10} className="text-muted-foreground" />
                        )}
                      </div>
                      <span className="text-[11px] font-medium truncate flex-1 max-w-[80px]">
                        {holding.name}
                      </span>
                      <span className={`text-[11px] font-semibold ${gainPercent >= 0 ? 'text-emerald-500' : 'text-red-500'}`}>
                        {gainPercent >= 0 ? '+' : ''}{gainPercent.toFixed(1)}%
                      </span>
                    </div>
                  );
                })}
            </div>
          </div>

          {/* KI Features Card */}
          <AiFeaturesCard
            onOpenInsights={() => setShowInsightsModal(true)}
            onOpenChat={() => onOpenChat?.()}
            onOpenPdfImport={() => setShowPdfImportModal(true)}
            onOpenCsvImport={() => setShowCsvImportModal(true)}
          />

          {/* Auto-Update */}
          <div
            className="glass-card p-3 flex flex-col justify-between min-w-[110px] cursor-help"
            title="Automatische Kursaktualisierung

Lädt aktuelle Kurse für Ihre Wertpapiere automatisch im gewählten Intervall.

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
              <div className="text-[10px] text-muted-foreground tabular-nums">
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
              : 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20'
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
          <div className="flex-1 glass-card p-4 min-w-0">
            <PortfolioChart
              portfolioData={filteredChartData}
              investedData={filteredInvestedData}
              timeRange={chartTimeRange}
              onTimeRangeChange={setChartTimeRange}
              currency={baseCurrency}
              currentTotalValue={totalValue}
              currentCostBasis={totalCostBasis}
            />
          </div>

          {/* Holdings Sidebar */}
          <div className="w-[340px] glass-card p-3 flex flex-col flex-shrink-0">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <span className="text-[10px] uppercase tracking-wider text-muted-foreground font-medium">
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
                  <span className="text-[9px] text-primary tabular-nums">
                    {batchProgress.current}/{batchProgress.total}
                  </span>
                )}
                <span className="text-[10px] text-muted-foreground">{dbHoldings.length} Titel</span>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto -mx-3 px-3 space-y-0.5">
              {holdingsByValue.map((holding) => {
                const cachedLogo = cachedLogos.get(holding.securityId);
                const logoUrl = holding.customLogo || cachedLogo?.url;
                const percent =
                  totalValue > 0 ? ((holding.currentValue || 0) / totalValue) * 100 : 0;
                const gainPercent = holding.gainLossPercent || 0;
                const isPositive = gainPercent >= 0;

                // Get AI analysis for this holding
                const analysis = analyses[holding.securityId];

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
                    {/* AI Trend Indicator */}
                    <TrendIndicator status={analysis?.trend} summary={analysis?.summary} />
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

        {/* Portfolio Insights Modal */}
        <PortfolioInsightsModal
          isOpen={showInsightsModal}
          onClose={() => setShowInsightsModal(false)}
        />

        {/* PDF Import Modal */}
        <PdfImportModal
          isOpen={showPdfImportModal}
          onClose={() => setShowPdfImportModal(false)}
          onSuccess={onRefresh}
        />

        {/* CSV Import Modal */}
        <CsvImportModal
          isOpen={showCsvImportModal}
          onClose={() => setShowCsvImportModal(false)}
          onSuccess={onRefresh}
        />
      </div>
    );
  }

  // Welcome screen (no holdings yet)
  return (
    <div className="h-full flex items-center justify-center">
      <div className="text-center max-w-sm">
        <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center mx-auto mb-6 backdrop-blur-sm border border-primary/10">
          <TrendingUp className="w-8 h-8 text-primary" />
        </div>
        <h2 className="text-xl font-light mb-2">Portfolio Now</h2>
        <p className="text-sm text-muted-foreground mb-8">
          Importieren Sie Ihre Portfolio Performance Datei, um zu starten
        </p>
        <button
          onClick={onImportPP}
          className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors mx-auto"
        >
          <Database size={16} />
          PP-Datei importieren
        </button>
      </div>
    </div>
  );
}
