import { Page, expect } from '@playwright/test';

/**
 * Format currency value for comparison
 */
export function formatCurrency(value: number, currency = 'EUR'): string {
  return new Intl.NumberFormat('de-DE', {
    style: 'currency',
    currency,
  }).format(value);
}

/**
 * Format percentage for comparison
 */
export function formatPercent(value: number): string {
  return new Intl.NumberFormat('de-DE', {
    style: 'percent',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value / 100);
}

/**
 * Get text content and parse as number
 */
export async function getNumericValue(page: Page, selector: string): Promise<number> {
  const text = await page.locator(selector).textContent();
  if (!text) return 0;

  // Remove currency symbols, thousand separators, and parse
  const cleaned = text
    .replace(/[€$£¥]/g, '')
    .replace(/\s/g, '')
    .replace(/\./g, '')
    .replace(',', '.');

  return parseFloat(cleaned) || 0;
}

/**
 * Wait for element and verify text contains expected value
 */
export async function expectTextContains(
  page: Page,
  selector: string,
  expected: string
): Promise<void> {
  const element = page.locator(selector);
  await expect(element).toBeVisible();
  await expect(element).toContainText(expected);
}

/**
 * Take a screenshot with consistent naming
 */
export async function takeScreenshot(
  page: Page,
  name: string,
  options?: { fullPage?: boolean }
): Promise<void> {
  await page.screenshot({
    path: `playwright-report/screenshots/${name}.png`,
    fullPage: options?.fullPage ?? false,
  });
}

/**
 * Wait for network idle (useful after data fetching)
 */
export async function waitForNetworkIdle(page: Page, timeout = 5000): Promise<void> {
  await page.waitForLoadState('networkidle', { timeout });
}

/**
 * Scroll element into view
 */
export async function scrollIntoView(page: Page, selector: string): Promise<void> {
  await page.locator(selector).scrollIntoViewIfNeeded();
}

/**
 * Check if element exists without failing
 */
export async function elementExists(page: Page, selector: string): Promise<boolean> {
  return (await page.locator(selector).count()) > 0;
}

/**
 * Wait for specific number of elements
 */
export async function waitForElementCount(
  page: Page,
  selector: string,
  count: number,
  timeout = 10000
): Promise<void> {
  await expect(page.locator(selector)).toHaveCount(count, { timeout });
}

/**
 * Simulate drag and drop
 */
export async function dragAndDrop(
  page: Page,
  sourceSelector: string,
  targetSelector: string
): Promise<void> {
  const source = page.locator(sourceSelector);
  const target = page.locator(targetSelector);

  await source.dragTo(target);
}

/**
 * Fill form field with validation
 */
export async function fillFormField(
  page: Page,
  selector: string,
  value: string
): Promise<void> {
  const field = page.locator(selector);
  await field.click();
  await field.clear();
  await field.fill(value);
}

/**
 * Select option from dropdown
 */
export async function selectOption(
  page: Page,
  selector: string,
  value: string
): Promise<void> {
  await page.locator(selector).selectOption(value);
}

/**
 * Click button and wait for response
 */
export async function clickAndWait(
  page: Page,
  selector: string,
  waitFor?: string
): Promise<void> {
  await page.locator(selector).click();

  if (waitFor) {
    await page.waitForSelector(waitFor, { state: 'visible' });
  } else {
    await page.waitForTimeout(500);
  }
}
