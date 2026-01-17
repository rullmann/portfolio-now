/**
 * Modal for suggesting quote providers for securities without configured feed.
 * Shows suggestions one by one with accept/skip/cancel workflow.
 */

import { useState, useEffect, useCallback } from 'react';
import { Sparkles, Check, X, SkipForward, RefreshCw, AlertCircle } from 'lucide-react';
import { useEscapeKey } from '../../lib/hooks';
import { suggestQuoteProviders, applyQuoteSuggestion, type QuoteSuggestion } from '../../lib/api';

interface QuoteSuggestionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete: () => void;
}

export function QuoteSuggestionModal({
  isOpen,
  onClose,
  onComplete,
}: QuoteSuggestionModalProps) {
  const [suggestions, setSuggestions] = useState<QuoteSuggestion[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [appliedCount, setAppliedCount] = useState(0);
  const [skippedCount, setSkippedCount] = useState(0);

  useEscapeKey(isOpen && !isApplying, onClose);

  // Load suggestions when modal opens
  useEffect(() => {
    if (isOpen) {
      loadSuggestions();
    }
  }, [isOpen]);

  const loadSuggestions = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    setCurrentIndex(0);
    setAppliedCount(0);
    setSkippedCount(0);

    try {
      const result = await suggestQuoteProviders();
      setSuggestions(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const currentSuggestion = suggestions[currentIndex];
  const isComplete = currentIndex >= suggestions.length && suggestions.length > 0;

  const handleApply = async () => {
    if (!currentSuggestion) return;

    setIsApplying(true);
    setError(null);

    try {
      await applyQuoteSuggestion(
        currentSuggestion.securityId,
        currentSuggestion.suggestedFeed,
        currentSuggestion.suggestedFeedUrl
      );
      setAppliedCount((c) => c + 1);
      setCurrentIndex((i) => i + 1);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsApplying(false);
    }
  };

  const handleSkip = () => {
    setSkippedCount((c) => c + 1);
    setCurrentIndex((i) => i + 1);
  };

  const handleClose = () => {
    if (appliedCount > 0) {
      onComplete();
    }
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={isApplying ? undefined : handleClose}
      />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-lg w-full max-w-lg mx-4 overflow-hidden">
        {/* Header */}
        <div className="bg-primary/10 p-6 text-center">
          <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <Sparkles size={32} className="text-primary" />
          </div>
          <h2 className="text-xl font-semibold">Kursquellen vorschlagen</h2>
          {!isLoading && suggestions.length > 0 && !isComplete && (
            <p className="text-muted-foreground mt-2">
              {currentIndex + 1} von {suggestions.length} Wertpapieren
            </p>
          )}
        </div>

        {/* Content */}
        <div className="p-6">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-8">
              <RefreshCw size={32} className="text-primary animate-spin mb-4" />
              <p className="text-muted-foreground">Analysiere Wertpapiere...</p>
            </div>
          ) : error ? (
            <div className="flex items-start gap-3 p-4 bg-destructive/10 border border-destructive/20 rounded-md">
              <AlertCircle className="text-destructive flex-shrink-0 mt-0.5" size={18} />
              <div className="text-sm">{error}</div>
            </div>
          ) : suggestions.length === 0 ? (
            <div className="text-center py-8">
              <Check size={48} className="text-green-500 mx-auto mb-4" />
              <p className="font-medium">Alle Wertpapiere haben bereits eine Kursquelle!</p>
              <p className="text-muted-foreground mt-2 text-sm">
                Es gibt keine Wertpapiere ohne konfigurierte Kursquelle.
              </p>
            </div>
          ) : isComplete ? (
            <div className="text-center py-8">
              <Check size={48} className="text-green-500 mx-auto mb-4" />
              <p className="font-medium">Fertig!</p>
              <div className="mt-4 space-y-1 text-sm">
                <p className="text-green-600">
                  {appliedCount} Kursquelle{appliedCount !== 1 ? 'n' : ''} übernommen
                </p>
                {skippedCount > 0 && (
                  <p className="text-muted-foreground">
                    {skippedCount} übersprungen
                  </p>
                )}
              </div>
            </div>
          ) : currentSuggestion ? (
            <div className="space-y-4">
              {/* Security Info */}
              <div className="bg-muted/50 rounded-lg p-4">
                <h3 className="font-semibold text-lg">{currentSuggestion.securityName}</h3>
                <div className="mt-2 text-sm text-muted-foreground space-y-1">
                  {currentSuggestion.isin && (
                    <p>
                      <span className="text-muted-foreground/60">ISIN:</span>{' '}
                      <span className="font-mono">{currentSuggestion.isin}</span>
                    </p>
                  )}
                  {currentSuggestion.ticker && (
                    <p>
                      <span className="text-muted-foreground/60">Ticker:</span>{' '}
                      <span className="font-mono">{currentSuggestion.ticker}</span>
                    </p>
                  )}
                </div>
              </div>

              {/* Suggestion */}
              <div className="border border-primary/30 bg-primary/5 rounded-lg p-4">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Vorgeschlagene Kursquelle:</span>
                  <span className="text-xs px-2 py-0.5 bg-primary/10 rounded text-primary">
                    {Math.round(currentSuggestion.confidence * 100)}% Konfidenz
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-primary text-lg">
                    {currentSuggestion.suggestedFeed}
                  </span>
                  {currentSuggestion.suggestedFeedUrl && (
                    <span className="text-muted-foreground font-mono text-sm">
                      ({currentSuggestion.suggestedFeedUrl})
                    </span>
                  )}
                </div>
                <p className="text-sm text-muted-foreground mt-2">
                  {currentSuggestion.reason}
                </p>
              </div>

              {/* Action Buttons */}
              <div className="flex gap-3 pt-2">
                <button
                  type="button"
                  onClick={handleClose}
                  disabled={isApplying}
                  className="px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors disabled:opacity-50"
                >
                  <X size={18} className="inline mr-1" />
                  Abbrechen
                </button>
                <button
                  type="button"
                  onClick={handleSkip}
                  disabled={isApplying}
                  className="flex-1 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors disabled:opacity-50"
                >
                  <SkipForward size={18} className="inline mr-1" />
                  Überspringen
                </button>
                <button
                  type="button"
                  onClick={handleApply}
                  disabled={isApplying}
                  className="flex-1 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50"
                >
                  {isApplying ? (
                    <>
                      <RefreshCw size={18} className="inline mr-1 animate-spin" />
                      Speichere...
                    </>
                  ) : (
                    <>
                      <Check size={18} className="inline mr-1" />
                      Übernehmen
                    </>
                  )}
                </button>
              </div>
            </div>
          ) : null}

          {/* Close button for completion/empty states */}
          {(isComplete || suggestions.length === 0) && !isLoading && (
            <div className="pt-4">
              <button
                type="button"
                onClick={handleClose}
                className="w-full px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
              >
                Schließen
              </button>
            </div>
          )}
        </div>

        {/* Progress indicator */}
        {!isLoading && suggestions.length > 0 && !isComplete && (
          <div className="h-1 bg-muted">
            <div
              className="h-full bg-primary transition-all duration-300"
              style={{ width: `${((currentIndex) / suggestions.length) * 100}%` }}
            />
          </div>
        )}
      </div>
    </div>
  );
}
