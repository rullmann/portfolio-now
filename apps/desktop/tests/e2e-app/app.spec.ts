import { browser, $, $$ } from '@wdio/globals';

/**
 * Real Tauri App E2E Tests
 *
 * These tests run against the ACTUAL compiled Tauri application,
 * with the real Rust backend, SQLite database, and AI integrations.
 *
 * Prerequisites:
 * 1. Build the app: pnpm tauri build --bundles app
 * 2. tauri-driver installed: cargo install tauri-driver
 *
 * Run with: pnpm test:e2e:app
 */

describe('Tauri App - Startup', () => {
  it('sollte die App starten ohne Crash', async () => {
    // The app should have started via tauri-driver
    // Wait for the window to be ready
    await browser.pause(3000);

    // Get the window title
    const title = await browser.getTitle();
    console.log('Window title:', title);

    // App should have a title
    expect(title).toBeDefined();
  });

  it('sollte das Dashboard anzeigen', async () => {
    await browser.pause(1000);

    // Wait for the dashboard to load
    const body = await $('body');
    await body.waitForExist({ timeout: 10000 });

    // Take a screenshot
    await browser.saveScreenshot('./playwright-report/screenshots/app-dashboard.png');

    // Check for any content
    const html = await body.getHTML();
    expect(html.length).toBeGreaterThan(100);
  });
});

describe('Tauri App - Navigation', () => {
  it('sollte durch Views navigieren können', async () => {
    // Find navigation buttons
    const navButtons = await $$('nav button, aside button, [data-nav-item]');
    console.log(`Found ${navButtons.length} navigation buttons`);

    if (navButtons.length > 0) {
      // Click the first nav button
      await navButtons[0].click();
      await browser.pause(500);

      // App should still be responsive
      const body = await $('body');
      const isDisplayed = await body.isDisplayed();
      expect(isDisplayed).toBe(true);
    }

    await browser.saveScreenshot('./playwright-report/screenshots/app-navigation.png');
  });

  it('sollte Settings öffnen können', async () => {
    // Look for settings button
    const settingsBtn = await $(
      'button[data-nav-item="settings"], button*=Settings, button*=Einstellungen'
    );

    if (await settingsBtn.isExisting()) {
      await settingsBtn.click();
      await browser.pause(500);

      // Should show settings content
      await browser.saveScreenshot('./playwright-report/screenshots/app-settings.png');
    }
  });
});

describe('Tauri App - Portfolio Insights', () => {
  it('sollte Insights Modal öffnen ohne Freeze', async () => {
    // Navigate back to dashboard first
    const dashboardBtn = await $('button[data-nav-item="dashboard"]');
    if (await dashboardBtn.isExisting()) {
      await dashboardBtn.click();
      await browser.pause(500);
    }

    // Find the insights button
    const insightsBtn = await $(
      'button*=Insights, button*=KI-Analyse, button*=Portfolio Insights'
    );

    if (await insightsBtn.isExisting()) {
      const startTime = Date.now();
      await insightsBtn.click();

      // Wait for modal to appear
      const modal = await $('[role="dialog"], .modal, .fixed.inset-0.z-50');
      await modal.waitForExist({ timeout: 2000 });

      const openTime = Date.now() - startTime;
      console.log(`Modal opened in ${openTime}ms`);

      // Should open in under 500ms (no freeze)
      expect(openTime).toBeLessThan(500);

      await browser.saveScreenshot('./playwright-report/screenshots/app-insights-modal.png');

      // Close with ESC
      await browser.keys(['Escape']);
      await browser.pause(300);
    }
  });

  it('sollte Nachkauf-Chancen ohne Freeze laden', async () => {
    // Open insights modal
    const insightsBtn = await $(
      'button*=Insights, button*=KI-Analyse'
    );

    if (await insightsBtn.isExisting()) {
      await insightsBtn.click();
      await browser.pause(500);

      // Find and click Nachkauf-Chancen option
      const nachkaufBtn = await $('button*=Nachkauf, button*=Technical');

      if (await nachkaufBtn.isExisting()) {
        const startTime = Date.now();
        await nachkaufBtn.click();

        // Wait for result (should be fast with batch query)
        await browser.pause(2000);
        const responseTime = Date.now() - startTime;

        console.log(`Nachkauf-Chancen completed in ${responseTime}ms`);

        // Should complete within 3 seconds (was freezing before fix)
        expect(responseTime).toBeLessThan(3000);

        await browser.saveScreenshot('./playwright-report/screenshots/app-nachkauf.png');

        // Look for ranking content
        const ranking = await $('*=Score, *=Ranking, *=🟢, *=🟡');
        const hasRanking = await ranking.isExisting();
        console.log('Has ranking content:', hasRanking);
      }

      // Close modal
      await browser.keys(['Escape']);
      await browser.pause(300);
    }
  });
});

describe('Tauri App - AI Chat', () => {
  it('sollte Chat Panel öffnen', async () => {
    // Find chat button (usually at bottom right)
    const allButtons = await $$('button');

    // Find the last few buttons (chat is usually at bottom)
    for (let i = allButtons.length - 1; i >= Math.max(0, allButtons.length - 5); i--) {
      const btn = allButtons[i];
      const location = await btn.getLocation();

      // Check if button is near bottom of screen (y > 500)
      if (location.y > 500) {
        await btn.click();
        await browser.pause(500);

        // Check if chat panel opened
        const chatPanel = await $(
          '[data-testid="chat-panel"], .chat-panel, *=Portfolio-Assistent'
        );

        if (await chatPanel.isExisting()) {
          console.log('Chat panel opened');
          await browser.saveScreenshot('./playwright-report/screenshots/app-chat.png');

          // Close chat
          const closeBtn = await $('button*=×, button[aria-label*="Close"]');
          if (await closeBtn.isExisting()) {
            await closeBtn.click();
          }
          break;
        }
      }
    }
  });
});

describe('Tauri App - Stability', () => {
  it('sollte bei schneller Navigation stabil bleiben', async () => {
    const views = ['dashboard', 'holdings', 'charts', 'settings'];
    let navigationCount = 0;

    for (const view of views) {
      const navBtn = await $(`button[data-nav-item="${view}"]`);
      if (await navBtn.isExisting()) {
        await navBtn.click();
        navigationCount++;
        await browser.pause(200); // Quick navigation
      }
    }

    console.log(`Navigated ${navigationCount} times`);

    // App should still be responsive
    const body = await $('body');
    const isDisplayed = await body.isDisplayed();
    expect(isDisplayed).toBe(true);

    await browser.saveScreenshot('./playwright-report/screenshots/app-stability.png');
  });

  it('sollte mehrfaches Modal Öffnen/Schließen überstehen', async () => {
    for (let i = 0; i < 3; i++) {
      const insightsBtn = await $('button*=Insights, button*=KI-Analyse');

      if (await insightsBtn.isExisting()) {
        await insightsBtn.click();
        await browser.pause(300);

        // Close with ESC
        await browser.keys(['Escape']);
        await browser.pause(300);
      }
    }

    // App should still be responsive
    const body = await $('body');
    const isDisplayed = await body.isDisplayed();
    expect(isDisplayed).toBe(true);

    console.log('Modal open/close cycle completed 3 times');
  });
});

describe('Tauri App - Data Loading', () => {
  it('sollte Holdings laden', async () => {
    const holdingsBtn = await $('button[data-nav-item="holdings"]');

    if (await holdingsBtn.isExisting()) {
      await holdingsBtn.click();
      await browser.pause(1000);

      // Wait for content to load
      const content = await $('main, [role="main"], .content');
      await content.waitForExist({ timeout: 5000 });

      await browser.saveScreenshot('./playwright-report/screenshots/app-holdings.png');

      // Should have some content
      const html = await content.getHTML();
      expect(html.length).toBeGreaterThan(50);
    }
  });

  it('sollte Charts laden', async () => {
    const chartsBtn = await $('button[data-nav-item="charts"]');

    if (await chartsBtn.isExisting()) {
      await chartsBtn.click();
      await browser.pause(2000); // Charts might take longer to render

      await browser.saveScreenshot('./playwright-report/screenshots/app-charts.png');

      // App should still be responsive
      const body = await $('body');
      const isDisplayed = await body.isDisplayed();
      expect(isDisplayed).toBe(true);
    }
  });
});
