import { create } from 'zustand';

export interface QuoteSyncProgressItem {
  securityId: number;
  securityName: string;
  ticker: string;
  provider: string;
  status: 'loading' | 'success' | 'error';
  price?: number;
  currency?: string;
  error?: string;
  currentIndex: number;
  totalCount: number;
}

interface QuoteSyncState {
  isActive: boolean;
  isComplete: boolean;
  items: Map<number, QuoteSyncProgressItem>;
  totalCount: number;
  startSync: (total: number) => void;
  updateProgress: (item: QuoteSyncProgressItem) => void;
  completeSync: () => void;
  dismiss: () => void;
}

export const useQuoteSyncStore = create<QuoteSyncState>()((set) => ({
  isActive: false,
  isComplete: false,
  items: new Map(),
  totalCount: 0,
  startSync: (total) =>
    set({
      isActive: true,
      isComplete: false,
      items: new Map(),
      totalCount: total,
    }),
  updateProgress: (item) =>
    set((state) => {
      const next = new Map(state.items);
      next.set(item.securityId, item);
      return { items: next };
    }),
  completeSync: () => set({ isComplete: true }),
  dismiss: () =>
    set({
      isActive: false,
      isComplete: false,
      items: new Map(),
      totalCount: 0,
    }),
}));
