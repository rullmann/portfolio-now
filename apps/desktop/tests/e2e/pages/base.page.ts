import { Page, Locator } from '@playwright/test';

/**
 * Base Page Object mit gemeinsamer Funktionalität
 */
export class BasePage {
  readonly page: Page;
  readonly sidebar: Locator;
  readonly header: Locator;
  readonly mainContent: Locator;
  readonly loadingIndicator: Locator;

  constructor(page: Page) {
    this.page = page;
    this.sidebar = page.locator('nav, [data-testid="sidebar"]');
    this.header = page.locator('header, [data-testid="header"]');
    this.mainContent = page.locator('main, [data-testid="main-content"]');
    this.loadingIndicator = page.locator('[data-testid="loading"], .loading-indicator, .animate-spin');
  }

  /**
   * Warte bis die Seite vollständig geladen ist
   */
  async waitForPageLoad(): Promise<void> {
    await this.page.waitForLoadState('domcontentloaded');

    // Wait for loading indicators to disappear
    const loadingCount = await this.loadingIndicator.count();
    if (loadingCount > 0) {
      await this.loadingIndicator.first().waitFor({ state: 'hidden', timeout: 10000 }).catch(() => {});
    }
  }

  /**
   * Navigiere zu einem View via Sidebar
   */
  async navigateTo(viewName: string): Promise<void> {
    // Try different selectors for navigation items
    const selectors = [
      `[data-testid="nav-${viewName}"]`,
      `[data-view="${viewName}"]`,
      `button:has-text("${viewName}")`,
      `a:has-text("${viewName}")`,
    ];

    for (const selector of selectors) {
      const element = this.sidebar.locator(selector);
      if (await element.count() > 0) {
        await element.first().click();
        await this.waitForPageLoad();
        return;
      }
    }

    throw new Error(`Navigation item not found: ${viewName}`);
  }

  /**
   * Öffne Modal/Dialog
   */
  async openModal(buttonSelector: string): Promise<Locator> {
    await this.page.locator(buttonSelector).click();
    const modal = this.page.locator('[role="dialog"], [data-testid="modal"], .modal');
    await modal.waitFor({ state: 'visible' });
    return modal;
  }

  /**
   * Schließe Modal/Dialog
   */
  async closeModal(): Promise<void> {
    const closeButton = this.page.locator('[data-testid="modal-close"], [aria-label="Close"], .modal button:has-text("×")');
    if (await closeButton.count() > 0) {
      await closeButton.first().click();
    } else {
      // Try ESC key
      await this.page.keyboard.press('Escape');
    }

    // Wait for modal to close
    await this.page.locator('[role="dialog"], [data-testid="modal"], .modal').waitFor({ state: 'hidden', timeout: 5000 }).catch(() => {});
  }

  /**
   * Prüfe ob Toast-Nachricht angezeigt wird
   */
  async expectToast(message: string): Promise<void> {
    const toast = this.page.locator('[data-testid="toast"], .toast, [role="alert"]').filter({ hasText: message });
    await toast.waitFor({ state: 'visible', timeout: 5000 });
  }

  /**
   * Hole den aktuellen View-Titel
   */
  async getViewTitle(): Promise<string> {
    const title = this.header.locator('h1, [data-testid="view-title"]');
    return await title.textContent() || '';
  }

  /**
   * Screenshot für Debugging
   */
  async screenshot(name: string): Promise<void> {
    await this.page.screenshot({ path: `playwright-report/screenshots/${name}.png` });
  }
}
