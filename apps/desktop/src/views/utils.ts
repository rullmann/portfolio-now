/**
 * Utility functions for views.
 */

import type {
  Security,
  PortfolioFile,
  Holding,
  GroupedHolding,
} from './types';

const SCALE = 100_000_000;
const BUY = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN'];
const SELL = ['SELL', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT'];

/**
 * Get latest price for a security.
 */
export function getLatestPrice(security: Security): number {
  // First try the latest element
  if (security.latest?.value) {
    return security.latest.value / SCALE;
  }
  // Fallback to last price in prices array
  if (security.prices && security.prices.length > 0) {
    const prices = security.prices;
    const sorted = [...prices].sort((a, b) => b.date.localeCompare(a.date));
    return sorted[0].value / SCALE;
  }
  return 0;
}

/**
 * Calculate holdings from portfolio transactions.
 */
export function calculateHoldings(portfolioFile: PortfolioFile): Holding[] {
  const securities = portfolioFile.securities || [];
  const portfolios = portfolioFile.portfolios || [];

  // Build UUID to index map for security lookup
  const securityIndexByUuid = new Map<string, number>();
  securities.forEach((sec, idx) => securityIndexByUuid.set(sec.uuid, idx));

  // Track shares per security UUID AND portfolio
  const sharesPerSecurityPortfolio = new Map<string, { shares: number; portfolioName: string; securityUuid: string }>();

  for (const portfolio of portfolios) {
    const portfolioName = portfolio.name || 'Unbenannt';
    const transactions = portfolio.transactions || [];

    for (const tx of transactions) {
      if (!tx.securityUuid) continue;

      const key = `${tx.securityUuid}:${portfolioName}`;
      const shares = (tx.shares || 0) / SCALE;
      const current = sharesPerSecurityPortfolio.get(key) || { shares: 0, portfolioName, securityUuid: tx.securityUuid };

      if (BUY.includes(tx.transactionType)) {
        current.shares += shares;
      } else if (SELL.includes(tx.transactionType)) {
        current.shares -= shares;
      }

      sharesPerSecurityPortfolio.set(key, current);
    }
  }

  // Build holdings array
  const holdings: Holding[] = [];

  for (const [, data] of sharesPerSecurityPortfolio.entries()) {
    if (data.shares <= 0.0001) continue;

    const securityIndex = securityIndexByUuid.get(data.securityUuid);
    if (securityIndex === undefined) continue;

    const security = securities[securityIndex];
    if (!security) continue;

    const latestPrice = getLatestPrice(security);
    const value = data.shares * latestPrice;

    holdings.push({
      securityIndex,
      security,
      shares: data.shares,
      latestPrice,
      value,
      currency: security.currency || 'EUR',
      portfolioName: data.portfolioName,
    });
  }

  // Sort by value descending
  holdings.sort((a, b) => b.value - a.value);

  return holdings;
}

/**
 * Group holdings by ISIN for aggregated view.
 */
export function groupHoldingsByISIN(holdings: Holding[]): GroupedHolding[] {
  const groups = new Map<string, GroupedHolding>();

  for (const holding of holdings) {
    const key = holding.security.isin || `name:${holding.security.name}`;

    const existing = groups.get(key);
    if (existing) {
      existing.totalShares += holding.shares;
      existing.totalValue += holding.value;
      existing.holdings.push(holding);
      if (holding.latestPrice > existing.latestPrice) {
        existing.latestPrice = holding.latestPrice;
      }
    } else {
      groups.set(key, {
        isin: holding.security.isin || null,
        name: holding.security.name,
        totalShares: holding.shares,
        totalValue: holding.value,
        currency: holding.currency,
        latestPrice: holding.latestPrice,
        holdings: [holding],
      });
    }
  }

  const result = Array.from(groups.values());
  result.sort((a, b) => b.totalValue - a.totalValue);

  return result;
}

/**
 * Calculate total portfolio value.
 */
export function calculateTotalValue(holdings: Holding[]): number {
  return holdings.reduce((sum, h) => sum + h.value, 0);
}

/**
 * Format number for display.
 */
export function formatNumber(value: number, decimals: number = 2): string {
  return value.toLocaleString('de-DE', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

/**
 * Format currency value for display.
 */
export function formatCurrency(value: number, currency: string = 'EUR'): string {
  return `${formatNumber(value)} ${currency}`;
}

/**
 * Format percentage for display.
 */
export function formatPercent(value: number): string {
  return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
}
