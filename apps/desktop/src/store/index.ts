/**
 * Global application state using Zustand.
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

// ============================================================================
// Types
// ============================================================================

export type View =
  | 'dashboard'
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
  | 'charts'
  | 'benchmark'
  | 'reports'
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
  { id: 'benchmark', label: 'Benchmark', icon: 'Target', section: 'analysis' },
  { id: 'reports', label: 'Berichte', icon: 'BarChart3', section: 'analysis' },
  // Tools section
  { id: 'plans', label: 'Sparpläne', icon: 'CalendarClock', section: 'tools' },
  { id: 'rebalancing', label: 'Rebalancing', icon: 'Scale', section: 'tools' },
  { id: 'charts', label: 'Technische Analyse', icon: 'CandlestickChart', section: 'tools' },
];

// ============================================================================
// UI State
// ============================================================================

interface UIState {
  currentView: View;
  sidebarCollapsed: boolean;
  scrollTarget: string | null;
  setCurrentView: (view: View) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setScrollTarget: (target: string | null) => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      currentView: 'dashboard',
      sidebarCollapsed: false,
      scrollTarget: null,
      setCurrentView: (view) => set({ currentView: view }),
      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
      setScrollTarget: (target) => set({ scrollTarget: target }),
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

// AI Model options per provider (updated January 2025)
export type ClaudeModel = 'claude-sonnet-4-5-20250514' | 'claude-haiku-4-5-20251015' | 'claude-opus-4-5-20251101';
export type OpenAIModel = 'gpt-4.1' | 'gpt-4.1-mini' | 'gpt-4o' | 'o3';
export type GeminiModel = 'gemini-3-pro-preview' | 'gemini-3-flash' | 'gemini-2.5-flash' | 'gemini-2.5-pro';

// AI Models - Updated January 2025
// Sources:
// - Claude: https://platform.claude.com/docs/en/about-claude/models/overview
// - OpenAI: https://platform.openai.com/docs/models
// - Gemini: https://ai.google.dev/gemini-api/docs/models
// - Perplexity: https://docs.perplexity.ai/guides/model-cards
export const AI_MODELS = {
  claude: [
    { id: 'claude-sonnet-4-5-20250514', name: 'Claude Sonnet 4.5', description: 'Ausgewogen' },
    { id: 'claude-haiku-4-5-20251015', name: 'Claude Haiku 4.5', description: 'Schnell & günstig' },
    { id: 'claude-opus-4-5-20251101', name: 'Claude Opus 4.5', description: 'Beste Qualität' },
  ],
  openai: [
    { id: 'gpt-4.1', name: 'GPT-4.1', description: 'Neuestes, 1M Kontext' },
    { id: 'gpt-4.1-mini', name: 'GPT-4.1 Mini', description: 'Schnell & günstig' },
    { id: 'gpt-4o', name: 'GPT-4o', description: 'Multimodal, bewährt' },
    { id: 'o3', name: 'o3', description: 'Reasoning-Modell' },
  ],
  gemini: [
    { id: 'gemini-3-flash', name: 'Gemini 3 Flash', description: 'Neuestes, schnell' },
    { id: 'gemini-3-pro-preview', name: 'Gemini 3 Pro', description: 'Beste Qualität (Preview)' },
    { id: 'gemini-2.5-flash', name: 'Gemini 2.5 Flash', description: 'Bewährt, schnell' },
    { id: 'gemini-2.5-pro', name: 'Gemini 2.5 Pro', description: 'Bewährt, beste Qualität' },
  ],
  perplexity: [
    { id: 'sonar-pro', name: 'Sonar Pro', description: 'Beste Qualität + Web-Suche' },
    { id: 'sonar', name: 'Sonar', description: 'Schnell + Web-Suche' },
    { id: 'sonar-reasoning-pro', name: 'Sonar Reasoning Pro', description: 'Reasoning mit CoT' },
    { id: 'sonar-deep-research', name: 'Sonar Deep Research', description: 'Experten-Recherche' },
  ],
} as const;

// Deprecated model mappings - auto-upgrade to replacements
const DEPRECATED_MODELS: Record<string, Record<string, string>> = {
  perplexity: {
    'sonar-reasoning': 'sonar-reasoning-pro',
  },
  claude: {
    'claude-3-opus-20240229': 'claude-sonnet-4-5-20250514',
    'claude-3-sonnet-20240229': 'claude-sonnet-4-5-20250514',
    'claude-3-haiku-20240307': 'claude-haiku-4-5-20251015',
    'claude-2.1': 'claude-sonnet-4-5-20250514',
  },
  openai: {
    'gpt-4-vision-preview': 'gpt-4o',
    'gpt-4-turbo': 'gpt-4.1',
    'gpt-4-turbo-preview': 'gpt-4.1',
  },
  gemini: {
    'gemini-pro-vision': 'gemini-2.0-flash',
    'gemini-1.5-pro': 'gemini-2.5-pro',
    'gemini-1.5-flash': 'gemini-2.5-flash',
  },
};

// Get upgraded model if deprecated
function getUpgradedModel(provider: string, model: string): string | null {
  return DEPRECATED_MODELS[provider]?.[model] || null;
}

interface SettingsState {
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
  setLanguage: (lang: 'de' | 'en') => void;
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  setBaseCurrency: (currency: string) => void;

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

  // Model migration tracking (not persisted)
  pendingModelMigration: { from: string; to: string; provider: string } | null;
  clearPendingModelMigration: () => void;
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
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
      setLanguage: (lang) => set({ language: lang }),
      setTheme: (theme) => set({ theme: theme }),
      setBaseCurrency: (currency) => set({ baseCurrency: currency }),

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

      // AI Analysis Settings
      aiProvider: 'claude',
      setAiProvider: (provider) => set({
        aiProvider: provider,
        // Reset model to default for new provider
        aiModel: provider === 'claude' ? 'claude-sonnet-4-5-20250514'
          : provider === 'openai' ? 'gpt-4.1'
          : 'gemini-3-flash',
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

      // Model migration tracking (transient, not persisted)
      pendingModelMigration: null,
      clearPendingModelMigration: () => set({ pendingModelMigration: null }),
    }),
    {
      name: 'portfolio-settings',
      version: 3, // Increment when model lists change (v3: Added Gemini 3)
      migrate: (persistedState, version) => {
        const state = persistedState as Partial<SettingsState>;

        // Migration: validate AI models (revalidate on each version bump)
        if (version < 3) {
          const provider = state.aiProvider || 'claude';
          const validModels = AI_MODELS[provider].map(m => m.id) as string[];

          // Reset to default if current model is not valid for the provider
          if (state.aiModel && !validModels.includes(state.aiModel)) {
            state.aiModel = AI_MODELS[provider][0].id;
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
      // Exclude transient state from persistence
      partialize: (state) => {
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        const { pendingModelMigration, clearPendingModelMigration, ...persisted } = state;
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
