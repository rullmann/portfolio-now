/**
 * Historical Quotes Modal
 * Loads historical prices for multiple securities with progress tracking.
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  X, RefreshCw, Check, AlertCircle, Clock, History, Download
} from 'lucide-react';
import { useEscapeKey } from '../../lib/hooks';
import {
  fetchHistoricalPricesBatch,
  cancelHistoricalBatch,
  type HistoricalBatchProgress,
  type HistoricalBatchResult,
  type ApiKeys,
} from '../../lib/api';
import { useSettingsStore } from '../../store';

interface HistoricalQuotesModalProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete?: () => void;
}

const STATUS_ICONS = {
  pending: Clock,
  loading: RefreshCw,
  success: Check,
  error: AlertCircle,
};

const STATUS_COLORS = {
  pending: 'text-muted-foreground',
  loading: 'text-primary',
  success: 'text-green-600',
  error: 'text-red-600',
};

export function HistoricalQuotesModal({ isOpen, onClose, onComplete }: HistoricalQuotesModalProps) {
  const currentYear = new Date().getFullYear();
  const [fromYear, setFromYear] = useState(currentYear - 3);
  const [onlyHeld, setOnlyHeld] = useState(true);
  const [forceReload, setForceReload] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [progress, setProgress] = useState<Map<number, HistoricalBatchProgress>>(new Map());
  const [result, setResult] = useState<HistoricalBatchResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const unlistenRef = useRef<UnlistenFn | null>(null);

  // Get API keys from settings
  const finnhubApiKey = useSettingsStore((state) => state.finnhubApiKey);
  const coingeckoApiKey = useSettingsStore((state) => state.coingeckoApiKey);
  const alphaVantageApiKey = useSettingsStore((state) => state.alphaVantageApiKey);
  const twelveDataApiKey = useSettingsStore((state) => state.twelveDataApiKey);

  // Note: handleClose is defined later, escape key handled via onClose
  // The modal close will trigger state reset via useEffect
  useEscapeKey(isOpen, () => {
    if (isLoading) {
      cancelHistoricalBatch().catch(console.error);
      setIsLoading(false);
    }
    onClose();
  });

  // Cleanup listener on unmount
  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);

  // Reset state when modal closes
  useEffect(() => {
    if (!isOpen) {
      setProgress(new Map());
      setResult(null);
      setError(null);
      setIsLoading(false);
      setForceReload(false);
    }
  }, [isOpen]);

  const handleStart = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    setProgress(new Map());
    setResult(null);

    // Set up progress listener
    try {
      unlistenRef.current = await listen<HistoricalBatchProgress>(
        'historical_quotes_progress',
        (event) => {
          setProgress((prev) => {
            const next = new Map(prev);
            next.set(event.payload.securityId, event.payload);
            return next;
          });
        }
      );
    } catch (err) {
      console.error('Failed to set up progress listener:', err);
    }

    // Build API keys
    const apiKeys: ApiKeys = {
      finnhub: finnhubApiKey || undefined,
      coingecko: coingeckoApiKey || undefined,
      alphaVantage: alphaVantageApiKey || undefined,
      twelveData: twelveDataApiKey || undefined,
    };

    try {
      const batchResult = await fetchHistoricalPricesBatch(
        {
          securityIds: [], // Empty = all securities with quote source
          fromYear,
          onlyHeld,
          force: forceReload,
        },
        apiKeys
      );
      setResult(batchResult);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
      // Cleanup listener
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    }
  }, [fromYear, onlyHeld, forceReload, finnhubApiKey, coingeckoApiKey, alphaVantageApiKey, twelveDataApiKey]);

  const handleClose = () => {
    if (isLoading) {
      // Cancel the batch fetch before closing
      cancelHistoricalBatch().catch(console.error);
      setIsLoading(false);
    }
    if (result && result.successful > 0) {
      onComplete?.();
    }
    onClose();
  };

  if (!isOpen) return null;

  // Convert progress map to sorted array
  const progressList = Array.from(progress.values()).sort((a, b) => a.currentIndex - b.currentIndex);
  const totalCount = progressList.length > 0 ? progressList[0].totalCount : 0;
  const completedCount = progressList.filter((p) => p.status === 'success' || p.status === 'error').length;
  const progressPercent = totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0;

  // Year options (2010 to current year)
  const yearOptions = [];
  for (let y = 2010; y <= currentYear; y++) {
    yearOptions.push(y);
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={isLoading ? undefined : handleClose}
      />

      {/* Modal */}
      <div className="relative bg-background rounded-xl shadow-2xl w-full max-w-3xl max-h-[85vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary/10 rounded-lg">
              <History size={20} className="text-primary" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">Historische Kurse laden</h2>
              <p className="text-sm text-muted-foreground">
                Kursdaten von Kursquellen für alle Wertpapiere abrufen
              </p>
            </div>
          </div>
          <button
            onClick={handleClose}
            className="p-2 hover:bg-muted rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-4">
          {/* Configuration - only show when not loading and no result */}
          {!isLoading && !result && (
            <div className="space-y-4">
              {/* Year selection */}
              <div className="flex items-center gap-4">
                <label className="text-sm font-medium min-w-[120px]">
                  Kurse laden ab Jahr:
                </label>
                <select
                  value={fromYear}
                  onChange={(e) => setFromYear(Number(e.target.value))}
                  className="px-3 py-2 border border-border rounded-lg bg-background text-sm"
                >
                  {yearOptions.map((y) => (
                    <option key={y} value={y}>{y}</option>
                  ))}
                </select>
                <span className="text-sm text-muted-foreground">bis: Heute</span>
              </div>

              {/* Only held checkbox */}
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={onlyHeld}
                  onChange={(e) => setOnlyHeld(e.target.checked)}
                  className="rounded border-border"
                />
                <span className="text-sm">Nur Wertpapiere im Bestand</span>
              </label>

              {/* Force reload checkbox */}
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={forceReload}
                  onChange={(e) => setForceReload(e.target.checked)}
                  className="rounded border-border"
                />
                <span className="text-sm">Alle Kurse überschreiben (Force)</span>
              </label>

              {/* Start button */}
              <button
                onClick={handleStart}
                className="flex items-center justify-center gap-2 w-full px-4 py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors font-medium"
              >
                <Download size={18} />
                Kurse laden starten
              </button>
            </div>
          )}

          {/* Progress display - CHANGED: removed && !result to keep visible after completion */}
          {(isLoading || progressList.length > 0) && (
            <div className="space-y-4">
              {/* Overall progress */}
              {totalCount > 0 && (
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span className="font-medium">
                      Fortschritt: {completedCount} / {totalCount} Wertpapiere
                    </span>
                    <span className="text-muted-foreground">{progressPercent}%</span>
                  </div>
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className="h-full bg-primary transition-all duration-300"
                      style={{ width: `${progressPercent}%` }}
                    />
                  </div>
                </div>
              )}

              {/* Security list */}
              <div className="space-y-2 max-h-[400px] overflow-y-auto">
                {progressList.map((item) => {
                  const StatusIcon = STATUS_ICONS[item.status];
                  const statusColor = STATUS_COLORS[item.status];

                  return (
                    <div
                      key={item.securityId}
                      className="flex items-center justify-between p-3 bg-muted/30 rounded-lg"
                    >
                      <div className="flex items-center gap-3">
                        <StatusIcon
                          size={18}
                          className={`${statusColor} ${item.status === 'loading' ? 'animate-spin' : ''}`}
                        />
                        <div>
                          <div className="font-medium text-sm">{item.securityName}</div>
                          <div className="text-xs text-muted-foreground">
                            {item.ticker} • {item.provider}
                          </div>
                        </div>
                      </div>
                      <div className="text-right text-sm">
                        {item.status === 'loading' && (
                          <span className="text-muted-foreground">Lädt...</span>
                        )}
                        {item.status === 'success' && (
                          <div className="text-xs space-y-0.5">
                            {item.spikesDeleted > 0 && (
                              <div className="text-amber-600">{item.spikesDeleted} Spikes gelöscht</div>
                            )}
                            <div className="text-green-600">
                              {item.quotesInserted === 0 && item.quotesUpdated === 0 ? (
                                'Aktuell'
                              ) : (
                                <>
                                  {item.quotesInserted > 0 && `${item.quotesInserted} neu`}
                                  {item.quotesUpdated > 0 && `${item.quotesInserted > 0 ? ' | ' : ''}${item.quotesUpdated} aktualisiert`}
                                  {item.quotesSkipped > 0 && ` | ${item.quotesSkipped} übersprungen`}
                                </>
                              )}
                            </div>
                          </div>
                        )}
                        {item.status === 'error' && (
                          <span className="text-red-600 text-xs" title={item.error}>
                            Fehler: {item.error?.slice(0, 40)}{(item.error?.length ?? 0) > 40 ? '...' : ''}
                          </span>
                        )}
                        {item.status === 'pending' && (
                          <span className="text-muted-foreground">Wartend</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Error display */}
          {error && (
            <div className="flex items-center gap-3 p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-red-600">
              <AlertCircle size={20} />
              <span>{error}</span>
            </div>
          )}

          {/* CHANGED: Compact result summary (appears below the security list) */}
          {result && (
            <div className="space-y-3 mt-4 pt-4 border-t border-border">
              {/* Compact stats row */}
              <div className="flex items-center justify-center gap-4 flex-wrap text-sm">
                <div className="flex items-center gap-1.5">
                  <Check size={16} className="text-green-600" />
                  <span className="font-medium text-green-600">{result.successful}</span>
                  <span className="text-muted-foreground">Erfolgreich</span>
                </div>
                <span className="text-muted-foreground">|</span>
                <div className="flex items-center gap-1.5">
                  <span className="font-medium text-primary">{result.totalInserted.toLocaleString('de-DE')}</span>
                  <span className="text-muted-foreground">Neu</span>
                </div>
                {result.totalSpikesDeleted > 0 && (
                  <>
                    <span className="text-muted-foreground">|</span>
                    <div className="flex items-center gap-1.5">
                      <span className="font-medium text-amber-600">{result.totalSpikesDeleted.toLocaleString('de-DE')}</span>
                      <span className="text-muted-foreground">Spikes</span>
                    </div>
                  </>
                )}
                {result.totalUpdated > 0 && (
                  <>
                    <span className="text-muted-foreground">|</span>
                    <div className="flex items-center gap-1.5">
                      <span className="font-medium text-blue-600">{result.totalUpdated.toLocaleString('de-DE')}</span>
                      <span className="text-muted-foreground">Aktualisiert</span>
                    </div>
                  </>
                )}
                {result.failed > 0 && (
                  <>
                    <span className="text-muted-foreground">|</span>
                    <div className="flex items-center gap-1.5">
                      <span className="font-medium text-red-600">{result.failed}</span>
                      <span className="text-muted-foreground">Fehler</span>
                    </div>
                  </>
                )}
              </div>

              {/* Error list if any */}
              {result.failed > 0 && (
                <div className="space-y-2">
                  <p className="text-sm font-medium text-red-600">Fehlgeschlagene Abrufe:</p>
                  <div className="max-h-[200px] overflow-y-auto space-y-1">
                    {result.progress
                      .filter((p) => p.status === 'error')
                      .map((item) => (
                        <div
                          key={item.securityId}
                          className="flex items-center justify-between p-2 bg-red-500/5 rounded text-sm"
                        >
                          <span>{item.securityName}</span>
                          <span className="text-red-600 text-xs" title={item.error}>
                            {item.error?.slice(0, 50)}{(item.error?.length ?? 0) > 50 ? '...' : ''}
                          </span>
                        </div>
                      ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end p-4 border-t border-border bg-muted/30">
          <button
            onClick={handleClose}
            className="px-4 py-2 bg-muted hover:bg-muted/80 rounded-lg transition-colors"
          >
            {result ? 'Schließen' : 'Abbrechen'}
          </button>
        </div>
      </div>
    </div>
  );
}
