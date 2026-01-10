/**
 * AI Analysis Panel for technical chart analysis.
 * Captures the chart as an image and sends it to an AI provider for analysis.
 */

import { useState, useMemo, useRef, useCallback } from 'react';
import { Sparkles, RefreshCw, AlertCircle, Settings, ChevronDown, ChevronUp, ArrowRight, Clock } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import ReactMarkdown from 'react-markdown';
import { useSettingsStore, useUIStore, AI_MODELS, toast } from '../../store';
import type { IndicatorConfig } from '../../lib/indicators';
import { AIProviderLogo, AI_PROVIDER_NAMES } from '../common/AIProviderLogo';
import { captureAndOptimizeChart, RateLimiter } from '../../lib/imageOptimization';

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
}

export function AIAnalysisPanel({
  chartRef,
  security,
  currentPrice,
  timeRange,
  indicators,
}: AIAnalysisPanelProps) {
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<string | null>(null);
  const [analysisInfo, setAnalysisInfo] = useState<{ provider: string; model: string; tokens?: number } | null>(null);
  const [error, setError] = useState<AiError | null>(null);
  const [retryCountdown, setRetryCountdown] = useState<number | null>(null);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const retryTimerRef = useRef<number | null>(null);
  const rateLimiterRef = useRef<RateLimiter>(new RateLimiter(5000)); // 5 second minimum between calls

  const { aiProvider, aiModel, setAiModel, anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey } = useSettingsStore();

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
      setAnalysisInfo({
        provider: result.provider,
        model: result.model,
        tokens: result.tokensUsed,
      });
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
              <div className="prose prose-sm dark:prose-invert max-w-none prose-headings:text-base prose-headings:font-semibold prose-headings:mt-3 prose-headings:mb-1 prose-p:my-1 prose-ul:my-1 prose-li:my-0">
                <ReactMarkdown>{analysis}</ReactMarkdown>
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
