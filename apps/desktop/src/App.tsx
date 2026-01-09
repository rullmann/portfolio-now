/**
 * Portfolio Performance Modern - Main Application
 *
 * Refactored modular structure:
 * - Store: Zustand for global state management
 * - Layout: Sidebar, Header, ErrorBanner, LoadingIndicator
 * - Views: Dashboard, Portfolio, Securities, Accounts, Transactions, Reports, Settings
 */

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import './index.css';

// Store
import {
  useUIStore,
  useAppStore,
  usePortfolioFileStore,
  useDataModeStore,
  useSettingsStore,
  toast,
} from './store';

// Layout components
import {
  Sidebar,
  Header,
  ErrorBanner,
  LoadingIndicator,
  ToastContainer,
} from './components/layout';

// Views
import {
  DashboardView,
  PortfolioView,
  SecuritiesViewWithErrorBoundary as SecuritiesView,
  AccountsView,
  TransactionsView,
  HoldingsView,
  AssetStatementView,
  WatchlistView,
  TaxonomiesView,
  InvestmentPlansView,
  RebalancingView,
  ChartsView,
  BenchmarkView,
  ReportsView,
  SettingsView,
} from './views';

// Types
import type { PortfolioFile, AggregatedHolding, PortfolioData } from './views';

// ============================================================================
// Main App Component
// ============================================================================

function App() {
  const { currentView } = useUIStore();
  const { setLoading, setError } = useAppStore();
  const { currentFilePath, setCurrentFilePath, setHasUnsavedChanges } = usePortfolioFileStore();
  const { setUseDbData } = useDataModeStore();
  const { theme } = useSettingsStore();

  // Legacy portfolio file state (for direct file editing)
  const [portfolioFile, setPortfolioFile] = useState<PortfolioFile | null>(null);

  // ============================================================================
  // Theme Management
  // ============================================================================

  useEffect(() => {
    const root = document.documentElement;

    if (theme === 'dark') {
      root.classList.add('dark');
    } else if (theme === 'light') {
      root.classList.remove('dark');
    } else {
      // System preference
      const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      if (prefersDark) {
        root.classList.add('dark');
      } else {
        root.classList.remove('dark');
      }
    }
  }, [theme]);

  // Listen for system theme changes when in 'system' mode
  useEffect(() => {
    if (theme !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (e: MediaQueryListEvent) => {
      if (e.matches) {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [theme]);

  // DB-based state
  const [dbPortfolios, setDbPortfolios] = useState<PortfolioData[]>([]);
  const [dbHoldings, setDbHoldings] = useState<AggregatedHolding[]>([]);
  const [dbPortfolioHistory, setDbPortfolioHistory] = useState<Array<{ date: string; value: number }>>([]);
  const [dbInvestedCapitalHistory, setDbInvestedCapitalHistory] = useState<Array<{ date: string; value: number }>>([]);

  // ============================================================================
  // Data Loading
  // ============================================================================

  const loadDbHoldings = useCallback(async () => {
    try {
      setLoading(true);

      // Get all portfolios for display
      const portfolios = await invoke<PortfolioData[]>('get_pp_portfolios', { importId: null });
      setDbPortfolios(portfolios);

      // Get aggregated holdings by ISIN
      const holdings = await invoke<AggregatedHolding[]>('get_all_holdings');
      setDbHoldings(holdings);

      // Get portfolio history for chart
      try {
        const history = await invoke<Array<{ date: string; value: number }>>('get_portfolio_history');
        setDbPortfolioHistory(history);
      } catch (historyErr) {
        console.warn('Could not load portfolio history:', historyErr);
      }

      // Get invested capital history for chart
      try {
        const investedHistory = await invoke<Array<{ date: string; value: number }>>('get_invested_capital_history');
        setDbInvestedCapitalHistory(investedHistory);
      } catch (investedErr) {
        console.warn('Could not load invested capital history:', investedErr);
      }

      setUseDbData(true);
    } catch (err) {
      setError(`Fehler beim Laden der Holdings: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [setLoading, setError, setUseDbData]);

  // Load DB data on mount
  useEffect(() => {
    loadDbHoldings();
  }, [loadDbHoldings]);

  // Sync quotes on startup
  useEffect(() => {
    const syncQuotesOnStartup = async () => {
      try {
        // Small delay to let UI render first
        await new Promise(resolve => setTimeout(resolve, 1000));

        const result = await invoke<{ total: number; success: number; errors: number }>('sync_all_prices', {
          onlyHeld: false, // Sync all securities including watchlist
          apiKeys: null,
        });

        if (result.success > 0) {
          toast.success(`${result.success} Kurse aktualisiert`);
          // Reload holdings to update values
          loadDbHoldings();
        }
        if (result.errors > 0) {
          console.warn(`${result.errors} Kurse konnten nicht aktualisiert werden`);
        }
      } catch (err) {
        console.warn('Quote sync on startup failed:', err);
      }
    };

    syncQuotesOnStartup();
  }, []); // Only run once on mount

  // ============================================================================
  // File Operations
  // ============================================================================

  const handleNewFile = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const newPortfolio = await invoke<PortfolioFile>('create_new_portfolio', {
        baseCurrency: 'EUR',
      });
      setPortfolioFile(newPortfolio);
      setCurrentFilePath(null);
      setHasUnsavedChanges(true);
    } catch (err) {
      setError(`Fehler beim Erstellen: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [setLoading, setError, setCurrentFilePath, setHasUnsavedChanges]);

  const handleOpenFile = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Portfolio Performance', extensions: ['portfolio'] },
          { name: 'Alle Dateien', extensions: ['*'] },
        ],
      });

      if (selected) {
        setLoading(true);
        setError(null);
        const result = await invoke<{ path: string; portfolio: PortfolioFile }>('open_portfolio_file', {
          path: selected,
        });
        setPortfolioFile(result.portfolio);
        setCurrentFilePath(result.path);
        setHasUnsavedChanges(false);
      }
    } catch (err) {
      setError(`Fehler beim Öffnen: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [setLoading, setError, setCurrentFilePath, setHasUnsavedChanges]);

  const handleSaveFile = useCallback(async () => {
    if (!portfolioFile) return;

    try {
      let savePath = currentFilePath;

      if (!savePath) {
        const selected = await save({
          filters: [
            { name: 'Portfolio Performance', extensions: ['portfolio'] },
          ],
          defaultPath: 'portfolio.portfolio',
        });
        if (!selected) return;
        savePath = selected;
      }

      setLoading(true);
      setError(null);
      await invoke('save_portfolio_file', {
        path: savePath,
        portfolio: portfolioFile,
      });
      setCurrentFilePath(savePath);
      setHasUnsavedChanges(false);
    } catch (err) {
      setError(`Fehler beim Speichern: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [portfolioFile, currentFilePath, setLoading, setError, setCurrentFilePath, setHasUnsavedChanges]);

  const handleSaveAsFile = useCallback(async () => {
    if (!portfolioFile) return;

    try {
      const selected = await save({
        filters: [
          { name: 'Portfolio Performance', extensions: ['portfolio'] },
        ],
        defaultPath: currentFilePath || 'portfolio.portfolio',
      });

      if (selected) {
        setLoading(true);
        setError(null);
        await invoke('save_portfolio_file', {
          path: selected,
          portfolio: portfolioFile,
        });
        setCurrentFilePath(selected);
        setHasUnsavedChanges(false);
      }
    } catch (err) {
      setError(`Fehler beim Speichern: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [portfolioFile, currentFilePath, setLoading, setError, setCurrentFilePath, setHasUnsavedChanges]);

  const handleImportToDb = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'Portfolio Performance', extensions: ['portfolio'] },
        ],
      });

      if (selected) {
        setLoading(true);
        setError(null);

        await invoke('import_pp_file', { path: selected });

        // Reload holdings
        await loadDbHoldings();
      }
    } catch (err) {
      setError(`Fehler beim Import: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [loadDbHoldings, setLoading, setError]);

  // ============================================================================
  // View Router
  // ============================================================================

  const renderView = () => {
    switch (currentView) {
      case 'dashboard':
        return (
          <DashboardView
            portfolioFile={portfolioFile}
            dbHoldings={dbHoldings}
            dbPortfolios={dbPortfolios}
            dbPortfolioHistory={dbPortfolioHistory}
            dbInvestedCapitalHistory={dbInvestedCapitalHistory}
            onOpenFile={handleOpenFile}
            onImportToDb={handleImportToDb}
            onRefreshHoldings={loadDbHoldings}
          />
        );
      case 'portfolio':
        return <PortfolioView portfolioFile={portfolioFile} dbPortfolios={dbPortfolios} />;
      case 'securities':
        return <SecuritiesView portfolioFile={portfolioFile} />;
      case 'accounts':
        return <AccountsView portfolioFile={portfolioFile} />;
      case 'transactions':
        return <TransactionsView portfolioFile={portfolioFile} />;
      case 'holdings':
        return <HoldingsView dbHoldings={dbHoldings} dbPortfolios={dbPortfolios} />;
      case 'asset-statement':
        return <AssetStatementView dbHoldings={dbHoldings} dbPortfolios={dbPortfolios} />;
      case 'watchlist':
        return <WatchlistView />;
      case 'taxonomies':
        return <TaxonomiesView />;
      case 'plans':
        return <InvestmentPlansView />;
      case 'rebalancing':
        return <RebalancingView />;
      case 'charts':
        return <ChartsView />;
      case 'benchmark':
        return <BenchmarkView />;
      case 'reports':
        return <ReportsView />;
      case 'settings':
        return <SettingsView />;
      default:
        return (
          <DashboardView
            portfolioFile={portfolioFile}
            dbHoldings={dbHoldings}
            dbPortfolios={dbPortfolios}
            dbPortfolioHistory={dbPortfolioHistory}
            dbInvestedCapitalHistory={dbInvestedCapitalHistory}
            onOpenFile={handleOpenFile}
            onImportToDb={handleImportToDb}
            onRefreshHoldings={loadDbHoldings}
          />
        );
    }
  };

  // ============================================================================
  // Render
  // ============================================================================

  return (
    <div className="flex h-screen bg-background">
      {/* Skip link for keyboard users */}
      <a href="#main-content" className="skip-link">
        Zum Hauptinhalt springen
      </a>

      {/* Sidebar */}
      <Sidebar />

      {/* Main Content */}
      <main id="main-content" className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <Header
          onNewFile={handleNewFile}
          onOpenFile={handleOpenFile}
          onSaveFile={handleSaveFile}
          onSaveAsFile={handleSaveAsFile}
          onImportToDb={handleImportToDb}
          onRefresh={loadDbHoldings}
          hasPortfolioFile={!!portfolioFile}
        />

        {/* Error Banner */}
        <ErrorBanner />

        {/* Loading Indicator */}
        <LoadingIndicator />

        {/* Content Area */}
        <div className="flex-1 overflow-auto p-4">
          {renderView()}
        </div>
      </main>

      {/* Toast notifications */}
      <ToastContainer />
    </div>
  );
}

export default App;
