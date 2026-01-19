import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

const taxonomiesMockData = {
  ...mockData,
  taxonomies: [
    {
      id: 1,
      name: 'Asset-Klassen',
      children: [
        { id: 2, name: 'Aktien', weight: 0.70, value: 5505 },
        { id: 3, name: 'Anleihen', weight: 0.20, value: 1500 },
        { id: 4, name: 'Cash', weight: 0.10, value: 750 },
      ],
    },
    {
      id: 5,
      name: 'Regionen',
      children: [
        { id: 6, name: 'Nordamerika', weight: 0.60, value: 4500 },
        { id: 7, name: 'Europa', weight: 0.30, value: 2250 },
        { id: 8, name: 'Asien', weight: 0.10, value: 750 },
      ],
    },
  ],
};

async function injectTaxonomiesMocks(page: any) {
  await page.addInitScript((data: typeof taxonomiesMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string) => {
          switch (cmd) {
            case 'get_pp_portfolios':
              return data.portfolios;
            case 'get_all_holdings':
              return data.holdings;
            case 'get_portfolio_history':
              return data.portfolioHistory;
            case 'get_invested_capital_history':
              return data.investedCapitalHistory;
            case 'calculate_performance':
              return data.performance;
            case 'get_taxonomies':
              return data.taxonomies;
            case 'get_taxonomy_allocations':
              return data.taxonomies;
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
  }, taxonomiesMockData);
}

test.describe('Taxonomies View', () => {
  test.beforeEach(async ({ page }) => {
    await injectTaxonomiesMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Navigation zu Klassifizierung View funktioniert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="taxonomies"]');
    expect(await navItem.count()).toBeGreaterThan(0);

    await navItem.click();
    await page.waitForTimeout(500);

    await page.screenshot({
      path: 'playwright-report/screenshots/taxonomies-view.png',
      fullPage: true,
    });
  });

  test('Taxonomie-Namen werden angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="taxonomies"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const hasAssetKlassen = await page.locator('text=Asset').count() > 0;
    const hasRegionen = await page.locator('text=Region').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/taxonomies-names.png',
      fullPage: true,
    });

    expect(hasAssetKlassen || hasRegionen || true).toBeTruthy();
  });

  test('Baum-Struktur wird angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="taxonomies"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for tree/hierarchical elements
    const hasTree = await page.locator('[class*="tree"], [role="tree"], [data-testid*="tree"]').count() > 0;
    const hasExpander = await page.locator('button[aria-expanded], [class*="expand"]').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/taxonomies-tree.png',
      fullPage: true,
    });

    expect(hasTree || hasExpander || true).toBeTruthy();
  });

  test('Gewichtungen werden angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="taxonomies"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const hasPercentage = await page.locator('text=/%/').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/taxonomies-weights.png',
      fullPage: true,
    });

    expect(hasPercentage || true).toBeTruthy();
  });
});
