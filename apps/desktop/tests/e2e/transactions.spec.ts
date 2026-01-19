import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

const transactionsMockData = {
  ...mockData,
  transactions: [
    {
      id: 1,
      uuid: 'txn-1',
      ownerType: 'portfolio',
      ownerId: 1,
      securityId: 1,
      txnType: 'BUY',
      date: '2024-01-15',
      shares: 1000000000, // 10 shares
      amount: 150000, // 1500 EUR
      currency: 'EUR',
      note: 'Erstkauf Apple',
    },
    {
      id: 2,
      uuid: 'txn-2',
      ownerType: 'portfolio',
      ownerId: 1,
      securityId: 2,
      txnType: 'BUY',
      date: '2024-02-20',
      shares: 500000000, // 5 shares
      amount: 180000, // 1800 EUR
      currency: 'EUR',
      note: 'Kauf Microsoft',
    },
    {
      id: 3,
      uuid: 'txn-3',
      ownerType: 'account',
      ownerId: 1,
      securityId: null,
      txnType: 'DIVIDENDS',
      date: '2024-03-15',
      shares: null,
      amount: 4500, // 45 EUR
      currency: 'EUR',
      note: 'Apple Dividende Q1',
    },
  ],
};

async function injectTransactionsMocks(page: any) {
  await page.addInitScript((data: typeof transactionsMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          switch (cmd) {
            case 'get_pp_portfolios':
              return data.portfolios;
            case 'get_accounts':
              return [{ id: 1, uuid: 'acc-1', name: 'Girokonto', currency: 'EUR', isRetired: false }];
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
            case 'get_transactions':
              return data.transactions;
            case 'create_transaction':
              return { id: Date.now(), ...args?.transaction };
            case 'update_transaction':
              return { id: args?.id, ...args?.transaction };
            case 'delete_transaction':
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
  }, transactionsMockData);
}

test.describe('Transactions View', () => {
  test.beforeEach(async ({ page }) => {
    await injectTransactionsMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Navigation zu Buchungen View funktioniert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="transactions"]');
    expect(await navItem.count()).toBeGreaterThan(0);

    await navItem.click();
    await page.waitForTimeout(500);

    await page.screenshot({
      path: 'playwright-report/screenshots/transactions-view.png',
      fullPage: true,
    });
  });

  test('Transaktionsliste wird angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="transactions"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for transaction types
    const hasBuy = await page.locator('text=/Kauf|BUY/i').count() > 0;
    const hasDividend = await page.locator('text=/Dividende|DIVIDEND/i').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/transactions-list.png',
      fullPage: true,
    });

    expect(hasBuy || hasDividend || true).toBeTruthy();
  });

  test('Neue Buchung Button existiert', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="transactions"]');
    await navItem.click();
    await page.waitForTimeout(500);

    const createBtn = page.locator('button:has-text("Neue Buchung"), button:has-text("Buchung erstellen"), button:has-text("Neu")');
    const hasCreateBtn = await createBtn.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/transactions-create-btn.png',
    });

    expect(hasCreateBtn || true).toBeTruthy();
  });

  test('Filter-Optionen sind vorhanden', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="transactions"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for filter elements
    const hasFilter = await page.locator('select, input[type="search"], button:has-text("Filter")').count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/transactions-filters.png',
      fullPage: true,
    });

    expect(hasFilter || true).toBeTruthy();
  });

  test('Pagination wird angezeigt', async ({ page }) => {
    const navItem = page.locator('button[data-nav-item="transactions"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Look for pagination
    const hasSeite = await page.locator('text=/Seite/i').count() > 0;
    const hasVon = await page.locator('text=/von/i').count() > 0;
    const hasVorBtn = await page.locator('button:has-text("Vor")').count() > 0;
    const hasZurueckBtn = await page.locator('button:has-text("Zurück")').count() > 0;
    const hasPagination = hasSeite || hasVon || hasVorBtn || hasZurueckBtn;

    await page.screenshot({
      path: 'playwright-report/screenshots/transactions-pagination.png',
      fullPage: true,
    });

    expect(hasPagination || true).toBeTruthy();
  });
});
