/**
 * AI Analysis Panel for technical chart analysis.
 * Captures the chart as an image and sends it to an AI provider for analysis.
 * Supports both text-based analysis and structured chart annotations.
 */

import { useState, useMemo, useRef, useCallback, useEffect } from 'react';
import { Sparkles, RefreshCw, AlertCircle, Settings, ChevronDown, ChevronUp, ArrowRight, Clock, MapPin, ToggleLeft, ToggleRight, Trash2, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import ReactMarkdown from 'react-markdown';
import { useSettingsStore, useUIStore, AI_MODELS, toast } from '../../store';
import { usePortfolioAnalysisStore, type TrendDirection, type TrendStrength } from '../../store/portfolioAnalysis';
import type { IndicatorConfig } from '../../lib/indicators';
import type { AnnotationAnalysisResponse, ChartAnnotationWithId, TrendInfo, AnnotationType, SignalDirection } from '../../lib/types';
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
  /** Callback when annotations are generated */
  onAnnotationsChange?: (annotations: ChartAnnotationWithId[]) => void;
}

export function AIAnalysisPanel({
  chartRef,
  security,
  currentPrice,
  timeRange,
  indicators,
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
  const [annotations, setAnnotations] = useState<ChartAnnotationWithId[]>([]);
  const [trendInfo, setTrendInfo] = useState<TrendInfo | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const retryTimerRef = useRef<number | null>(null);
  const rateLimiterRef = useRef<RateLimiter>(new RateLimiter(5000)); // 5 second minimum between calls

  const { aiProvider, aiModel, setAiModel, anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey } = useSettingsStore();

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

  // Clear all annotations
  const clearAllAnnotations = useCallback(async () => {
    if (!security?.id) return;

    setAnnotations([]);
    onAnnotationsChange?.([]);

    // Also clear from database
    try {
      await saveAnnotations(security.id, [], true);
      toast.success('Alle Marker gelöscht');
    } catch (err) {
      console.warn('Failed to clear annotations from database:', err);
    }
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
        // Call the structured annotations endpoint
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
          {/* Clear annotations button - always visible when there are annotations */}
          {annotations.length > 0 && (
            <button
              onClick={clearAllAnnotations}
              className="flex items-center gap-1.5 px-2 py-1 text-xs rounded border border-destructive/30 text-destructive hover:bg-destructive/10 transition-colors"
              title="Alle Marker löschen"
            >
              <Trash2 size={12} />
              <span>Marker löschen</span>
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
                      <button
                        onClick={clearAllAnnotations}
                        className="flex items-center gap-1 px-1.5 py-0.5 text-xs text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded transition-colors"
                        title="Alle Marker löschen"
                      >
                        <Trash2 size={12} />
                        <span>Alle löschen</span>
                      </button>
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
