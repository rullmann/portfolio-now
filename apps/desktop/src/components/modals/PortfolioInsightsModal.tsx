/**
 * Portfolio Insights Modal - AI-powered portfolio analysis.
 *
 * Displays AI-generated insights about the portfolio including
 * strengths, weaknesses, and recommendations.
 */

import { useState, useEffect, useMemo } from 'react';
import {
  X, Sparkles, RefreshCw, Loader2, AlertCircle, CheckCircle,
  TrendingUp, Target, AlertTriangle, Lightbulb, PieChart
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '../../store';
import { AIProviderLogo } from '../common/AIProviderLogo';
import ReactMarkdown from 'react-markdown';

interface PortfolioInsightsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface PortfolioInsightsResponse {
  analysis: string;
  provider: string;
  model: string;
  tokensUsed: number | null;
}

interface LoadingStep {
  id: string;
  label: string;
  status: 'pending' | 'loading' | 'done' | 'error';
}

interface ParsedSection {
  type: 'summary' | 'strengths' | 'risks' | 'recommendations' | 'other';
  title: string;
  content: string;
}

/** Parse markdown into structured sections */
function parseAnalysis(markdown: string): ParsedSection[] {
  const sections: ParsedSection[] = [];
  const lines = markdown.split('\n');

  let currentSection: ParsedSection | null = null;
  let contentLines: string[] = [];

  const detectSectionType = (title: string): ParsedSection['type'] => {
    const lower = title.toLowerCase();
    if (lower.includes('zusammenfassung') || lower.includes('übersicht') || lower.includes('summary')) {
      return 'summary';
    }
    if (lower.includes('stärke') || lower.includes('strength') || lower.includes('positiv')) {
      return 'strengths';
    }
    if (lower.includes('risik') || lower.includes('schwäche') || lower.includes('risk') || lower.includes('weakness')) {
      return 'risks';
    }
    if (lower.includes('empfehlung') || lower.includes('recommendation') || lower.includes('vorschlag') || lower.includes('maßnahme')) {
      return 'recommendations';
    }
    return 'other';
  };

  const saveCurrentSection = () => {
    if (currentSection && contentLines.length > 0) {
      currentSection.content = contentLines.join('\n').trim();
      sections.push(currentSection);
    }
    contentLines = [];
  };

  for (const line of lines) {
    // Check for markdown headers (## or ###)
    const headerMatch = line.match(/^#{1,3}\s+(.+)$/);
    if (headerMatch) {
      saveCurrentSection();
      const title = headerMatch[1].trim();
      currentSection = {
        type: detectSectionType(title),
        title,
        content: '',
      };
    } else if (currentSection) {
      contentLines.push(line);
    } else {
      // Content before first header - treat as summary
      if (line.trim()) {
        if (!currentSection) {
          currentSection = { type: 'summary', title: 'Zusammenfassung', content: '' };
        }
        contentLines.push(line);
      }
    }
  }

  saveCurrentSection();
  return sections;
}

/** Section card component */
function InsightCard({ section }: { section: ParsedSection }) {
  const getStyles = () => {
    switch (section.type) {
      case 'summary':
        return {
          bg: 'bg-primary/5 border-primary/20',
          icon: <PieChart className="h-5 w-5 text-primary" />,
          iconBg: 'bg-primary/10',
        };
      case 'strengths':
        return {
          bg: 'bg-green-500/5 border-green-500/20',
          icon: <TrendingUp className="h-5 w-5 text-green-600 dark:text-green-400" />,
          iconBg: 'bg-green-500/10',
        };
      case 'risks':
        return {
          bg: 'bg-orange-500/5 border-orange-500/20',
          icon: <AlertTriangle className="h-5 w-5 text-orange-600 dark:text-orange-400" />,
          iconBg: 'bg-orange-500/10',
        };
      case 'recommendations':
        return {
          bg: 'bg-blue-500/5 border-blue-500/20',
          icon: <Lightbulb className="h-5 w-5 text-blue-600 dark:text-blue-400" />,
          iconBg: 'bg-blue-500/10',
        };
      default:
        return {
          bg: 'bg-muted/50 border-border',
          icon: <Target className="h-5 w-5 text-muted-foreground" />,
          iconBg: 'bg-muted',
        };
    }
  };

  const styles = getStyles();

  return (
    <div className={`rounded-lg border p-4 ${styles.bg}`}>
      <div className="flex items-start gap-3">
        <div className={`p-2 rounded-lg ${styles.iconBg} shrink-0`}>
          {styles.icon}
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-sm mb-2">{section.title}</h3>
          <div className="prose prose-sm dark:prose-invert max-w-none text-[13px] leading-relaxed prose-p:my-1 prose-ul:my-1 prose-li:my-0.5">
            <ReactMarkdown>{section.content}</ReactMarkdown>
          </div>
        </div>
      </div>
    </div>
  );
}

export function PortfolioInsightsModal({ isOpen, onClose }: PortfolioInsightsModalProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<PortfolioInsightsResponse | null>(null);
  const [steps, setSteps] = useState<LoadingStep[]>([
    { id: 'holdings', label: 'Holdings laden', status: 'pending' },
    { id: 'performance', label: 'Performance berechnen', status: 'pending' },
    { id: 'analysis', label: 'KI-Analyse', status: 'pending' },
  ]);

  // Parse analysis into sections
  const parsedSections = useMemo(() => {
    if (!result?.analysis) return [];
    return parseAnalysis(result.analysis);
  }, [result?.analysis]);

  const {
    aiProvider,
    aiModel,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
    baseCurrency,
  } = useSettingsStore();

  const getApiKey = () => {
    switch (aiProvider) {
      case 'claude':
        return anthropicApiKey;
      case 'openai':
        return openaiApiKey;
      case 'gemini':
        return geminiApiKey;
      case 'perplexity':
        return perplexityApiKey;
      default:
        return '';
    }
  };

  const hasApiKey = () => {
    const key = getApiKey();
    return key && key.trim().length > 0;
  };

  const updateStep = (stepId: string, status: LoadingStep['status']) => {
    setSteps((prev) =>
      prev.map((s) => (s.id === stepId ? { ...s, status } : s))
    );
  };

  const resetSteps = () => {
    setSteps([
      { id: 'holdings', label: 'Holdings laden', status: 'pending' },
      { id: 'performance', label: 'Performance berechnen', status: 'pending' },
      { id: 'analysis', label: 'KI-Analyse', status: 'pending' },
    ]);
  };

  const analyzePortfolio = async () => {
    if (!hasApiKey()) {
      setError(`Bitte konfiguriere deinen ${aiProvider.toUpperCase()} API-Key in den Einstellungen.`);
      return;
    }

    setIsLoading(true);
    setError(null);
    setResult(null);
    resetSteps();

    try {
      // Simulate step progression for better UX
      updateStep('holdings', 'loading');
      await new Promise((r) => setTimeout(r, 300));
      updateStep('holdings', 'done');

      updateStep('performance', 'loading');
      await new Promise((r) => setTimeout(r, 300));
      updateStep('performance', 'done');

      updateStep('analysis', 'loading');

      const response = await invoke<PortfolioInsightsResponse>('analyze_portfolio_with_ai', {
        request: {
          provider: aiProvider,
          model: aiModel,
          apiKey: getApiKey(),
          baseCurrency: baseCurrency || 'EUR',
        },
      });

      updateStep('analysis', 'done');
      setResult(response);
    } catch (err) {
      updateStep('analysis', 'error');
      const errorMessage = typeof err === 'string' ? err : String(err);

      // Try to parse structured error
      try {
        const parsed = JSON.parse(errorMessage);
        if (parsed.message) {
          setError(parsed.message);
        } else {
          setError(errorMessage);
        }
      } catch {
        setError(errorMessage);
      }
    } finally {
      setIsLoading(false);
    }
  };

  // Auto-analyze when modal opens
  useEffect(() => {
    if (isOpen && !result && !isLoading && !error) {
      analyzePortfolio();
    }
  }, [isOpen]);

  // Reset state when modal closes
  useEffect(() => {
    if (!isOpen) {
      setResult(null);
      setError(null);
      resetSteps();
    }
  }, [isOpen]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-background border border-border rounded-lg shadow-xl w-full max-w-2xl max-h-[85vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-primary" />
            <h2 className="text-lg font-semibold">Portfolio Insights</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded hover:bg-muted transition-colors"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {/* Loading Steps */}
          {isLoading && (
            <div className="space-y-2 p-4 bg-muted/50 rounded-lg">
              {steps.map((step) => (
                <div key={step.id} className="flex items-center gap-3">
                  {step.status === 'pending' && (
                    <div className="h-4 w-4 rounded-full border-2 border-muted-foreground/30" />
                  )}
                  {step.status === 'loading' && (
                    <Loader2 className="h-4 w-4 animate-spin text-primary" />
                  )}
                  {step.status === 'done' && (
                    <CheckCircle className="h-4 w-4 text-green-500" />
                  )}
                  {step.status === 'error' && (
                    <AlertCircle className="h-4 w-4 text-destructive" />
                  )}
                  <span
                    className={
                      step.status === 'done'
                        ? 'text-muted-foreground'
                        : step.status === 'loading'
                        ? 'font-medium'
                        : ''
                    }
                  >
                    {step.label}
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* Error */}
          {error && !isLoading && (
            <div className="p-4 bg-destructive/10 border border-destructive/20 rounded-lg">
              <div className="flex items-start gap-3">
                <AlertCircle className="h-5 w-5 text-destructive flex-shrink-0 mt-0.5" />
                <div>
                  <p className="font-medium text-destructive">Analyse fehlgeschlagen</p>
                  <p className="text-sm text-muted-foreground mt-1">{error}</p>
                </div>
              </div>
            </div>
          )}

          {/* Result */}
          {result && !isLoading && (
            <div className="space-y-3">
              {parsedSections.length > 0 ? (
                // Structured display with cards
                parsedSections.map((section, index) => (
                  <InsightCard key={index} section={section} />
                ))
              ) : (
                // Fallback to plain markdown if parsing fails
                <div className="prose prose-sm dark:prose-invert max-w-none">
                  <ReactMarkdown>{result.analysis}</ReactMarkdown>
                </div>
              )}
            </div>
          )}

          {/* No API Key Warning */}
          {!hasApiKey() && !isLoading && !result && !error && (
            <div className="p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
              <div className="flex items-start gap-3">
                <AlertCircle className="h-5 w-5 text-yellow-500 flex-shrink-0 mt-0.5" />
                <div>
                  <p className="font-medium text-yellow-600 dark:text-yellow-400">
                    API-Key erforderlich
                  </p>
                  <p className="text-sm text-muted-foreground mt-1">
                    Bitte konfiguriere deinen {aiProvider.toUpperCase()} API-Key in den Einstellungen,
                    um KI-Analysen zu nutzen.
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between p-4 border-t border-border bg-muted/30">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            {result && (
              <>
                <AIProviderLogo provider={aiProvider} size={16} />
                <span>
                  {result.model}
                  {result.tokensUsed && ` (${result.tokensUsed.toLocaleString()} Tokens)`}
                </span>
              </>
            )}
          </div>

          <button
            onClick={analyzePortfolio}
            disabled={isLoading || !hasApiKey()}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {isLoading ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>Analysiere...</span>
              </>
            ) : (
              <>
                <RefreshCw className="h-4 w-4" />
                <span>Neu analysieren</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
