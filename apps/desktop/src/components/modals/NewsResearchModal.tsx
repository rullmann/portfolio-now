/**
 * NewsResearchModal - AI-powered news research for a security.
 *
 * Researches current news, analyst ratings, earnings, and events
 * using web search via Perplexity or OpenAI.
 */

import { useState, useRef, useCallback, useEffect } from 'react';
import { Newspaper, Loader2, AlertCircle, RefreshCw, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { useSettingsStore, type AiProvider } from '../../store';
import { useEscapeKey } from '../../lib/hooks/useEscapeKey';
import { AIModelSelector } from '../common/AIModelSelector';
import { SafeMarkdown } from '../common/SafeMarkdown';
import { RateLimiter } from '../../lib/imageOptimization';

interface NewsResearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  security: {
    id: number;
    name: string;
    ticker?: string | null;
    isin?: string | null;
    currency: string;
  };
  currentPrice?: number;
}

interface AiError {
  kind: 'rate_limit' | 'quota_exceeded' | 'invalid_api_key' | 'model_not_found' | 'server_error' | 'network_error' | 'other';
  message: string;
  provider: string;
  model: string;
  retryAfterSecs?: number;
  fallbackModel?: string;
}

interface NewsResearchResponse {
  analysis: string;
  provider: string;
  model: string;
  tokensUsed: number | null;
}

export function NewsResearchModal({ isOpen, onClose, security, currentPrice }: NewsResearchModalProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [result, setResult] = useState<NewsResearchResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [parsedError, setParsedError] = useState<AiError | null>(null);
  const [retryCountdown, setRetryCountdown] = useState(0);

  const { keys } = useSecureApiKeys();
  const { aiFeatureSettings, language } = useSettingsStore();

  // Temporary selection (not persisted unless "save as default")
  const [tempSelection, setTempSelection] = useState<{ provider: AiProvider; model: string } | null>(null);
  const newsConfig = aiFeatureSettings.newsResearch;
  const aiProvider = (tempSelection?.provider ?? newsConfig.provider) as AiProvider;
  const aiModel = tempSelection?.model ?? newsConfig.model;

  const rateLimiterRef = useRef<RateLimiter>(new RateLimiter(10000)); // 10s minimum
  const countdownIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Cleanup interval on unmount
  useEffect(() => {
    return () => {
      if (countdownIntervalRef.current) {
        clearInterval(countdownIntervalRef.current);
        countdownIntervalRef.current = null;
      }
    };
  }, []);

  useEscapeKey(isOpen, onClose);

  const getApiKey = useCallback(() => {
    switch (aiProvider) {
      case 'openai':
        return keys.openaiApiKey;
      case 'perplexity':
        return keys.perplexityApiKey;
      default:
        return '';
    }
  }, [aiProvider, keys]);

  const hasApiKey = () => {
    const key = getApiKey();
    return key && key.trim().length > 0;
  };

  const handleResearch = useCallback(async () => {
    if (!hasApiKey()) return;

    // Rate limiting
    if (!rateLimiterRef.current.canCall()) {
      const waitTime = Math.ceil(rateLimiterRef.current.timeUntilNextCall() / 1000);
      setError(`Bitte warte noch ${waitTime} Sekunden.`);
      return;
    }

    setIsLoading(true);
    setError(null);
    setParsedError(null);
    setResult(null);

    try {
      rateLimiterRef.current.markCalled();

      const response = await invoke<NewsResearchResponse>('research_security_news', {
        provider: aiProvider,
        model: aiModel,
        apiKey: getApiKey(),
        securityName: security.name,
        ticker: security.ticker || null,
        isin: security.isin || null,
        currency: security.currency,
        currentPrice: currentPrice ?? null,
        language: language,
      });

      setResult(response);
    } catch (err) {
      const errorStr = String(err);
      // Try to parse as AiError JSON
      try {
        const parsed = JSON.parse(errorStr) as AiError;
        setParsedError(parsed);
        setError(parsed.message);

        // Auto-countdown for rate limit errors
        if (parsed.kind === 'rate_limit' && parsed.retryAfterSecs) {
          // Clear any previous countdown interval
          if (countdownIntervalRef.current) {
            clearInterval(countdownIntervalRef.current);
          }
          let countdown = parsed.retryAfterSecs;
          setRetryCountdown(countdown);
          countdownIntervalRef.current = setInterval(() => {
            countdown--;
            setRetryCountdown(countdown);
            if (countdown <= 0) {
              clearInterval(countdownIntervalRef.current!);
              countdownIntervalRef.current = null;
            }
          }, 1000);
        }
      } catch {
        setError(errorStr);
      }
    } finally {
      setIsLoading(false);
    }
  }, [aiProvider, aiModel, security, currentPrice, language, getApiKey]);

  const handleClose = () => {
    if (countdownIntervalRef.current) {
      clearInterval(countdownIntervalRef.current);
      countdownIntervalRef.current = null;
    }
    setResult(null);
    setError(null);
    setParsedError(null);
    setTempSelection(null);
    setRetryCountdown(0);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={handleClose} />

      {/* Modal */}
      <div className="relative w-full max-w-2xl bg-card border border-border rounded-lg shadow-xl">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2">
            <Newspaper size={18} className="text-primary" />
            <h2 className="font-semibold text-sm">Nachrichten: {security.name}</h2>
          </div>
          <div className="flex items-center gap-2">
            <AIModelSelector
              featureId="newsResearch"
              requiresWebSearch
              value={{ provider: aiProvider, model: aiModel }}
              onChange={(val) => setTempSelection(val)}
              compact
              disabled={isLoading}
            />
            <button
              onClick={handleClose}
              className="p-1 hover:bg-muted rounded-md transition-colors"
            >
              <X size={18} className="text-muted-foreground" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-4 overflow-y-auto max-h-[60vh]">
          {/* No API Key */}
          {!hasApiKey() && (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <AlertCircle size={32} className="text-muted-foreground/50 mb-3" />
              <p className="text-sm text-muted-foreground mb-1">
                Kein API-Key hinterlegt.
              </p>
              <p className="text-xs text-muted-foreground">
                Konfiguriere einen Perplexity oder OpenAI API-Key in den Einstellungen.
              </p>
            </div>
          )}

          {/* Initial State (no result yet) */}
          {hasApiKey() && !isLoading && !result && !error && (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <Newspaper size={32} className="text-muted-foreground/30 mb-3" />
              <p className="text-sm text-muted-foreground">
                Recherchiere aktuelle Nachrichten, Analysten-Ratings und Termine.
              </p>
              <button
                onClick={handleResearch}
                className="mt-4 flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
              >
                <Newspaper size={16} />
                Recherchieren
              </button>
            </div>
          )}

          {/* Loading */}
          {isLoading && (
            <div className="flex flex-col items-center justify-center py-8">
              <Loader2 size={24} className="animate-spin text-primary mb-3" />
              <p className="text-sm text-muted-foreground">Recherchiere aktuelle Nachrichten...</p>
            </div>
          )}

          {/* Error */}
          {error && !isLoading && (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <AlertCircle size={24} className="text-destructive mb-3" />
              <p className="text-sm text-destructive mb-3">{error}</p>
              {parsedError?.kind === 'rate_limit' && retryCountdown > 0 ? (
                <p className="text-xs text-muted-foreground">
                  Erneut versuchen in {retryCountdown}s...
                </p>
              ) : (
                <button
                  onClick={handleResearch}
                  className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
                >
                  <RefreshCw size={14} />
                  Erneut versuchen
                </button>
              )}
            </div>
          )}

          {/* Result */}
          {result && !isLoading && (
            <div className="prose prose-sm dark:prose-invert max-w-none">
              <SafeMarkdown>{result.analysis}</SafeMarkdown>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t border-border">
          <span className="text-xs text-muted-foreground">
            Keine Anlageberatung.
            {result && (
              <>
                {' '} | {result.provider} / {result.model}
                {result.tokensUsed && ` | ${result.tokensUsed.toLocaleString()} Tokens`}
              </>
            )}
          </span>
          <div className="flex items-center gap-2">
            {result && (
              <button
                onClick={handleResearch}
                disabled={isLoading}
                className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <RefreshCw size={14} />
                Erneut recherchieren
              </button>
            )}
            <button
              onClick={handleClose}
              className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
            >
              Schließen
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
