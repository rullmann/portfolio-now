import { Page } from '@playwright/test';

/**
 * Mock-Daten für Tauri Commands
 */
export const mockData = {
  portfolios: [
    { id: 1, uuid: 'test-portfolio-1', name: 'Hauptdepot', referenceAccountId: 1, isRetired: false },
  ],
  accounts: [
    { id: 1, uuid: 'test-account-1', name: 'Girokonto', currency: 'EUR', isRetired: false },
  ],
  securities: [
    { id: 1, uuid: 'test-sec-1', name: 'Apple Inc.', currency: 'USD', isin: 'US0378331005', ticker: 'AAPL' },
    { id: 2, uuid: 'test-sec-2', name: 'Microsoft Corp.', currency: 'USD', isin: 'US5949181045', ticker: 'MSFT' },
  ],
  holdings: [
    {
      isin: 'US0378331005',
      name: 'Apple Inc.',
      currency: 'USD',
      securityId: 1,
      totalShares: 10,
      currentPrice: 180.50,
      currentValue: 1805.00,
      costBasis: 1500.00,
      purchasePrice: 150.00,
      gainLoss: 305.00,
      gainLossPercent: 20.33,
      dividendsTotal: 45.00,
      portfolios: [],
    },
    {
      isin: 'US5949181045',
      name: 'Microsoft Corp.',
      currency: 'USD',
      securityId: 2,
      totalShares: 5,
      currentPrice: 420.00,
      currentValue: 2100.00,
      costBasis: 1800.00,
      purchasePrice: 360.00,
      gainLoss: 300.00,
      gainLossPercent: 16.67,
      dividendsTotal: 25.00,
      portfolios: [],
    },
  ],
  portfolioHistory: [
    { date: '2024-01-01', value: 3000 },
    { date: '2024-02-01', value: 3200 },
    { date: '2024-03-01', value: 3500 },
    { date: '2024-04-01', value: 3400 },
    { date: '2024-05-01', value: 3800 },
    { date: '2024-06-01', value: 3905 },
  ],
  investedCapitalHistory: [
    { date: '2024-01-01', value: 3300 },
    { date: '2024-02-01', value: 3300 },
    { date: '2024-03-01', value: 3300 },
    { date: '2024-04-01', value: 3300 },
    { date: '2024-05-01', value: 3300 },
    { date: '2024-06-01', value: 3300 },
  ],
  performance: {
    ttwror: 0.1833,
    ttwrorAnnualized: 0.42,
    irr: 0.38,
    irrConverged: true,
    totalInvested: 3300,
    currentValue: 3905,
    absoluteGain: 605,
    days: 180,
  },
  watchlists: [
    { id: 1, name: 'Tech Stocks', securities: [] },
  ],
  // Portfolio Insights Mock Data
  portfolioInsights: {
    analysis: '## Portfolio-Analyse\n\n**Stärken:**\n- Gute Diversifikation\n- Starke Performance\n\n**Risiken:**\n- Tech-Konzentration\n\n**Empfehlungen:**\n- Rebalancing erwägen',
    provider: 'claude',
    model: 'claude-sonnet-4-5',
    tokensUsed: 1500,
  },
  technicalRanking: {
    analysis: '## Nachkauf-Ranking\n\n**🟢 Apple Inc. - Score: 75/100**\n  - RSI überverkauft (28)\n\n**🟡 Microsoft Corp. - Score: 52/100**\n  - Neutral\n\n### Legende\n- 🟢 Score 70-100: Gute Nachkauf-Gelegenheit\n- 🟡 Score 40-69: Neutral\n- 🔴 Score 0-39: Aktuell keine Nachkauf-Empfehlung',
    provider: 'local',
    model: 'technical-analysis',
    tokensUsed: null,
  },
};

/**
 * Injecte Tauri-Mocks in die Seite
 */
export async function injectTauriMocks(page: Page): Promise<void> {
  await page.addInitScript((data) => {
    // Mock window.__TAURI__
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[Tauri Mock] invoke:', cmd, args);

          switch (cmd) {
            case 'get_pp_portfolios':
              return data.portfolios;
            case 'get_accounts':
              return data.accounts;
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
            case 'get_watchlists':
              return data.watchlists;
            case 'get_taxonomies':
              return [];
            case 'get_investment_plans':
              return [];
            case 'get_benchmarks':
              return [];
            case 'get_transactions':
              return [];
            case 'get_dashboard_layout':
              return null;
            // Portfolio Insights Command
            case 'analyze_portfolio_with_ai':
              const request = args?.request || args;
              const technicalOnly = request?.technicalOnly || request?.technical_only;
              if (technicalOnly) {
                return data.technicalRanking;
              }
              return data.portfolioInsights;
            default:
              console.warn('[Tauri Mock] Unknown command:', cmd);
              return null;
          }
        },
      },
      event: {
        listen: async (event: string, handler: any) => {
          console.log('[Tauri Mock] listen:', event);
          return () => {}; // Return unsubscribe function
        },
        emit: async (event: string, payload: any) => {
          console.log('[Tauri Mock] emit:', event, payload);
        },
      },
    };

    // Also mock @tauri-apps/api imports
    (window as any).__TAURI_INTERNALS__ = {
      invoke: (window as any).__TAURI__.core.invoke,
    };
  }, mockData);
}

/**
 * Warte bis die App geladen ist
 */
export async function waitForAppReady(page: Page): Promise<void> {
  // Wait for the main app container to be visible
  await page.waitForSelector('[data-testid="app-container"], .app-container, #root > div', {
    state: 'visible',
    timeout: 30000,
  });

  // Wait for loading indicators to disappear
  await page.waitForFunction(() => {
    const loadingElements = document.querySelectorAll('[data-testid="loading"], .loading-indicator');
    return loadingElements.length === 0;
  }, { timeout: 10000 }).catch(() => {
    // Ignore timeout - loading might already be done
  });
}

/**
 * Schließe das WelcomeModal wenn es angezeigt wird
 */
export async function closeWelcomeModal(page: Page): Promise<void> {
  // Wait a bit for modal to potentially appear
  await page.waitForTimeout(500);

  // Check if WelcomeModal is present - look for "Überspringen" button
  const skipButton = page.locator('button:has-text("Überspringen")');
  if (await skipButton.isVisible({ timeout: 2000 }).catch(() => false)) {
    await skipButton.click();
    await page.waitForTimeout(300);
    return;
  }

  // Also try Escape key
  const modal = page.locator('.fixed.inset-0.z-50');
  if (await modal.isVisible({ timeout: 500 }).catch(() => false)) {
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);
  }
}

/**
 * Navigate zu einem bestimmten View
 */
export async function navigateToView(page: Page, viewId: string): Promise<void> {
  // Click on sidebar navigation item
  const navItem = page.locator(`[data-testid="nav-${viewId}"], [data-view="${viewId}"]`);

  if (await navItem.isVisible()) {
    await navItem.click();
  } else {
    // Fallback: Try to find by text content
    const sidebarItem = page.locator(`nav button, nav a`).filter({ hasText: new RegExp(viewId, 'i') });
    if (await sidebarItem.count() > 0) {
      await sidebarItem.first().click();
    }
  }

  // Wait for view to load
  await page.waitForTimeout(500);
}
