/**
 * Tweet Image Generator
 *
 * Creates an optimized chart image with overlay (header/footer bars)
 * for sharing on X/Twitter. Output: 1200×900 JPEG, typically 200-400KB.
 */

import type { TrendInfo, RiskRewardAnalysis } from './types';
import type { TechnicalSignal } from './signals';

const TARGET_WIDTH = 1200;
const TARGET_HEIGHT = 900;
const HEADER_HEIGHT = 80;
const FOOTER_HEIGHT = 60;
const JPEG_QUALITY = 0.90;

interface TweetImageInput {
  securityName: string;
  ticker?: string | null;
  currentPrice: number;
  currency: string;
  trendInfo?: TrendInfo;
  riskReward?: RiskRewardAnalysis;
  signals?: TechnicalSignal[];
  includeWatermark: boolean;
}

function getTrendColor(direction: string): string {
  switch (direction) {
    case 'bullish': return '#22c55e';
    case 'bearish': return '#ef4444';
    default: return '#a3a3a3';
  }
}

function getTrendLabel(direction: string, strength: string): string {
  const dir = direction === 'bullish' ? 'Bullish' : direction === 'bearish' ? 'Bearish' : 'Neutral';
  const str = strength === 'strong' ? 'Strong' : strength === 'moderate' ? 'Moderate' : 'Weak';
  return `${dir} (${str})`;
}

/**
 * Capture the chart container and create an X-optimized image with overlay
 */
export function captureChartForTweet(
  container: HTMLElement,
  input: TweetImageInput
): { base64: string; width: number; height: number } {
  const canvases = container.querySelectorAll('canvas');
  if (canvases.length === 0) {
    throw new Error('No canvas elements found in container');
  }

  // Capture original chart
  const containerRect = container.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;

  const chartCanvas = document.createElement('canvas');
  chartCanvas.width = containerRect.width * dpr;
  chartCanvas.height = containerRect.height * dpr;

  const chartCtx = chartCanvas.getContext('2d');
  if (!chartCtx) throw new Error('Failed to get chart canvas context');

  chartCtx.scale(dpr, dpr);

  // Fill chart background
  const isDark = document.documentElement.classList.contains('dark');
  chartCtx.fillStyle = isDark ? '#1a1a2e' : '#ffffff';
  chartCtx.fillRect(0, 0, containerRect.width, containerRect.height);

  // Draw each canvas layer
  canvases.forEach((canvas) => {
    const rect = canvas.getBoundingClientRect();
    const x = rect.left - containerRect.left;
    const y = rect.top - containerRect.top;
    chartCtx.drawImage(canvas, 0, 0, canvas.width, canvas.height, x, y, rect.width, rect.height);
  });

  // Create final composite canvas (1200×900)
  const finalCanvas = document.createElement('canvas');
  finalCanvas.width = TARGET_WIDTH;
  finalCanvas.height = TARGET_HEIGHT;

  const ctx = finalCanvas.getContext('2d');
  if (!ctx) throw new Error('Failed to get final canvas context');

  // Background
  const bgColor = isDark ? '#0f0f23' : '#f8fafc';
  ctx.fillStyle = bgColor;
  ctx.fillRect(0, 0, TARGET_WIDTH, TARGET_HEIGHT);

  // === Header Bar (80px) ===
  const headerBg = isDark ? '#1a1a2e' : '#ffffff';
  ctx.fillStyle = headerBg;
  ctx.fillRect(0, 0, TARGET_WIDTH, HEADER_HEIGHT);

  // Header border
  ctx.strokeStyle = isDark ? '#2d2d4a' : '#e2e8f0';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, HEADER_HEIGHT);
  ctx.lineTo(TARGET_WIDTH, HEADER_HEIGHT);
  ctx.stroke();

  // Security name + ticker
  ctx.fillStyle = isDark ? '#e2e8f0' : '#1e293b';
  ctx.font = 'bold 28px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
  const tickerDisplay = input.ticker ? `$${input.ticker}` : input.securityName;
  ctx.fillText(tickerDisplay, 24, 36);

  // Security full name (if ticker shown)
  if (input.ticker && input.securityName !== input.ticker) {
    ctx.fillStyle = isDark ? '#94a3b8' : '#64748b';
    ctx.font = '16px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
    ctx.fillText(input.securityName, 24, 60);
  }

  // Price (right side)
  const priceText = `${input.currentPrice.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${input.currency}`;
  ctx.fillStyle = isDark ? '#e2e8f0' : '#1e293b';
  ctx.font = 'bold 28px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
  const priceWidth = ctx.measureText(priceText).width;
  ctx.fillText(priceText, TARGET_WIDTH - priceWidth - 24, 36);

  // Trend badge (right side, below price)
  if (input.trendInfo) {
    const trendColor = getTrendColor(input.trendInfo.direction);
    const trendText = getTrendLabel(input.trendInfo.direction, input.trendInfo.strength);
    ctx.font = 'bold 14px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
    const trendWidth = ctx.measureText(trendText).width;
    const badgeX = TARGET_WIDTH - trendWidth - 36;
    const badgeY = 48;

    // Badge background
    ctx.fillStyle = trendColor + '20';
    ctx.beginPath();
    ctx.roundRect(badgeX - 4, badgeY - 12, trendWidth + 16, 20, 4);
    ctx.fill();

    // Badge text
    ctx.fillStyle = trendColor;
    ctx.fillText(trendText, badgeX + 4, badgeY + 2);
  }

  // === Chart Area ===
  const chartAreaY = HEADER_HEIGHT;
  const chartAreaHeight = TARGET_HEIGHT - HEADER_HEIGHT - FOOTER_HEIGHT;

  // Scale and draw chart into the chart area
  ctx.drawImage(
    chartCanvas,
    0, 0, chartCanvas.width, chartCanvas.height,
    0, chartAreaY, TARGET_WIDTH, chartAreaHeight
  );

  // === Footer Bar (60px) ===
  const footerY = TARGET_HEIGHT - FOOTER_HEIGHT;
  const footerBg = isDark ? '#1a1a2e' : '#ffffff';
  ctx.fillStyle = footerBg;
  ctx.fillRect(0, footerY, TARGET_WIDTH, FOOTER_HEIGHT);

  // Footer border
  ctx.strokeStyle = isDark ? '#2d2d4a' : '#e2e8f0';
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(0, footerY);
  ctx.lineTo(TARGET_WIDTH, footerY);
  ctx.stroke();

  // Signals summary (left)
  if (input.signals && input.signals.length > 0) {
    const signalNames = [...new Set(input.signals.slice(0, 3).map(s => {
      const shortNames: Record<string, string> = {
        macd_bullish_cross: 'MACD Cross',
        macd_bearish_cross: 'MACD Cross',
        rsi_oversold: 'RSI OS',
        rsi_overbought: 'RSI OB',
        golden_cross: 'Golden Cross',
        death_cross: 'Death Cross',
        bollinger_breakout_up: 'BB Breakout',
        bollinger_breakout_down: 'BB Breakout',
        divergence_bullish: 'Bull Div',
        divergence_bearish: 'Bear Div',
      };
      return shortNames[s.type] ?? s.type.replace(/_/g, ' ');
    }))];

    ctx.fillStyle = isDark ? '#94a3b8' : '#64748b';
    ctx.font = '14px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
    ctx.fillText(`Signals: ${signalNames.join(', ')}`, 24, footerY + 36);
  }

  // R/R (center)
  if (input.riskReward?.riskRewardRatio) {
    ctx.fillStyle = isDark ? '#94a3b8' : '#64748b';
    ctx.font = '14px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
    const rrText = `R/R 1:${input.riskReward.riskRewardRatio.toFixed(1)}`;
    const rrWidth = ctx.measureText(rrText).width;
    ctx.fillText(rrText, (TARGET_WIDTH - rrWidth) / 2, footerY + 36);
  }

  // Watermark (right)
  if (input.includeWatermark) {
    ctx.fillStyle = isDark ? '#4a4a6a' : '#94a3b8';
    ctx.font = '13px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
    const wmText = 'Portfolio Now';
    const wmWidth = ctx.measureText(wmText).width;
    ctx.fillText(wmText, TARGET_WIDTH - wmWidth - 24, footerY + 36);
  }

  // Export as JPEG
  const dataUrl = finalCanvas.toDataURL('image/jpeg', JPEG_QUALITY);
  const base64 = dataUrl.replace(/^data:image\/jpeg;base64,/, '');

  return {
    base64,
    width: TARGET_WIDTH,
    height: TARGET_HEIGHT,
  };
}
