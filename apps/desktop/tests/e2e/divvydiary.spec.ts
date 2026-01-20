import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

const divvyDiaryMockData = {
  ...mockData,
  divvyDiaryPortfolios: [
    { id: 'dd-portfolio-1', name: 'Mein DivvyDiary Portfolio' },
    { id: 'dd-portfolio-2', name: 'Zweites Portfolio' },
  ],
  uploadResult: {
    success: true,
    message: 'Export erfolgreich: 3 Wertpapiere, 10 Transaktionen',
    securitiesCount: 3,
    activitiesCount: 10,
  },
};

async function injectDivvyDiaryMocks(page: any) {
  await page.addInitScript((data: typeof divvyDiaryMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[DivvyDiary Mock] invoke:', cmd, args);

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
            // DivvyDiary Commands
            case 'get_divvydiary_portfolios':
              if (!args?.apiKey) {
                throw new Error('DivvyDiary API-Key fehlt');
              }
              return data.divvyDiaryPortfolios;
            case 'upload_to_divvydiary':
              if (!args?.apiKey) {
                throw new Error('DivvyDiary API-Key fehlt');
              }
              return data.uploadResult;
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
  }, divvyDiaryMockData);
}

test.describe('DivvyDiary Export', () => {
  test.beforeEach(async ({ page }) => {
    await injectDivvyDiaryMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Exportieren Dropdown enthält DivvyDiary Option', async ({ page }) => {
    // Click on Export dropdown
    const exportButton = page.locator('button:has-text("Exportieren")');
    await expect(exportButton).toBeVisible();
    await exportButton.click();
    await page.waitForTimeout(300);

    // Check DivvyDiary option exists
    const divvyDiaryOption = page.locator('text=DivvyDiary');
    const hasDivvyDiary = await divvyDiaryOption.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/divvydiary-export-dropdown.png',
      fullPage: true,
    });

    expect(hasDivvyDiary).toBeTruthy();
  });

  test('DivvyDiary Export Modal öffnet sich', async ({ page }) => {
    // Click on Export dropdown
    const exportButton = page.locator('button:has-text("Exportieren")');
    await exportButton.click();
    await page.waitForTimeout(300);

    // Click DivvyDiary option
    const divvyDiaryOption = page.locator('text=DivvyDiary');
    if (await divvyDiaryOption.count() > 0) {
      await divvyDiaryOption.click();
      await page.waitForTimeout(500);

      // Check modal is open
      const modalTitle = page.locator('text=DivvyDiary Export');
      const hasModal = await modalTitle.count() > 0;

      await page.screenshot({
        path: 'playwright-report/screenshots/divvydiary-export-modal.png',
        fullPage: true,
      });

      expect(hasModal).toBeTruthy();
    }
  });

  test('DivvyDiary Modal zeigt API-Key Eingabefeld', async ({ page }) => {
    // Open modal
    const exportButton = page.locator('button:has-text("Exportieren")');
    await exportButton.click();
    await page.waitForTimeout(300);

    const divvyDiaryOption = page.locator('text=DivvyDiary');
    if (await divvyDiaryOption.count() > 0) {
      await divvyDiaryOption.click();
      await page.waitForTimeout(500);

      // Check for API key input
      const apiKeyInput = page.locator('input[placeholder*="API-Key"]');
      const hasApiKeyInput = await apiKeyInput.count() > 0;

      expect(hasApiKeyInput).toBeTruthy();
    }
  });

  test('DivvyDiary Modal zeigt Portfolio-Auswahl', async ({ page }) => {
    // Open modal
    const exportButton = page.locator('button:has-text("Exportieren")');
    await exportButton.click();
    await page.waitForTimeout(300);

    const divvyDiaryOption = page.locator('text=DivvyDiary');
    if (await divvyDiaryOption.count() > 0) {
      await divvyDiaryOption.click();
      await page.waitForTimeout(500);

      // Check for portfolio selection
      const portfolioLabel = page.locator('text=/Lokale Portfolios|Portfolio/i');
      const hasPortfolioSelection = await portfolioLabel.count() > 0;

      expect(hasPortfolioSelection).toBeTruthy();
    }
  });

  test('DivvyDiary Modal zeigt Transaktionen-Toggle', async ({ page }) => {
    // Open modal
    const exportButton = page.locator('button:has-text("Exportieren")');
    await exportButton.click();
    await page.waitForTimeout(300);

    const divvyDiaryOption = page.locator('text=DivvyDiary');
    if (await divvyDiaryOption.count() > 0) {
      await divvyDiaryOption.click();
      await page.waitForTimeout(500);

      // Check for transactions toggle
      const transactionsToggle = page.locator('text=/Transaktionen übertragen/i');
      const hasTransactionsToggle = await transactionsToggle.count() > 0;

      expect(hasTransactionsToggle).toBeTruthy();
    }
  });

  test('DivvyDiary Modal zeigt Warnung', async ({ page }) => {
    // Open modal
    const exportButton = page.locator('button:has-text("Exportieren")');
    await exportButton.click();
    await page.waitForTimeout(300);

    const divvyDiaryOption = page.locator('text=DivvyDiary');
    if (await divvyDiaryOption.count() > 0) {
      await divvyDiaryOption.click();
      await page.waitForTimeout(500);

      // Check for warning about overwriting
      const warningText = page.locator('text=/überschrieben|Hinweis/i');
      const hasWarning = await warningText.count() > 0;

      expect(hasWarning).toBeTruthy();
    }
  });

  test('DivvyDiary Modal kann geschlossen werden', async ({ page }) => {
    // Open modal
    const exportButton = page.locator('button:has-text("Exportieren")');
    await exportButton.click();
    await page.waitForTimeout(300);

    const divvyDiaryOption = page.locator('text=DivvyDiary');
    if (await divvyDiaryOption.count() > 0) {
      await divvyDiaryOption.click();
      await page.waitForTimeout(500);

      // Close modal
      const closeButton = page.locator('button:has-text("Abbrechen")');
      if (await closeButton.count() > 0) {
        await closeButton.click();
        await page.waitForTimeout(300);

        // Verify modal is closed
        const modalTitle = page.locator('text=DivvyDiary Export');
        const modalClosed = await modalTitle.count() === 0;

        expect(modalClosed).toBeTruthy();
      }
    }
  });
});

test.describe('Settings - DivvyDiary', () => {
  test.beforeEach(async ({ page }) => {
    await injectDivvyDiaryMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Settings zeigt DivvyDiary API-Key Feld', async ({ page }) => {
    // Navigate to settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Check for DivvyDiary section
    const divvyDiaryLabel = page.locator('text=/DivvyDiary/i');
    const hasDivvyDiarySettings = await divvyDiaryLabel.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/settings-divvydiary.png',
      fullPage: true,
    });

    expect(hasDivvyDiarySettings).toBeTruthy();
  });

  test('Settings zeigt Externe Dienste Sektion', async ({ page }) => {
    // Navigate to settings
    const navItem = page.locator('button[data-nav-item="settings"]');
    await navItem.click();
    await page.waitForTimeout(500);

    // Check for External Services section
    const externalServicesHeader = page.locator('text=/Externe Dienste/i');
    const hasExternalServices = await externalServicesHeader.count() > 0;

    expect(hasExternalServices).toBeTruthy();
  });
});
