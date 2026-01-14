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
  SecurityChartData,
  PortfolioSummary,
  CreateSecurityRequest,
  UpdateSecurityRequest,
  SecurityResult,
  CreateAccountRequest,
  UpdateAccountRequest,
  AccountResult,
  CreatePortfolioRequest,
  UpdatePortfolioRequest,
  PortfolioResult,
  CreateTransactionRequest,
  TransactionResult,
  PerformanceResult,
  PeriodReturnData,
  ExchangeRateResult,
  ConversionResult,
  CsvExportResult,
  CsvPreview,
  CsvColumnMapping,
  CsvImportResult,
  TaxonomyData,
  ClassificationData,
  ClassificationAssignmentData,
  TaxonomyAllocation,
  CreateTaxonomyRequest,
  UpdateTaxonomyRequest,
  CreateClassificationRequest,
  UpdateClassificationRequest,
  AssignSecurityRequest,
  DividendReport,
  RealizedGainsReport,
  TaxReport,
  WatchlistData,
  QuoteSyncResult,
  WatchlistSecurityData,
  WatchlistWithSecurities,
  ExternalSecuritySearchResult,
  ExternalSearchResponse,
  StockSplitPreview,
  ApplyStockSplitRequest,
  ApplySpinOffRequest,
  CorporateActionResult,
  // PDF Import
  ParseResult,
  PdfImportPreview,
  PdfImportResult,
  SupportedBank,
  // Investment Plans
  InvestmentPlanData,
  CreateInvestmentPlanRequest,
  InvestmentPlanExecution,
  // Rebalancing
  RebalanceTarget,
  RebalanceAction,
  RebalancePreview,
  AiRebalanceSuggestion,
  // Benchmark
  BenchmarkData,
  BenchmarkComparison,
  BenchmarkDataPoint,
  // New types for Phase 1
  AggregatedHolding,
  PdfExportResult,
  DetectedSplit,
  SplitDetectionResult,
  PortfolioValuePoint,
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

/**
 * Rebuild FIFO cost basis lots from transactions.
 * Call this after fixing FIFO calculation logic.
 */
export async function rebuildFifoLots(): Promise<{ securitiesProcessed: number; lotsCreated: number }> {
  return invoke('rebuild_fifo_lots');
}

// ============================================================================
// Quote Sync API
// ============================================================================

/**
 * Sync prices for specific securities.
 * @param securityIds IDs of securities to sync
 * @param apiKeys Optional API keys for providers that require authentication
 */
export async function syncSecurityPrices(
  securityIds: number[],
  apiKeys?: ApiKeys
): Promise<QuoteSyncResult> {
  return invoke<QuoteSyncResult>('sync_security_prices', { securityIds, apiKeys });
}

/**
 * API keys for quote providers that require authentication.
 */
export interface ApiKeys {
  finnhub?: string;
  alphaVantage?: string;
  coingecko?: string;
  twelveData?: string;
}

/**
 * Quote provider information.
 */
export interface QuoteProvider {
  id: string;
  name: string;
  requiresApiKey: boolean;
  supportsHistorical: boolean;
}

/**
 * Sync prices for all securities with configured feed providers.
 * @param onlyHeld If true, only sync securities with current holdings (default: true)
 * @param apiKeys Optional API keys for providers that require authentication
 */
export async function syncAllPrices(
  onlyHeld: boolean = true,
  apiKeys?: ApiKeys
): Promise<QuoteSyncResult> {
  return invoke<QuoteSyncResult>('sync_all_prices', { onlyHeld, apiKeys });
}

/**
 * Get available quote providers.
 * Providers that require API keys are only included if the key is provided.
 * @param apiKeys Optional API keys to check availability
 */
export async function getAvailableQuoteProviders(apiKeys?: ApiKeys): Promise<QuoteProvider[]> {
  return invoke<QuoteProvider[]>('get_available_quote_providers', { apiKeys });
}

/**
 * Provider status information
 */
export interface ProviderInfo {
  name: string;
  securitiesCount: number;
  requiresApiKey: boolean;
  hasApiKey: boolean;
}

export interface ProviderSecurityCount {
  provider: string;
  count: number;
  canSync: boolean;
}

export interface SecurityProviderInfo {
  id: number;
  name: string;
  provider: string;
  reason: string;
}

export interface OutdatedQuoteInfo {
  id: number;
  name: string;
  ticker: string | null;
  lastQuoteDate: string | null;
  daysOld: number | null;
}

export interface QuoteSyncStatus {
  heldCount: number;
  syncedTodayCount: number;
  outdatedCount: number;
  today: string;
  outdatedSecurities: OutdatedQuoteInfo[];
}

export interface ProviderStatus {
  totalSecurities: number;
  configuredCount: number;
  missingApiKeyCount: number;
  manualCount: number;
  missingProviders: ProviderInfo[];
  byProvider: ProviderSecurityCount[];
  cannotSync: SecurityProviderInfo[];
  quoteStatus: QuoteSyncStatus;
}

/**
 * Get status of quote providers for all securities.
 * Shows which providers are configured, which need API keys, and which securities cannot sync.
 * @param apiKeys Optional API keys to check availability
 */
export async function getProviderStatus(apiKeys?: ApiKeys): Promise<ProviderStatus> {
  return invoke<ProviderStatus>('get_provider_status', { apiKeys });
}

/**
 * Fetch historical prices from the provider and save to database.
 * @param securityId ID of the security
 * @param from Start date (YYYY-MM-DD)
 * @param to End date (YYYY-MM-DD)
 * @param apiKeys Optional API keys for providers that require authentication
 */
export interface HistoricalQuote {
  date: string;
  close: number;
  high?: number;
  low?: number;
  open?: number;
  volume?: number;
}

export async function fetchHistoricalPrices(
  securityId: number,
  from: string,
  to: string,
  apiKeys?: ApiKeys
): Promise<HistoricalQuote[]> {
  return invoke<HistoricalQuote[]>('fetch_historical_prices', { securityId, from, to, apiKeys });
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
 * Upload a custom logo for a security (base64-encoded).
 */
export async function uploadSecurityLogo(securityId: number, logoData: string): Promise<void> {
  return invoke('upload_security_logo', { securityId, logoData });
}

/**
 * Delete the custom logo for a security.
 */
export async function deleteSecurityLogo(securityId: number): Promise<void> {
  return invoke('delete_security_logo', { securityId });
}

/**
 * Get the custom logo for a security.
 */
export async function getSecurityLogo(securityId: number): Promise<string | null> {
  return invoke<string | null>('get_security_logo', { securityId });
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
 * Get FIFO cost basis history and trade data for a security.
 * Used for the security detail chart showing:
 * - Cost basis evolution over time (Einstandskurs)
 * - Trade markers (buys/sells)
 */
export async function getFifoCostBasisHistory(
  securityId: number
): Promise<SecurityChartData> {
  return invoke<SecurityChartData>('get_fifo_cost_basis_history', { securityId });
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

// Note: getPortfolioHistory is defined below with optional date parameters

/**
 * Get invested capital history (cumulative BUY amounts minus SELL proceeds).
 * Shows how the actual money invested evolved over time.
 */
export async function getInvestedCapitalHistory(): Promise<Array<{ date: string; value: number }>> {
  return invoke<Array<{ date: string; value: number }>>('get_invested_capital_history');
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

// ============================================================================
// CRUD API - Securities
// ============================================================================

/**
 * Create a new security.
 */
export async function createSecurity(data: CreateSecurityRequest): Promise<SecurityResult> {
  return invoke<SecurityResult>('create_security', { data });
}

/**
 * Update an existing security.
 */
export async function updateSecurity(id: number, data: UpdateSecurityRequest): Promise<SecurityResult> {
  return invoke<SecurityResult>('update_security', { id, data });
}

/**
 * Delete a security (must have no transactions).
 */
export async function deleteSecurity(id: number): Promise<void> {
  return invoke('delete_security', { id });
}

/**
 * Search securities by name, ISIN, WKN, or ticker.
 */
export async function searchSecurities(
  query: string,
  limit?: number,
  offset?: number
): Promise<SecurityResult[]> {
  return invoke<SecurityResult[]>('search_securities', { query, limit, offset });
}

/**
 * Get a single security by ID.
 */
export async function getSecurity(id: number): Promise<SecurityResult> {
  return invoke<SecurityResult>('get_security', { id });
}

// ============================================================================
// CRUD API - Accounts
// ============================================================================

/**
 * Create a new account.
 */
export async function createAccount(data: CreateAccountRequest): Promise<AccountResult> {
  return invoke<AccountResult>('create_account', { data });
}

/**
 * Update an existing account.
 */
export async function updateAccount(id: number, data: UpdateAccountRequest): Promise<AccountResult> {
  return invoke<AccountResult>('update_account', { id, data });
}

/**
 * Delete an account (must have no transactions).
 */
export async function deleteAccount(id: number): Promise<void> {
  return invoke('delete_account', { id });
}

// ============================================================================
// CRUD API - Portfolios
// ============================================================================

/**
 * Create a new portfolio.
 */
export async function createPPPortfolio(data: CreatePortfolioRequest): Promise<PortfolioResult> {
  return invoke<PortfolioResult>('create_pp_portfolio_new', { data });
}

/**
 * Update an existing portfolio.
 */
export async function updatePPPortfolio(id: number, data: UpdatePortfolioRequest): Promise<PortfolioResult> {
  return invoke<PortfolioResult>('update_pp_portfolio', { id, data });
}

/**
 * Delete a portfolio (must have no transactions).
 */
export async function deletePPPortfolio(id: number): Promise<void> {
  return invoke('delete_pp_portfolio', { id });
}

// ============================================================================
// CRUD API - Transactions
// ============================================================================

/**
 * Create a new transaction.
 * For portfolio BUY/SELL, also creates a matching account transaction via cross-entry.
 */
export async function createTransaction(data: CreateTransactionRequest): Promise<TransactionResult> {
  return invoke<TransactionResult>('create_transaction', { data });
}

/**
 * Delete a transaction (also deletes linked cross-entry and account transaction if applicable).
 */
export async function deleteTransaction(id: number): Promise<void> {
  return invoke('delete_transaction', { id });
}

/**
 * Result of bulk transaction deletion.
 */
export interface BulkDeleteResult {
  /** Number of directly selected transactions that were deleted */
  deletedCount: number;
  /** Number of linked transactions that were also deleted (via cross-entries) */
  linkedDeletedCount: number;
  /** Security IDs that had their FIFO lots rebuilt */
  affectedSecurities: number[];
}

/**
 * Delete multiple transactions at once.
 * Automatically deletes linked cross-entry transactions (e.g., account side of BUY/SELL).
 * Rebuilds FIFO cost basis for affected securities.
 */
export async function deleteTransactionsBulk(ids: number[]): Promise<BulkDeleteResult> {
  return invoke<BulkDeleteResult>('delete_transactions_bulk', { ids });
}

/**
 * Update a transaction - supports all fields.
 */
export async function updateTransaction(id: number, data: {
  date?: string;
  amount?: number;      // cents
  shares?: number;      // scaled by 10^8
  note?: string;
  feeAmount?: number;   // cents
  taxAmount?: number;   // cents
  // Full edit support
  ownerType?: string;   // "portfolio" or "account"
  ownerId?: number;
  txnType?: string;
  securityId?: number;
  currency?: string;
}): Promise<void> {
  return invoke('update_transaction', { id, data });
}

/**
 * Get a single transaction by ID.
 */
export async function getTransaction(id: number): Promise<TransactionResult> {
  return invoke<TransactionResult>('get_transaction', { id });
}

// ============================================================================
// Performance API
// ============================================================================

/**
 * Calculate performance metrics (TTWROR, IRR) for a portfolio.
 */
export async function calculatePerformance(options?: {
  portfolioId?: number;
  startDate?: string;
  endDate?: string;
}): Promise<PerformanceResult> {
  return invoke<PerformanceResult>('calculate_performance', options ?? {});
}

/**
 * Get detailed period returns for performance analysis.
 */
export async function getPeriodReturns(options?: {
  portfolioId?: number;
  startDate?: string;
  endDate?: string;
}): Promise<PeriodReturnData[]> {
  return invoke<PeriodReturnData[]>('get_period_returns', options ?? {});
}

// ============================================================================
// Currency API
// ============================================================================

/**
 * Get exchange rate for a currency pair on a specific date.
 * Uses forward-fill: if no rate on date, uses most recent rate before.
 */
export async function getExchangeRate(
  base: string,
  target: string,
  date?: string
): Promise<ExchangeRateResult> {
  return invoke<ExchangeRateResult>('get_exchange_rate', { base, target, date });
}

/**
 * Convert an amount between currencies.
 */
export async function convertCurrency(
  amount: number,
  from: string,
  to: string,
  date?: string
): Promise<ConversionResult> {
  return invoke<ConversionResult>('convert_currency', { amount, from, to, date });
}

/**
 * Get the latest available exchange rate for a currency pair.
 */
export async function getLatestExchangeRate(
  base: string,
  target: string
): Promise<ExchangeRateResult> {
  return invoke<ExchangeRateResult>('get_latest_exchange_rate', { base, target });
}

/**
 * Get the configured base currency from client settings.
 */
export async function getBaseCurrency(): Promise<string> {
  return invoke<string>('get_base_currency');
}

/**
 * Get total holdings value converted to base currency.
 */
export async function getHoldingsInBaseCurrency(): Promise<number> {
  return invoke<number>('get_holdings_in_base_currency');
}

// ============================================================================
// Export API
// ============================================================================

export interface ExportResult {
  path: string;
  securitiesCount: number;
  accountsCount: number;
  portfoliosCount: number;
}

/**
 * Export the database to a .portfolio file (PP-compatible binary format).
 */
export async function exportDatabaseToPortfolio(path: string): Promise<ExportResult> {
  return invoke<ExportResult>('export_database_to_portfolio', { path });
}

// ============================================================================
// CSV Import/Export API
// ============================================================================

/**
 * Export transactions to CSV file.
 * @param path Output file path
 * @param ownerType Optional filter by "account" or "portfolio"
 * @param ownerId Optional filter by specific owner ID
 */
export async function exportTransactionsCsv(
  path: string,
  ownerType?: string,
  ownerId?: number
): Promise<CsvExportResult> {
  return invoke<CsvExportResult>('export_transactions_csv', { path, ownerType, ownerId });
}

/**
 * Export current holdings to CSV file.
 * @param path Output file path
 */
export async function exportHoldingsCsv(path: string): Promise<CsvExportResult> {
  return invoke<CsvExportResult>('export_holdings_csv', { path });
}

/**
 * Export all securities to CSV file.
 * @param path Output file path
 */
export async function exportSecuritiesCsv(path: string): Promise<CsvExportResult> {
  return invoke<CsvExportResult>('export_securities_csv', { path });
}

/**
 * Export all accounts with balances to CSV file.
 * @param path Output file path
 */
export async function exportAccountsCsv(path: string): Promise<CsvExportResult> {
  return invoke<CsvExportResult>('export_accounts_csv', { path });
}

/**
 * Preview a CSV file for import.
 * Returns column info and sample values for mapping.
 * @param path CSV file path
 */
export async function previewCsv(path: string): Promise<CsvPreview> {
  return invoke<CsvPreview>('preview_csv', { path });
}

/**
 * Import transactions from a CSV file.
 * @param path CSV file path
 * @param mapping Column mapping configuration
 * @param portfolioId Target portfolio ID
 * @param delimiter Optional delimiter character (auto-detected if not provided)
 */
export async function importTransactionsCsv(
  path: string,
  mapping: CsvColumnMapping,
  portfolioId: number,
  delimiter?: string
): Promise<CsvImportResult> {
  return invoke<CsvImportResult>('import_transactions_csv', {
    path,
    mapping,
    portfolioId,
    delimiter,
  });
}

/**
 * Import prices from a CSV file for a specific security.
 * @param path CSV file path
 * @param securityId Target security ID
 * @param dateColumn Column index for dates
 * @param priceColumn Column index for prices
 * @param delimiter Optional delimiter character (auto-detected if not provided)
 */
export async function importPricesCsv(
  path: string,
  securityId: number,
  dateColumn: number,
  priceColumn: number,
  delimiter?: string
): Promise<CsvImportResult> {
  return invoke<CsvImportResult>('import_prices_csv', {
    path,
    securityId,
    dateColumn,
    priceColumn,
    delimiter,
  });
}

// ============================================================================
// Taxonomy API
// ============================================================================

/**
 * Get all taxonomies.
 */
export async function getTaxonomies(): Promise<TaxonomyData[]> {
  return invoke<TaxonomyData[]>('get_taxonomies');
}

/**
 * Get a single taxonomy by ID.
 */
export async function getTaxonomy(id: number): Promise<TaxonomyData> {
  return invoke<TaxonomyData>('get_taxonomy', { id });
}

/**
 * Create a new taxonomy.
 */
export async function createTaxonomy(data: CreateTaxonomyRequest): Promise<TaxonomyData> {
  return invoke<TaxonomyData>('create_taxonomy', { data });
}

/**
 * Update a taxonomy.
 */
export async function updateTaxonomy(id: number, data: UpdateTaxonomyRequest): Promise<TaxonomyData> {
  return invoke<TaxonomyData>('update_taxonomy', { id, data });
}

/**
 * Delete a taxonomy and all its classifications.
 */
export async function deleteTaxonomy(id: number): Promise<void> {
  return invoke('delete_taxonomy', { id });
}

/**
 * Get all classifications for a taxonomy as a flat list.
 */
export async function getClassifications(taxonomyId: number): Promise<ClassificationData[]> {
  return invoke<ClassificationData[]>('get_classifications', { taxonomyId });
}

/**
 * Get classifications as a tree structure.
 */
export async function getClassificationTree(taxonomyId: number): Promise<ClassificationData[]> {
  return invoke<ClassificationData[]>('get_classification_tree', { taxonomyId });
}

/**
 * Create a new classification.
 */
export async function createClassification(data: CreateClassificationRequest): Promise<ClassificationData> {
  return invoke<ClassificationData>('create_classification', { data });
}

/**
 * Update a classification.
 */
export async function updateClassification(id: number, data: UpdateClassificationRequest): Promise<ClassificationData> {
  return invoke<ClassificationData>('update_classification', { id, data });
}

/**
 * Delete a classification (moves children to parent).
 */
export async function deleteClassification(id: number): Promise<void> {
  return invoke('delete_classification', { id });
}

/**
 * Get all assignments for a classification.
 */
export async function getClassificationAssignments(classificationId: number): Promise<ClassificationAssignmentData[]> {
  return invoke<ClassificationAssignmentData[]>('get_classification_assignments', { classificationId });
}

/**
 * Get all assignments for a security across all taxonomies.
 */
export async function getSecurityAssignments(securityId: number): Promise<ClassificationAssignmentData[]> {
  return invoke<ClassificationAssignmentData[]>('get_security_assignments', { securityId });
}

/**
 * Assign a security to a classification.
 */
export async function assignSecurity(data: AssignSecurityRequest): Promise<ClassificationAssignmentData> {
  return invoke<ClassificationAssignmentData>('assign_security', { data });
}

/**
 * Remove a security assignment.
 */
export async function removeAssignment(id: number): Promise<void> {
  return invoke('remove_assignment', { id });
}

/**
 * Calculate portfolio allocation by taxonomy.
 * @param taxonomyId The taxonomy to analyze
 * @param portfolioId Optional portfolio filter (all portfolios if not specified)
 */
export async function getTaxonomyAllocation(
  taxonomyId: number,
  portfolioId?: number
): Promise<TaxonomyAllocation[]> {
  return invoke<TaxonomyAllocation[]>('get_taxonomy_allocation', { taxonomyId, portfolioId });
}

/**
 * Create standard taxonomies (Asset Classes, Regions).
 */
export async function createStandardTaxonomies(): Promise<TaxonomyData[]> {
  return invoke<TaxonomyData[]>('create_standard_taxonomies');
}

// ============================================================================
// Reports API
// ============================================================================

/**
 * Generate a dividend report for a date range.
 * @param startDate Start date (YYYY-MM-DD)
 * @param endDate End date (YYYY-MM-DD)
 * @param portfolioId Optional portfolio filter
 */
export async function generateDividendReport(
  startDate: string,
  endDate: string,
  portfolioId?: number
): Promise<DividendReport> {
  return invoke<DividendReport>('generate_dividend_report', { startDate, endDate, portfolioId });
}

/**
 * Generate a realized gains report for a date range.
 * @param startDate Start date (YYYY-MM-DD)
 * @param endDate End date (YYYY-MM-DD)
 * @param portfolioId Optional portfolio filter
 */
export async function generateRealizedGainsReport(
  startDate: string,
  endDate: string,
  portfolioId?: number
): Promise<RealizedGainsReport> {
  return invoke<RealizedGainsReport>('generate_realized_gains_report', { startDate, endDate, portfolioId });
}

/**
 * Generate a combined tax report for a year.
 * Includes dividends and realized gains.
 * @param year Tax year (e.g., 2024)
 */
export async function generateTaxReport(year: number): Promise<TaxReport> {
  return invoke<TaxReport>('generate_tax_report', { year });
}

/**
 * Get dividend yield for a security (trailing 12 months).
 * @param securityId Security ID
 */
export async function getDividendYield(securityId: number): Promise<number> {
  return invoke<number>('get_dividend_yield', { securityId });
}

// ============================================================================
// Watchlist API
// ============================================================================

/**
 * Get all watchlists.
 */
export async function getWatchlists(): Promise<WatchlistData[]> {
  return invoke<WatchlistData[]>('get_watchlists');
}

/**
 * Get a single watchlist with all securities.
 */
export async function getWatchlist(id: number): Promise<WatchlistWithSecurities> {
  return invoke<WatchlistWithSecurities>('get_watchlist', { id });
}

/**
 * Create a new watchlist.
 */
export async function createWatchlist(name: string): Promise<WatchlistData> {
  return invoke<WatchlistData>('create_watchlist', { name });
}

/**
 * Rename a watchlist.
 */
export async function renameWatchlist(id: number, name: string): Promise<WatchlistData> {
  return invoke<WatchlistData>('rename_watchlist', { id, name });
}

/**
 * Delete a watchlist.
 */
export async function deleteWatchlist(id: number): Promise<void> {
  return invoke('delete_watchlist', { id });
}

/**
 * Get securities in a watchlist with price data.
 */
export async function getWatchlistSecurities(watchlistId: number): Promise<WatchlistSecurityData[]> {
  return invoke<WatchlistSecurityData[]>('get_watchlist_securities', { watchlistId });
}

/**
 * Add a security to a watchlist.
 */
export async function addToWatchlist(watchlistId: number, securityId: number): Promise<void> {
  return invoke('add_to_watchlist', { watchlistId, securityId });
}

/**
 * Remove a security from a watchlist.
 */
export async function removeFromWatchlist(watchlistId: number, securityId: number): Promise<void> {
  return invoke('remove_from_watchlist', { watchlistId, securityId });
}

/**
 * Add multiple securities to a watchlist.
 * @returns Number of securities added
 */
export async function addSecuritiesToWatchlist(
  watchlistId: number,
  securityIds: number[]
): Promise<number> {
  return invoke<number>('add_securities_to_watchlist', { watchlistId, securityIds });
}

/**
 * Get all watchlists that contain a security.
 */
export async function getWatchlistsForSecurity(securityId: number): Promise<WatchlistData[]> {
  return invoke<WatchlistData[]>('get_watchlists_for_security', { securityId });
}

// ============================================================================
// External Security Search API
// ============================================================================

/**
 * Search for securities from external providers (Portfolio Report, Alpha Vantage).
 * Results can be added to watchlist and then to the database.
 */
export async function searchExternalSecurities(
  query: string,
  alphaVantageApiKey?: string
): Promise<ExternalSearchResponse> {
  return invoke<ExternalSearchResponse>('search_external_securities', {
    query,
    alphaVantageApiKey,
  });
}

/**
 * Create a security from external search result and add to watchlist.
 * If security with same ISIN exists, uses existing. Otherwise creates new.
 * @returns The security ID (existing or newly created)
 */
export async function addExternalSecurityToWatchlist(
  watchlistId: number,
  searchResult: ExternalSecuritySearchResult
): Promise<number> {
  // First check if security with ISIN already exists
  if (searchResult.isin) {
    const existingSecurities = await invoke<SecurityData[]>('get_securities', { importId: null });
    const existing = existingSecurities.find(s => s.isin === searchResult.isin);
    if (existing) {
      // Add existing security to watchlist
      await addToWatchlist(watchlistId, existing.id);
      return existing.id;
    }
  }

  // Check by ticker if no ISIN
  if (searchResult.symbol) {
    const existingSecurities = await invoke<SecurityData[]>('get_securities', { importId: null });
    const existing = existingSecurities.find(s => s.ticker === searchResult.symbol);
    if (existing) {
      await addToWatchlist(watchlistId, existing.id);
      return existing.id;
    }
  }

  // Create new security
  const createRequest: CreateSecurityRequest = {
    name: searchResult.name,
    currency: searchResult.currency || 'EUR',
    isin: searchResult.isin,
    wkn: searchResult.wkn,
    ticker: searchResult.symbol,
    // Set appropriate feed based on provider
    feed: searchResult.provider === 'YAHOO' ? 'YAHOO' : 'YAHOO',
    feedUrl: undefined,
  };

  const newSecurity = await createSecurity(createRequest);

  // Add to watchlist
  await addToWatchlist(watchlistId, newSecurity.id);

  // Immediately sync prices for the new security
  try {
    await invoke('sync_security_prices', {
      securityIds: [newSecurity.id],
      apiKeys: null,
    });
  } catch (e) {
    console.warn('Failed to sync initial prices:', e);
  }

  return newSecurity.id;
}

// ============================================================================
// Corporate Actions API
// ============================================================================

/**
 * Preview the effect of a stock split.
 */
export async function previewStockSplit(
  securityId: number,
  effectiveDate: string,
  ratioFrom: number,
  ratioTo: number
): Promise<StockSplitPreview> {
  return invoke<StockSplitPreview>('preview_stock_split', {
    securityId,
    effectiveDate,
    ratioFrom,
    ratioTo,
  });
}

/**
 * Apply a stock split to a security.
 * Adjusts shares in transactions, FIFO lots, and optionally historical prices.
 */
export async function applyStockSplit(request: ApplyStockSplitRequest): Promise<CorporateActionResult> {
  return invoke<CorporateActionResult>('apply_stock_split', { request });
}

/**
 * Undo a previously applied stock split.
 */
export async function undoStockSplit(
  securityId: number,
  effectiveDate: string,
  ratioFrom: number,
  ratioTo: number,
  adjustPrices: boolean
): Promise<CorporateActionResult> {
  return invoke<CorporateActionResult>('undo_stock_split', {
    securityId,
    effectiveDate,
    ratioFrom,
    ratioTo,
    adjustPrices,
  });
}

/**
 * Apply a spin-off corporate action.
 * Creates holdings in a new security based on existing holdings.
 */
export async function applySpinOff(request: ApplySpinOffRequest): Promise<CorporateActionResult> {
  return invoke<CorporateActionResult>('apply_spin_off', { request });
}

/**
 * Get the split-adjusted price for a security.
 */
export async function getSplitAdjustedPrice(
  securityId: number,
  originalPrice: number,
  originalDate: string
): Promise<number> {
  return invoke<number>('get_split_adjusted_price', {
    securityId,
    originalPrice,
    originalDate,
  });
}

// ============================================================================
// PDF Import API
// ============================================================================

/**
 * Get list of supported banks for PDF import.
 */
export async function getSupportedBanks(): Promise<SupportedBank[]> {
  return invoke<SupportedBank[]>('get_supported_banks');
}

/**
 * Preview PDF import without making changes.
 * Shows which transactions will be imported and which securities need to be created.
 */
export async function previewPdfImport(pdfPath: string): Promise<PdfImportPreview> {
  return invoke<PdfImportPreview>('preview_pdf_import', { pdfPath });
}

/**
 * Import transactions from a PDF file.
 * @param pdfPath Path to the PDF file
 * @param portfolioId Portfolio to import buy/sell transactions to
 * @param accountId Account to import transactions to
 * @param createMissingSecurities Whether to create new securities for unknown ISINs
 * @param skipDuplicates Whether to skip potential duplicate transactions
 */
export async function importPdfTransactions(
  pdfPath: string,
  portfolioId: number,
  accountId: number,
  createMissingSecurities: boolean = true,
  skipDuplicates: boolean = true,
  typeOverrides?: Record<number, string>,
  feeOverrides?: Record<number, number>
): Promise<PdfImportResult> {
  return invoke<PdfImportResult>('import_pdf_transactions', {
    pdfPath,
    portfolioId,
    accountId,
    createMissingSecurities,
    skipDuplicates,
    typeOverrides: typeOverrides ?? null,
    feeOverrides: feeOverrides ?? null,
  });
}

/**
 * Extract raw text from a PDF for debugging or custom parsing.
 */
export async function extractPdfRawText(pdfPath: string): Promise<string> {
  return invoke<string>('extract_pdf_raw_text', { pdfPath });
}

/**
 * Parse PDF content that was already extracted.
 */
export async function parsePdfText(content: string): Promise<ParseResult> {
  return invoke<ParseResult>('parse_pdf_text', { content });
}

/**
 * Detect which bank a PDF is from.
 */
export async function detectPdfBank(pdfPath: string): Promise<string | null> {
  return invoke<string | null>('detect_pdf_bank', { pdfPath });
}

// ============================================================================
// Investment Plans API
// ============================================================================

/**
 * Get all investment plans.
 */
export async function getInvestmentPlans(): Promise<InvestmentPlanData[]> {
  return invoke<InvestmentPlanData[]>('get_investment_plans');
}

/**
 * Get a single investment plan with execution history.
 */
export async function getInvestmentPlan(id: number): Promise<InvestmentPlanData> {
  return invoke<InvestmentPlanData>('get_investment_plan', { id });
}

/**
 * Create a new investment plan.
 */
export async function createInvestmentPlan(data: CreateInvestmentPlanRequest): Promise<InvestmentPlanData> {
  return invoke<InvestmentPlanData>('create_investment_plan', { data });
}

/**
 * Update an investment plan.
 */
export async function updateInvestmentPlan(
  id: number,
  data: Partial<CreateInvestmentPlanRequest> & { isActive?: boolean }
): Promise<InvestmentPlanData> {
  return invoke<InvestmentPlanData>('update_investment_plan', { id, data });
}

/**
 * Delete an investment plan.
 */
export async function deleteInvestmentPlan(id: number): Promise<void> {
  return invoke('delete_investment_plan', { id });
}

/**
 * Get executions for a plan.
 */
export async function getInvestmentPlanExecutions(planId: number): Promise<InvestmentPlanExecution[]> {
  return invoke<InvestmentPlanExecution[]>('get_investment_plan_executions', { planId });
}

/**
 * Manually execute an investment plan.
 * @param planId Plan ID
 * @param date Execution date (YYYY-MM-DD)
 * @param price Optional price override
 */
export async function executeInvestmentPlan(
  planId: number,
  date: string,
  price?: number
): Promise<InvestmentPlanExecution> {
  return invoke<InvestmentPlanExecution>('execute_investment_plan', { planId, date, price });
}

/**
 * Get plans due for execution on a date.
 */
export async function getPlansDueForExecution(date: string): Promise<InvestmentPlanData[]> {
  return invoke<InvestmentPlanData[]>('get_plans_due_for_execution', { date });
}

// ============================================================================
// Rebalancing API
// ============================================================================

/**
 * Preview rebalancing actions.
 * @param portfolioId Portfolio to rebalance
 * @param targets Target allocations
 * @param newCash Optional additional cash to invest
 */
export async function previewRebalance(
  portfolioId: number,
  targets: RebalanceTarget[],
  newCash?: number
): Promise<RebalancePreview> {
  return invoke<RebalancePreview>('preview_rebalance', { portfolioId, targets, newCash });
}

/**
 * Execute rebalancing by creating buy/sell transactions.
 */
export async function executeRebalance(
  portfolioId: number,
  accountId: number,
  actions: RebalanceAction[],
  date?: string
): Promise<number> {
  return invoke<number>('execute_rebalance', { portfolioId, accountId, actions, date });
}

/**
 * Calculate deviation from target allocation.
 */
export async function calculateDeviation(
  portfolioId: number,
  targets: RebalanceTarget[]
): Promise<number> {
  return invoke<number>('calculate_deviation', { portfolioId, targets });
}

/**
 * Get AI-powered rebalancing suggestions.
 * @param portfolioId Portfolio to analyze
 * @param provider AI provider (claude, openai, gemini, perplexity)
 * @param model Model name
 * @param apiKey API key for the provider
 * @param baseCurrency Base currency for calculations
 */
export async function suggestRebalanceWithAi(
  portfolioId: number,
  provider: string,
  model: string,
  apiKey: string,
  baseCurrency: string
): Promise<AiRebalanceSuggestion> {
  return invoke<AiRebalanceSuggestion>('suggest_rebalance_with_ai', {
    request: {
      portfolioId,
      provider,
      model,
      apiKey,
      baseCurrency,
    },
  });
}

// ============================================================================
// Benchmark API
// ============================================================================

/**
 * Get all benchmarks.
 */
export async function getBenchmarks(): Promise<BenchmarkData[]> {
  return invoke<BenchmarkData[]>('get_benchmarks');
}

/**
 * Add a security as a benchmark.
 */
export async function addBenchmark(securityId: number, startDate?: string): Promise<BenchmarkData> {
  return invoke<BenchmarkData>('add_benchmark', { securityId, startDate });
}

/**
 * Remove a benchmark.
 */
export async function removeBenchmark(id: number): Promise<void> {
  return invoke('remove_benchmark', { id });
}

/**
 * Compare portfolio performance against a benchmark.
 * @param portfolioId Portfolio to compare (or null for all)
 * @param benchmarkId Benchmark to compare against
 * @param startDate Start date (YYYY-MM-DD)
 * @param endDate End date (YYYY-MM-DD)
 */
export async function compareToBenchmark(
  portfolioId: number | null,
  benchmarkId: number,
  startDate: string,
  endDate: string
): Promise<BenchmarkComparison> {
  return invoke<BenchmarkComparison>('compare_to_benchmark', {
    portfolioId,
    benchmarkId,
    startDate,
    endDate,
  });
}

/**
 * Get time series data for benchmark comparison chart.
 */
export async function getBenchmarkComparisonData(
  portfolioId: number | null,
  benchmarkId: number,
  startDate: string,
  endDate: string
): Promise<BenchmarkDataPoint[]> {
  return invoke<BenchmarkDataPoint[]>('get_benchmark_comparison_data', {
    portfolioId,
    benchmarkId,
    startDate,
    endDate,
  });
}

// ============================================================================
// Brandfetch (Logo) API
// ============================================================================

export interface LogoResult {
  securityId: number;
  logoUrl: string | null;
  domain: string | null;
}

export interface ReloadLogosResult {
  cleared: number;
  downloaded: number;
  failed: number;
  totalDomains: number;
}

/**
 * Get logo URL for a security.
 * Returns a Brandfetch CDN URL that can be used directly in img src.
 */
export async function fetchSecurityLogo(
  clientId: string,
  securityId: number,
  ticker: string | undefined,
  name: string
): Promise<LogoResult> {
  return invoke<LogoResult>('fetch_security_logo', {
    clientId,
    securityId,
    ticker: ticker || null,
    name,
  });
}

/**
 * Get cached logo path for a security (deprecated - always returns null).
 */
export async function getCachedLogo(
  securityId: number,
  ticker: string | undefined,
  name: string
): Promise<string | null> {
  return invoke<string | null>('get_cached_logo', {
    securityId,
    ticker: ticker || null,
    name,
  });
}

/**
 * Clear all cached logos.
 */
export async function clearLogoCache(): Promise<number> {
  return invoke<number>('clear_logo_cache');
}

/**
 * Batch get logo URLs for multiple securities.
 * Returns Brandfetch CDN URLs that can be used directly in img src.
 */
export async function fetchLogosBatch(
  clientId: string,
  securities: Array<{ id: number; ticker?: string; name: string }>
): Promise<LogoResult[]> {
  return invoke<LogoResult[]>('fetch_logos_batch', {
    clientId,
    securities: securities.map((s) => [s.id, s.ticker || null, s.name]),
  });
}

/**
 * Reload all logos - clears cache and re-downloads everything.
 */
export async function reloadAllLogos(
  clientId: string,
  securities: Array<{ id: number; ticker?: string; name: string }>
): Promise<ReloadLogosResult> {
  return invoke<ReloadLogosResult>('reload_all_logos', {
    clientId,
    securities: securities.map((s) => [s.id, s.ticker || null, s.name]),
  });
}

/**
 * Check if a logo is cached locally for a domain.
 */
export async function isLogoCached(domain: string): Promise<boolean> {
  return invoke<boolean>('is_logo_cached', { domain });
}

/**
 * Get cached logo as base64 data URL.
 * Returns null if not cached.
 */
export async function getCachedLogoData(domain: string): Promise<string | null> {
  return invoke<string | null>('get_cached_logo_data', { domain });
}

/**
 * Save logo to local cache.
 * @param domain The domain (e.g., "apple.com")
 * @param base64Data The image data as base64 (with or without data:image prefix)
 * @returns The file path where logo was saved
 */
export async function saveLogoToCache(domain: string, base64Data: string): Promise<string> {
  return invoke<string>('save_logo_to_cache', { domain, base64Data });
}

// ============================================================================
// Aggregated Holdings API
// ============================================================================

/**
 * Get all holdings aggregated by ISIN across all portfolios.
 * Includes current price, cost basis, and gain/loss calculations.
 */
export async function getAllHoldings(): Promise<AggregatedHolding[]> {
  return invoke<AggregatedHolding[]>('get_all_holdings');
}

/**
 * Get portfolio value history over time.
 * @param startDate Optional start date (YYYY-MM-DD)
 * @param endDate Optional end date (YYYY-MM-DD)
 */
export async function getPortfolioHistory(
  startDate?: string,
  endDate?: string
): Promise<PortfolioValuePoint[]> {
  return invoke<PortfolioValuePoint[]>('get_portfolio_history', { startDate, endDate });
}

// ============================================================================
// PDF Export API
// ============================================================================

/**
 * Export portfolio summary to PDF.
 * @param path Output file path
 * @param portfolioId Optional portfolio ID (null for all)
 */
export async function exportPortfolioSummaryPdf(
  path: string,
  portfolioId?: number | null
): Promise<PdfExportResult> {
  return invoke<PdfExportResult>('export_portfolio_summary_pdf', { path, portfolioId });
}

/**
 * Export holdings to PDF.
 * @param path Output file path
 * @param portfolioId Optional portfolio ID (null for all)
 * @param date Optional date (YYYY-MM-DD)
 */
export async function exportHoldingsPdf(
  path: string,
  portfolioId?: number | null,
  date?: string | null
): Promise<PdfExportResult> {
  return invoke<PdfExportResult>('export_holdings_pdf', { path, portfolioId, date });
}

/**
 * Export performance report to PDF.
 * @param path Output file path
 * @param startDate Start date (YYYY-MM-DD)
 * @param endDate End date (YYYY-MM-DD)
 * @param portfolioId Optional portfolio ID (null for all)
 */
export async function exportPerformancePdf(
  path: string,
  startDate: string,
  endDate: string,
  portfolioId?: number | null
): Promise<PdfExportResult> {
  return invoke<PdfExportResult>('export_performance_pdf', { path, startDate, endDate, portfolioId });
}

/**
 * Export dividend report to PDF.
 * @param path Output file path
 * @param year Year for the report
 * @param portfolioId Optional portfolio ID (null for all)
 */
export async function exportDividendPdf(
  path: string,
  year: number,
  portfolioId?: number | null
): Promise<PdfExportResult> {
  return invoke<PdfExportResult>('export_dividend_pdf', { path, year, portfolioId });
}

/**
 * Export tax report to PDF.
 * @param path Output file path
 * @param year Year for the report
 */
export async function exportTaxReportPdf(path: string, year: number): Promise<PdfExportResult> {
  return invoke<PdfExportResult>('export_tax_report_pdf', { path, year });
}

// ============================================================================
// Stock Split Detection API
// ============================================================================

/**
 * Detect potential stock splits for a security based on price patterns.
 * @param securityId Security to analyze
 */
export async function detectSecuritySplits(securityId: number): Promise<DetectedSplit[]> {
  return invoke<DetectedSplit[]>('detect_security_splits', { securityId });
}

/**
 * Detect potential stock splits for all securities.
 */
export async function detectAllSplits(): Promise<SplitDetectionResult> {
  return invoke<SplitDetectionResult>('detect_all_splits');
}

// ============================================================================
// Retire Entity API
// ============================================================================

/**
 * Mark a security as retired (no longer actively traded).
 * Does not delete the security, just sets isRetired flag.
 */
export async function retireSecurity(id: number): Promise<void> {
  return invoke('retire_security', { id });
}

/**
 * Mark an account as retired.
 */
export async function retireAccount(id: number): Promise<void> {
  return invoke('retire_account', { id });
}

/**
 * Mark a portfolio as retired.
 */
export async function retirePortfolio(id: number): Promise<void> {
  return invoke('retire_portfolio', { id });
}

// Note: exportDatabaseToPortfolio and getBaseCurrency are defined in Export API section above

// ============================================================================
// Chart Annotations API
// ============================================================================

import type { PersistedAnnotation, SaveAnnotationRequest } from './types';

/**
 * Save multiple annotations for a security.
 * @param securityId The security ID
 * @param annotations Array of annotations to save
 * @param clearExisting If true, clears existing AI annotations first
 */
export async function saveAnnotations(
  securityId: number,
  annotations: SaveAnnotationRequest[],
  clearExisting: boolean = true
): Promise<PersistedAnnotation[]> {
  return invoke<PersistedAnnotation[]>('save_annotations', {
    securityId,
    annotations,
    clearExisting,
  });
}

/**
 * Get annotations for a security.
 * @param securityId The security ID
 * @param visibleOnly If true, only returns visible annotations
 */
export async function getAnnotations(
  securityId: number,
  visibleOnly: boolean = true
): Promise<PersistedAnnotation[]> {
  return invoke<PersistedAnnotation[]>('get_annotations', { securityId, visibleOnly });
}

/**
 * Delete a single annotation.
 * @param annotationId The annotation ID to delete
 */
export async function deleteAnnotation(annotationId: number): Promise<void> {
  return invoke('delete_annotation', { annotationId });
}

/**
 * Toggle annotation visibility.
 * @param annotationId The annotation ID
 * @returns The new visibility state
 */
export async function toggleAnnotationVisibility(annotationId: number): Promise<boolean> {
  return invoke<boolean>('toggle_annotation_visibility', { annotationId });
}

/**
 * Clear all AI annotations for a security.
 * @param securityId The security ID
 * @returns Number of deleted annotations
 */
export async function clearAiAnnotations(securityId: number): Promise<number> {
  return invoke<number>('clear_ai_annotations', { securityId });
}

// ============================================================================
// Price Alerts
// ============================================================================

import type {
  PriceAlert,
  CreateAlertRequest,
  UpdateAlertRequest,
  TriggeredAlert,
} from './types';

/**
 * Get all price alerts, optionally filtered by security.
 */
export async function getPriceAlerts(securityId?: number): Promise<PriceAlert[]> {
  return invoke<PriceAlert[]>('get_price_alerts', { securityId });
}

/**
 * Get only active alerts.
 */
export async function getActiveAlerts(): Promise<PriceAlert[]> {
  return invoke<PriceAlert[]>('get_active_alerts');
}

/**
 * Create a new price alert.
 */
export async function createPriceAlert(request: CreateAlertRequest): Promise<PriceAlert> {
  return invoke<PriceAlert>('create_price_alert', { request });
}

/**
 * Update an existing price alert.
 */
export async function updatePriceAlert(request: UpdateAlertRequest): Promise<PriceAlert> {
  return invoke<PriceAlert>('update_price_alert', { request });
}

/**
 * Delete a price alert.
 */
export async function deletePriceAlert(id: number): Promise<void> {
  return invoke('delete_price_alert', { id });
}

/**
 * Toggle alert active status.
 */
export async function togglePriceAlert(id: number): Promise<PriceAlert> {
  return invoke<PriceAlert>('toggle_price_alert', { id });
}

/**
 * Check all active alerts against current prices.
 * Returns list of triggered alerts.
 */
export async function checkPriceAlerts(): Promise<TriggeredAlert[]> {
  return invoke<TriggeredAlert[]>('check_price_alerts');
}

/**
 * Reset triggered status for an alert (to allow re-triggering).
 */
export async function resetAlertTrigger(id: number): Promise<void> {
  return invoke('reset_alert_trigger', { id });
}
