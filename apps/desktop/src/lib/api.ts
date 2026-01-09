/**
 * API functions for interacting with the Tauri backend.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  ImportProgress,
  ImportResult,
  GoImportResult,
  ImportInfo,
  SecurityData,
  AccountData,
  PortfolioData,
  TransactionData,
  PriceData,
  HoldingData,
  PortfolioSummary,
} from './types';

// ============================================================================
// Import API
// ============================================================================

/**
 * Import a Portfolio Performance XML file using the Go sidecar.
 * This is faster and more robust than the Rust parser.
 * @param path Path to the XML file
 * @param outputPath Optional custom output database path
 * @param onProgress Callback for progress updates
 * @returns Import result with statistics
 */
export async function importPPFile(
  path: string,
  outputPath?: string,
  onProgress?: (progress: ImportProgress) => void
): Promise<GoImportResult> {
  let unlisten: UnlistenFn | null = null;

  try {
    // Set up progress listener
    if (onProgress) {
      unlisten = await listen<ImportProgress>('import-progress', (event) => {
        onProgress(event.payload);
      });
    }

    // Call the Go sidecar import command
    const result = await invoke<GoImportResult>('import_pp_file_go', { path, outputPath });

    if (!result.success) {
      throw new Error(result.error || 'Import failed');
    }

    return result;
  } finally {
    // Clean up listener
    if (unlisten) {
      unlisten();
    }
  }
}

/**
 * Import a Portfolio Performance XML file using the Rust parser (legacy).
 * @deprecated Use importPPFile instead (Go sidecar)
 */
export async function importPPFileRust(
  path: string,
  onProgress?: (progress: ImportProgress) => void
): Promise<ImportResult> {
  let unlisten: UnlistenFn | null = null;

  try {
    if (onProgress) {
      unlisten = await listen<ImportProgress>('import-progress', (event) => {
        onProgress(event.payload);
      });
    }

    const result = await invoke<ImportResult>('import_pp_file', { path });
    return result;
  } finally {
    if (unlisten) {
      unlisten();
    }
  }
}

/**
 * Get list of all imports.
 */
export async function getImports(): Promise<ImportInfo[]> {
  return invoke<ImportInfo[]>('get_imports');
}

/**
 * Delete an import and all related data.
 */
export async function deleteImport(importId: number): Promise<void> {
  return invoke('delete_import', { importId });
}

// ============================================================================
// Data Query API
// ============================================================================

/**
 * Get all securities from the database.
 * @param importId Optional filter by import ID
 */
export async function getSecurities(importId?: number): Promise<SecurityData[]> {
  return invoke<SecurityData[]>('get_securities', { importId });
}

/**
 * Get all accounts from the database.
 * @param importId Optional filter by import ID
 */
export async function getAccounts(importId?: number): Promise<AccountData[]> {
  return invoke<AccountData[]>('get_accounts', { importId });
}

/**
 * Get all portfolios from the database.
 * @param importId Optional filter by import ID
 */
export async function getPortfolios(importId?: number): Promise<PortfolioData[]> {
  return invoke<PortfolioData[]>('get_pp_portfolios', { importId });
}

/**
 * Get transactions with optional filters.
 */
export async function getTransactions(options?: {
  ownerType?: string;
  ownerId?: number;
  securityId?: number;
  limit?: number;
  offset?: number;
}): Promise<TransactionData[]> {
  return invoke<TransactionData[]>('get_transactions', options ?? {});
}

/**
 * Get price history for a security.
 */
export async function getPriceHistory(
  securityId: number,
  startDate?: string,
  endDate?: string
): Promise<PriceData[]> {
  return invoke<PriceData[]>('get_price_history', {
    securityId,
    startDate,
    endDate,
  });
}

/**
 * Get holdings for a portfolio.
 */
export async function getHoldings(portfolioId: number): Promise<HoldingData[]> {
  return invoke<HoldingData[]>('get_holdings', { portfolioId });
}

/**
 * Get summary statistics.
 */
export async function getPortfolioSummary(importId?: number): Promise<PortfolioSummary> {
  return invoke<PortfolioSummary>('get_portfolio_summary', { importId });
}

// ============================================================================
// Legacy API (for backward compatibility)
// ============================================================================

export interface LegacyOpenResult {
  path: string;
  portfolio: unknown; // PortfolioFile from legacy XML parser
}

/**
 * Open a portfolio file using the legacy parser.
 * @deprecated Use importPPFile instead for database storage
 */
export async function openPortfolioFile(path: string): Promise<LegacyOpenResult> {
  return invoke<LegacyOpenResult>('open_portfolio_file', { path });
}

/**
 * Save a portfolio file.
 */
export async function savePortfolioFile(path: string, portfolio: unknown): Promise<void> {
  return invoke('save_portfolio_file', { path, portfolio });
}

/**
 * Create a new empty portfolio.
 */
export async function createNewPortfolio(baseCurrency?: string): Promise<unknown> {
  return invoke('create_new_portfolio', { baseCurrency });
}
