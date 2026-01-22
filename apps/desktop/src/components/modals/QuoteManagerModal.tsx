/**
 * Unified Quote Manager Modal
 * Combines quote suggestion and audit functionality into one view.
 * Shows all securities with issues and validated fix suggestions.
 */

import { useState, useCallback, useEffect } from 'react';
import {
  AlertCircle, Check, RefreshCw, X, Zap,
  AlertTriangle, Clock, Database, Search, Bot
} from 'lucide-react';
import { useEscapeKey } from '../../lib/hooks';
import {
  quoteManagerAudit,
  applyQuoteManagerSuggestion,
  type QuoteManagerResult,
  type QuoteManagerItem,
  type ValidatedSuggestion
} from '../../lib/api';
import { QuoteAssistantModal } from './QuoteAssistantModal';

interface QuoteManagerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete: () => void;
}

const STATUS_CONFIG = {
  unconfigured: {
    icon: Database,
    label: 'Nicht konfiguriert',
    color: 'text-amber-600',
    bgColor: 'bg-amber-500/10',
    borderColor: 'border-amber-500/30',
  },
  error: {
    icon: AlertCircle,
    label: 'Fehler',
    color: 'text-red-600',
    bgColor: 'bg-red-500/10',
    borderColor: 'border-red-500/30',
  },
  stale: {
    icon: Clock,
    label: 'Veraltet',
    color: 'text-yellow-600',
    bgColor: 'bg-yellow-500/10',
    borderColor: 'border-yellow-500/30',
  },
  no_data: {
    icon: AlertTriangle,
    label: 'Keine Daten',
    color: 'text-orange-600',
    bgColor: 'bg-orange-500/10',
    borderColor: 'border-orange-500/30',
  },
};

const SOURCE_LABELS: Record<string, string> = {
  known_mapping: 'Bekanntes Mapping',
  isin_search: 'ISIN-Suche',
  suffix_variant: 'Börsen-Suffix',
  name_search: 'Namenssuche',
  tradingview_search: 'TradingView',
};

export function QuoteManagerModal({ isOpen, onClose, onComplete }: QuoteManagerModalProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [result, setResult] = useState<QuoteManagerResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [applyingId, setApplyingId] = useState<number | null>(null);
  const [appliedIds, setAppliedIds] = useState<Set<number>>(new Set());
  const [onlyHeld, setOnlyHeld] = useState(true);
  const [showAiAssistant, setShowAiAssistant] = useState(false);

  useEscapeKey(isOpen, onClose);

  const loadData = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await quoteManagerAudit(onlyHeld);
      setResult(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [onlyHeld]);

  // Load data when modal opens
  useEffect(() => {
    if (isOpen && !result && !isLoading) {
      loadData();
    }
  }, [isOpen, result, isLoading, loadData]);

  // Reset state when modal closes
  useEffect(() => {
    if (!isOpen) {
      setResult(null);
      setAppliedIds(new Set());
    }
  }, [isOpen]);

  const handleApply = async (item: QuoteManagerItem, suggestion: ValidatedSuggestion) => {
    setApplyingId(item.securityId);
    try {
      await applyQuoteManagerSuggestion(
        item.securityId,
        suggestion.provider,
        suggestion.symbol,
        suggestion.feedUrl
      );
      setAppliedIds(prev => new Set([...prev, item.securityId]));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setApplyingId(null);
    }
  };

  const handleClose = () => {
    if (appliedIds.size > 0) {
      onComplete();
    }
    onClose();
  };

  if (!isOpen) return null;

  const pendingItems = result?.items.filter(item => !appliedIds.has(item.securityId)) ?? [];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={handleClose}
      />

      {/* Modal */}
      <div className="relative bg-background rounded-xl shadow-2xl w-full max-w-4xl max-h-[85vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary/10 rounded-lg">
              <Zap size={20} className="text-primary" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">Kursquellen-Manager</h2>
              <p className="text-sm text-muted-foreground">
                Alle Probleme auf einen Blick mit validierten Lösungen
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowAiAssistant(true)}
              className="flex items-center gap-2 px-3 py-1.5 bg-primary/10 text-primary rounded-lg hover:bg-primary/20 transition-colors"
            >
              <Bot size={16} />
              <span className="text-sm font-medium">KI-Assistent</span>
            </button>
            <button
              onClick={handleClose}
              className="p-2 hover:bg-muted rounded-lg transition-colors"
            >
              <X size={20} />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-4">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-12 gap-4">
              <RefreshCw size={32} className="animate-spin text-primary" />
              <div className="text-center">
                <p className="font-medium">Analysiere Kursquellen...</p>
                <p className="text-sm text-muted-foreground">
                  Suche nach Problemen und validiere Lösungen
                </p>
              </div>
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center py-12 gap-4">
              <AlertCircle size={32} className="text-red-500" />
              <p className="text-red-600">{error}</p>
              <button
                onClick={loadData}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
              >
                Erneut versuchen
              </button>
            </div>
          ) : result && pendingItems.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 gap-4">
              <div className="p-4 bg-green-500/10 rounded-full">
                <Check size={32} className="text-green-600" />
              </div>
              <div className="text-center">
                <p className="font-medium text-lg">
                  {appliedIds.size > 0
                    ? `${appliedIds.size} Konfiguration${appliedIds.size > 1 ? 'en' : ''} angewendet!`
                    : 'Alle Kursquellen sind korrekt konfiguriert'}
                </p>
                <p className="text-sm text-muted-foreground mt-1">
                  {result.totalSecurities} Wertpapiere geprüft
                </p>
              </div>
            </div>
          ) : result ? (
            <div className="space-y-4">
              {/* Summary */}
              <div className="flex items-center justify-between bg-muted/50 rounded-lg p-3">
                <div className="flex items-center gap-4 text-sm">
                  <span>{result.totalWithIssues} Problem{result.totalWithIssues !== 1 ? 'e' : ''}</span>
                  {result.unconfiguredCount > 0 && (
                    <span className="text-amber-600">{result.unconfiguredCount} nicht konfiguriert</span>
                  )}
                  {result.errorCount > 0 && (
                    <span className="text-red-600">{result.errorCount} Fehler</span>
                  )}
                  {result.staleCount > 0 && (
                    <span className="text-yellow-600">{result.staleCount} veraltet</span>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <label className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={onlyHeld}
                      onChange={(e) => {
                        setOnlyHeld(e.target.checked);
                        setResult(null);
                      }}
                      className="rounded border-border"
                    />
                    Nur im Bestand
                  </label>
                  <button
                    onClick={loadData}
                    disabled={isLoading}
                    className="p-2 hover:bg-muted rounded-lg transition-colors"
                    title="Neu laden"
                  >
                    <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
                  </button>
                </div>
              </div>

              {/* Items */}
              <div className="space-y-3">
                {pendingItems.map((item) => {
                  const statusConfig = STATUS_CONFIG[item.status];
                  const StatusIcon = statusConfig.icon;
                  const isApplying = applyingId === item.securityId;

                  return (
                    <div
                      key={item.securityId}
                      className={`border rounded-lg p-4 ${statusConfig.borderColor} ${statusConfig.bgColor}`}
                    >
                      {/* Security Info */}
                      <div className="flex items-start justify-between mb-3">
                        <div>
                          <div className="flex items-center gap-2">
                            <StatusIcon size={16} className={statusConfig.color} />
                            <span className="font-medium">{item.securityName}</span>
                          </div>
                          <div className="flex items-center gap-3 mt-1 text-sm text-muted-foreground">
                            {item.isin && (
                              <span className="font-mono">{item.isin}</span>
                            )}
                            {item.ticker && (
                              <span className="font-mono">{item.ticker}</span>
                            )}
                            {item.currency && (
                              <span>{item.currency}</span>
                            )}
                          </div>
                        </div>
                        <span className={`text-xs px-2 py-1 rounded ${statusConfig.bgColor} ${statusConfig.color}`}>
                          {statusConfig.label}
                        </span>
                      </div>

                      {/* Status Message */}
                      <p className="text-sm text-muted-foreground mb-3">
                        {item.statusMessage}
                        {item.lastPriceDate && (
                          <span className="ml-2">
                            (Letzter Kurs: {item.lastPriceDate})
                          </span>
                        )}
                      </p>

                      {/* Suggestions */}
                      {item.suggestions.length > 0 ? (
                        <div className="space-y-2">
                          <p className="text-xs font-medium text-muted-foreground flex items-center gap-1">
                            <Check size={12} className="text-green-600" />
                            Validierte Vorschläge:
                          </p>
                          <div className="flex flex-wrap gap-2">
                            {item.suggestions.map((suggestion, idx) => (
                              <button
                                key={`${suggestion.provider}-${suggestion.symbol}-${idx}`}
                                onClick={() => handleApply(item, suggestion)}
                                disabled={isApplying}
                                className={`
                                  flex items-center gap-2 px-3 py-2 rounded-lg border
                                  ${idx === 0
                                    ? 'bg-primary text-primary-foreground border-primary hover:bg-primary/90'
                                    : 'bg-background border-border hover:bg-muted'
                                  }
                                  disabled:opacity-50 transition-colors
                                `}
                              >
                                {isApplying ? (
                                  <RefreshCw size={14} className="animate-spin" />
                                ) : (
                                  <Zap size={14} />
                                )}
                                <span className="font-mono text-sm">{suggestion.symbol}</span>
                                <span className="text-xs opacity-75">
                                  ({suggestion.provider})
                                </span>
                                <span className="text-xs opacity-75">
                                  {suggestion.validatedPrice.toFixed(2)}
                                </span>
                              </button>
                            ))}
                          </div>
                          <p className="text-xs text-muted-foreground">
                            Quelle: {SOURCE_LABELS[item.suggestions[0]?.source] || item.suggestions[0]?.source}
                          </p>
                        </div>
                      ) : (
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                          <Search size={14} />
                          <span>Keine automatische Lösung gefunden - manuelle Konfiguration erforderlich</span>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          ) : null}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t border-border bg-muted/30">
          <div className="text-sm text-muted-foreground">
            {appliedIds.size > 0 && (
              <span className="text-green-600 font-medium">
                {appliedIds.size} angewendet
              </span>
            )}
          </div>
          <button
            onClick={handleClose}
            className="px-4 py-2 bg-muted hover:bg-muted/80 rounded-lg transition-colors"
          >
            Schließen
          </button>
        </div>
      </div>

      {/* AI Quote Assistant Modal */}
      <QuoteAssistantModal
        isOpen={showAiAssistant}
        onClose={() => setShowAiAssistant(false)}
        onApplied={() => {
          // Refresh the list after AI applied changes
          loadData();
        }}
      />
    </div>
  );
}
