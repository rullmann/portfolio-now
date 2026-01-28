/**
 * Watchlist Widget - Shows watchlist with current prices
 */

import { useEffect, useState } from 'react';
import { Eye, RefreshCw, TrendingUp, TrendingDown } from 'lucide-react';
import { getWatchlists, getWatchlistSecurities } from '../../../lib/api';
import type { WatchlistData, WatchlistSecurityData } from '../../../lib/types';
import type { WidgetProps } from '../types';

interface WatchlistWidgetProps extends WidgetProps {
  currency?: string;
}

export function WatchlistWidget({ config }: WatchlistWidgetProps) {
  const watchlistId = config.settings?.watchlistId as number | undefined;
  const limit = (config.settings.limit as number) || 10;

  const [watchlists, setWatchlists] = useState<WatchlistData[]>([]);
  const [securities, setSecurities] = useState<WatchlistSecurityData[]>([]);
  const [selectedWatchlist, setSelectedWatchlist] = useState<number | null>(watchlistId ?? null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadWatchlists = async () => {
    try {
      const data = await getWatchlists();
      setWatchlists(data);

      // Select first watchlist if none specified
      if (!selectedWatchlist && data.length > 0) {
        setSelectedWatchlist(data[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    }
  };

  const loadSecurities = async () => {
    if (!selectedWatchlist) {
      setSecurities([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const data = await getWatchlistSecurities(selectedWatchlist);
      setSecurities(data.slice(0, limit));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadWatchlists();
  }, []);

  useEffect(() => {
    loadSecurities();
  }, [selectedWatchlist, limit]);

  const formatCurrency = (value: number, curr: string) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency: curr,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value);
  };

  const formatPercent = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'percent',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      signDisplay: 'exceptZero',
    }).format(value / 100);
  };

  if (loading && securities.length === 0) {
    return (
      <div className="h-full flex items-center justify-center">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col p-4">
        <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
          Watchlist
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center text-sm text-muted-foreground">
            <p>{error}</p>
            <button
              onClick={loadSecurities}
              className="mt-2 text-primary hover:underline"
            >
              Erneut versuchen
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (watchlists.length === 0) {
    return (
      <div className="h-full flex flex-col p-4">
        <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
          Watchlist
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <Eye className="h-8 w-8 text-muted-foreground/50 mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">
              Keine Watchlist vorhanden
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Erstellen Sie eine Watchlist, um Werte zu beobachten
            </p>
          </div>
        </div>
      </div>
    );
  }

  const currentWatchlist = watchlists.find((w) => w.id === selectedWatchlist);

  return (
    <div className="h-full flex flex-col p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="text-xs text-muted-foreground uppercase tracking-wide">
          Watchlist
        </div>
        {watchlists.length > 1 && (
          <select
            value={selectedWatchlist ?? ''}
            onChange={(e) => setSelectedWatchlist(Number(e.target.value))}
            className="text-xs bg-transparent border-none focus:outline-none cursor-pointer text-muted-foreground"
          >
            {watchlists.map((wl) => (
              <option key={wl.id} value={wl.id}>
                {wl.name}
              </option>
            ))}
          </select>
        )}
      </div>

      {currentWatchlist && watchlists.length === 1 && (
        <div className="text-xs text-muted-foreground mb-2">
          {currentWatchlist.name}
        </div>
      )}

      <div className="flex-1 overflow-auto space-y-1">
        {securities.length === 0 ? (
          <div className="text-center text-muted-foreground py-4 text-sm">
            Keine Wertpapiere in dieser Watchlist
          </div>
        ) : (
          securities.map((sec) => {
            const changePercent = sec.priceChangePercent ?? 0;
            const isPositive = changePercent >= 0;

            return (
              <div
                key={sec.securityId}
                className="flex items-center gap-2 py-1.5 px-2 rounded hover:bg-muted/30"
              >
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-medium truncate">
                    {sec.name}
                  </div>
                  <div className="text-[10px] text-muted-foreground">
                    {sec.ticker || sec.isin || '-'}
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-xs font-medium">
                    {sec.latestPrice
                      ? formatCurrency(sec.latestPrice, sec.currency)
                      : '-'}
                  </div>
                  {sec.priceChangePercent !== undefined && (
                    <div
                      className={`text-[10px] flex items-center justify-end gap-0.5 ${
                        isPositive ? 'text-green-600' : 'text-red-600'
                      }`}
                    >
                      {isPositive ? (
                        <TrendingUp className="h-2.5 w-2.5" />
                      ) : (
                        <TrendingDown className="h-2.5 w-2.5" />
                      )}
                      {formatPercent(changePercent)}
                    </div>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
