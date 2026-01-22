/**
 * Modal for auditing existing quote configurations.
 * Shows securities with missing, stale, config errors, or suspicious prices.
 * Now performs actual quote fetches to verify configuration works.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import { ClipboardCheck, Check, RefreshCw, AlertCircle, Clock, XCircle, AlertTriangle, Settings2, Wrench, Loader2, ChevronRight } from 'lucide-react';
import { useEscapeKey } from '../../lib/hooks';
import {
  auditQuoteConfigurations,
  getQuoteFixSuggestions,
  applyQuoteFix,
  type QuoteAuditSummary,
  type QuoteConfigAuditResult,
  type QuoteFixSuggestion,
  type ApiKeys
} from '../../lib/api';
import { useSettingsStore } from '../../store';

interface QuoteAuditModalProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete: () => void;
}

function StatusBadge({ status, days }: { status: string; days?: number }) {
  if (status === 'unconfigured') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-blue-100 text-blue-700">
        <Settings2 size={12} />
        Unkonfiguriert
      </span>
    );
  }
  if (status === 'config_error') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-orange-100 text-orange-700">
        <Settings2 size={12} />
        Konfig-Fehler
      </span>
    );
  }
  if (status === 'suspicious') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-purple-100 text-purple-700">
        <AlertTriangle size={12} />
        Verdächtig
      </span>
    );
  }
  if (status === 'missing') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-red-100 text-red-700">
        <XCircle size={12} />
        Keine Kurse
      </span>
    );
  }
  if (status === 'stale') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-yellow-100 text-yellow-700">
        <Clock size={12} />
        {days} Tage alt
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-green-100 text-green-700">
      <Check size={12} />
      OK
    </span>
  );
}

function formatPrice(price: number): string {
  return price.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

interface SecurityRowProps {
  result: QuoteConfigAuditResult;
  apiKeys: ApiKeys;
  onFixed: (securityId: number, status: string) => void;
}

function SecurityRow({ result, apiKeys, onFixed }: SecurityRowProps) {
  const [suggestions, setSuggestions] = useState<QuoteFixSuggestion[]>([]);
  const [isLoadingSuggestions, setIsLoadingSuggestions] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [applyError, setApplyError] = useState<string | null>(null);

  const canAutoFix = result.status === 'config_error' || result.status === 'suspicious' || result.status === 'unconfigured';

  const handleGetSuggestions = async () => {
    if (showSuggestions && suggestions.length > 0) {
      setShowSuggestions(false);
      return;
    }

    setIsLoadingSuggestions(true);
    setApplyError(null);
    try {
      const results = await getQuoteFixSuggestions(result.securityId, apiKeys);
      setSuggestions(results);
      setShowSuggestions(true);
    } catch (err) {
      setApplyError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoadingSuggestions(false);
    }
  };

  const handleApplySuggestion = async (suggestion: QuoteFixSuggestion) => {
    setIsApplying(true);
    setApplyError(null);
    try {
      await applyQuoteFix(
        suggestion.securityId,
        suggestion.suggestedProvider,
        suggestion.suggestedSymbol,
        suggestion.suggestedFeedUrl
      );
      // Notify parent to remove this security from list (local update, no refetch)
      onFixed(result.securityId, result.status);
    } catch (err) {
      setApplyError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsApplying(false);
    }
  };

  const getSourceLabel = (source: string) => {
    switch (source) {
      case 'known_mapping':
        return 'Bekannte Korrektur';
      case 'isin_search':
        return 'ISIN-Suche';
      case 'suffix_variant':
        return 'Suffix-Variante';
      case 'yahoo_search':
        return 'Yahoo Suche';
      case 'tradingview_search':
        return 'TradingView Suche';
      default:
        return source;
    }
  };

  return (
    <div className="p-3 border border-border rounded-lg bg-card space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex-1 min-w-0">
          <div className="font-medium truncate">{result.securityName}</div>
          <div className="text-xs text-muted-foreground flex items-center gap-2">
            <span>{result.feed}</span>
            {result.ticker && <span className="font-mono">{result.ticker}</span>}
            {result.lastPriceDate && (
              <span className="text-muted-foreground/70">Letzter Kurs: {result.lastPriceDate}</span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          {canAutoFix && (
            <button
              onClick={handleGetSuggestions}
              disabled={isLoadingSuggestions || isApplying}
              className="inline-flex items-center gap-1 px-2 py-1 text-xs font-medium rounded bg-blue-100 text-blue-700 hover:bg-blue-200 transition-colors disabled:opacity-50"
              title="Auto-Fix Vorschläge"
            >
              {isLoadingSuggestions ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <Wrench size={12} />
              )}
              Auto-Fix
              <ChevronRight size={12} className={`transition-transform ${showSuggestions ? 'rotate-90' : ''}`} />
            </button>
          )}
          <StatusBadge status={result.status} days={result.daysSinceLastPrice} />
        </div>
      </div>

      {/* Show error message for config_error */}
      {result.status === 'config_error' && result.errorMessage && (
        <div className="text-xs text-orange-600 bg-orange-50 p-2 rounded">
          {result.errorMessage}
        </div>
      )}

      {/* Show price comparison for suspicious */}
      {result.status === 'suspicious' && result.lastKnownPrice && result.fetchedPrice && result.priceDeviation && (
        <div className="text-xs bg-purple-50 p-2 rounded space-y-1">
          <div className="flex justify-between">
            <span className="text-purple-600">Letzter bekannter Kurs:</span>
            <span className="font-mono font-medium">{formatPrice(result.lastKnownPrice)}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-purple-600">Aktuell abgerufen:</span>
            <span className="font-mono font-medium">{formatPrice(result.fetchedPrice)}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-purple-600">Abweichung:</span>
            <span className={`font-mono font-medium ${result.priceDeviation > 0 ? 'text-green-600' : 'text-red-600'}`}>
              {result.priceDeviation > 0 ? '+' : ''}{result.priceDeviation.toFixed(1)}%
            </span>
          </div>
        </div>
      )}

      {/* Apply error */}
      {applyError && (
        <div className="text-xs text-red-600 bg-red-50 p-2 rounded">
          Fehler: {applyError}
        </div>
      )}

      {/* Suggestions */}
      {showSuggestions && (
        <div className="space-y-2 pt-2 border-t border-border">
          {suggestions.length === 0 ? (
            <div className="text-xs text-muted-foreground text-center py-2">
              Keine Vorschläge gefunden
            </div>
          ) : (
            <>
              <div className="text-xs font-medium text-muted-foreground">Vorschläge:</div>
              {suggestions.map((suggestion, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between p-2 bg-blue-50 rounded text-xs"
                >
                  <div className="flex-1">
                    <div className="font-medium text-blue-800 flex items-center gap-2">
                      <span>{suggestion.suggestedProvider}: {suggestion.suggestedSymbol}</span>
                      {suggestion.validatedPrice && (
                        <span className="inline-flex items-center gap-1 text-green-600 font-normal">
                          <Check size={12} />
                          Validiert
                        </span>
                      )}
                    </div>
                    <div className="text-blue-600">
                      {getSourceLabel(suggestion.source)}
                      {suggestion.validatedPrice && (
                        <span className="ml-2 text-green-600 font-medium">
                          Kurs: {formatPrice(suggestion.validatedPrice)}
                        </span>
                      )}
                    </div>
                  </div>
                  <button
                    onClick={() => handleApplySuggestion(suggestion)}
                    disabled={isApplying}
                    className="px-2 py-1 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors disabled:opacity-50 flex items-center gap-1"
                  >
                    {isApplying ? (
                      <Loader2 size={12} className="animate-spin" />
                    ) : (
                      <Check size={12} />
                    )}
                    Anwenden
                  </button>
                </div>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
}

export function QuoteAuditModal({
  isOpen,
  onClose,
  onComplete,
}: QuoteAuditModalProps) {
  const [auditData, setAuditData] = useState<QuoteAuditSummary | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasFixesApplied, setHasFixesApplied] = useState(false);

  // Get API keys from settings store
  const finnhubApiKey = useSettingsStore((state) => state.finnhubApiKey);
  const alphaVantageApiKey = useSettingsStore((state) => state.alphaVantageApiKey);
  const coingeckoApiKey = useSettingsStore((state) => state.coingeckoApiKey);
  const twelveDataApiKey = useSettingsStore((state) => state.twelveDataApiKey);

  useEscapeKey(isOpen, onClose);

  // Memoize API keys object
  const apiKeys = useMemo<ApiKeys>(() => ({
    finnhub: finnhubApiKey || undefined,
    alphaVantage: alphaVantageApiKey || undefined,
    coingecko: coingeckoApiKey || undefined,
    twelveData: twelveDataApiKey || undefined,
  }), [finnhubApiKey, alphaVantageApiKey, coingeckoApiKey, twelveDataApiKey]);

  const loadAuditData = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const result = await auditQuoteConfigurations(true, apiKeys);
      setAuditData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [apiKeys]);

  useEffect(() => {
    if (isOpen) {
      loadAuditData();
      setHasFixesApplied(false);
    }
  }, [isOpen, loadAuditData]);

  // Handle local removal of fixed security (without refetching all data)
  const handleSecurityFixed = useCallback((securityId: number, status: string) => {
    setHasFixesApplied(true);
    setAuditData(prev => {
      if (!prev) return null;
      return {
        ...prev,
        results: prev.results.filter(r => r.securityId !== securityId),
        okCount: prev.okCount + 1, // Fixed security is now OK
        unconfiguredCount: prev.unconfiguredCount - (status === 'unconfigured' ? 1 : 0),
        staleCount: prev.staleCount - (status === 'stale' ? 1 : 0),
        missingCount: prev.missingCount - (status === 'missing' ? 1 : 0),
        configErrorCount: prev.configErrorCount - (status === 'config_error' ? 1 : 0),
        suspiciousCount: prev.suspiciousCount - (status === 'suspicious' ? 1 : 0),
      };
    });
  }, []);

  const handleClose = () => {
    // Trigger parent refresh only when modal is closed and fixes were applied
    if (hasFixesApplied) {
      onComplete();
    }
    onClose();
  };

  if (!isOpen) return null;

  const hasIssues = auditData && (
    auditData.staleCount > 0 ||
    auditData.missingCount > 0 ||
    auditData.configErrorCount > 0 ||
    auditData.suspiciousCount > 0 ||
    auditData.unconfiguredCount > 0
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={handleClose}
      />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-lg w-full max-w-xl mx-4 max-h-[85vh] flex flex-col overflow-hidden">
        {/* Header */}
        <div className="bg-primary/10 p-6 text-center flex-shrink-0">
          <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <ClipboardCheck size={32} className="text-primary" />
          </div>
          <h2 className="text-xl font-semibold">Kursquellen-Prüfung</h2>
          {auditData && (
            <p className="text-muted-foreground mt-2">
              {auditData.totalAudited} Wertpapiere geprüft
            </p>
          )}
        </div>

        {/* Content */}
        <div className="p-6 flex-1 overflow-y-auto">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-8">
              <RefreshCw size={32} className="text-primary animate-spin mb-4" />
              <p className="text-muted-foreground">Prüfe Kursdaten und Konfiguration...</p>
              <p className="text-xs text-muted-foreground/70 mt-1">Ruft Kurse von den Providern ab</p>
            </div>
          ) : error ? (
            <div className="flex items-start gap-3 p-4 bg-destructive/10 border border-destructive/20 rounded-md">
              <AlertCircle className="text-destructive flex-shrink-0 mt-0.5" size={18} />
              <div className="text-sm">{error}</div>
            </div>
          ) : auditData && auditData.totalAudited === 0 ? (
            <div className="text-center py-8">
              <AlertCircle size={48} className="text-muted-foreground mx-auto mb-4" />
              <p className="font-medium">Keine konfigurierten Kursquellen gefunden</p>
              <p className="text-muted-foreground mt-2 text-sm">
                Konfiguriere zuerst Kursquellen für deine Wertpapiere.
              </p>
            </div>
          ) : auditData ? (
            <div className="space-y-4">
              {/* Summary Stats - 6 columns */}
              <div className="grid grid-cols-6 gap-2 text-center">
                <div className="bg-green-50 border border-green-200 rounded-lg p-2">
                  <div className="text-xl font-bold text-green-700">{auditData.okCount}</div>
                  <div className="text-xs text-green-600">Aktuell</div>
                </div>
                <div className="bg-blue-50 border border-blue-200 rounded-lg p-2">
                  <div className="text-xl font-bold text-blue-700">{auditData.unconfiguredCount}</div>
                  <div className="text-xs text-blue-600">Unkonfig.</div>
                </div>
                <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-2">
                  <div className="text-xl font-bold text-yellow-700">{auditData.staleCount}</div>
                  <div className="text-xs text-yellow-600">Veraltet</div>
                </div>
                <div className="bg-red-50 border border-red-200 rounded-lg p-2">
                  <div className="text-xl font-bold text-red-700">{auditData.missingCount}</div>
                  <div className="text-xs text-red-600">Keine Kurse</div>
                </div>
                <div className="bg-orange-50 border border-orange-200 rounded-lg p-2">
                  <div className="text-xl font-bold text-orange-700">{auditData.configErrorCount}</div>
                  <div className="text-xs text-orange-600">Konfig-Fehler</div>
                </div>
                <div className="bg-purple-50 border border-purple-200 rounded-lg p-2">
                  <div className="text-xl font-bold text-purple-700">{auditData.suspiciousCount}</div>
                  <div className="text-xs text-purple-600">Verdächtig</div>
                </div>
              </div>

              {/* Results List - only shows issues */}
              {hasIssues ? (
                <div className="space-y-2">
                  <div className="text-sm font-medium text-muted-foreground">
                    {auditData.results.length} Wertpapiere mit Problemen:
                  </div>
                  {auditData.results.map((result) => (
                    <SecurityRow
                      key={result.securityId}
                      result={result}
                      apiKeys={apiKeys}
                      onFixed={handleSecurityFixed}
                    />
                  ))}
                </div>
              ) : (
                <div className="text-center py-8 bg-green-50 border border-green-200 rounded-lg">
                  <Check size={48} className="text-green-600 mx-auto mb-4" />
                  <p className="font-medium text-green-700">Alle Kursquellen OK</p>
                  <p className="text-green-600 mt-2 text-sm">
                    Alle Kurse aktuell und plausibel. Konfiguration funktioniert.
                  </p>
                </div>
              )}
            </div>
          ) : null}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-border flex-shrink-0">
          <div className="flex gap-3">
            <button
              type="button"
              onClick={loadAuditData}
              disabled={isLoading}
              className="px-4 py-2 border border-border rounded-lg hover:bg-muted transition-colors disabled:opacity-50"
            >
              <RefreshCw size={16} className={`inline mr-1 ${isLoading ? 'animate-spin' : ''}`} />
              Aktualisieren
            </button>
            <button
              type="button"
              onClick={handleClose}
              className="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
            >
              Schließen
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
