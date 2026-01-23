/**
 * Simplified Rebalancing view - AI suggestions only, no execution.
 * Shows all holdings aggregated across all portfolios.
 * Includes allocation target management for alerts.
 */

import { useState, useEffect, useMemo } from 'react';
import { Scale, RefreshCw, Sparkles, AlertTriangle, X, ChevronDown, ChevronUp, Building2, Target, Bell } from 'lucide-react';
import { SafeMarkdown } from '../../components/common/SafeMarkdown';
import { getAllHoldings, suggestRebalanceWithAi } from '../../lib/api';
import type { AggregatedHolding, AiRebalanceSuggestion } from '../../lib/types';
import { toast, useSettingsStore } from '../../store';
import { AIProviderLogo } from '../../components/common';
import { AllocationTargetModal } from '../../components/modals';
import { AlertsPanel } from '../../components/alerts';

interface HoldingWithTarget {
  securityId: number;
  securityName: string;
  currentWeight: number;
  targetWeight: number;
  currentValue: number;
  shares: number;
  gainLoss: number;
  gainLossPercent: number;
  customLogo?: string;
  ticker?: string;
}

export function RebalancingView() {
  const [holdings, setHoldings] = useState<HoldingWithTarget[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingAi, setIsLoadingAi] = useState(false);
  const [aiSuggestion, setAiSuggestion] = useState<AiRebalanceSuggestion | null>(null);
  const [showAiReasoning, setShowAiReasoning] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showAlertsPanel, setShowAlertsPanel] = useState(false);
  const [targetModalOpen, setTargetModalOpen] = useState(false);
  const [selectedForTarget, setSelectedForTarget] = useState<{
    securityId: number;
    securityName: string;
    weight: number;
  } | null>(null);

  const {
    aiProvider,
    aiModel,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
    baseCurrency,
  } = useSettingsStore();

  // Check if AI is configured
  const hasAiConfigured = useMemo(() => {
    switch (aiProvider) {
      case 'claude':
        return !!anthropicApiKey;
      case 'openai':
        return !!openaiApiKey;
      case 'gemini':
        return !!geminiApiKey;
      case 'perplexity':
        return !!perplexityApiKey;
      default:
        return false;
    }
  }, [aiProvider, anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey]);

  const getApiKey = () => {
    switch (aiProvider) {
      case 'claude':
        return anthropicApiKey || '';
      case 'openai':
        return openaiApiKey || '';
      case 'gemini':
        return geminiApiKey || '';
      case 'perplexity':
        return perplexityApiKey || '';
      default:
        return '';
    }
  };

  // Create a simple logo map from customLogo data (keyed by security name)
  const logosByName = useMemo(() => {
    const map = new Map<string, string>();
    holdings.forEach((h) => {
      if (h.customLogo) {
        map.set(h.securityName, h.customLogo);
      }
    });
    return map;
  }, [holdings]);

  // Calculate totals
  const totalValue = useMemo(() => holdings.reduce((sum, h) => sum + h.currentValue, 0), [holdings]);
  const totalTargetWeight = useMemo(() => holdings.reduce((sum, h) => sum + h.targetWeight, 0), [holdings]);

  const loadHoldings = async () => {
    try {
      setIsLoading(true);
      setError(null);
      const data = await getAllHoldings();

      // Convert to HoldingWithTarget
      const total = data.reduce((sum, h) => sum + (h.currentValue || 0), 0);
      const holdingsWithTargets: HoldingWithTarget[] = data.map((h: AggregatedHolding) => {
        const weight = total > 0 ? ((h.currentValue || 0) / total) * 100 : 0;
        // Use first securityId from the array (aggregated holdings may have multiple)
        const primarySecurityId = h.securityIds && h.securityIds.length > 0 ? h.securityIds[0] : 0;
        return {
          securityId: primarySecurityId,
          securityName: h.name,
          currentWeight: weight,
          targetWeight: weight, // Start with current as target
          currentValue: h.currentValue || 0,
          shares: h.totalShares,
          gainLoss: h.gainLoss || 0,
          gainLossPercent: h.gainLossPercent || 0,
          customLogo: h.customLogo,
          ticker: h.ticker,
        };
      });

      // Sort by current value descending
      holdingsWithTargets.sort((a, b) => b.currentValue - a.currentValue);
      setHoldings(holdingsWithTargets);
      setAiSuggestion(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadHoldings();
  }, []);

  const handleAiSuggest = async () => {
    if (!hasAiConfigured || holdings.length === 0) return;
    try {
      setIsLoadingAi(true);
      setError(null);
      // Use portfolio_id = 0 to indicate "all portfolios"
      const suggestion = await suggestRebalanceWithAi(
        0, // 0 = all portfolios
        aiProvider,
        aiModel,
        getApiKey(),
        baseCurrency
      );
      // Apply AI suggestions to holdings (match by name since IDs may differ)
      setHoldings((prev) =>
        prev.map((h) => {
          const aiTarget = suggestion.targets.find(
            (at) => at.securityName.toLowerCase() === h.securityName.toLowerCase()
          );
          return aiTarget ? { ...h, targetWeight: aiTarget.targetWeight } : h;
        })
      );
      setAiSuggestion(suggestion);
      setShowAiReasoning(true);
      toast.success('KI-Vorschlag übernommen');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      toast.error('KI-Vorschlag fehlgeschlagen');
    } finally {
      setIsLoadingAi(false);
    }
  };

  const updateTargetWeight = (securityName: string, weight: number) => {
    setHoldings((prev) =>
      prev.map((h) => (h.securityName === securityName ? { ...h, targetWeight: weight } : h))
    );
  };

  const formatCurrency = (amount: number) => {
    // Values from getAllHoldings are already in EUR (not cents)
    return `${amount.toLocaleString('de-DE', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })} ${baseCurrency}`;
  };

  const openTargetModal = (holding?: HoldingWithTarget) => {
    if (holding) {
      setSelectedForTarget({
        securityId: holding.securityId,
        securityName: holding.securityName,
        weight: holding.targetWeight,
      });
    } else {
      setSelectedForTarget(null);
    }
    setTargetModalOpen(true);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Scale className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Rebalancing</h1>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowAlertsPanel(!showAlertsPanel)}
            className={`flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors ${
              showAlertsPanel ? 'bg-muted' : ''
            }`}
            title="Allokationswarnungen anzeigen"
          >
            <Bell size={16} />
            Warnungen
          </button>
          <button
            onClick={() => openTargetModal()}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
            title="Zielgewichtung hinzufügen"
          >
            <Target size={16} />
            Ziel
          </button>
          {hasAiConfigured && (
            <button
              onClick={handleAiSuggest}
              disabled={isLoadingAi || holdings.length === 0}
              className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
              title={`KI-Vorschlag mit ${aiProvider}`}
            >
              <Sparkles size={16} className={isLoadingAi ? 'animate-pulse' : ''} />
              {isLoadingAi ? 'Analysiere...' : 'KI-Vorschlag'}
            </button>
          )}
          <button
            onClick={loadHoldings}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
        </div>
      </div>

      {!hasAiConfigured && (
        <div className="p-3 bg-amber-500/10 border border-amber-500/20 rounded-md text-amber-700 dark:text-amber-400 text-sm">
          KI-Provider nicht konfiguriert. Bitte in den Einstellungen einen API-Key hinterlegen.
        </div>
      )}

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          {error}
        </div>
      )}

      {/* Summary Cards */}
      {holdings.length > 0 && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Gesamtwert</div>
            <div className="text-xl font-bold">{formatCurrency(totalValue)}</div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Positionen</div>
            <div className="text-xl font-bold">{holdings.length}</div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Ziel-Summe</div>
            <div className={`text-xl font-bold ${Math.abs(totalTargetWeight - 100) < 0.1 ? 'text-green-600' : 'text-amber-600'}`}>
              {totalTargetWeight.toFixed(1)}%
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Größte Position</div>
            <div className="text-xl font-bold">
              {holdings.length > 0 ? `${holdings[0].currentWeight.toFixed(1)}%` : '-'}
            </div>
          </div>
        </div>
      )}

      {/* Holdings Table */}
      <div className="bg-card rounded-lg border border-border p-4">
        <div className="flex items-center justify-between mb-4">
          <h2 className="font-semibold">Alle Positionen</h2>
          {holdings.length > 0 && (
            <span className="text-sm text-muted-foreground">
              Sortiert nach Wert
            </span>
          )}
        </div>

        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <RefreshCw className="animate-spin text-muted-foreground" size={24} />
          </div>
        ) : holdings.length === 0 ? (
          <div className="text-sm text-muted-foreground text-center py-12">
            <Scale className="w-12 h-12 mx-auto mb-3 opacity-50" />
            Keine Positionen vorhanden.
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 font-medium">Wertpapier</th>
                  <th className="text-right py-2 font-medium">Wert</th>
                  <th className="text-right py-2 font-medium">Gewinn/Verlust</th>
                  <th className="text-right py-2 font-medium">Aktuell %</th>
                  <th className="text-right py-2 font-medium w-28">Ziel %</th>
                  <th className="text-right py-2 font-medium">Diff</th>
                  <th className="text-right py-2 font-medium w-10"></th>
                </tr>
              </thead>
              <tbody>
                {holdings.map((h) => {
                  const diff = h.targetWeight - h.currentWeight;
                  const logo = logosByName.get(h.securityName);
                  return (
                    <tr key={h.securityName} className="border-b border-border last:border-0">
                      <td className="py-2">
                        <div className="flex items-center gap-2">
                          {logo ? (
                            <img
                              src={logo}
                              alt=""
                              className="w-6 h-6 rounded-md object-contain bg-white flex-shrink-0"
                            />
                          ) : (
                            <div className="w-6 h-6 rounded-md bg-muted flex items-center justify-center flex-shrink-0">
                              <Building2 size={12} className="text-muted-foreground" />
                            </div>
                          )}
                          <span className="font-medium">{h.securityName}</span>
                        </div>
                      </td>
                      <td className="py-2 text-right">{formatCurrency(h.currentValue)}</td>
                      <td className={`py-2 text-right ${h.gainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                        {h.gainLoss >= 0 ? '+' : ''}
                        {formatCurrency(h.gainLoss)}
                        <span className="text-xs ml-1">
                          ({h.gainLossPercent >= 0 ? '+' : ''}{h.gainLossPercent.toFixed(1)}%)
                        </span>
                      </td>
                      <td className="py-2 text-right text-muted-foreground">
                        {h.currentWeight.toFixed(1)}%
                      </td>
                      <td className="py-2 text-right">
                        <input
                          type="number"
                          key={`${h.securityName}-${h.targetWeight}`}
                          defaultValue={h.targetWeight.toFixed(1)}
                          onBlur={(e) => updateTargetWeight(h.securityName, parseFloat(e.target.value) || 0)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                              (e.target as HTMLInputElement).blur();
                            }
                          }}
                          step="0.1"
                          min="0"
                          max="100"
                          className="w-20 px-2 py-1 text-sm border border-border rounded bg-background text-right focus:outline-none focus:ring-2 focus:ring-primary"
                        />
                      </td>
                      <td
                        className={`py-2 text-right font-medium ${
                          Math.abs(diff) < 0.1
                            ? 'text-muted-foreground'
                            : diff > 0
                            ? 'text-green-600'
                            : 'text-red-600'
                        }`}
                      >
                        {diff > 0 ? '+' : ''}
                        {diff.toFixed(1)}%
                      </td>
                      <td className="py-2 text-right">
                        <button
                          onClick={() => openTargetModal(h)}
                          className="p-1 rounded hover:bg-muted text-muted-foreground hover:text-primary transition-colors"
                          title="Als Zielgewichtung speichern"
                        >
                          <Target size={14} />
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* AI Reasoning Panel */}
      {aiSuggestion && (
        <div className="bg-card rounded-lg border border-border overflow-hidden">
          <button
            onClick={() => setShowAiReasoning(!showAiReasoning)}
            className="w-full px-4 py-3 flex items-center justify-between hover:bg-muted/50 transition-colors"
          >
            <div className="flex items-center gap-2">
              <AIProviderLogo provider={aiProvider} size={20} />
              <span className="font-semibold">KI-Begründung</span>
              <span className="text-xs text-muted-foreground">({aiModel})</span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setAiSuggestion(null);
                }}
                className="p-1 hover:bg-muted rounded-md"
                title="Schließen"
              >
                <X size={16} className="text-muted-foreground" />
              </button>
              {showAiReasoning ? (
                <ChevronUp size={16} className="text-muted-foreground" />
              ) : (
                <ChevronDown size={16} className="text-muted-foreground" />
              )}
            </div>
          </button>
          {showAiReasoning && (
            <div className="px-4 pb-4 space-y-4">
              {/* Per-Security Reasons */}
              <div className="space-y-2">
                <h4 className="text-sm font-medium text-muted-foreground">Einzelempfehlungen</h4>
                <div className="grid gap-2">
                  {aiSuggestion.targets.map((t) => {
                    const logo = logosByName.get(t.securityName);
                    return (
                      <div
                        key={t.securityName}
                        className="flex items-start gap-3 p-2 bg-muted/30 rounded-md"
                      >
                        {logo ? (
                          <img
                            src={logo}
                            alt=""
                            className="w-5 h-5 rounded-md object-contain bg-white flex-shrink-0"
                          />
                        ) : (
                          <div className="w-5 h-5 rounded-md bg-muted flex items-center justify-center flex-shrink-0">
                            <Building2 size={10} className="text-muted-foreground" />
                          </div>
                        )}
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 text-sm">
                            <span className="font-medium truncate">{t.securityName}</span>
                            <span className="text-muted-foreground">
                              {t.currentWeight.toFixed(1)}% → {t.targetWeight.toFixed(1)}%
                            </span>
                          </div>
                          <p className="text-xs text-muted-foreground mt-0.5">{t.reason}</p>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>

              {/* Overall Reasoning */}
              <div>
                <h4 className="text-sm font-medium text-muted-foreground mb-2">Gesamtbegründung</h4>
                <div className="prose prose-sm prose-slate dark:prose-invert max-w-none">
                  <SafeMarkdown>{aiSuggestion.reasoning}</SafeMarkdown>
                </div>
              </div>

              {/* Risk Assessment */}
              <div>
                <h4 className="text-sm font-medium text-amber-600 mb-2 flex items-center gap-1">
                  <AlertTriangle size={14} />
                  Risikoeinschätzung
                </h4>
                <div className="prose prose-sm prose-slate dark:prose-invert max-w-none text-muted-foreground">
                  <SafeMarkdown>{aiSuggestion.riskAssessment}</SafeMarkdown>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Info Box */}
      {!aiSuggestion && holdings.length > 0 && !showAlertsPanel && (
        <div className="bg-muted/50 rounded-lg border border-border p-4 text-sm text-muted-foreground">
          <p className="font-medium text-foreground mb-1">So funktioniert es:</p>
          <ol className="list-decimal list-inside space-y-1">
            <li>Klicken Sie auf "KI-Vorschlag" für eine automatische Analyse</li>
            <li>Die KI schlägt optimale Zielgewichtungen vor</li>
            <li>Passen Sie die Zielwerte manuell an, falls gewünscht</li>
            <li>Klicken Sie auf "Ziel", um Warnungen bei Abweichungen zu erhalten</li>
            <li>Führen Sie die Käufe/Verkäufe manuell in Ihrem Depot aus</li>
          </ol>
        </div>
      )}

      {/* Alerts Panel */}
      {showAlertsPanel && (
        <div className="bg-card rounded-lg border border-border p-4">
          <AlertsPanel
            onAddTarget={() => openTargetModal()}
            className="min-h-[300px]"
          />
        </div>
      )}

      {/* Allocation Target Modal */}
      <AllocationTargetModal
        isOpen={targetModalOpen}
        onClose={() => {
          setTargetModalOpen(false);
          setSelectedForTarget(null);
        }}
        onSuccess={() => {
          // Refresh alerts panel if visible
          if (showAlertsPanel) {
            setShowAlertsPanel(false);
            setTimeout(() => setShowAlertsPanel(true), 100);
          }
        }}
        preSelectedSecurityId={selectedForTarget?.securityId}
        preSelectedSecurityName={selectedForTarget?.securityName}
        preSelectedWeight={selectedForTarget?.weight}
      />
    </div>
  );
}
