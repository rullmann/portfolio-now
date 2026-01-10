/**
 * Modal for adding a benchmark for portfolio comparison.
 */

import { useState, useEffect, useCallback } from 'react';
import { X, Search, Loader2, Globe, Database } from 'lucide-react';
import type { SecurityData } from '../../lib/types';
import { getSecurities, addBenchmark, searchExternalSecurities, createSecurity } from '../../lib/api';

interface ExternalSecurityResult {
  symbol: string;
  name: string;
  exchange?: string;
  type?: string;
  isin?: string;
  provider: string;
}

interface BenchmarkFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export function BenchmarkFormModal({
  isOpen,
  onClose,
  onSuccess,
}: BenchmarkFormModalProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [searchMode, setSearchMode] = useState<'internal' | 'external'>('internal');
  const [startDate, setStartDate] = useState(
    new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]
  );

  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [externalResults, setExternalResults] = useState<ExternalSecurityResult[]>([]);
  const [selectedSecurity, setSelectedSecurity] = useState<SecurityData | null>(null);
  const [selectedExternal, setSelectedExternal] = useState<ExternalSecurityResult | null>(null);

  const [isLoadingInternal, setIsLoadingInternal] = useState(false);
  const [isSearchingExternal, setIsSearchingExternal] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load internal securities when modal opens
  useEffect(() => {
    if (isOpen) {
      setIsLoadingInternal(true);
      getSecurities()
        .then(setSecurities)
        .catch((err) => console.error('Failed to load securities:', err))
        .finally(() => setIsLoadingInternal(false));
    }
  }, [isOpen]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setSearchQuery('');
      setSearchMode('internal');
      setStartDate(
        new Date(Date.now() - 365 * 24 * 60 * 60 * 1000).toISOString().split('T')[0]
      );
      setSelectedSecurity(null);
      setSelectedExternal(null);
      setExternalResults([]);
      setError(null);
    }
  }, [isOpen]);

  // Debounced external search
  const searchExternal = useCallback(async (query: string) => {
    if (query.length < 2) {
      setExternalResults([]);
      return;
    }

    setIsSearchingExternal(true);
    try {
      const results = await searchExternalSecurities(query);
      setExternalResults(results.results || []);
    } catch (err) {
      console.error('External search failed:', err);
      setExternalResults([]);
    } finally {
      setIsSearchingExternal(false);
    }
  }, []);

  // Trigger external search with debounce
  useEffect(() => {
    if (searchMode !== 'external' || !searchQuery) return;

    const timer = setTimeout(() => {
      searchExternal(searchQuery);
    }, 300);

    return () => clearTimeout(timer);
  }, [searchQuery, searchMode, searchExternal]);

  // Filter internal securities
  const filteredSecurities = securities.filter(
    (s) =>
      !s.isRetired &&
      (s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.isin?.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.ticker?.toLowerCase().includes(searchQuery.toLowerCase()))
  );

  const handleSelectInternal = (security: SecurityData) => {
    setSelectedSecurity(security);
    setSelectedExternal(null);
    setSearchQuery(security.name);
  };

  const handleSelectExternal = (result: ExternalSecurityResult) => {
    setSelectedExternal(result);
    setSelectedSecurity(null);
    setSearchQuery(result.name);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      let securityId: number;

      if (selectedSecurity) {
        // Use existing internal security
        securityId = selectedSecurity.id;
      } else if (selectedExternal) {
        // Create new security from external search result
        const result = await createSecurity({
          name: selectedExternal.name,
          isin: selectedExternal.isin || undefined,
          ticker: selectedExternal.symbol,
          currency: 'EUR', // Default, will be updated by quote sync
          feed: 'YAHOO',
        });
        securityId = result.id;
      } else {
        throw new Error('Bitte ein Wertpapier auswählen');
      }

      await addBenchmark(securityId, startDate);
      onSuccess();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border sticky top-0 bg-card">
          <h2 className="text-lg font-semibold">Benchmark hinzufügen</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          {error && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
              {error}
            </div>
          )}

          {/* Search Mode Toggle */}
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => {
                setSearchMode('internal');
                setSelectedExternal(null);
                setExternalResults([]);
              }}
              className={`flex items-center gap-2 px-3 py-1.5 text-sm rounded-md transition-colors ${
                searchMode === 'internal'
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted hover:bg-muted/80'
              }`}
            >
              <Database size={14} />
              Vorhandene
            </button>
            <button
              type="button"
              onClick={() => {
                setSearchMode('external');
                setSelectedSecurity(null);
              }}
              className={`flex items-center gap-2 px-3 py-1.5 text-sm rounded-md transition-colors ${
                searchMode === 'external'
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted hover:bg-muted/80'
              }`}
            >
              <Globe size={14} />
              Yahoo Finance
            </button>
          </div>

          {/* Search Input */}
          <div className="relative">
            <label className="block text-sm font-medium mb-1">
              Benchmark suchen <span className="text-destructive">*</span>
            </label>
            <div className="relative">
              <Search
                size={16}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
              />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => {
                  setSearchQuery(e.target.value);
                  setSelectedSecurity(null);
                  setSelectedExternal(null);
                }}
                disabled={isLoadingInternal}
                className="w-full pl-9 pr-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
                placeholder={
                  searchMode === 'internal'
                    ? 'z.B. MSCI World, S&P 500...'
                    : 'Nach Symbol oder Namen suchen...'
                }
              />
              {isSearchingExternal && (
                <Loader2 className="absolute right-3 top-1/2 -translate-y-1/2 animate-spin text-muted-foreground" size={16} />
              )}
            </div>
          </div>

          {/* Search Results */}
          {searchQuery && !selectedSecurity && !selectedExternal && (
            <div className="max-h-48 overflow-y-auto border border-border rounded-md">
              {searchMode === 'internal' ? (
                filteredSecurities.length > 0 ? (
                  filteredSecurities.slice(0, 10).map((s) => (
                    <button
                      key={s.id}
                      type="button"
                      onClick={() => handleSelectInternal(s)}
                      className="w-full px-3 py-2 text-left hover:bg-muted transition-colors border-b border-border last:border-0"
                    >
                      <div className="font-medium text-sm">{s.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {[s.isin, s.ticker].filter(Boolean).join(' · ')}
                      </div>
                    </button>
                  ))
                ) : (
                  <div className="px-3 py-4 text-sm text-muted-foreground text-center">
                    Keine Ergebnisse. Versuchen Sie die Yahoo-Suche.
                  </div>
                )
              ) : externalResults.length > 0 ? (
                externalResults.slice(0, 10).map((r, i) => (
                  <button
                    key={`${r.symbol}-${i}`}
                    type="button"
                    onClick={() => handleSelectExternal(r)}
                    className="w-full px-3 py-2 text-left hover:bg-muted transition-colors border-b border-border last:border-0"
                  >
                    <div className="font-medium text-sm">{r.name}</div>
                    <div className="text-xs text-muted-foreground flex items-center gap-2">
                      <span>{r.symbol}</span>
                      {r.exchange && <span>· {r.exchange}</span>}
                      <span className="ml-auto px-1.5 py-0.5 bg-muted rounded text-xs">
                        {r.provider}
                      </span>
                    </div>
                  </button>
                ))
              ) : isSearchingExternal ? (
                <div className="px-3 py-4 text-sm text-muted-foreground text-center">
                  Suche läuft...
                </div>
              ) : (
                <div className="px-3 py-4 text-sm text-muted-foreground text-center">
                  Keine Ergebnisse
                </div>
              )}
            </div>
          )}

          {/* Selected Security Display */}
          {(selectedSecurity || selectedExternal) && (
            <div className="p-3 bg-muted rounded-md">
              <div className="flex items-center justify-between">
                <div>
                  <div className="font-medium text-sm">
                    {selectedSecurity?.name || selectedExternal?.name}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {selectedSecurity
                      ? [selectedSecurity.isin, selectedSecurity.ticker]
                          .filter(Boolean)
                          .join(' · ')
                      : [selectedExternal?.symbol, selectedExternal?.exchange]
                          .filter(Boolean)
                          .join(' · ')}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => {
                    setSelectedSecurity(null);
                    setSelectedExternal(null);
                    setSearchQuery('');
                  }}
                  className="p-1 hover:bg-background rounded-md transition-colors"
                >
                  <X size={16} />
                </button>
              </div>
            </div>
          )}

          {/* Start Date */}
          <div>
            <label className="block text-sm font-medium mb-1">
              Vergleich ab Datum
            </label>
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
            <p className="text-xs text-muted-foreground mt-1">
              Ab diesem Datum wird der Benchmark mit Ihrem Portfolio verglichen.
            </p>
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4 border-t border-border">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors"
            >
              Abbrechen
            </button>
            <button
              type="submit"
              disabled={isSubmitting || (!selectedSecurity && !selectedExternal)}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Hinzufügen...' : 'Hinzufügen'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
