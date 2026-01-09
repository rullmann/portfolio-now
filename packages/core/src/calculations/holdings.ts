import type { Account, Security, Transaction } from '../models/types';

export interface Holding {
  securityId: string;
  security: Security;
  shares: number;
  purchaseValue: number; // Total cost basis
  currentValue: number;
  gainLoss: number;
  gainLossPercent: number;
}

export interface AccountBalance {
  accountId: string;
  balance: number;
  currency: string;
}

/**
 * Calculate current holdings from transactions
 */
export function calculateHoldings(
  accounts: Account[],
  securities: Security[]
): Holding[] {
  const holdingsMap = new Map<string, { shares: number; cost: number }>();

  // Aggregate all transactions
  for (const account of accounts) {
    if (account.type !== 'DEPOT') continue;

    for (const tx of account.transactions) {
      if (!tx.securityId) continue;

      const current = holdingsMap.get(tx.securityId) || { shares: 0, cost: 0 };

      switch (tx.type) {
        case 'BUY':
        case 'TRANSFER_IN':
          current.shares += tx.shares || 0;
          current.cost += tx.amount + tx.fees;
          break;
        case 'SELL':
        case 'TRANSFER_OUT':
          const soldShares = tx.shares || 0;
          if (current.shares > 0) {
            // Proportional cost reduction
            const costPerShare = current.cost / current.shares;
            current.cost -= costPerShare * soldShares;
          }
          current.shares -= soldShares;
          break;
      }

      holdingsMap.set(tx.securityId, current);
    }
  }

  // Build holdings list
  const holdings: Holding[] = [];
  const securityMap = new Map(securities.map((s) => [s.id, s]));

  for (const [securityId, data] of holdingsMap) {
    if (data.shares <= 0) continue;

    const security = securityMap.get(securityId);
    if (!security) continue;

    const currentValue = data.shares * (security.latestPrice || 0);
    const gainLoss = currentValue - data.cost;

    holdings.push({
      securityId,
      security,
      shares: data.shares,
      purchaseValue: data.cost,
      currentValue,
      gainLoss,
      gainLossPercent: data.cost > 0 ? (gainLoss / data.cost) * 100 : 0,
    });
  }

  return holdings.sort((a, b) => b.currentValue - a.currentValue);
}

/**
 * Calculate account balances
 */
export function calculateAccountBalances(accounts: Account[]): AccountBalance[] {
  return accounts
    .filter((a) => a.type === 'CASH')
    .map((account) => {
      let balance = 0;

      for (const tx of account.transactions) {
        switch (tx.type) {
          case 'DEPOSIT':
          case 'DIVIDEND':
          case 'INTEREST':
          case 'TRANSFER_IN':
            balance += tx.amount;
            break;
          case 'WITHDRAWAL':
          case 'FEES':
          case 'TAXES':
          case 'TRANSFER_OUT':
            balance -= tx.amount;
            break;
          case 'BUY':
            balance -= tx.amount + tx.fees + tx.taxes;
            break;
          case 'SELL':
            balance += tx.amount - tx.fees - tx.taxes;
            break;
        }
      }

      return {
        accountId: account.id,
        balance,
        currency: account.currency,
      };
    });
}
