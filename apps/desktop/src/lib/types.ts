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
  isin?: string;
  wkn?: string;
  ticker?: string;
  isRetired: boolean;
  latestPrice?: number;
  latestPriceDate?: string;
  pricesCount: number;
}

export interface AccountData {
  id: number;
  uuid: string;
  name: string;
  currency: string;
  isRetired: boolean;
  transactionsCount: number;
  balance: number;
}

export interface PortfolioData {
  id: number;
  uuid: string;
  name: string;
  referenceAccountName?: string;
  isRetired: boolean;
  transactionsCount: number;
  holdingsCount: number;
}

export interface TransactionData {
  id: number;
  uuid: string;
  ownerType: string;
  ownerName: string;
  txnType: string;
  date: string;
  amount: number;
  currency: string;
  shares?: number;
  securityName?: string;
  securityUuid?: string;
  note?: string;
  fees: number;
  taxes: number;
  hasForex: boolean;
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
// Helper Functions
// ============================================================================

export function formatCurrency(amount: number, currency: string = 'EUR'): string {
  return new Intl.NumberFormat('de-DE', {
    style: 'currency',
    currency,
  }).format(amount);
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
