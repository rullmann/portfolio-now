/**
 * Modal for displaying security detail chart with:
 * - Price history (line chart)
 * - FIFO cost basis evolution (stepped line - Einstandskurs)
 * - Trade markers (BUY/SELL)
 *
 * Similar to Portfolio Performance's Vermögensaufstellung chart view.
 */

import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { X, Building2, TrendingUp, TrendingDown } from 'lucide-react';
import { createChart, ColorType, LineSeries, createSeriesMarkers } from 'lightweight-charts';
import type { IChartApi, ISeriesApi, LineData, Time, SeriesMarker } from 'lightweight-charts';
import type { SecurityChartData, PriceData } from '../../lib/types';
import { getPriceHistory, getFifoCostBasisHistory, fetchLogosBatch, getCachedLogoData } from '../../lib/api';
import { useSettingsStore } from '../../store';
import { formatNumber } from '../../views/utils';

interface SecurityDetailChartModalProps {
  isOpen: boolean;
  onClose: () => void;
  securityId: number;
  securityName: string;
  ticker?: string;
  isin?: string;
  currency?: string;
  customLogo?: string;
}

// Time period options
type TimePeriod = '1M' | '3M' | '6M' | 'YTD' | '1Y' | '2Y' | '5Y' | 'MAX';

const TIME_PERIODS: { value: TimePeriod; label: string }[] = [
  { value: '1M', label: '1M' },
  { value: '3M', label: '3M' },
  { value: '6M', label: '6M' },
  { value: 'YTD', label: 'YTD' },
  { value: '1Y', label: '1J' },
  { value: '2Y', label: '2J' },
  { value: '5Y', label: '5J' },
  { value: 'MAX', label: 'Max' },
];

function getDateRange(period: TimePeriod): { from: string; to: string } {
  const now = new Date();
  const to = now.toISOString().split('T')[0];
  let from: Date;

  switch (period) {
    case '1M':
      from = new Date(now.getFullYear(), now.getMonth() - 1, now.getDate());
      break;
    case '3M':
      from = new Date(now.getFullYear(), now.getMonth() - 3, now.getDate());
      break;
    case '6M':
      from = new Date(now.getFullYear(), now.getMonth() - 6, now.getDate());
      break;
    case 'YTD':
      from = new Date(now.getFullYear(), 0, 1);
      break;
    case '1Y':
      from = new Date(now.getFullYear() - 1, now.getMonth(), now.getDate());
      break;
    case '2Y':
      from = new Date(now.getFullYear() - 2, now.getMonth(), now.getDate());
      break;
    case '5Y':
      from = new Date(now.getFullYear() - 5, now.getMonth(), now.getDate());
      break;
    case 'MAX':
    default:
      from = new Date(2000, 0, 1);
      break;
  }

  return { from: from.toISOString().split('T')[0], to };
}

export function SecurityDetailChartModal({
  isOpen,
  onClose,
  securityId,
  securityName,
  ticker,
  isin,
  currency = 'EUR',
  customLogo,
}: SecurityDetailChartModalProps) {
  const [selectedPeriod, setSelectedPeriod] = useState<TimePeriod>('2Y');
  const [prices, setPrices] = useState<PriceData[]>([]);
  const [chartData, setChartData] = useState<SecurityChartData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [logoUrl, setLogoUrl] = useState<string | null>(null);
  const [showCostBasis, setShowCostBasis] = useState(true);
  const [showTrades, setShowTrades] = useState(true);

  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const priceSeriesRef = useRef<ISeriesApi<'Line'> | null>(null);
  const costBasisSeriesRef = useRef<ISeriesApi<'Line'> | null>(null);

  const brandfetchApiKey = useSettingsStore((state) => state.brandfetchApiKey);

  // Load logo
  useEffect(() => {
    if (!isOpen) {
      setLogoUrl(null);
      return;
    }

    if (customLogo) {
      setLogoUrl(customLogo);
      return;
    }

    // Try to load from cache or fetch
    const loadLogo = async () => {
      if (!brandfetchApiKey || !securityName) return;

      try {
        // Try cache first
        const cachedLogo = await getCachedLogoData(securityName);
        if (cachedLogo) {
          setLogoUrl(cachedLogo);
          return;
        }

        // Fetch from API
        const results = await fetchLogosBatch(brandfetchApiKey, [{
          id: securityId,
          name: securityName,
          ticker,
        }]);

        if (results.length > 0 && results[0].logoUrl) {
          setLogoUrl(results[0].logoUrl);
        }
      } catch (err) {
        console.warn('Failed to load logo:', err);
      }
    };

    loadLogo();
  }, [isOpen, securityId, securityName, ticker, customLogo, brandfetchApiKey]);

  // Load data when modal opens or period changes
  useEffect(() => {
    if (!isOpen) return;

    const loadData = async () => {
      setIsLoading(true);
      try {
        const { from, to } = getDateRange(selectedPeriod);

        // Load price history and FIFO data in parallel
        const [priceData, fifoData] = await Promise.all([
          getPriceHistory(securityId, from, to),
          getFifoCostBasisHistory(securityId),
        ]);

        setPrices(priceData);
        setChartData(fifoData);
      } catch (err) {
        console.error('Failed to load chart data:', err);
      } finally {
        setIsLoading(false);
      }
    };

    loadData();
  }, [isOpen, securityId, selectedPeriod]);

  // Prepare chart data
  const priceChartData = useMemo(() => {
    return prices
      .map((p) => ({
        time: p.date as Time,
        value: p.value,
      }))
      .sort((a, b) => (a.time as string).localeCompare(b.time as string));
  }, [prices]);

  // Prepare cost basis data (stepped line)
  const costBasisChartData = useMemo(() => {
    if (!chartData?.costBasisHistory) return [];

    const { from } = getDateRange(selectedPeriod);
    const fromDate = from;

    // Filter to selected period and create stepped data
    return chartData.costBasisHistory
      .filter((s) => s.date >= fromDate)
      .flatMap((s, i, arr) => {
        const points: LineData<Time>[] = [{ time: s.date as Time, value: s.costPerShare }];

        // Add a point just before the next change to create stepped effect
        if (i < arr.length - 1) {
          const nextDate = arr[i + 1].date;
          // Add point at next date - 1 day with same value
          const d = new Date(nextDate);
          d.setDate(d.getDate() - 1);
          const dayBefore = d.toISOString().split('T')[0];
          if (dayBefore > s.date) {
            points.push({ time: dayBefore as Time, value: s.costPerShare });
          }
        }

        return points;
      })
      .sort((a, b) => (a.time as string).localeCompare(b.time as string));
  }, [chartData, selectedPeriod]);

  // Prepare trade markers for chart (v5 uses createSeriesMarkers)
  const tradeMarkers = useMemo((): SeriesMarker<Time>[] => {
    if (!chartData?.trades || !showTrades) return [];

    const { from } = getDateRange(selectedPeriod);

    return chartData.trades
      .filter((t) => t.date >= from)
      .map((t) => {
        const isBuy = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN'].includes(t.txnType);
        return {
          time: t.date as Time,
          position: isBuy ? 'belowBar' : 'aboveBar',
          color: isBuy ? '#22c55e' : '#ef4444',
          shape: isBuy ? 'arrowUp' : 'arrowDown',
          text: `${t.shares.toLocaleString('de-DE', { maximumFractionDigits: 2 })}`,
          size: 2,
        } as SeriesMarker<Time>;
      });
  }, [chartData, showTrades, selectedPeriod]);

  // Calculate stats
  const stats = useMemo(() => {
    if (priceChartData.length === 0) {
      return { current: 0, first: 0, change: 0, changePercent: 0, isPositive: true };
    }

    const first = priceChartData[0].value;
    const current = priceChartData[priceChartData.length - 1].value;
    const change = current - first;
    const changePercent = first > 0 ? (change / first) * 100 : 0;

    return {
      current,
      first,
      change,
      changePercent,
      isPositive: change >= 0,
    };
  }, [priceChartData]);

  // Current cost basis
  const currentCostBasis = useMemo(() => {
    if (!chartData?.costBasisHistory?.length) return null;
    return chartData.costBasisHistory[chartData.costBasisHistory.length - 1];
  }, [chartData]);

  // Initialize and update chart
  const updateChart = useCallback(() => {
    if (!chartContainerRef.current || priceChartData.length === 0) {
      return;
    }

    // Clean up existing chart
    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
      priceSeriesRef.current = null;
      costBasisSeriesRef.current = null;
    }

    const isDark = document.documentElement.classList.contains('dark');

    // Create chart
    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: isDark ? '#9ca3af' : '#6b7280',
        fontFamily: 'system-ui, -apple-system, sans-serif',
        attributionLogo: false,
      },
      grid: {
        vertLines: { color: isDark ? '#374151' : '#e5e7eb' },
        horzLines: { color: isDark ? '#374151' : '#e5e7eb' },
      },
      width: chartContainerRef.current.clientWidth,
      height: 500,
      rightPriceScale: {
        borderVisible: false,
      },
      timeScale: {
        borderVisible: false,
        timeVisible: false,
        fixLeftEdge: true,
        fixRightEdge: true,
      },
      crosshair: {
        vertLine: {
          width: 1,
          color: isDark ? '#6b7280' : '#9ca3af',
          style: 2,
        },
        horzLine: {
          width: 1,
          color: isDark ? '#6b7280' : '#9ca3af',
          style: 2,
        },
      },
    });

    chartRef.current = chart;

    // Price line series (green/teal)
    const priceSeries = chart.addSeries(LineSeries, {
      color: '#14b8a6', // Teal
      lineWidth: 2,
      priceFormat: {
        type: 'price',
        precision: 2,
        minMove: 0.01,
      },
    });
    priceSeriesRef.current = priceSeries;
    priceSeries.setData(priceChartData);

    // Add trade markers using v5 API (createSeriesMarkers)
    if (tradeMarkers.length > 0) {
      createSeriesMarkers(priceSeries, tradeMarkers);
    }

    // Cost basis line series (magenta/pink, stepped)
    if (showCostBasis && costBasisChartData.length > 0) {
      const costBasisSeries = chart.addSeries(LineSeries, {
        color: '#d946ef', // Magenta/Fuchsia
        lineWidth: 2,
        lineStyle: 0, // Solid
        priceFormat: {
          type: 'price',
          precision: 2,
          minMove: 0.01,
        },
      });
      costBasisSeriesRef.current = costBasisSeries;
      costBasisSeries.setData(costBasisChartData);
    }

    // Fit content
    chart.timeScale().fitContent();
  }, [priceChartData, costBasisChartData, tradeMarkers, showCostBasis]);

  // Effect to update chart when data changes
  useEffect(() => {
    if (!isOpen) return;
    updateChart();

    // Handle resize
    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
        priceSeriesRef.current = null;
        costBasisSeriesRef.current = null;
      }
    };
  }, [isOpen, updateChart]);

  // Handle escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    if (isOpen) {
      window.addEventListener('keydown', handleKeyDown);
      return () => window.removeEventListener('keydown', handleKeyDown);
    }
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-card rounded-lg shadow-xl border border-border w-full max-w-6xl mx-4 max-h-[95vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-3">
            {/* Logo */}
            {logoUrl ? (
              <img
                src={logoUrl}
                alt=""
                className="w-10 h-10 rounded-lg object-contain bg-muted"
                crossOrigin="anonymous"
              />
            ) : (
              <div className="w-10 h-10 rounded-lg bg-muted flex items-center justify-center">
                <Building2 size={24} className="text-muted-foreground" />
              </div>
            )}
            <div>
              <h2 className="text-lg font-semibold">{securityName}</h2>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                {ticker && <span className="font-mono">{ticker}</span>}
                {isin && <span className="font-mono">{isin}</span>}
              </div>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 rounded-md hover:bg-accent transition-colors"
            aria-label="Schließen"
          >
            <X size={20} />
          </button>
        </div>

        {/* Stats Bar */}
        <div className="flex items-center justify-between px-4 py-3 bg-muted/30 border-b border-border">
          <div className="flex items-center gap-6">
            {/* Current Price */}
            <div>
              <div className="text-xs text-muted-foreground">Aktueller Kurs</div>
              <div className="text-lg font-bold">
                {formatNumber(stats.current)} {currency}
              </div>
            </div>

            {/* Change */}
            <div>
              <div className="text-xs text-muted-foreground">Änderung ({selectedPeriod})</div>
              <div className={`text-lg font-bold flex items-center gap-1 ${
                stats.isPositive ? 'text-green-600' : 'text-red-600'
              }`}>
                {stats.isPositive ? <TrendingUp size={18} /> : <TrendingDown size={18} />}
                {stats.isPositive ? '+' : ''}{formatNumber(stats.change)} ({stats.changePercent >= 0 ? '+' : ''}{stats.changePercent.toFixed(2)}%)
              </div>
            </div>

            {/* Cost Basis */}
            {currentCostBasis && (
              <div>
                <div className="text-xs text-muted-foreground">Einstandskurs (FIFO)</div>
                <div className="text-lg font-bold text-fuchsia-500">
                  {formatNumber(currentCostBasis.costPerShare)} {currency}
                </div>
              </div>
            )}
          </div>

          {/* Period Selector */}
          <div className="flex gap-1 bg-muted p-1 rounded-md">
            {TIME_PERIODS.map((period) => (
              <button
                key={period.value}
                onClick={() => setSelectedPeriod(period.value)}
                className={`px-3 py-1 text-sm rounded transition-colors ${
                  selectedPeriod === period.value
                    ? 'bg-background shadow-sm'
                    : 'hover:bg-background/50'
                }`}
              >
                {period.label}
              </button>
            ))}
          </div>
        </div>

        {/* Legend / Options */}
        <div className="flex items-center gap-4 px-4 py-2 border-b border-border">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={showCostBasis}
              onChange={(e) => setShowCostBasis(e.target.checked)}
              className="w-4 h-4 rounded border-border"
            />
            <span className="flex items-center gap-1.5 text-sm">
              <span className="w-3 h-0.5 bg-fuchsia-500 rounded-full"></span>
              Einstandskurs (FIFO)
            </span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={showTrades}
              onChange={(e) => setShowTrades(e.target.checked)}
              className="w-4 h-4 rounded border-border"
            />
            <span className="flex items-center gap-1.5 text-sm">
              <span className="text-green-600">▲</span>
              <span className="text-red-600">▼</span>
              Trades
            </span>
          </label>
          <div className="flex items-center gap-1.5 text-sm text-muted-foreground ml-auto">
            <span className="w-3 h-0.5 bg-teal-500 rounded-full"></span>
            Kursverlauf
          </div>
        </div>

        {/* Chart */}
        <div className="p-4">
          {isLoading ? (
            <div className="h-[500px] flex items-center justify-center">
              <div className="text-muted-foreground">Lade Daten...</div>
            </div>
          ) : priceChartData.length === 0 ? (
            <div className="h-[500px] flex items-center justify-center">
              <div className="text-center">
                <div className="text-muted-foreground mb-2">Keine Kursdaten verfügbar</div>
                <div className="text-xs text-muted-foreground">
                  Synchronisieren Sie die Kurse für dieses Wertpapier
                </div>
              </div>
            </div>
          ) : (
            <div ref={chartContainerRef} className="h-[500px]" />
          )}
        </div>

        {/* Trade History Table */}
        {chartData?.trades && chartData.trades.length > 0 && (
          <div className="border-t border-border">
            <div className="px-4 py-2 bg-muted/30">
              <h3 className="text-sm font-medium">Transaktionen ({chartData.trades.length})</h3>
            </div>
            <div className="max-h-48 overflow-y-auto">
              <table className="w-full text-sm">
                <thead className="bg-muted/30 sticky top-0">
                  <tr>
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted-foreground">Datum</th>
                    <th className="px-4 py-2 text-left text-xs font-medium text-muted-foreground">Typ</th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-muted-foreground">Stück</th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-muted-foreground">Kurs</th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-muted-foreground">Betrag</th>
                    <th className="px-4 py-2 text-right text-xs font-medium text-muted-foreground">Gebühren</th>
                  </tr>
                </thead>
                <tbody>
                  {chartData.trades.slice().reverse().map((trade, i) => {
                    const isBuy = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN'].includes(trade.txnType);
                    return (
                      <tr key={i} className="border-b border-border hover:bg-accent/30">
                        <td className="px-4 py-1.5 font-mono text-xs">
                          {new Date(trade.date).toLocaleDateString('de-DE')}
                        </td>
                        <td className="px-4 py-1.5">
                          <span className={`px-2 py-0.5 rounded text-xs font-medium ${
                            isBuy ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400'
                                  : 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400'
                          }`}>
                            {trade.txnType}
                          </span>
                        </td>
                        <td className="px-4 py-1.5 text-right tabular-nums">
                          {trade.shares.toLocaleString('de-DE', { maximumFractionDigits: 4 })}
                        </td>
                        <td className="px-4 py-1.5 text-right tabular-nums">
                          {formatNumber(trade.pricePerShare)}
                        </td>
                        <td className="px-4 py-1.5 text-right tabular-nums font-medium">
                          {formatNumber(trade.amount)} {currency}
                        </td>
                        <td className="px-4 py-1.5 text-right tabular-nums text-muted-foreground">
                          {trade.fees > 0 ? formatNumber(trade.fees) : '-'}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
