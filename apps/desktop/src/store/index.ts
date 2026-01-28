/**
 * Global application state using Zustand.
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// Types
// ============================================================================

export type View =
  | 'dashboard'
  | 'widget-dashboard'
  | 'portfolio'
  | 'securities'
  | 'accounts'
  | 'transactions'
  | 'holdings'
  | 'dividends'
  | 'asset-statement'
  | 'watchlist'
  | 'taxonomies'
  | 'plans'
  | 'rebalancing'
  | 'optimization'
  | 'charts'
  | 'screener'
  | 'benchmark'
  | 'reports'
  | 'consortium'
  | 'settings';

export interface NavItem {
  id: View;
  label: string;
  icon: string; // Icon name from lucide-react
  section?: 'main' | 'analysis' | 'tools'; // For grouping in sidebar
}

export const navItems: NavItem[] = [
  // Main section
  { id: 'dashboard', label: 'Dashboard', icon: 'LayoutDashboard', section: 'main' },
  { id: 'widget-dashboard', label: 'Mein Dashboard', icon: 'LayoutGrid', section: 'main' },
  { id: 'portfolio', label: 'Portfolios', icon: 'Briefcase', section: 'main' },
  { id: 'securities', label: 'Wertpapiere', icon: 'TrendingUp', section: 'main' },
  { id: 'accounts', label: 'Konten', icon: 'Wallet', section: 'main' },
  { id: 'transactions', label: 'Buchungen', icon: 'ArrowRightLeft', section: 'main' },
  { id: 'holdings', label: 'Bestand', icon: 'PieChart', section: 'main' },
  { id: 'dividends', label: 'Dividenden', icon: 'Coins', section: 'main' },
  { id: 'watchlist', label: 'Watchlist', icon: 'Eye', section: 'main' },
  // Analysis section
  { id: 'asset-statement', label: 'Vermögensaufstellung', icon: 'Table2', section: 'analysis' },
  { id: 'taxonomies', label: 'Klassifizierung', icon: 'FolderTree', section: 'analysis' },
  // --- HIDDEN FOR v0.1.0 RELEASE (see RELEASE_NOTES.md) ---
  // { id: 'benchmark', label: 'Benchmark', icon: 'Target', section: 'analysis' },
  // { id: 'consortium', label: 'Portfolio-Gruppen', icon: 'FolderKanban', section: 'analysis' },
  // { id: 'reports', label: 'Berichte', icon: 'BarChart3', section: 'analysis' },
  // Tools section
  { id: 'optimization', label: 'Optimierung', icon: 'Sparkles', section: 'tools' },
  { id: 'charts', label: 'Technische Analyse', icon: 'CandlestickChart', section: 'tools' },
  // --- HIDDEN FOR v0.1.0 RELEASE (see RELEASE_NOTES.md) ---
  // { id: 'screener', label: 'Screener', icon: 'Search', section: 'tools' },
  // { id: 'plans', label: 'Sparpläne', icon: 'CalendarClock', section: 'tools' },
  // { id: 'rebalancing', label: 'Rebalancing', icon: 'Scale', section: 'tools' },
];

// ============================================================================
// UI State
// ============================================================================

interface UIState {
  currentView: View;
  sidebarCollapsed: boolean;
  scrollTarget: string | null;
  // PDF Import Modal state (global for cross-component access)
  pdfImportModalOpen: boolean;
  pdfImportInitialPath: string | null;
  setCurrentView: (view: View) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setScrollTarget: (target: string | null) => void;
  openPdfImportModal: (path?: string) => void;
  closePdfImportModal: () => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      currentView: 'dashboard',
      sidebarCollapsed: false,
      scrollTarget: null,
      pdfImportModalOpen: false,
      pdfImportInitialPath: null,
      setCurrentView: (view) => set({ currentView: view }),
      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
      setScrollTarget: (target) => set({ scrollTarget: target }),
      openPdfImportModal: (path) => set({ pdfImportModalOpen: true, pdfImportInitialPath: path || null }),
      closePdfImportModal: () => set({ pdfImportModalOpen: false, pdfImportInitialPath: null }),
    }),
    {
      name: 'portfolio-ui-state',
      partialize: (state) => ({ sidebarCollapsed: state.sidebarCollapsed }),
    }
  )
);

// ============================================================================
// App State (Loading, Errors, etc.)
// ============================================================================

interface AppState {
  isLoading: boolean;
  error: string | null;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
}

export const useAppStore = create<AppState>()((set) => ({
  isLoading: false,
  error: null,
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error }),
  clearError: () => set({ error: null }),
}));

// ============================================================================
// Data Refresh Trigger (for global data refresh after mutations)
// ============================================================================
// Verwende triggerDataRefresh() nach jeder Transaktion/Import um alle Views zu aktualisieren

interface DataRefreshState {
  refreshVersion: number;
  triggerDataRefresh: () => void;
}

export const useDataRefreshStore = create<DataRefreshState>()((set) => ({
  refreshVersion: 0,
  triggerDataRefresh: () => set((state) => ({ refreshVersion: state.refreshVersion + 1 })),
}));

// ============================================================================
// Expanded Groups State (for UI drill-down)
// ============================================================================

interface ExpandedGroupsState {
  expandedGroups: Set<string>;
  toggleGroup: (key: string) => void;
  expandGroup: (key: string) => void;
  collapseGroup: (key: string) => void;
  clearExpanded: () => void;
}

export const useExpandedGroupsStore = create<ExpandedGroupsState>()((set) => ({
  expandedGroups: new Set<string>(),
  toggleGroup: (key) =>
    set((state) => {
      const next = new Set(state.expandedGroups);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return { expandedGroups: next };
    }),
  expandGroup: (key) =>
    set((state) => {
      const next = new Set(state.expandedGroups);
      next.add(key);
      return { expandedGroups: next };
    }),
  collapseGroup: (key) =>
    set((state) => {
      const next = new Set(state.expandedGroups);
      next.delete(key);
      return { expandedGroups: next };
    }),
  clearExpanded: () => set({ expandedGroups: new Set() }),
}));

// ============================================================================
// Settings State
// ============================================================================

// Auto-update interval options (in minutes, 0 = disabled)
export type AutoUpdateInterval = 0 | 15 | 30 | 60;

// Chart time range options
export type ChartTimeRange = '1W' | '1M' | '3M' | '6M' | 'YTD' | '1Y' | '3Y' | '5Y' | 'MAX';

// AI Model options per provider (updated January 2026)
// ONLY models with confirmed vision/image input support
export type ClaudeModel = 'claude-sonnet-4-5-20250514' | 'claude-haiku-4-5-20251015';
export type OpenAIModel = 'gpt-5-mini' | 'gpt-4.1' | 'gpt-4o' | 'gpt-4o-mini';
export type GeminiModel = 'gemini-2.5-flash' | 'gemini-2.5-pro' | 'gemini-3-flash-preview' | 'gemini-3-pro-preview';

// AI Feature Types for individual configuration
export type AiFeatureId = 'chartAnalysis' | 'portfolioInsights' | 'chatAssistant' | 'pdfOcr' | 'csvImport' | 'quoteAssistant';
export type AiProvider = 'claude' | 'openai' | 'gemini' | 'perplexity';

export interface AiFeatureConfig {
  provider: AiProvider;
  model: string;
}

// Default model per provider
export const DEFAULT_MODELS: Record<AiProvider, string> = {
  claude: 'claude-sonnet-4-5-20250514',
  openai: 'gpt-4o',
  gemini: 'gemini-2.5-flash',
  perplexity: 'sonar-pro',
};

// Feature definitions with metadata
export interface AiFeatureDefinition {
  id: AiFeatureId;
  name: string;
  description: string;
  icon: string;
  requiresVision: boolean;
}

export const AI_FEATURES: AiFeatureDefinition[] = [
  { id: 'chartAnalysis', name: 'Chart-Analyse', description: 'Technische Analyse von Chart-Bildern mit KI', icon: 'BarChart3', requiresVision: true },
  { id: 'portfolioInsights', name: 'Portfolio Insights', description: 'Analyse von Stärken, Risiken und Empfehlungen', icon: 'Lightbulb', requiresVision: false },
  { id: 'chatAssistant', name: 'Chat-Assistent', description: 'Fragen zu deinem Portfolio beantworten', icon: 'MessageSquare', requiresVision: false },
  { id: 'pdfOcr', name: 'PDF OCR', description: 'Text aus gescannten Bank-PDFs extrahieren', icon: 'FileText', requiresVision: true },
  { id: 'csvImport', name: 'CSV-Import', description: 'Unbekannte Broker-Formate analysieren', icon: 'FileSpreadsheet', requiresVision: false },
  { id: 'quoteAssistant', name: 'Kursquellen-Assistent', description: 'Optimale Kursquellen mit Web-Suche finden', icon: 'Bot', requiresVision: false },
];

// AI Models - Updated January 2026 (Vision-only)
// Sources:
// - Claude: https://platform.claude.com/docs/en/about-claude/models/overview
// - OpenAI: https://platform.openai.com/docs/models
// - Gemini: https://ai.google.dev/gemini-api/docs/gemini-3
// - Perplexity: https://docs.perplexity.ai/guides/model-cards
export const AI_MODELS = {
  claude: [
    // Note: Opus 4.5 has NO vision support in API yet
    { id: 'claude-sonnet-4-5-20250514', name: 'Claude Sonnet 4.5', description: 'Beste Qualität mit Vision' },
    { id: 'claude-haiku-4-5-20251015', name: 'Claude Haiku 4.5', description: 'Schnell & günstig' },
  ],
  openai: [
    { id: 'gpt-5-mini', name: 'GPT-5 Mini', description: 'Neuestes GPT-5, schnell & günstig' },
    { id: 'gpt-4.1', name: 'GPT-4.1', description: '1M Kontext' },
    { id: 'gpt-4o', name: 'GPT-4o', description: 'Flagship Multimodal' },
    { id: 'gpt-4o-mini', name: 'GPT-4o Mini', description: 'Schnell & günstig' },
  ],
  gemini: [
    { id: 'gemini-2.5-flash', name: 'Gemini 2.5 Flash', description: 'Schnell & günstig, Free Tier' },
    { id: 'gemini-2.5-pro', name: 'Gemini 2.5 Pro', description: 'Beste Qualität (stabil)' },
    { id: 'gemini-3-flash-preview', name: 'Gemini 3 Flash', description: 'Neuestes Modell (Preview)' },
    { id: 'gemini-3-pro-preview', name: 'Gemini 3 Pro', description: 'Neuestes Pro (Preview)' },
  ],
  perplexity: [
    { id: 'sonar-pro', name: 'Sonar Pro', description: 'Vision + Web-Suche' },
    { id: 'sonar', name: 'Sonar', description: 'Schnell + Web-Suche' },
  ],
} as const;

// Vision model type from backend
export interface VisionModel {
  id: string;
  name: string;
  description: string;
}

// Cache for vision models loaded from backend
const visionModelCache: Record<string, VisionModel[]> = {};

/**
 * Get vision-capable models for a provider from the backend registry.
 * Uses caching to avoid repeated API calls.
 * Falls back to AI_MODELS if backend is unavailable.
 */
export async function getVisionModels(provider: string): Promise<VisionModel[]> {
  // Return cached result if available
  if (visionModelCache[provider]) {
    return visionModelCache[provider];
  }

  try {
    const models = await invoke<VisionModel[]>('get_vision_models', { provider });
    visionModelCache[provider] = models;
    return models;
  } catch (error) {
    console.warn(`Failed to load vision models from backend for ${provider}, using fallback:`, error);
    // Fallback to static AI_MODELS
    const fallback = AI_MODELS[provider as keyof typeof AI_MODELS];
    return fallback ? [...fallback] : [];
  }
}

/**
 * Clear the vision model cache (e.g., after settings change).
 */
export function clearVisionModelCache(): void {
  Object.keys(visionModelCache).forEach(key => delete visionModelCache[key]);
}

// Deprecated model mappings - auto-upgrade to replacements (January 2026)
const DEPRECATED_MODELS: Record<string, Record<string, string>> = {
  perplexity: {
    'sonar-reasoning': 'sonar-pro',
    'sonar-reasoning-pro': 'sonar-pro',
    'sonar-deep-research': 'sonar-pro',
  },
  claude: {
    // Opus 4.5 has no vision - map to Sonnet 4.5
    'claude-opus-4-5-20251101': 'claude-sonnet-4-5-20250514',
    'claude-3-opus-20240229': 'claude-sonnet-4-5-20250514',
    'claude-3-sonnet-20240229': 'claude-sonnet-4-5-20250514',
    'claude-3-haiku-20240307': 'claude-haiku-4-5-20251015',
    'claude-3-5-sonnet-20241022': 'claude-sonnet-4-5-20250514',
    'claude-3-5-haiku-20241022': 'claude-haiku-4-5-20251015',
    'claude-3-7-sonnet': 'claude-sonnet-4-5-20250514',
    'claude-2.1': 'claude-sonnet-4-5-20250514',
  },
  openai: {
    // o-series has no vision - map to vision models
    'o3': 'gpt-4.1',
    'o3-pro': 'gpt-4.1',
    'o4-mini': 'gpt-4o-mini',
    'o1': 'gpt-4o',
    'o1-preview': 'gpt-4o',
    'o1-mini': 'gpt-4o-mini',
    // Old models
    'gpt-4-vision-preview': 'gpt-4o',
    'gpt-4-turbo': 'gpt-4.1',
    'gpt-4-turbo-preview': 'gpt-4o',
  },
  gemini: {
    // Old models map to stable 2.5 versions
    'gemini-pro-vision': 'gemini-2.5-flash',
    'gemini-2.0-flash': 'gemini-2.5-flash',
    'gemini-2.0-flash-exp': 'gemini-2.5-flash',
    'gemini-1.5-pro': 'gemini-2.5-pro',
    'gemini-1.5-flash': 'gemini-2.5-flash',
    // Invalid model names (without -preview suffix)
    'gemini-3-flash': 'gemini-2.5-flash',
    'gemini-3-pro': 'gemini-2.5-pro',
  },
};

// Get upgraded model if deprecated
function getUpgradedModel(provider: string, model: string): string | null {
  return DEPRECATED_MODELS[provider]?.[model] || null;
}

interface SettingsState {
  // User profile
  userName: string;
  setUserName: (name: string) => void;
  profilePicture: string | null;
  setProfilePicture: (picture: string | null) => void;

  // Quote sync settings
  syncOnlyHeldSecurities: boolean;
  setSyncOnlyHeldSecurities: (value: boolean) => void;
  autoUpdateInterval: AutoUpdateInterval;
  setAutoUpdateInterval: (interval: AutoUpdateInterval) => void;
  lastSyncTime: string | null; // ISO string for persistence
  setLastSyncTime: (time: Date | null) => void;

  // Transaction settings
  deliveryMode: boolean; // When true: Buy→Delivery, Dividend→with withdrawal
  setDeliveryMode: (value: boolean) => void;

  // Display settings
  language: 'de' | 'en';
  theme: 'light' | 'dark' | 'system';
  baseCurrency: string;
  defaultChartTimeRange: ChartTimeRange;
  setLanguage: (lang: 'de' | 'en') => void;
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  setBaseCurrency: (currency: string) => void;
  setDefaultChartTimeRange: (range: ChartTimeRange) => void;

  // API Keys (Quote Providers)
  brandfetchApiKey: string;
  setBrandfetchApiKey: (key: string) => void;
  finnhubApiKey: string;
  setFinnhubApiKey: (key: string) => void;
  coingeckoApiKey: string;
  setCoingeckoApiKey: (key: string) => void;
  alphaVantageApiKey: string;
  setAlphaVantageApiKey: (key: string) => void;
  twelveDataApiKey: string;
  setTwelveDataApiKey: (key: string) => void;

  // AI Analysis Settings
  aiEnabled: boolean; // Global toggle to disable all AI features (keeps API keys)
  setAiEnabled: (enabled: boolean) => void;
  aiProvider: 'claude' | 'openai' | 'gemini' | 'perplexity';
  setAiProvider: (provider: 'claude' | 'openai' | 'gemini' | 'perplexity') => void;
  aiModel: string;
  setAiModel: (model: string) => void;
  anthropicApiKey: string;
  setAnthropicApiKey: (key: string) => void;
  openaiApiKey: string;
  setOpenaiApiKey: (key: string) => void;
  geminiApiKey: string;
  setGeminiApiKey: (key: string) => void;
  perplexityApiKey: string;
  setPerplexityApiKey: (key: string) => void;

  // External Services API Keys
  divvyDiaryApiKey: string;
  setDivvyDiaryApiKey: (key: string) => void;

  // Model migration tracking (not persisted)
  pendingModelMigration: { from: string; to: string; provider: string } | null;
  clearPendingModelMigration: () => void;

  // AI Feature-specific settings (per-feature provider/model configuration)
  aiFeatureSettings: Record<AiFeatureId, AiFeatureConfig>;
  setAiFeatureSetting: (featureId: AiFeatureId, config: AiFeatureConfig) => void;

  // Feature provider migration tracking (not persisted)
  // Used when an API key is removed and features need to migrate to another provider
  pendingFeatureMigration: {
    features: AiFeatureId[];
    fromProvider: AiProvider;
    availableProviders: AiProvider[];
  } | null;
  setPendingFeatureMigration: (migration: {
    features: AiFeatureId[];
    fromProvider: AiProvider;
    availableProviders: AiProvider[];
  } | null) => void;
  clearPendingFeatureMigration: () => void;

  // Symbol Validation Settings
  symbolValidation: {
    autoValidateIntervalDays: 0 | 7 | 14 | 30; // 0 = disabled
    lastAutoValidation: string | null; // ISO date string
    validateOnlyHeld: boolean;
    enableAiFallback: boolean;
  };
  setSymbolValidationSettings: (settings: Partial<SettingsState['symbolValidation']>) => void;

  // Chat Context Settings
  chatContextSize: number; // Number of messages to send to AI (sliding window)
  setChatContextSize: (size: number) => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      // User profile
      userName: '',
      setUserName: (name) => set({ userName: name }),
      profilePicture: null,
      setProfilePicture: (picture) => set({ profilePicture: picture }),

      // Quote sync - default to only held securities
      syncOnlyHeldSecurities: true,
      setSyncOnlyHeldSecurities: (value) => set({ syncOnlyHeldSecurities: value }),
      autoUpdateInterval: 0, // Disabled by default
      setAutoUpdateInterval: (interval) => set({ autoUpdateInterval: interval }),
      lastSyncTime: null,
      setLastSyncTime: (time) => set({ lastSyncTime: time ? time.toISOString() : null }),

      // Transaction settings - default to normal mode (Buy with cash movement)
      deliveryMode: false,
      setDeliveryMode: (value) => set({ deliveryMode: value }),

      // Display settings
      language: 'de',
      theme: 'system',
      baseCurrency: 'EUR',
      defaultChartTimeRange: 'MAX',
      setLanguage: (lang) => set({ language: lang }),
      setTheme: (theme) => set({ theme: theme }),
      setBaseCurrency: (currency) => set({ baseCurrency: currency }),
      setDefaultChartTimeRange: (range) => set({ defaultChartTimeRange: range }),

      // API Keys (Quote Providers)
      brandfetchApiKey: '',
      setBrandfetchApiKey: (key) => set({ brandfetchApiKey: key }),
      finnhubApiKey: '',
      setFinnhubApiKey: (key) => set({ finnhubApiKey: key }),
      coingeckoApiKey: '',
      setCoingeckoApiKey: (key) => set({ coingeckoApiKey: key }),
      alphaVantageApiKey: '',
      setAlphaVantageApiKey: (key) => set({ alphaVantageApiKey: key }),
      twelveDataApiKey: '',
      setTwelveDataApiKey: (key) => set({ twelveDataApiKey: key }),

      // AI Analysis Settings (Vision-only models, January 2026)
      aiEnabled: true, // Default: AI is enabled
      setAiEnabled: (enabled) => set({ aiEnabled: enabled }),
      aiProvider: 'claude',
      setAiProvider: (provider) => set({
        aiProvider: provider,
        // Reset model to default for new provider
        aiModel: provider === 'claude' ? 'claude-sonnet-4-5-20250514'
          : provider === 'openai' ? 'gpt-5-mini'
          : provider === 'gemini' ? 'gemini-2.5-flash'
          : 'sonar-pro',
      }),
      aiModel: 'claude-sonnet-4-5-20250514',
      setAiModel: (model) => set({ aiModel: model }),
      anthropicApiKey: '',
      setAnthropicApiKey: (key) => set({ anthropicApiKey: key }),
      openaiApiKey: '',
      setOpenaiApiKey: (key) => set({ openaiApiKey: key }),
      geminiApiKey: '',
      setGeminiApiKey: (key) => set({ geminiApiKey: key }),
      perplexityApiKey: '',
      setPerplexityApiKey: (key) => set({ perplexityApiKey: key }),

      // External Services API Keys
      divvyDiaryApiKey: '',
      setDivvyDiaryApiKey: (key) => set({ divvyDiaryApiKey: key }),

      // Model migration tracking (transient, not persisted)
      pendingModelMigration: null,
      clearPendingModelMigration: () => set({ pendingModelMigration: null }),

      // AI Feature-specific settings - default all to global aiProvider/aiModel
      aiFeatureSettings: {
        chartAnalysis: { provider: 'claude', model: 'claude-sonnet-4-5-20250514' },
        portfolioInsights: { provider: 'claude', model: 'claude-sonnet-4-5-20250514' },
        chatAssistant: { provider: 'claude', model: 'claude-sonnet-4-5-20250514' },
        pdfOcr: { provider: 'claude', model: 'claude-sonnet-4-5-20250514' },
        csvImport: { provider: 'claude', model: 'claude-sonnet-4-5-20250514' },
        quoteAssistant: { provider: 'perplexity', model: 'sonar-pro' }, // Web-Suche für aktuelle Ticker
      },
      setAiFeatureSetting: (featureId, config) => set((state) => ({
        aiFeatureSettings: {
          ...state.aiFeatureSettings,
          [featureId]: config,
        },
      })),

      // Feature provider migration tracking (transient, not persisted)
      pendingFeatureMigration: null,
      setPendingFeatureMigration: (migration) => set({ pendingFeatureMigration: migration }),
      clearPendingFeatureMigration: () => set({ pendingFeatureMigration: null }),

      // Symbol Validation Settings
      symbolValidation: {
        autoValidateIntervalDays: 0, // Disabled by default
        lastAutoValidation: null,
        validateOnlyHeld: true,
        enableAiFallback: true,
      },
      setSymbolValidationSettings: (settings) => set((state) => ({
        symbolValidation: { ...state.symbolValidation, ...settings },
      })),

      // Chat Context Settings
      chatContextSize: 20, // Default: 20 messages in context window
      setChatContextSize: (size) => set({ chatContextSize: size }),
    }),
    {
      name: 'portfolio-settings',
      version: 8, // v8: Added quoteAssistant to aiFeatureSettings
      migrate: (persistedState, version) => {
        const state = persistedState as Partial<SettingsState>;

        // Migration v4: Add userName if not present
        if (version < 4) {
          if (!state.userName) {
            state.userName = '';
          }
        }

        // Migration: validate AI models (revalidate on each version bump)
        if (version < 4) {
          const provider = state.aiProvider || 'claude';
          const validModels = AI_MODELS[provider].map(m => m.id) as string[];

          // Reset to default if current model is not valid for the provider
          if (state.aiModel && !validModels.includes(state.aiModel)) {
            state.aiModel = AI_MODELS[provider][0].id;
          }
        }

        // Migration v5: Initialize aiFeatureSettings from global aiProvider/aiModel
        if (version < 5) {
          const globalProvider = (state.aiProvider || 'claude') as AiProvider;
          const globalModel = state.aiModel || DEFAULT_MODELS[globalProvider];

          state.aiFeatureSettings = {
            chartAnalysis: { provider: globalProvider, model: globalModel },
            portfolioInsights: { provider: globalProvider, model: globalModel },
            chatAssistant: { provider: globalProvider, model: globalModel },
            pdfOcr: { provider: globalProvider, model: globalModel },
            csvImport: { provider: globalProvider, model: globalModel },
            quoteAssistant: { provider: 'perplexity' as AiProvider, model: 'sonar-pro' },
          };
        }

        // Migration v6: Initialize symbolValidation settings
        if (version < 6) {
          state.symbolValidation = {
            autoValidateIntervalDays: 0,
            lastAutoValidation: null,
            validateOnlyHeld: true,
            enableAiFallback: true,
          };
        }

        // Migration v7: Initialize chatContextSize
        if (version < 7) {
          state.chatContextSize = 20; // Default: 20 messages
        }

        // Migration v8: Add quoteAssistant to aiFeatureSettings (for existing users)
        if (version < 8) {
          if (state.aiFeatureSettings && !state.aiFeatureSettings.quoteAssistant) {
            state.aiFeatureSettings.quoteAssistant = {
              provider: 'perplexity' as AiProvider,
              model: 'sonar-pro',
            };
          }
        }

        return state as SettingsState;
      },
      merge: (persistedState, currentState) => {
        const merged = {
          ...currentState,
          ...(persistedState as Partial<SettingsState>),
        };

        // Auto-upgrade deprecated models
        const provider = merged.aiProvider || 'claude';
        const originalModel = merged.aiModel;
        const upgradedModel = getUpgradedModel(provider, merged.aiModel);

        if (upgradedModel) {
          console.log(`Auto-upgrading deprecated model ${merged.aiModel} to ${upgradedModel}`);
          merged.aiModel = upgradedModel;
          // Track migration for notification
          merged.pendingModelMigration = {
            from: originalModel,
            to: upgradedModel,
            provider,
          };
        }

        // Validate model exists in current list
        const validModels = AI_MODELS[provider].map(m => m.id) as string[];
        if (!validModels.includes(merged.aiModel)) {
          const defaultModel = AI_MODELS[provider][0].id;
          // Track migration if model was invalid
          if (!merged.pendingModelMigration && originalModel !== defaultModel) {
            merged.pendingModelMigration = {
              from: originalModel,
              to: defaultModel,
              provider,
            };
          }
          merged.aiModel = defaultModel;
        }

        return merged;
      },
      // Exclude transient state and API keys from persistence
      // API keys are stored securely via tauri-plugin-store (see useSecureApiKeys hook)
      // Profile picture is stored in the database
      partialize: (state) => {
        const {
          // Transient state
          pendingModelMigration,
          clearPendingModelMigration,
          pendingFeatureMigration,
          setPendingFeatureMigration,
          clearPendingFeatureMigration,
          // Profile picture (stored in database, not localStorage)
          profilePicture,
          setProfilePicture,
          // API keys (stored in Secure Storage, not localStorage)
          brandfetchApiKey,
          setBrandfetchApiKey,
          finnhubApiKey,
          setFinnhubApiKey,
          coingeckoApiKey,
          setCoingeckoApiKey,
          alphaVantageApiKey,
          setAlphaVantageApiKey,
          twelveDataApiKey,
          setTwelveDataApiKey,
          anthropicApiKey,
          setAnthropicApiKey,
          openaiApiKey,
          setOpenaiApiKey,
          geminiApiKey,
          setGeminiApiKey,
          perplexityApiKey,
          setPerplexityApiKey,
          divvyDiaryApiKey,
          setDivvyDiaryApiKey,
          // Keep the rest
          ...persisted
        } = state;
        // Suppress unused variable warnings
        void profilePicture; void setProfilePicture;
        void brandfetchApiKey; void setBrandfetchApiKey;
        void finnhubApiKey; void setFinnhubApiKey;
        void coingeckoApiKey; void setCoingeckoApiKey;
        void alphaVantageApiKey; void setAlphaVantageApiKey;
        void twelveDataApiKey; void setTwelveDataApiKey;
        void anthropicApiKey; void setAnthropicApiKey;
        void openaiApiKey; void setOpenaiApiKey;
        void geminiApiKey; void setGeminiApiKey;
        void perplexityApiKey; void setPerplexityApiKey;
        void divvyDiaryApiKey; void setDivvyDiaryApiKey;
        void pendingModelMigration; void clearPendingModelMigration;
        void pendingFeatureMigration; void setPendingFeatureMigration; void clearPendingFeatureMigration;
        return persisted as SettingsState;
      },
    }
  )
);

// ============================================================================
// Toast State
// ============================================================================

export type ToastType = 'success' | 'error' | 'info' | 'warning';

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration: number;
}

interface ToastState {
  toasts: Toast[];
  addToast: (type: ToastType, message: string, duration?: number) => string;
  removeToast: (id: string) => void;
  clearAllToasts: () => void;
}

export const useToastStore = create<ToastState>()((set, get) => ({
  toasts: [],
  addToast: (type, message, duration = 4000) => {
    const id = `toast-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const newToast: Toast = { id, type, message, duration };

    set((state) => ({
      // Keep max 3 toasts
      toasts: [...state.toasts.slice(-2), newToast],
    }));

    // Auto-dismiss
    if (duration > 0) {
      setTimeout(() => {
        get().removeToast(id);
      }, duration);
    }

    return id;
  },
  removeToast: (id) =>
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    })),
  clearAllToasts: () => set({ toasts: [] }),
}));

/**
 * Convenience functions for showing toasts.
 */
export const toast = {
  success: (message: string, duration?: number) =>
    useToastStore.getState().addToast('success', message, duration),
  error: (message: string, duration?: number) =>
    useToastStore.getState().addToast('error', message, duration ?? 6000),
  info: (message: string, duration?: number) =>
    useToastStore.getState().addToast('info', message, duration),
  warning: (message: string, duration?: number) =>
    useToastStore.getState().addToast('warning', message, duration ?? 5000),
};

// ============================================================================
// Combined Selectors (for convenience)
// ============================================================================

/**
 * Get current view label from navItems
 */
export function getViewLabel(view: View): string {
  const item = navItems.find((item) => item.id === view);
  return item?.label || 'Einstellungen';
}

// ============================================================================
// Model Capabilities Detection
// ============================================================================

export interface ModelCapabilities {
  vision: boolean;
  webSearch: boolean;
  pdfUpload: boolean;
}

/**
 * Web search capable models.
 * - OpenAI o3, o4-mini: web_search_preview tool (Note: these are deprecated in vision context)
 * - Perplexity sonar, sonar-pro: built-in web search via Sonar API
 */
const WEB_SEARCH_MODELS = [
  // OpenAI models with web search (if enabled)
  'o3',
  'o4-mini',
  // Perplexity models always have web search
  'sonar',
  'sonar-pro',
];

/**
 * PDF direct upload capable providers.
 * Claude and Gemini can process PDFs directly without converting to images.
 */
const PDF_UPLOAD_PROVIDERS = ['claude', 'gemini'];

/**
 * Get model capabilities based on provider and model.
 * Used to conditionally enable features like web search, news integration.
 *
 * @example
 * const caps = getModelCapabilities('perplexity', 'sonar-pro');
 * if (caps.webSearch) {
 *   // Show "Include News" checkbox
 * }
 */
export function getModelCapabilities(provider: string, model: string): ModelCapabilities {
  // All listed models have vision support (that's why they're in AI_MODELS)
  const vision = true;

  // Web search: check if model is in the list OR if provider is perplexity (all perplexity models have web search)
  const webSearch =
    provider === 'perplexity' ||
    WEB_SEARCH_MODELS.some(m => model.startsWith(m));

  // PDF upload: Claude and Gemini support direct PDF upload
  const pdfUpload = PDF_UPLOAD_PROVIDERS.includes(provider);

  return { vision, webSearch, pdfUpload };
}

/**
 * Check if current AI settings support web search.
 * Convenience function that uses the settings store.
 */
export function currentModelSupportsWebSearch(): boolean {
  const { aiProvider, aiModel } = useSettingsStore.getState();
  return getModelCapabilities(aiProvider, aiModel).webSearch;
}
