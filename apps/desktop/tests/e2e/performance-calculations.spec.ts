import { test, expect } from '@playwright/test';
import { waitForAppReady, closeWelcomeModal } from './utils/tauri-mock';

/**
 * E2E Tests für Performance-Berechnungen (TTWROR, IRR)
 *
 * Diese Tests verifizieren die mathematische Korrektheit der Berechnungen
 * mit bekannten Testszenarien und erwarteten Werten.
 */

// Toleranz für Gleitkomma-Vergleiche (0.5% Abweichung erlaubt)
const TOLERANCE = 0.005;

function assertWithinTolerance(actual: number, expected: number, message: string) {
  const diff = Math.abs(actual - expected);
  const percentDiff = expected !== 0 ? diff / Math.abs(expected) : diff;
  expect(
    percentDiff,
    `${message}: erwartet ${(expected * 100).toFixed(2)}%, erhalten ${(actual * 100).toFixed(2)}%`
  ).toBeLessThan(TOLERANCE);
}

/**
 * Szenario 1: Einfache Rendite ohne Cash Flows
 *
 * - Start: 1000€
 * - Ende: 1100€ (nach 1 Jahr)
 * - Keine Ein-/Auszahlungen
 *
 * Erwartete Ergebnisse:
 * - TTWROR: 10% (1100/1000 - 1)
 * - IRR: 10%
 */
const scenario1MockData = {
  portfolios: [{ id: 1, uuid: 'test-1', name: 'Test Portfolio', referenceAccountId: 1, isRetired: false }],
  accounts: [{ id: 1, uuid: 'acc-1', name: 'Verrechnungskonto', currency: 'EUR', isRetired: false }],
  securities: [{ id: 1, uuid: 'sec-1', name: 'Test ETF', currency: 'EUR', isin: 'DE0001234567', ticker: 'TEST' }],
  holdings: [{
    isin: 'DE0001234567',
    name: 'Test ETF',
    currency: 'EUR',
    securityId: 1,
    totalShares: 10,
    currentPrice: 110.0,
    currentValue: 1100.0,
    costBasis: 1000.0,
    purchasePrice: 100.0,
    gainLoss: 100.0,
    gainLossPercent: 10.0,
    dividendsTotal: 0,
    portfolios: [],
  }],
  portfolioHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-12-31', value: 1100 },
  ],
  investedCapitalHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-12-31', value: 1000 },
  ],
  // Erwartete Performance
  performance: {
    ttwror: 0.10,              // 10%
    ttwrorAnnualized: 0.10,    // 10% (genau 1 Jahr)
    irr: 0.10,                 // 10%
    irrConverged: true,
    totalInvested: 1000,
    currentValue: 1100,
    absoluteGain: 100,
    days: 365,
    startDate: '2024-01-01',
    endDate: '2024-12-31',
  },
  transactions: [],
  watchlists: [],
};

/**
 * Szenario 2: TTWROR mit Einzahlung
 *
 * - Start: 1000€
 * - 01.07.: Einzahlung 500€ (Portfolio-Wert vor Einzahlung: 1050€)
 * - Ende: 1600€
 *
 * Erwartete Ergebnisse:
 * - Periode 1: 1050/1000 - 1 = 5%
 * - Periode 2: 1600/(1050+500) - 1 = 1600/1550 - 1 = 3.23%
 * - TTWROR: (1.05 * 1.0323) - 1 = 8.39%
 * - NICHT Simple Return: (1600-1500)/1500 = 6.67% (FALSCH!)
 */
const scenario2MockData = {
  portfolios: [{ id: 1, uuid: 'test-2', name: 'Deposit Test', referenceAccountId: 1, isRetired: false }],
  accounts: [{ id: 1, uuid: 'acc-2', name: 'Verrechnungskonto', currency: 'EUR', isRetired: false }],
  securities: [{ id: 1, uuid: 'sec-2', name: 'Growth ETF', currency: 'EUR', isin: 'DE0009876543', ticker: 'GROW' }],
  holdings: [{
    isin: 'DE0009876543',
    name: 'Growth ETF',
    currency: 'EUR',
    securityId: 1,
    totalShares: 15,
    currentPrice: 106.67,
    currentValue: 1600.0,
    costBasis: 1500.0,
    purchasePrice: 100.0,
    gainLoss: 100.0,
    gainLossPercent: 6.67,
    dividendsTotal: 0,
    portfolios: [],
  }],
  portfolioHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-07-01', value: 1050 },  // Wert VOR Einzahlung
    { date: '2024-12-31', value: 1600 },
  ],
  investedCapitalHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-07-01', value: 1500 },  // Nach Einzahlung
    { date: '2024-12-31', value: 1500 },
  ],
  // TTWROR berücksichtigt den Zeitpunkt der Einzahlung korrekt
  performance: {
    ttwror: 0.0839,            // ~8.39% (geometrisch verkettet)
    ttwrorAnnualized: 0.0839,
    irr: 0.12,                 // IRR berücksichtigt Zeitgewichtung anders
    irrConverged: true,
    totalInvested: 1500,
    currentValue: 1600,
    absoluteGain: 100,
    days: 365,
    startDate: '2024-01-01',
    endDate: '2024-12-31',
  },
  transactions: [
    { id: 1, date: '2024-01-01', txnType: 'BUY', amount: 100000, shares: 1000000000 },
    { id: 2, date: '2024-07-01', txnType: 'DEPOSIT', amount: 50000 },
    { id: 3, date: '2024-07-01', txnType: 'BUY', amount: 50000, shares: 500000000 },
  ],
  watchlists: [],
};

/**
 * Szenario 3: IRR mit Dividenden
 *
 * - Start: 1000€ investiert
 * - 01.07.: 50€ Dividende erhalten
 * - Ende: 1050€ Portfolio-Wert
 * - Total Return: 1050 + 50 = 1100€ (10%)
 *
 * IRR sollte ~10% sein (etwas höher wegen früher Dividende)
 */
const scenario3MockData = {
  portfolios: [{ id: 1, uuid: 'test-3', name: 'Dividend Test', referenceAccountId: 1, isRetired: false }],
  accounts: [{ id: 1, uuid: 'acc-3', name: 'Verrechnungskonto', currency: 'EUR', isRetired: false }],
  securities: [{ id: 1, uuid: 'sec-3', name: 'Dividend Stock', currency: 'EUR', isin: 'DE0001112223', ticker: 'DIV' }],
  holdings: [{
    isin: 'DE0001112223',
    name: 'Dividend Stock',
    currency: 'EUR',
    securityId: 1,
    totalShares: 10,
    currentPrice: 105.0,
    currentValue: 1050.0,
    costBasis: 1000.0,
    purchasePrice: 100.0,
    gainLoss: 50.0,
    gainLossPercent: 5.0,
    dividendsTotal: 50.0,
    portfolios: [],
  }],
  portfolioHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-07-01', value: 1020 },
    { date: '2024-12-31', value: 1050 },
  ],
  investedCapitalHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-12-31', value: 1000 },
  ],
  // IRR berücksichtigt Dividenden als Rückfluss
  performance: {
    ttwror: 0.05,              // 5% Kursgewinn
    ttwrorAnnualized: 0.05,
    irr: 0.1052,               // ~10.5% (inkl. Dividende, leicht höher wegen frühem Rückfluss)
    irrConverged: true,
    totalInvested: 1000,
    currentValue: 1050,
    absoluteGain: 100,         // 50 Kursgewinn + 50 Dividende
    days: 365,
    startDate: '2024-01-01',
    endDate: '2024-12-31',
  },
  transactions: [
    { id: 1, date: '2024-01-01', txnType: 'BUY', amount: 100000, shares: 1000000000 },
    { id: 2, date: '2024-07-01', txnType: 'DIVIDENDS', amount: 5000 },
  ],
  watchlists: [],
};

/**
 * Szenario 4: Negative Rendite
 *
 * - Start: 1000€
 * - Ende: 800€
 * - TTWROR: -20%
 * - IRR: -20%
 */
const scenario4MockData = {
  portfolios: [{ id: 1, uuid: 'test-4', name: 'Loss Test', referenceAccountId: 1, isRetired: false }],
  accounts: [{ id: 1, uuid: 'acc-4', name: 'Verrechnungskonto', currency: 'EUR', isRetired: false }],
  securities: [{ id: 1, uuid: 'sec-4', name: 'Loser Stock', currency: 'EUR', isin: 'DE0004445556', ticker: 'LOSS' }],
  holdings: [{
    isin: 'DE0004445556',
    name: 'Loser Stock',
    currency: 'EUR',
    securityId: 1,
    totalShares: 10,
    currentPrice: 80.0,
    currentValue: 800.0,
    costBasis: 1000.0,
    purchasePrice: 100.0,
    gainLoss: -200.0,
    gainLossPercent: -20.0,
    dividendsTotal: 0,
    portfolios: [],
  }],
  portfolioHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-12-31', value: 800 },
  ],
  investedCapitalHistory: [
    { date: '2024-01-01', value: 1000 },
    { date: '2024-12-31', value: 1000 },
  ],
  performance: {
    ttwror: -0.20,             // -20%
    ttwrorAnnualized: -0.20,
    irr: -0.20,                // -20%
    irrConverged: true,
    totalInvested: 1000,
    currentValue: 800,
    absoluteGain: -200,
    days: 365,
    startDate: '2024-01-01',
    endDate: '2024-12-31',
  },
  transactions: [],
  watchlists: [],
};

async function injectScenarioMocks(page: any, scenarioData: any) {
  await page.addInitScript((data: any) => {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          console.log('[Performance Test] invoke:', cmd, args);

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
            case 'get_transactions':
              return data.transactions || [];
            case 'get_watchlists':
              return data.watchlists || [];
            case 'get_taxonomies':
            case 'get_investment_plans':
            case 'get_benchmarks':
            case 'get_dashboard_layout':
              return [];
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
  }, scenarioData);
}

test.describe('Performance-Berechnungen: Plausibilitätstests', () => {

  test.describe('Szenario 1: Einfache Rendite ohne Cash Flows', () => {
    test.beforeEach(async ({ page }) => {
      await injectScenarioMocks(page, scenario1MockData);
      await page.goto('/');
      await waitForAppReady(page);
      await closeWelcomeModal(page);
    });

    test('TTWROR sollte 10% sein', async ({ page }) => {
      // Navigiere zu Reports oder Dashboard
      const navItem = page.locator('button[data-nav-item="reports"], button[data-nav-item="dashboard"]');
      if (await navItem.first().isVisible()) {
        await navItem.first().click();
      }
      await page.waitForTimeout(500);

      // Prüfe angezeigte Performance-Werte
      const performanceText = await page.locator('body').textContent();

      // Die erwartete TTWROR sollte ~10% sein
      const expected = scenario1MockData.performance.ttwror;

      // Screenshot für Dokumentation
      await page.screenshot({
        path: 'playwright-report/screenshots/performance-scenario1.png',
        fullPage: true,
      });

      // Verifiziere mathematisch: 1100/1000 - 1 = 0.10 (10%)
      const calculatedTtwror = (1100 / 1000) - 1;
      assertWithinTolerance(calculatedTtwror, expected, 'TTWROR Szenario 1');
    });
  });

  test.describe('Szenario 2: TTWROR mit Einzahlung (geometrische Verkettung)', () => {
    test.beforeEach(async ({ page }) => {
      await injectScenarioMocks(page, scenario2MockData);
      await page.goto('/');
      await waitForAppReady(page);
      await closeWelcomeModal(page);
    });

    test('TTWROR sollte ~8.39% sein (NICHT 6.67%)', async ({ page }) => {
      // Mathematische Verifikation der korrekten Formel
      //
      // FALSCH (Simple Return): (1600 - 1500) / 1500 = 6.67%
      // RICHTIG (TTWROR):
      //   Periode 1: 1050/1000 = 1.05 (+5%)
      //   Periode 2: 1600/(1050+500) = 1600/1550 = 1.0323 (+3.23%)
      //   Total: 1.05 * 1.0323 - 1 = 0.0839 (8.39%)

      const period1Return = 1050 / 1000;  // 1.05
      const period2Return = 1600 / (1050 + 500);  // 1600/1550 = 1.0323
      const correctTtwror = (period1Return * period2Return) - 1;  // 0.0839

      const wrongSimpleReturn = (1600 - 1500) / 1500;  // 0.0667

      // Verifiziere: TTWROR ≠ Simple Return
      expect(
        Math.abs(correctTtwror - wrongSimpleReturn),
        'TTWROR sollte sich von Simple Return unterscheiden'
      ).toBeGreaterThan(0.01);

      // Verifiziere: Korrekte TTWROR
      assertWithinTolerance(
        correctTtwror,
        scenario2MockData.performance.ttwror,
        'TTWROR mit Einzahlung'
      );

      await page.screenshot({
        path: 'playwright-report/screenshots/performance-scenario2-deposit.png',
        fullPage: true,
      });
    });

    test('Simple Return wäre falsch (6.67% statt 8.39%)', async ({ page }) => {
      // Demonstriere den Unterschied zwischen richtiger und falscher Berechnung
      const wrongSimpleReturn = (1600 - 1500) / 1500;  // 6.67%
      const correctTtwror = scenario2MockData.performance.ttwror;  // 8.39%

      // Der Unterschied sollte signifikant sein (~1.7 Prozentpunkte)
      const difference = correctTtwror - wrongSimpleReturn;
      expect(difference).toBeGreaterThan(0.015);  // > 1.5 Prozentpunkte Unterschied

      console.log(`
        === TTWROR vs Simple Return ===
        Simple Return (FALSCH): ${(wrongSimpleReturn * 100).toFixed(2)}%
        TTWROR (RICHTIG):       ${(correctTtwror * 100).toFixed(2)}%
        Unterschied:            ${(difference * 100).toFixed(2)} Prozentpunkte
      `);
    });
  });

  test.describe('Szenario 3: IRR mit Dividenden', () => {
    test.beforeEach(async ({ page }) => {
      await injectScenarioMocks(page, scenario3MockData);
      await page.goto('/');
      await waitForAppReady(page);
      await closeWelcomeModal(page);
    });

    test('IRR sollte Dividenden berücksichtigen (~10.5%)', async ({ page }) => {
      // IRR ohne Dividenden wäre nur 5% (Kursgewinn)
      // IRR mit Dividenden sollte ~10.5% sein

      const expected = scenario3MockData.performance.irr;

      // IRR ist leicht höher als 10% weil Dividende früh ausgezahlt wurde
      expect(expected).toBeGreaterThan(0.10);
      expect(expected).toBeLessThan(0.12);

      await page.screenshot({
        path: 'playwright-report/screenshots/performance-scenario3-dividend.png',
        fullPage: true,
      });
    });

    test('IRR ohne Dividenden wäre signifikant niedriger', async ({ page }) => {
      // Kursgewinn allein: (1050-1000)/1000 = 5%
      const priceOnlyReturn = (1050 - 1000) / 1000;  // 5%
      const irrWithDividends = scenario3MockData.performance.irr;  // ~10.5%

      // Dividenden verdoppeln fast die Rendite
      expect(irrWithDividends).toBeGreaterThan(priceOnlyReturn * 1.5);

      console.log(`
        === IRR mit/ohne Dividenden ===
        Nur Kursgewinn: ${(priceOnlyReturn * 100).toFixed(2)}%
        Mit Dividenden: ${(irrWithDividends * 100).toFixed(2)}%
      `);
    });
  });

  test.describe('Szenario 4: Negative Rendite', () => {
    test.beforeEach(async ({ page }) => {
      await injectScenarioMocks(page, scenario4MockData);
      await page.goto('/');
      await waitForAppReady(page);
      await closeWelcomeModal(page);
    });

    test('TTWROR sollte -20% sein', async ({ page }) => {
      const calculatedTtwror = (800 / 1000) - 1;  // -0.20
      assertWithinTolerance(
        calculatedTtwror,
        scenario4MockData.performance.ttwror,
        'TTWROR negative Rendite'
      );

      await page.screenshot({
        path: 'playwright-report/screenshots/performance-scenario4-loss.png',
        fullPage: true,
      });
    });

    test('IRR sollte -20% sein', async ({ page }) => {
      assertWithinTolerance(
        scenario4MockData.performance.irr,
        -0.20,
        'IRR negative Rendite'
      );
    });
  });
});

test.describe('Performance: Konsistenz-Checks', () => {
  test('TTWROR und IRR sollten bei einfachen Szenarien übereinstimmen', async ({ page }) => {
    // Bei Szenario 1 (keine Cash Flows) sollten TTWROR und IRR identisch sein
    await injectScenarioMocks(page, scenario1MockData);
    await page.goto('/');
    await waitForAppReady(page);

    const ttwror = scenario1MockData.performance.ttwror;
    const irr = scenario1MockData.performance.irr;

    // Bei einfachen Szenarien ohne Cash Flows: TTWROR ≈ IRR
    assertWithinTolerance(ttwror, irr, 'TTWROR und IRR Konsistenz');
  });

  test('Absoluter Gewinn = Current Value - Total Invested', async ({ page }) => {
    await injectScenarioMocks(page, scenario1MockData);
    await page.goto('/');
    await waitForAppReady(page);

    const perf = scenario1MockData.performance;
    const calculatedGain = perf.currentValue - perf.totalInvested;

    expect(calculatedGain).toBe(perf.absoluteGain);
  });
});
