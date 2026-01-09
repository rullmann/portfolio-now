// Core type definitions for Portfolio Performance

export type Currency = 'EUR' | 'USD' | 'CHF' | 'GBP' | string;

export type TransactionType =
  | 'BUY'
  | 'SELL'
  | 'DIVIDEND'
  | 'INTEREST'
  | 'DEPOSIT'
  | 'WITHDRAWAL'
  | 'TRANSFER_IN'
  | 'TRANSFER_OUT'
  | 'FEES'
  | 'TAXES';

export type AccountType = 'DEPOT' | 'CASH';

export type SecurityType =
  | 'STOCK'
  | 'ETF'
  | 'FUND'
  | 'BOND'
  | 'CRYPTO'
  | 'COMMODITY'
  | 'OTHER';

export interface PriceEntry {
  date: string; // ISO date string YYYY-MM-DD
  close: number;
  high?: number;
  low?: number;
  volume?: number;
}

export interface Security {
  id: string;
  name: string;
  type: SecurityType;
  isin?: string;
  wkn?: string;
  ticker?: string;
  currency: Currency;
  feed?: string; // Quote feed identifier
  feedUrl?: string;
  latestPrice?: number;
  latestPriceDate?: string;
  note?: string;
  isRetired?: boolean;
  prices: PriceEntry[];
  attributes: Record<string, string>;
}

export interface Transaction {
  id: string;
  type: TransactionType;
  date: string; // ISO date string
  securityId?: string;
  shares?: number;
  amount: number; // In account currency
  currencyGrossAmount?: number;
  exchangeRate?: number;
  fees: number;
  taxes: number;
  note?: string;
  source?: string; // e.g., "PDF Import: Trade Republic"
}

export interface Account {
  id: string;
  name: string;
  type: AccountType;
  currency: Currency;
  isRetired?: boolean;
  note?: string;
  transactions: Transaction[];
}

export interface TaxonomyAssignment {
  classificationId: string;
  weight: number; // 0-100
}

export interface TaxonomyClassification {
  id: string;
  name: string;
  color?: string;
  parentId?: string;
  children: TaxonomyClassification[];
}

export interface Taxonomy {
  id: string;
  name: string;
  root: TaxonomyClassification;
  assignments: Map<string, TaxonomyAssignment[]>; // securityId -> assignments
}

export interface Portfolio {
  id: string;
  name: string;
  baseCurrency: Currency;
  accounts: Account[];
  securities: Security[];
  taxonomies: Taxonomy[];
  createdAt: string;
  updatedAt: string;
  note?: string;
}

export interface Client {
  version: number;
  portfolios: Portfolio[];
  settings: ClientSettings;
}

export interface ClientSettings {
  language: string;
  theme: 'light' | 'dark' | 'system';
  dateFormat: string;
  numberFormat: string;
}
