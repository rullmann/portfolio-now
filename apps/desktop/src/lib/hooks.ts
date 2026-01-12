/**
 * React hooks for Portfolio Performance data.
 */

import { useState, useEffect, useCallback } from 'react';
import * as api from './api';
import type {
  ImportProgress,
  GoImportResult,
  ImportInfo,
  SecurityData,
  AccountData,
  PortfolioData,
  TransactionData,
  HoldingData,
  PortfolioSummary,
} from './types';

// ============================================================================
// Import Hook
// ============================================================================

export interface UseImportState {
  isImporting: boolean;
  progress: ImportProgress | null;
  result: GoImportResult | null;
  error: string | null;
  importFile: (path: string, outputPath?: string) => Promise<GoImportResult | null>;
  reset: () => void;
}

export function useImport(): UseImportState {
  const [isImporting, setIsImporting] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [result, setResult] = useState<GoImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const importFile = useCallback(async (path: string, outputPath?: string): Promise<GoImportResult | null> => {
    setIsImporting(true);
    setProgress(null);
    setResult(null);
    setError(null);

    try {
      const importResult = await api.importPPFile(path, outputPath, (p) => {
        setProgress(p);
      });
      setResult(importResult);
      return importResult;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      return null;
    } finally {
      setIsImporting(false);
    }
  }, []);

  const reset = useCallback(() => {
    setIsImporting(false);
    setProgress(null);
    setResult(null);
    setError(null);
  }, []);

  return { isImporting, progress, result, error, importFile, reset };
}

// ============================================================================
// Data Hooks
// ============================================================================

export interface UseDataState<T> {
  data: T | null;
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useImports(): UseDataState<ImportInfo[]> & { deleteImport: (id: number) => Promise<void> } {
  const [data, setData] = useState<ImportInfo[] | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const imports = await api.getImports();
      setData(imports);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const deleteImport = useCallback(async (id: number) => {
    await api.deleteImport(id);
    await refresh();
  }, [refresh]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { data, isLoading, error, refresh, deleteImport };
}

export function useSecurities(importId?: number): UseDataState<SecurityData[]> {
  const [data, setData] = useState<SecurityData[] | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const securities = await api.getSecurities(importId);
      setData(securities);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [importId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { data, isLoading, error, refresh };
}

export function useAccounts(importId?: number): UseDataState<AccountData[]> {
  const [data, setData] = useState<AccountData[] | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const accounts = await api.getAccounts(importId);
      setData(accounts);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [importId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { data, isLoading, error, refresh };
}

export function usePortfolios(importId?: number): UseDataState<PortfolioData[]> {
  const [data, setData] = useState<PortfolioData[] | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const portfolios = await api.getPortfolios(importId);
      setData(portfolios);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [importId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { data, isLoading, error, refresh };
}

export function useTransactions(options?: {
  ownerType?: string;
  ownerId?: number;
  securityId?: number;
  limit?: number;
  offset?: number;
}): UseDataState<TransactionData[]> {
  const [data, setData] = useState<TransactionData[] | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const transactions = await api.getTransactions(options);
      setData(transactions);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [options?.ownerType, options?.ownerId, options?.securityId, options?.limit, options?.offset]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { data, isLoading, error, refresh };
}

export function useHoldings(portfolioId: number): UseDataState<HoldingData[]> {
  const [data, setData] = useState<HoldingData[] | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const holdings = await api.getHoldings(portfolioId);
      setData(holdings);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [portfolioId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { data, isLoading, error, refresh };
}

export function usePortfolioSummary(importId?: number): UseDataState<PortfolioSummary> {
  const [data, setData] = useState<PortfolioSummary | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const summary = await api.getPortfolioSummary(importId);
      setData(summary);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [importId]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { data, isLoading, error, refresh };
}

// ============================================================================
// Logo Caching Hook
// ============================================================================

export interface CachedLogo {
  url: string;
  isLocal: boolean;
  domain: string;
}

export interface UseCachedLogosState {
  logos: Map<number, CachedLogo>;
  isLoading: boolean;
  refresh: () => Promise<void>;
}

interface SecurityInfo {
  id: number;
  ticker?: string;
  name: string;
}

/**
 * Hook for loading logos with local caching.
 * - First checks local cache (works even without API key)
 * - Falls back to Brandfetch CDN if API key is available
 * - Downloads and caches logos from CDN
 */
export function useCachedLogos(
  securities: SecurityInfo[],
  brandfetchApiKey: string | null
): UseCachedLogosState {
  const [logos, setLogos] = useState<Map<number, CachedLogo>>(new Map());
  const [isLoading, setIsLoading] = useState(false);

  const loadLogos = useCallback(async () => {
    if (securities.length === 0) {
      setLogos(new Map());
      return;
    }

    setIsLoading(true);

    try {
      // Get domain info (works even without API key)
      // API key is passed to get CDN URLs, but domains are always returned
      const results = await api.fetchLogosBatch(brandfetchApiKey || '', securities);
      const newLogos = new Map<number, CachedLogo>();

      // Process each result
      for (const result of results) {
        // Need at least a domain to look up cached logos
        if (!result.domain) continue;

        // Check if cached locally
        const cachedData = await api.getCachedLogoData(result.domain);

        if (cachedData) {
          // Use local cache (works even without API key!)
          newLogos.set(result.securityId, {
            url: cachedData,
            isLocal: true,
            domain: result.domain,
          });
        } else if (result.logoUrl) {
          // Not cached but have CDN URL - use it and try to cache
          newLogos.set(result.securityId, {
            url: result.logoUrl,
            isLocal: false,
            domain: result.domain,
          });

          // Try to download and cache in background
          fetchAndCacheLogo(result.logoUrl, result.domain, result.securityId, setLogos);
        }
        // If no cache and no CDN URL (no API key), skip this security
      }

      setLogos(newLogos);
    } catch (err) {
      console.error('Failed to load logos:', err);
    } finally {
      setIsLoading(false);
    }
  }, [securities, brandfetchApiKey]);

  useEffect(() => {
    loadLogos();
  }, [loadLogos]);

  return { logos, isLoading, refresh: loadLogos };
}

/**
 * Fetch logo from CDN and save to local cache.
 */
async function fetchAndCacheLogo(
  cdnUrl: string,
  domain: string,
  securityId: number,
  setLogos: React.Dispatch<React.SetStateAction<Map<number, CachedLogo>>>
): Promise<void> {
  try {
    // Fetch the image
    const response = await fetch(cdnUrl);
    if (!response.ok) return;

    const blob = await response.blob();

    // Convert to base64
    const reader = new FileReader();
    reader.onload = async () => {
      const base64 = reader.result as string;

      try {
        // Save to local cache
        await api.saveLogoToCache(domain, base64);

        // Update state to reflect local cache
        setLogos((prev) => {
          const newMap = new Map(prev);
          const existing = newMap.get(securityId);
          if (existing) {
            newMap.set(securityId, {
              url: base64,
              isLocal: true,
              domain: existing.domain,
            });
          }
          return newMap;
        });
      } catch (err) {
        console.error('Failed to cache logo for', domain, err);
      }
    };
    reader.readAsDataURL(blob);
  } catch (err) {
    // Silently fail - logo will continue using CDN URL
    console.debug('Failed to fetch logo for caching:', domain, err);
  }
}

// ============================================================================
// Re-export batch analysis hook
// ============================================================================

export { usePortfolioBatchAnalysis } from './hooks/usePortfolioBatchAnalysis';
export type { HoldingForAnalysis, BatchAnalysisOptions, UseBatchAnalysisResult } from './hooks/usePortfolioBatchAnalysis';

// ============================================================================
// Re-export escape key hook
// ============================================================================

export { useEscapeKey } from './hooks/useEscapeKey';
