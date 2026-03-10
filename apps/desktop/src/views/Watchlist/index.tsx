/**
 * Watchlist view for tracking securities of interest.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Eye, Plus, Trash2, RefreshCw, TrendingUp, TrendingDown, Newspaper, History, Search } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { getWatchlists, getWatchlistSecurities, createWatchlist, deleteWatchlist, removeFromWatchlist, getPriceHistory } from '../../lib/api';
import { TradingViewMiniChart } from '../../components/charts';
import { SecurityLogo, RegimeBadge, SetupScoreBadge } from '../../components/common';
import { SecuritySearchModal } from '../../components/modals';
import { NewsResearchModal } from '../../components/modals/NewsResearchModal';
import { HistoricalQuotesModal } from '../../components/modals/HistoricalQuotesModal';
import { useCachedLogos } from '../../lib/hooks';
import { useSettingsStore } from '../../store';
import { convertToOHLC } from '../../lib/indicators';
import type { RegimeAnalysis, SetupScore } from '../../lib/indicators';
import { detectRegime, scoreSetup } from '../../lib/indicators-rust';
import type { WatchlistData, WatchlistSecurityData, PriceData } from '../../lib/types';

export function WatchlistView() {
  const [watchlists, setWatchlists] = useState<WatchlistData[]>([]);
  const [selectedWatchlist, setSelectedWatchlist] = useState<number | null>(null);
  const [securities, setSecurities] = useState<WatchlistSecurityData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [newWatchlistName, setNewWatchlistName] = useState('');
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [priceHistories, setPriceHistories] = useState<Record<number, PriceData[]>>({});
  const [tradingData, setTradingData] = useState<Record<number, { regime: RegimeAnalysis; setup: SetupScore }>>({});
  const { brandfetchApiKey, aiEnabled } = useSettingsStore();
  const [newsModalSecurity, setNewsModalSecurity] = useState<WatchlistSecurityData | null>(null);
  const [showHistoricalModal, setShowHistoricalModal] = useState(false);
  const [showSearchModal, setShowSearchModal] = useState(false);
  const [pendingHistorySecurityIds, setPendingHistorySecurityIds] = useState<number[]>([]);

  // Prepare securities for logo loading
  const securitiesForLogos = useMemo(() =>
    securities.map((s) => ({
      id: s.securityId,
      ticker: s.ticker || undefined,
      name: s.name,
    })),
    [securities]
  );

  // Load logos
  const { logos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

  const loadWatchlists = async () => {
    try {
      setIsLoading(true);
      const data = await getWatchlists();
      setWatchlists(data);
      if (data.length > 0 && !selectedWatchlist) {
        setSelectedWatchlist(data[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const loadSecurities = async (watchlistId: number) => {
    try {
      const data = await getWatchlistSecurities(watchlistId);
      setSecurities(data);
    } catch (err) {
      console.error('Error loading watchlist securities:', err);
    }
  };

  // Load 6-month price history for all securities (needed for regime detection)
  const loadPriceHistories = useCallback(async () => {
    if (securities.length === 0) return;

    const sixMonthsAgo = new Date();
    sixMonthsAgo.setMonth(sixMonthsAgo.getMonth() - 6);
    const from = sixMonthsAgo.toISOString().split('T')[0];
    const to = new Date().toISOString().split('T')[0];

    const histories: Record<number, PriceData[]> = {};
    const trading: Record<number, { regime: RegimeAnalysis; setup: SetupScore }> = {};

    await Promise.all(
      securities.map(async (security) => {
        try {
          const prices = await getPriceHistory(security.securityId, from, to);
          histories[security.securityId] = prices;

          // Compute regime + setup if enough data
          if (prices.length >= 50) {
            const ohlc = convertToOHLC(prices, 1.5);
            const [regime, setup] = await Promise.all([
              detectRegime(ohlc),
              scoreSetup(ohlc),
            ]);
            trading[security.securityId] = { regime, setup };
          }
        } catch {
          // Ignore errors for individual securities
        }
      })
    );

    setPriceHistories(histories);
    setTradingData(trading);
  }, [securities]);

  useEffect(() => {
    loadWatchlists();
  }, []);

  // Listen for watchlist updates from ChatBot
  useEffect(() => {
    const unlisten = listen('watchlist-updated', () => {
      // Reload watchlists when updated via ChatBot
      loadWatchlists();
      if (selectedWatchlist) {
        loadSecurities(selectedWatchlist);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [selectedWatchlist]);

  // Load price histories when securities change
  useEffect(() => {
    loadPriceHistories();
  }, [loadPriceHistories]);

  useEffect(() => {
    if (selectedWatchlist) {
      loadSecurities(selectedWatchlist);
    }
  }, [selectedWatchlist]);

  const handleCreateWatchlist = async () => {
    if (!newWatchlistName.trim()) return;
    try {
      await createWatchlist(newWatchlistName);
      setNewWatchlistName('');
      setShowCreateForm(false);
      await loadWatchlists();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleDeleteWatchlist = async (id: number) => {
    if (!confirm('Watchlist wirklich löschen?')) return;
    try {
      await deleteWatchlist(id);
      if (selectedWatchlist === id) {
        setSelectedWatchlist(null);
        setSecurities([]);
      }
      await loadWatchlists();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleRemoveSecurity = async (securityId: number) => {
    if (!selectedWatchlist) return;
    try {
      await removeFromWatchlist(selectedWatchlist, securityId);
      await loadSecurities(selectedWatchlist);
      await loadWatchlists(); // Update count in sidebar
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const formatPrice = (price: number | undefined | null, currency: string) => {
    if (price == null) return '-';  // Prüft auf null UND undefined
    return `${price.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Eye className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Watchlist</h1>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowSearchModal(true)}
            disabled={!selectedWatchlist}
            className="flex items-center gap-2 px-4 py-1.5 text-sm font-medium bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors shadow-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Search size={16} />
            Wertpapier suchen
          </button>
          <button
            onClick={() => setShowHistoricalModal(true)}
            disabled={!selectedWatchlist || securities.length === 0}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            title="Komplette Kurshistorie für Watchlist-Titel laden"
          >
            <History size={16} />
            Historie laden
          </button>
          <button
            onClick={loadWatchlists}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
        </div>
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Watchlist sidebar */}
        <div className="lg:col-span-1 space-y-4">
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="flex items-center justify-between mb-4">
              <h2 className="font-semibold">Meine Watchlists</h2>
              <button
                onClick={() => setShowCreateForm(true)}
                className="p-1 hover:bg-muted rounded-md transition-colors"
                title="Neue Watchlist"
              >
                <Plus size={18} />
              </button>
            </div>

            {showCreateForm && (
              <div className="mb-4 p-3 bg-muted rounded-md">
                <input
                  type="text"
                  value={newWatchlistName}
                  onChange={(e) => setNewWatchlistName(e.target.value)}
                  placeholder="Name der Watchlist"
                  className="w-full px-2 py-1 text-sm border border-border rounded bg-background"
                  onKeyDown={(e) => e.key === 'Enter' && handleCreateWatchlist()}
                />
                <div className="flex gap-2 mt-2">
                  <button
                    onClick={handleCreateWatchlist}
                    className="flex-1 px-2 py-1 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90"
                  >
                    Erstellen
                  </button>
                  <button
                    onClick={() => {
                      setShowCreateForm(false);
                      setNewWatchlistName('');
                    }}
                    className="flex-1 px-2 py-1 text-xs border border-border rounded hover:bg-muted"
                  >
                    Abbrechen
                  </button>
                </div>
              </div>
            )}

            <div className="space-y-1">
              {watchlists.map((wl) => (
                <div
                  key={wl.id}
                  className={`flex items-center justify-between p-2 rounded-md cursor-pointer transition-colors ${
                    selectedWatchlist === wl.id
                      ? 'bg-primary text-primary-foreground'
                      : 'hover:bg-muted'
                  }`}
                  onClick={() => setSelectedWatchlist(wl.id)}
                >
                  <div>
                    <div className="font-medium text-sm">{wl.name}</div>
                    <div className={`text-xs ${selectedWatchlist === wl.id ? 'text-primary-foreground/70' : 'text-muted-foreground'}`}>
                      {wl.securitiesCount} Wertpapiere
                    </div>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteWatchlist(wl.id);
                    }}
                    className={`p-1 rounded hover:bg-destructive/20 ${
                      selectedWatchlist === wl.id ? 'text-primary-foreground' : 'text-muted-foreground'
                    }`}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}

              {watchlists.length === 0 && !isLoading && (
                <div className="text-sm text-muted-foreground text-center py-4">
                  Keine Watchlists vorhanden
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Securities list */}
        <div className="lg:col-span-3">
          <div className="bg-card rounded-lg border border-border">
            {selectedWatchlist ? (
              securities.length > 0 ? (
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-border bg-muted/50">
                        <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
                        <th className="text-left py-3 px-4 font-medium">ISIN</th>
                        <th className="text-center py-3 px-4 font-medium w-32">6M</th>
                        <th className="text-right py-3 px-4 font-medium">Kurs</th>
                        <th className="text-right py-3 px-4 font-medium">Änderung</th>
                        <th className="text-center py-3 px-4 font-medium">Regime</th>
                        <th className="text-center py-3 px-4 font-medium">Score</th>
                        <th className="text-right py-3 px-4 font-medium">Aktionen</th>
                      </tr>
                    </thead>
                    <tbody>
                      {securities.map((security) => {
                        const changePercent = security.priceChangePercent;
                        const isPositive = changePercent != null && changePercent >= 0;
                        return (
                          <tr key={security.securityId} className="border-b border-border last:border-0 hover:bg-muted/30">
                            <td className="py-3 px-4">
                              <div className="flex items-center gap-3">
                                <SecurityLogo securityId={security.securityId} logos={logos} size={32} />
                                <div>
                                  <div className="font-medium">{security.name}</div>
                                  {security.ticker && (
                                    <div className="text-xs text-muted-foreground">{security.ticker}</div>
                                  )}
                                </div>
                              </div>
                            </td>
                            <td className="py-3 px-4 font-mono text-muted-foreground">
                              {security.isin || '-'}
                            </td>
                            <td className="py-3 px-4">
                              {priceHistories[security.securityId] && priceHistories[security.securityId].length > 0 ? (
                                <TradingViewMiniChart
                                  data={priceHistories[security.securityId].map(p => ({ date: p.date, value: p.value }))}
                                  width={100}
                                  height={32}
                                />
                              ) : (
                                <div className="w-[100px] h-[32px] flex items-center justify-center text-muted-foreground text-xs">-</div>
                              )}
                            </td>
                            <td className="py-3 px-4 text-right font-medium">
                              {formatPrice(security.latestPrice, security.currency)}
                            </td>
                            <td className="py-3 px-4 text-right">
                              {changePercent != null ? (
                                <div className={`flex items-center justify-end gap-1 ${isPositive ? 'text-green-600' : 'text-red-600'}`}>
                                  {isPositive ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
                                  <span>{isPositive ? '+' : ''}{changePercent.toFixed(2)}%</span>
                                </div>
                              ) : (
                                <span className="text-muted-foreground">-</span>
                              )}
                            </td>
                            <td className="py-3 px-4 text-center">
                              {tradingData[security.securityId] ? (
                                <RegimeBadge regime={tradingData[security.securityId].regime.regime} />
                              ) : (
                                <span className="text-xs text-muted-foreground">-</span>
                              )}
                            </td>
                            <td className="py-3 px-4 text-center">
                              {tradingData[security.securityId] ? (
                                <SetupScoreBadge score={tradingData[security.securityId].setup.totalScore} />
                              ) : (
                                <span className="text-xs text-muted-foreground">-</span>
                              )}
                            </td>
                            <td className="py-3 px-4 text-right">
                              <div className="flex justify-end gap-1">
                                {aiEnabled && (
                                  <button
                                    onClick={() => setNewsModalSecurity(security)}
                                    className="p-1 hover:bg-muted rounded-md transition-colors"
                                    title="Nachrichten recherchieren"
                                  >
                                    <Newspaper size={14} className="text-muted-foreground" />
                                  </button>
                                )}
                                <button
                                  onClick={() => handleRemoveSecurity(security.securityId)}
                                  className="p-1 hover:bg-destructive/10 rounded-md text-destructive"
                                  title="Entfernen"
                                >
                                  <Trash2 size={14} />
                                </button>
                              </div>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="p-10 text-center">
                  <div className="w-14 h-14 rounded-xl bg-primary/10 flex items-center justify-center mx-auto mb-4">
                    <Search size={24} className="text-primary" />
                  </div>
                  <p className="font-medium text-foreground mb-1">Watchlist ist leer</p>
                  <p className="text-sm text-muted-foreground mb-5">Suche nach Aktien, ETFs oder Krypto und füge sie zur Watchlist hinzu.</p>
                  <button
                    onClick={() => setShowSearchModal(true)}
                    className="inline-flex items-center gap-2 px-5 py-2.5 text-sm font-medium bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors shadow-sm"
                  >
                    <Plus size={16} />
                    Wertpapier suchen
                  </button>
                </div>
              )
            ) : (
              <div className="p-10 text-center">
                <div className="w-14 h-14 rounded-xl bg-muted flex items-center justify-center mx-auto mb-4">
                  <Eye size={24} className="text-muted-foreground" />
                </div>
                <p className="font-medium text-foreground mb-1">Keine Watchlist ausgewählt</p>
                <p className="text-sm text-muted-foreground">Wähle links eine Watchlist aus oder erstelle eine neue.</p>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Security Search Modal */}
      <SecuritySearchModal
        isOpen={showSearchModal}
        onClose={() => {
          setShowSearchModal(false);
          // Auto-open historical quotes for newly added securities
          if (pendingHistorySecurityIds.length > 0) {
            setShowHistoricalModal(true);
          }
        }}
        defaultWatchlistId={selectedWatchlist ?? undefined}
        onSecurityAdded={(securityId) => {
          if (selectedWatchlist) {
            loadSecurities(selectedWatchlist);
          }
          loadWatchlists();
          // Auto-open historical quotes dialog for the new security
          setPendingHistorySecurityIds(prev => [...prev, securityId]);
        }}
      />

      {/* Historical Quotes Modal */}
      <HistoricalQuotesModal
        isOpen={showHistoricalModal}
        onClose={() => {
          setShowHistoricalModal(false);
          setPendingHistorySecurityIds([]);
        }}
        onComplete={loadPriceHistories}
        securityIds={pendingHistorySecurityIds.length > 0 ? pendingHistorySecurityIds : securities.map((s) => s.securityId)}
      />

      {/* News Research Modal */}
      {newsModalSecurity && (
        <NewsResearchModal
          isOpen={!!newsModalSecurity}
          onClose={() => setNewsModalSecurity(null)}
          security={{
            id: newsModalSecurity.securityId,
            name: newsModalSecurity.name,
            ticker: newsModalSecurity.ticker,
            isin: newsModalSecurity.isin,
            currency: newsModalSecurity.currency,
          }}
          currentPrice={newsModalSecurity.latestPrice ?? undefined}
        />
      )}
    </div>
  );
}
