/**
 * Screener view for filtering securities by technical indicators.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
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
} from 'lucide-react';
import {
  getSecurities,
  getPriceHistory,
} from '../../lib/api';
import { convertToOHLC } from '../../lib/indicators';
import {
  runScreener,
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
import { SecurityLogo } from '../../components/common';
import { useCachedLogos, type CachedLogo } from '../../lib/hooks';
import { useSettingsStore, useUIStore } from '../../store';
import type { SecurityData as APISecurity, PriceData } from '../../lib/types';

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
// Results Table Component
// ============================================================================

function ResultsTable({
  results,
  logos,
  onSelectSecurity,
}: {
  results: ScreenerResult[];
  logos: Map<number, CachedLogo>;
  onSelectSecurity: (securityId: number) => void;
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
              <th className="text-left py-3 px-4 font-medium">Erfüllte Filter</th>
              <th className="w-10"></th>
            </tr>
          </thead>
          <tbody>
            {results.map((result) => (
              <tr
                key={result.securityId}
                className="border-b border-border last:border-0 hover:bg-muted/30 cursor-pointer"
                onClick={() => onSelectSecurity(result.securityId)}
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

  const { brandfetchApiKey } = useSettingsStore();
  const { setCurrentView, setScrollTarget } = useUIStore();

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

    const screenerResults = runScreener(securitiesData, filters);
    setResults(screenerResults);
  }, [securitiesData, filters, loadPriceData]);

  // Re-run screener when data is loaded
  useEffect(() => {
    if (securitiesData.length > 0 && filters.some((f) => f.enabled)) {
      const screenerResults = runScreener(securitiesData, filters);
      setResults(screenerResults);
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
          <button
            onClick={handleRunScreener}
            disabled={isRunning || activeFilterCount === 0}
            className="flex items-center gap-2 px-4 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            {isRunning ? (
              <>
                <RefreshCw size={16} className="animate-spin" />
                {loadingProgress.current}/{loadingProgress.total}
              </>
            ) : (
              <>
                <Play size={16} />
                Screener starten
              </>
            )}
          </button>
        </div>
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

      {/* Results */}
      {results.length > 0 ? (
        <ResultsTable
          results={results}
          logos={logos}
          onSelectSecurity={handleSelectSecurity}
        />
      ) : (
        <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
          {activeFilterCount === 0 ? (
            <>
              <Filter className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Fügen Sie Filter hinzu oder wählen Sie ein Preset.</p>
              <p className="text-sm mt-1">
                Der Screener durchsucht {securities.length} Wertpapiere.
              </p>
            </>
          ) : isLoading || isRunning ? (
            <>
              <RefreshCw className="w-12 h-12 mx-auto mb-3 opacity-50 animate-spin" />
              <p>Lade Kursdaten...</p>
              <p className="text-sm mt-1">
                {loadingProgress.current}/{loadingProgress.total} Wertpapiere
              </p>
            </>
          ) : (
            <>
              <Search className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Keine Wertpapiere erfüllen die Filterkriterien.</p>
              <p className="text-sm mt-1">
                Versuchen Sie weniger restriktive Filter.
              </p>
            </>
          )}
        </div>
      )}

      {/* Info Box */}
      <div className="text-xs text-muted-foreground p-3 bg-muted/50 rounded-lg">
        <strong>Hinweis:</strong> Der Screener analysiert Kursdaten der letzten 6 Monate.
        Wertpapiere mit weniger als 20 Datenpunkten werden übersprungen.
        Klicken Sie auf ein Ergebnis, um die detaillierte Chart-Analyse zu öffnen.
      </div>
    </div>
  );
}

export default ScreenerView;
