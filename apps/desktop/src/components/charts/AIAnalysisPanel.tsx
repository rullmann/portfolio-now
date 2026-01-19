/**
 * AI Analysis Panel for technical chart analysis.
 * Captures the chart as an image and sends it to an AI provider for analysis.
 * Supports both text-based analysis and structured chart annotations.
 */

import { useState, useMemo, useRef, useCallback, useEffect } from 'react';
import { Sparkles, RefreshCw, AlertCircle, Settings, ChevronDown, ChevronUp, ArrowRight, Clock, MapPin, ToggleLeft, ToggleRight, Trash2, X, Zap, Bell, Target } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import ReactMarkdown from 'react-markdown';
import { useSettingsStore, useUIStore, AI_MODELS, toast, getModelCapabilities } from '../../store';
import { usePortfolioAnalysisStore, type TrendDirection, type TrendStrength } from '../../store/portfolioAnalysis';
import type { IndicatorConfig, OHLCData } from '../../lib/indicators';
import { calculateRSI, calculateMACD, calculateSMA, calculateEMA, calculateBollinger, calculateATR } from '../../lib/indicators';
import type {
  AnnotationAnalysisResponse,
  EnhancedAnnotationAnalysisResponse,
  ChartAnnotationWithId,
  TrendInfo,
  AnnotationType,
  SignalDirection,
  IndicatorValue,
  CandleData,
  VolumeAnalysis,
  EnhancedChartContext,
  AlertSuggestion,
  RiskRewardAnalysis,
} from '../../lib/types';
import { AIProviderLogo, AI_PROVIDER_NAMES } from '../common/AIProviderLogo';
import { captureAndOptimizeChart, RateLimiter } from '../../lib/imageOptimization';
import { saveAnnotations, getAnnotations } from '../../lib/api';

// Local SecurityData type (simplified version used in ChartsView)
interface SecurityData {
  id: number;
  name: string;
  isin: string | null;
  ticker: string | null;
  currency: string;
}

interface ChartAnalysisResponse {
  analysis: string;
  provider: string;
  model: string;
  tokensUsed?: number;
}

// Structured AI error from backend
interface AiError {
  kind: 'rate_limit' | 'quota_exceeded' | 'invalid_api_key' | 'model_not_found' | 'server_error' | 'network_error' | 'other';
  message: string;
  provider: string;
  model: string;
  retryAfterSecs?: number;
  fallbackModel?: string;
}

interface AIAnalysisPanelProps {
  chartRef: React.RefObject<HTMLDivElement>;
  security: SecurityData | null;
  currentPrice: number;
  timeRange: string;
  indicators: IndicatorConfig[];
  /** OHLC data for enhanced analysis with indicator values */
  ohlcData?: OHLCData[];
  /** Callback when annotations are generated */
  onAnnotationsChange?: (annotations: ChartAnnotationWithId[]) => void;
}

export function AIAnalysisPanel({
  chartRef,
  security,
  currentPrice,
  timeRange,
  indicators,
  ohlcData,
  onAnnotationsChange,
}: AIAnalysisPanelProps) {
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<string | null>(null);
  const [analysisInfo, setAnalysisInfo] = useState<{ provider: string; model: string; tokens?: number } | null>(null);
  const [error, setError] = useState<AiError | null>(null);
  const [retryCountdown, setRetryCountdown] = useState<number | null>(null);
  const [isCollapsed, setIsCollapsed] = useState(false);
  // Annotations mode state
  const [useAnnotations, setUseAnnotations] = useState(true);
  const [useEnhancedAnalysis, setUseEnhancedAnalysis] = useState(true); // Enhanced mode with indicator values
  const [includeWebContext, setIncludeWebContext] = useState(false); // Web search for news (Perplexity only)
  const [annotations, setAnnotations] = useState<ChartAnnotationWithId[]>([]);
  const [trendInfo, setTrendInfo] = useState<TrendInfo | null>(null);
  // Enhanced analysis results
  const [alerts, setAlerts] = useState<AlertSuggestion[]>([]);
  const [riskReward, setRiskReward] = useState<RiskRewardAnalysis | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const retryTimerRef = useRef<number | null>(null);
  const rateLimiterRef = useRef<RateLimiter>(new RateLimiter(5000)); // 5 second minimum between calls

  const {
    aiEnabled,
    aiFeatureSettings,
    setAiFeatureSetting,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
  } = useSettingsStore();

  // Get feature-specific provider and model for Chart Analysis
  const chartAnalysisConfig = aiFeatureSettings.chartAnalysis;
  const aiProvider = chartAnalysisConfig.provider as 'claude' | 'openai' | 'gemini' | 'perplexity';
  const aiModel = chartAnalysisConfig.model;

  // Update the feature-specific model
  const setAiModel = (model: string) => {
    setAiFeatureSetting('chartAnalysis', { provider: aiProvider, model });
  };

  // Portfolio analysis store for trend indicators in Dashboard
  const { setAnalysis: setPortfolioAnalysis } = usePortfolioAnalysisStore();

  // Get API key for selected provider
  const apiKey = useMemo(() => {
    switch (aiProvider) {
      case 'claude':
        return anthropicApiKey;
      case 'openai':
        return openaiApiKey;
      case 'gemini':
        return geminiApiKey;
      case 'perplexity':
        return perplexityApiKey;
    }
  }, [aiProvider, anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey]);

  // Get current model name
  const modelName = useMemo(() => {
    const models = AI_MODELS[aiProvider];
    const model = models.find(m => m.id === aiModel);
    return model?.name || aiModel;
  }, [aiProvider, aiModel]);

  // Check if current model supports web search (for news integration)
  const supportsWebSearch = useMemo(() => {
    return getModelCapabilities(aiProvider, aiModel).webSearch;
  }, [aiProvider, aiModel]);

  // Parse error from backend (could be JSON or plain string)
  const parseError = useCallback((err: unknown): AiError | null => {
    const errStr = err instanceof Error ? err.message : String(err);

    // Try to parse as JSON (structured error)
    try {
      const parsed = JSON.parse(errStr);
      if (parsed.kind && parsed.message) {
        return parsed as AiError;
      }
    } catch {
      // Not JSON, use as plain message
    }

    // Return as generic error
    return {
      kind: 'other',
      message: errStr,
      provider: AI_PROVIDER_NAMES[aiProvider],
      model: aiModel,
    };
  }, [aiProvider, aiModel]);

  // Start countdown for auto-retry
  const startRetryCountdown = useCallback((seconds: number) => {
    setRetryCountdown(seconds);

    const tick = () => {
      setRetryCountdown(prev => {
        if (prev === null || prev <= 1) {
          return null;
        }
        retryTimerRef.current = window.setTimeout(tick, 1000);
        return prev - 1;
      });
    };

    retryTimerRef.current = window.setTimeout(tick, 1000);
  }, []);

  // Clear retry timer
  const clearRetryTimer = useCallback(() => {
    if (retryTimerRef.current) {
      clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
    setRetryCountdown(null);
  }, []);

  // Switch to fallback model
  const switchToFallback = useCallback((fallbackModel: string) => {
    setAiModel(fallbackModel);
    setError(null);
    toast.info(`Modell gewechselt zu: ${fallbackModel}`);
  }, [setAiModel]);

  // Clear all annotations AND analysis text
  const clearAnalysis = useCallback(async () => {
    // Clear analysis text and related state
    setAnalysis(null);
    setAnalysisInfo(null);
    setTrendInfo(null);
    setAlerts([]);
    setRiskReward(null);

    // Clear annotations
    setAnnotations([]);
    onAnnotationsChange?.([]);

    // Also clear from database
    if (security?.id) {
      try {
        await saveAnnotations(security.id, [], true);
      } catch (err) {
        console.warn('Failed to clear annotations from database:', err);
      }
    }

    toast.success('Analyse gelöscht');
  }, [security?.id, onAnnotationsChange]);

  // Remove a single annotation
  const removeAnnotation = useCallback(async (annotationId: string) => {
    if (!security?.id) return;

    const updatedAnnotations = annotations.filter(a => a.id !== annotationId);
    setAnnotations(updatedAnnotations);
    onAnnotationsChange?.(updatedAnnotations);

    // Update database
    try {
      await saveAnnotations(
        security.id,
        updatedAnnotations.map(a => ({
          annotationType: a.type,
          price: a.price,
          time: a.time,
          timeEnd: a.timeEnd,
          title: a.title,
          description: a.description,
          confidence: a.confidence,
          signal: a.signal,
          source: 'ai' as const,
        })),
        true
      );
    } catch (err) {
      console.warn('Failed to update annotations in database:', err);
    }
  }, [security?.id, annotations, onAnnotationsChange]);

  // Clear analysis when security changes
  useEffect(() => {
    setAnalysis(null);
    setAnalysisInfo(null);
    setTrendInfo(null);
    setAlerts([]);
    setRiskReward(null);
    setError(null);
    setAnnotations([]);
    onAnnotationsChange?.([]);
  }, [security?.id]);

  // Load persisted annotations when security changes
  useEffect(() => {
    const loadPersistedAnnotations = async () => {
      if (!security?.id) return;

      try {
        const persisted = await getAnnotations(security.id, true);
        if (persisted.length > 0) {
          // Convert persisted annotations to ChartAnnotationWithId format
          const withIds: ChartAnnotationWithId[] = persisted.map(a => ({
            id: `persisted-${a.id}`,
            type: a.annotationType as AnnotationType,
            price: a.price,
            time: a.time,
            timeEnd: a.timeEnd,
            title: a.title,
            description: a.description || '',
            confidence: a.confidence,
            signal: a.signal as SignalDirection | undefined,
          }));
          setAnnotations(withIds);
          onAnnotationsChange?.(withIds);
        }
      } catch (err) {
        console.warn('Failed to load persisted annotations:', err);
      }
    };

    loadPersistedAnnotations();
  }, [security?.id, onAnnotationsChange]);

  // Build enhanced context with indicator values, OHLC data, and volume analysis
  const buildEnhancedContext = useCallback((): EnhancedChartContext | null => {
    if (!security || !ohlcData || ohlcData.length === 0) return null;

    const indicatorValues: IndicatorValue[] = [];

    // Calculate values for each enabled indicator
    for (const indicator of indicators.filter(i => i.enabled)) {
      switch (indicator.type) {
        case 'rsi': {
          const rsiData = calculateRSI(ohlcData, indicator.params.period || 14);
          const lastRsi = rsiData[rsiData.length - 1]?.value;
          const prevRsi = rsiData[rsiData.length - 2]?.value;
          if (lastRsi !== null && lastRsi !== undefined) {
            let signal: string | undefined;
            if (lastRsi > 70) signal = 'overbought';
            else if (lastRsi < 30) signal = 'oversold';
            else if (lastRsi > 50) signal = 'bullish';
            else signal = 'bearish';
            indicatorValues.push({
              name: 'RSI',
              params: `${indicator.params.period || 14}`,
              currentValue: lastRsi,
              previousValue: prevRsi ?? undefined,
              signal,
            });
          }
          break;
        }
        case 'macd': {
          const macdResult = calculateMACD(
            ohlcData,
            indicator.params.fast || 12,
            indicator.params.slow || 26,
            indicator.params.signal || 9
          );
          const lastMacd = macdResult.macd[macdResult.macd.length - 1]?.value;
          const lastHist = macdResult.histogram[macdResult.histogram.length - 1]?.value;
          const prevHist = macdResult.histogram[macdResult.histogram.length - 2]?.value;
          if (lastMacd !== null && lastMacd !== undefined) {
            let signal: string | undefined;
            if (lastHist !== null && prevHist !== null && lastHist !== undefined && prevHist !== undefined) {
              if (prevHist < 0 && lastHist > 0) signal = 'bullish_crossover';
              else if (prevHist > 0 && lastHist < 0) signal = 'bearish_crossover';
              else if (lastHist > 0) signal = 'bullish';
              else signal = 'bearish';
            }
            indicatorValues.push({
              name: 'MACD',
              params: `${indicator.params.fast || 12},${indicator.params.slow || 26},${indicator.params.signal || 9}`,
              currentValue: lastMacd,
              signal,
            });
          }
          break;
        }
        case 'sma': {
          const smaData = calculateSMA(ohlcData, indicator.params.period || 20);
          const lastSma = smaData[smaData.length - 1]?.value;
          if (lastSma !== null && lastSma !== undefined) {
            const lastClose = ohlcData[ohlcData.length - 1].close;
            const signal = lastClose > lastSma ? 'price_above' : 'price_below';
            indicatorValues.push({
              name: 'SMA',
              params: `${indicator.params.period || 20}`,
              currentValue: lastSma,
              signal,
            });
          }
          break;
        }
        case 'ema': {
          const emaData = calculateEMA(ohlcData, indicator.params.period || 20);
          const lastEma = emaData[emaData.length - 1]?.value;
          if (lastEma !== null && lastEma !== undefined) {
            const lastClose = ohlcData[ohlcData.length - 1].close;
            const signal = lastClose > lastEma ? 'price_above' : 'price_below';
            indicatorValues.push({
              name: 'EMA',
              params: `${indicator.params.period || 20}`,
              currentValue: lastEma,
              signal,
            });
          }
          break;
        }
        case 'bollinger': {
          const bbData = calculateBollinger(ohlcData, indicator.params.period || 20, indicator.params.stdDev || 2);
          const lastUpper = bbData.upper[bbData.upper.length - 1]?.value;
          const lastMiddle = bbData.middle[bbData.middle.length - 1]?.value;
          const lastLower = bbData.lower[bbData.lower.length - 1]?.value;
          if (lastMiddle !== null && lastMiddle !== undefined) {
            const lastClose = ohlcData[ohlcData.length - 1].close;
            let signal: string | undefined;
            if (lastUpper && lastClose > lastUpper) signal = 'above_upper';
            else if (lastLower && lastClose < lastLower) signal = 'below_lower';
            else signal = 'within_bands';
            indicatorValues.push({
              name: 'BOLLINGER',
              params: `${indicator.params.period || 20},${indicator.params.stdDev || 2}`,
              currentValue: lastMiddle,
              signal,
            });
          }
          break;
        }
        case 'atr': {
          const atrData = calculateATR(ohlcData, indicator.params.period || 14);
          const lastAtr = atrData[atrData.length - 1]?.value;
          if (lastAtr !== null && lastAtr !== undefined) {
            indicatorValues.push({
              name: 'ATR',
              params: `${indicator.params.period || 14}`,
              currentValue: lastAtr,
            });
          }
          break;
        }
      }
    }

    // Calculate volume analysis
    let volumeAnalysis: VolumeAnalysis | undefined;
    const volumeData = ohlcData.filter(d => d.volume !== undefined && d.volume > 0);
    if (volumeData.length >= 20) {
      const currentVolume = volumeData[volumeData.length - 1].volume!;
      const last20Volumes = volumeData.slice(-20).map(d => d.volume!);
      const avgVolume = last20Volumes.reduce((a, b) => a + b, 0) / last20Volumes.length;
      const volumeRatio = currentVolume / avgVolume;

      // Determine volume trend (compare last 5 days to previous 5 days)
      const recent5 = volumeData.slice(-5).map(d => d.volume!);
      const prev5 = volumeData.slice(-10, -5).map(d => d.volume!);
      const recentAvg = recent5.reduce((a, b) => a + b, 0) / recent5.length;
      const prevAvg = prev5.reduce((a, b) => a + b, 0) / prev5.length;
      const volumeTrend = recentAvg > prevAvg * 1.1 ? 'increasing' : recentAvg < prevAvg * 0.9 ? 'decreasing' : 'stable';

      volumeAnalysis = {
        currentVolume,
        avgVolume20d: avgVolume,
        volumeRatio,
        volumeTrend,
      };
    }

    // Get last 50 candles for context
    const candles: CandleData[] = ohlcData.slice(-50).map(d => ({
      date: d.time,
      open: d.open,
      high: d.high,
      low: d.low,
      close: d.close,
      volume: d.volume,
    }));

    // Calculate price statistics
    const lastClose = ohlcData[ohlcData.length - 1].close;
    const firstClose = ohlcData[0].close;
    const priceChangePercent = ((lastClose - firstClose) / firstClose) * 100;

    // Find 52-week high/low (if enough data)
    let high52Week: number | undefined;
    let low52Week: number | undefined;
    let distanceFromHighPercent: number | undefined;
    if (ohlcData.length >= 52) {
      const yearData = ohlcData.slice(-252); // Approximately 1 year of trading days
      high52Week = Math.max(...yearData.map(d => d.high));
      low52Week = Math.min(...yearData.map(d => d.low));
      distanceFromHighPercent = ((high52Week - lastClose) / high52Week) * 100;
    }

    return {
      securityName: security.name,
      ticker: security.ticker || undefined,
      currency: security.currency,
      currentPrice,
      timeframe: timeRange,
      indicatorValues,
      candles,
      volumeAnalysis,
      priceChangePercent,
      high52Week,
      low52Week,
      distanceFromHighPercent,
      // Only enable web context if model supports it and user enabled it
      includeWebContext: supportsWebSearch && includeWebContext,
    };
  }, [security, ohlcData, indicators, currentPrice, timeRange, supportsWebSearch, includeWebContext]);

  const handleAnalyze = async (overrideModel?: string) => {
    if (!chartRef.current || !security || !apiKey) return;

    // Rate limiting check
    if (!rateLimiterRef.current.canCall()) {
      const waitTime = Math.ceil(rateLimiterRef.current.timeUntilNextCall() / 1000);
      toast.warning(`Bitte warte noch ${waitTime} Sekunden vor der nächsten Analyse`);
      return;
    }

    clearRetryTimer();
    setIsAnalyzing(true);
    setError(null);
    setIsCollapsed(false);

    try {
      // Capture and optimize chart image
      const { base64: imageBase64, savings } = captureAndOptimizeChart(chartRef.current);
      console.log(`Chart image optimized: ${savings}`);

      // Mark call time for rate limiting
      rateLimiterRef.current.markCalled();

      const context = {
        securityName: security.name,
        ticker: security.ticker,
        currency: security.currency,
        currentPrice,
        timeframe: timeRange,
        indicators: indicators
          .filter((i) => i.enabled)
          .map((i) => {
            const params = Object.entries(i.params)
              .filter(([, v]) => v !== undefined)
              .map(([, v]) => v)
              .join(',');
            return `${i.type.toUpperCase()}(${params})`;
          }),
      };

      const modelToUse = overrideModel || aiModel;

      if (useAnnotations) {
        // Check if enhanced analysis is available (with OHLC data)
        const enhancedContext = useEnhancedAnalysis ? buildEnhancedContext() : null;

        if (enhancedContext && useEnhancedAnalysis) {
          // Call the enhanced analysis endpoint with indicator values
          const result = await invoke<EnhancedAnnotationAnalysisResponse>('analyze_chart_enhanced', {
            request: {
              imageBase64,
              provider: aiProvider,
              model: modelToUse,
              apiKey,
              context: enhancedContext,
            },
          });

          // Add unique IDs to annotations for React keys
          const annotationsWithIds: ChartAnnotationWithId[] = result.annotations.map((a, idx) => ({
            ...a,
            id: `${Date.now()}-${idx}`,
          }));

          setAnalysis(result.analysis);
          setAnnotations(annotationsWithIds);
          setTrendInfo(result.trend);
          setAlerts(result.alerts || []);
          setRiskReward(result.riskReward || null);
          setAnalysisInfo({
            provider: result.provider,
            model: result.model,
            tokens: result.tokensUsed,
          });

          // Notify parent about new annotations
          onAnnotationsChange?.(annotationsWithIds);

          // Persist annotations to database
          if (security.id) {
            try {
              await saveAnnotations(
                security.id,
                result.annotations.map(a => ({
                  annotationType: a.type,
                  price: a.price,
                  time: a.time,
                  timeEnd: a.timeEnd,
                  title: a.title,
                  description: a.description,
                  confidence: a.confidence,
                  signal: a.signal,
                  provider: result.provider,
                  model: result.model,
                })),
                true // Clear existing AI annotations
              );
            } catch (persistErr) {
              console.warn('Failed to persist annotations:', persistErr);
            }

            // Update portfolio analysis store for Dashboard trend indicators
            if (result.trend) {
              const firstSentence = result.analysis.split(/[.!?]/)[0]?.trim() || result.analysis.slice(0, 100);
              setPortfolioAnalysis(security.id, {
                securityId: security.id,
                name: security.name,
                trend: result.trend.direction as TrendDirection,
                strength: result.trend.strength as TrendStrength,
                confidence: result.trend.confidence,
                summary: firstSentence,
              });
            }
          }
        } else {
          // Call the standard structured annotations endpoint
          const result = await invoke<AnnotationAnalysisResponse>('analyze_chart_with_annotations', {
            request: {
              imageBase64,
              provider: aiProvider,
              model: modelToUse,
              apiKey,
              context,
            },
          });

          // Add unique IDs to annotations for React keys
          const annotationsWithIds: ChartAnnotationWithId[] = result.annotations.map((a, idx) => ({
            ...a,
            id: `${Date.now()}-${idx}`,
          }));

          setAnalysis(result.analysis);
          setAnnotations(annotationsWithIds);
          setTrendInfo(result.trend);
          setAlerts([]); // Clear alerts when using standard endpoint
          setRiskReward(null);
          setAnalysisInfo({
            provider: result.provider,
            model: result.model,
            tokens: result.tokensUsed,
          });

          // Notify parent about new annotations
          onAnnotationsChange?.(annotationsWithIds);

          // Persist annotations to database
          if (security.id) {
            try {
              await saveAnnotations(
                security.id,
                result.annotations.map(a => ({
                  annotationType: a.type,
                  price: a.price,
                  time: a.time,
                  timeEnd: a.timeEnd,
                  title: a.title,
                  description: a.description,
                  confidence: a.confidence,
                  signal: a.signal,
                  provider: result.provider,
                  model: result.model,
                })),
                true // Clear existing AI annotations
              );
            } catch (persistErr) {
              console.warn('Failed to persist annotations:', persistErr);
            }

            // Update portfolio analysis store for Dashboard trend indicators
            if (result.trend) {
              const firstSentence = result.analysis.split(/[.!?]/)[0]?.trim() || result.analysis.slice(0, 100);
              setPortfolioAnalysis(security.id, {
                securityId: security.id,
                name: security.name,
                trend: result.trend.direction as TrendDirection,
                strength: result.trend.strength as TrendStrength,
                confidence: result.trend.confidence,
                summary: firstSentence,
              });
            }
          }
        }
      } else {
        // Call the original text-only endpoint
        const result = await invoke<ChartAnalysisResponse>('analyze_chart_with_ai', {
          request: {
            imageBase64,
            provider: aiProvider,
            model: modelToUse,
            apiKey,
            context,
          },
        });

        setAnalysis(result.analysis);
        setAnnotations([]);
        setTrendInfo(null);
        setAnalysisInfo({
          provider: result.provider,
          model: result.model,
          tokens: result.tokensUsed,
        });

        // Clear annotations when using text mode
        onAnnotationsChange?.([]);
      }
    } catch (err) {
      const parsedError = parseError(err);
      setError(parsedError);

      // Auto-start countdown for rate limit errors
      if (parsedError?.kind === 'rate_limit' && parsedError.retryAfterSecs) {
        startRetryCountdown(parsedError.retryAfterSecs);
      }
    } finally {
      setIsAnalyzing(false);
    }
  };

  const { setCurrentView, setScrollTarget } = useUIStore();

  const navigateToAiSettings = () => {
    setScrollTarget('ai-analysis');
    setCurrentView('settings');
  };

  // Render error with appropriate actions
  const renderError = () => {
    if (!error) return null;

    const getErrorIcon = () => {
      switch (error.kind) {
        case 'rate_limit':
          return <Clock size={16} className="shrink-0 mt-0.5" />;
        case 'quota_exceeded':
          return <AlertCircle size={16} className="shrink-0 mt-0.5" />;
        case 'invalid_api_key':
          return <Settings size={16} className="shrink-0 mt-0.5" />;
        default:
          return <AlertCircle size={16} className="shrink-0 mt-0.5" />;
      }
    };

    const getErrorColor = () => {
      switch (error.kind) {
        case 'rate_limit':
          return 'text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20';
        case 'quota_exceeded':
          return 'text-orange-600 dark:text-orange-400 bg-orange-50 dark:bg-orange-900/20';
        default:
          return 'text-destructive bg-destructive/10';
      }
    };

    return (
      <div className={`flex flex-col gap-3 text-sm p-3 rounded-md ${getErrorColor()}`}>
        <div className="flex items-start gap-2">
          {getErrorIcon()}
          <div className="flex-1">
            <p className="font-medium">
              {error.kind === 'rate_limit' && 'Zu viele Anfragen'}
              {error.kind === 'quota_exceeded' && 'Kontingent erschöpft'}
              {error.kind === 'invalid_api_key' && 'Ungültiger API Key'}
              {error.kind === 'model_not_found' && 'Modell nicht verfügbar'}
              {error.kind === 'server_error' && 'Server-Fehler'}
              {error.kind === 'network_error' && 'Netzwerkfehler'}
              {error.kind === 'other' && 'Fehler bei der Analyse'}
            </p>
            <p className="text-xs mt-1 opacity-80">{error.message}</p>
          </div>
        </div>

        {/* Action buttons based on error type */}
        <div className="flex flex-wrap gap-2">
          {/* Rate limit: show countdown or retry button */}
          {error.kind === 'rate_limit' && (
            retryCountdown !== null ? (
              <span className="text-xs opacity-70">
                Erneuter Versuch in {retryCountdown}s...
              </span>
            ) : (
              <button
                onClick={() => handleAnalyze()}
                className="flex items-center gap-1.5 px-2 py-1 text-xs bg-background border border-current/20 rounded hover:bg-muted transition-colors"
              >
                <RefreshCw size={12} />
                Erneut versuchen
              </button>
            )
          )}

          {/* Quota exceeded: show fallback option */}
          {error.kind === 'quota_exceeded' && error.fallbackModel && (
            <button
              onClick={() => switchToFallback(error.fallbackModel!)}
              className="flex items-center gap-1.5 px-2 py-1 text-xs bg-background border border-current/20 rounded hover:bg-muted transition-colors"
            >
              <ArrowRight size={12} />
              Zu {error.fallbackModel} wechseln
            </button>
          )}

          {/* Model not found: show fallback option */}
          {error.kind === 'model_not_found' && error.fallbackModel && (
            <button
              onClick={() => switchToFallback(error.fallbackModel!)}
              className="flex items-center gap-1.5 px-2 py-1 text-xs bg-background border border-current/20 rounded hover:bg-muted transition-colors"
            >
              <ArrowRight size={12} />
              {error.fallbackModel} verwenden
            </button>
          )}

          {/* Invalid API key: link to settings */}
          {error.kind === 'invalid_api_key' && (
            <button
              onClick={navigateToAiSettings}
              className="flex items-center gap-1.5 px-2 py-1 text-xs bg-background border border-current/20 rounded hover:bg-muted transition-colors"
            >
              <Settings size={12} />
              Einstellungen öffnen
            </button>
          )}

          {/* Server/network error: retry button */}
          {(error.kind === 'server_error' || error.kind === 'network_error') && (
            <button
              onClick={() => handleAnalyze()}
              className="flex items-center gap-1.5 px-2 py-1 text-xs bg-background border border-current/20 rounded hover:bg-muted transition-colors"
            >
              <RefreshCw size={12} />
              Erneut versuchen
            </button>
          )}
        </div>
      </div>
    );
  };

  // AI is globally disabled
  if (!aiEnabled) {
    return null;
  }

  // No API key configured
  if (!apiKey) {
    return (
      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-amber-100 dark:bg-amber-900/30 rounded-lg">
            <AlertCircle size={20} className="text-amber-600 dark:text-amber-400" />
          </div>
          <div className="flex-1">
            <p className="text-sm font-medium">KI-Analyse nicht konfiguriert</p>
            <p className="text-xs text-muted-foreground mt-0.5">
              Bitte hinterlege einen API Key in den Einstellungen.
            </p>
          </div>
          <button
            onClick={navigateToAiSettings}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            <Settings size={14} />
            Einstellungen
          </button>
        </div>
      </div>
    );
  }

  return (
    <div ref={panelRef} className="bg-card border border-border rounded-lg overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-border bg-muted/30">
        <button
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="flex items-center gap-2 hover:text-primary transition-colors"
        >
          <Sparkles size={16} className="text-primary" />
          <span className="font-medium">KI-Analyse</span>
          <div className="flex items-center gap-1.5 px-2 py-0.5 bg-muted rounded border border-border/50">
            <AIProviderLogo provider={aiProvider} size={14} />
            <span className="text-xs font-medium">{AI_PROVIDER_NAMES[aiProvider]}</span>
            <span className="text-xs text-muted-foreground">|</span>
            <span className="text-xs text-muted-foreground">{modelName}</span>
          </div>
          {isCollapsed ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
        </button>
        <div className="flex items-center gap-2">
          {/* Clear analysis button - visible when there is analysis or annotations */}
          {(analysis || annotations.length > 0) && (
            <button
              onClick={clearAnalysis}
              className="flex items-center gap-1.5 px-2 py-1 text-xs rounded border border-destructive/30 text-destructive hover:bg-destructive/10 transition-colors"
              title="Analyse und Marker löschen"
            >
              <Trash2 size={12} />
              <span>Analyse löschen</span>
            </button>
          )}
          {/* Enhanced analysis toggle */}
          {ohlcData && ohlcData.length > 0 && useAnnotations && (
            <button
              onClick={() => setUseEnhancedAnalysis(!useEnhancedAnalysis)}
              className="flex items-center gap-1.5 px-2 py-1 text-xs rounded border border-border hover:bg-muted transition-colors"
              title={useEnhancedAnalysis ? 'Erweiterte Analyse mit Indikator-Werten' : 'Standard-Analyse'}
            >
              {useEnhancedAnalysis ? (
                <>
                  <ToggleRight size={14} className="text-primary" />
                  <Zap size={12} className="text-primary" />
                </>
              ) : (
                <>
                  <ToggleLeft size={14} className="text-muted-foreground" />
                  <Zap size={12} className="text-muted-foreground" />
                </>
              )}
            </button>
          )}
          {/* Web search / News toggle (only for Perplexity and similar models) */}
          {supportsWebSearch && useEnhancedAnalysis && useAnnotations && (
            <button
              onClick={() => setIncludeWebContext(!includeWebContext)}
              className="flex items-center gap-1.5 px-2 py-1 text-xs rounded border border-border hover:bg-muted transition-colors"
              title={includeWebContext ? 'News-Recherche aktiv (Web-Suche)' : 'News-Recherche deaktiviert'}
            >
              {includeWebContext ? (
                <span className="text-primary text-[10px] font-medium">📰 News</span>
              ) : (
                <span className="text-muted-foreground text-[10px]">📰 News</span>
              )}
            </button>
          )}
          {/* Annotations toggle */}
          <button
            onClick={() => setUseAnnotations(!useAnnotations)}
            className="flex items-center gap-1.5 px-2 py-1 text-xs rounded border border-border hover:bg-muted transition-colors"
            title={useAnnotations ? 'Chart-Marker aktiv' : 'Nur Text-Analyse'}
          >
            {useAnnotations ? (
              <>
                <ToggleRight size={14} className="text-primary" />
                <MapPin size={12} className="text-primary" />
              </>
            ) : (
              <>
                <ToggleLeft size={14} className="text-muted-foreground" />
                <MapPin size={12} className="text-muted-foreground" />
              </>
            )}
          </button>
          <button
            onClick={() => handleAnalyze()}
            disabled={isAnalyzing || !security}
            className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {isAnalyzing ? (
              <>
                <RefreshCw size={14} className="animate-spin" />
                Analysiere...
              </>
            ) : (
              <>
                <Sparkles size={14} />
                Chart analysieren
              </>
            )}
          </button>
        </div>
      </div>

      {/* Content */}
      {!isCollapsed && (
        <>
          <div className="p-4 h-64 overflow-y-auto">
            {error && renderError()}
            {isAnalyzing ? (
              <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <RefreshCw size={32} className="animate-spin mb-3 opacity-50" />
                <p className="text-sm">Analyse wird erstellt...</p>
              </div>
            ) : analysis ? (
              <div className="space-y-3">
                {/* Trend indicator */}
                {trendInfo && (
                  <div className="flex items-center gap-2 px-3 py-2 rounded-md bg-muted/50">
                    <span className="text-xs font-medium">Trend:</span>
                    <span className={`text-xs font-semibold ${
                      trendInfo.direction === 'bullish' ? 'text-green-600 dark:text-green-400' :
                      trendInfo.direction === 'bearish' ? 'text-red-600 dark:text-red-400' :
                      'text-muted-foreground'
                    }`}>
                      {trendInfo.direction === 'bullish' ? '↑ Bullish' :
                       trendInfo.direction === 'bearish' ? '↓ Bearish' : '→ Neutral'}
                    </span>
                    <span className="text-xs text-muted-foreground">|</span>
                    <span className="text-xs text-muted-foreground">
                      {trendInfo.strength === 'strong' ? 'Stark' :
                       trendInfo.strength === 'moderate' ? 'Moderat' : 'Schwach'}
                    </span>
                    {annotations.length > 0 && (
                      <>
                        <span className="text-xs text-muted-foreground">|</span>
                        <span className="text-xs text-muted-foreground">
                          {annotations.length} Marker im Chart
                        </span>
                      </>
                    )}
                  </div>
                )}

                {/* Analysis text */}
                <div className="prose prose-xs dark:prose-invert max-w-none text-[13px] leading-relaxed prose-headings:text-sm prose-headings:font-semibold prose-headings:mt-2.5 prose-headings:mb-0.5 prose-p:my-0.5 prose-ul:my-0.5 prose-li:my-0 prose-strong:font-semibold">
                  <ReactMarkdown>{analysis}</ReactMarkdown>
                </div>

                {/* Annotations list */}
                {annotations.length > 0 && (
                  <div className="space-y-1.5 pt-2 border-t border-border">
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-medium text-muted-foreground">Chart-Marker:</span>
                    </div>
                    {annotations.map(annotation => (
                      <div
                        key={annotation.id}
                        className={`group flex items-start gap-2 p-2 rounded text-xs ${
                          annotation.signal === 'bullish' ? 'bg-green-500/10' :
                          annotation.signal === 'bearish' ? 'bg-red-500/10' :
                          'bg-muted/50'
                        }`}
                      >
                        <span className="shrink-0">
                          {annotation.type === 'support' ? '🟢' :
                           annotation.type === 'resistance' ? '🔴' :
                           annotation.type === 'pattern' ? '📐' :
                           annotation.type === 'signal' ? '⚡' :
                           annotation.type === 'target' ? '🎯' :
                           annotation.type === 'stoploss' ? '🛑' : '📝'}
                        </span>
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="font-medium">{annotation.title}</span>
                            <span className="text-muted-foreground">
                              @ {annotation.price.toFixed(2)}
                            </span>
                            <span className="ml-auto text-muted-foreground">
                              {Math.round(annotation.confidence * 100)}%
                            </span>
                            <button
                              onClick={() => removeAnnotation(annotation.id)}
                              className="opacity-0 group-hover:opacity-100 p-0.5 text-muted-foreground hover:text-destructive transition-opacity"
                              title="Marker löschen"
                            >
                              <X size={14} />
                            </button>
                          </div>
                          <p className="text-muted-foreground mt-0.5 line-clamp-2">
                            {annotation.description}
                          </p>
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {/* Alert suggestions */}
                {alerts.length > 0 && (
                  <div className="space-y-1.5 pt-2 border-t border-border">
                    <div className="flex items-center gap-2">
                      <Bell size={14} className="text-amber-500" />
                      <span className="text-xs font-medium text-muted-foreground">Alarm-Vorschläge:</span>
                    </div>
                    {alerts.map((alert, idx) => (
                      <div
                        key={`alert-${idx}`}
                        className={`flex items-start gap-2 p-2 rounded text-xs ${
                          alert.priority === 'high' ? 'bg-amber-500/10 border-l-2 border-amber-500' :
                          alert.priority === 'medium' ? 'bg-blue-500/10 border-l-2 border-blue-500' :
                          'bg-muted/50 border-l-2 border-muted-foreground/30'
                        }`}
                      >
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="font-medium">
                              {alert.condition === 'above' ? '↑' : alert.condition === 'below' ? '↓' : '↔'}
                              {' '}{security?.currency} {alert.price.toFixed(2)}
                            </span>
                            <span className={`text-[10px] px-1.5 py-0.5 rounded ${
                              alert.priority === 'high' ? 'bg-amber-500/20 text-amber-600 dark:text-amber-400' :
                              alert.priority === 'medium' ? 'bg-blue-500/20 text-blue-600 dark:text-blue-400' :
                              'bg-muted text-muted-foreground'
                            }`}>
                              {alert.priority === 'high' ? 'Hoch' : alert.priority === 'medium' ? 'Mittel' : 'Niedrig'}
                            </span>
                          </div>
                          <p className="text-muted-foreground mt-0.5">{alert.reason}</p>
                        </div>
                      </div>
                    ))}
                  </div>
                )}

                {/* Risk/Reward Analysis - only show if all required fields are present */}
                {riskReward && riskReward.entryPrice != null && riskReward.stopLoss != null && riskReward.takeProfit != null && riskReward.riskRewardRatio != null && (
                  <div className="space-y-2 pt-2 border-t border-border">
                    <div className="flex items-center gap-2">
                      <Target size={14} className="text-blue-500" />
                      <span className="text-xs font-medium text-muted-foreground">Risk/Reward Analyse:</span>
                    </div>
                    <div className="grid grid-cols-3 gap-2 text-xs">
                      <div className="p-2 rounded bg-blue-500/10 text-center">
                        <div className="text-muted-foreground text-[10px] mb-0.5">Entry</div>
                        <div className="font-semibold">{security?.currency} {riskReward.entryPrice.toFixed(2)}</div>
                      </div>
                      <div className="p-2 rounded bg-red-500/10 text-center">
                        <div className="text-muted-foreground text-[10px] mb-0.5">Stop-Loss</div>
                        <div className="font-semibold text-red-600 dark:text-red-400">{security?.currency} {riskReward.stopLoss.toFixed(2)}</div>
                        <div className="text-[10px] text-muted-foreground">
                          {((riskReward.entryPrice - riskReward.stopLoss) / riskReward.entryPrice * 100).toFixed(1)}%
                        </div>
                      </div>
                      <div className="p-2 rounded bg-green-500/10 text-center">
                        <div className="text-muted-foreground text-[10px] mb-0.5">Take-Profit</div>
                        <div className="font-semibold text-green-600 dark:text-green-400">{security?.currency} {riskReward.takeProfit.toFixed(2)}</div>
                        <div className="text-[10px] text-muted-foreground">
                          +{((riskReward.takeProfit - riskReward.entryPrice) / riskReward.entryPrice * 100).toFixed(1)}%
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center justify-between p-2 rounded bg-muted/50">
                      <span className="text-xs text-muted-foreground">Risk/Reward Ratio:</span>
                      <span className={`text-sm font-semibold ${
                        riskReward.riskRewardRatio >= 2 ? 'text-green-600 dark:text-green-400' :
                        riskReward.riskRewardRatio >= 1 ? 'text-amber-600 dark:text-amber-400' :
                        'text-red-600 dark:text-red-400'
                      }`}>
                        1:{riskReward.riskRewardRatio.toFixed(1)}
                      </span>
                    </div>
                    {riskReward.rationale && <p className="text-xs text-muted-foreground">{riskReward.rationale}</p>}
                  </div>
                )}
              </div>
            ) : !error ? (
              <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <Sparkles size={32} className="mb-3 opacity-30" />
                <p className="text-sm">Klicke "Chart analysieren" um eine KI-gestützte</p>
                <p className="text-sm">technische Analyse zu erhalten.</p>
              </div>
            ) : null}
          </div>

          {/* Footer */}
          <div className="px-4 py-2 border-t border-border bg-muted/30 flex items-center justify-between">
            <span className="text-xs text-muted-foreground">
              Dies ist keine Anlageberatung. Analysen können fehlerhaft sein.
            </span>
            {analysisInfo && (
              <span className="text-xs text-muted-foreground">
                {analysisInfo.model}
                {analysisInfo.tokens && ` | ${analysisInfo.tokens.toLocaleString()} Tokens`}
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}
