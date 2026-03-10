/**
 * Charts View - TradingView-like Technical Analysis
 *
 * Features:
 * - Security selection with filter (holdings/all)
 * - External security search
 * - Candlestick chart with volume
 * - Technical indicators (RSI, MACD, SMA, EMA, Bollinger, ATR)
 * - Time range selection
 * - Fullscreen mode
 */

import { useState, useEffect, useMemo, useCallback, useRef, Component, type ReactNode } from 'react';
import type { IChartApi, ISeriesApi } from 'lightweight-charts';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Search,
  RefreshCw,
  Calendar,
  TrendingUp,
  Loader2,
  AlertTriangle,
  Briefcase,
  Eye,
  Plus,
  Maximize2,
  Minimize2,
  CandlestickChart,
  GitCompare,
  Check,
  Pencil,
  Newspaper,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-react';
import { TradingViewChart } from '../../components/charts/TradingViewChart';
import { IndicatorsPanel } from '../../components/charts/IndicatorsPanel';
import { AIAnalysisPanel } from '../../components/charts/AIAnalysisPanel';
import { SignalsPanel } from '../../components/charts/SignalsPanel';
import { AlertsPanel } from '../../components/charts/AlertsPanel';
import { TradingAnalysisPanel } from '../../components/charts/TradingAnalysisPanel';
import { ComparisonChart, COMPARISON_COLORS, type ComparisonSecurity, DrawingTools, type Drawing, PatternStatisticsPanel, ShareToXButton } from '../../components/charts';
import { SecuritySearchModal } from '../../components/modals';
import { NewsResearchModal } from '../../components/modals/NewsResearchModal';
import { SecurityLogo } from '../../components/common';
import type { IndicatorConfig, OHLCData, CandleInterval } from '../../lib/indicators';
import { convertToOHLC, aggregateOHLC } from '../../lib/indicators';
import { convertToHeikinAshi, getAllSignals as getAllSignalsRust } from '../../lib/indicators-rust';
import { useSettingsStore, useAppStore, useUIStore } from '../../store';
import { useCachedLogos } from '../../lib/hooks';
import {
  getWatchlists,
  getWatchlistSecurities,
  getPriceHistoryWithOutliers,
  getChartDrawings,
  saveChartDrawing,
  deleteChartDrawing,
  searchExternalSecurities,
  addExternalSecurityToWatchlist,
  createWatchlist,
  type OutlierSummary,
  type ChartDrawingResponse,
} from '../../lib/api';
import type { WatchlistSecurityData, ChartAnnotationWithId, ExternalSecuritySearchResult } from '../../lib/types';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import type { TechnicalSignal } from '../../lib/signals';
import type { AggregatedHolding } from '../types';

// ============================================================================
// Error Boundary for Chart
// ============================================================================

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class ChartErrorBoundary extends Component<{ children: ReactNode }, ErrorBoundaryState> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('Chart error:', error);
    console.error('Error message:', error.message);
    console.error('Error stack:', error.stack);
    console.error('Component stack:', errorInfo.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="h-full flex flex-col items-center justify-center text-muted-foreground p-8">
          <AlertTriangle size={48} className="mb-4 text-yellow-500" />
          <p className="text-lg font-medium mb-2">Chart-Fehler</p>
          <p className="text-sm text-center mb-4">
            Ein Fehler ist beim Rendern des Charts aufgetreten.
          </p>
          <pre className="text-xs bg-muted p-2 rounded max-w-full overflow-auto">
            {this.state.error?.message}
          </pre>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg"
          >
            Erneut versuchen
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}

// ============================================================================
// Types
// ============================================================================

interface SecurityData {
  id: number;
  name: string;
  isin: string | null;
  ticker: string | null;
  currency: string;
}

interface EnrichedSecurity extends SecurityData {
  isInHoldings: boolean;
  isWatchlistOnly: boolean;
}

interface PriceData {
  date: string;
  value: number;
  volume?: number;
}

type TimeRange = '1M' | '3M' | '6M' | '1Y' | '2Y' | '5Y' | 'MAX';
type FilterMode = 'holdings' | 'watchlist' | 'all';

// ============================================================================
// Time Range Options
// ============================================================================

const timeRanges: { value: TimeRange; label: string }[] = [
  { value: '1M', label: '1M' },
  { value: '3M', label: '3M' },
  { value: '6M', label: '6M' },
  { value: '1Y', label: '1J' },
  { value: '2Y', label: '2J' },
  { value: '5Y', label: '5J' },
  { value: 'MAX', label: 'Max' },
];

const candleIntervals: { value: CandleInterval; label: string }[] = [
  { value: 'D', label: 'T' },   // Tag (Daily)
  { value: 'W', label: 'W' },   // Woche (Weekly)
  { value: 'M', label: 'M' },   // Monat (Monthly)
];

// ============================================================================
// Main Component
// ============================================================================

export function ChartsView() {
  const { theme, brandfetchApiKey, aiEnabled } = useSettingsStore();
  const { keys: apiKeys } = useSecureApiKeys();

  // State
  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [selectedSecurity, setSelectedSecurity] = useState<SecurityData | null>(null);
  const [priceData, setPriceData] = useState<PriceData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [timeRange, setTimeRange] = useState<TimeRange>('1Y');
  const [candleInterval, setCandleInterval] = useState<CandleInterval>('D');
  const [indicators, setIndicators] = useState<IndicatorConfig[]>([
    {
      id: 'sma-default',
      type: 'sma',
      enabled: true,
      params: { period: 20 },
      color: '#2196f3',
    },
    {
      id: 'rsi-default',
      type: 'rsi',
      enabled: true,
      params: { period: 14 },
    },
  ]);

  // Filter & Fullscreen state
  const appMode = useAppStore((s) => s.appMode);
  const [filterMode, setFilterMode] = useState<FilterMode>(appMode === 'analysis' ? 'watchlist' : 'holdings');
  const [holdingsSecurityIds, setHoldingsSecurityIds] = useState<Set<number>>(new Set());
  const [watchlistSecurityIds, setWatchlistSecurityIds] = useState<Set<number>>(new Set());
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isSearchModalOpen, setIsSearchModalOpen] = useState(false);
  const [isNewsModalOpen, setIsNewsModalOpen] = useState(false);

  // Inline external search state
  const [externalResults, setExternalResults] = useState<ExternalSecuritySearchResult[]>([]);
  const [isSearchingExternal, setIsSearchingExternal] = useState(false);
  const [isAddingExternal, setIsAddingExternal] = useState<string | null>(null); // symbol being added
  const externalSearchRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // AI Annotations state
  const [chartAnnotations, setChartAnnotations] = useState<ChartAnnotationWithId[]>([]);

  // Lazy-load earlier data state

  // Heikin-Ashi mode
  const [useHeikinAshi, setUseHeikinAshi] = useState(false);

  // Left sidebar collapsed state
  const [leftSidebarCollapsed, setLeftSidebarCollapsed] = useState(false);

  // Logarithmic scale mode
  const [useLogScale, setUseLogScale] = useState(false);

  // Comparison mode
  const [isComparisonMode, setIsComparisonMode] = useState(false);
  const [comparisonSecurities, setComparisonSecurities] = useState<Set<number>>(new Set());
  const [comparisonData, setComparisonData] = useState<Map<number, { date: string; close: number }[]>>(new Map());

  // Drawing mode
  const [isDrawingMode, setIsDrawingMode] = useState(false);
  const [drawings, setDrawings] = useState<Drawing[]>([]);
  // Track saved drawing IDs to detect deletions
  const savedDrawingIds = useRef<Set<string>>(new Set());

  // Chart API for drawing tools coordinate conversion
  const [chartApiState, setChartApiState] = useState<IChartApi | null>(null);
  const [mainSeriesState, setMainSeriesState] = useState<ISeriesApi<'Candlestick'> | null>(null);
  const [chartContainerSize, setChartContainerSize] = useState({ width: 800, height: 500 });

  // Outlier detection state
  const [outlierSummary, setOutlierSummary] = useState<OutlierSummary | null>(null);

  // Refs for AI chart capture (normal and fullscreen)
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const fullscreenChartRef = useRef<HTMLDivElement>(null);

  // Ref for API keys (avoids adding to loadPriceData dependency array)
  const apiKeysRef = useRef(apiKeys);
  useEffect(() => { apiKeysRef.current = apiKeys; }, [apiKeys]);

  // ============================================================================
  // Data Loading
  // ============================================================================

  // Load securities
  const loadSecurities = useCallback(async () => {
    try {
      const data = await invoke<SecurityData[]>('get_securities', { importId: null });
      const withData = data.filter(s => s.ticker || s.isin);
      setSecurities(withData);
    } catch (err) {
      console.error('Failed to load securities:', err);
    }
  }, []);

  // Load holdings IDs
  useEffect(() => {
    const loadHoldings = async () => {
      try {
        const holdings = await invoke<AggregatedHolding[]>('get_all_holdings');
        const ids = new Set(holdings.flatMap(h => h.securityIds));
        setHoldingsSecurityIds(ids);
      } catch (err) {
        console.error('Failed to load holdings:', err);
      }
    };
    loadHoldings();
  }, []);

  // Load watchlist security IDs
  useEffect(() => {
    const loadWatchlistSecurities = async () => {
      try {
        const watchlists = await getWatchlists();
        const allWatchlistSecurityIds = new Set<number>();

        for (const wl of watchlists) {
          const securities = await getWatchlistSecurities(wl.id);
          securities.forEach((s: WatchlistSecurityData) => allWatchlistSecurityIds.add(s.securityId));
        }

        setWatchlistSecurityIds(allWatchlistSecurityIds);
      } catch (err) {
        console.error('Failed to load watchlist securities:', err);
      }
    };
    loadWatchlistSecurities();
  }, []);

  // Load securities on mount
  useEffect(() => {
    loadSecurities();
  }, [loadSecurities]);

  // Listen for watchlist updates (e.g. from ChatBot adding/removing securities)
  useEffect(() => {
    const unlisten = listen('watchlist-updated', () => {
      loadSecurities();
      // Reload watchlist IDs so sidebar filter updates
      (async () => {
        try {
          const watchlists = await getWatchlists();
          const newIds = new Set<number>();
          for (const wl of watchlists) {
            const secs = await getWatchlistSecurities(wl.id);
            secs.forEach((s: WatchlistSecurityData) => newIds.add(s.securityId));
          }
          // If current security was removed from watchlist and we're in watchlist filter mode,
          // deselect it so the chart clears
          if (
            filterMode === 'watchlist' &&
            selectedSecurity &&
            watchlistSecurityIds.has(selectedSecurity.id) &&
            !newIds.has(selectedSecurity.id)
          ) {
            setSelectedSecurity(null);
          }
          setWatchlistSecurityIds(newIds);
        } catch (err) {
          console.error('Failed to reload watchlist securities:', err);
        }
      })();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [loadSecurities, filterMode, selectedSecurity, watchlistSecurityIds]);

  // Enrich securities with holding/watchlist status (must be before useEffects that reference displayedSecurities)
  const enrichedSecurities = useMemo<EnrichedSecurity[]>(() => {
    return securities.map(s => ({
      ...s,
      isInHoldings: holdingsSecurityIds.has(s.id),
      isWatchlistOnly: !holdingsSecurityIds.has(s.id) && watchlistSecurityIds.has(s.id),
    }));
  }, [securities, holdingsSecurityIds, watchlistSecurityIds]);

  // Filter securities based on mode
  const displayedSecurities = useMemo(() => {
    if (filterMode === 'holdings') {
      return enrichedSecurities.filter(s => s.isInHoldings);
    }
    if (filterMode === 'watchlist') {
      return enrichedSecurities.filter(s => s.isWatchlistOnly || watchlistSecurityIds.has(s.id));
    }
    // 'all' mode: holdings + watchlist securities
    return enrichedSecurities.filter(s => s.isInHoldings || s.isWatchlistOnly);
  }, [enrichedSecurities, filterMode, watchlistSecurityIds]);

  // Auto-select first security when filter changes
  useEffect(() => {
    if (displayedSecurities.length > 0 && !selectedSecurity) {
      setSelectedSecurity(displayedSecurities[0]);
    }
  }, [holdingsSecurityIds, watchlistSecurityIds, securities, displayedSecurities, selectedSecurity]);

  // Handle pending chart navigation from ChatBot
  const pendingChartName = useUIStore(s => s.pendingChartSecurityName);
  const pendingChartTimeRange = useUIStore(s => s.pendingChartTimeRange);
  const pendingChartIndicators = useUIStore(s => s.pendingChartIndicators);
  useEffect(() => {
    if (!pendingChartName || securities.length === 0) return;

    // Apply time range if provided
    if (pendingChartTimeRange) {
      const validRanges: TimeRange[] = ['1M', '3M', '6M', '1Y', '2Y', '5Y', 'MAX'];
      if (validRanges.includes(pendingChartTimeRange as TimeRange)) {
        setTimeRange(pendingChartTimeRange as TimeRange);
      }
    }

    // Apply indicators if provided
    if (pendingChartIndicators && pendingChartIndicators.length > 0) {
      const newIndicators: IndicatorConfig[] = [];
      const colorMap: Record<string, string> = {
        sma: '#2196f3', sma50: '#ff9800', sma200: '#e91e63',
        ema: '#ff9800', ema50: '#4caf50',
        bollinger: '#9c27b0', vwap: '#e91e63', ichimoku: '#00bcd4',
      };
      for (const ind of pendingChartIndicators) {
        const lower = ind.toLowerCase();
        // Parse indicator string like "sma20", "rsi14", "macd", "bollinger"
        const match = lower.match(/^(sma|ema|rsi|atr|adx)(\d+)$/);
        if (match) {
          const [, type, period] = match;
          const colorKey = `${type}${period === '50' ? '50' : period === '200' ? '200' : ''}`;
          newIndicators.push({
            id: `chat-${type}-${period}`,
            type: type as IndicatorConfig['type'],
            enabled: true,
            params: { period: parseInt(period) },
            color: colorMap[colorKey] || colorMap[type],
          });
        } else if (lower === 'macd') {
          newIndicators.push({ id: 'chat-macd', type: 'macd', enabled: true, params: { fast: 12, slow: 26, signal: 9 } });
        } else if (lower === 'bollinger') {
          newIndicators.push({ id: 'chat-bollinger', type: 'bollinger', enabled: true, params: { period: 20, stdDev: 2 }, color: '#9c27b0' });
        } else if (lower === 'vwap') {
          newIndicators.push({ id: 'chat-vwap', type: 'vwap', enabled: true, params: {}, color: '#e91e63' });
        } else if (lower === 'stochastic') {
          newIndicators.push({ id: 'chat-stochastic', type: 'stochastic', enabled: true, params: { kPeriod: 14, kSlowPeriod: 3, dPeriod: 3 } });
        } else if (lower === 'obv') {
          newIndicators.push({ id: 'chat-obv', type: 'obv', enabled: true, params: {} });
        } else if (lower === 'ichimoku') {
          newIndicators.push({ id: 'chat-ichimoku', type: 'ichimoku', enabled: true, params: { tenkan: 9, kijun: 26, senkouB: 52 }, color: '#00bcd4' });
        }
      }
      if (newIndicators.length > 0) {
        setIndicators(newIndicators);
      }
    }

    const lowerPending = pendingChartName.toLowerCase();
    // Try exact match first, then partial (includes), then ticker
    const match =
      securities.find(s => s.name.toLowerCase() === lowerPending) ||
      securities.find(s => s.name.toLowerCase().includes(lowerPending)) ||
      securities.find(s => lowerPending.includes(s.name.toLowerCase())) ||
      securities.find(s => s.ticker?.toLowerCase() === lowerPending);

    if (match) {
      setSelectedSecurity(match);
      // If security exists but is not on any watchlist, add it (check live from DB to avoid race)
      (async () => {
        try {
          const watchlists = await getWatchlists();
          let alreadyOnWatchlist = false;
          for (const wl of watchlists) {
            const secs = await getWatchlistSecurities(wl.id);
            if (secs.some((s: WatchlistSecurityData) => s.securityId === match.id)) {
              alreadyOnWatchlist = true;
              break;
            }
          }
          if (!alreadyOnWatchlist) {
            let watchlistId: number;
            if (watchlists.length === 0) {
              const newWl = await createWatchlist('Watchlist');
              watchlistId = newWl.id;
            } else {
              watchlistId = watchlists[0].id;
            }
            await invoke('add_to_watchlist', { watchlistId, securityId: match.id });
            setWatchlistSecurityIds(prev => new Set([...prev, match.id]));
          }
        } catch (err) {
          console.warn('Failed to auto-add security to watchlist:', err);
        }
      })();
      useUIStore.getState().clearPendingChart();
    } else {
      // Security not in DB — search externally and add to watchlist automatically
      const addExternal = async () => {
        try {
          const response = await searchExternalSecurities(
            pendingChartName,
            apiKeys.alphaVantageApiKey || undefined
          );
          if (response.results.length > 0) {
            const best = response.results[0];
            // Get or create default watchlist
            const watchlists = await getWatchlists();
            let watchlistId: number;
            if (watchlists.length === 0) {
              const newWl = await createWatchlist('Watchlist');
              watchlistId = newWl.id;
            } else {
              watchlistId = watchlists[0].id;
            }
            const securityId = await addExternalSecurityToWatchlist(watchlistId, best);
            await loadSecurities();
            setWatchlistSecurityIds(prev => new Set([...prev, securityId]));
            // After reload, find and select the new security
            const data = await invoke<SecurityData[]>('get_securities', { importId: null });
            const added = data.find(s => s.id === securityId);
            if (added) {
              setSelectedSecurity(added);
            }
          }
        } catch (err) {
          console.warn('Failed to add external security from chat:', err);
        }
        useUIStore.getState().clearPendingChart();
      };
      addExternal();
    }
  }, [pendingChartName, securities, loadSecurities, pendingChartIndicators, pendingChartTimeRange, apiKeys.alphaVantageApiKey]);

  // Prepare securities for logo loading
  const securitiesForLogos = useMemo(() =>
    securities.map(s => ({
      id: s.id,
      ticker: s.ticker || undefined,
      name: s.name,
    })),
    [securities]
  );

  // Load logos
  const { logos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

  // Apply text search on displayed securities
  const filteredSecurities = useMemo(() => {
    if (!searchQuery) return displayedSecurities;
    const query = searchQuery.toLowerCase();
    return displayedSecurities.filter(
      s =>
        s.name.toLowerCase().includes(query) ||
        s.isin?.toLowerCase().includes(query) ||
        s.ticker?.toLowerCase().includes(query)
    );
  }, [displayedSecurities, searchQuery]);

  // Auto-search externally when local search yields no results
  useEffect(() => {
    if (externalSearchRef.current) clearTimeout(externalSearchRef.current);

    // Only search if query is long enough and no local results
    if (searchQuery.length < 2 || filteredSecurities.length > 0) {
      setExternalResults([]);
      setIsSearchingExternal(false);
      return;
    }

    setIsSearchingExternal(true);
    externalSearchRef.current = setTimeout(async () => {
      try {
        const response = await searchExternalSecurities(
          searchQuery,
          apiKeys.alphaVantageApiKey || undefined
        );
        setExternalResults(response.results.slice(0, 8));
      } catch (err) {
        console.warn('External search failed:', err);
        setExternalResults([]);
      } finally {
        setIsSearchingExternal(false);
      }
    }, 400);

    return () => {
      if (externalSearchRef.current) clearTimeout(externalSearchRef.current);
    };
  }, [searchQuery, filteredSecurities.length, apiKeys.alphaVantageApiKey]);

  // Add external security and auto-select it
  const handleAddExternal = async (result: ExternalSecuritySearchResult) => {
    setIsAddingExternal(result.symbol);
    try {
      // Get or create a default watchlist
      const watchlists = await getWatchlists();
      let watchlistId: number;
      if (watchlists.length === 0) {
        const newWl = await createWatchlist('Watchlist');
        watchlistId = newWl.id;
      } else {
        watchlistId = watchlists[0].id;
      }

      // Add security and get ID
      const securityId = await addExternalSecurityToWatchlist(watchlistId, result);

      // Reload securities list
      await loadSecurities();

      // Update watchlist IDs
      setWatchlistSecurityIds(prev => new Set([...prev, securityId]));

      // Auto-select the new security
      const allSecurities = await invoke<SecurityData[]>('get_securities', { importId: null });
      const added = allSecurities.find(s => s.id === securityId);
      if (added) {
        setSelectedSecurity(added);
      }

      // Clear search
      setSearchQuery('');
      setExternalResults([]);
    } catch (err) {
      console.error('Failed to add external security:', err);
    } finally {
      setIsAddingExternal(null);
    }
  };

  // Load ALL price data when security changes (timeRange only controls visible window)
  const loadPriceData = useCallback(async () => {
    if (!selectedSecurity) {
      setPriceData([]);
      setChartAnnotations([]);
      setOutlierSummary(null);
      return;
    }

    setIsLoading(true);
    setChartAnnotations([]);
    setOutlierSummary(null);
    try {
      const startDate = '2000-01-01'; // Always load ALL available data
      const endDate = new Date().toISOString().split('T')[0];

      // First try to get cached data with outlier detection
      let result = await getPriceHistoryWithOutliers(selectedSecurity.id, startDate, undefined);

      // If no data, fetch from provider (Yahoo)
      if (result.prices.length === 0) {
        try {
          await invoke('fetch_historical_prices', {
            securityId: selectedSecurity.id,
            from: startDate,
            to: endDate,
            apiKeys: null,
          });
          // Re-fetch from cache after download
          result = await getPriceHistoryWithOutliers(selectedSecurity.id, startDate, undefined);
        } catch (fetchErr) {
          console.warn('Failed to fetch historical prices from provider:', fetchErr);
        }
      }

      // Sync stale data (last price > 3 days old, covers weekends)
      if (result.prices.length > 0) {
        const lastDate = new Date(result.prices[result.prices.length - 1].date);
        const daysSince = Math.floor((Date.now() - lastDate.getTime()) / 86_400_000);
        if (daysSince > 3) {
          try {
            const keys = apiKeysRef.current;
            await invoke('sync_security_prices', {
              securityIds: [selectedSecurity.id],
              apiKeys: {
                finnhub: keys.finnhubApiKey || null,
                alphaVantage: keys.alphaVantageApiKey || null,
                coingecko: keys.coingeckoApiKey || null,
                twelveData: keys.twelveDataApiKey || null,
              },
            });
            // Fill gap with historical data
            await invoke('fetch_historical_prices', {
              securityId: selectedSecurity.id,
              from: result.prices[result.prices.length - 1].date,
              to: endDate,
              apiKeys: null,
            });
            result = await getPriceHistoryWithOutliers(selectedSecurity.id, startDate, undefined);
          } catch (syncErr) {
            console.warn('Failed to sync stale prices:', syncErr);
          }
        }
      }

      // Extract price data and outlier info (including OHLC when available)
      const data: PriceData[] = result.prices.map(p => ({
        date: p.date,
        value: p.value,
        open: p.open,
        high: p.high,
        low: p.low,
        volume: p.volume,
      }));

      setPriceData(data);
      setOutlierSummary(result.summary);
      // Log outliers if found
      if (result.summary.outlierCount > 0) {
        console.warn(
          `[Outlier Detection] ${selectedSecurity.name}: ${result.summary.outlierCount} outlier(s) detected`,
          result.summary.outliers
        );
      }
    } catch (err) {
      console.error('Failed to load price data:', err);
      setPriceData([]);
      setOutlierSummary(null);
    } finally {
      setIsLoading(false);
    }
  }, [selectedSecurity]); // No longer depends on timeRange

  useEffect(() => {
    loadPriceData();
  }, [loadPriceData]);

  // Load comparison data when comparison securities change
  useEffect(() => {
    if (!isComparisonMode || comparisonSecurities.size === 0) {
      setComparisonData(new Map());
      return;
    }

    const loadComparisonData = async () => {
      const startDate = new Date();
      switch (timeRange) {
        case '1M': startDate.setMonth(startDate.getMonth() - 1); break;
        case '3M': startDate.setMonth(startDate.getMonth() - 3); break;
        case '6M': startDate.setMonth(startDate.getMonth() - 6); break;
        case '1Y': startDate.setFullYear(startDate.getFullYear() - 1); break;
        case '2Y': startDate.setFullYear(startDate.getFullYear() - 2); break;
        case '5Y': startDate.setFullYear(startDate.getFullYear() - 5); break;
        case 'MAX': startDate.setFullYear(2000); break;
        default: startDate.setFullYear(startDate.getFullYear() - 1);
      }

      const newData = new Map<number, { date: string; close: number }[]>();

      for (const secId of comparisonSecurities) {
        try {
          const data = await invoke<PriceData[]>('get_price_history', {
            securityId: secId,
            startDate: startDate.toISOString().split('T')[0],
            endDate: null,
          });
          if (data.length > 0) {
            newData.set(secId, data.map(d => ({ date: d.date, close: d.value })));
          }
        } catch (err) {
          console.error(`Failed to load comparison data for security ${secId}:`, err);
        }
      }

      setComparisonData(newData);
    };

    loadComparisonData();
  }, [isComparisonMode, comparisonSecurities, timeRange]);

  // Toggle comparison security selection
  const toggleComparisonSecurity = (securityId: number) => {
    setComparisonSecurities(prev => {
      const next = new Set(prev);
      if (next.has(securityId)) {
        next.delete(securityId);
      } else if (next.size < 8) { // Max 8 securities for comparison
        next.add(securityId);
      }
      return next;
    });
  };

  // Prepare comparison chart data
  const comparisonSecuritiesData = useMemo<ComparisonSecurity[]>(() => {
    if (!isComparisonMode) return [];

    const result: ComparisonSecurity[] = [];
    let colorIndex = 0;

    comparisonSecurities.forEach(secId => {
      const security = securities.find(s => s.id === secId);
      const data = comparisonData.get(secId);

      if (security && data && data.length > 0) {
        result.push({
          id: secId,
          name: security.name,
          ticker: security.ticker || undefined,
          color: COMPARISON_COLORS[colorIndex % COMPARISON_COLORS.length],
          data,
        });
        colorIndex++;
      }
    });

    return result;
  }, [isComparisonMode, comparisonSecurities, comparisonData, securities]);

  // Convert to OHLC data (with optional aggregation + Heikin-Ashi)
  const [ohlcData, setOhlcData] = useState<OHLCData[]>([]);
  useEffect(() => {
    if (priceData.length === 0) { setOhlcData([]); return; }
    const daily = convertToOHLC(priceData, 1.5);
    const data = aggregateOHLC(daily, candleInterval);
    if (useHeikinAshi) {
      convertToHeikinAshi(data).then(setOhlcData).catch(() => setOhlcData(data));
    } else {
      setOhlcData(data);
    }
  }, [priceData, useHeikinAshi, candleInterval]);

  // Compute signals for Share button via Rust
  const [chartSignals, setChartSignals] = useState<TechnicalSignal[]>([]);
  useEffect(() => {
    if (ohlcData.length < 30) { setChartSignals([]); return; }
    getAllSignalsRust(ohlcData).then(setChartSignals).catch(() => setChartSignals([]));
  }, [ohlcData]);

  // Set visible range based on timeRange selection (controls zoom, not data loading)
  useEffect(() => {
    if (!chartApiState || ohlcData.length === 0) return;

    if (timeRange === 'MAX') {
      chartApiState.timeScale().fitContent();
      return;
    }

    const now = new Date();
    const start = new Date();
    switch (timeRange) {
      case '1M': start.setMonth(now.getMonth() - 1); break;
      case '3M': start.setMonth(now.getMonth() - 3); break;
      case '6M': start.setMonth(now.getMonth() - 6); break;
      case '1Y': start.setFullYear(now.getFullYear() - 1); break;
      case '2Y': start.setFullYear(now.getFullYear() - 2); break;
      case '5Y': start.setFullYear(now.getFullYear() - 5); break;
    }

    const startStr = start.toISOString().split('T')[0];
    const endStr = now.toISOString().split('T')[0];

    try {
      chartApiState.timeScale().setVisibleRange({
        from: startStr as string & Record<string, never>,
        to: endStr as string & Record<string, never>,
      });
    } catch {
      chartApiState.timeScale().fitContent();
    }
  }, [chartApiState, timeRange, ohlcData.length]);

  // Handle search modal security added — auto-select the added security
  const handleSecurityAdded = async (securityId: number) => {
    setWatchlistSecurityIds(prev => new Set([...prev, securityId]));
    await loadSecurities();
    const allSecurities = await invoke<SecurityData[]>('get_securities', { importId: null });
    const added = allSecurities.find(s => s.id === securityId);
    if (added) setSelectedSecurity(added);
  };

  // Load drawings when security changes
  useEffect(() => {
    if (!selectedSecurity) {
      setDrawings([]);
      savedDrawingIds.current.clear();
      return;
    }

    const loadDrawings = async () => {
      try {
        const saved = await getChartDrawings(selectedSecurity.id);
        // Convert saved drawings to Drawing format
        const loadedDrawings: Drawing[] = saved.map((d: ChartDrawingResponse) => ({
          id: d.id,
          type: d.drawingType as Drawing['type'],
          points: d.points.map(p => ({ x: p.x, y: p.y, time: p.time, price: p.price })),
          color: d.color,
          lineWidth: d.lineWidth,
          fibLevels: d.fibLevels,
        }));
        setDrawings(loadedDrawings);
        savedDrawingIds.current = new Set(saved.map(d => d.id));
      } catch (err) {
        console.error('Failed to load chart drawings:', err);
        setDrawings([]);
        savedDrawingIds.current.clear();
      }
    };

    loadDrawings();
  }, [selectedSecurity]);

  // Handle drawings change - save new drawings and delete removed ones
  const handleDrawingsChange = useCallback(async (newDrawings: Drawing[]) => {
    setDrawings(newDrawings);

    if (!selectedSecurity) return;

    const newIds = new Set(newDrawings.map(d => d.id));
    const oldIds = savedDrawingIds.current;

    // Find deleted drawings and remove them from DB
    for (const oldId of oldIds) {
      if (!newIds.has(oldId)) {
        try {
          await deleteChartDrawing(parseInt(oldId, 10));
        } catch (err) {
          console.error('Failed to delete drawing:', err);
        }
      }
    }

    // Find new drawings and save them to DB
    for (const drawing of newDrawings) {
      if (!oldIds.has(drawing.id)) {
        try {
          const saved = await saveChartDrawing({
            securityId: selectedSecurity.id,
            drawingType: drawing.type,
            points: drawing.points,
            color: drawing.color,
            lineWidth: drawing.lineWidth,
            fibLevels: drawing.fibLevels,
          });
          // Update the drawing ID to the DB ID
          drawing.id = saved.id;
          savedDrawingIds.current.add(saved.id);
        } catch (err) {
          console.error('Failed to save drawing:', err);
        }
      }
    }

    // Update tracked IDs
    savedDrawingIds.current = new Set(newDrawings.map(d => d.id));
  }, [selectedSecurity]);

  // Chart ready callback for drawing tools
  const handleChartReady = useCallback((api: IChartApi, series: ISeriesApi<'Candlestick'>) => {
    setChartApiState(api);
    setMainSeriesState(series);
  }, []);

  // Track chart container size for drawing canvas
  useEffect(() => {
    const container = chartContainerRef.current;
    if (!container) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width: w, height: h } = entry.contentRect;
        if (w > 0 && h > 0) {
          setChartContainerSize({ width: w, height: h });
        }
      }
    });
    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Handle ESC key for fullscreen
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isFullscreen) {
        setIsFullscreen(false);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isFullscreen]);

  // ============================================================================
  // Render
  // ============================================================================

  const resolvedTheme = theme === 'system' ? 'dark' : theme;

  // Fullscreen mode
  if (isFullscreen) {
    // Calculate heights for fullscreen layout
    const headerHeight = 52; // Header with title and controls
    const aiPanelHeight = 280; // Fixed height for AI panel
    const chartHeight = window.innerHeight - headerHeight - aiPanelHeight;

    return (
      <div className="fixed inset-0 z-50 bg-background flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-border">
          <div className="flex items-center gap-4">
            {selectedSecurity && (
              <div className="flex items-center gap-3 text-lg font-semibold">
                <SecurityLogo securityId={selectedSecurity.id} logos={logos} size={32} />
                <span>{selectedSecurity.ticker || selectedSecurity.name}</span>
                <span className="text-muted-foreground text-base">{selectedSecurity.currency}</span>
              </div>
            )}

            {/* Time Range Selector */}
            <div className="flex gap-1">
              {timeRanges.map(range => (
                <button
                  key={range.value}
                  onClick={() => setTimeRange(range.value)}
                  className={`px-2 py-0.5 text-xs font-medium rounded transition-colors ${
                    timeRange === range.value
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted text-muted-foreground hover:bg-muted/80'
                  }`}
                >
                  {range.label}
                </button>
              ))}
            </div>

            {/* Candle Interval Selector */}
            <div className="w-px h-5 bg-border" />
            <div className="flex gap-1">
              {candleIntervals.map(interval => (
                <button
                  key={interval.value}
                  onClick={() => setCandleInterval(interval.value)}
                  className={`px-2 py-0.5 text-xs font-medium rounded transition-colors ${
                    candleInterval === interval.value
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted text-muted-foreground hover:bg-muted/80'
                  }`}
                >
                  {interval.label}
                </button>
              ))}
            </div>

            {/* Heikin-Ashi Toggle */}
            <button
              onClick={() => setUseHeikinAshi(!useHeikinAshi)}
              className={`flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors ${
                useHeikinAshi
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground hover:bg-muted/80'
              }`}
              title={useHeikinAshi ? 'Zu normalen Kerzen wechseln' : 'Zu Heikin-Ashi wechseln'}
            >
              <CandlestickChart size={14} />
              HA
            </button>

            {/* Log Scale Toggle */}
            <button
              onClick={() => setUseLogScale(!useLogScale)}
              className={`flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors ${
                useLogScale
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground hover:bg-muted/80'
              }`}
              title={useLogScale ? 'Zu linearer Skala wechseln' : 'Zu logarithmischer Skala wechseln'}
            >
              Log
            </button>

            {/* Drawing Tools Toggle */}
            <button
              onClick={() => setIsDrawingMode(!isDrawingMode)}
              className={`flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors ${
                isDrawingMode
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground hover:bg-muted/80'
              }`}
              title={isDrawingMode ? 'Zeichnen beenden' : 'Zeichenwerkzeuge'}
            >
              <Pencil size={14} />
              Zeichnen
            </button>

            {/* Share to X Button */}
            {selectedSecurity && (
              <ShareToXButton
                variant="icon"
                chartRef={fullscreenChartRef}
                security={selectedSecurity}
                currentPrice={ohlcData[ohlcData.length - 1]?.close || 0}
                signals={chartSignals}
              />
            )}

            {/* News Research Button */}
            {selectedSecurity && aiEnabled && (
              <button
                onClick={() => setIsNewsModalOpen(true)}
                className="flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors bg-muted text-muted-foreground hover:bg-muted/80"
                title="Nachrichten recherchieren"
              >
                <Newspaper size={14} />
                News
              </button>
            )}
          </div>

          <button
            onClick={() => setIsFullscreen(false)}
            className="p-2 hover:bg-muted rounded-lg transition-colors flex items-center gap-2"
          >
            <Minimize2 size={18} />
            <span className="text-sm">ESC</span>
          </button>
        </div>

        {/* Main Content */}
        <div className="flex-1 flex min-h-0">
          {/* Left: Chart + AI Analysis */}
          <div className="flex-1 flex flex-col min-w-0">
            {/* Chart */}
            <div ref={fullscreenChartRef} className="flex-1 min-h-0">
              {isLoading ? (
                <div className="h-full flex items-center justify-center">
                  <Loader2 size={32} className="animate-spin text-muted-foreground" />
                </div>
              ) : ohlcData.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
                  <TrendingUp size={48} className="mb-4 opacity-50" />
                  <p className="text-lg font-medium">Keine Preisdaten verfügbar</p>
                </div>
              ) : (
                <ChartErrorBoundary>
                  <TradingViewChart
                    data={ohlcData}
                    indicators={indicators}
                    height={chartHeight}
                    theme={resolvedTheme}
                    showVolume={true}
                    symbol={selectedSecurity?.ticker || selectedSecurity?.name}
                    logScale={useLogScale}
                    annotations={chartAnnotations}
                    onChartReady={handleChartReady}
                  />
                </ChartErrorBoundary>
              )}
            </div>

            {/* AI Analysis Panel - Fullscreen */}
            <div className="border-t border-border">
              <AIAnalysisPanel
                chartRef={fullscreenChartRef}
                security={selectedSecurity}
                currentPrice={ohlcData[ohlcData.length - 1]?.close || 0}
                timeRange={timeRange}
                indicators={indicators}
                ohlcData={ohlcData}
                onAnnotationsChange={setChartAnnotations}
              />
            </div>
          </div>

          {/* Indicators, Signals & Alerts Panel (narrower in fullscreen) */}
          <div className="w-64 border-l border-border p-4 overflow-auto space-y-4">
            <IndicatorsPanel indicators={indicators} onIndicatorsChange={setIndicators} />
            <SignalsPanel data={ohlcData} />
            <AlertsPanel
              securityId={selectedSecurity?.id || null}
              currentPrice={ohlcData[ohlcData.length - 1]?.close}
              currency={selectedSecurity?.currency}
            />
          </div>
        </div>
      </div>
    );
  }

  // Normal mode
  return (
    <>
      <div className="h-full flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <TrendingUp className="text-primary" size={24} />
            <h1 className="text-xl font-semibold">Technische Analyse</h1>
          </div>
          <button
            onClick={loadPriceData}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw size={14} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
        </div>

        <div className="flex-1 flex gap-4 min-h-0">
          {/* Left Sidebar - Security Selection */}
          <div className={`flex-shrink-0 flex flex-col card-surface overflow-hidden border border-border rounded-xl transition-[width] duration-200 ${leftSidebarCollapsed ? 'w-10' : 'w-72'}`}>
            {leftSidebarCollapsed ? (
              <div className="flex flex-col items-center py-2 gap-2">
                <button
                  onClick={() => setLeftSidebarCollapsed(false)}
                  className="p-1.5 rounded hover:bg-muted transition-colors"
                  title="Seitenleiste aufklappen"
                >
                  <PanelLeftOpen size={16} className="text-muted-foreground" />
                </button>
                <Search size={14} className="text-muted-foreground/50" />
              </div>
            ) : (
            <>
            {/* Filter Toggle — only show when securities exist */}
            {securities.length > 0 && (
              <div className="p-2 border-b border-border flex gap-1">
                <button
                  onClick={() => setFilterMode('holdings')}
                  className={`flex-1 px-2 py-1.5 text-xs font-semibold rounded-md flex items-center justify-center gap-1 transition-colors ${
                    filterMode === 'holdings'
                      ? 'bg-primary text-primary-foreground shadow-sm'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                  }`}
                >
                  <Briefcase size={12} />
                  Bestand
                </button>
                <button
                  onClick={() => setFilterMode('watchlist')}
                  className={`flex-1 px-2 py-1.5 text-xs font-semibold rounded-md flex items-center justify-center gap-1 transition-colors ${
                    filterMode === 'watchlist'
                      ? 'bg-primary text-primary-foreground shadow-sm'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                  }`}
                >
                  <Eye size={12} />
                  Watchlist
                </button>
                <button
                  onClick={() => setFilterMode('all')}
                  className={`flex-1 px-2 py-1.5 text-xs font-semibold rounded-md flex items-center justify-center gap-1 transition-colors ${
                    filterMode === 'all'
                      ? 'bg-primary text-primary-foreground shadow-sm'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                  }`}
                >
                  Alle
                </button>
              </div>
            )}

            {/* Search Input */}
            <div className="p-3 border-b border-border flex gap-2 items-center">
              <div className="relative flex-1">
                <Search
                  size={14}
                  className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none"
                />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={e => setSearchQuery(e.target.value)}
                  placeholder={securities.length === 0 ? 'Wertpapier suchen, z.B. AAPL' : 'Suchen...'}
                  className="w-full pl-8 pr-3 py-2 text-sm bg-muted/60 border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary focus:border-primary placeholder:text-muted-foreground/60"
                />
              </div>
              <button
                onClick={() => setLeftSidebarCollapsed(true)}
                className="p-1.5 rounded hover:bg-muted transition-colors shrink-0"
                title="Seitenleiste einklappen"
              >
                <PanelLeftClose size={16} className="text-muted-foreground" />
              </button>
            </div>

            {/* Securities List */}
            <div className="flex-1 overflow-auto">
              {/* Empty state — welcoming onboarding */}
              {filteredSecurities.length === 0 && searchQuery.length < 2 && (
                <div className="p-5 flex flex-col items-center text-center">
                  <div className="w-12 h-12 rounded-xl bg-primary/10 flex items-center justify-center mb-3">
                    <Search size={20} className="text-primary" />
                  </div>
                  <p className="text-sm font-medium mb-1">
                    {securities.length === 0 ? 'Wertpapier hinzufügen' : filterMode === 'holdings' ? 'Kein Bestand' : filterMode === 'watchlist' ? 'Watchlist leer' : 'Keine Treffer'}
                  </p>
                  <p className="text-xs text-muted-foreground mb-4">
                    {securities.length === 0
                      ? 'Tippe einen Ticker oder Namen in die Suche — z.B. AAPL, SAP, Bitcoin'
                      : filterMode === 'holdings'
                        ? 'Wechsle zu "Watchlist" oder suche ein neues Wertpapier'
                        : filterMode === 'watchlist'
                          ? 'Füge Wertpapiere über die Suche zur Watchlist hinzu'
                          : 'Nutze die Suche um Wertpapiere zu finden'}
                  </p>
                  {securities.length === 0 && (
                    <button
                      onClick={() => setIsSearchModalOpen(true)}
                      className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors shadow-sm"
                    >
                      <Plus size={14} />
                      Erweiterte Suche
                    </button>
                  )}
                </div>
              )}

              {/* Local results */}
              {filteredSecurities.map(security => {
                const isSelected = isComparisonMode
                  ? comparisonSecurities.has(security.id)
                  : selectedSecurity?.id === security.id;

                return (
                  <button
                    key={security.id}
                    onClick={() => {
                      if (isComparisonMode) {
                        toggleComparisonSecurity(security.id);
                      } else {
                        setSelectedSecurity(security);
                      }
                    }}
                    className={`w-full text-left px-3 py-2.5 border-b border-border/40 transition-colors ${
                      isSelected
                        ? 'bg-primary/10 border-l-[3px] border-l-primary'
                        : 'hover:bg-muted/70'
                    }`}
                  >
                    <div className="flex items-center gap-2.5">
                      {isComparisonMode && (
                        <div
                          className={`w-5 h-5 rounded border-2 flex items-center justify-center flex-shrink-0 transition-colors ${
                            isSelected
                              ? 'bg-primary border-primary text-primary-foreground'
                              : 'border-muted-foreground/40 hover:border-primary/60'
                          }`}
                        >
                          {isSelected && <Check size={12} />}
                        </div>
                      )}
                      <SecurityLogo securityId={security.id} logos={logos} size={28} />
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <div className="font-medium text-sm truncate">{security.name}</div>
                          {security.isWatchlistOnly && (
                            <span className="px-1.5 py-0.5 text-[10px] font-medium bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-300 rounded flex-shrink-0">
                              Watchlist
                            </span>
                          )}
                        </div>
                        <div className="text-xs text-muted-foreground mt-0.5">
                          {security.ticker && <span className="font-mono mr-2">{security.ticker}</span>}
                          {security.isin && <span>{security.isin}</span>}
                        </div>
                      </div>
                    </div>
                  </button>
                );
              })}

              {/* Inline External Search Results */}
              {searchQuery.length >= 2 && filteredSecurities.length === 0 && (
                <>
                  {isSearchingExternal ? (
                    <div className="p-5 flex flex-col items-center gap-2 text-sm text-muted-foreground">
                      <Loader2 size={18} className="animate-spin text-primary" />
                      <span>Suche nach &quot;{searchQuery}&quot;...</span>
                    </div>
                  ) : externalResults.length > 0 ? (
                    <>
                      <div className="px-3 py-2 text-[11px] font-semibold text-muted-foreground uppercase tracking-wider bg-muted/30 border-b border-border">
                        Ergebnisse hinzufügen
                      </div>
                      {externalResults.map(result => (
                        <button
                          key={`${result.provider}-${result.symbol}`}
                          onClick={() => handleAddExternal(result)}
                          disabled={isAddingExternal !== null}
                          className="w-full text-left px-3 py-2.5 border-b border-border/40 hover:bg-green-500/5 dark:hover:bg-green-500/10 transition-colors group disabled:opacity-50"
                        >
                          <div className="flex items-center gap-2.5">
                            <div className={`w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors ${
                              isAddingExternal === result.symbol
                                ? 'bg-primary/20'
                                : 'bg-green-500/10 group-hover:bg-green-500/20 dark:bg-green-500/10 dark:group-hover:bg-green-500/20'
                            }`}>
                              {isAddingExternal === result.symbol ? (
                                <Loader2 size={14} className="animate-spin text-primary" />
                              ) : (
                                <Plus size={14} className="text-green-600 dark:text-green-400" />
                              )}
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="font-medium text-sm truncate">{result.name}</div>
                              <div className="flex items-center gap-1.5 text-xs text-muted-foreground mt-0.5">
                                <span className="font-mono font-medium text-foreground/70">{result.symbol}</span>
                                {result.currency && <span>· {result.currency}</span>}
                                {result.securityType && (
                                  <span className="px-1 py-0.5 text-[9px] font-medium bg-muted rounded">{result.securityType}</span>
                                )}
                              </div>
                            </div>
                          </div>
                        </button>
                      ))}
                    </>
                  ) : (
                    <div className="p-5 text-center">
                      <p className="text-sm text-muted-foreground">Keine Ergebnisse für &quot;{searchQuery}&quot;</p>
                      <button
                        onClick={() => setIsSearchModalOpen(true)}
                        className="mt-3 text-xs font-medium text-primary hover:text-primary/80 transition-colors"
                      >
                        Erweiterte Suche starten
                      </button>
                    </div>
                  )}
                </>
              )}
            </div>
            </>
            )}
          </div>

          {/* Main Content */}
          <div className="flex-1 flex flex-col min-w-0">
            {/* Time Range Selector */}
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <Calendar size={14} className="text-muted-foreground" />
                <span className="text-sm text-muted-foreground">Zeitraum:</span>
                <div className="flex gap-1">
                  {timeRanges.map(range => (
                    <button
                      key={range.value}
                      onClick={() => setTimeRange(range.value)}
                      className={`px-2 py-0.5 text-xs font-medium rounded transition-colors ${
                        timeRange === range.value
                          ? 'bg-primary text-primary-foreground'
                          : 'bg-muted text-muted-foreground hover:bg-muted/80'
                      }`}
                    >
                      {range.label}
                    </button>
                  ))}
                </div>
                <div className="w-px h-5 bg-border" />
                <div className="flex gap-1">
                  {candleIntervals.map(interval => (
                    <button
                      key={interval.value}
                      onClick={() => setCandleInterval(interval.value)}
                      className={`px-2 py-0.5 text-xs font-medium rounded transition-colors ${
                        candleInterval === interval.value
                          ? 'bg-primary text-primary-foreground'
                          : 'bg-muted text-muted-foreground hover:bg-muted/80'
                      }`}
                    >
                      {interval.label}
                    </button>
                  ))}
                </div>
              </div>

              <div className="flex items-center gap-2">
                {/* Comparison Mode Toggle */}
                <button
                  onClick={() => {
                    setIsComparisonMode(!isComparisonMode);
                    if (!isComparisonMode && selectedSecurity) {
                      // When entering comparison mode, add current security
                      setComparisonSecurities(new Set([selectedSecurity.id]));
                    }
                  }}
                  className={`flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors ${
                    isComparisonMode
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted text-muted-foreground hover:bg-muted/80'
                  }`}
                  title={isComparisonMode ? 'Vergleich beenden' : 'Wertpapiere vergleichen'}
                >
                  <GitCompare size={14} />
                  Vergleich
                </button>

                {/* Heikin-Ashi Toggle */}
                {!isComparisonMode && (
                  <button
                    onClick={() => setUseHeikinAshi(!useHeikinAshi)}
                    className={`flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors ${
                      useHeikinAshi
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted text-muted-foreground hover:bg-muted/80'
                    }`}
                    title={useHeikinAshi ? 'Zu normalen Kerzen wechseln' : 'Zu Heikin-Ashi wechseln'}
                  >
                    <CandlestickChart size={14} />
                    HA
                  </button>
                )}

                {/* Log Scale Toggle */}
                {!isComparisonMode && (
                  <button
                    onClick={() => setUseLogScale(!useLogScale)}
                    className={`flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors ${
                      useLogScale
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted text-muted-foreground hover:bg-muted/80'
                    }`}
                    title={useLogScale ? 'Zu linearer Skala wechseln' : 'Zu logarithmischer Skala wechseln'}
                  >
                    Log
                  </button>
                )}

                {/* Drawing Tools Toggle */}
                {!isComparisonMode && (
                  <button
                    onClick={() => setIsDrawingMode(!isDrawingMode)}
                    className={`flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors ${
                      isDrawingMode
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted text-muted-foreground hover:bg-muted/80'
                    }`}
                    title={isDrawingMode ? 'Zeichnen beenden' : 'Zeichenwerkzeuge'}
                  >
                    <Pencil size={14} />
                    Zeichnen
                  </button>
                )}

                {/* Share to X Button */}
                {!isComparisonMode && selectedSecurity && (
                  <ShareToXButton
                    variant="icon"
                    chartRef={chartContainerRef}
                    security={selectedSecurity}
                    currentPrice={ohlcData[ohlcData.length - 1]?.close || 0}
                    signals={chartSignals}
                  />
                )}

                {/* News Research Button */}
                {!isComparisonMode && selectedSecurity && aiEnabled && (
                  <button
                    onClick={() => setIsNewsModalOpen(true)}
                    className="flex items-center gap-1.5 px-2 py-1 text-xs font-medium rounded transition-colors bg-muted text-muted-foreground hover:bg-muted/80"
                    title="Nachrichten recherchieren"
                  >
                    <Newspaper size={14} />
                    News
                  </button>
                )}

                {isComparisonMode ? (
                  <span className="text-xs text-muted-foreground">
                    {comparisonSecurities.size}/8 ausgewählt
                  </span>
                ) : selectedSecurity && (
                  <div className="flex items-center gap-2 text-sm">
                    <SecurityLogo securityId={selectedSecurity.id} logos={logos} size={24} />
                    <span className="font-semibold">{selectedSecurity.name}</span>
                    <span className="text-muted-foreground">{selectedSecurity.currency}</span>
                  </div>
                )}
                <button
                  onClick={() => setIsFullscreen(true)}
                  className="p-1.5 hover:bg-muted rounded-lg transition-colors"
                  title="Vollbild"
                >
                  <Maximize2 size={16} />
                </button>
              </div>
            </div>

            {/* Outlier Warning Banner */}
            {outlierSummary && outlierSummary.outlierCount > 0 && (
              <div className="mb-2 p-2 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg flex items-start gap-2">
                <AlertTriangle size={16} className="text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-amber-800 dark:text-amber-300">
                    {outlierSummary.outlierCount} Kursausreißer erkannt
                  </div>
                  <div className="text-xs text-amber-700 dark:text-amber-400 mt-0.5">
                    Die folgenden Kursdaten zeigen ungewöhnliche Tagesänderungen (&gt;30% oder Spike-Muster) und werden
                    bei Analysen nicht berücksichtigt:
                  </div>
                  <div className="text-xs text-amber-600 dark:text-amber-500 mt-1 font-mono">
                    {outlierSummary.outliers.slice(0, 5).map((o, i) => (
                      <span key={o.date} className="inline-block mr-2">
                        {o.date}: {o.changePercent > 0 ? '+' : ''}{o.changePercent.toFixed(1)}%
                        {i < Math.min(4, outlierSummary.outliers.length - 1) && ','}
                      </span>
                    ))}
                    {outlierSummary.outliers.length > 5 && (
                      <span className="text-amber-500 dark:text-amber-600">
                        ... und {outlierSummary.outliers.length - 5} weitere
                      </span>
                    )}
                  </div>
                </div>
              </div>
            )}

            {/* Chart Area */}
            <div
              ref={chartContainerRef}
              className="flex-1 card-elevated overflow-hidden min-h-0"
            >
              {isComparisonMode ? (
                // Comparison Mode Chart
                comparisonSecuritiesData.length === 0 ? (
                  <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
                    <GitCompare size={48} className="mb-4 opacity-50" />
                    <p className="text-lg font-medium">Wertpapiere auswählen</p>
                    <p className="text-sm">Wähle bis zu 8 Wertpapiere zum Vergleichen aus.</p>
                  </div>
                ) : (
                  <ChartErrorBoundary>
                    <ComparisonChart
                      securities={comparisonSecuritiesData}
                      height={500}
                      theme={resolvedTheme}
                      normalize={true}
                    />
                  </ChartErrorBoundary>
                )
              ) : (
                // Single Security Chart
                isLoading ? (
                  <div className="h-full flex items-center justify-center">
                    <Loader2 size={32} className="animate-spin text-muted-foreground" />
                  </div>
                ) : ohlcData.length === 0 ? (
                  <div className="h-full flex flex-col items-center justify-center text-muted-foreground">
                    <TrendingUp size={48} className="mb-4 opacity-50" />
                    <p className="text-lg font-medium">Keine Preisdaten verfügbar</p>
                    <p className="text-sm">Wähle ein Wertpapier aus oder synchronisiere die Kurse.</p>
                  </div>
                ) : (
                  <div className="relative h-full">
                    <ChartErrorBoundary>
                      <TradingViewChart
                        data={ohlcData}
                        indicators={indicators}
                        height={500}
                        theme={resolvedTheme}
                        showVolume={true}
                        symbol={selectedSecurity?.ticker || selectedSecurity?.name}
                        logScale={useLogScale}
                        annotations={chartAnnotations}
                        onChartReady={handleChartReady}
                      />
                    </ChartErrorBoundary>
                    {/* Drawing Tools Overlay */}
                    <DrawingTools
                      key={selectedSecurity?.id}
                      chartApi={chartApiState}
                      mainSeries={mainSeriesState}
                      width={chartContainerSize.width}
                      height={chartContainerSize.height}
                      enabled={isDrawingMode}
                      initialDrawings={drawings}
                      onDrawingsChange={handleDrawingsChange}
                    />
                  </div>
                )
              )}
            </div>

            {/* AI Analysis Panel - only in single security mode */}
            {!isComparisonMode && (
              <AIAnalysisPanel
                chartRef={chartContainerRef}
                security={selectedSecurity}
                currentPrice={ohlcData[ohlcData.length - 1]?.close || 0}
                timeRange={timeRange}
                indicators={indicators}
                ohlcData={ohlcData}
                onAnnotationsChange={setChartAnnotations}
              />
            )}

          </div>

          {/* Right Sidebar - Trading Analysis, Indicators, Signals, Alerts & Pattern Statistics */}
          <div className="w-72 flex-shrink-0 space-y-4 overflow-y-auto">
            {!isComparisonMode && ohlcData.length >= 20 && (
              <TradingAnalysisPanel
                data={ohlcData}
                currency={selectedSecurity?.currency}
                currentPrice={ohlcData[ohlcData.length - 1]?.close}
              />
            )}
            <IndicatorsPanel indicators={indicators} onIndicatorsChange={setIndicators} />
            <SignalsPanel data={ohlcData} />
            <AlertsPanel
              securityId={selectedSecurity?.id || null}
              currentPrice={ohlcData[ohlcData.length - 1]?.close}
              currency={selectedSecurity?.currency}
            />
            <PatternStatisticsPanel securityId={selectedSecurity?.id} />

            {/* Chart Info */}
            {ohlcData.length > 0 && (
              <div className="mt-4 card-surface p-3">
                <div className="text-xs text-muted-foreground font-medium mb-2">Chart-Info</div>
                <div className="space-y-1 text-xs">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Datenpunkte:</span>
                    <span className="font-mono tabular-nums">{ohlcData.length}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Intervall:</span>
                    <span className="font-mono tabular-nums">
                      {candleInterval === 'D' ? 'Täglich' : candleInterval === 'W' ? 'Wöchentlich' : 'Monatlich'}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Zeitraum:</span>
                    <span className="font-mono tabular-nums">
                      {ohlcData[0]?.time} - {ohlcData[ohlcData.length - 1]?.time}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Aktuell:</span>
                    <span className="font-mono tabular-nums font-semibold">
                      {ohlcData[ohlcData.length - 1]?.close.toFixed(2)} {selectedSecurity?.currency}
                    </span>
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Search Modal */}
      <SecuritySearchModal
        isOpen={isSearchModalOpen}
        onClose={() => setIsSearchModalOpen(false)}
        onSecurityAdded={handleSecurityAdded}
      />

      {/* News Research Modal */}
      {selectedSecurity && (
        <NewsResearchModal
          isOpen={isNewsModalOpen}
          onClose={() => setIsNewsModalOpen(false)}
          security={{
            id: selectedSecurity.id,
            name: selectedSecurity.name,
            ticker: selectedSecurity.ticker,
            isin: selectedSecurity.isin,
            currency: selectedSecurity.currency,
          }}
          currentPrice={ohlcData[ohlcData.length - 1]?.close}
        />
      )}
    </>
  );
}

export default ChartsView;
