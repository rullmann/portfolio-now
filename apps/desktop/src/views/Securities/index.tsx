/**
 * Securities view for displaying and managing securities.
 */

import { useState, useEffect, useCallback, useRef, Component, type ReactNode, type ChangeEvent } from 'react';
import { Plus, Pencil, Trash2, AlertCircle, RefreshCw, Download, Building2, Upload, HardDrive, Globe } from 'lucide-react';
import type { SecurityData } from '../../lib/types';
import {
  getSecurities,
  deleteSecurity,
  syncAllPrices,
  syncSecurityPrices,
  fetchLogosBatch,
  getCachedLogoData,
  saveLogoToCache,
  uploadSecurityLogo,
  deleteSecurityLogo,
} from '../../lib/api';
import { SecurityFormModal, SecurityPriceModal } from '../../components/modals';
import { formatCurrency } from '../../lib/types';
import { useSettingsStore } from '../../store';

// Logo data with source tracking
interface LogoData {
  url: string;
  domain: string;
  isFresh: boolean; // true = newly loaded from CDN, false = from cache
}

// Error boundary to catch rendering errors
interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class SecuritiesErrorBoundary extends Component<{ children: ReactNode }, ErrorBoundaryState> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: { componentStack: string }) {
    console.error('Securities view error:', error, info);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="p-6 bg-destructive/10 border border-destructive/20 rounded-lg">
          <h2 className="text-lg font-semibold text-destructive mb-2">Fehler in Wertpapier-Ansicht</h2>
          <pre className="text-sm text-destructive whitespace-pre-wrap">{this.state.error?.message}</pre>
          <pre className="text-xs text-muted-foreground mt-2 whitespace-pre-wrap">{this.state.error?.stack}</pre>
        </div>
      );
    }
    return this.props.children;
  }
}

// Legacy types for direct file viewing
interface LegacySecurity {
  uuid: string;
  name: string;
  currency: string;
  isin?: string | null;
  ticker?: string | null;
  wkn?: string | null;
}

interface PortfolioFile {
  securities?: LegacySecurity[];
}

interface SecuritiesViewProps {
  portfolioFile: PortfolioFile | null;
}

type StatusFilter = 'all' | 'withHoldings' | 'withoutHoldings' | 'retired';

export function SecuritiesView({ portfolioFile }: SecuritiesViewProps) {
  const [dbSecurities, setDbSecurities] = useState<SecurityData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingSecurity, setEditingSecurity] = useState<SecurityData | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);
  const [syncingSecurityId, setSyncingSecurityId] = useState<number | null>(null);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('withHoldings');
  const [searchQuery, setSearchQuery] = useState('');
  const [priceModalSecurity, setPriceModalSecurity] = useState<SecurityData | null>(null);
  const [logos, setLogos] = useState<Map<number, LogoData>>(new Map());
  const [uploadingLogoId, setUploadingLogoId] = useState<number | null>(null);
  const [logoMenuOpen, setLogoMenuOpen] = useState<number | null>(null);
  const [recentlyUploadedLogos, setRecentlyUploadedLogos] = useState<Set<number>>(new Set());

  // Track which logos need caching after they load
  const logosToCache = useRef<Map<number, { url: string; domain: string }>>(new Map());
  const logoInputRef = useRef<HTMLInputElement>(null);
  const pendingLogoSecurityId = useRef<number | null>(null);

  // Close logo menu when clicking outside
  useEffect(() => {
    const handleClickOutside = () => setLogoMenuOpen(null);
    if (logoMenuOpen !== null) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  }, [logoMenuOpen]);

  // Get settings from store
  const syncOnlyHeldSecurities = useSettingsStore((state) => state.syncOnlyHeldSecurities);
  const brandfetchApiKey = useSettingsStore((state) => state.brandfetchApiKey);
  const finnhubApiKey = useSettingsStore((state) => state.finnhubApiKey);
  const coingeckoApiKey = useSettingsStore((state) => state.coingeckoApiKey);
  const alphaVantageApiKey = useSettingsStore((state) => state.alphaVantageApiKey);
  const twelveDataApiKey = useSettingsStore((state) => state.twelveDataApiKey);

  const loadSecurities = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getSecurities();
      setDbSecurities(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSecurities();
  }, [loadSecurities]);

  // Load logos for securities (cache-first strategy)
  useEffect(() => {
    const loadLogos = async () => {
      if (dbSecurities.length === 0 || !brandfetchApiKey) {
        return;
      }

      try {
        // Get logo URLs for all securities
        const securitiesToFetch = dbSecurities.map((s) => ({
          id: s.id,
          ticker: s.ticker || undefined,
          name: s.name || '',
        }));

        const results = await fetchLogosBatch(brandfetchApiKey, securitiesToFetch);
        const newLogos = new Map<number, LogoData>();
        const toCacheMap = new Map<number, { url: string; domain: string }>();

        // For each result, check cache first
        for (const result of results) {
          if (result.logoUrl && result.domain) {
            // Try to get from cache first
            const cachedData = await getCachedLogoData(result.domain);

            if (cachedData) {
              // Use cached data URL (no green border)
              newLogos.set(result.securityId, {
                url: cachedData,
                domain: result.domain,
                isFresh: false,
              });
            } else {
              // Use CDN URL and mark for caching (green border)
              newLogos.set(result.securityId, {
                url: result.logoUrl,
                domain: result.domain,
                isFresh: true,
              });
              toCacheMap.set(result.securityId, {
                url: result.logoUrl,
                domain: result.domain,
              });
            }
          }
        }

        setLogos(newLogos);
        logosToCache.current = toCacheMap;
      } catch (err) {
        console.error('Failed to load logos:', err);
      }
    };

    loadLogos();
  }, [dbSecurities, brandfetchApiKey]);

  // Handle logo load - cache fresh logos after they load in the browser
  const handleLogoLoad = useCallback(async (securityId: number, imgElement: HTMLImageElement) => {
    const toCache = logosToCache.current.get(securityId);
    if (!toCache) return;

    try {
      // Create canvas and draw the loaded image
      const canvas = document.createElement('canvas');
      canvas.width = imgElement.naturalWidth || 64;
      canvas.height = imgElement.naturalHeight || 64;

      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      ctx.drawImage(imgElement, 0, 0);

      // Get base64 PNG data
      const base64Data = canvas.toDataURL('image/png');

      // Save to cache
      await saveLogoToCache(toCache.domain, base64Data);

      // Remove from "to cache" list
      logosToCache.current.delete(securityId);

      // After a delay, mark as no longer fresh (remove green border)
      setTimeout(() => {
        setLogos((prev) => {
          const current = prev.get(securityId);
          if (current && current.isFresh) {
            const updated = new Map(prev);
            updated.set(securityId, { ...current, isFresh: false });
            return updated;
          }
          return prev;
        });
      }, 2000); // Keep green border for 2 seconds
    } catch (err) {
      console.error('Failed to cache logo:', err);
    }
  }, []);

  const handleCreate = () => {
    setEditingSecurity(null);
    setIsModalOpen(true);
  };

  // Logo upload handlers
  const handleLogoUploadClick = (securityId: number) => {
    pendingLogoSecurityId.current = securityId;
    logoInputRef.current?.click();
  };

  const handleLogoFileChange = async (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    const securityId = pendingLogoSecurityId.current;

    if (!file || !securityId) {
      return;
    }

    // Validate file type
    if (!file.type.startsWith('image/')) {
      setError('Bitte wählen Sie eine Bilddatei aus.');
      return;
    }

    // Validate file size (max 500KB)
    if (file.size > 500 * 1024) {
      setError('Das Bild ist zu groß. Maximale Größe: 500 KB.');
      return;
    }

    setUploadingLogoId(securityId);
    setError(null);

    try {
      // Read file as base64
      const reader = new FileReader();
      reader.onload = async () => {
        const base64 = reader.result as string;
        await uploadSecurityLogo(securityId, base64);

        // Reload securities to get the updated logo
        await loadSecurities();
        setSuccess('Logo erfolgreich hochgeladen.');
        setTimeout(() => setSuccess(null), 3000);

        // Show blue ring briefly (3 seconds)
        setRecentlyUploadedLogos((prev) => new Set(prev).add(securityId));
        setTimeout(() => {
          setRecentlyUploadedLogos((prev) => {
            const next = new Set(prev);
            next.delete(securityId);
            return next;
          });
        }, 3000);
      };
      reader.onerror = () => {
        setError('Fehler beim Lesen der Datei.');
      };
      reader.readAsDataURL(file);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploadingLogoId(null);
      pendingLogoSecurityId.current = null;
      // Reset file input
      if (logoInputRef.current) {
        logoInputRef.current.value = '';
      }
    }
  };

  const handleDeleteLogo = async (securityId: number) => {
    if (!confirm('Eigenes Logo wirklich löschen?')) {
      return;
    }

    setUploadingLogoId(securityId);
    setError(null);

    try {
      await deleteSecurityLogo(securityId);
      await loadSecurities();
      setSuccess('Logo erfolgreich gelöscht.');
      setTimeout(() => setSuccess(null), 3000);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setUploadingLogoId(null);
    }
  };

  const handleEdit = (security: SecurityData) => {
    setEditingSecurity(security);
    setIsModalOpen(true);
  };

  const handleDelete = async (security: SecurityData) => {
    if (!confirm(`Wertpapier "${security.name}" wirklich löschen?`)) {
      return;
    }

    setDeletingId(security.id);
    setError(null);

    try {
      await deleteSecurity(security.id);
      await loadSecurities();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeletingId(null);
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setEditingSecurity(null);
  };

  const handleModalSuccess = () => {
    loadSecurities();
  };

  const handleSyncPrices = async () => {
    setIsSyncing(true);
    setError(null);
    setSuccess(null);

    try {
      // Build API keys object
      const apiKeys = {
        finnhub: finnhubApiKey || undefined,
        coingecko: coingeckoApiKey || undefined,
        alphaVantage: alphaVantageApiKey || undefined,
        twelveData: twelveDataApiKey || undefined,
      };

      const result = await syncAllPrices(syncOnlyHeldSecurities, apiKeys);
      if (result.errors > 0) {
        setError(`${result.errors} Fehler beim Abrufen: ${result.errorMessages.slice(0, 3).join(', ')}${result.errorMessages.length > 3 ? '...' : ''}`);
      }
      const modeText = syncOnlyHeldSecurities ? ' (nur im Bestand)' : '';
      setSuccess(`${result.success} von ${result.total} Kurse aktualisiert${modeText}`);
      await loadSecurities(); // Reload to show updated prices
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSyncing(false);
    }
  };

  const handleSyncSingleSecurity = async (securityId: number) => {
    setSyncingSecurityId(securityId);
    setError(null);

    try {
      const apiKeys = {
        finnhub: finnhubApiKey || undefined,
        coingecko: coingeckoApiKey || undefined,
        alphaVantage: alphaVantageApiKey || undefined,
        twelveData: twelveDataApiKey || undefined,
      };

      const result = await syncSecurityPrices([securityId], apiKeys);
      if (result.errors > 0) {
        setError(result.errorMessages[0] || 'Fehler beim Abrufen');
      } else if (result.success > 0) {
        setSuccess('Kurs aktualisiert');
        await loadSecurities();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSyncingSecurityId(null);
    }
  };

  // Use DB data if available, otherwise fall back to legacy file data
  const hasDbData = dbSecurities.length > 0;
  const legacySecurities = portfolioFile?.securities || [];

  // Filter securities based on status and search query
  const filteredSecurities = dbSecurities.filter((security) => {
    // Status filter
    if (statusFilter === 'withHoldings' && security.currentHoldings <= 0) return false;
    if (statusFilter === 'withoutHoldings' && security.currentHoldings > 0) return false;
    if (statusFilter === 'retired' && !security.isRetired) return false;

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        security.name?.toLowerCase().includes(query) ||
        security.isin?.toLowerCase().includes(query) ||
        security.ticker?.toLowerCase().includes(query) ||
        security.wkn?.toLowerCase().includes(query)
      );
    }
    return true;
  });

  // Count stats
  const withHoldingsCount = dbSecurities.filter(s => s.currentHoldings > 0).length;
  const withoutHoldingsCount = dbSecurities.filter(s => s.currentHoldings <= 0 && !s.isRetired).length;
  const retiredCount = dbSecurities.filter(s => s.isRetired).length;

  return (
    <div className="space-y-4">
      {/* Hidden file input for logo upload */}
      <input
        ref={logoInputRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={handleLogoFileChange}
      />

      {/* Header with actions */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">
          Wertpapiere ({filteredSecurities.length} von {dbSecurities.length})
        </h2>
        <div className="flex gap-2">
          <button
            onClick={handleSyncPrices}
            disabled={isSyncing || isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50"
            title="Kurse für alle Wertpapiere mit Kursquelle abrufen"
          >
            <Download size={16} className={isSyncing ? 'animate-pulse' : ''} />
            {isSyncing ? 'Lade Kurse...' : 'Kurse abrufen'}
          </button>
          <button
            onClick={loadSecurities}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
          <button
            onClick={handleCreate}
            className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
          >
            <Plus size={16} />
            Neu
          </button>
        </div>
      </div>

      {/* Filter bar */}
      {hasDbData && (
        <div className="flex flex-wrap items-center gap-3">
          {/* Search input */}
          <input
            type="text"
            placeholder="Suchen (Name, ISIN, Ticker...)"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="px-3 py-1.5 text-sm border border-border rounded-md bg-background w-64"
          />

          {/* Status filter buttons */}
          <div className="flex gap-1 bg-muted p-1 rounded-md">
            <button
              onClick={() => setStatusFilter('all')}
              className={`px-3 py-1 text-sm rounded transition-colors ${
                statusFilter === 'all'
                  ? 'bg-background shadow-sm'
                  : 'hover:bg-background/50'
              }`}
            >
              Alle ({dbSecurities.length})
            </button>
            <button
              onClick={() => setStatusFilter('withHoldings')}
              className={`px-3 py-1 text-sm rounded transition-colors ${
                statusFilter === 'withHoldings'
                  ? 'bg-background shadow-sm text-green-600'
                  : 'hover:bg-background/50'
              }`}
            >
              Mit Bestand ({withHoldingsCount})
            </button>
            <button
              onClick={() => setStatusFilter('withoutHoldings')}
              className={`px-3 py-1 text-sm rounded transition-colors ${
                statusFilter === 'withoutHoldings'
                  ? 'bg-background shadow-sm text-amber-600'
                  : 'hover:bg-background/50'
              }`}
            >
              Ohne Bestand ({withoutHoldingsCount})
            </button>
            <button
              onClick={() => setStatusFilter('retired')}
              className={`px-3 py-1 text-sm rounded transition-colors ${
                statusFilter === 'retired'
                  ? 'bg-background shadow-sm text-muted-foreground'
                  : 'hover:bg-background/50'
              }`}
            >
              Ausgemustert ({retiredCount})
            </button>
          </div>
        </div>
      )}

      {/* Success message */}
      {success && (
        <div className="flex items-center gap-2 p-3 bg-green-500/10 border border-green-500/20 rounded-md text-green-600 text-sm">
          {success}
        </div>
      )}

      {/* Error message */}
      {error && (
        <div className="flex items-center gap-2 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          <AlertCircle size={16} />
          {error}
        </div>
      )}

      {/* Main content */}
      <div className="bg-card rounded-lg border border-border">
        {isLoading && dbSecurities.length === 0 ? (
          <div className="p-6 text-center text-muted-foreground">
            Lade Wertpapiere...
          </div>
        ) : hasDbData ? (
          /* Database securities table */
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50">
                  <th className="text-left py-3 px-4 font-medium">Name</th>
                  <th className="text-left py-3 px-4 font-medium">ISIN</th>
                  <th className="text-left py-3 px-4 font-medium">Ticker</th>
                  <th className="text-left py-3 px-4 font-medium">Kursquelle</th>
                  <th className="text-right py-3 px-4 font-medium">Bestand</th>
                  <th className="text-right py-3 px-4 font-medium">Letzter Kurs</th>
                  <th className="text-left py-3 px-4 font-medium">Kursdatum</th>
                  <th className="text-left py-3 px-4 font-medium">Abgerufen</th>
                  <th className="text-right py-3 px-4 font-medium">Aktionen</th>
                </tr>
              </thead>
              <tbody>
                {filteredSecurities.length === 0 ? (
                  <tr>
                    <td colSpan={9} className="py-8 text-center text-muted-foreground">
                      Keine Wertpapiere gefunden
                    </td>
                  </tr>
                ) : filteredSecurities.map((security) => (
                  <tr
                    key={security.id}
                    className={`border-b border-border last:border-0 hover:bg-muted/30 transition-colors ${
                      security.isRetired ? 'opacity-60' : ''
                    }`}
                  >
                    <td className="py-3 px-4">
                      <div className="flex items-center gap-2">
                        {/* Security Logo with dropdown menu */}
                        {(() => {
                          const logoData = logos.get(security.id);
                          const hasCustomLogo = !!security.customLogo;
                          const isUploading = uploadingLogoId === security.id;
                          const isMenuOpen = logoMenuOpen === security.id;
                          const showBlueRing = recentlyUploadedLogos.has(security.id);

                          return (
                            <div className="relative flex-shrink-0">
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  setLogoMenuOpen(isMenuOpen ? null : security.id);
                                }}
                                className={`w-8 h-8 rounded bg-muted flex items-center justify-center overflow-hidden transition-all duration-300 hover:ring-2 hover:ring-primary/50 ${
                                  showBlueRing
                                    ? 'ring-2 ring-blue-500 ring-offset-1'
                                    : logoData?.isFresh
                                      ? 'ring-2 ring-green-500 ring-offset-1'
                                      : ''
                                }`}
                                title="Klicken für Logo-Optionen"
                              >
                                {isUploading ? (
                                  <RefreshCw size={14} className="animate-spin text-muted-foreground" />
                                ) : hasCustomLogo ? (
                                  <img
                                    src={security.customLogo}
                                    alt=""
                                    className="w-full h-full object-contain"
                                  />
                                ) : logoData ? (
                                  <img
                                    src={logoData.url}
                                    alt=""
                                    className="w-full h-full object-contain"
                                    crossOrigin="anonymous"
                                    onLoad={(e) => {
                                      if (logoData.isFresh) {
                                        handleLogoLoad(security.id, e.currentTarget);
                                      }
                                    }}
                                    onError={(e) => {
                                      e.currentTarget.style.display = 'none';
                                    }}
                                  />
                                ) : (
                                  <Building2 size={16} className="text-muted-foreground" />
                                )}
                              </button>

                              {/* Dropdown menu */}
                              {isMenuOpen && (
                                <div
                                  className="absolute left-0 top-full mt-1 z-50 bg-popover border border-border rounded-md shadow-lg py-1 min-w-[180px]"
                                  onClick={(e) => e.stopPropagation()}
                                >
                                  {/* Logo source indicator */}
                                  {(hasCustomLogo || logoData) && (
                                    <div className="flex items-center gap-2 px-3 py-1.5 text-xs text-muted-foreground border-b border-border mb-1">
                                      {hasCustomLogo || !logoData?.isFresh ? (
                                        <>
                                          <HardDrive size={12} className="text-green-600" />
                                          <span>Logo (lokal)</span>
                                        </>
                                      ) : (
                                        <>
                                          <Globe size={12} className="text-blue-500" />
                                          <span>Logo (web)</span>
                                        </>
                                      )}
                                    </div>
                                  )}
                                  <button
                                    onClick={() => {
                                      setLogoMenuOpen(null);
                                      handleLogoUploadClick(security.id);
                                    }}
                                    className="w-full flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors text-left"
                                  >
                                    <Upload size={14} />
                                    Logo hochladen
                                  </button>
                                  {hasCustomLogo && (
                                    <button
                                      onClick={() => {
                                        setLogoMenuOpen(null);
                                        handleDeleteLogo(security.id);
                                      }}
                                      className="w-full flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-accent transition-colors text-left text-destructive"
                                    >
                                      <Trash2 size={14} />
                                      Logo entfernen
                                    </button>
                                  )}
                                </div>
                              )}
                            </div>
                          );
                        })()}
                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => setPriceModalSecurity(security)}
                            className={`text-left hover:text-primary hover:underline transition-colors ${
                              security.isRetired ? 'text-muted-foreground line-through' : ''
                            }`}
                            title="Kursverlauf anzeigen"
                          >
                            {security.name || 'Unbekannt'}
                          </button>
                          {security.isRetired && (
                            <span className="px-1.5 py-0.5 text-[10px] bg-muted rounded text-muted-foreground">
                              ausgemustert
                            </span>
                          )}
                        </div>
                      </div>
                    </td>
                    <td className="py-3 px-4 font-mono text-muted-foreground">
                      {security.isin || '-'}
                    </td>
                    <td className="py-3 px-4 font-mono text-muted-foreground">
                      {security.ticker || '-'}
                    </td>
                    <td className="py-3 px-4 text-muted-foreground">
                      {security.feed ? (
                        <span className="px-2 py-0.5 text-xs bg-primary/10 rounded text-primary">
                          {security.feed}
                        </span>
                      ) : (
                        <span className="text-muted-foreground/50">-</span>
                      )}
                    </td>
                    <td className="py-3 px-4 text-right tabular-nums">
                      {security.currentHoldings > 0 ? (
                        <span className="text-green-600 font-medium">
                          {security.currentHoldings.toLocaleString('de-DE', { maximumFractionDigits: 4 })}
                        </span>
                      ) : (
                        <span className="text-muted-foreground">-</span>
                      )}
                    </td>
                    <td className="py-3 px-4 text-right tabular-nums">
                      {security.latestPrice
                        ? formatCurrency(security.latestPrice, security.currency)
                        : '-'}
                    </td>
                    <td className="py-3 px-4 text-muted-foreground text-sm">
                      {security.latestPriceDate
                        ? new Date(security.latestPriceDate).toLocaleDateString('de-DE', {
                            day: '2-digit',
                            month: '2-digit',
                            year: 'numeric',
                          })
                        : '-'}
                    </td>
                    <td className="py-3 px-4 text-muted-foreground text-xs">
                      {security.updatedAt
                        ? new Date(security.updatedAt).toLocaleString('de-DE', {
                            day: '2-digit',
                            month: '2-digit',
                            year: 'numeric',
                            hour: '2-digit',
                            minute: '2-digit',
                          })
                        : '-'}
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex justify-end gap-1">
                        <button
                          onClick={() => handleSyncSingleSecurity(security.id)}
                          disabled={syncingSecurityId === security.id || !security.feed}
                          className="p-1.5 hover:bg-muted rounded-md transition-colors disabled:opacity-50"
                          title={security.feed ? 'Kurs abrufen' : 'Keine Kursquelle konfiguriert'}
                        >
                          <RefreshCw
                            size={16}
                            className={
                              syncingSecurityId === security.id
                                ? 'text-primary animate-spin'
                                : 'text-muted-foreground'
                            }
                          />
                        </button>
                        <button
                          onClick={() => handleEdit(security)}
                          className="p-1.5 hover:bg-muted rounded-md transition-colors"
                          title="Bearbeiten"
                        >
                          <Pencil size={16} className="text-muted-foreground" />
                        </button>
                        <button
                          onClick={() => handleDelete(security)}
                          disabled={deletingId === security.id}
                          className="p-1.5 hover:bg-destructive/10 rounded-md transition-colors disabled:opacity-50"
                          title="Löschen"
                        >
                          <Trash2
                            size={16}
                            className={
                              deletingId === security.id
                                ? 'text-muted-foreground animate-pulse'
                                : 'text-destructive'
                            }
                          />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : legacySecurities.length > 0 ? (
          /* Legacy file securities table */
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50">
                  <th className="text-left py-3 px-4 font-medium">Name</th>
                  <th className="text-left py-3 px-4 font-medium">ISIN</th>
                  <th className="text-left py-3 px-4 font-medium">Ticker</th>
                  <th className="text-left py-3 px-4 font-medium">Währung</th>
                </tr>
              </thead>
              <tbody>
                {legacySecurities.map((security, index) => (
                  <tr
                    key={security.uuid || `sec-${index}`}
                    className="border-b border-border last:border-0"
                  >
                    <td className="py-3 px-4">{security.name || 'Unbekannt'}</td>
                    <td className="py-3 px-4 font-mono text-muted-foreground">
                      {security.isin || '-'}
                    </td>
                    <td className="py-3 px-4 font-mono text-muted-foreground">
                      {security.ticker || '-'}
                    </td>
                    <td className="py-3 px-4 text-muted-foreground">
                      {security.currency}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="p-6 text-center text-muted-foreground">
            Keine Wertpapiere vorhanden. Importieren Sie eine .portfolio Datei oder erstellen Sie ein neues Wertpapier.
          </div>
        )}
      </div>

      {/* Security Form Modal */}
      <SecurityFormModal
        isOpen={isModalOpen}
        onClose={handleModalClose}
        onSuccess={handleModalSuccess}
        security={editingSecurity}
      />

      {/* Security Price Modal */}
      <SecurityPriceModal
        isOpen={!!priceModalSecurity}
        onClose={() => setPriceModalSecurity(null)}
        security={priceModalSecurity}
      />
    </div>
  );
}

// Wrapped export with error boundary
export function SecuritiesViewWithErrorBoundary(props: SecuritiesViewProps) {
  return (
    <SecuritiesErrorBoundary>
      <SecuritiesView {...props} />
    </SecuritiesErrorBoundary>
  );
}
