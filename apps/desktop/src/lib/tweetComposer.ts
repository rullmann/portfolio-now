/**
 * Tweet Text Composer
 *
 * Generates formatted tweet text from chart analysis data.
 * Respects the 280-character limit for X/Twitter.
 */

import type { TrendInfo, RiskRewardAnalysis } from './types';
import type { TechnicalSignal } from './signals';

export interface TweetComposeInput {
  securityName: string;
  ticker?: string | null;
  currentPrice: number;
  currency: string;
  trendInfo?: TrendInfo;
  riskReward?: RiskRewardAnalysis;
  signals?: TechnicalSignal[];
  patterns?: string[];
  analysis?: string;
  hashtags: string;
  language: 'de' | 'en';
}

const TWEET_MAX_LENGTH = 280;

function formatPrice(price: number, currency: string): string {
  return `${price.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
}

function getTrendEmoji(direction: string): string {
  switch (direction) {
    case 'bullish': return '\u{1F4C8}';
    case 'bearish': return '\u{1F4C9}';
    default: return '\u{27A1}\u{FE0F}';
  }
}

function getTrendLabel(direction: string, strength: string, lang: 'de' | 'en'): string {
  const dirLabels: Record<string, Record<string, string>> = {
    de: { bullish: 'Bullish', bearish: 'Bearish', neutral: 'Neutral' },
    en: { bullish: 'Bullish', bearish: 'Bearish', neutral: 'Neutral' },
  };
  const strLabels: Record<string, Record<string, string>> = {
    de: { strong: 'stark', moderate: 'moderat', weak: 'schwach' },
    en: { strong: 'strong', moderate: 'moderate', weak: 'weak' },
  };
  return `${dirLabels[lang][direction] ?? direction} (${strLabels[lang][strength] ?? strength})`;
}

function getSignalSummary(signals: TechnicalSignal[], maxLength: number): string {
  if (signals.length === 0) return '';

  const signalNames: Record<string, string> = {
    rsi_oversold: 'RSI Oversold',
    rsi_overbought: 'RSI Overbought',
    macd_bullish_cross: 'MACD Cross',
    macd_bearish_cross: 'MACD Cross',
    bollinger_squeeze: 'BB Squeeze',
    bollinger_breakout_up: 'BB Breakout',
    bollinger_breakout_down: 'BB Breakout',
    stochastic_oversold: 'Stoch Oversold',
    stochastic_overbought: 'Stoch Overbought',
    stochastic_bullish_cross: 'Stoch Cross',
    stochastic_bearish_cross: 'Stoch Cross',
    adx_trend_start: 'ADX Trend',
    adx_trend_strong: 'ADX Strong',
    golden_cross: 'Golden Cross',
    death_cross: 'Death Cross',
    divergence_bullish: 'Bull Divergence',
    divergence_bearish: 'Bear Divergence',
  };

  // Deduplicate by short name
  const seen = new Set<string>();
  const shortNames: string[] = [];
  for (const signal of signals) {
    const name = signalNames[signal.type] ?? signal.type;
    if (!seen.has(name)) {
      seen.add(name);
      shortNames.push(name);
    }
  }

  let result = shortNames.join(', ');
  if (result.length > maxLength) {
    // Truncate and show count
    result = shortNames.slice(0, 2).join(', ');
    const remaining = shortNames.length - 2;
    if (remaining > 0) {
      result += ` +${remaining}`;
    }
  }
  return result;
}

/**
 * Compose the main tweet text (max 280 chars)
 */
export function composeTweet(input: TweetComposeInput): string {
  const { securityName, ticker, currentPrice, currency, trendInfo, riskReward, signals, hashtags, language } = input;

  const lines: string[] = [];

  // Line 1: Security info
  const tickerDisplay = ticker ? `$${ticker}` : securityName;
  lines.push(`\u{1F4CA} ${tickerDisplay} | ${formatPrice(currentPrice, currency)}`);

  // Line 2: Trend (if available)
  if (trendInfo) {
    const emoji = getTrendEmoji(trendInfo.direction);
    const label = getTrendLabel(trendInfo.direction, trendInfo.strength, language);
    lines.push(`${emoji} Trend: ${label}`);
  }

  // Line 3: Signals (if available)
  if (signals && signals.length > 0) {
    const signalText = getSignalSummary(signals, 40);
    if (signalText) {
      lines.push(`\u{26A1} ${signalText}`);
    }
  }

  // Line 4: Risk/Reward (if available)
  if (riskReward?.entryPrice && riskReward?.stopLoss && riskReward?.takeProfit) {
    const entry = Math.round(riskReward.entryPrice);
    const sl = Math.round(riskReward.stopLoss);
    const tp = Math.round(riskReward.takeProfit);
    const ratio = riskReward.riskRewardRatio
      ? ` (R/R 1:${riskReward.riskRewardRatio.toFixed(1)})`
      : '';
    lines.push(`\u{1F3AF} Entry ${entry} | SL ${sl} | TP ${tp}${ratio}`);
  }

  // Hashtags
  if (hashtags.trim()) {
    lines.push(hashtags.trim());
  }

  let text = lines.join('\n');

  // Truncate to 280 chars if needed (drop lines from middle)
  while (text.length > TWEET_MAX_LENGTH && lines.length > 2) {
    // Remove the line before hashtags (keep first and last)
    lines.splice(lines.length - 2, 1);
    text = lines.join('\n');
  }

  // Final safety truncation
  if (text.length > TWEET_MAX_LENGTH) {
    text = text.substring(0, TWEET_MAX_LENGTH - 1) + '\u{2026}';
  }

  return text;
}

/**
 * Split analysis text into thread chunks (max 280 chars each)
 */
export function splitIntoThreadChunks(text: string, maxLength: number = TWEET_MAX_LENGTH): string[] {
  if (text.length <= maxLength) return [text];

  const chunks: string[] = [];
  const sentences = text.split(/(?<=[.!?])\s+/);
  let current = '';

  for (const sentence of sentences) {
    const candidate = current ? `${current} ${sentence}` : sentence;
    if (candidate.length <= maxLength) {
      current = candidate;
    } else {
      if (current) chunks.push(current);
      // If single sentence is too long, split by words
      if (sentence.length > maxLength) {
        const words = sentence.split(' ');
        current = '';
        for (const word of words) {
          const next = current ? `${current} ${word}` : word;
          if (next.length <= maxLength) {
            current = next;
          } else {
            if (current) chunks.push(current);
            current = word;
          }
        }
      } else {
        current = sentence;
      }
    }
  }
  if (current) chunks.push(current);

  return chunks;
}
