import { Page, Locator, expect } from '@playwright/test';
import { BasePage } from './base.page';

/**
 * Dashboard Page Object
 */
export class DashboardPage extends BasePage {
  // Metric Cards
  readonly portfolioValue: Locator;
  readonly todayChange: Locator;
  readonly ttwrorCard: Locator;
  readonly irrCard: Locator;
  readonly costBasisCard: Locator;

  // Main Content
  readonly portfolioChart: Locator;
  readonly holdingsList: Locator;
  readonly holdingItems: Locator;

  // Actions
  readonly refreshButton: Locator;
  readonly importButton: Locator;
  readonly aiInsightsButton: Locator;
  readonly addWidgetButton: Locator;

  // Time Range
  readonly timeRangeButtons: Locator;

  constructor(page: Page) {
    super(page);

    // Metric Cards
    this.portfolioValue = page.locator('[data-testid="portfolio-value"], .metric-card:has-text("Depotwert")');
    this.todayChange = page.locator('[data-testid="today-change"], .metric-card:has-text("Heute")');
    this.ttwrorCard = page.locator('[data-testid="ttwror"], .metric-card:has-text("TTWROR")');
    this.irrCard = page.locator('[data-testid="irr"], .metric-card:has-text("IRR")');
    this.costBasisCard = page.locator('[data-testid="cost-basis"], .metric-card:has-text("Einstand")');

    // Main Content
    this.portfolioChart = page.locator('[data-testid="portfolio-chart"], .recharts-responsive-container');
    this.holdingsList = page.locator('[data-testid="holdings-list"], .holdings-sidebar');
    this.holdingItems = page.locator('[data-testid="holding-item"], .holding-row');

    // Actions
    this.refreshButton = page.locator('[data-testid="refresh-btn"], button:has-text("Aktualisieren")');
    this.importButton = page.locator('[data-testid="import-btn"], button:has-text("Importieren")');
    this.aiInsightsButton = page.locator('[data-testid="ai-insights-btn"], button:has-text("KI")');
    this.addWidgetButton = page.locator('[data-testid="add-widget-btn"], button:has-text("Widget")');

    // Time Range
    this.timeRangeButtons = page.locator('[data-testid="time-range"] button, .time-range-selector button');
  }

  /**
   * Gehe zum Dashboard
   */
  async goto(): Promise<void> {
    await this.page.goto('/');
    await this.waitForPageLoad();
  }

  /**
   * Hole den Depotwert als Zahl
   */
  async getPortfolioValueNumber(): Promise<number> {
    const text = await this.portfolioValue.textContent();
    if (!text) return 0;

    // Parse German number format: "3.905,00 €" -> 3905.00
    const match = text.match(/[\d.,]+/);
    if (!match) return 0;

    return parseFloat(match[0].replace(/\./g, '').replace(',', '.'));
  }

  /**
   * Hole TTWROR als Prozent
   */
  async getTtwrorPercent(): Promise<number> {
    const text = await this.ttwrorCard.textContent();
    if (!text) return 0;

    const match = text.match(/([-+]?[\d.,]+)\s*%/);
    if (!match) return 0;

    return parseFloat(match[1].replace(',', '.'));
  }

  /**
   * Wähle Zeitraum
   */
  async selectTimeRange(range: '1W' | '1M' | '3M' | '6M' | 'YTD' | '1Y' | '3Y' | '5Y' | 'MAX'): Promise<void> {
    const button = this.timeRangeButtons.filter({ hasText: range });
    await button.click();
    await this.page.waitForTimeout(500); // Wait for data to reload
  }

  /**
   * Zähle Holdings
   */
  async getHoldingsCount(): Promise<number> {
    return await this.holdingItems.count();
  }

  /**
   * Klicke auf ein Holding
   */
  async clickHolding(name: string): Promise<void> {
    const holding = this.holdingItems.filter({ hasText: name });
    await holding.click();
  }

  /**
   * Öffne AI Insights Modal
   */
  async openAiInsights(): Promise<void> {
    await this.aiInsightsButton.click();
    await this.page.locator('[data-testid="ai-insights-modal"], .modal:has-text("Insights")').waitFor({ state: 'visible' });
  }

  /**
   * Prüfe ob Chart sichtbar ist
   */
  async isChartVisible(): Promise<boolean> {
    return await this.portfolioChart.isVisible();
  }

  /**
   * Prüfe Dashboard Metriken
   */
  async verifyMetricsDisplayed(): Promise<void> {
    await expect(this.portfolioValue).toBeVisible();
    await expect(this.ttwrorCard).toBeVisible();
    await expect(this.portfolioChart).toBeVisible();
  }

  /**
   * Hole alle Holding-Namen
   */
  async getHoldingNames(): Promise<string[]> {
    const names: string[] = [];
    const count = await this.holdingItems.count();

    for (let i = 0; i < count; i++) {
      const text = await this.holdingItems.nth(i).locator('.holding-name, [data-testid="holding-name"]').textContent();
      if (text) names.push(text.trim());
    }

    return names;
  }
}
