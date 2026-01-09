// Portfolio Performance XML file format types
// Compatible with PP version 68

export const CURRENT_VERSION = 68;
export const SHARES_FACTOR = 100_000_000; // 10^8
export const AMOUNT_FACTOR = 100; // Cents to currency

// ============================================================================
// Main Client Structure
// ============================================================================

export interface PPClient {
  version: number;
  baseCurrency: string;
  securities: PPSecurity[];
  watchlists: PPWatchlist[];
  accounts: PPAccount[];
  portfolios: PPPortfolio[];
  plans: PPInvestmentPlan[];
  taxonomies: PPTaxonomy[];
  dashboards: PPDashboard[];
  properties: Record<string, string>;
  settings: PPSettings;
}

// ============================================================================
// Securities
// ============================================================================

export interface PPSecurity {
  uuid: string;
  name: string;
  currencyCode: string;
  onlineId?: string;
  isin?: string;
  wkn?: string;
  tickerSymbol?: string;
  calendar?: string;
  feed?: string;
  feedURL?: string;
  latestFeed?: string;
  latestFeedURL?: string;
  prices: PPPrice[];
  latest?: PPLatestPrice;
  events: PPSecurityEvent[];
  attributes?: Record<string, string>;
  isRetired: boolean;
  note?: string;
  updatedAt?: string;
}

export interface PPPrice {
  date: string; // Format: YYYY-MM-DD
  value: number; // Value in cents
}

export interface PPLatestPrice {
  date: string;
  value: number;
  high?: number;
  low?: number;
  volume?: number;
}

export interface PPSecurityEvent {
  date: string;
  type: string; // STOCK_SPLIT, DIVIDEND, etc.
  details?: string;
}

// ============================================================================
// Watchlists
// ============================================================================

export interface PPWatchlist {
  name: string;
  securities: string[]; // UUIDs
}

// ============================================================================
// Accounts
// ============================================================================

export interface PPAccount {
  uuid: string;
  name: string;
  currencyCode: string;
  isRetired: boolean;
  note?: string;
  attributes?: Record<string, string>;
  updatedAt?: string;
  transactions: PPAccountTransaction[];
}

export interface PPAccountTransaction {
  uuid: string;
  date: string;
  type: PPAccountTransactionType;
  currencyCode: string;
  amount: number;
  shares?: number;
  security?: string;
  crossEntry?: string;
  fees?: number;
  taxes?: number;
  note?: string;
  source?: string;
  units?: PPTransactionUnit[];
}

export type PPAccountTransactionType =
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

// ============================================================================
// Portfolios (Depots)
// ============================================================================

export interface PPPortfolio {
  uuid: string;
  name: string;
  referenceAccount: string;
  isRetired: boolean;
  note?: string;
  attributes?: Record<string, string>;
  updatedAt?: string;
  transactions: PPPortfolioTransaction[];
}

export interface PPPortfolioTransaction {
  uuid: string;
  date: string;
  type: PPPortfolioTransactionType;
  currencyCode: string;
  amount: number;
  shares: number;
  security: string;
  crossEntry?: string;
  fees?: number;
  taxes?: number;
  note?: string;
  source?: string;
  units?: PPTransactionUnit[];
}

export type PPPortfolioTransactionType =
  | 'BUY'
  | 'SELL'
  | 'DELIVERY_INBOUND'
  | 'DELIVERY_OUTBOUND';

export interface PPTransactionUnit {
  type: string; // FEE, TAX, GROSS_VALUE, etc.
  amount: number;
  currencyCode?: string;
  fxAmount?: number;
  fxCurrencyCode?: string;
  exchangeRate?: string;
}

// ============================================================================
// Investment Plans
// ============================================================================

export interface PPInvestmentPlan {
  name: string;
  security?: string;
  portfolio?: string;
  account?: string;
  amount: number;
  interval: number;
  start?: string;
  autoGenerate: boolean;
}

// ============================================================================
// Taxonomies
// ============================================================================

export interface PPTaxonomy {
  id: string;
  name: string;
  root?: PPClassification;
}

export interface PPClassification {
  id: string;
  name: string;
  color?: string;
  weight?: number;
  children: PPClassification[];
  assignments: PPAssignment[];
}

export interface PPAssignment {
  investmentVehicle: string;
  weight: number;
  rank?: number;
}

// ============================================================================
// Dashboards
// ============================================================================

export interface PPDashboard {
  name: string;
  configuration?: string;
  columns: PPDashboardColumn[];
}

export interface PPDashboardColumn {
  weight?: number;
  widgets: PPDashboardWidget[];
}

export interface PPDashboardWidget {
  type: string;
  label?: string;
  configuration?: string;
}

// ============================================================================
// Settings
// ============================================================================

export interface PPSettings {
  bookmarkColumns?: string;
  attributeTypes?: PPAttributeType[];
  configurationSets?: string;
}

export interface PPAttributeType {
  id: string;
  name: string;
  columnLabel?: string;
  target?: string;
  type?: string;
  converterClass?: string;
}

// ============================================================================
// Conversion helpers
// ============================================================================

export function sharesToNumber(shares: number): number {
  return shares / SHARES_FACTOR;
}

export function numberToShares(num: number): number {
  return Math.round(num * SHARES_FACTOR);
}

export function centsToAmount(cents: number): number {
  return cents / AMOUNT_FACTOR;
}

export function amountToCents(amount: number): number {
  return Math.round(amount * AMOUNT_FACTOR);
}

// ============================================================================
// Factory functions
// ============================================================================

export function createEmptyClient(): PPClient {
  return {
    version: CURRENT_VERSION,
    baseCurrency: 'EUR',
    securities: [],
    watchlists: [],
    accounts: [],
    portfolios: [],
    plans: [],
    taxonomies: [],
    dashboards: [],
    properties: {},
    settings: {},
  };
}

export function createSecurity(name: string, currencyCode = 'EUR'): PPSecurity {
  return {
    uuid: crypto.randomUUID(),
    name,
    currencyCode,
    prices: [],
    events: [],
    isRetired: false,
  };
}

export function createAccount(name: string, currencyCode = 'EUR'): PPAccount {
  return {
    uuid: crypto.randomUUID(),
    name,
    currencyCode,
    isRetired: false,
    transactions: [],
  };
}

export function createPortfolio(name: string, referenceAccountUuid: string): PPPortfolio {
  return {
    uuid: crypto.randomUUID(),
    name,
    referenceAccount: referenceAccountUuid,
    isRetired: false,
    transactions: [],
  };
}
