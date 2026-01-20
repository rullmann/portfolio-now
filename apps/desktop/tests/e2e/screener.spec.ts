import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

const screenerMockData = {
  ...mockData,
  screenerResults: [
    {
      id: 1,
      name: 'Apple Inc.',
      isin: 'US0378331005',
      ticker: 'AAPL',
      price: 180.50,
      change: 2.3,
      pe: 28.5,
      marketCap: 2800000000000,
      dividendYield: 0.5,
    },
    {
      id: 2,
      name: 'Microsoft Corp.',
      isin: 'US5949181045',
      ticker: 'MSFT',
      price: 420.00,
      change: -1.2,
      pe: 35.2,
      marketCap: 3100000000000,
      dividendYield: 0.7,
    },
  ],
};

async function injectScreenerMocks(page: any) {
  await page.addInitScript((data: typeof screenerMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string) => {
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
            case 'search_external_securities':
              return data.screenerResults;
            case 'screen_securities':
              return data.screenerResults;
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
  }, screenerMockData);
}

test.describe('Screener View', () => {
  test.beforeEach(async ({ page }) => {
    await injectScreenerMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Navigation zu Screener View funktioniert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="screener"]');
    expect(await navItem.count()).toBeGreaterThan(0);

    await navItem.click();
    await page.waitForTimeout(500);

    await page.screenshot({
      path: 'playwright-report/screenshots/screener-view.png',
      fullPage: true,
    });
  });

  test('Suchfeld existiert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="screener"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const hasSearch = await page.locator('input[type="search"], input[placeholder*="Such"], input[placeholder*="Search"]').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/screener-search.png',
      fullPage: true,
    });

    expect(hasSearch).toBeTruthy();
  });

  test('Filter-Optionen existieren', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="screener"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for filter elements
    const hasFilters = await page.locator('text=/Filter|P\\/E|KGV|Market Cap|Dividende/i').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/screener-filters.png',
      fullPage: true,
    });

    expect(hasFilters).toBeTruthy();
  });

  test('Ergebnistabelle wird angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="screener"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for table
    const hasTable = await page.locator('table, [role="table"], [data-testid*="table"]').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/screener-results.png',
      fullPage: true,
    });

    expect(hasTable).toBeTruthy();
  });

  test('Sortierung kann geändert werden', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="screener"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for sortable headers
    const hasSortable = await page.locator('th[class*="sort"], button[aria-sort], th button').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/screener-sorting.png',
      fullPage: true,
    });

    expect(hasSortable).toBeTruthy();
  });
});
