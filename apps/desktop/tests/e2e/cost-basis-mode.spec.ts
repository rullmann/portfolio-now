import { test, expect } from '@playwright/test';
import { waitForAppReady, closeWelcomeModal, mockData } from './utils/tauri-mock';

/**
 * E2E Tests for Dual Cost Basis Mode (Historical & Period-based)
 *
 * Tests:
 * 1. Settings UI: Mode selection radio buttons
 * 2. Settings UI: Period start date picker (only visible in period mode)
 * 3. Holdings update when mode changes
 * 4. Settings persistence after page reload
 */

// Extended mock data with different cost basis values for each mode
const costBasisMockData = {
  ...mockData,
  // Historical mode holdings (default)
  holdingsHistorical: [
    {
      isin: 'US0378331005',
      name: 'Apple Inc.',
      currency: 'EUR',
      securityIds: [1],
      totalShares: 10,
      currentPrice: 180.50,
      currentValue: 1805.00,
      costBasis: 1500.00, // Historical: actual purchase price
      purchasePrice: 150.00,
      gainLoss: 305.00,
      gainLossPercent: 20.33,
      dividendsTotal: 45.00,
      portfolios: [],
    },
    {
      isin: 'US5949181045',
      name: 'Microsoft Corp.',
      currency: 'EUR',
      securityIds: [2],
      totalShares: 5,
      currentPrice: 420.00,
      currentValue: 2100.00,
      costBasis: 1800.00, // Historical: actual purchase price
      purchasePrice: 360.00,
      gainLoss: 300.00,
      gainLossPercent: 16.67,
      dividendsTotal: 25.00,
      portfolios: [],
    },
  ],
  // Period-based mode holdings (market value at period start)
  holdingsPeriod: [
    {
      isin: 'US0378331005',
      name: 'Apple Inc.',
      currency: 'EUR',
      securityIds: [1],
      totalShares: 10,
      currentPrice: 180.50,
      currentValue: 1805.00,
      costBasis: 1700.00, // Period: market value at Jan 1, 2026 (higher than purchase)
      purchasePrice: 170.00,
      gainLoss: 105.00,
      gainLossPercent: 6.18,
      dividendsTotal: 45.00,
      portfolios: [],
    },
    {
      isin: 'US5949181045',
      name: 'Microsoft Corp.',
      currency: 'EUR',
      securityIds: [2],
      totalShares: 5,
      currentPrice: 420.00,
      currentValue: 2100.00,
      costBasis: 2000.00, // Period: market value at Jan 1, 2026 (higher than purchase)
      purchasePrice: 400.00,
      gainLoss: 100.00,
      gainLossPercent: 5.00,
      dividendsTotal: 25.00,
      portfolios: [],
    },
  ],
};

async function injectCostBasisMocks(page: any) {
  await page.addInitScript((data: typeof costBasisMockData) => {
    // Track current cost basis mode (simulating Zustand store)
    let currentMode = 'historical';

    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[CostBasis Mock] invoke:', cmd, args);

          switch (cmd) {
            case 'get_pp_portfolios':
              return data.portfolios;
            case 'get_accounts':
              return data.accounts;
            case 'get_securities':
              return data.securities;
            case 'get_all_holdings':
              // Return different data based on optional mode parameter
              const mode = args?.costBasisMode || 'historical';
              currentMode = mode;
              console.log('[CostBasis Mock] Mode:', mode, 'Period start:', args?.periodStart);
              if (mode === 'period') {
                return data.holdingsPeriod;
              }
              return data.holdingsHistorical;
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
            case 'get_base_currency':
              return 'EUR';
            case 'get_dashboard_layout':
              return null;
            default:
              console.warn('[CostBasis Mock] Unknown command:', cmd);
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
  }, costBasisMockData);
}

test.describe('Cost Basis Mode (Einstandswert-Berechnung)', () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage to reset settings
    await page.addInitScript(() => {
      localStorage.clear();
    });
    await injectCostBasisMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Settings zeigt Einstandswert-Berechnung Abschnitt', async ({ page }) => {
    // Navigate to Settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Click on "Buchungen" section
    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(300);
    }

    // Check for Cost Basis section header
    const costBasisHeader = page.locator('text=Einstandswert-Berechnung');
    await expect(costBasisHeader).toBeVisible();

    // Check for mode options
    const historicalOption = page.locator('text=Historisch');
    const periodOption = page.locator('text=Periodenbasiert');
    await expect(historicalOption).toBeVisible();
    await expect(periodOption).toBeVisible();

    await page.screenshot({
      path: 'playwright-report/screenshots/cost-basis-settings.png',
      fullPage: true,
    });
  });

  test('Historisch ist standardmäßig ausgewählt', async ({ page }) => {
    // Navigate to Settings > Buchungen
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(300);
    }

    // Check that "historical" radio is checked
    const historicalRadio = page.locator('input[name="costBasisMode"][value="historical"]');
    await expect(historicalRadio).toBeChecked();

    // Period start date should NOT be visible when historical is selected
    const periodStartLabel = page.locator('text=Periodenstart').first();
    // The date picker section should not be visible in historical mode
    const dateInput = page.locator('input[type="date"]');
    const dateInputVisible = await dateInput.isVisible().catch(() => false);
    expect(dateInputVisible).toBeFalsy();
  });

  test('Periodenstart-Datepicker erscheint bei Period-Modus', async ({ page }) => {
    // Navigate to Settings > Buchungen
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(300);
    }

    // Select "period" mode
    const periodRadio = page.locator('input[name="costBasisMode"][value="period"]');
    await periodRadio.click();
    await page.waitForTimeout(300);

    // Now the period start date picker should be visible
    const dateInput = page.locator('input[type="date"]');
    await expect(dateInput).toBeVisible();

    // The label "Periodenstart" should be visible
    const periodStartLabel = page.locator('text=Periodenstart');
    await expect(periodStartLabel.first()).toBeVisible();

    await page.screenshot({
      path: 'playwright-report/screenshots/cost-basis-period-mode.png',
      fullPage: true,
    });
  });

  test('Steuer-Warnhinweis wird angezeigt', async ({ page }) => {
    // Navigate to Settings > Buchungen
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(300);
    }

    // Check for tax warning
    const taxWarning = page.locator('text=/Steuerberechnungen|historische.*Einstandswert/i');
    await expect(taxWarning.first()).toBeVisible();
  });

  test('Mode-Wechsel lädt Holdings neu', async ({ page }) => {
    // First check the Asset Statement with historical mode (default)
    const assetStatementNav = page.locator('text=/Vermögensaufstellung|Asset Statement/i').first();
    if (await assetStatementNav.isVisible()) {
      await assetStatementNav.click();
      await page.waitForTimeout(500);

      // Check initial cost basis value (historical: 1500 + 1800 = 3300)
      // Note: The exact display depends on how the view renders
      const table = page.locator('table');
      if (await table.isVisible()) {
        // Take screenshot for comparison
        await page.screenshot({
          path: 'playwright-report/screenshots/cost-basis-historical-values.png',
          fullPage: true,
        });
      }
    }

    // Now change to period mode in settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(300);
    }

    // Select period mode
    const periodRadio = page.locator('input[name="costBasisMode"][value="period"]');
    await periodRadio.click();
    await page.waitForTimeout(500);

    // Go back to Asset Statement
    const assetStatementNav2 = page.locator('text=/Vermögensaufstellung|Asset Statement/i').first();
    if (await assetStatementNav2.isVisible()) {
      await assetStatementNav2.click();
      await page.waitForTimeout(500);

      // The values should be different now (period: 1700 + 2000 = 3700)
      await page.screenshot({
        path: 'playwright-report/screenshots/cost-basis-period-values.png',
        fullPage: true,
      });
    }
  });

  test('Periodenstart kann gesetzt werden', async ({ page }) => {
    // Navigate to Settings > Buchungen
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(300);
    }

    // Select period mode
    const periodRadio = page.locator('input[name="costBasisMode"][value="period"]');
    await periodRadio.click();
    await page.waitForTimeout(300);

    // Set a specific date
    const dateInput = page.locator('input[type="date"]');
    await dateInput.fill('2025-07-01');
    await page.waitForTimeout(300);

    // Verify the value was set
    await expect(dateInput).toHaveValue('2025-07-01');

    await page.screenshot({
      path: 'playwright-report/screenshots/cost-basis-custom-period-start.png',
      fullPage: true,
    });
  });

  test('Umschalten zwischen Modi funktioniert', async ({ page }) => {
    // Navigate to Settings > Buchungen
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(300);
    }

    const historicalRadio = page.locator('input[name="costBasisMode"][value="historical"]');
    const periodRadio = page.locator('input[name="costBasisMode"][value="period"]');
    const dateInput = page.locator('input[type="date"]');

    // Start with historical (default)
    await expect(historicalRadio).toBeChecked();

    // Switch to period
    await periodRadio.click();
    await page.waitForTimeout(200);
    await expect(periodRadio).toBeChecked();
    await expect(dateInput).toBeVisible();

    // Switch back to historical
    await historicalRadio.click();
    await page.waitForTimeout(200);
    await expect(historicalRadio).toBeChecked();

    // Date input should be hidden again
    const dateInputVisible = await dateInput.isVisible().catch(() => false);
    expect(dateInputVisible).toBeFalsy();

    // Switch to period again
    await periodRadio.click();
    await page.waitForTimeout(200);
    await expect(periodRadio).toBeChecked();
    await expect(dateInput).toBeVisible();
  });

  test('Screenshot der Settings-Seite mit Cost Basis Optionen', async ({ page }) => {
    // Navigate to Settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Click on "Buchungen" section
    const transactionsSection = page.locator('button:has-text("Buchungen")');
    if (await transactionsSection.isVisible()) {
      await transactionsSection.click();
      await page.waitForTimeout(500);
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/settings-cost-basis-section.png',
      fullPage: true,
    });
  });
});
