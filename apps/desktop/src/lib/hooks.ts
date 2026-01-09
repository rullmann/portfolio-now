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
