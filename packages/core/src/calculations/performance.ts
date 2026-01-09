import type { Account, Transaction } from '../models/types';

export interface PerformanceResult {
  ttwror: number; // True Time-Weighted Rate of Return (%)
  irr: number; // Internal Rate of Return (%)
  absoluteGain: number;
  totalInvested: number;
  totalWithdrawn: number;
  currentValue: number;
}

export interface CashFlow {
  date: string;
  amount: number;
}

/**
 * Calculate True Time-Weighted Rate of Return (TTWROR)
 *
 * TTWROR eliminates the impact of cash flows to measure
 * pure investment performance.
 */
export function calculateTTWROR(
  cashFlows: CashFlow[],
  valuations: { date: string; value: number }[]
): number {
  if (valuations.length < 2) return 0;

  // Sort by date
  const sortedValuations = [...valuations].sort(
    (a, b) => new Date(a.date).getTime() - new Date(b.date).getTime()
  );
  const sortedFlows = [...cashFlows].sort(
    (a, b) => new Date(a.date).getTime() - new Date(b.date).getTime()
  );

  let cumulativeReturn = 1;

  for (let i = 1; i < sortedValuations.length; i++) {
    const prevValue = sortedValuations[i - 1].value;
    const currValue = sortedValuations[i].value;
    const prevDate = sortedValuations[i - 1].date;
    const currDate = sortedValuations[i].date;

    // Find cash flows in this period
    const periodFlows = sortedFlows.filter(
      (f) => f.date > prevDate && f.date <= currDate
    );
    const netFlow = periodFlows.reduce((sum, f) => sum + f.amount, 0);

    // Period return: (End Value) / (Start Value + Cash Flows)
    const denominator = prevValue + netFlow;
    if (denominator > 0) {
      const periodReturn = currValue / denominator;
      cumulativeReturn *= periodReturn;
    }
  }

  return (cumulativeReturn - 1) * 100;
}

/**
 * Calculate Internal Rate of Return (IRR) using Newton-Raphson method
 *
 * IRR is the discount rate that makes NPV of all cash flows equal to zero.
 */
export function calculateIRR(
  cashFlows: CashFlow[],
  finalValue: number,
  finalDate: string,
  maxIterations = 100,
  tolerance = 0.0001
): number {
  if (cashFlows.length === 0) return 0;

  const allFlows = [
    ...cashFlows.map((cf) => ({
      date: new Date(cf.date),
      amount: -cf.amount, // Investments are negative
    })),
    {
      date: new Date(finalDate),
      amount: finalValue, // Final value is positive
    },
  ].sort((a, b) => a.date.getTime() - b.date.getTime());

  const startDate = allFlows[0].date;

  // Years from start for each cash flow
  const flows = allFlows.map((f) => ({
    years: (f.date.getTime() - startDate.getTime()) / (365.25 * 24 * 60 * 60 * 1000),
    amount: f.amount,
  }));

  // Newton-Raphson iteration
  let rate = 0.1; // Initial guess: 10%

  for (let i = 0; i < maxIterations; i++) {
    let npv = 0;
    let derivative = 0;

    for (const flow of flows) {
      const discountFactor = Math.pow(1 + rate, flow.years);
      npv += flow.amount / discountFactor;
      derivative -= (flow.years * flow.amount) / Math.pow(1 + rate, flow.years + 1);
    }

    if (Math.abs(npv) < tolerance) {
      return rate * 100;
    }

    if (derivative === 0) break;
    rate = rate - npv / derivative;

    // Clamp to reasonable range
    if (rate < -0.99) rate = -0.99;
    if (rate > 10) rate = 10;
  }

  return rate * 100;
}

/**
 * Calculate overall performance for a portfolio
 */
export function calculatePortfolioPerformance(
  accounts: Account[],
  currentValue: number
): PerformanceResult {
  const cashFlows: CashFlow[] = [];
  let totalInvested = 0;
  let totalWithdrawn = 0;

  for (const account of accounts) {
    for (const tx of account.transactions) {
      switch (tx.type) {
        case 'DEPOSIT':
          totalInvested += tx.amount;
          cashFlows.push({ date: tx.date, amount: tx.amount });
          break;
        case 'WITHDRAWAL':
          totalWithdrawn += tx.amount;
          cashFlows.push({ date: tx.date, amount: -tx.amount });
          break;
      }
    }
  }

  const netInvested = totalInvested - totalWithdrawn;
  const absoluteGain = currentValue - netInvested;

  // For IRR calculation
  const today = new Date().toISOString().split('T')[0];
  const irr = calculateIRR(cashFlows, currentValue, today);

  return {
    ttwror: 0, // Would need historical valuations
    irr,
    absoluteGain,
    totalInvested,
    totalWithdrawn,
    currentValue,
  };
}
