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

// API
import { validateAllSecurities, fetchHistoricalPricesBatch, getSecurity, fetchSecurityLogo, saveLogoToCache, isLogoCached } from './lib/api';

// Store
import {
  useUIStore,
  useAppStore,
  useSettingsStore,
  AI_FEATURES,
  AI_MODELS,
  DEFAULT_MODELS,
  type AiFeatureId,
  type AiProvider,
} from './store';

// Layout components
import {
  Sidebar,
  Header,
  ErrorBanner,
  LoadingIndicator,
  ToastContainer,
  QuoteSyncOverlay,
} from './components/layout';

// Chat components
import { ChatButton, ChatPanel } from './components/chat';

// Modals
import { AiMigrationModal } from './components/modals';

// Onboarding
import { OnboardingWizard } from './components/onboarding';

// Secure Storage
import { useSecureApiKeys } from './hooks/useSecureApiKeys';

// Views
import {
  DashboardView,
  WidgetDashboardView,
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
  OptimizationView,
  ChartsView,
  ScreenerView,
  BenchmarkView,
  ReportsView,
  SettingsView,
  ConsortiumView,
} from './views';

// Types
import type { AggregatedHolding, PortfolioData } from './views';

// ============================================================================
// Main App Component
// ============================================================================

// Provider names for display
const PROVIDER_NAMES: Record<AiProvider, string> = {
  claude: 'Claude',
  openai: 'OpenAI',
  gemini: 'Gemini',
  perplexity: 'Perplexity',
  openrouter: 'OpenRouter',
};

function App() {
  const { currentView } = useUIStore();
  const { setLoading, setError } = useAppStore();
  const { theme, aiEnabled, aiFeatureSettings, setAiFeatureSetting, setPendingFeatureMigration, symbolValidation, setSymbolValidationSettings, setProfilePicture } = useSettingsStore();

  // Load API keys from secure storage on app start
  // This syncs secure storage with the Zustand store for component access
  // NOTE: The hook now stores keys in local state first (not just Zustand) to prevent
  // race conditions. When isLoading becomes false, keys are guaranteed to be available.
  const { keys: apiKeys, isLoading: apiKeysLoading } = useSecureApiKeys();

  // Load profile picture from database on app start
  useEffect(() => {
    const loadProfilePicture = async () => {
      try {
        const picture = await invoke<string | null>('get_user_profile_picture');
        setProfilePicture(picture);
      } catch (err) {
        console.error('Failed to load profile picture:', err);
      }
    };
    loadProfilePicture();
  }, [setProfilePicture]);

  // ============================================================================
  // AI Feature Provider Migration Check
  // ============================================================================

  // Check for AI features that need migration when API keys change
  useEffect(() => {
    // Wait for API keys to be loaded
    if (apiKeysLoading || !aiEnabled) return;

    // Determine which providers have API keys configured
    const availableProviders: AiProvider[] = [];
    if (apiKeys.anthropicApiKey?.trim()) availableProviders.push('claude');
    if (apiKeys.openaiApiKey?.trim()) availableProviders.push('openai');
    if (apiKeys.geminiApiKey?.trim()) availableProviders.push('gemini');
    if (apiKeys.perplexityApiKey?.trim()) availableProviders.push('perplexity');

    // No providers available - nothing to migrate to
    if (availableProviders.length === 0) return;

    // Check which features need migration (provider no longer available)
    const featuresToMigrate: { featureId: AiFeatureId; fromProvider: AiProvider }[] = [];

    AI_FEATURES.forEach((feature) => {
      const config = aiFeatureSettings[feature.id];
      if (config && !availableProviders.includes(config.provider)) {
        featuresToMigrate.push({
          featureId: feature.id,
          fromProvider: config.provider,
        });
      }
    });

    // No features need migration
    if (featuresToMigrate.length === 0) return;

    // Group features by their current (unavailable) provider
    const byProvider = featuresToMigrate.reduce((acc, item) => {
      if (!acc[item.fromProvider]) acc[item.fromProvider] = [];
      acc[item.fromProvider].push(item.featureId);
      return acc;
    }, {} as Record<AiProvider, AiFeatureId[]>);

    // Handle migration for each provider group
    Object.entries(byProvider).forEach(([fromProviderStr, features]) => {
      const fromProvider = fromProviderStr as AiProvider;

      if (availableProviders.length === 1) {
        // Only one provider available - auto-migrate silently
        const targetProvider = availableProviders[0];
        const models = AI_MODELS[targetProvider] || [];
        const defaultModel = models[0]?.id || DEFAULT_MODELS[targetProvider];

        features.forEach((featureId) => {
          setAiFeatureSetting(featureId, {
            provider: targetProvider,
            model: defaultModel,
          });
        });

        // Show toast notification
        const featureNames = features
          .map((id) => AI_FEATURES.find((f) => f.id === id)?.name || id)
          .join(', ');
        toast.info(
          `KI-Funktionen migriert: ${featureNames} von ${PROVIDER_NAMES[fromProvider]} zu ${PROVIDER_NAMES[targetProvider]}`
        );
      } else {
        // Multiple providers available - show migration dialog
        setPendingFeatureMigration({
          features,
          fromProvider,
          availableProviders,
        });
      }
    });
  }, [apiKeysLoading, apiKeys, aiEnabled, aiFeatureSettings, setAiFeatureSetting, setPendingFeatureMigration]);

  // ============================================================================
  // Auto Symbol Validation Check
  // ============================================================================

  useEffect(() => {
    // Wait for API keys to be loaded
    if (apiKeysLoading) return;

    const { autoValidateIntervalDays, lastAutoValidation, validateOnlyHeld, enableAiFallback } = symbolValidation;

    // Skip if auto-validation is disabled
    if (autoValidateIntervalDays === 0) return;

    // Calculate days since last validation
    const now = new Date();
    const lastValidation = lastAutoValidation ? new Date(lastAutoValidation) : null;
    const daysSinceLast = lastValidation
      ? Math.floor((now.getTime() - lastValidation.getTime()) / (1000 * 60 * 60 * 24))
      : Infinity;

    // Check if validation is due
    if (daysSinceLast < autoValidateIntervalDays) return;

    // Run background validation
    const runBackgroundValidation = async () => {
      try {
        // Collect API keys for validation
        const validationApiKeys = {
          coingeckoApiKey: apiKeys.coingeckoApiKey || undefined,
          finnhubApiKey: apiKeys.finnhubApiKey || undefined,
          alphaVantageApiKey: apiKeys.alphaVantageApiKey || undefined,
          twelveDataApiKey: apiKeys.twelveDataApiKey || undefined,
        };

        // Get AI config if enabled
        const aiConfig = enableAiFallback && aiEnabled ? {
          enabled: true,
          provider: aiFeatureSettings.portfolioInsights.provider,
          model: aiFeatureSettings.portfolioInsights.model,
          apiKey: (() => {
            switch (aiFeatureSettings.portfolioInsights.provider) {
              case 'claude': return apiKeys.anthropicApiKey || '';
              case 'openai': return apiKeys.openaiApiKey || '';
              case 'gemini': return apiKeys.geminiApiKey || '';
              case 'perplexity': return apiKeys.perplexityApiKey || '';
              default: return '';
            }
          })(),
        } : undefined;

        const result = await validateAllSecurities({
          onlyHeld: validateOnlyHeld,
          force: false,
          apiKeys: validationApiKeys,
          aiConfig: aiConfig,
        });

        // Update last validation timestamp
        setSymbolValidationSettings({ lastAutoValidation: now.toISOString() });

        // Notify user about results
        const summary = result.summary;
        if (summary && (summary.validated > 0 || summary.aiSuggested > 0 || summary.failed > 0)) {
          const messages: string[] = [];
          if (summary.validated > 0) messages.push(`${summary.validated} validiert`);
          if (summary.aiSuggested > 0) messages.push(`${summary.aiSuggested} KI-Vorschläge`);
          if (summary.failed > 0) messages.push(`${summary.failed} fehlgeschlagen`);
          toast.info(`Symbol-Validierung: ${messages.join(', ')}`);
        }
      } catch (err) {
        console.warn('Auto-validation failed:', err);
      }
    };

    // Run after a short delay to not block app startup
    const timer = setTimeout(runBackgroundValidation, 5000);
    return () => clearTimeout(timer);
  }, [apiKeysLoading, apiKeys, symbolValidation, aiEnabled, aiFeatureSettings, setSymbolValidationSettings]);

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

  // ============================================================================
  // Global Drag & Drop Prevention
  // ============================================================================
  // Prevents files from being opened by the browser/Tauri when dropped on
  // areas without specific D&D handlers (e.g., sidebar, dashboard, etc.)
  // NOTE: Only preventDefault, NOT stopPropagation - stopPropagation breaks Tauri's native D&D
  useEffect(() => {
    const preventDefaultDrop = (e: DragEvent) => {
      // Only preventDefault to stop browser from opening files
      // Do NOT stopPropagation - it breaks Tauri's onDragDropEvent
      e.preventDefault();
    };

    // Register global handlers on document level
    document.addEventListener('dragover', preventDefaultDrop);
    document.addEventListener('drop', preventDefaultDrop);

    return () => {
      document.removeEventListener('dragover', preventDefaultDrop);
      document.removeEventListener('drop', preventDefaultDrop);
    };
  }, []);

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

      // Load all data sources in parallel for faster startup
      const [portfolios, holdings, history, investedHistory] = await Promise.all([
        invoke<PortfolioData[]>('get_pp_portfolios', { importId: null }),
        invoke<AggregatedHolding[]>('get_all_holdings'),
        invoke<Array<{ date: string; value: number }>>('get_portfolio_history').catch((err) => {
          console.warn('Could not load portfolio history:', err);
          return [] as Array<{ date: string; value: number }>;
        }),
        invoke<Array<{ date: string; value: number }>>('get_invested_capital_history').catch((err) => {
          console.warn('Could not load invested capital history:', err);
          return [] as Array<{ date: string; value: number }>;
        }),
      ]);

      setDbPortfolios(portfolios);
      setDbHoldings(holdings);
      setDbPortfolioHistory(history);
      setDbInvestedCapitalHistory(investedHistory);
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

  // Sync historical exchange rates on app startup (for correct portfolio valuation)
  useEffect(() => {
    const syncExchangeRates = async () => {
      try {
        const result = await invoke<{
          fromDate: string;
          toDate: string;
          fetchedCount: number;
          addedCount: number;
          totalCount: number;
        }>('sync_historical_exchange_rates');

        if (result.addedCount > 0) {
          console.log(
            `Exchange rates synced: ${result.addedCount} new rates added (total: ${result.totalCount})`
          );
          // Reload data to recalculate portfolio values with correct FX rates
          loadDbData();
          invalidateAllQueries();
        }
      } catch (err) {
        console.warn('Failed to sync exchange rates:', err);
      }
    };

    // Run after a short delay to not block initial app load
    const timer = setTimeout(syncExchangeRates, 2000);
    return () => clearTimeout(timer);
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

  // Auto-fetch historical prices and logo when a security is added to a watchlist
  useEffect(() => {
    if (apiKeysLoading) return;

    const unlisten = listen<number>('watchlist-security-added', (event) => {
      const securityId = event.payload;
      console.log('Watchlist security added, fetching historical prices for security:', securityId);

      const quoteApiKeys = {
        finnhub: apiKeys.finnhubApiKey || undefined,
        alphaVantage: apiKeys.alphaVantageApiKey || undefined,
        coingecko: apiKeys.coingeckoApiKey || undefined,
        twelveData: apiKeys.twelveDataApiKey || undefined,
      };

      // Fetch historical prices
      fetchHistoricalPricesBatch(
        { securityIds: [securityId], fromYear: 2000, onlyHeld: false, force: false },
        quoteApiKeys
      )
        .then((result) => {
          console.log('Historical prices fetched for watchlist security:', securityId, result);
        })
        .catch((err) => {
          console.warn('Failed to fetch historical prices for watchlist security:', securityId, err);
        });

      // Pre-cache logo
      const brandfetchKey = apiKeys.brandfetchApiKey;
      if (brandfetchKey) {
        getSecurity(securityId)
          .then(async (sec) => {
            const logoResult = await fetchSecurityLogo(brandfetchKey, securityId, sec.ticker, sec.name);
            if (logoResult.domain && logoResult.logoUrl) {
              const cached = await isLogoCached(logoResult.domain);
              if (!cached) {
                try {
                  const response = await fetch(logoResult.logoUrl);
                  if (response.ok) {
                    const blob = await response.blob();
                    const reader = new FileReader();
                    reader.onload = async () => {
                      await saveLogoToCache(logoResult.domain!, reader.result as string);
                      console.log('Logo cached for watchlist security:', sec.name);
                    };
                    reader.readAsDataURL(blob);
                  }
                } catch {
                  // Logo caching is best-effort
                }
              }
            }
          })
          .catch((err) => {
            console.warn('Failed to pre-cache logo for watchlist security:', securityId, err);
          });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [apiKeysLoading, apiKeys]);

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
      case 'widget-dashboard':
        return (
          <WidgetDashboardView
            dbHoldings={dbHoldings}
            dbPortfolioHistory={dbPortfolioHistory}
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
      case 'optimization':
        return <OptimizationView />;
      case 'charts':
        return <ChartsView />;
      case 'screener':
        return <ScreenerView />;
      case 'benchmark':
        return <BenchmarkView />;
      case 'reports':
        return <ReportsView />;
      case 'consortium':
        return <ConsortiumView />;
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
  // ============================================================================
  // App Mode
  // ============================================================================

  const appMode = useAppStore((s) => s.appMode);
  const setCurrentView = useUIStore((s) => s.setCurrentView);

  // Validate currentView matches appMode on startup
  useEffect(() => {
    if (appMode === 'analysis') {
      const analysisViews = ['watchlist', 'charts', 'screener', 'settings'];
      if (!analysisViews.includes(currentView)) {
        setCurrentView('watchlist');
      }
    } else if (appMode === 'portfolio') {
      const portfolioViews = ['dashboard', 'widget-dashboard', 'portfolio', 'accounts', 'transactions', 'holdings', 'dividends', 'asset-statement', 'taxonomies', 'optimization', 'watchlist', 'securities', 'charts', 'screener', 'settings', 'plans', 'reports', 'rebalancing', 'benchmark', 'consortium'];
      if (!portfolioViews.includes(currentView)) {
        setCurrentView('dashboard');
      }
    }
  }, [appMode]); // eslint-disable-line react-hooks/exhaustive-deps

  // Render
  // ============================================================================

  // Onboarding wizard (first launch)
  if (appMode === null) {
    return (
      <QueryClientProvider client={queryClient}>
        <OnboardingWizard />
        <ToastContainer />
      </QueryClientProvider>
    );
  }

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
        <main id="main-content" className="flex-1 flex flex-col overflow-hidden min-w-0">
          {/* Header */}
          <Header
            onImportPP={handleImportPP}
            onRefresh={loadDbData}
            onOpenChat={() => setIsChatOpen(true)}
          />

          {/* Error Banner */}
          <ErrorBanner />

          {/* Loading Indicator */}
          <LoadingIndicator />

          {/* Content Area */}
          <div key={currentView} className="flex-1 overflow-auto px-5 py-4 animate-fade-in">
            {renderView()}
          </div>
        </main>

        {/* Chat interface - flex flow for Charts, overlay for other views */}
        {aiEnabled && (
          <>
            {!isChatOpen && <ChatButton onClick={() => setIsChatOpen(true)} />}
            {isChatOpen && (
              <ChatPanel isOpen={isChatOpen} onClose={() => setIsChatOpen(false)} overlay={currentView !== 'charts'} />
            )}
          </>
        )}

        {/* Quote sync progress overlay */}
        <QuoteSyncOverlay />

        {/* Toast notifications */}
        <ToastContainer />

        {/* AI feature migration modal - shown when API keys are removed */}
        <AiMigrationModal />
      </div>
    </QueryClientProvider>
  );
}

export default App;
