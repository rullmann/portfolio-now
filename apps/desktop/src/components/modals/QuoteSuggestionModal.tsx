/**
 * Modal for suggesting quote providers for securities without configured feed.
 * Shows suggestions one by one with accept/skip/cancel workflow.
 */

import { useState, useCallback, useEffect } from 'react';
import { Sparkles, Check, X, SkipForward, RefreshCw, AlertCircle, Briefcase, Database } from 'lucide-react';
import { useEscapeKey } from '../../lib/hooks';
import { suggestQuoteProviders, applyQuoteSuggestion, getUnconfiguredSecuritiesCount, type QuoteSuggestion } from '../../lib/api';

type FilterMode = 'held' | 'all';

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
  const [filterMode, setFilterMode] = useState<FilterMode>('held');
  const [showModeSelection, setShowModeSelection] = useState(true);
  const [suggestions, setSuggestions] = useState<QuoteSuggestion[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [appliedCount, setAppliedCount] = useState(0);
  const [skippedCount, setSkippedCount] = useState(0);
  const [counts, setCounts] = useState<{ held: number; total: number } | null>(null);

  useEscapeKey(isOpen && !isApplying, onClose);

  // Load counts when modal opens
  const loadCounts = useCallback(async () => {
    try {
      const info = await getUnconfiguredSecuritiesCount();
      setCounts({ held: info.heldUnconfigured, total: info.totalUnconfigured });
    } catch (err) {
      console.error('Failed to load counts:', err);
    }
  }, []);

  // Reset state when modal opens
  const handleOpen = useCallback(() => {
    setShowModeSelection(true);
    setSuggestions([]);
    setCurrentIndex(0);
    setAppliedCount(0);
    setSkippedCount(0);
    setError(null);
    loadCounts();
  }, [loadCounts]);

  // Load suggestions with selected filter mode
  const loadSuggestions = useCallback(async (mode: FilterMode) => {
    setIsLoading(true);
    setError(null);
    setCurrentIndex(0);
    setAppliedCount(0);
    setSkippedCount(0);
    setShowModeSelection(false);

    try {
      const heldOnly = mode === 'held';
      const result = await suggestQuoteProviders(undefined, heldOnly);
      setSuggestions(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Start with selected mode
  const handleStartAnalysis = () => {
    loadSuggestions(filterMode);
  };

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
        currentSuggestion.suggestedFeedUrl,
        currentSuggestion.suggestedTicker
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
    // Reset state for next open
    setCounts(null);
    setShowModeSelection(true);
    setSuggestions([]);
    onClose();
  };

  // Reset and load counts when modal opens
  useEffect(() => {
    if (isOpen && showModeSelection && counts === null) {
      handleOpen();
    }
  }, [isOpen, showModeSelection, counts, handleOpen]);

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
          {!isLoading && !showModeSelection && suggestions.length > 0 && !isComplete && (
            <p className="text-muted-foreground mt-2">
              {currentIndex + 1} von {suggestions.length} Wertpapieren
            </p>
          )}
        </div>

        {/* Content */}
        <div className="p-6">
          {/* Mode Selection */}
          {showModeSelection ? (
            <div className="space-y-4">
              <p className="text-sm text-muted-foreground text-center">
                Welche Wertpapiere sollen analysiert werden?
              </p>

              {/* Radio Options */}
              <div className="space-y-3">
                <label
                  className={`flex items-center gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
                    filterMode === 'held'
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  <input
                    type="radio"
                    name="filterMode"
                    value="held"
                    checked={filterMode === 'held'}
                    onChange={() => setFilterMode('held')}
                    className="sr-only"
                  />
                  <div className={`w-10 h-10 rounded-full flex items-center justify-center ${
                    filterMode === 'held' ? 'bg-primary/20' : 'bg-muted'
                  }`}>
                    <Briefcase size={20} className={filterMode === 'held' ? 'text-primary' : 'text-muted-foreground'} />
                  </div>
                  <div className="flex-1">
                    <div className="font-medium">Nur im Bestand</div>
                    <div className="text-sm text-muted-foreground">
                      Wertpapiere mit aktuellen Positionen
                      {counts && <span className="ml-1">({counts.held})</span>}
                    </div>
                  </div>
                  <div className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                    filterMode === 'held' ? 'border-primary' : 'border-muted-foreground/30'
                  }`}>
                    {filterMode === 'held' && <div className="w-2.5 h-2.5 rounded-full bg-primary" />}
                  </div>
                </label>

                <label
                  className={`flex items-center gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
                    filterMode === 'all'
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  <input
                    type="radio"
                    name="filterMode"
                    value="all"
                    checked={filterMode === 'all'}
                    onChange={() => setFilterMode('all')}
                    className="sr-only"
                  />
                  <div className={`w-10 h-10 rounded-full flex items-center justify-center ${
                    filterMode === 'all' ? 'bg-primary/20' : 'bg-muted'
                  }`}>
                    <Database size={20} className={filterMode === 'all' ? 'text-primary' : 'text-muted-foreground'} />
                  </div>
                  <div className="flex-1">
                    <div className="font-medium">Alle Wertpapiere</div>
                    <div className="text-sm text-muted-foreground">
                      Alle nicht archivierten Wertpapiere
                      {counts && <span className="ml-1">({counts.total})</span>}
                    </div>
                  </div>
                  <div className={`w-5 h-5 rounded-full border-2 flex items-center justify-center ${
                    filterMode === 'all' ? 'border-primary' : 'border-muted-foreground/30'
                  }`}>
                    {filterMode === 'all' && <div className="w-2.5 h-2.5 rounded-full bg-primary" />}
                  </div>
                </label>
              </div>

              {/* Action Buttons */}
              <div className="flex gap-3 pt-2">
                <button
                  type="button"
                  onClick={handleClose}
                  className="px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
                >
                  Abbrechen
                </button>
                <button
                  type="button"
                  onClick={handleStartAnalysis}
                  className="flex-1 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
                >
                  <Sparkles size={18} className="inline mr-2" />
                  Analyse starten
                </button>
              </div>
            </div>
          ) : isLoading ? (
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
                {/* Show suggested ticker if security has no ticker */}
                {currentSuggestion.suggestedTicker && !currentSuggestion.ticker && (
                  <div className="mt-3 pt-3 border-t border-primary/20">
                    <div className="flex items-center gap-2">
                      <span className="text-sm text-muted-foreground">Ticker-Vorschlag:</span>
                      <span className="font-mono font-medium text-primary">
                        {currentSuggestion.suggestedTicker}
                      </span>
                    </div>
                    <p className="text-xs text-muted-foreground mt-1">
                      Der Ticker wird automatisch gesetzt, da keiner vorhanden ist.
                    </p>
                  </div>
                )}
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
          {(isComplete || suggestions.length === 0) && !isLoading && !showModeSelection && (
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
