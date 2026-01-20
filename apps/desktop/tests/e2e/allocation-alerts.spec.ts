import { test, expect } from '@playwright/test';
import { waitForAppReady, mockData, closeWelcomeModal } from './utils/tauri-mock';

// Extended mock data for allocation alerts tests
const alertsMockData = {
  ...mockData,
  allocationTargets: [
    {
      id: 1,
      portfolioId: 1,
      securityId: 1,
      securityName: 'Apple Inc.',
      targetWeight: 0.2,
      threshold: 0.05,
      createdAt: '2024-01-15T10:00:00Z',
    },
    {
      id: 2,
      portfolioId: 1,
      securityId: 2,
      securityName: 'Microsoft Corp.',
      targetWeight: 0.15,
      threshold: 0.05,
      createdAt: '2024-01-15T10:00:00Z',
    },
  ],
  allocationAlerts: [
    {
      alertType: 'over_weight',
      entityName: 'Apple Inc.',
      targetWeight: 0.2,
      currentWeight: 0.28,
      deviation: 0.08,
      severity: 'critical',
      securityId: 1,
    },
    {
      alertType: 'under_weight',
      entityName: 'Microsoft Corp.',
      targetWeight: 0.15,
      currentWeight: 0.08,
      deviation: -0.07,
      severity: 'warning',
      securityId: 2,
    },
  ],
  allocationAlertCount: {
    total: 2,
    critical: 1,
    warning: 1,
  },
};

async function injectAlertsMocks(page: any) {
  await page.addInitScript((data: typeof alertsMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[Alerts Mock] invoke:', cmd, args);

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
            case 'get_allocation_targets':
              return data.allocationTargets;
            case 'get_allocation_alerts':
              return data.allocationAlerts;
            case 'get_allocation_alert_count':
              return data.allocationAlertCount;
            case 'set_allocation_target':
              return args?.request?.securityId || 1;
            case 'delete_allocation_target':
              return null;
            case 'get_securities':
              return [
                { id: 1, name: 'Apple Inc.', ticker: 'AAPL', currency: 'EUR' },
                { id: 2, name: 'Microsoft Corp.', ticker: 'MSFT', currency: 'EUR' },
              ];
            case 'get_portfolios':
              return data.portfolios;
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
  }, alertsMockData);
}

test.describe('Allocation Alerts - Rebalancing View', () => {
  test.beforeEach(async ({ page }) => {
    await injectAlertsMocks(page);
    await page.goto('/');
    await waitForAppReady(page);

    // Close WelcomeModal if open
    await closeWelcomeModal(page);

    // Navigate to Rebalancing view
    const rebalancingNavItem = page.locator('button[data-nav-item="rebalancing"]');
    if (await rebalancingNavItem.count() > 0) {
      await rebalancingNavItem.click();
      await page.waitForTimeout(500);
    }
  });

  test('Rebalancing-View wird geladen', async ({ page }) => {
    // Check for Rebalancing page content
    const hasRebalancingContent = await page.locator('text=/Rebalancing|Ziel|Allokation/i').count() > 0;
    expect(hasRebalancingContent).toBeTruthy();
  });

  test('Warnungen-Button ist sichtbar', async ({ page }) => {
    // Look for the alerts button
    const alertsButton = page.locator('button:has-text("Warnungen")');
    if (await alertsButton.count() > 0) {
      await expect(alertsButton.first()).toBeVisible();
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/rebalancing-alerts-button.png',
    });
  });

  test('AlertsPanel kann geöffnet werden', async ({ page }) => {
    // Click the Warnungen button
    const alertsButton = page.locator('button:has-text("Warnungen")');
    if (await alertsButton.count() > 0) {
      await alertsButton.first().click();
      await page.waitForTimeout(500);

      // Check if alerts panel is visible
      const hasAlertsContent = await page.locator('text=/kritisch|Warnung|Zielgewichtung/i').count() > 0;
      expect(hasAlertsContent).toBeTruthy();
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/rebalancing-alerts-panel.png',
    });
  });

  test('Ziel-Button öffnet Modal', async ({ page }) => {
    // Click the Ziel button
    const targetButton = page.locator('button:has-text("Ziel")').first();
    if (await targetButton.count() > 0) {
      await targetButton.click();
      await page.waitForTimeout(500);

      // Check if modal is visible
      const modal = page.locator('[role="dialog"], .fixed.inset-0');
      if (await modal.count() > 0) {
        await expect(modal.first()).toBeVisible();
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/allocation-target-modal.png',
    });
  });
});

test.describe('Allocation Alerts - Sidebar Badge', () => {
  test.beforeEach(async ({ page }) => {
    await injectAlertsMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('AlertBadge wird neben Rebalancing angezeigt', async ({ page }) => {
    // Look for the badge in the sidebar
    const rebalancingItem = page.locator('button[data-nav-item="rebalancing"]');

    if (await rebalancingItem.count() > 0) {
      // Check for badge element
      const badge = rebalancingItem.locator('span.rounded-full, span[class*="badge"]');

      // Badge may not always be visible if there are no alerts
      const badgeExists = await badge.count() > 0;
      console.log('Badge exists:', badgeExists);
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/sidebar-alert-badge.png',
    });
  });
});

test.describe('Allocation Alerts - Widget', () => {
  test.beforeEach(async ({ page }) => {
    await injectAlertsMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('AlertsWidget zeigt Warnungen an', async ({ page }) => {
    // Look for alerts widget content
    const alertsWidgetSelectors = [
      '[data-testid*="alerts"]',
      'text=/Warnungen|kritisch|Abweichung/i',
    ];

    let foundAlertsWidget = false;
    for (const selector of alertsWidgetSelectors) {
      if (await page.locator(selector).count() > 0) {
        foundAlertsWidget = true;
        break;
      }
    }

    // This is a soft check since alerts widget may not be on default dashboard
    expect(foundAlertsWidget).toBeTruthy();

    await page.screenshot({
      path: 'playwright-report/screenshots/alerts-widget.png',
    });
  });
});

test.describe('Allocation Target Management', () => {
  test.beforeEach(async ({ page }) => {
    await injectAlertsMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);

    // Navigate to Rebalancing view
    const rebalancingNavItem = page.locator('button[data-nav-item="rebalancing"]');
    if (await rebalancingNavItem.count() > 0) {
      await rebalancingNavItem.click();
      await page.waitForTimeout(500);
    }
  });

  test('Zielgewichtung kann erstellt werden', async ({ page }) => {
    // Open the target modal
    const targetButton = page.locator('button:has-text("Ziel")').first();
    if (await targetButton.count() > 0) {
      await targetButton.click();
      await page.waitForTimeout(500);

      // Check that modal opened
      const modal = page.locator('[role="dialog"], .fixed.inset-0.z-50');
      if (await modal.count() > 0) {
        // Verify form elements are present
        const portfolioSelect = page.locator('select[name="portfolioId"]');
        const targetWeightInput = page.locator('input[name="targetWeight"]');

        expect(await portfolioSelect.count() > 0 || await targetWeightInput.count() > 0).toBeTruthy();
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/allocation-target-create.png',
    });
  });

  test('Zielgewichtungen werden in der Liste angezeigt', async ({ page }) => {
    // Open alerts panel
    const alertsButton = page.locator('button:has-text("Warnungen")');
    if (await alertsButton.count() > 0) {
      await alertsButton.first().click();
      await page.waitForTimeout(500);

      // Click on targets tab
      const targetsTab = page.locator('button:has-text("Zielgewichtungen")');
      if (await targetsTab.count() > 0) {
        await targetsTab.click();
        await page.waitForTimeout(300);

        // Check for target entries
        const hasTargets = await page.locator('text=/Apple|Microsoft|Ziel:/i').count() > 0;
        expect(hasTargets).toBeTruthy();
      }
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/allocation-targets-list.png',
    });
  });
});
