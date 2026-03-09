import { getLatestExchangeRate } from './api';
import { getCachedModels, type AiProvider } from '../store';

const CURRENCY_SYMBOLS: Record<string, string> = {
  EUR: '€', USD: '$', GBP: '£', CHF: 'CHF ', JPY: '¥',
};

/** Cached exchange rate (USD -> baseCurrency) */
let cachedRate: { currency: string; rate: number } | null = null;

/** Get cached or fetch USD -> baseCurrency exchange rate */
async function getUsdRate(baseCurrency: string): Promise<number> {
  if (baseCurrency === 'USD') return 1;
  if (cachedRate?.currency === baseCurrency) return cachedRate.rate;
  try {
    const r = await getLatestExchangeRate('USD', baseCurrency);
    cachedRate = { currency: baseCurrency, rate: r.rate };
    return r.rate;
  } catch {
    return 1; // fallback to USD
  }
}

/** Calculate cost from token counts and model pricing (USD per 1M tokens) */
function calcCost(inputTokens: number, outputTokens: number, pricingInput: number, pricingOutput: number): number {
  return (inputTokens * pricingInput + outputTokens * pricingOutput) / 1_000_000;
}

/** Format cost for display: "€0,03" or "$0.03" */
export function formatCost(cost: number, baseCurrency: string): string {
  const sym = CURRENCY_SYMBOLS[baseCurrency] || baseCurrency + ' ';
  if (cost < 0.005) return `<${sym}0,01`;
  return `${sym}${cost.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

export interface AiCostResult {
  costDisplay: string;
  totalTokens: number;
}

/**
 * Calculate and format the cost of an AI request.
 * Returns formatted cost string in user's base currency + total token count.
 */
export async function calculateAiCost(
  provider: string,
  model: string,
  inputTokens: number | null | undefined,
  outputTokens: number | null | undefined,
  tokensUsed: number | null | undefined,
  baseCurrency: string,
): Promise<AiCostResult | null> {
  const total = tokensUsed ?? ((inputTokens ?? 0) + (outputTokens ?? 0));
  if (!total) return null;

  // Map provider display name to AiProvider key
  const providerMap: Record<string, AiProvider> = {
    Claude: 'claude', OpenAI: 'openai', Gemini: 'gemini',
    Perplexity: 'perplexity', OpenRouter: 'openrouter',
  };
  const providerKey = providerMap[provider];
  if (!providerKey) return { costDisplay: '', totalTokens: total };

  // Find pricing from cached models
  const models = getCachedModels(providerKey);
  const modelInfo = models.find(m => m.id === model);
  if (!modelInfo?.pricingInput || !modelInfo?.pricingOutput) {
    return { costDisplay: '', totalTokens: total };
  }

  const inp = inputTokens ?? Math.round(total * 0.8); // estimate 80% input if not available
  const out = outputTokens ?? (total - inp);
  const costUsd = calcCost(inp, out, modelInfo.pricingInput, modelInfo.pricingOutput);

  const rate = await getUsdRate(baseCurrency);
  const costLocal = costUsd * rate;

  return {
    costDisplay: formatCost(costLocal, baseCurrency),
    totalTokens: total,
  };
}
