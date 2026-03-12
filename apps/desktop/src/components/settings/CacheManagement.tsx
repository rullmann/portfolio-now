import { useState, useEffect } from 'react';
import { Database, Trash2, RefreshCw, AlertTriangle } from 'lucide-react';
import { getCachedSecurities, deleteCachedDataForSecurity, deleteAllOrphanedCache, rebuildCacheMeta } from '../../lib/api';
import type { CachedSecurityInfo } from '../../lib/api';
import { toast } from '../../store';

export function CacheManagement() {
  const [securities, setSecurities] = useState<CachedSecurityInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deletingOrphans, setDeletingOrphans] = useState(false);

  const loadData = async () => {
    try {
      setLoading(true);
      const data = await getCachedSecurities();
      setSecurities(data);
    } catch (err) {
      toast.error(`Fehler beim Laden: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const orphanCount = securities.filter(s => !s.isInWatchlist && !s.isHeld).length;
  const totalDataPoints = securities.reduce((sum, s) => sum + s.totalDataPoints, 0);
  const selected = securities.find(s => s.securityId === selectedId);

  const handleDelete = async () => {
    if (!selectedId) return;
    setDeleting(true);
    try {
      const result = await deleteCachedDataForSecurity(selectedId);
      const total = result.pricesDeleted + result.earningsDeleted + result.dividendsDeleted + result.latestPricesDeleted;
      toast.success(`${total} Datenpunkte gelöscht`);
      setSelectedId(null);
      await loadData();
    } catch (err) {
      toast.error(`Fehler: ${err}`);
    } finally {
      setDeleting(false);
    }
  };

  const handleDeleteOrphans = async () => {
    setDeletingOrphans(true);
    try {
      const result = await deleteAllOrphanedCache();
      const total = result.pricesDeleted + result.earningsDeleted + result.dividendsDeleted + result.latestPricesDeleted;
      toast.success(`${total} verwaiste Datenpunkte gelöscht`);
      await loadData();
    } catch (err) {
      toast.error(`Fehler: ${err}`);
    } finally {
      setDeletingOrphans(false);
    }
  };

  const handleRebuild = async () => {
    try {
      const count = await rebuildCacheMeta();
      toast.success(`Cache-Index neu aufgebaut: ${count} Wertpapiere`);
      await loadData();
    } catch (err) {
      toast.error(`Fehler: ${err}`);
    }
  };

  return (
    <div className="bg-card rounded-lg border border-border p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Database size={20} className="text-primary" />
          <h3 className="text-lg font-semibold">Zwischengespeicherte Daten</h3>
        </div>
        <button
          onClick={handleRebuild}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs border border-border rounded-md hover:bg-muted transition-colors"
        >
          <RefreshCw size={14} />
          Index neu aufbauen
        </button>
      </div>
      <p className="text-sm text-muted-foreground mb-4">
        Kurse, Dividenden und Earnings werden dauerhaft gespeichert und bleiben auch nach Entfernung aus Watchlists erhalten.
        {securities.length > 0 && (
          <span className="ml-1">
            {securities.length} Wertpapiere, {totalDataPoints.toLocaleString('de-DE')} Datenpunkte gesamt.
          </span>
        )}
      </p>

      {loading ? (
        <div className="text-sm text-muted-foreground py-4">Lade...</div>
      ) : securities.length === 0 ? (
        <div className="text-sm text-muted-foreground py-4">Keine zwischengespeicherten Daten vorhanden.</div>
      ) : (
        <div className="space-y-4">
          {/* Security selector */}
          <div>
            <label className="text-sm font-medium mb-1.5 block">Wertpapier auswählen</label>
            <select
              value={selectedId ?? ''}
              onChange={(e) => setSelectedId(e.target.value ? Number(e.target.value) : null)}
              className="w-full px-3 py-2 text-sm bg-background border border-border rounded-md"
            >
              <option value="">-- Wertpapier wählen --</option>
              {securities.map(s => (
                <option key={s.securityId} value={s.securityId}>
                  {s.name}
                  {s.ticker ? ` (${s.ticker})` : ''}
                  {!s.isInWatchlist && !s.isHeld ? ' [verwaist]' : ''}
                  {` — ${s.totalDataPoints} Datenpunkte`}
                </option>
              ))}
            </select>
          </div>

          {/* Selected security details */}
          {selected && (
            <div className="bg-muted/50 rounded-md p-4 text-sm space-y-2">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Kurse</span>
                <span>{selected.priceCount.toLocaleString('de-DE')}{selected.priceDateMin && selected.priceDateMax ? ` (${selected.priceDateMin} – ${selected.priceDateMax})` : ''}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Dividenden</span>
                <span>{selected.dividendCount}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Earnings</span>
                <span>{selected.earningsCount}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Status</span>
                <span>
                  {selected.isInWatchlist && 'In Watchlist'}
                  {selected.isHeld && (selected.isInWatchlist ? ' + Im Depot' : 'Im Depot')}
                  {!selected.isInWatchlist && !selected.isHeld && (
                    <span className="text-amber-500">Verwaist</span>
                  )}
                </span>
              </div>
              <div className="pt-2 border-t border-border space-y-2">
                {(selected.isInWatchlist || selected.isHeld) && (
                  <p className="text-xs text-amber-500">
                    Dieses Wertpapier ist {selected.isHeld ? 'im Depot' : 'in einer Watchlist'}. Daten werden beim nächsten Sync erneut geladen.
                  </p>
                )}
                <button
                  onClick={handleDelete}
                  disabled={deleting}
                  className="flex items-center gap-2 px-4 py-2 text-sm bg-destructive text-destructive-foreground rounded-md hover:bg-destructive/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <Trash2 size={14} />
                  {deleting ? 'Lösche...' : 'Daten für dieses Wertpapier löschen'}
                </button>
              </div>
            </div>
          )}

          {/* Bulk delete orphans */}
          {orphanCount > 0 && (
            <div className="flex items-start gap-3 bg-amber-500/5 border border-amber-500/30 rounded-md p-4">
              <AlertTriangle size={18} className="text-amber-500 mt-0.5 shrink-0" />
              <div className="flex-1">
                <p className="text-sm">
                  <strong>{orphanCount}</strong> Wertpapier{orphanCount !== 1 ? 'e' : ''} mit gespeicherten Daten {orphanCount !== 1 ? 'sind' : 'ist'} nicht mehr in einer Watchlist oder einem Depot.
                </p>
                <button
                  onClick={handleDeleteOrphans}
                  disabled={deletingOrphans}
                  className="mt-2 flex items-center gap-2 px-3 py-1.5 text-sm border border-amber-500/50 text-amber-600 dark:text-amber-400 rounded-md hover:bg-amber-500/10 transition-colors disabled:opacity-50"
                >
                  <Trash2 size={14} />
                  {deletingOrphans ? 'Lösche...' : 'Alle verwaisten Daten löschen'}
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
