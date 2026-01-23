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
  BrokerDetectionResult,
  BrokerTemplateSummary,
  AiCsvAnalysisResponse,
  TaxonomyData,
  ClassificationData,
  ClassificationAssignmentData,
  SecurityClassification,
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
  // Custom Attributes
  AttributeType,
  CreateAttributeTypeRequest,
  UpdateAttributeTypeRequest,
  AttributeValue,
  SetAttributeValueRequest,
  SecurityWithAttribute,
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

// ============================================================================
// Quote Provider Suggestions
// ============================================================================

/**
 * Quote provider suggestion from rule-based analysis
 */
export interface QuoteSuggestion {
  securityId: number;
  securityName: string;
  isin: string | null;
  ticker: string | null;
  suggestedFeed: string;
  suggestedFeedUrl: string | null;
  /** Suggested ticker if security has no ticker */
  suggestedTicker: string | null;
  confidence: number;
  reason: string;
}

/**
 * Info about securities without configured quote providers
 */
export interface UnconfiguredSecuritiesInfo {
  totalUnconfigured: number;
  heldUnconfigured: number;
}

/**
 * Get quote provider suggestions for securities without configured feed.
 * Uses rule-based analysis (ISIN prefix, ticker format, crypto detection).
 * @param portfolioId Optional portfolio ID to filter to held securities in specific portfolio
 * @param heldOnly If true, filter to securities held in any portfolio (ignored if portfolioId is set)
 */
export async function suggestQuoteProviders(portfolioId?: number, heldOnly?: boolean): Promise<QuoteSuggestion[]> {
  return invoke<QuoteSuggestion[]>('suggest_quote_providers', { portfolioId, heldOnly });
}

/**
 * Apply a quote provider suggestion to a security.
 * @param securityId Security to update
 * @param feed Provider name (e.g., "YAHOO", "COINGECKO")
 * @param feedUrl Optional feed URL/suffix (e.g., ".SW" for Swiss stocks)
 * @param ticker Optional ticker to set (used when suggesting ticker for securities without one)
 */
export async function applyQuoteSuggestion(
  securityId: number,
  feed: string,
  feedUrl?: string | null,
  ticker?: string | null
): Promise<void> {
  return invoke<void>('apply_quote_suggestion', { securityId, feed, feedUrl, ticker });
}

/**
 * Get count of securities without configured quote provider.
 */
export async function getUnconfiguredSecuritiesCount(): Promise<UnconfiguredSecuritiesInfo> {
  return invoke<UnconfiguredSecuritiesInfo>('get_unconfigured_securities_count', {});
}

// =============================================================================
// QUOTE CONFIGURATION AUDIT
// =============================================================================

/**
 * Result of auditing a single security's quote configuration
 */
export interface QuoteConfigAuditResult {
  securityId: number;
  securityName: string;
  ticker?: string;
  feed: string;
  /** Status: "ok", "stale", "missing", "config_error", "suspicious", "unconfigured" */
  status: 'ok' | 'stale' | 'missing' | 'config_error' | 'suspicious' | 'unconfigured';
  lastPriceDate?: string;
  daysSinceLastPrice?: number;
  /** Error message when status is "config_error" or "unconfigured" */
  errorMessage?: string;
  /** Last known price from database */
  lastKnownPrice?: number;
  /** Price fetched during audit */
  fetchedPrice?: number;
  /** Price deviation in percent (for "suspicious" status) */
  priceDeviation?: number;
}

/**
 * Summary of audit results
 */
export interface QuoteAuditSummary {
  totalAudited: number;
  okCount: number;
  staleCount: number;
  missingCount: number;
  /** Count of securities where fetch failed */
  configErrorCount: number;
  /** Count of securities with suspicious price deviation (>50%) */
  suspiciousCount: number;
  /** Count of securities without configured feed */
  unconfiguredCount: number;
  results: QuoteConfigAuditResult[];
}

/**
 * Audit all configured quote sources and provide improvement recommendations.
 * Now performs actual quote fetches to verify configuration works.
 * @param onlyHeld If true, only audit securities with current holdings (default: true)
 * @param apiKeys Optional API keys for providers that require authentication
 */
export async function auditQuoteConfigurations(
  onlyHeld?: boolean,
  apiKeys?: ApiKeys
): Promise<QuoteAuditSummary> {
  return invoke<QuoteAuditSummary>('audit_quote_configurations', { onlyHeld, apiKeys });
}

/**
 * Suggestion for fixing a broken quote configuration
 */
export interface QuoteFixSuggestion {
  securityId: number;
  currentProvider: string;
  currentSymbol?: string;
  suggestedProvider: string;
  suggestedSymbol: string;
  suggestedFeedUrl?: string;
  /** Source of the suggestion: "known_mapping", "isin_search", "suffix_variant", "yahoo_search", "tradingview_search" */
  source: string;
  /** Confidence score 0-1 */
  confidence: number;
  /** Validated price from actual quote fetch (only set if validation succeeded) */
  validatedPrice?: number;
}

/**
 * Get fix suggestions for a security with broken quote configuration.
 * @param securityId ID of the security
 * @param apiKeys Optional API keys for providers that require authentication
 */
export async function getQuoteFixSuggestions(
  securityId: number,
  apiKeys?: ApiKeys
): Promise<QuoteFixSuggestion[]> {
  return invoke<QuoteFixSuggestion[]>('get_quote_fix_suggestions', { securityId, apiKeys });
}

/**
 * Apply a quote fix suggestion to a security.
 * @param securityId ID of the security
 * @param newProvider New quote provider
 * @param newSymbol New ticker symbol
 * @param newFeedUrl Optional new feed URL
 */
export async function applyQuoteFix(
  securityId: number,
  newProvider: string,
  newSymbol: string,
  newFeedUrl?: string
): Promise<void> {
  return invoke<void>('apply_quote_fix', { securityId, newProvider, newSymbol, newFeedUrl });
}

// =============================================================================
// UNIFIED QUOTE MANAGER
// =============================================================================

/**
 * A validated suggestion that has been tested and works
 */
export interface ValidatedSuggestion {
  provider: string;
  symbol: string;
  feedUrl?: string;
  /** How the suggestion was found */
  source: string;
  /** Validated price from actual fetch */
  validatedPrice: number;
}

/**
 * A security that needs attention (no feed, broken feed, stale prices, etc.)
 */
export interface QuoteManagerItem {
  securityId: number;
  securityName: string;
  isin?: string;
  ticker?: string;
  currency?: string;
  /** Current feed (if any) */
  currentFeed?: string;
  currentFeedUrl?: string;
  /** Problem status: "unconfigured", "error", "stale", "no_data" */
  status: 'unconfigured' | 'error' | 'stale' | 'no_data';
  statusMessage: string;
  /** Last known price date */
  lastPriceDate?: string;
  /** Days since last price */
  daysSincePrice?: number;
  /** Validated fix suggestions (already tested to work) */
  suggestions: ValidatedSuggestion[];
}

/**
 * Summary result from the unified quote manager
 */
export interface QuoteManagerResult {
  totalSecurities: number;
  totalWithIssues: number;
  unconfiguredCount: number;
  errorCount: number;
  staleCount: number;
  noDataCount: number;
  /** All securities with issues and their validated suggestions */
  items: QuoteManagerItem[];
}

/**
 * Unified quote manager - finds all problematic securities and provides validated fix suggestions.
 * This combines suggest + audit + validate all in one call.
 * @param onlyHeld If true, only check securities held in portfolios
 */
export async function quoteManagerAudit(onlyHeld?: boolean): Promise<QuoteManagerResult> {
  return invoke<QuoteManagerResult>('quote_manager_audit', { onlyHeld });
}

/**
 * Apply a suggestion from the quote manager.
 * @param securityId ID of the security
 * @param provider Provider name (e.g., "YAHOO")
 * @param symbol Symbol to use
 * @param feedUrl Optional feed URL/suffix
 */
export async function applyQuoteManagerSuggestion(
  securityId: number,
  provider: string,
  symbol: string,
  feedUrl?: string
): Promise<void> {
  return invoke<void>('apply_quote_manager_suggestion', { securityId, provider, symbol, feedUrl });
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
 * Information about a single price outlier.
 */
export interface OutlierInfo {
  date: string;
  value: number;
  previousValue: number;
  changePercent: number;
}

/**
 * Summary of outliers detected in price data.
 */
export interface OutlierSummary {
  /** Total number of price points */
  totalPrices: number;
  /** Number of detected outliers */
  outlierCount: number;
  /** List of outlier details */
  outliers: OutlierInfo[];
  /** Whether data quality is good (few outliers) */
  dataQualityGood: boolean;
}

/**
 * Price data point with outlier detection.
 */
export interface PriceDataWithOutliers {
  date: string;
  value: number;
  /** Whether this price is detected as an outlier (>75% daily change) */
  isOutlier: boolean;
  /** Percentage change from previous day */
  changePercent: number | null;
}

/**
 * Price history with outlier analysis.
 */
export interface PriceHistoryWithOutliers {
  prices: PriceDataWithOutliers[];
  summary: OutlierSummary;
}

/**
 * Filtered price history (outliers removed).
 */
export interface FilteredPriceHistory {
  prices: PriceData[];
  summary: OutlierSummary;
}

/**
 * Get price history with outlier detection.
 * Use this for charts where you want to visually mark outliers.
 *
 * @param securityId The security ID
 * @param startDate Optional start date (ISO format)
 * @param endDate Optional end date (ISO format)
 * @returns Prices with outlier flags and a summary
 */
export async function getPriceHistoryWithOutliers(
  securityId: number,
  startDate?: string,
  endDate?: string
): Promise<PriceHistoryWithOutliers> {
  return invoke<PriceHistoryWithOutliers>('get_price_history_with_outliers', {
    securityId,
    startDate,
    endDate,
  });
}

/**
 * Get filtered price history (outliers removed).
 * Use this for performance calculations and analysis where outliers would distort results.
 *
 * @param securityId The security ID
 * @param startDate Optional start date (ISO format)
 * @param endDate Optional end date (ISO format)
 * @returns Filtered prices plus a summary of what was removed
 */
export async function getPriceHistoryFiltered(
  securityId: number,
  startDate?: string,
  endDate?: string
): Promise<FilteredPriceHistory> {
  return invoke<FilteredPriceHistory>('get_price_history_filtered', {
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
 * Result from creating a security with historical data fetch.
 */
export interface SecurityWithHistoryResult {
  security: SecurityResult;
  historyFetched: boolean;
  quotesCount: number;
  oldestDate: string | null;
  newestDate: string | null;
  error: string | null;
}

/**
 * Create a new security and automatically fetch historical prices from Yahoo Finance.
 * This is ideal for stocks and ETFs where you want historical data immediately.
 *
 * @param data Security creation data (same as createSecurity)
 * @param historyYears Number of years of historical data to fetch (default: 10)
 * @returns The created security plus historical data fetch results
 */
export async function createSecurityWithHistory(
  data: CreateSecurityRequest,
  historyYears?: number
): Promise<SecurityWithHistoryResult> {
  return invoke<SecurityWithHistoryResult>('create_security_with_history', { data, historyYears });
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

/**
 * Risk metrics (Sharpe, Sortino, Drawdown, Volatility, Beta/Alpha)
 */
export interface RiskMetrics {
  sharpeRatio: number;
  sortinoRatio: number;
  maxDrawdown: number;
  maxDrawdownStart: string | null;
  maxDrawdownEnd: string | null;
  volatility: number;
  beta: number | null;
  alpha: number | null;
  calmarRatio: number | null;
  dataPoints: number;
}

/**
 * Calculate risk metrics for a portfolio.
 */
export async function calculateRiskMetrics(options?: {
  portfolioId?: number;
  startDate?: string;
  endDate?: string;
  benchmarkId?: number;
  riskFreeRate?: number;
}): Promise<RiskMetrics> {
  return invoke<RiskMetrics>('calculate_risk_metrics', options ?? {});
}

// ============================================================================
// Portfolio Optimization API (Markowitz)
// ============================================================================

/**
 * Security info for optimization
 */
export interface OptimizationSecurityInfo {
  id: number;
  name: string;
  ticker: string | null;
}

/**
 * Correlation pair between two securities
 */
export interface CorrelationPair {
  security1Id: number;
  security1Name: string;
  security2Id: number;
  security2Name: string;
  correlation: number;
}

/**
 * Full correlation matrix result
 */
export interface CorrelationMatrix {
  securities: OptimizationSecurityInfo[];
  matrix: number[][];
  pairs: CorrelationPair[];
}

/**
 * A point on the efficient frontier
 */
export interface EfficientFrontierPoint {
  expectedReturn: number;
  volatility: number;
  sharpeRatio: number;
  weights: Record<number, number>;
}

/**
 * Efficient frontier result
 */
export interface EfficientFrontier {
  points: EfficientFrontierPoint[];
  minVariancePortfolio: EfficientFrontierPoint;
  maxSharpePortfolio: EfficientFrontierPoint;
  currentPortfolio: EfficientFrontierPoint;
  securities: OptimizationSecurityInfo[];
}

/**
 * Calculate correlation matrix for portfolio holdings.
 */
export async function calculateCorrelationMatrix(options?: {
  portfolioId?: number;
  startDate?: string;
  endDate?: string;
}): Promise<CorrelationMatrix> {
  return invoke<CorrelationMatrix>('calculate_correlation_matrix', options ?? {});
}

/**
 * Calculate efficient frontier for portfolio.
 */
export async function calculateEfficientFrontier(options?: {
  portfolioId?: number;
  startDate?: string;
  endDate?: string;
  riskFreeRate?: number;
  numPoints?: number;
}): Promise<EfficientFrontier> {
  return invoke<EfficientFrontier>('calculate_efficient_frontier', options ?? {});
}

/**
 * Get optimal portfolio weights for target return.
 */
export async function getOptimalWeights(options: {
  targetReturn: number;
  portfolioId?: number;
  startDate?: string;
  endDate?: string;
}): Promise<Record<number, number>> {
  return invoke<Record<number, number>>('get_optimal_weights', options);
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

/**
 * Detect broker format from CSV file headers.
 * @param path CSV file path
 */
export async function detectCsvBroker(path: string): Promise<BrokerDetectionResult> {
  return invoke<BrokerDetectionResult>('detect_csv_broker', { path });
}

/**
 * Get list of available broker templates.
 */
export async function getBrokerTemplates(): Promise<BrokerTemplateSummary[]> {
  return invoke<BrokerTemplateSummary[]>('get_broker_templates');
}

/**
 * Import transactions using a broker template.
 * @param path CSV file path
 * @param templateId Broker template ID
 * @param portfolioId Target portfolio ID
 */
export async function importCsvWithTemplate(
  path: string,
  templateId: string,
  portfolioId: number
): Promise<CsvImportResult> {
  return invoke<CsvImportResult>('import_csv_with_template', {
    path,
    templateId,
    portfolioId,
  });
}

/**
 * Analyze CSV with AI assistance (Code-first, AI fallback).
 * Use this when automatic detection fails or has low confidence.
 * @param csvContent First ~20 lines of CSV for analysis
 * @param provider AI provider (claude, openai, gemini, perplexity)
 * @param model AI model ID
 * @param apiKey API key for the provider
 */
export async function analyzeCsvWithAi(
  csvContent: string,
  provider: string,
  model: string,
  apiKey: string
): Promise<AiCsvAnalysisResponse> {
  return invoke<AiCsvAnalysisResponse>('analyze_csv_with_ai', {
    csvContent,
    provider,
    model,
    apiKey,
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
 * Get all security classifications for a specific taxonomy.
 * Used for grouping in asset statement view.
 */
export async function getAllSecurityClassifications(taxonomyId: number): Promise<SecurityClassification[]> {
  return invoke<SecurityClassification[]>('get_all_security_classifications', { taxonomyId });
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
// Monthly/Yearly Returns API (Heatmap Widget)
// ============================================================================

// Types are defined in types.ts
import type { MonthlyReturn, YearlyReturn } from './types';
export type { MonthlyReturn, YearlyReturn };

/**
 * Get monthly returns for heatmap visualization.
 * @param portfolioId Optional portfolio filter
 * @param years Optional years to include
 */
export async function getMonthlyReturns(
  portfolioId?: number,
  years?: number[]
): Promise<MonthlyReturn[]> {
  return invoke<MonthlyReturn[]>('get_monthly_returns', { portfolioId, years });
}

/**
 * Get yearly returns for year returns widget.
 * @param portfolioId Optional portfolio filter
 */
export async function getYearlyReturns(portfolioId?: number): Promise<YearlyReturn[]> {
  return invoke<YearlyReturn[]>('get_yearly_returns', { portfolioId });
}

// ============================================================================
// Dividend Calendar & Forecast API
// ============================================================================

/** A single dividend event for the calendar */
export interface CalendarDividend {
  date: string;
  securityId: number;
  securityName: string;
  securityIsin?: string;
  amount: number;
  currency: string;
  isEstimated: boolean;
}

/** Calendar data for a month */
export interface MonthCalendarData {
  year: number;
  month: number;
  totalAmount: number;
  currency: string;
  dividends: CalendarDividend[];
}

/** Dividend payment pattern for a security */
export interface DividendPattern {
  securityId: number;
  securityName: string;
  securityIsin?: string;
  /** Payment frequency: MONTHLY, QUARTERLY, SEMI_ANNUAL, ANNUAL, IRREGULAR */
  frequency: string;
  /** Typical payment months (1-12) */
  paymentMonths: number[];
  /** Average dividend per share */
  avgPerShare: number;
  /** Last 4 dividends per share for trend */
  recentAmounts: number[];
  /** Growth rate (year over year) */
  growthRate?: number;
  currency: string;
}

/** Expected payment in forecast */
export interface ExpectedPayment {
  month: number;
  estimatedAmount: number;
  isReceived: boolean;
  actualAmount?: number;
}

/** Per-security forecast */
export interface SecurityForecast {
  securityId: number;
  securityName: string;
  securityIsin?: string;
  pattern: DividendPattern;
  /** Current shares held */
  sharesHeld: number;
  /** Estimated annual dividends */
  estimatedAnnual: number;
  /** Expected payments this year */
  expectedPayments: ExpectedPayment[];
}

/** Monthly forecast */
export interface MonthForecast {
  month: number;
  monthName: string;
  estimated: number;
  received: number;
  isPast: boolean;
}

/** Annual dividend forecast */
export interface DividendForecast {
  year: number;
  currency: string;
  totalEstimated: number;
  totalReceived: number;
  totalRemaining: number;
  byMonth: MonthForecast[];
  bySecurity: SecurityForecast[];
}

/**
 * Get dividend calendar for a specific year/month.
 * @param year Year to get calendar for
 * @param month Optional month (1-12) for single month view
 */
export async function getDividendCalendar(
  year: number,
  month?: number
): Promise<MonthCalendarData[]> {
  return invoke<MonthCalendarData[]>('get_dividend_calendar', { year, month });
}

/**
 * Get dividend patterns for all securities with dividend history.
 */
export async function getDividendPatterns(): Promise<DividendPattern[]> {
  return invoke<DividendPattern[]>('get_dividend_patterns');
}

/**
 * Estimate annual dividends based on historical patterns.
 * @param year Year to forecast (defaults to current year)
 */
export async function estimateAnnualDividends(year?: number): Promise<DividendForecast> {
  return invoke<DividendForecast>('estimate_annual_dividends', { year });
}

/**
 * Get portfolio-wide dividend yield (trailing 12 months).
 */
export async function getPortfolioDividendYield(): Promise<number> {
  return invoke<number>('get_portfolio_dividend_yield');
}

// ============================================================================
// Ex-Dividend Management API
// ============================================================================

/** Ex-dividend entry with security details */
export interface ExDividend {
  id: number;
  securityId: number;
  securityName: string;
  securityIsin?: string;
  exDate: string;
  recordDate?: string;
  payDate?: string;
  amount?: number;
  currency?: string;
  frequency?: string;
  source?: string;
  isConfirmed: boolean;
  note?: string;
  createdAt: string;
}

/** Request to create/update an ex-dividend entry */
export interface ExDividendRequest {
  securityId: number;
  exDate: string;
  recordDate?: string;
  payDate?: string;
  amount?: number;
  currency?: string;
  frequency?: string;
  source?: string;
  isConfirmed?: boolean;
  note?: string;
}

/** A calendar event (ex-dividend, record date, or payment) */
export interface DividendCalendarEvent {
  date: string;
  /** Event type: "ex_dividend", "record_date", "payment" */
  eventType: string;
  securityId: number;
  securityName: string;
  securityIsin?: string;
  amount?: number;
  currency?: string;
  isConfirmed: boolean;
  relatedExDate?: string;
}

/** Enhanced calendar data combining ex-dividends with payments */
export interface EnhancedMonthCalendarData {
  year: number;
  month: number;
  events: DividendCalendarEvent[];
  totalExDividends: number;
  totalPayments: number;
}

/**
 * Get ex-dividend entries for a date range.
 * @param startDate Start of date range (ISO format)
 * @param endDate End of date range (ISO format)
 * @param securityId Optional filter by security
 */
export async function getExDividends(
  startDate: string,
  endDate: string,
  securityId?: number
): Promise<ExDividend[]> {
  return invoke<ExDividend[]>('get_ex_dividends', { startDate, endDate, securityId });
}

/**
 * Create a new ex-dividend entry.
 * @param request Ex-dividend data
 */
export async function createExDividend(request: ExDividendRequest): Promise<ExDividend> {
  return invoke<ExDividend>('create_ex_dividend', { request });
}

/**
 * Update an existing ex-dividend entry.
 * @param id Ex-dividend ID
 * @param request Updated data
 */
export async function updateExDividend(id: number, request: ExDividendRequest): Promise<ExDividend> {
  return invoke<ExDividend>('update_ex_dividend', { id, request });
}

/**
 * Delete an ex-dividend entry.
 * @param id Ex-dividend ID
 */
export async function deleteExDividend(id: number): Promise<void> {
  return invoke<void>('delete_ex_dividend', { id });
}

/**
 * Get upcoming ex-dividend dates for held securities.
 * @param days Number of days to look ahead (default: 30)
 */
export async function getUpcomingExDividends(days?: number): Promise<ExDividend[]> {
  return invoke<ExDividend[]>('get_upcoming_ex_dividends', { days });
}

/**
 * Get enhanced dividend calendar combining ex-dividends with payment dates.
 * @param year Year to get calendar for
 * @param month Optional month (1-12)
 */
export async function getEnhancedDividendCalendar(
  year: number,
  month?: number
): Promise<EnhancedMonthCalendarData[]> {
  return invoke<EnhancedMonthCalendarData[]>('get_enhanced_dividend_calendar', { year, month });
}

// ============================================================================
// German Tax API (DE)
// ============================================================================

/** Tax settings for a year */
export interface TaxSettings {
  year: number;
  isMarried: boolean;
  kirchensteuerRate?: number;
  bundesland?: string;
  freistellungLimit: number;
  freistellungUsed: number;
}

/** Individual taxable item */
export interface TaxableItem {
  date: string;
  securityName: string;
  securityIsin?: string;
  grossAmount: number;
  withholdingTax: number;
  netAmount: number;
  itemType: string;
}

/** Data for German tax form "Anlage KAP" */
export interface AnlageKapData {
  zeile7InlandDividenden: number;
  zeile8AuslandDividenden: number;
  zeile14Zinsen: number;
  zeile15Veraeusserungsgewinne: number;
  zeile16Veraeusserungsverluste: number;
  zeile47AuslaendischeSteuern: number;
  zeile48Kapest: number;
  zeile49Soli: number;
  zeile50Kist: number;
}

/** Detailed German tax report */
export interface GermanTaxReport {
  year: number;
  currency: string;
  settings: TaxSettings;
  dividendIncomeGross: number;
  interestIncomeGross: number;
  realizedGains: number;
  realizedLosses: number;
  totalTaxableIncome: number;
  freistellungAvailable: number;
  freistellungUsed: number;
  lossCarryforward: number;
  taxableAfterDeductions: number;
  foreignWithholdingTax: number;
  creditableForeignTax: number;
  abgeltungssteuer: number;
  solidaritaetszuschlag: number;
  kirchensteuer: number;
  totalGermanTax: number;
  taxAlreadyPaid: number;
  remainingTaxLiability: number;
  dividendDetails: TaxableItem[];
  gainsDetails: TaxableItem[];
  lossesDetails: TaxableItem[];
  anlageKap: AnlageKapData;
}

/** Freistellung status */
export interface FreistellungStatus {
  year: number;
  limit: number;
  used: number;
  remaining: number;
  isMarried: boolean;
  usagePercent: number;
}

/**
 * Get tax settings for a year.
 */
export async function getTaxSettings(year: number): Promise<TaxSettings> {
  return invoke<TaxSettings>('get_tax_settings', { year });
}

/**
 * Save tax settings for a year.
 */
export async function saveTaxSettings(settings: TaxSettings): Promise<void> {
  return invoke('save_tax_settings', { settings });
}

/**
 * Generate detailed German tax report for a year.
 */
export async function generateGermanTaxReport(year: number): Promise<GermanTaxReport> {
  return invoke<GermanTaxReport>('generate_german_tax_report', { year });
}

/**
 * Get Freistellung status for a year.
 */
export async function getFreistellungStatus(year: number): Promise<FreistellungStatus> {
  return invoke<FreistellungStatus>('get_freistellung_status', { year });
}

/**
 * Update Freistellung used amount.
 */
export async function updateFreistellungUsed(year: number, amount: number): Promise<void> {
  return invoke('update_freistellung_used', { year, amount });
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
// Merger & Acquisition API
// ============================================================================

/**
 * Request for applying a merger/acquisition.
 */
export interface ApplyMergerRequest {
  sourceSecurityId: number;
  targetSecurityId: number;
  effectiveDate: string;
  shareRatio: number;      // target shares per source share
  cashPerShare: number;    // cash component per source share (in cents)
  cashCurrency?: string;
  note?: string;
}

/**
 * Information about an affected portfolio in a merger.
 */
export interface MergerAffectedPortfolio {
  portfolioId: number;
  portfolioName: string;
  sourceShares: number;    // scaled 10^8
  targetShares: number;    // scaled 10^8
  cashAmount: number;      // cents
  costBasisTransferred: number;  // cents
}

/**
 * Preview of a merger's effects.
 */
export interface MergerPreview {
  sourceSecurityId: number;
  sourceSecurityName: string;
  targetSecurityId: number;
  targetSecurityName: string;
  effectiveDate: string;
  shareRatio: number;
  cashPerShare: number;
  cashCurrency: string;
  totalSourceShares: number;      // scaled 10^8
  totalTargetShares: number;      // scaled 10^8
  totalCashAmount: number;        // cents
  totalCostBasisTransferred: number;  // cents
  affectedPortfolios: MergerAffectedPortfolio[];
}

/**
 * Preview the effect of a merger/acquisition.
 */
export async function previewMerger(
  sourceSecurityId: number,
  targetSecurityId: number,
  effectiveDate: string,
  shareRatio: number,
  cashPerShare: number,
  cashCurrency?: string
): Promise<MergerPreview> {
  return invoke<MergerPreview>('preview_merger', {
    sourceSecurityId,
    targetSecurityId,
    effectiveDate,
    shareRatio,
    cashPerShare,
    cashCurrency,
  });
}

/**
 * Apply a merger/acquisition corporate action.
 * Creates DELIVERY_OUTBOUND for source shares and DELIVERY_INBOUND for target shares.
 * Optionally creates dividend transaction for cash component.
 */
export async function applyMerger(request: ApplyMergerRequest): Promise<CorporateActionResult> {
  return invoke<CorporateActionResult>('apply_merger', { request });
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
  AllocationTarget,
  SetAllocationTargetRequest,
  AllocationAlert,
  AllocationAlertCount,
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

// ============================================================================
// Allocation Alerts
// ============================================================================

/**
 * Get all allocation targets for a portfolio.
 */
export async function getAllocationTargets(portfolioId: number): Promise<AllocationTarget[]> {
  return invoke<AllocationTarget[]>('get_allocation_targets', { portfolioId });
}

/**
 * Set (create or update) an allocation target.
 */
export async function setAllocationTarget(request: SetAllocationTargetRequest): Promise<number> {
  return invoke<number>('set_allocation_target', { request });
}

/**
 * Delete an allocation target.
 */
export async function deleteAllocationTarget(id: number): Promise<void> {
  return invoke('delete_allocation_target', { id });
}

/**
 * Get allocation alerts for a portfolio or all portfolios.
 */
export async function getAllocationAlerts(portfolioId?: number): Promise<AllocationAlert[]> {
  return invoke<AllocationAlert[]>('get_allocation_alerts', { portfolioId });
}

/**
 * Get the count of active allocation alerts (for badge display).
 */
export async function getAllocationAlertCount(portfolioId?: number): Promise<AllocationAlertCount> {
  return invoke<AllocationAlertCount>('get_allocation_alert_count', { portfolioId });
}

// ============================================================================
// Dashboard Widget System
// ============================================================================

import type {
  DashboardLayout,
  WidgetDefinition,
} from '../components/dashboard/types';

/**
 * Get available widget definitions for the catalog.
 */
export async function getAvailableWidgets(): Promise<WidgetDefinition[]> {
  return invoke<WidgetDefinition[]>('get_available_widgets');
}

/**
 * Get a dashboard layout by ID, or the default layout if no ID provided.
 */
export async function getDashboardLayout(layoutId?: number): Promise<DashboardLayout | null> {
  return invoke<DashboardLayout | null>('get_dashboard_layout', { layoutId });
}

/**
 * Save a dashboard layout (creates new if id=0, updates if id>0).
 */
export async function saveDashboardLayout(layout: DashboardLayout): Promise<number> {
  return invoke<number>('save_dashboard_layout', { layout });
}

/**
 * Delete a dashboard layout.
 */
export async function deleteDashboardLayout(layoutId: number): Promise<void> {
  return invoke('delete_dashboard_layout', { layoutId });
}

/**
 * Get all dashboard layouts.
 */
export async function getAllDashboardLayouts(): Promise<DashboardLayout[]> {
  return invoke<DashboardLayout[]>('get_all_dashboard_layouts');
}

/**
 * Create the default dashboard layout.
 */
export async function createDefaultDashboardLayout(): Promise<DashboardLayout> {
  return invoke<DashboardLayout>('create_default_dashboard_layout');
}

// ============================================================================
// Custom Attributes API
// ============================================================================

/**
 * Get all attribute type definitions.
 * @param target - Optional filter by target entity (security, account, portfolio)
 */
export async function getAttributeTypes(target?: 'security' | 'account' | 'portfolio'): Promise<AttributeType[]> {
  return invoke<AttributeType[]>('get_attribute_types', { target });
}

/**
 * Create a new attribute type.
 */
export async function createAttributeType(request: CreateAttributeTypeRequest): Promise<AttributeType> {
  return invoke<AttributeType>('create_attribute_type', { request });
}

/**
 * Update an existing attribute type.
 */
export async function updateAttributeType(id: number, request: UpdateAttributeTypeRequest): Promise<AttributeType> {
  return invoke<AttributeType>('update_attribute_type', { id, request });
}

/**
 * Delete an attribute type.
 */
export async function deleteAttributeType(id: number): Promise<void> {
  return invoke('delete_attribute_type', { id });
}

/**
 * Get all attribute values for a security.
 */
export async function getSecurityAttributes(securityId: number): Promise<AttributeValue[]> {
  return invoke<AttributeValue[]>('get_security_attributes', { securityId });
}

/**
 * Set an attribute value for a security.
 */
export async function setSecurityAttribute(request: SetAttributeValueRequest): Promise<void> {
  return invoke('set_security_attribute', { request });
}

/**
 * Remove an attribute value from a security.
 */
export async function removeSecurityAttribute(securityId: number, attributeTypeId: number): Promise<void> {
  return invoke('remove_security_attribute', { securityId, attributeTypeId });
}

/**
 * Get all securities with their values for a specific attribute type.
 */
export async function getSecuritiesByAttribute(attributeTypeId: number): Promise<SecurityWithAttribute[]> {
  const data = await invoke<[number, string, string | null][]>('get_securities_by_attribute', { attributeTypeId });
  return data.map(([securityId, securityName, value]) => ({
    securityId,
    securityName,
    value: value ?? undefined,
  }));
}

// ============================================================================
// Consortium (Portfolio Groups) API
// ============================================================================

import type {
  Consortium,
  CreateConsortiumRequest,
  ConsortiumPerformance,
  PortfolioComparison,
  ConsortiumHistory,
} from './types';

/**
 * Get all consortiums (portfolio groups).
 */
export async function getConsortiums(): Promise<Consortium[]> {
  return invoke<Consortium[]>('get_consortiums');
}

/**
 * Create a new consortium.
 */
export async function createConsortium(request: CreateConsortiumRequest): Promise<Consortium> {
  return invoke<Consortium>('create_consortium', { request });
}

/**
 * Update an existing consortium.
 */
export async function updateConsortium(id: number, request: CreateConsortiumRequest): Promise<Consortium> {
  return invoke<Consortium>('update_consortium', { id, request });
}

/**
 * Delete a consortium.
 */
export async function deleteConsortium(id: number): Promise<void> {
  return invoke('delete_consortium', { id });
}

/**
 * Get combined performance metrics for a consortium.
 * Includes TTWROR, IRR, risk metrics, and per-portfolio breakdown.
 */
export async function getConsortiumPerformance(consortiumId: number): Promise<ConsortiumPerformance> {
  return invoke<ConsortiumPerformance>('get_consortium_performance', { consortiumId });
}

/**
 * Compare multiple portfolios side-by-side.
 */
export async function comparePortfolios(portfolioIds: number[]): Promise<PortfolioComparison> {
  return invoke<PortfolioComparison>('compare_portfolios', { portfolioIds });
}

/**
 * Get historical performance data for a consortium (for charts).
 */
export async function getConsortiumHistory(
  consortiumId: number,
  startDate?: string,
  endDate?: string
): Promise<ConsortiumHistory> {
  return invoke<ConsortiumHistory>('get_consortium_history', { consortiumId, startDate, endDate });
}

// ============================================================================
// Symbol Validation API
// ============================================================================

import type {
  ValidationResponse,
  ValidationResult,
  ValidationStatusSummary,
  ValidateSecuritiesRequest,
  ValidateSingleRequest,
  ApplyValidationRequest,
} from './types';

/**
 * Validate all securities' quote configurations.
 * @param request Validation request with options
 */
export async function validateAllSecurities(request: ValidateSecuritiesRequest): Promise<ValidationResponse> {
  return invoke<ValidationResponse>('validate_all_securities_cmd', { request });
}

/**
 * Validate a single security's quote configuration.
 * @param request Validation request for single security
 */
export async function validateSecurity(request: ValidateSingleRequest): Promise<ValidationResult> {
  return invoke<ValidationResult>('validate_security_cmd', { request });
}

/**
 * Apply a validated configuration to a security.
 * Updates the security's feed, ticker, and feed_url.
 * @param request The security ID and validated config to apply
 */
export async function applyValidationResult(request: ApplyValidationRequest): Promise<void> {
  return invoke('apply_validation_result_cmd', { request });
}

/**
 * Get validation status summary.
 * @param onlyHeld Only include held securities in the summary
 */
export async function getValidationStatus(onlyHeld: boolean): Promise<ValidationStatusSummary> {
  return invoke<ValidationStatusSummary>('get_validation_status_cmd', { onlyHeld });
}

// ============================================================================
// Chart Drawings API
// ============================================================================

/** Point data for chart drawings */
export interface ChartDrawingPoint {
  x: number;
  y: number;
  time?: string;
  price?: number;
}

/** Input for saving a chart drawing */
export interface ChartDrawingInput {
  securityId: number;
  drawingType: string;
  points: ChartDrawingPoint[];
  color: string;
  lineWidth: number;
  fibLevels?: number[];
}

/** Response from chart drawing operations */
export interface ChartDrawingResponse {
  id: string;
  uuid: string;
  securityId: number;
  drawingType: string;
  points: ChartDrawingPoint[];
  color: string;
  lineWidth: number;
  fibLevels?: number[];
  isVisible: boolean;
  createdAt: string;
}

/**
 * Save a chart drawing to the database.
 * @param drawing The drawing data to save
 */
export async function saveChartDrawing(drawing: ChartDrawingInput): Promise<ChartDrawingResponse> {
  return invoke<ChartDrawingResponse>('save_chart_drawing', { drawing });
}

/**
 * Get all drawings for a security.
 * @param securityId The security ID
 */
export async function getChartDrawings(securityId: number): Promise<ChartDrawingResponse[]> {
  return invoke<ChartDrawingResponse[]>('get_chart_drawings', { securityId });
}

/**
 * Delete a specific chart drawing.
 * @param drawingId The drawing ID to delete
 */
export async function deleteChartDrawing(drawingId: number): Promise<void> {
  return invoke('delete_chart_drawing', { drawingId });
}

/**
 * Clear all drawings for a security.
 * @param securityId The security ID
 */
export async function clearChartDrawings(securityId: number): Promise<void> {
  return invoke('clear_chart_drawings', { securityId });
}

// ============================================================================
// Pattern Statistics API
// ============================================================================

/** Input for saving a detected pattern */
export interface PatternDetectionInput {
  securityId: number;
  patternType: string;
  detectedAt: string;
  priceAtDetection: number;
  predictedDirection: 'bullish' | 'bearish' | 'neutral';
}

/** Pattern history entry */
export interface PatternHistory {
  id: number;
  securityId: number;
  patternType: string;
  detectedAt: string;
  priceAtDetection: number;
  predictedDirection: string;
  actualOutcome?: string;
  priceAfter5d?: number;
  priceAfter10d?: number;
  priceChange5dPercent?: number;
  priceChange10dPercent?: number;
  evaluatedAt?: string;
  createdAt: string;
}

/** Statistics for a pattern type */
export interface PatternStatistics {
  patternType: string;
  totalCount: number;
  successCount: number;
  failureCount: number;
  pendingCount: number;
  successRate: number;
  avgGainOnSuccess?: number;
  avgLossOnFailure?: number;
}

/** Result of pattern evaluation */
export interface PatternEvaluationResult {
  patternsEvaluated: number;
  successes: number;
  failures: number;
}

/**
 * Save a detected pattern to the database for tracking.
 * @param pattern The pattern detection data
 */
export async function savePatternDetection(pattern: PatternDetectionInput): Promise<number> {
  return invoke<number>('save_pattern_detection', { pattern });
}

/**
 * Evaluate pending patterns that are old enough (5+ days).
 * Compares the price at detection with later prices to determine success.
 */
export async function evaluatePatternOutcomes(): Promise<PatternEvaluationResult> {
  return invoke<PatternEvaluationResult>('evaluate_pattern_outcomes');
}

/**
 * Get statistics for all pattern types.
 */
export async function getPatternStatistics(): Promise<PatternStatistics[]> {
  return invoke<PatternStatistics[]>('get_pattern_statistics');
}

/**
 * Get pattern history for a specific security.
 * @param securityId The security ID
 */
export async function getPatternHistory(securityId: number): Promise<PatternHistory[]> {
  return invoke<PatternHistory[]>('get_pattern_history', { securityId });
}

// ============================================================================
// User Profile API
// ============================================================================

/**
 * Set the user's profile picture.
 * @param pictureBase64 Base64 encoded image data, or null to remove
 */
export async function setUserProfilePicture(pictureBase64: string | null): Promise<void> {
  return invoke('set_user_profile_picture', { pictureBase64 });
}

/**
 * Get the user's profile picture.
 * @returns Base64 encoded image data, or null if not set
 */
export async function getUserProfilePicture(): Promise<string | null> {
  return invoke<string | null>('get_user_profile_picture');
}

// ============================================================================
// User-defined Query Templates API
// ============================================================================

import type {
  UserTemplate,
  UserTemplateInput,
  UserTemplateTestResult,
} from './types';

/**
 * Get all user-defined query templates.
 */
export async function getUserTemplates(): Promise<UserTemplate[]> {
  return invoke<UserTemplate[]>('get_user_templates');
}

/**
 * Create a new user-defined query template.
 */
export async function createUserTemplate(template: UserTemplateInput): Promise<UserTemplate> {
  return invoke<UserTemplate>('create_user_template', { template });
}

/**
 * Update an existing user-defined query template.
 */
export async function updateUserTemplate(id: number, template: UserTemplateInput): Promise<UserTemplate> {
  return invoke<UserTemplate>('update_user_template', { id, template });
}

/**
 * Delete a user-defined query template.
 */
export async function deleteUserTemplate(id: number): Promise<void> {
  return invoke('delete_user_template', { id });
}

/**
 * Test a user-defined query template without saving it.
 * Validates SQL and returns preview results.
 */
export async function testUserTemplate(template: UserTemplateInput): Promise<UserTemplateTestResult> {
  return invoke<UserTemplateTestResult>('test_user_template', { template });
}

/**
 * Get the system prompt section including user-defined templates.
 */
export async function getQueryTemplatesPrompt(): Promise<string> {
  return invoke<string>('get_query_templates_prompt');
}
