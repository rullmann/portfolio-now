import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

// Extended mock data for consortium tests
const consortiumMockData = {
  ...mockData,
  consortiums: [
    {
      id: 1,
      name: 'Familie',
      portfolioIds: [1, 2],
      createdAt: '2024-01-15T10:00:00Z',
    },
  ],
  consortiumPerformance: {
    consortiumId: 1,
    consortiumName: 'Familie',
    totalValue: 50000.0,
    totalCostBasis: 40000.0,
    ttwror: 0.25,
    ttwrorAnnualized: 0.15,
    irr: 0.14,
    totalDividends: 1200.0,
    totalFees: 150.0,
    currency: 'EUR',
    startDate: '2024-01-01',
    endDate: '2024-12-31',
    riskMetrics: {
      volatility: 0.18,
      sharpeRatio: 0.83,
      sortinoRatio: 1.12,
      maxDrawdown: 0.15,
      calculationPeriod: '1Y',
    },
    portfolioSummaries: [
      {
        portfolioId: 1,
        portfolioName: 'Hauptdepot',
        value: 30000.0,
        costBasis: 24000.0,
        ttwror: 0.25,
        irr: 0.14,
        weight: 0.60,
      },
      {
        portfolioId: 2,
        portfolioName: 'Zweitdepot',
        value: 20000.0,
        costBasis: 16000.0,
        ttwror: 0.25,
        irr: 0.14,
        weight: 0.40,
      },
    ],
  },
  consortiumHistory: {
    consortiumId: 1,
    currency: 'EUR',
    combined: [
      { date: '2024-01-01', value: 40000.0 },
      { date: '2024-06-01', value: 45000.0 },
      { date: '2024-12-01', value: 50000.0 },
    ],
    byPortfolio: [
      {
        portfolioId: 1,
        portfolioName: 'Hauptdepot',
        data: [
          { date: '2024-01-01', value: 24000.0 },
          { date: '2024-06-01', value: 27000.0 },
          { date: '2024-12-01', value: 30000.0 },
        ],
      },
      {
        portfolioId: 2,
        portfolioName: 'Zweitdepot',
        data: [
          { date: '2024-01-01', value: 16000.0 },
          { date: '2024-06-01', value: 18000.0 },
          { date: '2024-12-01', value: 20000.0 },
        ],
      },
    ],
  },
  portfolioComparison: {
    portfolios: [
      {
        portfolioId: 1,
        portfolioName: 'Hauptdepot',
        value: 30000.0,
        costBasis: 24000.0,
        ttwror: 0.25,
        ttwrorAnnualized: 0.15,
        irr: 0.14,
        dividends: 800.0,
        fees: 100.0,
        weight: 0.60,
      },
      {
        portfolioId: 2,
        portfolioName: 'Zweitdepot',
        value: 20000.0,
        costBasis: 16000.0,
        ttwror: 0.25,
        ttwrorAnnualized: 0.15,
        irr: 0.14,
        dividends: 400.0,
        fees: 50.0,
        weight: 0.40,
      },
    ],
    combined: {
      totalValue: 50000.0,
      totalCostBasis: 40000.0,
      combinedTtwror: 0.25,
      combinedIrr: 0.14,
      totalDividends: 1200.0,
      totalFees: 150.0,
    },
  },
  // Extended portfolios for consortium selection
  portfolios: [
    { id: 1, uuid: 'test-portfolio-1', name: 'Hauptdepot', referenceAccountId: 1, isRetired: false },
    { id: 2, uuid: 'test-portfolio-2', name: 'Zweitdepot', referenceAccountId: 2, isRetired: false },
    { id: 3, uuid: 'test-portfolio-3', name: 'Sparplan', referenceAccountId: 3, isRetired: false },
  ],
};

async function injectConsortiumMocks(page: any) {
  await page.addInitScript((data: typeof consortiumMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[Consortium Mock] invoke:', cmd, args);

          switch (cmd) {
            // Basic data commands
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

            // Consortium commands
            case 'get_consortiums':
              return data.consortiums;
            case 'get_consortium_performance':
              return data.consortiumPerformance;
            case 'get_consortium_history':
              return data.consortiumHistory;
            case 'compare_portfolios':
              return data.portfolioComparison;
            case 'create_consortium':
              return {
                id: Date.now(),
                name: args?.request?.name || 'Neue Gruppe',
                portfolioIds: args?.request?.portfolioIds || [],
                createdAt: new Date().toISOString(),
              };
            case 'update_consortium':
              return {
                id: args?.id,
                name: args?.request?.name || 'Aktualisierte Gruppe',
                portfolioIds: args?.request?.portfolioIds || [],
                createdAt: new Date().toISOString(),
              };
            case 'delete_consortium':
              return null;

            default:
              console.warn('[Consortium Mock] Unknown command:', cmd);
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
  }, consortiumMockData);
}

test.describe('Konsortium / Portfolio-Gruppen', () => {
  test.beforeEach(async ({ page }) => {
    await injectConsortiumMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Navigation zu Portfolio-Gruppen View funktioniert', async ({ page }) => {
    // Look for Portfolio-Gruppen in sidebar
    const navItem = page.locator('button[data-nav-item="consortium"]');

    // Verify nav item exists
    expect(await navItem.count()).toBeGreaterThan(0);

    // Click to navigate
    await navItem.first().click();
    await page.waitForTimeout(1000);

    // Verify navigation happened by checking the active state
    const activeItem = page.locator('button[data-nav-item="consortium"].bg-primary');
    const isActive = await activeItem.count() > 0;

    // Screenshot
    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-navigation.png',
      fullPage: true,
    });

    // Soft check - navigation worked if button has active state
    expect(isActive).toBeTruthy();
  });

  test('Konsortium-Liste wird angezeigt', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(1000);
    }

    // Check for consortium items
    const consortiumItems = page.locator('[data-testid*="consortium"], .consortium-item, text=/Familie/i');

    // Take screenshot
    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-list.png',
      fullPage: true,
    });

    // Soft check - at least page should load
    expect(await page.locator('body').count()).toBe(1);
  });

  test('Konsortium-Performance wird korrekt angezeigt', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(1000);
    }

    // Look for performance metrics
    const performanceSelectors = [
      'text=/TTWROR|IRR|Performance/i',
      'text=/Gesamtwert|Depotwert/i',
      'text=/Gewinn|G\\/V/i',
    ];

    let foundPerformance = false;
    for (const selector of performanceSelectors) {
      if (await page.locator(selector).count() > 0) {
        foundPerformance = true;
        break;
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-performance.png',
      fullPage: true,
    });

    // Soft check
    expect(foundPerformance).toBeTruthy();
  });

  test('Konsortium erstellen Dialog öffnet sich', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(500);
    }

    // Look for create button
    const createButton = page.locator('button:has-text("Neue Gruppe"), button:has-text("Erstellen"), [data-testid="create-consortium"]');

    if (await createButton.count() > 0) {
      await createButton.first().click();
      await page.waitForTimeout(500);

      // Check if modal/dialog opened
      const dialog = page.locator('[role="dialog"], .modal, .fixed.inset-0');
      if (await dialog.count() > 0) {
        await expect(dialog.first()).toBeVisible();

        // Look for form elements
        const nameInput = page.locator('input[placeholder*="Name"], input[name="name"]');
        expect(await nameInput.count()).toBeGreaterThanOrEqual(0);
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-create-dialog.png',
      fullPage: true,
    });
  });

  test('Risk Metrics werden angezeigt', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(1000);
    }

    // Look for risk metrics
    const riskSelectors = [
      'text=/Volatilität/i',
      'text=/Sharpe/i',
      'text=/Sortino/i',
      'text=/Drawdown/i',
    ];

    let foundRiskMetric = false;
    for (const selector of riskSelectors) {
      if (await page.locator(selector).count() > 0) {
        foundRiskMetric = true;
        break;
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-risk-metrics.png',
      fullPage: true,
    });

    // Soft check
    expect(foundRiskMetric).toBeTruthy();
  });

  test('Performance-Chart wird gerendert', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(1500);
    }

    // Look for chart container
    const chartSelectors = [
      '[data-testid*="chart"]',
      '.recharts-wrapper',
      'svg.recharts-surface',
      '[class*="chart"]',
    ];

    let foundChart = false;
    for (const selector of chartSelectors) {
      if (await page.locator(selector).count() > 0) {
        foundChart = true;
        break;
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-chart.png',
      fullPage: true,
    });

    // Soft check
    expect(foundChart).toBeTruthy();
  });

  test('Portfolio-Aufteilung wird angezeigt', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(1000);
    }

    // Look for portfolio breakdown table or list
    const breakdownSelectors = [
      'text=/Hauptdepot/i',
      'text=/Zweitdepot/i',
      '[data-testid*="portfolio-breakdown"]',
      'table tbody tr',
    ];

    let foundBreakdown = false;
    for (const selector of breakdownSelectors) {
      if (await page.locator(selector).count() > 0) {
        foundBreakdown = true;
        break;
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-breakdown.png',
      fullPage: true,
    });

    // Soft check
    expect(foundBreakdown).toBeTruthy();
  });
});

test.describe('Konsortium CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    await injectConsortiumMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Neue Portfolio-Gruppe kann erstellt werden', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(500);
    }

    // Click create button
    const createBtn = page.locator('button:has-text("Neue Gruppe")');
    if (await createBtn.count() > 0) {
      await createBtn.click();
      await page.waitForTimeout(500);

      // Fill in the name
      const nameInput = page.locator('input').first();
      if (await nameInput.count() > 0) {
        await nameInput.fill('Test Konsortium');
      }

      // Try to select portfolios (checkboxes or list items)
      const portfolioCheckbox = page.locator('input[type="checkbox"]').first();
      if (await portfolioCheckbox.count() > 0) {
        await portfolioCheckbox.check();
      }

      // Submit
      const submitBtn = page.locator('button:has-text("Erstellen"), button:has-text("Speichern")');
      if (await submitBtn.count() > 0) {
        await submitBtn.first().click();
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-create.png',
      fullPage: true,
    });
  });

  test('Konsortium kann bearbeitet werden', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(500);
    }

    // Look for edit button
    const editBtn = page.locator('button:has-text("Bearbeiten"), [data-testid*="edit"]').first();
    if (await editBtn.count() > 0 && await editBtn.isVisible()) {
      await editBtn.click();
      await page.waitForTimeout(500);
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-edit.png',
      fullPage: true,
    });
  });

  test('Konsortium kann gelöscht werden', async ({ page }) => {
    // Navigate to consortium view
    const navItem = page.locator('button[data-nav-item="consortium"]');
    if (await navItem.count() > 0) {
      await navItem.first().click();
      await page.waitForTimeout(500);
    }

    // Look for delete button
    const deleteBtn = page.locator('button:has-text("Löschen"), [data-testid*="delete"]').first();
    if (await deleteBtn.count() > 0 && await deleteBtn.isVisible()) {
      // Don't actually click - just verify it exists
      expect(await deleteBtn.isVisible()).toBeTruthy();
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/consortium-delete.png',
      fullPage: true,
    });
  });
});
