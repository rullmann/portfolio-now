/**
 * TypeScript types for the Portfolio Performance import API.
 * These types match the Rust command interfaces.
 */

// ============================================================================
// Import Types
// ============================================================================

export interface ImportProgress {
  stage: string;
  message: string;
  percent: number;
  current?: number;
  total?: number;
}

export interface ImportResult {
  importId: number;
  filePath: string;
  version: number;
  baseCurrency: string;
  securitiesCount: number;
  accountsCount: number;
  portfoliosCount: number;
  transactionsCount: number;
  pricesCount: number;
  warnings: string[];
}

// Go sidecar import result
export interface GoImportResult {
  success: boolean;
  message?: string;
  error?: string;
  outputPath?: string;
  duration?: string;
  stats?: GoImportStats;
}

export interface GoImportStats {
  securities: number;
  accounts: number;
  portfolios: number;
  transactions: number;
  prices: number;
  unresolvedRefs: number;
  errors?: string[];
}

export interface ImportInfo {
  id: number;
  filePath: string;
  importedAt: string;
  version: number;
  baseCurrency: string;
  securitiesCount: number;
  accountsCount: number;
  portfoliosCount: number;
  transactionsCount: number;
}

// ============================================================================
// Data Query Types
// ============================================================================

export interface SecurityData {
  id: number;
  uuid: string;
  name: string;
  currency: string;
  targetCurrency?: string;  // Target currency for conversion (PP field)
  isin?: string;
  wkn?: string;
  ticker?: string;
  feed?: string;           // Provider for historical quotes
  feedUrl?: string;        // URL/suffix for historical quotes
  latestFeed?: string;     // Provider for current quotes
  latestFeedUrl?: string;  // URL/suffix for current quotes
  isRetired: boolean;
  latestPrice?: number;
  latestPriceDate?: string;
  updatedAt?: string; // When the price was last fetched
  pricesCount: number;
  currentHoldings: number; // Total shares held across all portfolios
  customLogo?: string; // Base64-encoded custom logo
  note?: string;
  attributes?: Record<string, string>;  // Custom attributes (PP field)
  properties?: Record<string, string>;  // System properties (PP field)
}

export interface AccountData {
  id: number;
  uuid: string;
  name: string;
  currency: string;
  isRetired: boolean;
  transactionsCount: number;
  balance: number;
  note?: string;
  updatedAt?: string;
  attributes?: Record<string, string>;  // Custom attributes (PP field)
}

export interface PortfolioData {
  id: number;
  uuid: string;
  name: string;
  referenceAccountName: string | null;
  referenceAccountId?: number;
  isRetired: boolean;
  transactionsCount: number;
  holdingsCount: number;
  note?: string;
  updatedAt?: string;
  attributes?: Record<string, string>;  // Custom attributes (PP field)
}

export interface TransactionData {
  id: number;
  uuid: string;
  ownerType: string;
  ownerName: string;
  ownerId: number;
  txnType: string;
  date: string;
  amount: number;
  currency: string;
  shares?: number;
  securityId?: number;
  securityName?: string;
  securityUuid?: string;
  note?: string;
  fees: number;
  taxes: number;
  hasForex: boolean;
  source?: string;
  updatedAt?: string;
  // Transfer tracking (PP fields)
  otherAccountId?: number;
  otherAccountUuid?: string;
  otherPortfolioId?: number;
  otherPortfolioUuid?: string;
  otherUpdatedAt?: string;
}

export interface PriceData {
  date: string;
  value: number;
}

export interface HoldingData {
  securityId: number;
  securityUuid: string;
  securityName: string;
  currency: string;
  shares: number;
  currentPrice?: number;
  currentValue?: number;
  costBasis: number;
  gainLoss?: number;
  gainLossPercent?: number;
}

// ============================================================================
// FIFO Cost Basis History Types
// ============================================================================

/** Snapshot of FIFO cost basis at a point in time */
export interface FifoCostBasisSnapshot {
  date: string;
  shares: number;
  costPerShare: number;
  totalCostBasis: number;
}

/** Trade marker for chart visualization */
export interface TradeMarker {
  date: string;
  txnType: string;
  shares: number;
  pricePerShare: number;
  amount: number;
  fees: number;
  taxes: number;
}

/** Complete data for security detail chart */
export interface SecurityChartData {
  costBasisHistory: FifoCostBasisSnapshot[];
  trades: TradeMarker[];
}

export interface PortfolioSummary {
  totalSecurities: number;
  totalAccounts: number;
  totalPortfolios: number;
  totalTransactions: number;
  totalPrices: number;
  dateRange?: [string, string];
}

// ============================================================================
// Transaction Types (for display)
// ============================================================================

export type AccountTransactionType =
  | 'DEPOSIT'
  | 'REMOVAL'
  | 'INTEREST'
  | 'INTEREST_CHARGE'
  | 'DIVIDENDS'
  | 'FEES'
  | 'FEES_REFUND'
  | 'TAXES'
  | 'TAX_REFUND'
  | 'BUY'
  | 'SELL'
  | 'TRANSFER_IN'
  | 'TRANSFER_OUT';

export type PortfolioTransactionType =
  | 'BUY'
  | 'SELL'
  | 'TRANSFER_IN'
  | 'TRANSFER_OUT'
  | 'DELIVERY_INBOUND'
  | 'DELIVERY_OUTBOUND';

// ============================================================================
// Quote Sync Types
// ============================================================================

export interface QuoteSyncResult {
  total: number;
  success: number;
  errors: number;
  errorMessages: string[];
}

// ============================================================================
// Helper Functions
// ============================================================================

// Valid ISO 4217 currency codes
const VALID_CURRENCIES = new Set([
  'EUR', 'USD', 'GBP', 'CHF', 'JPY', 'CAD', 'AUD', 'NZD', 'SEK', 'NOK', 'DKK',
  'PLN', 'CZK', 'HUF', 'RON', 'BGN', 'HRK', 'RUB', 'TRY', 'ZAR', 'MXN', 'BRL',
  'INR', 'CNY', 'HKD', 'SGD', 'KRW', 'TWD', 'THB', 'IDR', 'MYR', 'PHP', 'VND',
  'AED', 'SAR', 'ILS', 'EGP', 'NGN', 'KES', 'GHS', 'XOF', 'XAF',
]);

export function formatCurrency(amount: number, currency: string = 'EUR'): string {
  // Validate currency code - must be 3 uppercase letters and a known currency
  const normalizedCurrency = (currency || 'EUR').toUpperCase().trim();
  const validCurrency = VALID_CURRENCIES.has(normalizedCurrency) ? normalizedCurrency : 'EUR';

  try {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency: validCurrency,
    }).format(amount);
  } catch {
    // Fallback if currency formatting fails
    return `${amount.toFixed(2)} ${validCurrency}`;
  }
}

export function formatNumber(value: number, decimals: number = 2): string {
  return new Intl.NumberFormat('de-DE', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value);
}

export function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  return new Intl.DateTimeFormat('de-DE', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).format(date);
}

export function formatDateTime(dateStr: string): string {
  const date = new Date(dateStr);
  return new Intl.DateTimeFormat('de-DE', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}

/**
 * Extrahiert das Datum (YYYY-MM-DD) aus einem Datetime-String.
 * Für HTML <input type="date">.
 */
export function extractDateForInput(dateStr: string | null | undefined): string {
  if (!dateStr) return new Date().toISOString().split('T')[0];
  // "2024-01-15 09:30:00" → "2024-01-15"
  // "2024-01-15T09:30:00" → "2024-01-15"
  const part = dateStr.split(' ')[0].split('T')[0];
  return part || new Date().toISOString().split('T')[0];
}

/**
 * Extrahiert die Uhrzeit (HH:MM) aus einem Datetime-String.
 * Für HTML <input type="time">.
 */
export function extractTimeForInput(dateStr: string | null | undefined): string {
  if (!dateStr) return '00:00';
  // "2024-01-15 09:30:00" → "09:30"
  const parts = dateStr.split(' ');
  if (parts.length >= 2) {
    return parts[1].substring(0, 5);
  }
  // ISO format: "2024-01-15T09:30:00"
  const tParts = dateStr.split('T');
  if (tParts.length >= 2) {
    return tParts[1].substring(0, 5);
  }
  return '00:00';
}

/**
 * Kombiniert Datum und Zeit zu einem Datetime-String.
 * Für Backend-Speicherung.
 */
export function combineDateAndTime(date: string, time: string): string {
  const timePart = time || '00:00';
  return `${date} ${timePart}:00`;
}

export function getTransactionTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    DEPOSIT: 'Einlage',
    REMOVAL: 'Entnahme',
    INTEREST: 'Zinsen',
    INTEREST_CHARGE: 'Zinsbelastung',
    DIVIDENDS: 'Dividende',
    FEES: 'Gebühren',
    FEES_REFUND: 'Gebührenerstattung',
    TAXES: 'Steuern',
    TAX_REFUND: 'Steuererstattung',
    BUY: 'Kauf',
    SELL: 'Verkauf',
    TRANSFER_IN: 'Umbuchung (Eingang)',
    TRANSFER_OUT: 'Umbuchung (Ausgang)',
    DELIVERY_INBOUND: 'Einlieferung',
    DELIVERY_OUTBOUND: 'Auslieferung',
  };
  return labels[type] || type;
}

export function isPositiveTransaction(type: string): boolean {
  const positive = ['DEPOSIT', 'INTEREST', 'DIVIDENDS', 'FEES_REFUND', 'TAX_REFUND', 'SELL', 'TRANSFER_IN'];
  return positive.includes(type);
}

// ============================================================================
// CRUD Types
// ============================================================================

export interface CreateSecurityRequest {
  name: string;
  currency: string;
  targetCurrency?: string;  // Target currency for conversion
  isin?: string;
  wkn?: string;
  ticker?: string;
  feed?: string;           // Provider for historical quotes
  feedUrl?: string;        // URL/suffix for historical quotes
  latestFeed?: string;     // Provider for current quotes
  latestFeedUrl?: string;  // URL/suffix for current quotes
  note?: string;
  attributes?: Record<string, string>;
  properties?: Record<string, string>;
}

export interface UpdateSecurityRequest {
  name?: string;
  currency?: string;
  targetCurrency?: string;  // Target currency for conversion
  isin?: string;
  wkn?: string;
  ticker?: string;
  feed?: string;           // Provider for historical quotes
  feedUrl?: string;        // URL/suffix for historical quotes
  latestFeed?: string;     // Provider for current quotes
  latestFeedUrl?: string;  // URL/suffix for current quotes
  note?: string;
  isRetired?: boolean;
  attributes?: Record<string, string>;
  properties?: Record<string, string>;
}

export interface SecurityResult {
  id: number;
  uuid: string;
  name: string;
  currency: string;
  isin?: string;
  wkn?: string;
  ticker?: string;
  feed?: string;
  feedUrl?: string;
  latestFeed?: string;
  latestFeedUrl?: string;
  note?: string;
  isRetired: boolean;
}

export interface CreateAccountRequest {
  name: string;
  currency: string;
  note?: string;
  attributes?: Record<string, string>;
}

export interface UpdateAccountRequest {
  name?: string;
  currency?: string;
  note?: string;
  isRetired?: boolean;
  attributes?: Record<string, string>;
}

export interface AccountResult {
  id: number;
  uuid: string;
  name: string;
  currency: string;
  note?: string;
  isRetired: boolean;
}

export interface CreatePortfolioRequest {
  name: string;
  referenceAccountId?: number;
  note?: string;
  attributes?: Record<string, string>;
}

export interface UpdatePortfolioRequest {
  name?: string;
  referenceAccountId?: number;
  note?: string;
  isRetired?: boolean;
  attributes?: Record<string, string>;
}

export interface PortfolioResult {
  id: number;
  uuid: string;
  name: string;
  referenceAccountId?: number;
  note?: string;
  isRetired: boolean;
}

// ============================================================================
// Transaction CRUD Types
// ============================================================================

export interface TransactionUnitData {
  unitType: string; // FEE, TAX, GROSS_VALUE
  amount: number;   // × 10²
  currency: string;
  forexAmount?: number;
  forexCurrency?: string;
  exchangeRate?: number;
}

export interface CreateTransactionRequest {
  ownerType: string;        // "account" | "portfolio"
  ownerId: number;
  txnType: string;          // BUY, SELL, DIVIDEND, etc.
  date: string;             // ISO date string
  amount: number;           // × 10²
  currency: string;
  shares?: number;          // × 10⁸
  securityId?: number;
  note?: string;
  units?: TransactionUnitData[];
  referenceAccountId?: number; // For portfolio BUY/SELL
  otherAccountId?: number;     // For TRANSFER_IN/OUT (account transfers)
  otherPortfolioId?: number;   // For TRANSFER_IN/OUT (portfolio transfers)
}

export interface TransactionResult {
  id: number;
  uuid: string;
  ownerType: string;
  ownerId: number;
  txnType: string;
  date: string;
  amount: number;
  currency: string;
  shares?: number;
  securityId?: number;
  note?: string;
  crossEntryId?: number;
}

// ============================================================================
// Performance Types
// ============================================================================

export interface PerformanceResult {
  /** TTWROR as percentage */
  ttwror: number;
  /** Annualized TTWROR as percentage */
  ttwrorAnnualized: number;
  /** IRR as percentage */
  irr: number;
  /** Whether IRR calculation converged */
  irrConverged: boolean;
  /** Number of days in the period */
  days: number;
  /** Start date */
  startDate: string;
  /** End date */
  endDate: string;
  /** Current portfolio value */
  currentValue: number;
  /** Total invested */
  totalInvested: number;
  /** Absolute gain/loss */
  absoluteGain: number;
}

export interface PeriodReturnData {
  startDate: string;
  endDate: string;
  startValue: number;
  endValue: number;
  cashFlow: number;
  returnRate: number;
}

// ============================================================================
// Currency Types
// ============================================================================

export interface ExchangeRateResult {
  base: string;
  target: string;
  rate: number;
  date: string;
}

export interface ConversionResult {
  originalAmount: number;
  originalCurrency: string;
  convertedAmount: number;
  targetCurrency: string;
  rate: number;
  date: string;
}

// ============================================================================
// CSV Import/Export Types
// ============================================================================

export interface CsvExportResult {
  path: string;
  rowsExported: number;
}

export interface CsvColumn {
  index: number;
  name: string;
  sampleValues: string[];
}

export interface CsvPreview {
  columns: CsvColumn[];
  rowCount: number;
  delimiter: string;
}

export interface CsvColumnMapping {
  date?: number;
  txnType?: number;
  securityName?: number;
  isin?: number;
  shares?: number;
  amount?: number;
  currency?: number;
  fees?: number;
  taxes?: number;
  note?: number;
}

export interface CsvImportResult {
  rowsImported: number;
  rowsSkipped: number;
  errors: string[];
}

// ============================================================================
// Taxonomy Types
// ============================================================================

export interface TaxonomyData {
  id: number;
  uuid: string;
  name: string;
  source?: string;
  classificationsCount: number;
}

export interface ClassificationData {
  id: number;
  uuid: string;
  taxonomyId: number;
  parentId?: number;
  name: string;
  color?: string;
  /** Weight in basis points (10000 = 100%) */
  weight?: number;
  rank?: number;
  children: ClassificationData[];
  assignmentsCount: number;
}

export interface ClassificationAssignmentData {
  id: number;
  classificationId: number;
  classificationName: string;
  vehicleType: string;
  vehicleUuid: string;
  vehicleName: string;
  /** Weight in basis points (10000 = 100%) */
  weight: number;
  rank?: number;
}

export interface TaxonomyAllocation {
  classificationId: number;
  classificationName: string;
  color?: string;
  path: string[];
  /** Value in cents */
  value: number;
  /** Percentage of total (0.0 - 100.0) */
  percentage: number;
}

export interface CreateTaxonomyRequest {
  name: string;
  source?: string;
}

export interface UpdateTaxonomyRequest {
  name?: string;
  source?: string;
}

export interface CreateClassificationRequest {
  taxonomyId: number;
  parentId?: number;
  name: string;
  color?: string;
  weight?: number;
}

export interface UpdateClassificationRequest {
  name?: string;
  color?: string;
  weight?: number;
  parentId?: number;
  rank?: number;
}

export interface AssignSecurityRequest {
  classificationId: number;
  securityId: number;
  /** Weight in basis points (10000 = 100%) */
  weight: number;
}

// ============================================================================
// Report Types
// ============================================================================

export interface DividendEntry {
  date: string;
  securityId: number;
  securityName: string;
  securityIsin?: string;
  portfolioName: string;
  grossAmount: number;
  currency: string;
  taxes: number;
  netAmount: number;
  shares?: number;
  perShare?: number;
}

export interface DividendBySecurity {
  securityId: number;
  securityName: string;
  securityIsin?: string;
  totalGross: number;
  totalTaxes: number;
  totalNet: number;
  paymentCount: number;
}

export interface DividendByMonth {
  month: string;
  totalGross: number;
  totalTaxes: number;
  totalNet: number;
}

export interface DividendReport {
  startDate: string;
  endDate: string;
  totalGross: number;
  totalTaxes: number;
  totalNet: number;
  currency: string;
  entries: DividendEntry[];
  bySecurity: DividendBySecurity[];
  byMonth: DividendByMonth[];
}

export interface RealizedGain {
  date: string;
  securityId: number;
  securityName: string;
  securityIsin?: string;
  portfolioName: string;
  shares: number;
  proceeds: number;
  costBasis: number;
  gain: number;
  gainPercent: number;
  holdingDays: number;
  isLongTerm: boolean;
  currency: string;
  fees: number;
  taxes: number;
}

export interface GainBySecurity {
  securityId: number;
  securityName: string;
  securityIsin?: string;
  totalProceeds: number;
  totalCostBasis: number;
  totalGain: number;
  saleCount: number;
}

export interface RealizedGainsReport {
  startDate: string;
  endDate: string;
  totalProceeds: number;
  totalCostBasis: number;
  totalGain: number;
  totalFees: number;
  totalTaxes: number;
  currency: string;
  entries: RealizedGain[];
  bySecurity: GainBySecurity[];
  shortTermGain: number;
  longTermGain: number;
}

export interface TaxReport {
  year: number;
  currency: string;
  dividendIncome: number;
  dividendTaxesWithheld: number;
  shortTermGains: number;
  longTermGains: number;
  totalCapitalGains: number;
  totalFees: number;
  capitalGainsTaxes: number;
  dividends: DividendReport;
  realizedGains: RealizedGainsReport;
}

// ============================================================================
// Watchlist Types
// ============================================================================

export interface WatchlistData {
  id: number;
  name: string;
  securitiesCount: number;
}

export interface WatchlistSecurityData {
  securityId: number;
  securityUuid: string;
  name: string;
  isin?: string;
  ticker?: string;
  currency: string;
  latestPrice?: number;
  latestDate?: string;
  priceChange?: number;
  priceChangePercent?: number;
  high52w?: number;
  low52w?: number;
}

export interface WatchlistWithSecurities {
  id: number;
  name: string;
  securities: WatchlistSecurityData[];
}

// ============================================================================
// External Security Search Types
// ============================================================================

export interface ExternalSecuritySearchResult {
  symbol: string;
  name: string;
  isin?: string;
  wkn?: string;
  securityType?: string;
  currency?: string;
  region?: string;
  provider: string;
  providerId?: string;
}

export interface ExternalSearchResponse {
  results: ExternalSecuritySearchResult[];
  providersUsed: string[];
  errors: string[];
}

// ============================================================================
// Corporate Actions Types
// ============================================================================

export type CorporateActionType =
  | 'StockSplit'
  | 'ReverseSplit'
  | 'SpinOff'
  | 'Merger'
  | 'StockDividend'
  | 'RightsIssue'
  | 'SymbolChange';

export interface AffectedPortfolio {
  portfolioId: number;
  portfolioName: string;
  sharesBefore: number;
  sharesAfter: number;
}

export interface StockSplitPreview {
  securityName: string;
  effectiveDate: string;
  ratioDisplay: string;
  affectedPortfolios: AffectedPortfolio[];
  totalSharesBefore: number;
  totalSharesAfter: number;
  fifoLotsCount: number;
  pricesCount: number;
}

export interface ApplyStockSplitRequest {
  securityId: number;
  effectiveDate: string;
  ratioFrom: number;
  ratioTo: number;
  adjustPrices: boolean;
  adjustFifo?: boolean;
  note?: string;
}

export interface ApplySpinOffRequest {
  sourceSecurityId: number;
  targetSecurityId: number;
  effectiveDate: string;
  costAllocation: number;
  shareRatio: number;
  note?: string;
}

export interface CorporateActionResult {
  success: boolean;
  message: string;
  transactionsAdjusted: number;
  fifoLotsAdjusted: number;
  pricesAdjusted: number;
}

// ============================================================================
// PDF Import Types
// ============================================================================

export type ParsedTransactionType =
  | 'Buy'
  | 'Sell'
  | 'Dividend'
  | 'Interest'
  | 'Deposit'
  | 'Withdrawal'
  | 'Fee'
  | 'TaxRefund'
  | 'StockSplit'
  | 'TransferIn'
  | 'TransferOut'
  | 'Unknown';

export interface ParsedTransaction {
  date: string;
  txnType: ParsedTransactionType;
  securityName?: string;
  isin?: string;
  wkn?: string;
  shares?: number;
  pricePerShare?: number;
  grossAmount: number;
  fees: number;
  taxes: number;
  netAmount: number;
  currency: string;
  note?: string;
  exchangeRate?: number;
  forexCurrency?: string;
}

export interface ParseResult {
  bank: string;
  transactions: ParsedTransaction[];
  warnings: string[];
  rawText?: string;
}

export interface SecurityMatch {
  isin?: string;
  wkn?: string;
  name?: string;
  existingId?: number;
  existingName?: string;
}

export interface PotentialDuplicate {
  transactionIndex: number;
  existingTxnId: number;
  date: string;
  amount: number;
  securityName?: string;
  txnType: string;
}

export interface PdfImportPreview {
  bank: string;
  transactions: ParsedTransaction[];
  warnings: string[];
  newSecurities: SecurityMatch[];
  matchedSecurities: SecurityMatch[];
  potentialDuplicates: PotentialDuplicate[];
}

export interface PdfImportResult {
  success: boolean;
  bank: string;
  transactionsImported: number;
  transactionsSkipped: number;
  securitiesCreated: number;
  errors: string[];
  warnings: string[];
}

export interface SupportedBank {
  id: string;
  name: string;
  description: string;
}

// ============================================================================
// Investment Plan Types
// ============================================================================

export type PlanInterval = 'WEEKLY' | 'BIWEEKLY' | 'MONTHLY' | 'QUARTERLY' | 'YEARLY';

export interface InvestmentPlanData {
  id: number;
  name: string;
  securityId: number;
  securityName: string;
  accountId: number;
  accountName: string;
  portfolioId: number;
  portfolioName: string;
  interval: PlanInterval;
  amount: number;
  currency: string;
  dayOfMonth: number;
  startDate: string;
  endDate?: string;
  isActive: boolean;
  lastExecution?: string;
  nextExecution?: string;
  totalInvested: number;
  executionCount: number;
}

export interface CreateInvestmentPlanRequest {
  name: string;
  securityId: number;
  accountId: number;
  portfolioId: number;
  interval: PlanInterval;
  amount: number;
  dayOfMonth: number;
  startDate: string;
  endDate?: string;
}

export interface InvestmentPlanExecution {
  id: number;
  planId: number;
  date: string;
  shares: number;
  price: number;
  amount: number;
  fees: number;
  transactionId: number;
}

// ============================================================================
// Rebalancing Types
// ============================================================================

export interface RebalanceTarget {
  securityId?: number;
  classificationId?: number;
  targetWeight: number;
  currentWeight?: number;
  currentValue?: number;
}

export interface RebalanceAction {
  securityId: number;
  securityName: string;
  action: 'BUY' | 'SELL';
  shares: number;
  amount: number;
  currentWeight: number;
  targetWeight: number;
}

export interface RebalancePreview {
  totalValue: number;
  newCash?: number;
  targets: RebalanceTarget[];
  actions: RebalanceAction[];
  deviationBefore: number;
  deviationAfter: number;
}

export interface AiRebalanceTargetSuggestion {
  securityId: number;
  securityName: string;
  currentWeight: number;
  targetWeight: number;
  reason: string;
}

export interface AiRebalanceSuggestion {
  targets: AiRebalanceTargetSuggestion[];
  reasoning: string;
  riskAssessment: string;
}

// ============================================================================
// Benchmark Types
// ============================================================================

export interface BenchmarkData {
  id: number;
  securityId: number;
  securityName: string;
  isin?: string;
  startDate: string;
}

export interface BenchmarkComparison {
  portfolioReturn: number;
  benchmarkReturn: number;
  alpha: number;
  beta: number;
  sharpeRatio: number;
  correlation: number;
  trackingError: number;
}

export interface BenchmarkDataPoint {
  date: string;
  portfolioValue: number;
  portfolioReturn: number;
  benchmarkValue: number;
  benchmarkReturn: number;
}

// ============================================================================
// Aggregated Holdings Types (for get_all_holdings)
// ============================================================================

export interface AggregatedHolding {
  isin: string;
  name: string;
  currency: string;
  totalShares: number;
  currentPrice?: number;
  currentValue?: number;
  totalCostBasis: number;
  gainLoss?: number;
  gainLossPercent?: number;
  securityIds: number[];
  customLogo?: string;
  ticker?: string;
  latestPriceDate?: string;
}

// ============================================================================
// PDF Export Types
// ============================================================================

export interface PdfExportResult {
  success: boolean;
  path: string;
  pages: number;
  error?: string;
}

// ============================================================================
// Stock Split Detection Types
// ============================================================================

export interface DetectedSplit {
  securityId: number;
  securityName: string;
  date: string;
  suggestedRatio: string;
  priceBeforeNormalized: number;
  priceAfterNormalized: number;
  confidence: number;
}

export interface SplitDetectionResult {
  detectedSplits: DetectedSplit[];
  errors: string[];
}

// ============================================================================
// Portfolio Value History Types
// ============================================================================

export interface PortfolioValuePoint {
  date: string;
  value: number;
  investedCapital?: number;
}

export interface InvestedCapitalPoint {
  date: string;
  amount: number;
}

// ============================================================================
// FIFO Rebuild Types
// ============================================================================

export interface RebuildFifoResult {
  securitiesProcessed: number;
  lotsCreated: number;
  consumptionsCreated: number;
}

// ============================================================================
// AI Chart Annotation Types
// ============================================================================

/** Type of chart annotation */
export type AnnotationType =
  | 'support'
  | 'resistance'
  | 'trendline'
  | 'pattern'
  | 'signal'
  | 'target'
  | 'stoploss'
  | 'note';

/** Signal direction for annotations */
export type SignalDirection = 'bullish' | 'bearish' | 'neutral';

/** Trend direction */
export type TrendDirection = 'bullish' | 'bearish' | 'neutral';

/** Trend strength */
export type TrendStrength = 'strong' | 'moderate' | 'weak';

/** Trend information from AI analysis */
export interface TrendInfo {
  direction: TrendDirection;
  strength: TrendStrength;
  /** Confidence score (0.0 - 1.0) */
  confidence?: number;
}

/** A single chart annotation from AI analysis */
export interface ChartAnnotation {
  /** Annotation type (support, resistance, pattern, etc.) */
  type: AnnotationType;
  /** Price level (Y-coordinate) */
  price: number;
  /** Optional time/date (X-coordinate) - null for horizontal lines */
  time?: string | null;
  /** Optional end time for ranges/trendlines */
  timeEnd?: string | null;
  /** Short title (max 20 chars) */
  title: string;
  /** Detailed explanation */
  description: string;
  /** Confidence score (0.0 - 1.0) */
  confidence: number;
  /** Signal direction (bullish/bearish/neutral) */
  signal?: SignalDirection | null;
}

/** Response from AI analysis with annotations */
export interface AnnotationAnalysisResponse {
  /** Overall analysis text */
  analysis: string;
  /** Trend information */
  trend: TrendInfo;
  /** Array of chart annotations */
  annotations: ChartAnnotation[];
  /** AI provider used */
  provider: string;
  /** Model used */
  model: string;
  /** Tokens used (if available) */
  tokensUsed?: number;
}

// ============================================================================
// Enhanced Chart Analysis Types
// ============================================================================

/** A single indicator reading with current value and signal */
export interface IndicatorValue {
  /** Indicator name (e.g., "RSI") */
  name: string;
  /** Indicator parameters (e.g., "14") */
  params: string;
  /** Current calculated value (e.g., 72.5) */
  currentValue: number;
  /** Previous value for trend detection */
  previousValue?: number;
  /** Signal interpretation (e.g., "overbought", "bullish_crossover") */
  signal?: string;
}

/** OHLC candlestick data for a single period */
export interface CandleData {
  date: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

/** Volume analysis context */
export interface VolumeAnalysis {
  currentVolume: number;
  avgVolume20d: number;
  /** Ratio of current volume to average (e.g., 1.5 = 50% above average) */
  volumeRatio: number;
  /** Volume trend direction ("increasing", "decreasing", "stable") */
  volumeTrend: string;
}

/** Enhanced chart context with indicator values, OHLC data, and volume analysis */
export interface EnhancedChartContext {
  securityName: string;
  ticker?: string;
  currency: string;
  currentPrice: number;
  timeframe: string;
  /** Indicator values with actual readings */
  indicatorValues: IndicatorValue[];
  /** Last N candles OHLC data */
  candles?: CandleData[];
  /** Volume analysis */
  volumeAnalysis?: VolumeAnalysis;
  /** Price change percentage for the period */
  priceChangePercent?: number;
  /** 52-week high */
  high52Week?: number;
  /** 52-week low */
  low52Week?: number;
  /** Distance from 52-week high as percentage */
  distanceFromHighPercent?: number;
  /** Include web context (news, earnings, analyst ratings) - only for web-capable models */
  includeWebContext?: boolean;
}

/** AI-suggested price alert */
export interface AlertSuggestion {
  /** Price level for alert */
  price: number;
  /** Alert condition ("above", "below", "crosses_up", "crosses_down") */
  condition: string;
  /** Reason for the alert */
  reason: string;
  /** Alert priority ("high", "medium", "low") */
  priority: string;
}

/** Risk/Reward analysis from AI */
export interface RiskRewardAnalysis {
  /** Suggested entry price */
  entryPrice?: number;
  /** Stop-loss price */
  stopLoss?: number;
  /** Take-profit target */
  takeProfit?: number;
  /** Risk/Reward ratio (e.g., 2.5 = 2.5:1) */
  riskRewardRatio?: number;
  /** Explanation of the setup */
  rationale?: string;
}

/** Enhanced response from AI analysis with alerts and risk/reward */
export interface EnhancedAnnotationAnalysisResponse {
  /** Overall analysis text */
  analysis: string;
  /** Trend information */
  trend: TrendInfo;
  /** Array of chart annotations */
  annotations: ChartAnnotation[];
  /** Suggested price alerts */
  alerts: AlertSuggestion[];
  /** Risk/Reward analysis (optional if no clear setup) */
  riskReward?: RiskRewardAnalysis;
  /** AI provider used */
  provider: string;
  /** Model used */
  model: string;
  /** Tokens used (if available) */
  tokensUsed?: number;
}

/** Annotation with generated ID for React rendering */
export interface ChartAnnotationWithId extends ChartAnnotation {
  id: string;
}

/** Annotation style configuration */
export interface AnnotationStyle {
  color: string;
  lineStyle: 'solid' | 'dashed' | 'dotted';
  lineWidth: number;
  icon?: string;
}

// ============================================================================
// Persisted Annotation Types (Database)
// ============================================================================

/** Annotation data as stored in database */
export interface PersistedAnnotation {
  id: number;
  uuid: string;
  securityId: number;
  annotationType: AnnotationType;
  price: number;
  time?: string | null;
  timeEnd?: string | null;
  title: string;
  description?: string | null;
  confidence: number;
  signal?: SignalDirection | null;
  source: 'ai' | 'user';
  provider?: string | null;
  model?: string | null;
  isVisible: boolean;
  createdAt: string;
}

/** Request to save annotations to database */
export interface SaveAnnotationRequest {
  annotationType: AnnotationType;
  price: number;
  time?: string | null;
  timeEnd?: string | null;
  title: string;
  description?: string | null;
  confidence: number;
  signal?: SignalDirection | null;
  source?: 'ai' | 'user';
  provider?: string | null;
  model?: string | null;
}

// ============================================================================
// Price Alerts
// ============================================================================

export type AlertType =
  | 'price_above'
  | 'price_below'
  | 'price_crosses'
  | 'rsi_above'
  | 'rsi_below'
  | 'volume_spike'
  | 'divergence'
  | 'pattern_detected'
  | 'support_break'
  | 'resistance_break';

export interface PriceAlert {
  id: number;
  uuid: string;
  securityId: number;
  securityName?: string;
  securityTicker?: string;
  alertType: AlertType;
  targetValue: number;
  targetValue2?: number;
  isActive: boolean;
  isTriggered: boolean;
  triggerCount: number;
  lastTriggeredAt?: string;
  lastTriggeredPrice?: number;
  note?: string;
  createdAt: string;
}

export interface CreateAlertRequest {
  securityId: number;
  alertType: AlertType;
  targetValue: number;
  targetValue2?: number;
  note?: string;
}

export interface UpdateAlertRequest {
  id: number;
  targetValue?: number;
  targetValue2?: number;
  isActive?: boolean;
  note?: string;
}

export interface TriggeredAlert {
  alert: PriceAlert;
  currentPrice: number;
  triggerReason: string;
}
