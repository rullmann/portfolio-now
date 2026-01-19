import { test, expect, Page } from '@playwright/test';
import { mockData, closeWelcomeModal } from './utils/tauri-mock';

// Extended mock data for custom attributes tests
const attributesMockData = {
  ...mockData,
  attributeTypes: [
    {
      id: 1,
      uuid: 'attr-uuid-1',
      name: 'Sektor',
      columnLabel: 'Sector',
      target: 'security',
      dataType: 'STRING',
      converterClass: null,
      source: null,
      createdAt: '2024-01-15T10:00:00Z',
      updatedAt: null,
    },
    {
      id: 2,
      uuid: 'attr-uuid-2',
      name: 'ESG Score',
      columnLabel: 'ESG',
      target: 'security',
      dataType: 'DOUBLE_NUMBER',
      converterClass: null,
      source: null,
      createdAt: '2024-01-15T10:00:00Z',
      updatedAt: null,
    },
    {
      id: 3,
      uuid: 'attr-uuid-3',
      name: 'Dividendenzahler',
      columnLabel: null,
      target: 'security',
      dataType: 'BOOLEAN',
      converterClass: null,
      source: null,
      createdAt: '2024-01-15T10:00:00Z',
      updatedAt: null,
    },
  ],
  securityAttributes: [
    {
      attributeTypeId: 1,
      attributeTypeName: 'Sektor',
      attributeTypeUuid: 'attr-uuid-1',
      dataType: 'STRING',
      value: 'Technology',
    },
    {
      attributeTypeId: 2,
      attributeTypeName: 'ESG Score',
      attributeTypeUuid: 'attr-uuid-2',
      dataType: 'DOUBLE_NUMBER',
      value: '85.5',
    },
    {
      attributeTypeId: 3,
      attributeTypeName: 'Dividendenzahler',
      attributeTypeUuid: 'attr-uuid-3',
      dataType: 'BOOLEAN',
      value: 'true',
    },
  ],
};

async function injectAttributesMocks(page: Page) {
  await page.addInitScript((data: typeof attributesMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[Attributes Mock] invoke:', cmd, args);

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
            case 'get_attribute_types':
              if (args?.target) {
                return data.attributeTypes.filter((t: any) => t.target === args.target);
              }
              return data.attributeTypes;
            case 'get_security_attributes':
              return data.securityAttributes;
            case 'create_attribute_type':
              return {
                ...args?.request,
                id: Date.now(),
                uuid: `attr-uuid-${Date.now()}`,
                createdAt: new Date().toISOString(),
              };
            case 'update_attribute_type':
              return data.attributeTypes.find((t: any) => t.id === args?.id);
            case 'delete_attribute_type':
              return null;
            case 'set_security_attribute':
              return null;
            case 'remove_security_attribute':
              return null;
            default:
              console.log('[Attributes Mock] Unhandled command:', cmd);
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
  }, attributesMockData);
}

async function waitForAppReady(page: Page) {
  await page.waitForSelector('#root > div', { state: 'visible', timeout: 10000 });
  await page.waitForTimeout(1000);
}

test.describe('Custom Attributes', () => {
  test.beforeEach(async ({ page }) => {
    await injectAttributesMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Settings shows attribute type manager section', async ({ page }) => {
    // Navigate to Settings using sidebar
    const settingsNav = page.locator('button, a').filter({ hasText: /Einstellungen/i }).first();
    await settingsNav.click();
    await page.waitForTimeout(500);

    // Should show the "Erweiterte Daten" section
    await expect(page.locator('h2:has-text("Erweiterte Daten")')).toBeVisible();

    // Should show "Benutzerdefinierte Attribute" expandable section
    await expect(page.locator('text=Benutzerdefinierte Attribute')).toBeVisible();
  });

  test('Attribute type manager expands and shows attribute types', async ({ page }) => {
    const settingsNav = page.locator('button, a').filter({ hasText: /Einstellungen/i }).first();
    await settingsNav.click();
    await page.waitForTimeout(500);

    // Expand the attribute type manager
    await page.click('text=Benutzerdefinierte Attribute');
    await page.waitForTimeout(500);

    // Should show the description
    await expect(page.locator('text=Erstelle eigene Attribute')).toBeVisible();

    // Should show attribute types
    await expect(page.locator('text=Sektor')).toBeVisible();
    await expect(page.locator('text=ESG Score')).toBeVisible();
  });

  test('Create new attribute type button shows form', async ({ page }) => {
    const settingsNav = page.locator('button, a').filter({ hasText: /Einstellungen/i }).first();
    await settingsNav.click();
    await page.waitForTimeout(500);

    await page.click('text=Benutzerdefinierte Attribute');
    await page.waitForTimeout(500);

    // Click create button
    await page.click('text=Neues Attribut erstellen');

    // Should show create form
    await expect(page.locator('h4:has-text("Neues Attribut erstellen")')).toBeVisible();
  });

  test('Attribute types show correct data type labels', async ({ page }) => {
    const settingsNav = page.locator('button, a').filter({ hasText: /Einstellungen/i }).first();
    await settingsNav.click();
    await page.waitForTimeout(500);

    await page.click('text=Benutzerdefinierte Attribute');
    await page.waitForTimeout(500);

    // Check data type labels are shown
    await expect(page.locator('text=Text').first()).toBeVisible();
    await expect(page.locator('text=Dezimalzahl').first()).toBeVisible();
    await expect(page.locator('text=Ja/Nein').first()).toBeVisible();
  });

  test('Attribute count is shown after expanding', async ({ page }) => {
    const settingsNav = page.locator('button, a').filter({ hasText: /Einstellungen/i }).first();
    await settingsNav.click();
    await page.waitForTimeout(500);

    // First expand to load the data
    await page.click('text=Benutzerdefinierte Attribute');
    await page.waitForTimeout(500);

    // After loading, the count (3) should be visible
    // Note: Count is shown once data is loaded
    await expect(page.locator('text=(3)')).toBeVisible();
  });
});
