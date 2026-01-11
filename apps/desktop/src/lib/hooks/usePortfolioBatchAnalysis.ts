/**
 * Hook for batch AI analysis of portfolio holdings.
 * Handles rate limiting, progress tracking, and abort functionality.
 */

import { useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  usePortfolioAnalysisStore,
  type TrendDirection,
  type TrendStrength,
} from '../../store/portfolioAnalysis';
import { useSettingsStore, toast } from '../../store';
import type { AnnotationAnalysisResponse } from '../types';

// ============================================================================
// Types
// ============================================================================

export interface HoldingForAnalysis {
  securityId: number;
  name: string;
  ticker: string | null;
  currency: string;
  currentPrice?: number;
}

interface AnalyzeChartRequest {
  imageBase64: string;
  provider: string;
  model: string;
  apiKey: string;
  context: {
    securityName: string;
    ticker: string | null;
    currency: string;
    currentPrice?: number;
    timeframe: string;
    indicators: string[];
  };
}

export interface BatchAnalysisOptions {
  /** Maximum concurrent analyses (default: 3) */
  maxConcurrent?: number;
  /** Delay between batches in ms (default: 5000) */
  batchDelayMs?: number;
  /** Timeframe for chart analysis (default: '1Y') */
  timeframe?: string;
}

export interface UseBatchAnalysisResult {
  /** Start batch analysis for the given holdings */
  startBatchAnalysis: (
    holdings: HoldingForAnalysis[],
    captureChart: (securityId: number) => Promise<string | null>,
    options?: BatchAnalysisOptions
  ) => Promise<void>;
  /** Request abort of running analysis */
  requestAbort: () => void;
  /** Whether analysis is running */
  isAnalyzing: boolean;
  /** Current progress */
  progress: { current: number; total: number };
  /** Whether abort was requested */
  abortRequested: boolean;
}

// ============================================================================
// Hook
// ============================================================================

export function usePortfolioBatchAnalysis(): UseBatchAnalysisResult {
  const {
    setAnalysis,
    setAnalysisPending,
    setAnalysisError,
    startBatchAnalysis: storeStartBatch,
    updateProgress,
    finishBatchAnalysis,
    requestAbort: storeRequestAbort,
    resetAbort,
    isAnalyzing,
    progress,
    abortRequested,
  } = usePortfolioAnalysisStore();

  const { aiProvider, aiModel, anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey } =
    useSettingsStore();

  const abortRef = useRef(false);

  // Get API key for current provider
  const getApiKey = useCallback(() => {
    switch (aiProvider) {
      case 'claude':
        return anthropicApiKey;
      case 'openai':
        return openaiApiKey;
      case 'gemini':
        return geminiApiKey;
      case 'perplexity':
        return perplexityApiKey;
    }
  }, [aiProvider, anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey]);

  // Parse trend from AI response
  const parseTrendFromResponse = (
    response: AnnotationAnalysisResponse
  ): { trend: TrendDirection; strength: TrendStrength; confidence: number; summary: string } => {
    const trend = response.trend?.direction || 'neutral';
    const strength = response.trend?.strength || 'moderate';
    const confidence = response.trend?.confidence || 0.5;

    // Extract first sentence as summary
    const firstSentence = response.analysis.split(/[.!?]/)[0]?.trim() || response.analysis.slice(0, 100);

    return {
      trend: trend as TrendDirection,
      strength: strength as TrendStrength,
      confidence,
      summary: firstSentence,
    };
  };

  // Analyze single holding
  const analyzeHolding = useCallback(
    async (
      holding: HoldingForAnalysis,
      imageBase64: string,
      timeframe: string
    ): Promise<void> => {
      const apiKey = getApiKey();
      if (!apiKey) {
        throw new Error('Kein API Key konfiguriert');
      }

      const request: AnalyzeChartRequest = {
        imageBase64,
        provider: aiProvider,
        model: aiModel,
        apiKey,
        context: {
          securityName: holding.name,
          ticker: holding.ticker,
          currency: holding.currency,
          currentPrice: holding.currentPrice,
          timeframe,
          indicators: ['RSI(14)', 'MACD(12,26,9)'], // Default indicators
        },
      };

      const result = await invoke<AnnotationAnalysisResponse>('analyze_chart_with_annotations', {
        request,
      });

      const parsed = parseTrendFromResponse(result);

      setAnalysis(holding.securityId, {
        securityId: holding.securityId,
        name: holding.name,
        trend: parsed.trend,
        strength: parsed.strength,
        confidence: parsed.confidence,
        summary: parsed.summary,
      });
    },
    [aiProvider, aiModel, getApiKey, setAnalysis]
  );

  // Process batch with rate limiting
  const processBatch = useCallback(
    async (
      batch: HoldingForAnalysis[],
      captureChart: (securityId: number) => Promise<string | null>,
      timeframe: string
    ): Promise<void> => {
      const promises = batch.map(async (holding) => {
        if (abortRef.current) return;

        try {
          // Capture chart image
          const imageBase64 = await captureChart(holding.securityId);
          if (!imageBase64) {
            setAnalysisError(holding.securityId, holding.name, 'Chart konnte nicht erfasst werden');
            return;
          }

          // Run analysis
          await analyzeHolding(holding, imageBase64, timeframe);
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          setAnalysisError(holding.securityId, holding.name, message);
        }
      });

      await Promise.all(promises);
    },
    [analyzeHolding, setAnalysisError]
  );

  // Main batch analysis function
  const startBatchAnalysis = useCallback(
    async (
      holdings: HoldingForAnalysis[],
      captureChart: (securityId: number) => Promise<string | null>,
      options?: BatchAnalysisOptions
    ): Promise<void> => {
      const apiKey = getApiKey();
      if (!apiKey) {
        toast.error('Bitte konfiguriere einen AI API Key in den Einstellungen');
        return;
      }

      if (holdings.length === 0) {
        toast.warning('Keine Holdings zum Analysieren gefunden');
        return;
      }

      const maxConcurrent = options?.maxConcurrent ?? 3;
      const batchDelayMs = options?.batchDelayMs ?? 5000;
      const timeframe = options?.timeframe ?? '1Y';

      // Reset abort flag and start
      abortRef.current = false;
      resetAbort();
      storeStartBatch(holdings.length);

      // Mark all as pending
      holdings.forEach((h) => setAnalysisPending(h.securityId, h.name));

      let processed = 0;

      try {
        // Process in batches
        for (let i = 0; i < holdings.length; i += maxConcurrent) {
          if (abortRef.current) {
            toast.info('Batch-Analyse abgebrochen');
            break;
          }

          const batch = holdings.slice(i, i + maxConcurrent);
          await processBatch(batch, captureChart, timeframe);

          processed += batch.length;
          updateProgress(processed);

          // Delay between batches (except for last batch)
          if (i + maxConcurrent < holdings.length && !abortRef.current) {
            await new Promise((resolve) => setTimeout(resolve, batchDelayMs));
          }
        }

        if (!abortRef.current) {
          toast.success(`Analyse abgeschlossen für ${holdings.length} Holdings`);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        toast.error(`Batch-Analyse Fehler: ${message}`);
      } finally {
        finishBatchAnalysis();
      }
    },
    [
      getApiKey,
      resetAbort,
      storeStartBatch,
      setAnalysisPending,
      processBatch,
      updateProgress,
      finishBatchAnalysis,
    ]
  );

  // Request abort
  const requestAbort = useCallback(() => {
    abortRef.current = true;
    storeRequestAbort();
  }, [storeRequestAbort]);

  return {
    startBatchAnalysis,
    requestAbort,
    isAnalyzing,
    progress,
    abortRequested,
  };
}

// ============================================================================
// Helper: Sleep utility
// ============================================================================

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
