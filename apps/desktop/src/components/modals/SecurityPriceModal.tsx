/**
 * Modal for displaying security price chart with TradingView Lightweight Charts.
 * Redesigned with larger layout, modern styling, and per-feature AI provider.
 */

import { useState, useEffect, useMemo, useRef } from 'react';
import { X, TrendingUp, TrendingDown, Building2, LineChart, Table2, Sparkles, RefreshCw, ChevronDown, ChevronUp, RotateCcw } from 'lucide-react';
import { createChart, ColorType, AreaSeries, type IChartApi, type ISeriesApi, type AreaData, type Time } from 'lightweight-charts';
import { invoke } from '@tauri-apps/api/core';
import { SafeMarkdown } from '../common/SafeMarkdown';
import { AIModelSelector } from '../common/AIModelSelector';
import { formatDate, type SecurityData, type PriceData } from '../../lib/types';
import { getPriceHistory, fetchLogosBatch, getCachedLogoData, fetchHistoricalPrices, forceReloadHistoricalPrices } from '../../lib/api';
import { useSettingsStore, type AiProvider } from '../../store';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { useEscapeKey } from '../../lib/hooks';

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
  useEscapeKey(isOpen, onClose);

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
  const alphaVantageApiKey = useSettingsStore((state) => state.alphaVantageApiKey);
  const twelveDataApiKey = useSettingsStore((state) => state.twelveDataApiKey);

  // Per-feature AI settings (chartAnalysis)
  const { aiFeatureSettings, aiEnabled } = useSettingsStore();
  const chartAnalysisConfig = aiFeatureSettings.chartAnalysis;

  // Temporary model selection (not persisted unless "save as default" is checked)
  const [tempSelection, setTempSelection] = useState<{ provider: AiProvider; model: string } | null>(null);

  const aiProvider = (tempSelection?.provider ?? chartAnalysisConfig.provider) as AiProvider;
  const aiModel = tempSelection?.model ?? chartAnalysisConfig.model;

  // Secure API keys
  const { keys } = useSecureApiKeys();

  // Get API key for selected provider (from secure storage)
  const aiApiKey = useMemo(() => {
    switch (aiProvider) {
      case 'claude': return keys.anthropicApiKey;
      case 'openai': return keys.openaiApiKey;
      case 'gemini': return keys.geminiApiKey;
      case 'perplexity': return keys.perplexityApiKey;
    }
  }, [aiProvider, keys]);

  // AI Analysis state
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<string | null>(null);
  const [analysisInfo, setAnalysisInfo] = useState<{ provider: string; model: string; tokens?: number } | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [isAiCollapsed, setIsAiCollapsed] = useState(true);

  // Force reload state
  const [isForceReloading, setIsForceReloading] = useState(false);
  const [forceReloadMessage, setForceReloadMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  // Reset AI analysis when modal closes or security changes
  useEffect(() => {
    if (!isOpen) {
      setAnalysis(null);
      setAnalysisInfo(null);
      setAnalysisError(null);
      setIsAiCollapsed(true);
      setTempSelection(null);
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
      ctx.fillStyle = isDark ? '#0d1117' : '#ffffff';
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

  // Force reload historical prices (delete + reload)
  const handleForceReload = async () => {
    if (!security) return;

    if (!confirm(`Alle historischen Kurse für "${security.name}" löschen und komplett neu laden?\n\nDies kann nicht rückgängig gemacht werden.`)) {
      return;
    }

    setIsForceReloading(true);
    setForceReloadMessage(null);

    try {
      const apiKeys = {
        finnhub: finnhubApiKey || undefined,
        coingecko: coingeckoApiKey || undefined,
        alphaVantage: alphaVantageApiKey || undefined,
        twelveData: twelveDataApiKey || undefined,
      };

      const result = await forceReloadHistoricalPrices(security.id, 2010, apiKeys);

      if (result.error) {
        setForceReloadMessage({
          type: 'error',
          text: `${result.deletedCount} Kurse gelöscht, aber Fehler beim Laden: ${result.error}`,
        });
      } else {
        setForceReloadMessage({
          type: 'success',
          text: `${result.deletedCount} alte Kurse gelöscht, ${result.insertedCount} neue Kurse eingefügt (${result.dateFrom} - ${result.dateTo})`,
        });

        const { from, to } = getDateRange(selectedPeriod);
        const newPrices = await getPriceHistory(security.id, from, to);
        setPrices(newPrices);
      }
    } catch (err) {
      setForceReloadMessage({
        type: 'error',
        text: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setIsForceReloading(false);
    }
  };

  // Load logo when modal opens
  useEffect(() => {
    if (!isOpen || !security) {
      setLogoUrl(null);
      return;
    }

    if (security.customLogo) {
      setLogoUrl(security.customLogo);
      return;
    }

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
        let data = await getPriceHistory(security.id, from, to);

        if (data.length < 5 && security.feed) {
          try {
            const apiKeys = {
              finnhub: finnhubApiKey || undefined,
              coingecko: coingeckoApiKey || undefined,
            };
            await fetchHistoricalPrices(security.id, from, to, apiKeys);
            data = await getPriceHistory(security.id, from, to);
          } catch (fetchErr) {
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

  // Process chart data
  const chartData = useMemo(() => {
    if (!prices || prices.length === 0) return [];

    return prices
      .map((p) => ({
        time: p.date as Time,
        value: p.value,
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

    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
      seriesRef.current = null;
    }

    const isDark = document.documentElement.classList.contains('dark');

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: 'transparent' },
        textColor: isDark ? '#9ca3af' : '#6b7280',
        fontFamily: 'Geist Mono, ui-monospace, monospace',
        attributionLogo: false,
      },
      grid: {
        vertLines: { color: isDark ? 'rgba(255,255,255,0.03)' : 'rgba(0,0,0,0.04)' },
        horzLines: { color: isDark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.06)' },
      },
      width: chartContainerRef.current.clientWidth,
      height: 400,
      rightPriceScale: {
        borderVisible: false,
        entireTextOnly: true,
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
          color: isDark ? 'rgba(255,255,255,0.15)' : 'rgba(0,0,0,0.15)',
          style: 2,
          labelBackgroundColor: isDark ? '#1e293b' : '#f1f5f9',
        },
        horzLine: {
          width: 1,
          color: isDark ? 'rgba(255,255,255,0.15)' : 'rgba(0,0,0,0.15)',
          style: 2,
          labelBackgroundColor: isDark ? '#1e293b' : '#f1f5f9',
        },
      },
      handleScroll: false,
      handleScale: false,
    });

    chartRef.current = chart;

    const areaSeries = chart.addSeries(AreaSeries, {
      lineColor: stats.isPositive ? '#22c55e' : '#ef4444',
      lineWidth: 3,
      topColor: stats.isPositive ? 'rgba(34, 197, 94, 0.35)' : 'rgba(239, 68, 68, 0.35)',
      bottomColor: stats.isPositive ? 'rgba(34, 197, 94, 0.02)' : 'rgba(239, 68, 68, 0.02)',
      crosshairMarkerRadius: 5,
      crosshairMarkerBorderWidth: 2,
      crosshairMarkerBackgroundColor: stats.isPositive ? '#22c55e' : '#ef4444',
      crosshairMarkerBorderColor: isDark ? '#0d1117' : '#ffffff',
      priceFormat: {
        type: 'price',
        precision: 2,
        minMove: 0.01,
      },
    });

    seriesRef.current = areaSeries as ISeriesApi<'Area'>;
    areaSeries.setData(chartData as AreaData<Time>[]);
    chart.timeScale().fitContent();

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
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />

      {/* Modal */}
      <div className="relative card-elevated w-full max-w-5xl mx-6 max-h-[92vh] overflow-hidden animate-scale-in">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-border/50">
          <div className="flex items-center gap-4">
            {/* Logo */}
            {logoUrl ? (
              <img
                src={logoUrl}
                alt=""
                className="w-12 h-12 rounded-xl object-contain bg-muted/50 p-1"
                crossOrigin="anonymous"
              />
            ) : (
              <div className="w-12 h-12 rounded-xl bg-muted/50 flex items-center justify-center">
                <Building2 size={24} className="text-muted-foreground" />
              </div>
            )}
            <div>
              <h2 className="text-title">{security?.name || 'Kursverlauf'}</h2>
              <div className="flex items-center gap-3 text-sm text-muted-foreground mt-0.5">
                {security?.ticker && <span className="font-mono tabular-nums">{security.ticker}</span>}
                {security?.isin && <span className="font-mono tabular-nums text-muted-foreground/60">{security.isin}</span>}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {/* View mode tabs */}
            <div className="flex gap-1 bg-muted/50 p-1 rounded-lg">
              <button
                onClick={() => setViewMode('chart')}
                className={`flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md transition-colors ${
                  viewMode === 'chart'
                    ? 'bg-background shadow-elevation-1 font-medium'
                    : 'hover:bg-background/50 text-muted-foreground'
                }`}
              >
                <LineChart size={14} />
                Chart
              </button>
              <button
                onClick={() => setViewMode('table')}
                className={`flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md transition-colors ${
                  viewMode === 'table'
                    ? 'bg-background shadow-elevation-1 font-medium'
                    : 'hover:bg-background/50 text-muted-foreground'
                }`}
              >
                <Table2 size={14} />
                Kurse
              </button>
            </div>
            <button
              onClick={onClose}
              className="p-2 hover:bg-muted rounded-lg transition-colors text-muted-foreground hover:text-foreground"
            >
              <X size={20} />
            </button>
          </div>
        </div>

        {viewMode === 'chart' ? (
          <>
            {/* Price Hero */}
            <div className="px-6 pt-5 pb-4">
              {chartData.length > 0 ? (
                <div className="flex items-end justify-between">
                  <div>
                    <div className="flex items-baseline gap-2">
                      <span className="text-4xl font-light font-mono tabular-nums tracking-tight">
                        {stats.latestPrice.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                      </span>
                      <span className="text-base text-muted-foreground/70 font-medium">{currency}</span>
                    </div>
                    <div className="flex items-center gap-2 mt-1.5">
                      <div className={`flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${
                        stats.isPositive
                          ? 'bg-positive/10 text-positive'
                          : 'bg-negative/10 text-negative'
                      }`}>
                        {stats.isPositive ? <TrendingUp size={12} /> : <TrendingDown size={12} />}
                        <span className="font-mono tabular-nums">
                          {stats.isPositive ? '+' : ''}{stats.change.toFixed(2)} ({stats.isPositive ? '+' : ''}{stats.changePercent.toFixed(2)}%)
                        </span>
                      </div>
                      <span className="text-xs text-muted-foreground/50">
                        {TIME_PERIODS.find(p => p.value === selectedPeriod)?.label}
                      </span>
                    </div>
                  </div>

                  {/* Time Period Selector */}
                  <div className="flex gap-0.5 bg-muted/40 p-1 rounded-lg">
                    {TIME_PERIODS.map((period) => (
                      <button
                        key={period.value}
                        onClick={() => setSelectedPeriod(period.value)}
                        className={`px-2.5 py-1 text-xs font-medium rounded-md transition-all ${
                          selectedPeriod === period.value
                            ? 'bg-primary text-primary-foreground shadow-sm'
                            : 'text-muted-foreground hover:text-foreground hover:bg-muted/80'
                        }`}
                      >
                        {period.label}
                      </button>
                    ))}
                  </div>
                </div>
              ) : (
                <div className="text-muted-foreground text-sm">Keine Kursdaten</div>
              )}
            </div>

            {/* Chart Area */}
            <div className="px-3 pb-1">
              {isLoading ? (
                <div className="flex items-center justify-center h-[400px] text-muted-foreground">
                  <div className="flex flex-col items-center gap-2">
                    <RefreshCw size={20} className="animate-spin opacity-40" />
                    <span className="text-sm">Lade Kursdaten...</span>
                  </div>
                </div>
              ) : error ? (
                <div className="flex items-center justify-center h-[400px] text-destructive text-sm">
                  {error}
                </div>
              ) : chartData.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-[400px] text-muted-foreground">
                  <LineChart size={36} className="mb-3 opacity-20" />
                  <p className="text-sm">Keine Kursdaten für diesen Zeitraum</p>
                </div>
              ) : (
                <div ref={chartContainerRef} className="w-full" />
              )}
            </div>

            {/* Stats Row */}
            {chartData.length > 0 && (
              <div className="px-6 pb-4">
                <div className="grid grid-cols-4 gap-3">
                  <div className="rounded-lg bg-muted/30 px-3 py-2.5">
                    <p className="text-[10px] uppercase tracking-wider text-muted-foreground/60 mb-0.5">Eröffnung</p>
                    <p className="text-sm font-mono tabular-nums font-medium">{stats.firstPrice.toLocaleString('de-DE', { minimumFractionDigits: 2 })} <span className="text-muted-foreground/50">{currency}</span></p>
                  </div>
                  <div className="rounded-lg bg-positive/5 border border-positive/10 px-3 py-2.5">
                    <p className="text-[10px] uppercase tracking-wider text-positive/60 mb-0.5">Hoch</p>
                    <p className="text-sm font-mono tabular-nums font-medium text-positive">{stats.max.toLocaleString('de-DE', { minimumFractionDigits: 2 })} <span className="text-positive/50">{currency}</span></p>
                  </div>
                  <div className="rounded-lg bg-negative/5 border border-negative/10 px-3 py-2.5">
                    <p className="text-[10px] uppercase tracking-wider text-negative/60 mb-0.5">Tief</p>
                    <p className="text-sm font-mono tabular-nums font-medium text-negative">{stats.min.toLocaleString('de-DE', { minimumFractionDigits: 2 })} <span className="text-negative/50">{currency}</span></p>
                  </div>
                  <div className="rounded-lg bg-muted/30 px-3 py-2.5">
                    <p className="text-[10px] uppercase tracking-wider text-muted-foreground/60 mb-0.5">Aktuell</p>
                    <p className="text-sm font-mono tabular-nums font-medium">{stats.latestPrice.toLocaleString('de-DE', { minimumFractionDigits: 2 })} <span className="text-muted-foreground/50">{currency}</span></p>
                  </div>
                </div>
              </div>
            )}

            {/* AI Analysis Section */}
            {aiEnabled && chartData.length > 0 && (
              <div className="border-t border-border/30">
                {/* AI Header */}
                <div className="flex items-center justify-between px-6 py-2.5">
                  <button
                    onClick={() => setIsAiCollapsed(!isAiCollapsed)}
                    className="flex items-center gap-2 group"
                  >
                    <Sparkles size={14} className="text-primary/70 group-hover:text-primary transition-colors" />
                    <span className="text-sm text-muted-foreground group-hover:text-foreground transition-colors">KI-Analyse</span>
                    {isAiCollapsed ? <ChevronDown size={12} className="text-muted-foreground/40" /> : <ChevronUp size={12} className="text-muted-foreground/40" />}
                  </button>
                  <div className="flex items-center gap-2">
                    <AIModelSelector
                      featureId="chartAnalysis"
                      requiresVision={true}
                      value={{ provider: aiProvider, model: aiModel }}
                      onChange={setTempSelection}
                      compact
                      disabled={isAnalyzing}
                    />
                    <button
                      onClick={handleAnalyze}
                      disabled={isAnalyzing || !aiApiKey}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-primary/90 text-primary-foreground rounded-md hover:bg-primary disabled:opacity-40 disabled:cursor-not-allowed transition-all font-medium"
                    >
                      {isAnalyzing ? (
                        <>
                          <RefreshCw size={11} className="animate-spin" />
                          Analysiere...
                        </>
                      ) : (
                        <>
                          <Sparkles size={11} />
                          Analysieren
                        </>
                      )}
                    </button>
                  </div>
                </div>

                {/* AI Content */}
                {!isAiCollapsed && (
                  <div className="px-6 pb-4">
                    <div className="rounded-lg bg-muted/20 border border-border/30 p-4 max-h-56 overflow-y-auto">
                      {analysisError ? (
                        <div className="text-destructive text-sm">{analysisError}</div>
                      ) : isAnalyzing ? (
                        <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
                          <RefreshCw size={20} className="animate-spin mb-3 opacity-30" />
                          <p className="text-xs">Analyse wird erstellt...</p>
                        </div>
                      ) : analysis ? (
                        <div className="prose-ai">
                          <SafeMarkdown>{analysis}</SafeMarkdown>
                        </div>
                      ) : (
                        <div className="flex items-center justify-center py-6 text-muted-foreground/40 text-xs gap-2">
                          <Sparkles size={14} />
                          <span>Klicke "Analysieren" für eine KI-Einschätzung</span>
                        </div>
                      )}
                    </div>
                    {analysisInfo && (
                      <div className="flex items-center justify-between mt-1.5 px-1">
                        <span className="text-[10px] text-muted-foreground/40">Keine Anlageberatung</span>
                        <span className="text-[10px] text-muted-foreground/40 font-mono">
                          {analysisInfo.model}{analysisInfo.tokens && ` · ${analysisInfo.tokens.toLocaleString()} Tokens`}
                        </span>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </>
        ) : (
          /* Table View - Price History */
          <div className="flex flex-col max-h-[calc(92vh-80px)]">
            {/* Table Header */}
            <div className="flex items-center justify-between px-5 py-3 border-b border-border/50">
              <div className="flex items-center gap-3">
                <span className="text-sm text-muted-foreground">
                  {prices.length} Kursbuchungen
                </span>
                {security?.feed && (
                  <button
                    onClick={handleForceReload}
                    disabled={isForceReloading}
                    className="flex items-center gap-1.5 px-2 py-1 text-xs border border-amber-500/50 text-amber-600 rounded-lg hover:bg-amber-500/10 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    title="Alle Kurse löschen und komplett neu laden"
                  >
                    <RotateCcw size={12} className={isForceReloading ? 'animate-spin' : ''} />
                    {isForceReloading ? 'Lädt...' : 'Neu laden'}
                  </button>
                )}
              </div>
              <div className="flex gap-1">
                {TIME_PERIODS.map((period) => (
                  <button
                    key={period.value}
                    onClick={() => setSelectedPeriod(period.value)}
                    className={`px-2.5 py-1 text-xs font-medium rounded-lg transition-colors ${
                      selectedPeriod === period.value
                        ? 'bg-primary text-primary-foreground'
                        : 'text-muted-foreground hover:bg-muted'
                    }`}
                  >
                    {period.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Force Reload Message */}
            {forceReloadMessage && (
              <div className={`px-5 py-2 text-sm ${
                forceReloadMessage.type === 'success'
                  ? 'bg-positive-muted text-positive'
                  : 'bg-negative-muted text-negative'
              }`}>
                {forceReloadMessage.text}
              </div>
            )}

            {/* Scrollable Table */}
            <div className="flex-1 overflow-y-auto">
              {isLoading ? (
                <div className="flex items-center justify-center h-48 text-muted-foreground">
                  <div className="flex items-center gap-3">
                    <RefreshCw size={16} className="animate-spin" />
                    <span>Lade Kursdaten...</span>
                  </div>
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
                  <thead className="sticky top-0 bg-card border-b border-border/50">
                    <tr>
                      <th className="text-left py-2.5 px-5 text-label">Datum</th>
                      <th className="text-right py-2.5 px-5 text-label">Kurs</th>
                    </tr>
                  </thead>
                  <tbody>
                    {[...prices]
                      .sort((a, b) => b.date.localeCompare(a.date))
                      .map((price, index) => (
                        <tr
                          key={`${price.date}-${index}`}
                          className="border-b border-border/30 last:border-0 hover:bg-muted/30 transition-colors"
                        >
                          <td className="py-2.5 px-5 text-muted-foreground">
                            {formatDate(price.date)}
                          </td>
                          <td className="py-2.5 px-5 text-right font-mono tabular-nums">
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
