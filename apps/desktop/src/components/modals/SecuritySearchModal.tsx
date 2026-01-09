/**
 * Modal for searching external securities (Portfolio Report, Alpha Vantage)
 * and adding them to a watchlist.
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { X, Search, Loader2, Plus, Star, AlertCircle, Check } from 'lucide-react';
import { useSettingsStore } from '../../store';
import { toast } from '../../store';
import {
  searchExternalSecurities,
  addExternalSecurityToWatchlist,
  getWatchlists,
  createWatchlist,
} from '../../lib/api';
import type {
  ExternalSecuritySearchResult,
  WatchlistData,
} from '../../lib/types';

interface SecuritySearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSecurityAdded?: (securityId: number) => void;
  defaultWatchlistId?: number;
}

export function SecuritySearchModal({
  isOpen,
  onClose,
  onSecurityAdded,
  defaultWatchlistId,
}: SecuritySearchModalProps) {
  const { alphaVantageApiKey } = useSettingsStore();

  // Search state
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<ExternalSecuritySearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchErrors, setSearchErrors] = useState<string[]>([]);
  const [providersUsed, setProvidersUsed] = useState<string[]>([]);

  // Watchlist state
  const [watchlists, setWatchlists] = useState<WatchlistData[]>([]);
  const [selectedWatchlistId, setSelectedWatchlistId] = useState<number | null>(
    defaultWatchlistId || null
  );
  const [isCreatingWatchlist, setIsCreatingWatchlist] = useState(false);
  const [newWatchlistName, setNewWatchlistName] = useState('');

  // Adding state
  const [addingSymbol, setAddingSymbol] = useState<string | null>(null);
  const [addedSymbols, setAddedSymbols] = useState<Set<string>>(new Set());

  // Debounce timer
  const debounceRef = useRef<NodeJS.Timeout>();
  const inputRef = useRef<HTMLInputElement>(null);

  // Load watchlists on mount
  useEffect(() => {
    if (isOpen) {
      loadWatchlists();
      // Focus input when modal opens
      setTimeout(() => inputRef.current?.focus(), 100);
      // Reset state
      setQuery('');
      setResults([]);
      setAddedSymbols(new Set());
    }
  }, [isOpen]);

  const loadWatchlists = async () => {
    try {
      const lists = await getWatchlists();
      setWatchlists(lists);
      if (lists.length > 0 && !selectedWatchlistId) {
        setSelectedWatchlistId(lists[0].id);
      }
    } catch (err) {
      console.error('Failed to load watchlists:', err);
    }
  };

  // Debounced search
  const performSearch = useCallback(
    async (searchQuery: string) => {
      if (searchQuery.trim().length < 2) {
        setResults([]);
        setSearchErrors([]);
        setProvidersUsed([]);
        return;
      }

      setIsSearching(true);
      try {
        const response = await searchExternalSecurities(
          searchQuery,
          alphaVantageApiKey || undefined
        );
        setResults(response.results);
        setSearchErrors(response.errors);
        setProvidersUsed(response.providersUsed);
      } catch (err) {
        console.error('Search failed:', err);
        setSearchErrors([String(err)]);
        setResults([]);
      } finally {
        setIsSearching(false);
      }
    },
    [alphaVantageApiKey]
  );

  // Handle input change with debounce
  const handleQueryChange = (value: string) => {
    setQuery(value);

    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    debounceRef.current = setTimeout(() => {
      performSearch(value);
    }, 300); // 300ms debounce
  };

  // Handle adding security to watchlist
  const handleAddToWatchlist = async (result: ExternalSecuritySearchResult) => {
    if (!selectedWatchlistId) {
      toast.warning('Bitte wähle eine Watchlist aus');
      return;
    }

    setAddingSymbol(result.symbol);
    try {
      const securityId = await addExternalSecurityToWatchlist(
        selectedWatchlistId,
        result
      );

      setAddedSymbols(prev => new Set([...prev, result.symbol]));
      toast.success(`${result.name} zur Watchlist hinzugefügt`);
      onSecurityAdded?.(securityId);
    } catch (err) {
      console.error('Failed to add security:', err);
      toast.error(`Fehler: ${err}`);
    } finally {
      setAddingSymbol(null);
    }
  };

  // Create new watchlist
  const handleCreateWatchlist = async () => {
    if (!newWatchlistName.trim()) return;

    try {
      const newList = await createWatchlist(newWatchlistName.trim());
      setWatchlists(prev => [...prev, newList]);
      setSelectedWatchlistId(newList.id);
      setNewWatchlistName('');
      setIsCreatingWatchlist(false);
      toast.success(`Watchlist "${newList.name}" erstellt`);
    } catch (err) {
      toast.error(`Fehler: ${err}`);
    }
  };

  // Handle ESC key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-2xl max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div>
            <h2 className="text-lg font-semibold">Wertpapier suchen</h2>
            <p className="text-xs text-muted-foreground">
              Suche nach Aktien, ETFs und Fonds zum Hinzufügen zur Watchlist
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 hover:bg-muted rounded-lg transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Search Input */}
        <div className="p-4 border-b border-border">
          <div className="relative">
            <Search
              size={16}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
            />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={e => handleQueryChange(e.target.value)}
              placeholder="Name, ISIN, WKN oder Symbol eingeben..."
              className="w-full pl-10 pr-4 py-2.5 bg-muted border-none rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
            />
            {isSearching && (
              <Loader2
                size={16}
                className="absolute right-3 top-1/2 -translate-y-1/2 animate-spin text-muted-foreground"
              />
            )}
          </div>

          {/* Provider badges */}
          {providersUsed.length > 0 && (
            <div className="flex gap-2 mt-2">
              <span className="text-xs text-muted-foreground">Quellen:</span>
              {providersUsed.map(provider => (
                <span
                  key={provider}
                  className="text-xs px-2 py-0.5 bg-primary/10 text-primary rounded"
                >
                  {provider === 'YAHOO'
                    ? 'Yahoo Finance'
                    : provider === 'ALPHAVANTAGE'
                      ? 'Alpha Vantage'
                      : provider}
                </span>
              ))}
            </div>
          )}

          {/* Errors */}
          {searchErrors.length > 0 && (
            <div className="mt-2 flex items-start gap-2 text-xs text-amber-600">
              <AlertCircle size={14} className="mt-0.5 flex-shrink-0" />
              <div>{searchErrors.join(', ')}</div>
            </div>
          )}
        </div>

        {/* Watchlist Selector */}
        <div className="px-4 py-2 border-b border-border flex items-center gap-2">
          <Star size={14} className="text-muted-foreground" />
          <span className="text-sm text-muted-foreground">Zur Watchlist:</span>

          {isCreatingWatchlist || watchlists.length === 0 ? (
            <div className="flex items-center gap-2 flex-1">
              <input
                type="text"
                value={newWatchlistName}
                onChange={e => setNewWatchlistName(e.target.value)}
                placeholder={watchlists.length === 0 ? "Neue Watchlist erstellen..." : "Name der neuen Watchlist"}
                className="flex-1 px-2 py-1 text-sm bg-muted border-none rounded focus:outline-none focus:ring-1 focus:ring-primary"
                autoFocus
                onKeyDown={e => {
                  if (e.key === 'Enter') handleCreateWatchlist();
                  if (e.key === 'Escape' && watchlists.length > 0) setIsCreatingWatchlist(false);
                }}
              />
              <button
                onClick={handleCreateWatchlist}
                className="px-2 py-1 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90"
              >
                Erstellen
              </button>
              {watchlists.length > 0 && (
                <button
                  onClick={() => setIsCreatingWatchlist(false)}
                  className="px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
                >
                  Abbrechen
                </button>
              )}
            </div>
          ) : (
            <>
              <select
                value={selectedWatchlistId || ''}
                onChange={e => setSelectedWatchlistId(Number(e.target.value))}
                className="flex-1 px-2 py-1 text-sm bg-muted border-none rounded focus:outline-none focus:ring-1 focus:ring-primary"
              >
                {watchlists.map(wl => (
                  <option key={wl.id} value={wl.id}>
                    {wl.name} ({wl.securitiesCount})
                  </option>
                ))}
              </select>
              <button
                onClick={() => setIsCreatingWatchlist(true)}
                className="px-2 py-1 text-xs text-muted-foreground hover:text-primary flex items-center gap-1"
              >
                <Plus size={12} />
                Neue
              </button>
            </>
          )}
        </div>

        {/* Results */}
        <div className="flex-1 overflow-auto">
          {results.length === 0 && query.length >= 2 && !isSearching ? (
            <div className="p-8 text-center text-muted-foreground">
              <Search size={32} className="mx-auto mb-2 opacity-50" />
              <p>Keine Ergebnisse gefunden</p>
              <p className="text-xs mt-1">Versuche einen anderen Suchbegriff</p>
            </div>
          ) : results.length === 0 && query.length < 2 ? (
            <div className="p-8 text-center text-muted-foreground">
              <p>Gib mindestens 2 Zeichen ein, um zu suchen</p>
            </div>
          ) : (
            <div className="divide-y divide-border">
              {results.map(result => (
                <div
                  key={`${result.provider}-${result.symbol}`}
                  className="p-3 hover:bg-muted/50 flex items-center justify-between"
                >
                  <div className="flex-1 min-w-0">
                    <div className="font-medium truncate">{result.name}</div>
                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <span className="font-mono">{result.symbol}</span>
                      {result.isin && (
                        <>
                          <span>|</span>
                          <span className="font-mono">{result.isin}</span>
                        </>
                      )}
                      {result.currency && (
                        <>
                          <span>|</span>
                          <span>{result.currency}</span>
                        </>
                      )}
                      {result.securityType && (
                        <span className="px-1.5 py-0.5 bg-muted rounded text-[10px]">
                          {result.securityType}
                        </span>
                      )}
                    </div>
                  </div>

                  <button
                    onClick={() => handleAddToWatchlist(result)}
                    disabled={
                      addingSymbol === result.symbol ||
                      addedSymbols.has(result.symbol) ||
                      !selectedWatchlistId
                    }
                    className={`ml-4 px-3 py-1.5 text-sm rounded-lg flex items-center gap-1.5 transition-colors ${
                      addedSymbols.has(result.symbol)
                        ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                        : 'bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50'
                    }`}
                  >
                    {addingSymbol === result.symbol ? (
                      <Loader2 size={14} className="animate-spin" />
                    ) : addedSymbols.has(result.symbol) ? (
                      <>
                        <Check size={14} />
                        Hinzugefügt
                      </>
                    ) : (
                      <>
                        <Plus size={14} />
                        Hinzufügen
                      </>
                    )}
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-3 border-t border-border text-xs text-muted-foreground text-center">
          {!alphaVantageApiKey && (
            <span>
              Tipp: Mit Alpha Vantage API Key (Einstellungen) erhältst du mehr
              Suchergebnisse
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

export default SecuritySearchModal;
