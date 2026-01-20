import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

// Extended mock data for portfolio tests
const portfolioMockData = {
  ...mockData,
  portfolios: [
    { id: 1, uuid: 'portfolio-1', name: 'Hauptdepot', referenceAccountId: 1, isRetired: false },
    { id: 2, uuid: 'portfolio-2', name: 'Sparplan-Depot', referenceAccountId: 2, isRetired: false },
    { id: 3, uuid: 'portfolio-3', name: 'Altes Depot', referenceAccountId: 3, isRetired: true },
  ],
  accounts: [
    { id: 1, uuid: 'account-1', name: 'Girokonto', currency: 'EUR', isRetired: false },
    { id: 2, uuid: 'account-2', name: 'Tagesgeld', currency: 'EUR', isRetired: false },
    { id: 3, uuid: 'account-3', name: 'Altes Konto', currency: 'EUR', isRetired: true },
  ],
};

async function injectPortfolioMocks(page: any) {
  await page.addInitScript((data: typeof portfolioMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[Portfolio Mock] invoke:', cmd, args);

          switch (cmd) {
            case 'get_pp_portfolios':
              return data.portfolios;
            case 'get_accounts':
              return data.accounts;
            case 'get_all_holdings':
              return data.holdings;
            case 'get_portfolio_history':
              return data.portfolioHistory;
            case 'get_invested_capital_history':
              return data.investedCapitalHistory;
            case 'calculate_performance':
              return data.performance;
            case 'create_portfolio':
              return { id: Date.now(), ...args?.portfolio };
            case 'update_portfolio':
              return { id: args?.id, ...args?.portfolio };
            case 'delete_portfolio':
              return null;
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
  }, portfolioMockData);
}

test.describe('Portfolio View', () => {
  test.beforeEach(async ({ page }) => {
    await injectPortfolioMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Navigation zu Portfolio View funktioniert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="portfolio"]');
    expect(await navItem.count()).toBeGreaterThan(0);

    await navItem.click();
    await page.waitForTimeout(500);

    await page.screenshot({
      path: 'playwright-report/screenshots/portfolio-view.png',
      fullPage: true,
    });
  });

  test('Portfolio-Liste wird angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="portfolio"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Check for portfolio items
    const hasHauptdepot = await page.locator('text=Hauptdepot').count() > 0;
    const hasSparplan = await page.locator('text=Sparplan').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/portfolio-list.png',
      fullPage: true,
    });

    expect(hasHauptdepot || hasSparplan).toBeTruthy();
  });

  test('Neues Portfolio erstellen Button existiert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="portfolio"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const createBtn = page.locator('button:has-text("Neues Portfolio"), button:has-text("Portfolio erstellen"), button:has-text("Neu")');
    const hasCreateBtn = await createBtn.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/portfolio-create-btn.png',
    });

    expect(hasCreateBtn).toBeTruthy();
  });

  test('Portfolio Details werden angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="portfolio"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for detail elements
    const detailSelectors = [
      'text=/Depotwert|Wert|Value/i',
      'text=/Performance|TTWROR|IRR/i',
      'text=/Gewinn|G\\/V/i',
    ];

    let foundDetail = false;
    for (const selector of detailSelectors) {
      if (await page.locator(selector).count() > 0) {
        foundDetail = true;
        break;
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/portfolio-details.png',
      fullPage: true,
    });

    expect(foundDetail).toBeTruthy();
  });
});
