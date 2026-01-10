/**
 * Modal for displaying security price chart with TradingView Lightweight Charts.
 * Best practices based on Yahoo Finance and Apple Stocks.
 */

import { useState, useEffect, useMemo, useRef } from 'react';
import { X, TrendingUp, TrendingDown, Building2, LineChart, Table2, Sparkles, RefreshCw, ChevronDown, ChevronUp } from 'lucide-react';
import { createChart, ColorType, AreaSeries, type IChartApi, type ISeriesApi, type AreaData, type Time } from 'lightweight-charts';
import { invoke } from '@tauri-apps/api/core';
import ReactMarkdown from 'react-markdown';
import type { SecurityData, PriceData } from '../../lib/types';
import { getPriceHistory, fetchLogosBatch, getCachedLogoData, fetchHistoricalPrices } from '../../lib/api';
import { useSettingsStore } from '../../store';

interface ChartAnalysisResponse {
  analysis: string;
  provider: string;
  model: string;
  tokensUsed?: number;
}

interface SecurityPriceModalProps {
  isOpen: boolean;
  onClose: () => void;
  security: SecurityData | null;
}

// View mode
type ViewMode = 'chart' | 'table';

// Time period options (like Yahoo Finance, Apple Stocks)
type TimePeriod = '1W' | '1M' | '3M' | '6M' | 'YTD' | '1Y' | '2Y' | '5Y' | 'MAX';

const TIME_PERIODS: { value: TimePeriod; label: string }[] = [
  { value: '1W', label: '1W' },
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
    case '1W':
      from = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
      break;
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

export function SecurityPriceModal({ isOpen, onClose, security }: SecurityPriceModalProps) {
  const [viewMode, setViewMode] = useState<ViewMode>('chart');
  const [selectedPeriod, setSelectedPeriod] = useState<TimePeriod>('1Y');
  const [prices, setPrices] = useState<PriceData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logoUrl, setLogoUrl] = useState<string | null>(null);

  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const seriesRef = useRef<ISeriesApi<'Area'> | null>(null);

  const brandfetchApiKey = useSettingsStore((state) => state.brandfetchApiKey);
  const finnhubApiKey = useSettingsStore((state) => state.finnhubApiKey);
  const coingeckoApiKey = useSettingsStore((state) => state.coingeckoApiKey);
  const aiProvider = useSettingsStore((state) => state.aiProvider);
  const aiModel = useSettingsStore((state) => state.aiModel);
  const anthropicApiKey = useSettingsStore((state) => state.anthropicApiKey);
  const openaiApiKey = useSettingsStore((state) => state.openaiApiKey);
  const geminiApiKey = useSettingsStore((state) => state.geminiApiKey);

  // AI Analysis state
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<string | null>(null);
  const [analysisInfo, setAnalysisInfo] = useState<{ provider: string; model: string; tokens?: number } | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [isAiCollapsed, setIsAiCollapsed] = useState(true);

  // Get API key for selected provider
  const aiApiKey = useMemo(() => {
    switch (aiProvider) {
      case 'claude': return anthropicApiKey;
      case 'openai': return openaiApiKey;
      case 'gemini': return geminiApiKey;
    }
  }, [aiProvider, anthropicApiKey, openaiApiKey, geminiApiKey]);

  const providerName = useMemo(() => {
    switch (aiProvider) {
      case 'claude': return 'Claude';
      case 'openai': return 'GPT-4';
      case 'gemini': return 'Gemini';
    }
  }, [aiProvider]);

  // Reset AI analysis when modal closes or security changes
  useEffect(() => {
    if (!isOpen) {
      setAnalysis(null);
      setAnalysisInfo(null);
      setAnalysisError(null);
      setIsAiCollapsed(true);
    }
  }, [isOpen, security?.id]);

  const handleAnalyze = async () => {
    if (!chartContainerRef.current || !security || !aiApiKey) return;

    setIsAnalyzing(true);
    setAnalysisError(null);
    setIsAiCollapsed(false);

    try {
      // Find all canvases in the chart container
      const canvases = chartContainerRef.current.querySelectorAll('canvas');
      if (canvases.length === 0) throw new Error('Chart canvas nicht gefunden');

      // Create combined canvas with proper device pixel ratio handling
      const containerRect = chartContainerRef.current.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;

      const combinedCanvas = document.createElement('canvas');
      combinedCanvas.width = containerRect.width * dpr;
      combinedCanvas.height = containerRect.height * dpr;

      const ctx = combinedCanvas.getContext('2d');
      if (!ctx) throw new Error('Canvas context nicht verfügbar');

      ctx.scale(dpr, dpr);

      const isDark = document.documentElement.classList.contains('dark');
      ctx.fillStyle = isDark ? '#1f2937' : '#ffffff';
      ctx.fillRect(0, 0, containerRect.width, containerRect.height);

      // Draw each canvas layer at correct position
      canvases.forEach((canvas) => {
        const rect = canvas.getBoundingClientRect();
        const x = rect.left - containerRect.left;
        const y = rect.top - containerRect.top;
        ctx.drawImage(
          canvas,
          0, 0, canvas.width, canvas.height,
          x, y, rect.width, rect.height
        );
      });

      const imageBase64 = combinedCanvas.toDataURL('image/png').split(',')[1];

      const context = {
        securityName: security.name,
        ticker: security.ticker,
        currency: security.currency,
        currentPrice: stats.latestPrice,
        timeframe: selectedPeriod,
        indicators: [],
      };

      const result = await invoke<ChartAnalysisResponse>('analyze_chart_with_ai', {
        request: { imageBase64, provider: aiProvider, model: aiModel, apiKey: aiApiKey, context },
      });

      setAnalysis(result.analysis);
      setAnalysisInfo({ provider: result.provider, model: result.model, tokens: result.tokensUsed });
    } catch (err) {
      setAnalysisError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsAnalyzing(false);
    }
  };

  // Load logo when modal opens
  useEffect(() => {
    if (!isOpen || !security) {
      setLogoUrl(null);
      return;
    }

    // Use custom logo if available
    if (security.customLogo) {
      setLogoUrl(security.customLogo);
      return;
    }

    // Try to fetch from Brandfetch
    const loadLogo = async () => {
      if (!brandfetchApiKey) return;

      try {
        const results = await fetchLogosBatch(brandfetchApiKey, [
          { id: security.id, ticker: security.ticker || undefined, name: security.name || '' },
        ]);

        if (results.length > 0 && results[0].logoUrl && results[0].domain) {
          const cached = await getCachedLogoData(results[0].domain);
          setLogoUrl(cached || results[0].logoUrl);
        }
      } catch (err) {
        console.error('Failed to load logo:', err);
      }
    };

    loadLogo();
  }, [isOpen, security, brandfetchApiKey]);

  // Load prices when modal opens or period changes
  useEffect(() => {
    if (!isOpen || !security) {
      setPrices([]);
      return;
    }

    const loadPrices = async () => {
      setIsLoading(true);
      setError(null);

      try {
        const { from, to } = getDateRange(selectedPeriod);

        // First try to get prices from database
        let data = await getPriceHistory(security.id, from, to);

        // If insufficient data (less than 5 points), try fetching from provider
        if (data.length < 5 && security.feed) {
          try {
            const apiKeys = {
              finnhub: finnhubApiKey || undefined,
              coingecko: coingeckoApiKey || undefined,
            };

            // Fetch historical prices from provider (also saves to DB)
            await fetchHistoricalPrices(security.id, from, to, apiKeys);

            // Reload from database
            data = await getPriceHistory(security.id, from, to);
          } catch (fetchErr) {
            // If fetching fails, continue with whatever data we have
            console.warn('Failed to fetch historical prices:', fetchErr);
          }
        }

        setPrices(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoading(false);
      }
    };

    loadPrices();
  }, [isOpen, security, selectedPeriod, finnhubApiKey, coingeckoApiKey]);

  // Process chart data - NO conversion needed, backend already converts!
  const chartData = useMemo(() => {
    if (!prices || prices.length === 0) return [];

    return prices
      .map((p) => ({
        time: p.date as Time,
        value: p.value, // Already decimal from backend
      }))
      .sort((a, b) => (a.time as string).localeCompare(b.time as string));
  }, [prices]);

  // Calculate statistics
  const stats = useMemo(() => {
    if (chartData.length === 0) {
      return { latestPrice: 0, firstPrice: 0, change: 0, changePercent: 0, isPositive: true, min: 0, max: 0 };
    }

    const latestPrice = chartData[chartData.length - 1]?.value || 0;
    const firstPrice = chartData[0]?.value || 0;
    const change = latestPrice - firstPrice;
    const changePercent = firstPrice > 0 ? (change / firstPrice) * 100 : 0;
    const isPositive = change >= 0;
    const min = Math.min(...chartData.map((d) => d.value));
    const max = Math.max(...chartData.map((d) => d.value));

    return { latestPrice, firstPrice, change, changePercent, isPositive, min, max };
  }, [chartData]);

  // Initialize and update chart
  useEffect(() => {
    if (!isOpen || viewMode !== 'chart' || !chartContainerRef.current || chartData.length === 0) {
      return;
    }

    // Clean up existing chart
    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
      seriesRef.current = null;
    }

    // Detect dark mode
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
      height: 350,
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
      handleScroll: false,
      handleScale: false,
    });

    chartRef.current = chart;

    // Create area series with gradient
    const areaColor = stats.isPositive ? '#22c55e' : '#ef4444';
    const areaSeries = chart.addSeries(AreaSeries, {
      lineColor: areaColor,
      lineWidth: 2,
      topColor: stats.isPositive ? 'rgba(34, 197, 94, 0.4)' : 'rgba(239, 68, 68, 0.4)',
      bottomColor: stats.isPositive ? 'rgba(34, 197, 94, 0.0)' : 'rgba(239, 68, 68, 0.0)',
      priceFormat: {
        type: 'price',
        precision: 2,
        minMove: 0.01,
      },
    });

    seriesRef.current = areaSeries as ISeriesApi<'Area'>;

    // Set data
    areaSeries.setData(chartData as AreaData<Time>[]);

    // Fit content
    chart.timeScale().fitContent();

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
        seriesRef.current = null;
      }
    };
  }, [isOpen, viewMode, chartData, stats.isPositive]);

  if (!isOpen) return null;

  const currency = security?.currency || 'EUR';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-card rounded-lg shadow-xl border border-border w-full max-w-3xl mx-4 max-h-[90vh] overflow-hidden">
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
              <h2 className="text-lg font-semibold">{security?.name || 'Kursverlauf'}</h2>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                {security?.ticker && <span className="font-mono">{security.ticker}</span>}
                {security?.isin && <span className="font-mono">{security.isin}</span>}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {/* View mode tabs */}
            <div className="flex gap-1 bg-muted p-1 rounded-md">
              <button
                onClick={() => setViewMode('chart')}
                className={`flex items-center gap-1.5 px-3 py-1 text-sm rounded transition-colors ${
                  viewMode === 'chart'
                    ? 'bg-background shadow-sm'
                    : 'hover:bg-background/50'
                }`}
              >
                <LineChart size={14} />
                Chart
              </button>
              <button
                onClick={() => setViewMode('table')}
                className={`flex items-center gap-1.5 px-3 py-1 text-sm rounded transition-colors ${
                  viewMode === 'table'
                    ? 'bg-background shadow-sm'
                    : 'hover:bg-background/50'
                }`}
              >
                <Table2 size={14} />
                Kurse
              </button>
            </div>
            <button
              onClick={onClose}
              className="p-2 hover:bg-muted rounded-md transition-colors"
            >
              <X size={20} />
            </button>
          </div>
        </div>

        {viewMode === 'chart' ? (
          <>
            {/* Price Info */}
            {chartData.length > 0 && (
              <div className="p-4 border-b border-border">
                <div className="flex items-baseline gap-4">
                  <span className="text-3xl font-bold tabular-nums">
                    {stats.latestPrice.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    <span className="text-lg font-normal text-muted-foreground ml-1">{currency}</span>
                  </span>
                  <div className={`flex items-center gap-1 ${stats.isPositive ? 'text-green-500' : 'text-red-500'}`}>
                    {stats.isPositive ? <TrendingUp size={20} /> : <TrendingDown size={20} />}
                    <span className="font-medium tabular-nums">
                      {stats.isPositive ? '+' : ''}{stats.change.toFixed(2)} ({stats.isPositive ? '+' : ''}{stats.changePercent.toFixed(2)}%)
                    </span>
                  </div>
                </div>
                <p className="text-sm text-muted-foreground mt-1">
                  {TIME_PERIODS.find(p => p.value === selectedPeriod)?.label} Zeitraum
                </p>
              </div>
            )}

            {/* Time Period Selector */}
            <div className="flex gap-1 p-4 bg-muted/30 overflow-x-auto">
              {TIME_PERIODS.map((period) => (
                <button
                  key={period.value}
                  onClick={() => setSelectedPeriod(period.value)}
                  className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors whitespace-nowrap ${
                    selectedPeriod === period.value
                      ? 'bg-primary text-primary-foreground'
                      : 'hover:bg-muted'
                  }`}
                >
                  {period.label}
                </button>
              ))}
            </div>

            {/* Chart Area */}
            <div className="p-4">
              {isLoading ? (
                <div className="flex items-center justify-center h-[350px] text-muted-foreground">
                  <div className="animate-pulse">Lade Kursdaten...</div>
                </div>
              ) : error ? (
                <div className="flex items-center justify-center h-[350px] text-destructive">
                  {error}
                </div>
              ) : chartData.length === 0 ? (
                <div className="flex items-center justify-center h-[350px] text-muted-foreground">
                  Keine Kursdaten für diesen Zeitraum verfügbar
                </div>
              ) : (
                <div ref={chartContainerRef} className="w-full" />
              )}
            </div>

            {/* Footer with stats */}
            {chartData.length > 0 && (
              <div className="p-4 border-t border-border bg-muted/30">
                <div className="grid grid-cols-4 gap-4 text-sm">
                  <div>
                    <p className="text-muted-foreground">Eröffnung</p>
                    <p className="font-medium tabular-nums">{stats.firstPrice.toLocaleString('de-DE', { minimumFractionDigits: 2 })} {currency}</p>
                  </div>
                  <div>
                    <p className="text-muted-foreground">Hoch</p>
                    <p className="font-medium text-green-600 tabular-nums">{stats.max.toLocaleString('de-DE', { minimumFractionDigits: 2 })} {currency}</p>
                  </div>
                  <div>
                    <p className="text-muted-foreground">Tief</p>
                    <p className="font-medium text-red-600 tabular-nums">{stats.min.toLocaleString('de-DE', { minimumFractionDigits: 2 })} {currency}</p>
                  </div>
                  <div>
                    <p className="text-muted-foreground">Aktuell</p>
                    <p className="font-medium tabular-nums">{stats.latestPrice.toLocaleString('de-DE', { minimumFractionDigits: 2 })} {currency}</p>
                  </div>
                </div>
              </div>
            )}

            {/* AI Analysis Section */}
            {aiApiKey && chartData.length > 0 && (
              <div className="border-t border-border">
                {/* AI Header */}
                <div className="flex items-center justify-between p-3 bg-muted/30">
                  <button
                    onClick={() => setIsAiCollapsed(!isAiCollapsed)}
                    className="flex items-center gap-2 hover:text-primary transition-colors"
                  >
                    <Sparkles size={16} className="text-primary" />
                    <span className="font-medium text-sm">KI-Analyse</span>
                    <span className="text-xs text-muted-foreground px-2 py-0.5 bg-muted rounded">
                      {providerName}
                    </span>
                    {isAiCollapsed ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
                  </button>
                  <button
                    onClick={handleAnalyze}
                    disabled={isAnalyzing}
                    className="flex items-center gap-2 px-3 py-1 text-xs bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 transition-colors"
                  >
                    {isAnalyzing ? (
                      <>
                        <RefreshCw size={12} className="animate-spin" />
                        Analysiere...
                      </>
                    ) : (
                      <>
                        <Sparkles size={12} />
                        Analysieren
                      </>
                    )}
                  </button>
                </div>

                {/* AI Content */}
                {!isAiCollapsed && (
                  <div className="p-4 h-48 overflow-y-auto border-t border-border">
                    {analysisError ? (
                      <div className="text-destructive text-sm">{analysisError}</div>
                    ) : isAnalyzing ? (
                      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                        <RefreshCw size={24} className="animate-spin mb-2 opacity-50" />
                        <p className="text-sm">Analyse wird erstellt...</p>
                      </div>
                    ) : analysis ? (
                      <div className="prose prose-sm dark:prose-invert max-w-none prose-headings:text-sm prose-headings:font-semibold prose-headings:mt-2 prose-headings:mb-1 prose-p:my-1 prose-ul:my-1 prose-li:my-0">
                        <ReactMarkdown>{analysis}</ReactMarkdown>
                      </div>
                    ) : (
                      <div className="flex flex-col items-center justify-center h-full text-muted-foreground text-sm">
                        <Sparkles size={24} className="mb-2 opacity-30" />
                        <p>Klicke "Analysieren" für KI-Einschätzung</p>
                      </div>
                    )}
                  </div>
                )}

                {/* AI Footer */}
                {!isAiCollapsed && (
                  <div className="px-4 py-2 border-t border-border bg-muted/30 flex items-center justify-between">
                    <span className="text-xs text-muted-foreground">Keine Anlageberatung</span>
                    {analysisInfo && (
                      <span className="text-xs text-muted-foreground">
                        {analysisInfo.model}
                        {analysisInfo.tokens && ` | ${analysisInfo.tokens.toLocaleString()} Tokens`}
                      </span>
                    )}
                  </div>
                )}
              </div>
            )}
          </>
        ) : (
          /* Table View - Price History */
          <div className="flex flex-col max-h-[500px]">
            {/* Table Header */}
            <div className="flex items-center justify-between p-4 border-b border-border bg-muted/30">
              <span className="text-sm text-muted-foreground">
                {prices.length} Kursbuchungen
              </span>
              {/* Time Period Selector for Table too */}
              <div className="flex gap-1 overflow-x-auto">
                {TIME_PERIODS.map((period) => (
                  <button
                    key={period.value}
                    onClick={() => setSelectedPeriod(period.value)}
                    className={`px-2 py-1 text-xs font-medium rounded transition-colors whitespace-nowrap ${
                      selectedPeriod === period.value
                        ? 'bg-primary text-primary-foreground'
                        : 'hover:bg-muted'
                    }`}
                  >
                    {period.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Scrollable Table */}
            <div className="flex-1 overflow-y-auto">
              {isLoading ? (
                <div className="flex items-center justify-center h-48 text-muted-foreground">
                  <div className="animate-pulse">Lade Kursdaten...</div>
                </div>
              ) : error ? (
                <div className="flex items-center justify-center h-48 text-destructive">
                  {error}
                </div>
              ) : prices.length === 0 ? (
                <div className="flex items-center justify-center h-48 text-muted-foreground">
                  Keine Kursdaten für diesen Zeitraum verfügbar
                </div>
              ) : (
                <table className="w-full text-sm">
                  <thead className="sticky top-0 bg-card border-b border-border">
                    <tr>
                      <th className="text-left py-2 px-4 font-medium">Datum</th>
                      <th className="text-right py-2 px-4 font-medium">Kurs</th>
                    </tr>
                  </thead>
                  <tbody>
                    {[...prices]
                      .sort((a, b) => b.date.localeCompare(a.date))
                      .map((price, index) => (
                        <tr
                          key={`${price.date}-${index}`}
                          className="border-b border-border/50 last:border-0 hover:bg-muted/30"
                        >
                          <td className="py-2 px-4 text-muted-foreground">
                            {new Date(price.date).toLocaleDateString('de-DE', {
                              weekday: 'short',
                              day: '2-digit',
                              month: '2-digit',
                              year: 'numeric',
                            })}
                          </td>
                          <td className="py-2 px-4 text-right font-mono tabular-nums">
                            {price.value.toLocaleString('de-DE', {
                              minimumFractionDigits: 2,
                              maximumFractionDigits: 4,
                            })}{' '}
                            <span className="text-muted-foreground">{currency}</span>
                          </td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
