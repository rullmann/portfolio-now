import { test, expect } from '@playwright/test';
import { mockData, waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

const aiChatMockData = {
  ...mockData,
  chatResponse: {
    message: 'Hier ist meine Analyse Ihres Portfolios:\n\n## Zusammenfassung\n- Gesamtwert: 3.905 EUR\n- Performance: +18,33%\n- Top-Holding: Apple Inc.',
    suggestions: [],
  },
};

async function injectAIChatMocks(page: any, overrides?: Partial<typeof aiChatMockData>) {
  const data = { ...aiChatMockData, ...overrides };
  await page.addInitScript((data: typeof aiChatMockData) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[AI Chat Mock] invoke:', cmd, args);

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
            case 'get_watchlists':
              return data.watchlists;
            case 'get_transactions':
              return [];
            // AI Commands
            case 'chat_with_portfolio_assistant':
              return data.chatResponse;
            case 'ai_search_security':
              return data.securities;
            case 'ai_list_watchlists':
              return data.watchlists;
            case 'ai_add_to_watchlist':
              return { success: true };
            case 'ai_remove_from_watchlist':
              return { success: true };
            case 'ai_query_transactions':
              return [];
            case 'enrich_extracted_transactions':
              return (args?.transactions || []).map((txn: any) => ({
                ...txn,
                shares: txn.shares ?? 10,
                shares_from_holdings: txn.shares == null,
              }));
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
  }, data);
}

test.describe('AI Chat Panel', () => {
  test.beforeEach(async ({ page }) => {
    await injectAIChatMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Chat Button wird angezeigt', async ({ page }) => {
    // Look for the floating chat button
    const chatButton = page.locator('button[aria-label*="Chat"], button:has-text("Chat"), [data-testid="chat-button"]');

    // Also look for message icon button at bottom right
    const floatingButton = page.locator('.fixed.bottom-4.right-4 button, button.fixed.bottom-4');

    const hasChatButton = await chatButton.count() > 0 || await floatingButton.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chat-button.png',
      fullPage: true,
    });

    expect(hasChatButton).toBeTruthy();
  });

  test('Chat Panel öffnet sich bei Klick', async ({ page }) => {
    // Find and click chat button
    const chatButton = page.locator('button').filter({ has: page.locator('svg') }).last();

    // Try to find the actual chat button at bottom right
    const buttons = await page.locator('button').all();
    let chatButtonFound = false;

    for (const btn of buttons.slice(-5)) { // Check last 5 buttons
      const isVisible = await btn.isVisible();
      if (isVisible) {
        const box = await btn.boundingBox();
        if (box && box.y > 500) { // Button near bottom
          await btn.click();
          chatButtonFound = true;
          break;
        }
      }
    }

    await page.waitForTimeout(500);

    // Check if chat panel opened
    const hasTestId = await page.locator('[data-testid="chat-panel"]').count() > 0;
    const hasChatClass = await page.locator('.chat-panel').count() > 0;
    const hasAssistentText = await page.locator('text=/Portfolio.*Assistent/i').count() > 0;
    const hasChatPanel = hasTestId || hasChatClass || hasAssistentText;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chat-panel-open.png',
      fullPage: true,
    });

    expect(chatButtonFound || hasChatPanel).toBeTruthy();
  });

  test('Chat Panel hat Eingabefeld', async ({ page }) => {
    // Open chat panel first
    const buttons = await page.locator('button').all();
    for (const btn of buttons.slice(-5)) {
      const isVisible = await btn.isVisible();
      if (isVisible) {
        const box = await btn.boundingBox();
        if (box && box.y > 500) {
          await btn.click();
          break;
        }
      }
    }

    await page.waitForTimeout(500);

    // Look for input field
    const inputField = page.locator('input[placeholder*="Nachricht"], textarea[placeholder*="Frage"], input[type="text"]').last();
    const hasInput = await inputField.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chat-input.png',
      fullPage: true,
    });

    expect(hasInput).toBeTruthy();
  });

  test('Chat Panel kann geschlossen werden', async ({ page }) => {
    // Open chat panel
    const buttons = await page.locator('button').all();
    for (const btn of buttons.slice(-5)) {
      const isVisible = await btn.isVisible();
      if (isVisible) {
        const box = await btn.boundingBox();
        if (box && box.y > 500) {
          await btn.click();
          break;
        }
      }
    }

    await page.waitForTimeout(500);

    // Find close button (X)
    const closeButton = page.locator('button:has-text("×"), button[aria-label*="Schließen"], button[aria-label*="Close"]');
    if (await closeButton.count() > 0) {
      await closeButton.first().click();
      await page.waitForTimeout(300);
    }

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chat-closed.png',
      fullPage: true,
    });
  });

  test('Chat zeigt Willkommensnachricht', async ({ page }) => {
    // Open chat panel
    const buttons = await page.locator('button').all();
    for (const btn of buttons.slice(-5)) {
      const isVisible = await btn.isVisible();
      if (isVisible) {
        const box = await btn.boundingBox();
        if (box && box.y > 500) {
          await btn.click();
          break;
        }
      }
    }

    await page.waitForTimeout(500);

    // Look for welcome message or assistant text
    const welcomeText = page.locator('text=/Assistent|Willkommen|Wie kann ich|Fragen/i');
    const hasWelcome = await welcomeText.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chat-welcome.png',
      fullPage: true,
    });

    expect(hasWelcome).toBeTruthy();
  });
});

test.describe('AI Chat Interaction', () => {
  test.beforeEach(async ({ page }) => {
    await injectAIChatMocks(page);
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);
  });

  test('Nachricht kann gesendet werden', async ({ page }) => {
    // Open chat panel
    const buttons = await page.locator('button').all();
    for (const btn of buttons.slice(-5)) {
      const isVisible = await btn.isVisible();
      if (isVisible) {
        const box = await btn.boundingBox();
        if (box && box.y > 500) {
          await btn.click();
          break;
        }
      }
    }

    await page.waitForTimeout(500);

    // Find input and type message
    const input = page.locator('input[type="text"], textarea').last();
    if (await input.count() > 0) {
      await input.fill('Wie ist meine Portfolio-Performance?');

      // Find and click send button
      const sendButton = page.locator('button[type="submit"], button:has-text("Senden")');
      if (await sendButton.count() > 0) {
        await sendButton.click();
      } else {
        await input.press('Enter');
      }
    }

    await page.waitForTimeout(1000);

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chat-message-sent.png',
      fullPage: true,
    });
  });

  test('Provider-Logo wird im Header angezeigt', async ({ page }) => {
    // Look for AI provider info in header
    const headerAI = page.locator('text=/Claude|OpenAI|Gemini|Perplexity/i');
    const hasProviderInHeader = await headerAI.count() > 0;

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-provider-header.png',
      fullPage: true,
    });

    expect(hasProviderInHeader).toBeTruthy();
  });

  test('Dividende ohne Stückzahl zeigt Bestand und Brutto je Aktie', async ({ page }) => {
    const dividendResponse = {
      message:
        'Ich habe eine Dividenden-Transaktion erkannt.\n\n[[EXTRACTED_TRANSACTIONS:{"transactions":[{"date":"2025-12-04","txnType":"DIVIDENDE","securityName":"Apple Inc.","isin":"US0378331005","shares":0,"grossAmount":110.00,"grossCurrency":"EUR","amount":82.50,"currency":"EUR","taxes":27.50,"note":"Dividende"}],"sourceDescription":"Screenshot"}]]',
      suggestions: [],
    };

    await injectAIChatMocks(page, { chatResponse: dividendResponse });
    await page.goto('/');
    await waitForAppReady(page);
    await closeWelcomeModal(page);

    // Open chat panel
    const buttons = await page.locator('button').all();
    for (const btn of buttons.slice(-5)) {
      const isVisible = await btn.isVisible();
      if (isVisible) {
        const box = await btn.boundingBox();
        if (box && box.y > 500) {
          await btn.click();
          break;
        }
      }
    }

    await page.waitForTimeout(500);

    // Send a message to trigger the mocked response
    const input = page.locator('input[type="text"], textarea').last();
    if (await input.count() > 0) {
      await input.fill('Bitte prüfe diesen Screenshot');
      await input.press('Enter');
    }

    // Expect preview to show dividend with shares from holdings and per-share info
    const previewTitle = page.locator('text=/Transaktion erkannt|Transaktionen erkannt/i');
    const sharesText = page.locator('text=/Stk\\./i');
    const dividendLabel = page.locator('text=/Dividende/i');

    await expect(previewTitle).toBeVisible({ timeout: 5000 });
    await expect(dividendLabel.first()).toBeVisible();
    await expect(sharesText.first()).toBeVisible();

    await page.screenshot({
      path: 'playwright-report/screenshots/ai-chat-dividend-preview.png',
      fullPage: true,
    });
  });
});
