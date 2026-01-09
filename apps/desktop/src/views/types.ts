/**
 * Shared types for views.
 * These are legacy types for direct portfolio file viewing.
 */

// ============================================================================
// Portfolio File Types (Legacy - direct file viewing)
// ============================================================================

export interface PriceEntry {
  date: string;
  value: number; // price * 10^8
}

export interface LatestPrice {
  date?: string | null;
  value?: number | null;
  high?: number | null;
  low?: number | null;
  volume?: number | null;
}

export interface Security {
  uuid: string;
  name: string;
  currency: string;
  isin?: string | null;
  ticker?: string | null;
  wkn?: string | null;
  feed?: string | null;
  prices: PriceEntry[];
  latest?: LatestPrice | null;
}

export interface AccountTransaction {
  uuid: string;
  date: string;
  transactionType: string;
  amount: { amount: number; currency: string };
  shares?: number | null;
}

export interface Account {
  uuid: string;
  name: string;
  currency: string;
  transactions: AccountTransaction[];
}

export interface PortfolioTransaction {
  uuid: string;
  date: string;
  transactionType: string;
  amount: { amount: number; currency: string };
  shares: number;
  securityUuid?: string | null;
}

export interface Portfolio {
  uuid: string;
  name: string;
  referenceAccountUuid?: string | null;
  transactions: PortfolioTransaction[];
}

export interface PortfolioFile {
  version: number;
  baseCurrency: string;
  securities: Security[];
  accounts: Account[];
  portfolios: Portfolio[];
  watchlists: Array<{ name: string }>;
  taxonomies: Array<{ id: string; name: string }>;
}

// ============================================================================
// Database Types
// ============================================================================

export interface PortfolioData {
  id: number;
  uuid: string;
  name: string;
  referenceAccountName: string | null;
  isRetired: boolean;
  transactionsCount: number;
  holdingsCount: number;
}

export interface PortfolioHolding {
  portfolioName: string;
  shares: number;
  value: number | null;
}

export interface AggregatedHolding {
  isin: string;
  name: string;
  currency: string;
  securityId: number;
  totalShares: number;
  currentPrice: number | null;
  currentValue: number | null;
  /** Einstandswert (total cost basis from FIFO) */
  costBasis: number;
  /** Einstandskurs (cost per share = costBasis / totalShares) */
  purchasePrice: number | null;
  /** Gewinn/Verlust (unrealized gain/loss) */
  gainLoss: number | null;
  /** Abs.Perf. % Seit (performance percentage) */
  gainLossPercent: number | null;
  /** ΣDiv Seit (total dividends received for this position) */
  dividendsTotal: number;
  portfolios: PortfolioHolding[];
  customLogo?: string; // Base64-encoded custom logo
}

// ============================================================================
// Calculated Types
// ============================================================================

export interface Holding {
  securityIndex: number;
  security: Security;
  shares: number;
  latestPrice: number;
  value: number;
  currency: string;
  portfolioName?: string;
}

export interface GroupedHolding {
  isin: string | null;
  name: string;
  totalShares: number;
  totalValue: number;
  currency: string;
  latestPrice: number;
  holdings: Holding[];
}

// ============================================================================
// Scale Factors
// ============================================================================

export const PRICE_SCALE = 100_000_000;
export const SHARES_SCALE = 100_000_000;

// Transaction types
export const BUY_TYPES = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN'];
export const SELL_TYPES = ['SELL', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT'];
