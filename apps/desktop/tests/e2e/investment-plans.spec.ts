import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

const investmentPlansMockData = {
  ...mockData,
  investmentPlans: [
    {
      id: 1,
      name: 'ETF Sparplan',
      securityId: 1,
      securityName: 'MSCI World ETF',
      portfolioId: 1,
      accountId: 1,
      amount: 50000, // 500 EUR
      interval: 'MONTHLY',
      startDate: '2024-01-01',
      autoGenerate: true,
      fees: 0,
      taxes: 0,
    },
    {
      id: 2,
      name: 'Aktien Sparplan',
      securityId: 2,
      securityName: 'Apple Inc.',
      portfolioId: 1,
      accountId: 1,
      amount: 25000, // 250 EUR
      interval: 'MONTHLY',
      startDate: '2024-02-01',
      autoGenerate: false,
      fees: 150, // 1.50 EUR
      taxes: 0,
    },
  ],
};

async function injectInvestmentPlansMocks(page: any) {
  await page.addInitScript((data: typeof investmentPlansMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          switch (cmd) {
            case 'get_pp_portfolios':
              return data.portfolios;
            case 'get_accounts':
              return [{ id: 1, uuid: 'acc-1', name: 'Girokonto', currency: 'EUR' }];
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
            case 'get_investment_plans':
              return data.investmentPlans;
            case 'create_investment_plan':
              return { id: Date.now(), ...args?.plan };
            case 'update_investment_plan':
              return { id: args?.id, ...args?.plan };
            case 'delete_investment_plan':
              return null;
            case 'execute_investment_plan':
              return { success: true };
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
  }, investmentPlansMockData);
}

test.describe('Investment Plans View (Sparpläne)', () => {
  test.beforeEach(async ({ page }) => {
    await injectInvestmentPlansMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Navigation zu Sparpläne View funktioniert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="plans"]');
    expect(await navItem.count()).toBeGreaterThan(0);

    await navItem.click();
    await page.waitForTimeout(500);

    await page.screenshot({
      path: 'playwright-report/screenshots/investment-plans-view.png',
      fullPage: true,
    });
  });

  test('Sparplan-Liste wird angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="plans"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const hasETFSparplan = await page.locator('text=ETF Sparplan').count() > 0;
    const hasAktienSparplan = await page.locator('text=Aktien Sparplan').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/investment-plans-list.png',
      fullPage: true,
    });

    expect(hasETFSparplan || hasAktienSparplan).toBeTruthy();
  });

  test('Neuer Sparplan Button existiert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="plans"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const createBtn = page.locator('button:has-text("Neuer Sparplan"), button:has-text("Sparplan erstellen"), button:has-text("Neu")');
    const hasCreateBtn = await createBtn.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/investment-plans-create-btn.png',
    });

    expect(hasCreateBtn).toBeTruthy();
  });

  test('Intervall wird angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="plans"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const hasInterval = await page.locator('text=/Monatlich|Vierteljährlich|Monthly|Quarterly/i').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/investment-plans-interval.png',
      fullPage: true,
    });

    expect(hasInterval).toBeTruthy();
  });

  test('Ausführen Button existiert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="plans"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const executeBtn = page.locator('button:has-text("Ausführen"), button:has-text("Jetzt ausführen")');
    const hasExecuteBtn = await executeBtn.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/investment-plans-execute.png',
    });

    expect(hasExecuteBtn).toBeTruthy();
  });
});
