/**
 * AI Analysis Panel for technical chart analysis.
 * Captures the chart as an image and sends it to an AI provider for analysis.
 */

import { useState, useMemo, useRef } from 'react';
import { Sparkles, RefreshCw, AlertCircle, Settings, ChevronDown, ChevronUp } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import ReactMarkdown from 'react-markdown';
import { useSettingsStore, useUIStore } from '../../store';
import type { IndicatorConfig } from '../../lib/indicators';

// Local SecurityData type (simplified version used in ChartsView)
interface SecurityData {
  id: number;
  name: string;
  isin: string | null;
  ticker: string | null;
  currency: string;
}

interface ChartAnalysisResponse {
  analysis: string;
  provider: string;
  model: string;
  tokensUsed?: number;
}

interface AIAnalysisPanelProps {
  chartRef: React.RefObject<HTMLDivElement>;
  security: SecurityData | null;
  currentPrice: number;
  timeRange: string;
  indicators: IndicatorConfig[];
}

export function AIAnalysisPanel({
  chartRef,
  security,
  currentPrice,
  timeRange,
  indicators,
}: AIAnalysisPanelProps) {
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [analysis, setAnalysis] = useState<string | null>(null);
  const [analysisInfo, setAnalysisInfo] = useState<{ provider: string; model: string; tokens?: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  const { aiProvider, anthropicApiKey, openaiApiKey, geminiApiKey } = useSettingsStore();

  // Get API key for selected provider
  const apiKey = useMemo(() => {
    switch (aiProvider) {
      case 'claude':
        return anthropicApiKey;
      case 'openai':
        return openaiApiKey;
      case 'gemini':
        return geminiApiKey;
    }
  }, [aiProvider, anthropicApiKey, openaiApiKey, geminiApiKey]);

  const providerName = useMemo(() => {
    switch (aiProvider) {
      case 'claude':
        return 'Claude';
      case 'openai':
        return 'GPT-4';
      case 'gemini':
        return 'Gemini';
    }
  }, [aiProvider]);

  const handleAnalyze = async () => {
    if (!chartRef.current || !security || !apiKey) return;

    setIsAnalyzing(true);
    setError(null);
    setIsCollapsed(false);

    try {
      // Find all canvases in the chart container
      const canvases = chartRef.current.querySelectorAll('canvas');
      if (canvases.length === 0) {
        throw new Error('Chart canvas nicht gefunden');
      }

      // Create a combined canvas with all chart layers
      const container = chartRef.current;
      const combinedCanvas = document.createElement('canvas');
      combinedCanvas.width = container.clientWidth;
      combinedCanvas.height = container.clientHeight;
      const ctx = combinedCanvas.getContext('2d');

      if (!ctx) {
        throw new Error('Canvas context nicht verfügbar');
      }

      // Fill with background color
      const isDark = document.documentElement.classList.contains('dark');
      ctx.fillStyle = isDark ? '#1f2937' : '#ffffff';
      ctx.fillRect(0, 0, combinedCanvas.width, combinedCanvas.height);

      // Draw each canvas layer
      canvases.forEach((canvas) => {
        const rect = canvas.getBoundingClientRect();
        const containerRect = container.getBoundingClientRect();
        const x = rect.left - containerRect.left;
        const y = rect.top - containerRect.top;
        ctx.drawImage(canvas, x, y);
      });

      // Convert to base64
      const imageBase64 = combinedCanvas.toDataURL('image/png').split(',')[1];

      // Build context
      const context = {
        securityName: security.name,
        ticker: security.ticker,
        currency: security.currency,
        currentPrice,
        timeframe: timeRange,
        indicators: indicators
          .filter((i) => i.enabled)
          .map((i) => {
            const params = Object.entries(i.params)
              .filter(([, v]) => v !== undefined)
              .map(([, v]) => v)
              .join(',');
            return `${i.type.toUpperCase()}(${params})`;
          }),
      };

      // Call backend
      const result = await invoke<ChartAnalysisResponse>('analyze_chart_with_ai', {
        request: {
          imageBase64,
          provider: aiProvider,
          apiKey,
          context,
        },
      });

      setAnalysis(result.analysis);
      setAnalysisInfo({
        provider: result.provider,
        model: result.model,
        tokens: result.tokensUsed,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsAnalyzing(false);
    }
  };

  const { setCurrentView, setScrollTarget } = useUIStore();

  const navigateToAiSettings = () => {
    setScrollTarget('ai-analysis');
    setCurrentView('settings');
  };

  // No API key configured
  if (!apiKey) {
    return (
      <div className="bg-card border border-border rounded-lg p-4">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-amber-100 dark:bg-amber-900/30 rounded-lg">
            <AlertCircle size={20} className="text-amber-600 dark:text-amber-400" />
          </div>
          <div className="flex-1">
            <p className="text-sm font-medium">KI-Analyse nicht konfiguriert</p>
            <p className="text-xs text-muted-foreground mt-0.5">
              Bitte hinterlege einen API Key in den Einstellungen.
            </p>
          </div>
          <button
            onClick={navigateToAiSettings}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            <Settings size={14} />
            Einstellungen
          </button>
        </div>
      </div>
    );
  }

  return (
    <div ref={panelRef} className="bg-card border border-border rounded-lg overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-border bg-muted/30">
        <button
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="flex items-center gap-2 hover:text-primary transition-colors"
        >
          <Sparkles size={16} className="text-primary" />
          <span className="font-medium">KI-Analyse</span>
          <span className="text-xs text-muted-foreground px-2 py-0.5 bg-muted rounded">
            {providerName}
          </span>
          {isCollapsed ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
        </button>
        <button
          onClick={handleAnalyze}
          disabled={isAnalyzing || !security}
          className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {isAnalyzing ? (
            <>
              <RefreshCw size={14} className="animate-spin" />
              Analysiere...
            </>
          ) : (
            <>
              <Sparkles size={14} />
              Chart analysieren
            </>
          )}
        </button>
      </div>

      {/* Content */}
      {!isCollapsed && (
        <>
          <div className="p-4 max-h-80 overflow-y-auto">
            {error && (
              <div className="flex items-start gap-2 text-destructive text-sm bg-destructive/10 p-3 rounded-md">
                <AlertCircle size={16} className="shrink-0 mt-0.5" />
                <div>
                  <p className="font-medium">Fehler bei der Analyse</p>
                  <p className="text-xs mt-1 opacity-80">{error}</p>
                </div>
              </div>
            )}
            {analysis ? (
              <div className="prose prose-sm dark:prose-invert max-w-none prose-headings:text-base prose-headings:font-semibold prose-p:my-2 prose-ul:my-2 prose-li:my-0.5">
                <ReactMarkdown>{analysis}</ReactMarkdown>
              </div>
            ) : (
              <div className="text-muted-foreground text-sm text-center py-8">
                <Sparkles size={32} className="mx-auto mb-3 opacity-30" />
                <p>Klicke "Chart analysieren" um eine KI-gestützte</p>
                <p>technische Analyse zu erhalten.</p>
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="px-4 py-2 border-t border-border bg-muted/30 flex items-center justify-between">
            <span className="text-xs text-muted-foreground">
              Dies ist keine Anlageberatung. Analysen können fehlerhaft sein.
            </span>
            {analysisInfo && (
              <span className="text-xs text-muted-foreground">
                {analysisInfo.model}
                {analysisInfo.tokens && ` | ${analysisInfo.tokens.toLocaleString()} Tokens`}
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}
