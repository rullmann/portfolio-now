import { useEffect, useRef, useMemo } from 'react';
import { RefreshCw, Check, AlertTriangle, AlertCircle, X } from 'lucide-react';
import { useQuoteSyncStore } from '../../store/quoteSyncStore';
import { formatCurrency } from '../../lib/types';

export function QuoteSyncOverlay() {
  const { isActive, isComplete, items, totalCount, dismiss } =
    useQuoteSyncStore();
  const listRef = useRef<HTMLDivElement>(null);

  const itemsList = useMemo(() => Array.from(items.values()), [items]);

  const successCount = useMemo(
    () => itemsList.filter((i) => i.status === 'success').length,
    [itemsList]
  );
  const errorCount = useMemo(
    () => itemsList.filter((i) => i.status === 'error').length,
    [itemsList]
  );
  const currentLoading = useMemo(
    () => itemsList.find((i) => i.status === 'loading'),
    [itemsList]
  );

  const effectiveTotal = totalCount || (currentLoading?.totalCount ?? 0);
  const processed = successCount + errorCount;
  const progressPercent =
    effectiveTotal > 0 ? Math.round((processed / effectiveTotal) * 100) : 0;

  // Auto-scroll to bottom when new items arrive
  useEffect(() => {
    if (listRef.current) {
      listRef.current.scrollTop = listRef.current.scrollHeight;
    }
  }, [itemsList.length]);

  if (!isActive) return null;

  return (
    <div className="fixed bottom-20 right-4 z-40 w-[380px] rounded-xl border border-border/50 bg-card/80 backdrop-blur-xl shadow-xl animate-in slide-in-from-bottom-5 fade-in duration-300">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border/30">
        <div className="flex items-center gap-2">
          {!isComplete ? (
            <RefreshCw size={16} className="text-primary animate-spin" />
          ) : errorCount > 0 ? (
            <AlertTriangle size={16} className="text-amber-500" />
          ) : (
            <Check size={16} className="text-green-500" />
          )}
          <span className="text-sm font-medium">Kurse aktualisieren</span>
        </div>
        <button
          onClick={dismiss}
          className="p-1 rounded-md hover:bg-muted transition-colors text-muted-foreground"
        >
          <X size={14} />
        </button>
      </div>

      {/* Progress bar */}
      {effectiveTotal > 0 && (
        <div className="px-4 pt-3 pb-2">
          <div className="flex items-center justify-between text-xs text-muted-foreground mb-1.5">
            <span>
              {processed} von {effectiveTotal}
            </span>
            <span>{progressPercent}%</span>
          </div>
          <div className="h-1 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full rounded-full transition-all duration-500 ease-out"
              style={{
                width: `${progressPercent}%`,
                backgroundColor:
                  isComplete && errorCount > 0
                    ? 'var(--color-amber-500, #f59e0b)'
                    : isComplete
                      ? 'var(--color-green-500, #22c55e)'
                      : 'hsl(var(--primary))',
              }}
            />
          </div>
        </div>
      )}

      {/* Items list */}
      {itemsList.length > 0 && (
        <div
          ref={listRef}
          className="max-h-[240px] overflow-y-auto px-2 py-1"
        >
          {itemsList.map((item) => (
            <div
              key={item.securityId}
              className="flex items-center gap-2 px-2 py-1.5 rounded-md text-xs animate-in fade-in slide-in-from-bottom-2 duration-200"
            >
              {/* Status icon */}
              <div className="flex-shrink-0 w-4">
                {item.status === 'loading' ? (
                  <RefreshCw
                    size={12}
                    className="text-primary animate-spin"
                  />
                ) : item.status === 'success' ? (
                  <Check size={12} className="text-green-500" />
                ) : (
                  <AlertCircle size={12} className="text-red-500" />
                )}
              </div>

              {/* Ticker */}
              <span className="w-[60px] truncate font-mono text-muted-foreground">
                {item.ticker || item.securityName.slice(0, 8)}
              </span>

              {/* Name */}
              <span className="flex-1 truncate">
                {item.securityName}
              </span>

              {/* Value / Status */}
              <span className="flex-shrink-0 text-right text-muted-foreground">
                {item.status === 'loading' ? (
                  'Laden...'
                ) : item.status === 'success' && item.price != null ? (
                  formatCurrency(item.price, item.currency || 'EUR')
                ) : item.status === 'error' ? (
                  <span className="text-red-500 truncate max-w-[100px] inline-block">
                    {item.error
                      ? item.error.length > 20
                        ? item.error.slice(0, 20) + '...'
                        : item.error
                      : 'Fehler'}
                  </span>
                ) : null}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Summary (when complete) */}
      {isComplete && (
        <div className="px-4 py-2.5 border-t border-border/30 text-xs text-muted-foreground">
          {successCount > 0 && (
            <span>
              {successCount} aktualisiert
            </span>
          )}
          {errorCount > 0 && (
            <span>
              {successCount > 0 ? ' · ' : ''}
              <span className="text-red-500">{errorCount} Fehler</span>
            </span>
          )}
          {successCount === 0 && errorCount === 0 && (
            <span>Keine Wertpapiere synchronisiert</span>
          )}
        </div>
      )}
    </div>
  );
}
