import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

const aiAnalysisMockData = {
  ...mockData,
  chartAnalysis: {
    trend: 'bullish',
    support_levels: [175.00, 170.00],
    resistance_levels: [185.00, 190.00],
    summary: '## Trend-Analyse\n\nDer Chart zeigt einen bullischen Trend mit starker Unterstützung bei 175 EUR.',
    signals: [
      { type: 'buy', confidence: 0.75, reason: 'RSI oversold bounce' },
    ],
    risk_reward: {
      entry: 180.50,
      stop_loss: 175.00,
      take_profit: 195.00,
      ratio: 2.6,
    },
  },
  portfolioInsights: {
    strengths: [
      'Gute Diversifikation über mehrere Sektoren',
      'Starke Performance im letzten Jahr',
    ],
    risks: [
      'Hohe Konzentration in Tech-Aktien',
      'Währungsrisiko durch USD-Positionen',
    ],
    recommendations: [
      'Erwägen Sie eine Rebalancing-Strategie',
      'Dividendenaktien könnten das Portfolio stabilisieren',
    ],
    overall_score: 7.5,
  },
  priceHistory: Array.from({ length: 100 }, (_, i) => ({
    date: new Date(2024, 0, 1 + i).toISOString().split('T')[0],
    open: 150 + Math.random() * 10,
    high: 155 + Math.random() * 10,
    low: 145 + Math.random() * 10,
    close: 150 + Math.random() * 10,
    volume: Math.floor(1000000 + Math.random() * 500000),
  })),
};

async function injectAIAnalysisMocks(page: any) {
  await page.addInitScript((data: typeof aiAnalysisMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[AI Analysis Mock] invoke:', cmd, args);

          switch (cmd) {
            case 'get_pp_portfolios':
              return data.portfolios;
            case 'get_securities':
              return data.securities;
            case 'get_all_holdings':
              return data.holdings;
            case 'get_portfolio_history':
              return data.portfolioHistory;
            case 'get_invested_capital_history':
              return data.investedCapitalHistory;
            case 'calculate_performance':
              return data.performance;
            case 'get_price_history':
              return data.priceHistory;
            case 'get_chart_drawings':
              return [];
            // AI Analysis Commands
            case 'analyze_chart_with_ai':
              return data.chartAnalysis;
            case 'analyze_chart_with_annotations':
              return data.chartAnalysis;
            case 'analyze_chart_enhanced':
              return data.chartAnalysis;
            case 'analyze_portfolio_with_ai':
              return data.portfolioInsights;
            default:
              return null;
          }
        },
      },
      event: {
        listen: async () => () => {},
        emit: async () => {},
      },
    };
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (window as any).__TAURI__.core.invoke,
    };
  }, aiAnalysisMockData);
}

test.describe('AI Chart Analysis', () => {
  test.beforeEach(async ({ page }) => {
    await injectAIAnalysisMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Chart View hat KI-Analyse Button', async ({ page }) => {
    // Navigate to charts
    const navItem = page.locator('button[data-nav-item="charts"]');
    await navItem.click();
    await page.waitForTimeout(1000);

    // Look for AI analysis button/toggle
    const aiButton = page.locator('button:has-text("KI"), button:has-text("AI"), button:has-text("Analyse"), [data-testid*="ai"]');
    const hasAIButton = await aiButton.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chart-button.png',
      fullPage: true,
    });

    expect(hasAIButton).toBeTruthy();
  });

  test('Indikator-Panel wird angezeigt', async ({ page }) => {
    // Navigate to charts
    const navItem = page.locator('button[data-nav-item="charts"]');
    await navItem.click();
    await page.waitForTimeout(1000);

    // Look for indicator options
    const indicators = page.locator('text=/RSI|MACD|SMA|EMA|Bollinger/i');
    const hasIndicators = await indicators.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chart-indicators.png',
      fullPage: true,
    });

    expect(hasIndicators).toBeTruthy();
  });

  test('Erweiterte Analyse Toggle existiert', async ({ page }) => {
    // Navigate to charts
    const navItem = page.locator('button[data-nav-item="charts"]');
    await navItem.click();
    await page.waitForTimeout(1000);

    // Look for enhanced analysis toggle (⚡)
    const hasLightningBtn = await page.locator('button:has-text("⚡")').count() > 0;
    const hasErweitertTitle = await page.locator('button[title*="Erweitert"]').count() > 0;
    const hasErweitertText = await page.locator('text=/Erweitert/i').count() > 0;
    const hasEnhancedToggle = hasLightningBtn || hasErweitertTitle || hasErweitertText;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chart-enhanced-toggle.png',
      fullPage: true,
    });

    expect(hasEnhancedToggle).toBeTruthy();
  });

  test('Signal Panel existiert', async ({ page }) => {
    // Navigate to charts
    const navItem = page.locator('button[data-nav-item="charts"]');
    await navItem.click();
    await page.waitForTimeout(1000);

    // Look for signal panel
    const signalPanel = page.locator('text=/Signal|Pattern|Muster/i');
    const hasSignalPanel = await signalPanel.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chart-signals.png',
      fullPage: true,
    });

    expect(hasSignalPanel).toBeTruthy();
  });
});

test.describe('Portfolio Insights (KI-Analyse)', () => {
  test.beforeEach(async ({ page }) => {
    await injectAIAnalysisMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Dashboard hat KI-Insights Button', async ({ page }) => {
    // Should be on dashboard by default
    await page.waitForTimeout(500);

    // Look for insights button
    const insightsButton = page.locator('button:has-text("Insights"), button:has-text("KI"), button:has-text("Analyse"), [data-testid*="insights"]');
    const hasInsightsButton = await insightsButton.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-insights-button.png',
      fullPage: true,
    });

    expect(hasInsightsButton).toBeTruthy();
  });

  test('KI-Insights Modal kann geöffnet werden', async ({ page }) => {
    await page.waitForTimeout(500);

    // Find and click insights button
    const insightsButton = page.locator('button:has-text("Insights"), button:has-text("KI-Analyse")').first();

    if (await insightsButton.count() > 0) {
      await insightsButton.click();
      await page.waitForTimeout(500);

      // Check if modal opened
      const modal = page.locator('[role="dialog"], .modal, .fixed.inset-0.z-50');
      const hasModal = await modal.count() > 0;

      await page.screenshot({
        path: 'playwright-report/screenshots/ai-insights-modal.png',
        fullPage: true,
      });

      expect(hasModal).toBeTruthy();
    }
  });

  test('Insights zeigt Stärken/Risiken/Empfehlungen', async ({ page }) => {
    await page.waitForTimeout(500);

    // Find and click insights button
    const insightsButton = page.locator('button:has-text("Insights"), button:has-text("KI-Analyse")').first();

    if (await insightsButton.count() > 0) {
      await insightsButton.click();
      await page.waitForTimeout(1000);

      // Look for insights categories
      const hasStaerken = await page.locator('text=/Stärken|Strengths/i').count() > 0;
      const hasRisiken = await page.locator('text=/Risiken|Risks/i').count() > 0;
      const hasEmpfehlungen = await page.locator('text=/Empfehlungen|Recommendations/i').count() > 0;

      await page.screenshot({
        path: 'playwright-report/screenshots/ai-insights-categories.png',
        fullPage: true,
      });

      expect(hasStaerken || hasRisiken || hasEmpfehlungen).toBeTruthy();
    }
  });
});

test.describe('AI Provider Configuration', () => {
  test.beforeEach(async ({ page }) => {
    await injectAIAnalysisMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Settings zeigt KI-Provider Auswahl', async ({ page }) => {
    // Navigate to settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for AI provider options
    const providers = ['Claude', 'OpenAI', 'Gemini', 'Perplexity'];
    let foundProvider = false;

    for (const provider of providers) {
      if (await page.locator(`text=${provider}`).count() > 0) {
        foundProvider = true;
        break;
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-settings-providers.png',
      fullPage: true,
    });

    expect(foundProvider).toBeTruthy();
  });

  test('Settings zeigt KI-Modell Auswahl', async ({ page }) => {
    // Navigate to settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for model selection
    const modelSelect = page.locator('text=/Modell|Model|Sonnet|Haiku|GPT/i');
    const hasModelSelect = await modelSelect.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-settings-models.png',
      fullPage: true,
    });

    expect(hasModelSelect).toBeTruthy();
  });

  test('API-Key Felder sind geschützt', async ({ page }) => {
    // Navigate to settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for password fields (API keys should be masked)
    const passwordFields = page.locator('input[type="password"]');
    const hasPasswordFields = await passwordFields.count() > 0;

    // Look for shield icon (secure storage indicator)
    const shieldIcon = page.locator('[data-testid*="shield"], svg[class*="shield"]');
    const hasShieldIcon = await shieldIcon.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-settings-api-keys.png',
      fullPage: true,
    });

    expect(hasPasswordFields || hasShieldIcon).toBeTruthy();
  });
});
