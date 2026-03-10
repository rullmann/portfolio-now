/**
 * Screener view for filtering securities by technical indicators.
 */

import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import {
  Search,
  Filter,
  Plus,
  Trash2,
  RefreshCw,
  TrendingUp,
  TrendingDown,
  Zap,
  ChevronRight,
  AlertCircle,
  Play,
  X,
  Globe,
  Database,
  Bot,
  Loader2,
  Square,
  AlertTriangle,
  ChevronDown,
  Info,
} from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import {
  getSecurities,
  getPriceHistory,
  getMarketIndices,
  screenMarket,
  cancelMarketScreener,
  analyzeMarketScreenerWithAi,
  addExternalSecurityToWatchlist,
  createWatchlist,
  getWatchlists,
} from '../../lib/api';
import { convertToOHLC } from '../../lib/indicators';
import {
  screenerPresets,
  indicatorLabels,
  conditionLabels,
  createFilter,
  applyPreset,
  type ScreenerFilter,
  type ScreenerResult,
  type SecurityData,
  type ScreenerIndicator,
  type ScreenerCondition,
  type ScreenerPreset,
} from '../../lib/screener';
import { runScreener, detectRegime, scoreSetup } from '../../lib/indicators-rust';
import { SecurityLogo, RegimeBadge, SetupScoreBadge, SafeMarkdown } from '../../components/common';
import type { RegimeAnalysis, SetupScore } from '../../lib/indicators';
import { useCachedLogos, type CachedLogo } from '../../lib/hooks';
import { useSettingsStore, useUIStore } from '../../store';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import type { SecurityData as APISecurity, PriceData, MarketScreenerProgress, BreakoutScore } from '../../lib/types';

// ============================================================================
// Breakout Score Components
// ============================================================================

function BreakoutBadge({ score }: { score: BreakoutScore }) {
  const colorMap: Record<BreakoutScore['classification'], string> = {
    veryLikely: 'bg-green-500/20 text-green-400',
    likely: 'bg-blue-500/20 text-blue-400',
    possible: 'bg-amber-500/20 text-amber-400',
    unlikely: 'bg-muted text-muted-foreground',
  };
  const colorClass = colorMap[score.classification];
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium ${colorClass}`}>
      {score.totalPoints}/{score.maxPoints}
      {score.downgraded && (
        <AlertTriangle size={10} className="text-amber-400 shrink-0" />
      )}
    </span>
  );
}

function BreakoutDetailPanel({ score }: { score: BreakoutScore }) {
  const [open, setOpen] = useState(false);

  return (
    <div className="mt-1">
      <button
        onClick={(e) => { e.stopPropagation(); setOpen(!open); }}
        className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
      >
        <ChevronDown size={12} className={`transition-transform ${open ? 'rotate-180' : ''}`} />
        Regeldetails
      </button>
      {open && (
        <div className="mt-2 space-y-1.5 p-3 bg-muted/50 rounded-lg border border-border text-xs" onClick={(e) => e.stopPropagation()}>
          {score.rules.map((rule, i) => (
            <div key={i} className="flex items-start gap-2">
              <div className="flex gap-0.5 mt-0.5 shrink-0">
                {Array.from({ length: rule.maxPoints }).map((_, dot) => (
                  <span
                    key={dot}
                    className={`w-2 h-2 rounded-full border ${dot < rule.points ? 'bg-primary border-primary' : 'bg-transparent border-muted-foreground/40'}`}
                  />
                ))}
              </div>
              <div className="min-w-0">
                <span className="font-medium">{rule.ruleName}</span>
                <span className="text-muted-foreground ml-1">({rule.points}/{rule.maxPoints})</span>
                {rule.description && (
                  <p className="text-muted-foreground mt-0.5">{rule.description}</p>
                )}
              </div>
            </div>
          ))}
          {score.downgraded && score.downgradeReason && (
            <div className="flex items-start gap-1.5 mt-2 pt-2 border-t border-border text-amber-400">
              <AlertTriangle size={12} className="shrink-0 mt-0.5" />
              <span>{score.downgradeReason}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Filter Builder Component
// ============================================================================

function FilterBuilder({
  filters,
  onAddFilter,
  onRemoveFilter,
  onToggleFilter,
  onApplyPreset,
}: {
  filters: ScreenerFilter[];
  onAddFilter: (filter: ScreenerFilter) => void;
  onRemoveFilter: (id: string) => void;
  onToggleFilter: (id: string) => void;
  onApplyPreset: (preset: ScreenerPreset) => void;
}) {
  const [newIndicator, setNewIndicator] = useState<ScreenerIndicator>('rsi');
  const [newCondition, setNewCondition] = useState<ScreenerCondition>('below');
  const [newValue, setNewValue] = useState('30');
  const [newValue2, setNewValue2] = useState('');
  const [showPresets, setShowPresets] = useState(false);

  const handleAddFilter = () => {
    const value = parseFloat(newValue);
    if (isNaN(value)) return;

    const value2 = newCondition === 'between' ? parseFloat(newValue2) : undefined;
    if (newCondition === 'between' && (isNaN(value2!) || value2 === undefined)) return;

    onAddFilter(createFilter(newIndicator, newCondition, value, value2));
  };

  const indicatorOptions: ScreenerIndicator[] = [
    'rsi',
    'price',
    'volume',
    'macd',
    'macd_signal',
    'macd_histogram',
    'bollinger_upper',
    'bollinger_lower',
    'bollinger_width',
    'stochastic_k',
    'stochastic_d',
    'adx',
    'di_plus',
    'di_minus',
    'sma_20',
    'sma_50',
    'sma_200',
    'change_1d',
    'change_5d',
    'change_20d',
  ];

  const conditionOptions: ScreenerCondition[] = [
    'above',
    'below',
    'between',
    'crosses_above',
    'crosses_below',
    'increasing',
    'decreasing',
  ];

  return (
    <div className="bg-card rounded-lg border border-border p-4 space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="font-semibold flex items-center gap-2">
          <Filter size={18} />
          Filter
        </h2>
        <button
          onClick={() => setShowPresets(!showPresets)}
          className="flex items-center gap-1 px-2 py-1 text-xs bg-muted rounded hover:bg-muted/80 transition-colors"
        >
          <Zap size={12} />
          Presets
        </button>
      </div>

      {/* Presets Dropdown */}
      {showPresets && (
        <div className="p-3 bg-muted rounded-lg space-y-2">
          <div className="text-xs font-medium text-muted-foreground mb-2">Preset wählen:</div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            {screenerPresets.map((preset) => (
              <button
                key={preset.id}
                onClick={() => {
                  onApplyPreset(preset);
                  setShowPresets(false);
                }}
                className="p-2 text-left bg-background rounded border border-border hover:border-primary transition-colors"
              >
                <div className="text-sm font-medium">{preset.name}</div>
                <div className="text-xs text-muted-foreground">{preset.description}</div>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Active Filters */}
      {filters.length > 0 && (
        <div className="space-y-2">
          {filters.map((filter) => (
            <div
              key={filter.id}
              className={`flex items-center gap-2 p-2 rounded border transition-colors ${
                filter.enabled
                  ? 'bg-primary/5 border-primary/20'
                  : 'bg-muted/50 border-border opacity-60'
              }`}
            >
              <button
                onClick={() => onToggleFilter(filter.id)}
                className={`w-4 h-4 rounded border flex items-center justify-center ${
                  filter.enabled
                    ? 'bg-primary border-primary text-primary-foreground'
                    : 'border-muted-foreground'
                }`}
              >
                {filter.enabled && <span className="text-xs">✓</span>}
              </button>
              <span className="flex-1 text-sm">
                <span className="font-medium">{indicatorLabels[filter.indicator]}</span>
                {' '}
                <span className="text-muted-foreground">{conditionLabels[filter.condition]}</span>
                {' '}
                <span className="font-mono">{filter.value}</span>
                {filter.value2 !== undefined && (
                  <>
                    {' - '}
                    <span className="font-mono">{filter.value2}</span>
                  </>
                )}
              </span>
              <button
                onClick={() => onRemoveFilter(filter.id)}
                className="p-1 hover:bg-destructive/10 rounded text-destructive"
              >
                <X size={14} />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Add Filter Form */}
      <div className="flex flex-wrap items-center gap-2 p-3 bg-muted/50 rounded-lg">
        <select
          value={newIndicator}
          onChange={(e) => setNewIndicator(e.target.value as ScreenerIndicator)}
          className="px-2 py-1.5 text-sm bg-background border border-border rounded focus:outline-none focus:ring-2 focus:ring-primary"
        >
          {indicatorOptions.map((ind) => (
            <option key={ind} value={ind}>
              {indicatorLabels[ind]}
            </option>
          ))}
        </select>

        <select
          value={newCondition}
          onChange={(e) => setNewCondition(e.target.value as ScreenerCondition)}
          className="px-2 py-1.5 text-sm bg-background border border-border rounded focus:outline-none focus:ring-2 focus:ring-primary"
        >
          {conditionOptions.map((cond) => (
            <option key={cond} value={cond}>
              {conditionLabels[cond]}
            </option>
          ))}
        </select>

        <input
          type="number"
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          placeholder="Wert"
          className="w-20 px-2 py-1.5 text-sm bg-background border border-border rounded focus:outline-none focus:ring-2 focus:ring-primary"
        />

        {newCondition === 'between' && (
          <>
            <span className="text-sm text-muted-foreground">und</span>
            <input
              type="number"
              value={newValue2}
              onChange={(e) => setNewValue2(e.target.value)}
              placeholder="Wert 2"
              className="w-20 px-2 py-1.5 text-sm bg-background border border-border rounded focus:outline-none focus:ring-2 focus:ring-primary"
            />
          </>
        )}

        <button
          onClick={handleAddFilter}
          className="flex items-center gap-1 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors"
        >
          <Plus size={14} />
          Filter hinzufügen
        </button>
      </div>
    </div>
  );
}

// ============================================================================
// Market Results Table Component (with Breakout column)
// ============================================================================

function MarketResultsTable({
  results,
  sortBy,
  onSortChange,
}: {
  results: ScreenerResult[];
  sortBy: 'default' | 'breakout';
  onSortChange: (sort: 'default' | 'breakout') => void;
}) {
  const formatChange = (change: number | undefined) => {
    if (change === undefined) return null;
    const isPositive = change >= 0;
    return (
      <span className={`flex items-center gap-1 ${isPositive ? 'text-green-600' : 'text-red-600'}`}>
        {isPositive ? <TrendingUp size={12} /> : <TrendingDown size={12} />}
        {isPositive ? '+' : ''}{change.toFixed(2)}%
      </span>
    );
  };

  return (
    <div className="bg-card rounded-lg border border-border overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/50">
              <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
              <th className="text-right py-3 px-4 font-medium">Kurs</th>
              <th className="text-right py-3 px-4 font-medium">1T</th>
              <th className="text-right py-3 px-4 font-medium">5T</th>
              <th className="text-right py-3 px-4 font-medium">RSI</th>
              <th className="text-left py-3 px-4 font-medium">Erfüllte Filter</th>
              <th
                className="text-center py-3 px-4 font-medium cursor-pointer hover:text-primary transition-colors select-none"
                onClick={() => onSortChange(sortBy === 'breakout' ? 'default' : 'breakout')}
                title="Nach Breakout-Score sortieren"
              >
                Breakout {sortBy === 'breakout' ? '\u2193' : ''}
              </th>
            </tr>
          </thead>
          <tbody>
            {results.map((result) => (
              <tr
                key={result.securityId}
                className="border-b border-border last:border-0 hover:bg-muted/30"
              >
                <td className="py-3 px-4">
                  <div>
                    <div className="font-medium">{result.securityName}</div>
                    <div className="text-xs text-muted-foreground">
                      {result.ticker || result.isin || '-'}
                    </div>
                  </div>
                </td>
                <td className="py-3 px-4 text-right font-mono">
                  {result.lastPrice.toFixed(2)} {result.currency}
                </td>
                <td className="py-3 px-4 text-right">
                  {formatChange(result.change1d)}
                </td>
                <td className="py-3 px-4 text-right">
                  {formatChange(result.change5d)}
                </td>
                <td className="py-3 px-4 text-right font-mono">
                  {result.currentValues.rsi !== undefined ? result.currentValues.rsi.toFixed(1) : '-'}
                </td>
                <td className="py-3 px-4">
                  <div className="flex flex-wrap gap-1">
                    {result.matchedFilters.slice(0, 2).map((filter, i) => (
                      <span
                        key={i}
                        className="px-1.5 py-0.5 text-xs bg-primary/10 text-primary rounded"
                      >
                        {filter}
                      </span>
                    ))}
                    {result.matchedFilters.length > 2 && (
                      <span className="px-1.5 py-0.5 text-xs bg-muted text-muted-foreground rounded">
                        +{result.matchedFilters.length - 2}
                      </span>
                    )}
                  </div>
                </td>
                <td className="py-3 px-4">
                  {result.breakoutScore ? (
                    <div>
                      <BreakoutBadge score={result.breakoutScore} />
                      <BreakoutDetailPanel score={result.breakoutScore} />
                    </div>
                  ) : (
                    <span className="text-xs text-muted-foreground">-</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ============================================================================
// Results Table Component
// ============================================================================

function ResultsTable({
  results,
  logos,
  onSelectSecurity,
  tradingData,
  sortBy,
  onSortChange,
}: {
  results: ScreenerResult[];
  logos: Map<number, CachedLogo>;
  onSelectSecurity?: (securityId: number) => void;
  tradingData: Record<number, { regime: RegimeAnalysis; setup: SetupScore }>;
  sortBy: 'default' | 'score';
  onSortChange: (sort: 'default' | 'score') => void;
}) {
  const formatValue = (value: number | undefined, decimals: number = 2) => {
    if (value === undefined) return '-';
    return value.toFixed(decimals);
  };

  const formatChange = (change: number | undefined) => {
    if (change === undefined) return null;
    const isPositive = change >= 0;
    return (
      <span className={`flex items-center gap-1 ${isPositive ? 'text-green-600' : 'text-red-600'}`}>
        {isPositive ? <TrendingUp size={12} /> : <TrendingDown size={12} />}
        {isPositive ? '+' : ''}{change.toFixed(2)}%
      </span>
    );
  };

  return (
    <div className="bg-card rounded-lg border border-border overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/50">
              <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
              <th className="text-right py-3 px-4 font-medium">Kurs</th>
              <th className="text-right py-3 px-4 font-medium">1T</th>
              <th className="text-right py-3 px-4 font-medium">5T</th>
              <th className="text-right py-3 px-4 font-medium">20T</th>
              <th className="text-right py-3 px-4 font-medium">RSI</th>
              <th className="text-right py-3 px-4 font-medium">ADX</th>
              <th className="text-center py-3 px-4 font-medium">Regime</th>
              <th
                className="text-center py-3 px-4 font-medium cursor-pointer hover:text-primary transition-colors"
                onClick={() => onSortChange(sortBy === 'score' ? 'default' : 'score')}
                title="Nach Score sortieren"
              >
                Score {sortBy === 'score' ? '\u2193' : ''}
              </th>
              <th className="text-left py-3 px-4 font-medium">Erfüllte Filter</th>
              <th className="w-10"></th>
            </tr>
          </thead>
          <tbody>
            {results.map((result) => (
              <tr
                key={result.securityId}
                className={`border-b border-border last:border-0 hover:bg-muted/30 ${onSelectSecurity ? 'cursor-pointer' : ''}`}
                onClick={() => onSelectSecurity?.(result.securityId)}
              >
                <td className="py-3 px-4">
                  <div className="flex items-center gap-3">
                    <SecurityLogo securityId={result.securityId} logos={logos} size={32} />
                    <div>
                      <div className="font-medium">{result.securityName}</div>
                      <div className="text-xs text-muted-foreground">
                        {result.ticker || result.isin || '-'}
                      </div>
                    </div>
                  </div>
                </td>
                <td className="py-3 px-4 text-right font-mono">
                  {formatValue(result.lastPrice)} {result.currency}
                </td>
                <td className="py-3 px-4 text-right">
                  {formatChange(result.change1d)}
                </td>
                <td className="py-3 px-4 text-right">
                  {formatChange(result.change5d)}
                </td>
                <td className="py-3 px-4 text-right">
                  {formatChange(result.change20d)}
                </td>
                <td className="py-3 px-4 text-right font-mono">
                  {formatValue(result.currentValues.rsi)}
                </td>
                <td className="py-3 px-4 text-right font-mono">
                  {formatValue(result.currentValues.adx)}
                </td>
                <td className="py-3 px-4 text-center">
                  {tradingData[result.securityId] ? (
                    <RegimeBadge regime={tradingData[result.securityId].regime.regime} />
                  ) : (
                    <span className="text-xs text-muted-foreground">-</span>
                  )}
                </td>
                <td className="py-3 px-4 text-center">
                  {tradingData[result.securityId] ? (
                    <SetupScoreBadge score={tradingData[result.securityId].setup.totalScore} />
                  ) : (
                    <span className="text-xs text-muted-foreground">-</span>
                  )}
                </td>
                <td className="py-3 px-4">
                  <div className="flex flex-wrap gap-1">
                    {result.matchedFilters.slice(0, 2).map((filter, i) => (
                      <span
                        key={i}
                        className="px-1.5 py-0.5 text-xs bg-primary/10 text-primary rounded"
                      >
                        {filter}
                      </span>
                    ))}
                    {result.matchedFilters.length > 2 && (
                      <span className="px-1.5 py-0.5 text-xs bg-muted text-muted-foreground rounded">
                        +{result.matchedFilters.length - 2}
                      </span>
                    )}
                  </div>
                </td>
                <td className="py-3 px-4">
                  <ChevronRight size={16} className="text-muted-foreground" />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function ScreenerView() {
  const [filters, setFilters] = useState<ScreenerFilter[]>([]);
  const [results, setResults] = useState<ScreenerResult[]>([]);
  const [securities, setSecurities] = useState<APISecurity[]>([]);
  const [securitiesData, setSecuritiesData] = useState<SecurityData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loadingProgress, setLoadingProgress] = useState({ current: 0, total: 0 });
  const [tradingData, setTradingData] = useState<Record<number, { regime: RegimeAnalysis; setup: SetupScore }>>({});
  const [sortBy, setSortBy] = useState<'default' | 'score'>('default');
  const [marketSortBy, setMarketSortBy] = useState<'default' | 'breakout'>('breakout');

  // Market Screener state
  const [mode, setMode] = useState<'local' | 'market'>('market');
  const [marketIndices, setMarketIndices] = useState<import('../../lib/types').MarketIndex[]>([]);
  const [selectedIndex, setSelectedIndex] = useState('dax40');
  const [marketProgress, setMarketProgress] = useState<import('../../lib/types').MarketScreenerProgress | null>(null);
  const [aiAnalysis, setAiAnalysis] = useState<string | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);

  const { brandfetchApiKey, aiEnabled, aiProvider, aiModel } = useSettingsStore();
  const { setCurrentView, setScrollTarget } = useUIStore();
  const pendingScreener = useUIStore((s) => s.pendingScreenerFilters);
  const clearPendingScreener = useUIStore((s) => s.clearPendingScreener);
  const { keys: apiKeys } = useSecureApiKeys();

  // Track whether we need to auto-run after applying chat filters
  const chatAutoRunRef = useRef<'market' | 'local' | null>(null);

  // Apply screener filters sent from ChatPanel
  useEffect(() => {
    if (!pendingScreener) return;

    const newFilters = pendingScreener.filters.map((f, i) => ({
      ...f,
      id: `chat-${i}-${Date.now()}`,
      enabled: true,
    })) as ScreenerFilter[];
    setFilters(newFilters);
    setResults([]);
    setAiAnalysis(null);

    if (pendingScreener.mode === 'market') {
      setMode('market');
      if (pendingScreener.indexId) {
        setSelectedIndex(pendingScreener.indexId);
      }
      chatAutoRunRef.current = 'market';
    } else {
      setMode('local');
      chatAutoRunRef.current = 'local';
    }

    clearPendingScreener();
  }, [pendingScreener, clearPendingScreener]);

  // Load market indices on mount
  useEffect(() => {
    getMarketIndices().then(setMarketIndices).catch(() => {});
  }, []);

  // Listen for market screener progress events
  useEffect(() => {
    const unlisten = listen<MarketScreenerProgress>('market_screener_progress', (event) => {
      setMarketProgress(event.payload);
      if (event.payload.status === 'done') {
        setTimeout(() => setMarketProgress(null), 500);
      }
    });
    return () => { unlisten.then(fn => fn()); };
  }, []);

  // Market screener run handler
  const handleRunMarketScreener = useCallback(async () => {
    setIsRunning(true);
    setError(null);
    setResults([]);
    setAiAnalysis(null);
    setMarketSortBy('breakout');
    try {
      const response = await screenMarket(selectedIndex, filters);
      setResults(response.results);
      if (response.totalErrors > 0) {
        setError(`${response.totalErrors} von ${response.totalScanned + response.totalErrors} Ticker konnten nicht geladen werden`);
      }
      // Compute regime + setup for results
      const trading: Record<number, { regime: RegimeAnalysis; setup: SetupScore }> = {};
      // For market results, regime/setup data comes from the screener engine
      setTradingData(trading);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsRunning(false);
    }
  }, [selectedIndex, filters]);

  // AI analysis of market results
  const handleAiAnalysis = useCallback(async () => {
    if (!aiEnabled || !aiProvider || !aiModel || results.length === 0) return;

    const providerKeyMap: Record<string, string> = {
      claude: 'anthropicApiKey',
      openai: 'openaiApiKey',
      gemini: 'geminiApiKey',
      perplexity: 'perplexityApiKey',
      openrouter: 'openrouterApiKey',
    };
    const apiKey = (apiKeys as Record<string, string>)[providerKeyMap[aiProvider] || ''];
    if (!apiKey) {
      setError('Kein API-Key für den KI-Anbieter hinterlegt. Bitte in den Einstellungen konfigurieren.');
      return;
    }

    setIsAnalyzing(true);
    setAiAnalysis(null);
    try {
      const indexName = marketIndices.find(i => i.id === selectedIndex)?.name || selectedIndex;
      const filterDesc = filters.filter(f => f.enabled).map(f =>
        `${indicatorLabels[f.indicator as ScreenerIndicator] || f.indicator} ${conditionLabels[f.condition as ScreenerCondition] || f.condition} ${f.value}${f.value2 ? `-${f.value2}` : ''}`
      ).join(', ') || 'Keine Filter';

      const aiResults = results.slice(0, 20).map(r => ({
        symbol: r.ticker || r.securityName,
        name: r.securityName,
        price: r.lastPrice,
        currency: r.currency || 'USD',
        change1d: r.change1d,
        change5d: r.change5d,
        change20d: r.change20d,
        rsi: r.currentValues?.rsi ?? undefined,
        matchedFilters: r.matchedFilters,
        breakoutScore: r.breakoutScore,
      }));

      const analysis = await analyzeMarketScreenerWithAi({
        provider: aiProvider,
        model: aiModel,
        apiKey,
        indexName,
        filterDescription: filterDesc,
        results: aiResults,
      });
      setAiAnalysis(analysis);

    } catch (err) {
      const errStr = err instanceof Error ? err.message : String(err);
      setError(`KI-Analyse fehlgeschlagen: ${errStr}`);
    } finally {
      setIsAnalyzing(false);
    }
  }, [aiEnabled, aiProvider, aiModel, apiKeys, results, marketIndices, selectedIndex, filters]);

  // Add market result to watchlist
  const handleAddToWatchlist = useCallback(async (result: ScreenerResult) => {
    try {
      // Ensure a watchlist exists
      let watchlists = await getWatchlists();
      if (watchlists.length === 0) {
        await createWatchlist('Watchlist');
        watchlists = await getWatchlists();
      }
      await addExternalSecurityToWatchlist(watchlists[0].id, {
        symbol: result.ticker || '',
        name: result.securityName,
        securityType: 'Equity',
        currency: result.currency,
        provider: 'YAHOO',
        providerId: result.ticker || '',
      });
      // Show brief success state
    } catch (err) {
      console.error('Failed to add to watchlist:', err);
    }
  }, []);

  // Prepare securities for logo loading
  const securitiesForLogos = useMemo(() =>
    securities.map((s) => ({
      id: s.id,
      ticker: s.ticker || undefined,
      name: s.name,
    })),
    [securities]
  );

  // Load logos
  const { logos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

  // Load all securities on mount
  useEffect(() => {
    const loadSecurities = async () => {
      try {
        setIsLoading(true);
        const data = await getSecurities();
        // Filter only active securities
        setSecurities(data.filter((s) => !s.isRetired));
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoading(false);
      }
    };
    loadSecurities();
  }, []);

  // Load price data for all securities
  const loadPriceData = useCallback(async () => {
    if (securities.length === 0) return;

    setIsRunning(true);
    setLoadingProgress({ current: 0, total: securities.length });
    setError(null);

    const sixMonthsAgo = new Date();
    sixMonthsAgo.setMonth(sixMonthsAgo.getMonth() - 6);
    const from = sixMonthsAgo.toISOString().split('T')[0];
    const to = new Date().toISOString().split('T')[0];

    const dataList: SecurityData[] = [];
    let completed = 0;

    await Promise.all(
      securities.map(async (security) => {
        try {
          const prices: PriceData[] = await getPriceHistory(security.id, from, to);
          if (prices.length >= 20) {
            const ohlcData = convertToOHLC(prices, 1.5);
            dataList.push({
              securityId: security.id,
              name: security.name,
              ticker: security.ticker || undefined,
              isin: security.isin || undefined,
              currency: security.currency || 'EUR',
              ohlcData,
            });
          }
        } catch {
          // Skip securities without price data
        } finally {
          completed++;
          setLoadingProgress({ current: completed, total: securities.length });
        }
      })
    );

    setSecuritiesData(dataList);
    setIsRunning(false);
  }, [securities]);

  // Run screener
  const handleRunScreener = useCallback(() => {
    if (securitiesData.length === 0) {
      // First load the data, then run screener
      loadPriceData().then(() => {
        // Screener will run automatically due to useEffect below
      });
      return;
    }

    const activeFilters = filters.filter((f) => f.enabled);
    if (activeFilters.length === 0) {
      setResults([]);
      return;
    }

    runScreener(securitiesData, filters).then(setResults).catch(() => setResults([]));
  }, [securitiesData, filters, loadPriceData]);

  // Auto-run screener after chat navigation applied filters
  useEffect(() => {
    if (!chatAutoRunRef.current) return;
    const runMode = chatAutoRunRef.current;
    chatAutoRunRef.current = null;

    // Use setTimeout to ensure state (filters, mode, selectedIndex) is settled
    setTimeout(() => {
      if (runMode === 'market') {
        handleRunMarketScreener();
      } else {
        handleRunScreener();
      }
    }, 100);
  }, [filters, mode, selectedIndex, handleRunMarketScreener, handleRunScreener]);

  // Re-run screener when data is loaded
  useEffect(() => {
    if (securitiesData.length > 0 && filters.some((f) => f.enabled)) {
      runScreener(securitiesData, filters).then(async (res) => {
        setResults(res);
        // Compute regime + setup for each result
        const trading: Record<number, { regime: RegimeAnalysis; setup: SetupScore }> = {};
        await Promise.all(
          res.map(async (r) => {
            const secData = securitiesData.find(s => s.securityId === r.securityId);
            if (secData && secData.ohlcData.length >= 50) {
              try {
                const [regime, setup] = await Promise.all([
                  detectRegime(secData.ohlcData),
                  scoreSetup(secData.ohlcData),
                ]);
                trading[r.securityId] = { regime, setup };
              } catch { /* ignore */ }
            }
          })
        );
        setTradingData(trading);
      }).catch(() => setResults([]));
    }
  }, [securitiesData, filters]);

  // Filter handlers
  const handleAddFilter = (filter: ScreenerFilter) => {
    setFilters((prev) => [...prev, filter]);
  };

  const handleRemoveFilter = (id: string) => {
    setFilters((prev) => prev.filter((f) => f.id !== id));
  };

  const handleToggleFilter = (id: string) => {
    setFilters((prev) =>
      prev.map((f) => (f.id === id ? { ...f, enabled: !f.enabled } : f))
    );
  };

  const handleApplyPreset = (preset: ScreenerPreset) => {
    const presetFilters = applyPreset(preset);
    setFilters(presetFilters);
  };

  const handleClearFilters = () => {
    setFilters([]);
    setResults([]);
  };

  const handleSelectSecurity = (securityId: number) => {
    // Navigate to Charts view with this security selected
    setScrollTarget(securityId.toString());
    setCurrentView('charts');
  };

  const activeFilterCount = filters.filter((f) => f.enabled).length;

  // Sort results (local screener)
  const sortedResults = useMemo(() => {
    if (sortBy === 'score') {
      return [...results].sort((a, b) => {
        const scoreA = tradingData[a.securityId]?.setup.totalScore ?? 0;
        const scoreB = tradingData[b.securityId]?.setup.totalScore ?? 0;
        return scoreB - scoreA;
      });
    }
    return results;
  }, [results, sortBy, tradingData]);

  // Sort market results by breakout score
  const sortedMarketResults = useMemo(() => {
    if (marketSortBy === 'breakout') {
      return [...results].sort((a, b) => {
        const scoreA = a.breakoutScore?.totalPoints ?? -1;
        const scoreB = b.breakoutScore?.totalPoints ?? -1;
        return scoreB - scoreA;
      });
    }
    return results;
  }, [results, marketSortBy]);

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Search className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Screener</h1>
          {results.length > 0 && (
            <span className="px-2 py-0.5 text-sm bg-primary/10 text-primary rounded">
              {results.length} Treffer
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {filters.length > 0 && (
            <button
              onClick={handleClearFilters}
              className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
            >
              <Trash2 size={16} />
              Filter löschen
            </button>
          )}
          {mode === 'market' && results.length > 0 && aiEnabled && (
            <button
              onClick={handleAiAnalysis}
              disabled={isAnalyzing}
              className="flex items-center gap-2 px-3 py-1.5 text-sm border border-primary/50 text-primary rounded-md hover:bg-primary/10 transition-colors disabled:opacity-50"
            >
              {isAnalyzing ? <Loader2 size={16} className="animate-spin" /> : <Bot size={16} />}
              KI-Analyse
            </button>
          )}
          <button
            onClick={mode === 'market' ? handleRunMarketScreener : handleRunScreener}
            disabled={isRunning || activeFilterCount === 0}
            className="flex items-center gap-2 px-4 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isRunning ? (
              <>
                <RefreshCw size={16} className="animate-spin" />
                {mode === 'market' && marketProgress
                  ? `${marketProgress.current}/${marketProgress.total}`
                  : `${loadingProgress.current}/${loadingProgress.total}`}
              </>
            ) : (
              <>
                <Play size={16} />
                Screener starten
              </>
            )}
          </button>
          {isRunning && mode === 'market' && (
            <button
              onClick={() => cancelMarketScreener()}
              className="flex items-center gap-1 px-3 py-1.5 text-sm border border-destructive/50 text-destructive rounded-md hover:bg-destructive/10 transition-colors"
            >
              <Square size={14} />
              Stopp
            </button>
          )}
        </div>
      </div>

      {/* Mode Toggle + Index Selector */}
      <div className="flex items-center gap-4">
        <div className="flex gap-1 p-1 bg-muted rounded-lg">
          <button
            onClick={() => { setMode('local'); setResults([]); setAiAnalysis(null); }}
            className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${
              mode === 'local'
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <Database size={14} />
            Meine Wertpapiere
          </button>
          <button
            onClick={() => { setMode('market'); setResults([]); setAiAnalysis(null); }}
            className={`flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors ${
              mode === 'market'
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            <Globe size={14} />
            Markt-Screener
          </button>
        </div>

        {mode === 'market' && (
          <div className="flex items-center gap-2">
            <select
              value={selectedIndex}
              onChange={(e) => { setSelectedIndex(e.target.value); setResults([]); setAiAnalysis(null); }}
              className="px-3 py-1.5 text-sm bg-card border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
            >
              {marketIndices.map((idx) => (
                <option key={idx.id} value={idx.id}>
                  {idx.name} ({idx.tickerCount} Aktien)
                </option>
              ))}
            </select>
            {marketProgress && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 size={14} className="animate-spin text-primary" />
                <span>{marketProgress.currentSymbol}</span>
                <span className="text-xs">({marketProgress.current}/{marketProgress.total})</span>
              </div>
            )}
          </div>
        )}
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm flex items-center gap-2">
          <AlertCircle size={16} />
          {error}
        </div>
      )}

      {/* Filter Builder */}
      <FilterBuilder
        filters={filters}
        onAddFilter={handleAddFilter}
        onRemoveFilter={handleRemoveFilter}
        onToggleFilter={handleToggleFilter}
        onApplyPreset={handleApplyPreset}
      />

      {/* AI Analysis Panel */}
      {aiAnalysis && (
        <div className="bg-card rounded-lg border border-primary/30 p-4">
          <div className="flex items-center gap-2 mb-3">
            <Bot size={18} className="text-primary" />
            <h3 className="font-semibold text-sm">KI-Analyse &amp; Handlungsempfehlung</h3>
            <button onClick={() => setAiAnalysis(null)} className="ml-auto p-1 hover:bg-muted rounded">
              <X size={14} />
            </button>
          </div>
          <div className="flex items-start gap-2 mb-3 p-2.5 bg-muted/60 rounded-md text-xs text-muted-foreground">
            <Info size={14} className="shrink-0 mt-0.5 text-primary/70" />
            <span>Die KI analysiert die Breakout-Kandidaten und gibt konkrete Handlungsempfehlungen basierend auf dem 6-Regeln-System.</span>
          </div>
          <div className="prose prose-sm dark:prose-invert max-w-none">
            <SafeMarkdown>{aiAnalysis}</SafeMarkdown>
          </div>
        </div>
      )}


      {/* Results */}
      {(mode === 'market' ? sortedMarketResults : sortedResults).length > 0 ? (
        <>
          {mode === 'market' ? (
            <MarketResultsTable
              results={sortedMarketResults}
              sortBy={marketSortBy}
              onSortChange={setMarketSortBy}
            />
          ) : (
            <ResultsTable
              results={sortedResults}
              logos={logos}
              onSelectSecurity={handleSelectSecurity}
              tradingData={tradingData}
              sortBy={sortBy}
              onSortChange={setSortBy}
            />
          )}
          {/* Add to Watchlist for market results */}
          {mode === 'market' && (
            <div className="bg-card rounded-lg border border-border overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border bg-muted/50">
                    <th className="text-left py-2 px-4 font-semibold">Ergebnisse zur Watchlist hinzufügen</th>
                  </tr>
                </thead>
                <tbody>
                  {sortedMarketResults.map((r) => (
                    <tr key={r.securityId} className="border-b border-border/50 hover:bg-muted/30">
                      <td className="py-2 px-4 flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <span className="font-medium">{r.ticker || r.securityName}</span>
                          <span className="text-muted-foreground text-xs">{r.securityName}</span>
                          <span className="text-xs">{r.lastPrice.toFixed(2)} {r.currency || ''}</span>
                          {r.change1d != null && (
                            <span className={`text-xs ${r.change1d >= 0 ? 'text-green-500' : 'text-red-500'}`}>
                              {r.change1d >= 0 ? '+' : ''}{r.change1d.toFixed(1)}%
                            </span>
                          )}
                          {r.breakoutScore && (
                            <BreakoutBadge score={r.breakoutScore} />
                          )}
                        </div>
                        <button
                          onClick={() => handleAddToWatchlist(r)}
                          className="flex items-center gap-1.5 px-3 py-1 text-xs font-medium text-primary border border-primary/50 rounded hover:bg-primary/10 transition-colors"
                        >
                          <Plus size={12} />
                          Watchlist
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      ) : (
        <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
          {activeFilterCount === 0 ? (
            <>
              <Filter className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Füge Filter hinzu oder wähle ein Preset.</p>
              <p className="text-sm mt-1">
                {mode === 'market'
                  ? `Der Markt-Screener scannt den ${marketIndices.find(i => i.id === selectedIndex)?.name || selectedIndex} live via Yahoo Finance.`
                  : `Der Screener durchsucht ${securities.length} Wertpapiere.`}
              </p>
            </>
          ) : isLoading || isRunning ? (
            <>
              <RefreshCw className="w-12 h-12 mx-auto mb-3 opacity-50 animate-spin" />
              <p>{mode === 'market' ? 'Scanne Marktdaten...' : 'Lade Kursdaten...'}</p>
              <p className="text-sm mt-1">
                {mode === 'market' && marketProgress
                  ? `${marketProgress.currentSymbol} (${marketProgress.current}/${marketProgress.total})`
                  : `${loadingProgress.current}/${loadingProgress.total} Wertpapiere`}
              </p>
            </>
          ) : (
            <>
              <Search className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Keine Wertpapiere erfüllen die Filterkriterien.</p>
              <p className="text-sm mt-1">
                Versuche weniger restriktive Filter.
              </p>
            </>
          )}
        </div>
      )}

      {/* Info Box */}
      <div className="text-xs text-muted-foreground p-3 bg-muted/50 rounded-lg">
        <strong>Hinweis:</strong> {mode === 'market'
          ? 'Der Markt-Screener lädt live Kursdaten via Yahoo Finance (6 Monate). Die Daten werden nicht gespeichert.'
          : 'Der Screener analysiert Kursdaten der letzten 6 Monate. Wertpapiere mit weniger als 20 Datenpunkten werden übersprungen.'}
        {' '}Klicke auf ein Ergebnis, um die detaillierte Chart-Analyse zu öffnen.
      </div>
    </div>
  );
}

export default ScreenerView;
