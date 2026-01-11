/**
 * Portfolio-wide AI Analysis Store
 * Tracks batch analysis results for all holdings with trend indicators.
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

// ============================================================================
// Types
// ============================================================================

export type TrendDirection = 'bullish' | 'bearish' | 'neutral';
export type TrendStrength = 'strong' | 'moderate' | 'weak';
export type AnalysisStatus = TrendDirection | 'pending' | 'error';

export interface HoldingAnalysis {
  securityId: number;
  name: string;
  trend: AnalysisStatus;
  strength?: TrendStrength;
  confidence?: number;
  summary?: string;
  errorMessage?: string;
  analyzedAt?: string;
}

interface BatchProgress {
  current: number;
  total: number;
}

interface PortfolioAnalysisState {
  // Analysis results per security
  analyses: Record<number, HoldingAnalysis>;

  // Batch analysis state
  isAnalyzing: boolean;
  progress: BatchProgress;
  abortRequested: boolean;
  lastBatchRun: string | null;

  // Actions
  setAnalysis: (securityId: number, analysis: HoldingAnalysis) => void;
  setAnalysisPending: (securityId: number, name: string) => void;
  setAnalysisError: (securityId: number, name: string, errorMessage: string) => void;
  clearAnalysis: (securityId: number) => void;
  clearAllAnalyses: () => void;

  // Batch control
  startBatchAnalysis: (total: number) => void;
  updateProgress: (current: number) => void;
  finishBatchAnalysis: () => void;
  requestAbort: () => void;
  resetAbort: () => void;

  // Selectors
  getAnalysis: (securityId: number) => HoldingAnalysis | undefined;
  getTrend: (securityId: number) => AnalysisStatus | undefined;
}

// ============================================================================
// Store
// ============================================================================

export const usePortfolioAnalysisStore = create<PortfolioAnalysisState>()(
  persist(
    (set, get) => ({
      // Initial state
      analyses: {},
      isAnalyzing: false,
      progress: { current: 0, total: 0 },
      abortRequested: false,
      lastBatchRun: null,

      // Set a complete analysis result
      setAnalysis: (securityId, analysis) =>
        set((state) => ({
          analyses: {
            ...state.analyses,
            [securityId]: {
              ...analysis,
              analyzedAt: new Date().toISOString(),
            },
          },
        })),

      // Mark a security as pending analysis
      setAnalysisPending: (securityId, name) =>
        set((state) => ({
          analyses: {
            ...state.analyses,
            [securityId]: {
              securityId,
              name,
              trend: 'pending',
            },
          },
        })),

      // Mark a security as failed
      setAnalysisError: (securityId, name, errorMessage) =>
        set((state) => ({
          analyses: {
            ...state.analyses,
            [securityId]: {
              securityId,
              name,
              trend: 'error',
              errorMessage,
              analyzedAt: new Date().toISOString(),
            },
          },
        })),

      // Clear single analysis
      clearAnalysis: (securityId) =>
        set((state) => {
          const { [securityId]: _, ...rest } = state.analyses;
          return { analyses: rest };
        }),

      // Clear all analyses
      clearAllAnalyses: () =>
        set({ analyses: {}, lastBatchRun: null }),

      // Start batch analysis
      startBatchAnalysis: (total) =>
        set({
          isAnalyzing: true,
          progress: { current: 0, total },
          abortRequested: false,
        }),

      // Update progress
      updateProgress: (current) =>
        set((state) => ({
          progress: { ...state.progress, current },
        })),

      // Finish batch analysis
      finishBatchAnalysis: () =>
        set({
          isAnalyzing: false,
          progress: { current: 0, total: 0 },
          abortRequested: false,
          lastBatchRun: new Date().toISOString(),
        }),

      // Request abort
      requestAbort: () => set({ abortRequested: true }),

      // Reset abort flag
      resetAbort: () => set({ abortRequested: false }),

      // Get single analysis
      getAnalysis: (securityId) => get().analyses[securityId],

      // Get trend for a security
      getTrend: (securityId) => get().analyses[securityId]?.trend,
    }),
    {
      name: 'portfolio-analysis',
      // Only persist analyses and lastBatchRun, not transient state
      partialize: (state) => ({
        analyses: state.analyses,
        lastBatchRun: state.lastBatchRun,
      }),
    }
  )
);

// ============================================================================
// Convenience Hooks
// ============================================================================

/**
 * Get trend color class for a security
 */
export function getTrendColorClass(trend: AnalysisStatus | undefined): string {
  switch (trend) {
    case 'bullish':
      return 'bg-emerald-500';
    case 'bearish':
      return 'bg-red-500';
    case 'neutral':
      return 'bg-amber-500';
    case 'pending':
      return 'bg-gray-300 animate-pulse';
    case 'error':
      return 'bg-gray-400';
    default:
      return 'bg-gray-200';
  }
}

/**
 * Get trend text color class
 */
export function getTrendTextColorClass(trend: AnalysisStatus | undefined): string {
  switch (trend) {
    case 'bullish':
      return 'text-emerald-600';
    case 'bearish':
      return 'text-red-600';
    case 'neutral':
      return 'text-amber-600';
    case 'pending':
      return 'text-gray-400';
    case 'error':
      return 'text-gray-500';
    default:
      return 'text-gray-400';
  }
}

/**
 * Get trend label
 */
export function getTrendLabel(trend: AnalysisStatus | undefined): string {
  switch (trend) {
    case 'bullish':
      return 'Bullish';
    case 'bearish':
      return 'Bearish';
    case 'neutral':
      return 'Neutral';
    case 'pending':
      return 'Analyse...';
    case 'error':
      return 'Fehler';
    default:
      return '-';
  }
}
