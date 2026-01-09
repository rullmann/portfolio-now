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
  setCurrentView: (view: View) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      currentView: 'dashboard',
      sidebarCollapsed: false,
      setCurrentView: (view) => set({ currentView: view }),
      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
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
// Portfolio File State (Legacy - for direct file editing)
// ============================================================================

export interface PortfolioFileState {
  currentFilePath: string | null;
  hasUnsavedChanges: boolean;
  setCurrentFilePath: (path: string | null) => void;
  setHasUnsavedChanges: (hasChanges: boolean) => void;
}

export const usePortfolioFileStore = create<PortfolioFileState>()((set) => ({
  currentFilePath: null,
  hasUnsavedChanges: false,
  setCurrentFilePath: (path) => set({ currentFilePath: path }),
  setHasUnsavedChanges: (hasChanges) => set({ hasUnsavedChanges: hasChanges }),
}));

// ============================================================================
// Data Mode State
// ============================================================================

interface DataModeState {
  useDbData: boolean;
  setUseDbData: (useDb: boolean) => void;
}

export const useDataModeStore = create<DataModeState>()(
  persist(
    (set) => ({
      useDbData: true, // Default to DB mode
      setUseDbData: (useDb) => set({ useDbData: useDb }),
    }),
    {
      name: 'portfolio-data-mode',
    }
  )
);

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

interface SettingsState {
  // Quote sync settings
  syncOnlyHeldSecurities: boolean;
  setSyncOnlyHeldSecurities: (value: boolean) => void;
  autoUpdateInterval: AutoUpdateInterval;
  setAutoUpdateInterval: (interval: AutoUpdateInterval) => void;
  lastSyncTime: string | null; // ISO string for persistence
  setLastSyncTime: (time: Date | null) => void;

  // Display settings
  language: 'de' | 'en';
  theme: 'light' | 'dark' | 'system';
  baseCurrency: string;
  setLanguage: (lang: 'de' | 'en') => void;
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  setBaseCurrency: (currency: string) => void;

  // API Keys
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

      // Display settings
      language: 'de',
      theme: 'system',
      baseCurrency: 'EUR',
      setLanguage: (lang) => set({ language: lang }),
      setTheme: (theme) => set({ theme: theme }),
      setBaseCurrency: (currency) => set({ baseCurrency: currency }),

      // API Keys
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
    }),
    {
      name: 'portfolio-settings',
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
