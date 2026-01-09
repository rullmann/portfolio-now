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

import { useState, useEffect, useMemo, useCallback, Component, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
} from 'lucide-react';
import { TradingViewChart } from '../../components/charts/TradingViewChart';
import { IndicatorsPanel } from '../../components/charts/IndicatorsPanel';
import { SecuritySearchModal } from '../../components/modals';
import type { IndicatorConfig, OHLCData } from '../../lib/indicators';
import { convertToOHLC } from '../../lib/indicators';
import { useSettingsStore } from '../../store';
import { getWatchlists, getWatchlistSecurities } from '../../lib/api';
import type { WatchlistSecurityData } from '../../lib/types';
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
}

type TimeRange = '1M' | '3M' | '6M' | '1Y' | '2Y' | '5Y' | 'MAX';
type FilterMode = 'holdings' | 'all';

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

// ============================================================================
// Main Component
// ============================================================================

export function ChartsView() {
  const { theme } = useSettingsStore();

  // State
  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [selectedSecurity, setSelectedSecurity] = useState<SecurityData | null>(null);
  const [priceData, setPriceData] = useState<PriceData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [timeRange, setTimeRange] = useState<TimeRange>('1Y');
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
  const [filterMode, setFilterMode] = useState<FilterMode>('holdings');
  const [holdingsSecurityIds, setHoldingsSecurityIds] = useState<Set<number>>(new Set());
  const [watchlistSecurityIds, setWatchlistSecurityIds] = useState<Set<number>>(new Set());
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isSearchModalOpen, setIsSearchModalOpen] = useState(false);

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
        const ids = new Set(holdings.map(h => h.securityId));
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

  // Auto-select first security when filter changes
  useEffect(() => {
    if (displayedSecurities.length > 0 && !selectedSecurity) {
      setSelectedSecurity(displayedSecurities[0]);
    }
  }, [holdingsSecurityIds, watchlistSecurityIds, securities]);

  // Enrich securities with holding/watchlist status
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
    // 'all' mode: holdings + watchlist securities
    return enrichedSecurities.filter(s => s.isInHoldings || s.isWatchlistOnly);
  }, [enrichedSecurities, filterMode]);

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

  // Load price data when security changes
  const loadPriceData = useCallback(async () => {
    if (!selectedSecurity) {
      setPriceData([]);
      return;
    }

    setIsLoading(true);
    try {
      let startDate: string;
      const now = new Date();

      switch (timeRange) {
        case '1M':
          startDate = new Date(now.getFullYear(), now.getMonth() - 1, now.getDate()).toISOString().split('T')[0];
          break;
        case '3M':
          startDate = new Date(now.getFullYear(), now.getMonth() - 3, now.getDate()).toISOString().split('T')[0];
          break;
        case '6M':
          startDate = new Date(now.getFullYear(), now.getMonth() - 6, now.getDate()).toISOString().split('T')[0];
          break;
        case '1Y':
          startDate = new Date(now.getFullYear() - 1, now.getMonth(), now.getDate()).toISOString().split('T')[0];
          break;
        case '2Y':
          startDate = new Date(now.getFullYear() - 2, now.getMonth(), now.getDate()).toISOString().split('T')[0];
          break;
        case '5Y':
          startDate = new Date(now.getFullYear() - 5, now.getMonth(), now.getDate()).toISOString().split('T')[0];
          break;
        case 'MAX':
        default:
          startDate = '2000-01-01';
      }

      const endDate = new Date().toISOString().split('T')[0];

      // First try to get cached data
      let data = await invoke<PriceData[]>('get_price_history', {
        securityId: selectedSecurity.id,
        startDate,
        endDate: null,
      });

      // If no data, fetch from provider (Yahoo)
      if (data.length === 0) {
        try {
          await invoke('fetch_historical_prices', {
            securityId: selectedSecurity.id,
            from: startDate,
            to: endDate,
            apiKeys: null,
          });
          // Re-fetch from cache after download
          data = await invoke<PriceData[]>('get_price_history', {
            securityId: selectedSecurity.id,
            startDate,
            endDate: null,
          });
        } catch (fetchErr) {
          console.warn('Failed to fetch historical prices from provider:', fetchErr);
        }
      }

      setPriceData(data);
    } catch (err) {
      console.error('Failed to load price data:', err);
      setPriceData([]);
    } finally {
      setIsLoading(false);
    }
  }, [selectedSecurity, timeRange]);

  useEffect(() => {
    loadPriceData();
  }, [loadPriceData]);

  // Convert to OHLC data
  const ohlcData = useMemo<OHLCData[]>(() => {
    if (priceData.length === 0) return [];
    return convertToOHLC(priceData, 1.5);
  }, [priceData]);

  // Handle search modal security added
  const handleSecurityAdded = (securityId: number) => {
    setWatchlistSecurityIds(prev => new Set([...prev, securityId]));
    loadSecurities();
  };

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
    return (
      <div className="fixed inset-0 z-50 bg-background flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-border">
          <div className="flex items-center gap-4">
            {selectedSecurity && (
              <div className="text-lg font-semibold">
                {selectedSecurity.ticker || selectedSecurity.name}
                <span className="text-muted-foreground ml-2">{selectedSecurity.currency}</span>
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
          {/* Chart */}
          <div className="flex-1 min-w-0">
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
                  height={window.innerHeight - 60}
                  theme={resolvedTheme}
                  showVolume={true}
                  symbol={selectedSecurity?.ticker || selectedSecurity?.name}
                />
              </ChartErrorBoundary>
            )}
          </div>

          {/* Indicators Panel (narrower in fullscreen) */}
          <div className="w-64 border-l border-border p-4 overflow-auto">
            <IndicatorsPanel indicators={indicators} onIndicatorsChange={setIndicators} />
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
            className="flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            <RefreshCw size={14} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
        </div>

        <div className="flex-1 flex gap-4 min-h-0">
          {/* Left Sidebar - Security Selection */}
          <div className="w-64 flex-shrink-0 flex flex-col bg-card border border-border rounded-lg overflow-hidden">
            {/* Filter Toggle */}
            <div className="p-2 border-b border-border flex gap-1">
              <button
                onClick={() => setFilterMode('holdings')}
                className={`flex-1 px-2 py-1 text-xs font-medium rounded flex items-center justify-center gap-1 transition-colors ${
                  filterMode === 'holdings'
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-muted-foreground hover:bg-muted/80'
                }`}
              >
                <Briefcase size={12} />
                Im Bestand
              </button>
              <button
                onClick={() => setFilterMode('all')}
                className={`flex-1 px-2 py-1 text-xs font-medium rounded flex items-center justify-center gap-1 transition-colors ${
                  filterMode === 'all'
                    ? 'bg-primary text-primary-foreground'
                    : 'bg-muted text-muted-foreground hover:bg-muted/80'
                }`}
              >
                <Eye size={12} />
                Alle
              </button>
            </div>

            {/* Search + Add Button */}
            <div className="p-3 border-b border-border">
              <div className="relative">
                <Search
                  size={14}
                  className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground"
                />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={e => setSearchQuery(e.target.value)}
                  placeholder="Wertpapier suchen..."
                  className="w-full pl-8 pr-10 py-1.5 text-sm bg-muted border-none rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
                />
                <button
                  onClick={() => setIsSearchModalOpen(true)}
                  className="absolute right-1.5 top-1/2 -translate-y-1/2 p-1 hover:bg-primary/10 rounded transition-colors"
                  title="Externes Wertpapier suchen"
                >
                  <Plus size={14} className="text-primary" />
                </button>
              </div>
            </div>

            {/* Securities List */}
            <div className="flex-1 overflow-auto">
              {filteredSecurities.length === 0 ? (
                <div className="p-4 text-center text-sm text-muted-foreground">
                  {filterMode === 'holdings'
                    ? 'Keine Wertpapiere im Bestand'
                    : 'Keine Wertpapiere gefunden'}
                </div>
              ) : (
                filteredSecurities.map(security => (
                  <button
                    key={security.id}
                    onClick={() => setSelectedSecurity(security)}
                    className={`w-full text-left px-3 py-2 border-b border-border/50 hover:bg-muted/50 transition-colors ${
                      selectedSecurity?.id === security.id
                        ? 'bg-primary/10 border-l-2 border-l-primary'
                        : ''
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <div className="font-medium text-sm truncate flex-1">{security.name}</div>
                      {security.isWatchlistOnly && (
                        <span className="px-1 py-0.5 text-[10px] bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400 rounded flex-shrink-0">
                          Watchlist
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {security.ticker && <span className="mr-2">{security.ticker}</span>}
                      {security.isin && <span>{security.isin}</span>}
                    </div>
                  </button>
                ))
              )}
            </div>
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
              </div>

              <div className="flex items-center gap-2">
                {selectedSecurity && (
                  <div className="text-sm">
                    <span className="font-semibold">{selectedSecurity.name}</span>
                    <span className="text-muted-foreground ml-2">{selectedSecurity.currency}</span>
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

            {/* Chart Area */}
            <div className="flex-1 bg-card border border-border rounded-lg overflow-hidden min-h-0">
              {isLoading ? (
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
                <ChartErrorBoundary>
                  <TradingViewChart
                    data={ohlcData}
                    indicators={indicators}
                    height={500}
                    theme={resolvedTheme}
                    showVolume={true}
                    symbol={selectedSecurity?.ticker || selectedSecurity?.name}
                  />
                </ChartErrorBoundary>
              )}
            </div>
          </div>

          {/* Right Sidebar - Indicators */}
          <div className="w-72 flex-shrink-0">
            <IndicatorsPanel indicators={indicators} onIndicatorsChange={setIndicators} />

            {/* Chart Info */}
            {ohlcData.length > 0 && (
              <div className="mt-4 bg-card border border-border rounded-lg p-3">
                <div className="text-xs text-muted-foreground font-medium mb-2">Chart-Info</div>
                <div className="space-y-1 text-xs">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Datenpunkte:</span>
                    <span className="font-mono">{ohlcData.length}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Zeitraum:</span>
                    <span className="font-mono">
                      {ohlcData[0]?.time} - {ohlcData[ohlcData.length - 1]?.time}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Aktuell:</span>
                    <span className="font-mono font-semibold">
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
    </>
  );
}

export default ChartsView;
