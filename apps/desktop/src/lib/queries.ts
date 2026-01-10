/**
 * TanStack Query hooks for data fetching with caching and background refetching.
 */

import { useQuery, useMutation, useQueryClient, QueryClient } from '@tanstack/react-query';
import { getErrorMessage } from './errors';
import {
  getSecurities,
  getAccounts,
  getPortfolios,
  getTransactions,
  getAllHoldings,
  getPortfolioHistory,
  getInvestedCapitalHistory,
  getTaxonomies,
  getWatchlists,
  getInvestmentPlans,
  getBenchmarks,
  syncAllPrices,
  createSecurity,
  updateSecurity,
  deleteSecurity,
  createAccount,
  updateAccount,
  deleteAccount,
  createPPPortfolio,
  updatePPPortfolio,
  deletePPPortfolio,
} from './api';
import type {
  CreateSecurityRequest,
  CreateAccountRequest,
  CreatePortfolioRequest,
} from './types';

// ============================================================================
// Query Client Configuration
// ============================================================================

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      gcTime: 1000 * 60 * 30, // 30 minutes (formerly cacheTime)
      retry: (failureCount, error) => {
        // Don't retry on validation or not-found errors
        const message = getErrorMessage(error);
        if (
          message.includes('nicht gefunden') ||
          message.includes('Ungültig')
        ) {
          return false;
        }
        return failureCount < 2;
      },
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: false, // Don't retry mutations by default
    },
  },
});

// ============================================================================
// Query Keys
// ============================================================================

export const queryKeys = {
  // Core data
  securities: ['securities'] as const,
  accounts: ['accounts'] as const,
  portfolios: ['portfolios'] as const,
  transactions: (filters?: { ownerType?: string; ownerId?: number; securityId?: number }) =>
    ['transactions', filters] as const,
  holdings: ['holdings'] as const,
  portfolioHistory: ['portfolioHistory'] as const,
  investedCapitalHistory: ['investedCapitalHistory'] as const,

  // Features
  taxonomies: ['taxonomies'] as const,
  watchlists: ['watchlists'] as const,
  investmentPlans: ['investmentPlans'] as const,
  benchmarks: ['benchmarks'] as const,

  // Detail queries
  security: (id: number) => ['securities', id] as const,
  account: (id: number) => ['accounts', id] as const,
  portfolio: (id: number) => ['portfolios', id] as const,
};

// ============================================================================
// Securities Queries
// ============================================================================

export function useSecurities() {
  return useQuery({
    queryKey: queryKeys.securities,
    queryFn: () => getSecurities(),
  });
}

export function useCreateSecurity() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateSecurityRequest) => createSecurity(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.securities });
    },
  });
}

export function useUpdateSecurity() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<CreateSecurityRequest> }) =>
      updateSecurity(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.securities });
      queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
    },
  });
}

export function useDeleteSecurity() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => deleteSecurity(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.securities });
      queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
    },
  });
}

// ============================================================================
// Accounts Queries
// ============================================================================

export function useAccounts() {
  return useQuery({
    queryKey: queryKeys.accounts,
    queryFn: () => getAccounts(),
  });
}

export function useCreateAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateAccountRequest) => createAccount(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.accounts });
    },
  });
}

export function useUpdateAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<CreateAccountRequest> }) =>
      updateAccount(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.accounts });
    },
  });
}

export function useDeleteAccount() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => deleteAccount(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.accounts });
    },
  });
}

// ============================================================================
// Portfolios Queries
// ============================================================================

export function usePortfolios() {
  return useQuery({
    queryKey: queryKeys.portfolios,
    queryFn: () => getPortfolios(),
  });
}

export function useCreatePortfolio() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreatePortfolioRequest) => createPPPortfolio(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.portfolios });
    },
  });
}

export function useUpdatePortfolio() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: number; data: Partial<CreatePortfolioRequest> }) =>
      updatePPPortfolio(id, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.portfolios });
    },
  });
}

export function useDeletePortfolio() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => deletePPPortfolio(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.portfolios });
      queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
    },
  });
}

// ============================================================================
// Transactions Queries
// ============================================================================

export function useTransactions(filters?: {
  ownerType?: string;
  ownerId?: number;
  securityId?: number;
  limit?: number;
  offset?: number;
}) {
  return useQuery({
    queryKey: queryKeys.transactions(filters),
    queryFn: () => getTransactions(filters),
  });
}

// ============================================================================
// Holdings & Portfolio History Queries
// ============================================================================

export function useHoldings() {
  return useQuery({
    queryKey: queryKeys.holdings,
    queryFn: getAllHoldings,
  });
}

export function usePortfolioHistory() {
  return useQuery({
    queryKey: queryKeys.portfolioHistory,
    queryFn: () => getPortfolioHistory(),
  });
}

export function useInvestedCapitalHistory() {
  return useQuery({
    queryKey: queryKeys.investedCapitalHistory,
    queryFn: getInvestedCapitalHistory,
  });
}

// ============================================================================
// Feature Queries
// ============================================================================

export function useTaxonomies() {
  return useQuery({
    queryKey: queryKeys.taxonomies,
    queryFn: getTaxonomies,
  });
}

export function useWatchlists() {
  return useQuery({
    queryKey: queryKeys.watchlists,
    queryFn: getWatchlists,
  });
}

export function useInvestmentPlans() {
  return useQuery({
    queryKey: queryKeys.investmentPlans,
    queryFn: getInvestmentPlans,
  });
}

export function useBenchmarks() {
  return useQuery({
    queryKey: queryKeys.benchmarks,
    queryFn: getBenchmarks,
  });
}

// ============================================================================
// Sync Mutations
// ============================================================================

export function useSyncAllPrices() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { onlyHeld?: boolean; apiKeys?: Record<string, string> }) =>
      syncAllPrices(params.onlyHeld, params.apiKeys),
    onSuccess: () => {
      // Invalidate all data that depends on prices
      queryClient.invalidateQueries({ queryKey: queryKeys.securities });
      queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
      queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
    },
  });
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Invalidate all queries - useful after import or major changes
 */
export function invalidateAllQueries() {
  queryClient.invalidateQueries();
}

/**
 * Prefetch common data for faster navigation
 */
export async function prefetchCommonData() {
  await Promise.all([
    queryClient.prefetchQuery({
      queryKey: queryKeys.securities,
      queryFn: () => getSecurities(),
    }),
    queryClient.prefetchQuery({
      queryKey: queryKeys.accounts,
      queryFn: () => getAccounts(),
    }),
    queryClient.prefetchQuery({
      queryKey: queryKeys.portfolios,
      queryFn: () => getPortfolios(),
    }),
    queryClient.prefetchQuery({
      queryKey: queryKeys.holdings,
      queryFn: () => getAllHoldings(),
    }),
  ]);
}
