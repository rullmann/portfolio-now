/**
 * Portfolio Performance Modern - Main Application
 *
 * Refactored modular structure:
 * - Store: Zustand for global state management
 * - TanStack Query: Server state management with caching
 * - Layout: Sidebar, Header, ErrorBanner, LoadingIndicator
 * - Views: Dashboard, Portfolio, Securities, Accounts, Transactions, Reports, Settings
 */

import { useCallback, useEffect, useState } from 'react';
import { QueryClientProvider } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import './index.css';

// TanStack Query
import { queryClient, invalidateAllQueries } from './lib/queries';

// Error Handling
import { setGlobalErrorHandler } from './lib/errors';
import { toast } from './store';

// Store
import {
  useUIStore,
  useAppStore,
  useSettingsStore,
} from './store';

// Layout components
import {
  Sidebar,
  Header,
  ErrorBanner,
  LoadingIndicator,
  ToastContainer,
} from './components/layout';

// Chat components
import { ChatButton, ChatPanel } from './components/chat';

// Modals
import { WelcomeModal } from './components/modals';

// Views
import {
  DashboardView,
  PortfolioView,
  SecuritiesViewWithErrorBoundary as SecuritiesView,
  AccountsView,
  TransactionsView,
  HoldingsView,
  DividendsView,
  AssetStatementView,
  WatchlistView,
  TaxonomiesView,
  InvestmentPlansView,
  RebalancingView,
  ChartsView,
  ScreenerView,
  BenchmarkView,
  ReportsView,
  SettingsView,
} from './views';

// Types
import type { AggregatedHolding, PortfolioData } from './views';

// ============================================================================
// Main App Component
// ============================================================================

function App() {
  const { currentView } = useUIStore();
  const { setLoading, setError } = useAppStore();
  const { theme, userName } = useSettingsStore();

  // Welcome modal state - show only on first launch when no userName is set
  const [showWelcome, setShowWelcome] = useState<boolean | null>(null);

  // Check if we should show the welcome modal (only once on first mount)
  useEffect(() => {
    // Zustand persist might not have rehydrated yet, wait a tick
    const timer = setTimeout(() => {
      // Show welcome only if userName has never been set (is empty string from default)
      // Once user skips or sets a name, it's been "seen"
      const hasSeenWelcome = localStorage.getItem('portfolio-welcome-seen');
      if (!hasSeenWelcome && !userName) {
        setShowWelcome(true);
      } else {
        setShowWelcome(false);
      }
    }, 100);
    return () => clearTimeout(timer);
  }, []);

  const handleWelcomeClose = () => {
    setShowWelcome(false);
    localStorage.setItem('portfolio-welcome-seen', 'true');
  };

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

  // Chat panel state
  const [isChatOpen, setIsChatOpen] = useState(false);

  // ============================================================================
  // Data Loading
  // ============================================================================

  const loadDbData = useCallback(async () => {
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
    } catch (err) {
      setError(`Fehler beim Laden der Daten: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [setLoading, setError]);

  // Load DB data on mount
  useEffect(() => {
    loadDbData();
  }, [loadDbData]);

  // Listen for data_changed events from backend
  useEffect(() => {
    const unlisten = listen<{ entity: string; action: string }>('data_changed', (event) => {
      console.log('Data changed event received:', event.payload);
      // Invalidate all TanStack Query caches
      invalidateAllQueries();
      // Reload local state data
      loadDbData();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadDbData]);

  // Set up global error handler
  useEffect(() => {
    setGlobalErrorHandler((error) => {
      // Show toast for user-facing errors
      toast.error(error.message);
      // Also set the app error state for persistent display
      setError(error.message);
    });
  }, [setError]);

  // Check for AI model migrations and notify user
  const { pendingModelMigration, clearPendingModelMigration } = useSettingsStore();
  useEffect(() => {
    if (pendingModelMigration) {
      const { from, to, provider } = pendingModelMigration;
      const providerName = provider === 'claude' ? 'Claude'
        : provider === 'openai' ? 'OpenAI'
        : provider === 'gemini' ? 'Gemini'
        : 'Perplexity';

      toast.info(
        `KI-Modell aktualisiert: ${from} → ${to} (${providerName}). Das alte Modell ist nicht mehr verfügbar.`
      );
      clearPendingModelMigration();
    }
  }, [pendingModelMigration, clearPendingModelMigration]);


  // ============================================================================
  // Import Handler
  // ============================================================================

  const handleImportPP = useCallback(async () => {
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

        // Invalidate all TanStack Query caches
        invalidateAllQueries();

        // Reload data
        await loadDbData();

        toast.success('Import erfolgreich abgeschlossen');
      }
    } catch (err) {
      setError(`Fehler beim Import: ${err}`);
    } finally {
      setLoading(false);
    }
  }, [loadDbData, setLoading, setError]);

  // ============================================================================
  // View Router
  // ============================================================================

  const renderView = () => {
    switch (currentView) {
      case 'dashboard':
        return (
          <DashboardView
            dbHoldings={dbHoldings}
            dbPortfolios={dbPortfolios}
            dbPortfolioHistory={dbPortfolioHistory}
            dbInvestedCapitalHistory={dbInvestedCapitalHistory}
            onImportPP={handleImportPP}
            onRefresh={loadDbData}
          />
        );
      case 'portfolio':
        return <PortfolioView dbPortfolios={dbPortfolios} />;
      case 'securities':
        return <SecuritiesView />;
      case 'accounts':
        return <AccountsView />;
      case 'transactions':
        return <TransactionsView />;
      case 'holdings':
        return <HoldingsView dbHoldings={dbHoldings} dbPortfolios={dbPortfolios} />;
      case 'dividends':
        return <DividendsView />;
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
      case 'screener':
        return <ScreenerView />;
      case 'benchmark':
        return <BenchmarkView />;
      case 'reports':
        return <ReportsView />;
      case 'settings':
        return <SettingsView />;
      default:
        return (
          <DashboardView
            dbHoldings={dbHoldings}
            dbPortfolios={dbPortfolios}
            dbPortfolioHistory={dbPortfolioHistory}
            dbInvestedCapitalHistory={dbInvestedCapitalHistory}
            onImportPP={handleImportPP}
            onRefresh={loadDbData}
          />
        );
    }
  };

  // ============================================================================
  // Render
  // ============================================================================

  return (
    <QueryClientProvider client={queryClient}>
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
            onImportPP={handleImportPP}
            onRefresh={loadDbData}
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

        {/* Chat interface */}
        <ChatButton onClick={() => setIsChatOpen(true)} />
        <ChatPanel isOpen={isChatOpen} onClose={() => setIsChatOpen(false)} />

        {/* Welcome modal for first-time users */}
        <WelcomeModal isOpen={showWelcome === true} onClose={handleWelcomeClose} />
      </div>
    </QueryClientProvider>
  );
}

export default App;
