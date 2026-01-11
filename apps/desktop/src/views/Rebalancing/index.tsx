/**
 * Rebalancing view for portfolio rebalancing.
 */

import { useState, useEffect, useMemo } from 'react';
import { Scale, RefreshCw, Play, TrendingUp, TrendingDown, Download, AlertTriangle, CheckCircle2 } from 'lucide-react';
import { getPortfolios, getAccounts, getHoldings, previewRebalance, executeRebalance } from '../../lib/api';
import type { PortfolioData, AccountData, RebalanceTarget, RebalancePreview } from '../../lib/types';
import { toast, useSettingsStore } from '../../store';
import { SecurityLogo } from '../../components/common';
import { useCachedLogos } from '../../lib/hooks';

interface TargetWithHolding extends RebalanceTarget {
  securityName: string;
  currentShares: number;
  currentValue: number;
}

export function RebalancingView() {
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [accounts, setAccounts] = useState<AccountData[]>([]);
  const [selectedPortfolio, setSelectedPortfolio] = useState<number | null>(null);
  const [selectedAccount, setSelectedAccount] = useState<number | null>(null);
  const [targets, setTargets] = useState<TargetWithHolding[]>([]);
  const [preview, setPreview] = useState<RebalancePreview | null>(null);
  const [newCash, setNewCash] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingHoldings, setIsLoadingHoldings] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);
  const [executionSuccess, setExecutionSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { brandfetchApiKey } = useSettingsStore();

  // Prepare securities for logo loading
  const securitiesForLogos = useMemo(() =>
    targets
      .filter((t) => t.securityId !== undefined)
      .map((t) => ({
        id: t.securityId!,
        ticker: undefined,
        name: t.securityName,
      })),
    [targets]
  );

  // Load logos
  const { logos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

  const loadPortfolios = async () => {
    try {
      setIsLoading(true);
      const [portfolioData, accountData] = await Promise.all([
        getPortfolios(),
        getAccounts(),
      ]);
      const activePortfolios = portfolioData.filter((p) => !p.isRetired);
      const activeAccounts = accountData.filter((a) => !a.isRetired);
      setPortfolios(activePortfolios);
      setAccounts(activeAccounts);
      if (activePortfolios.length > 0 && !selectedPortfolio) {
        setSelectedPortfolio(activePortfolios[0].id);
      }
      if (activeAccounts.length > 0 && !selectedAccount) {
        setSelectedAccount(activeAccounts[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const loadHoldings = async (portfolioId: number) => {
    try {
      setIsLoadingHoldings(true);
      const data = await getHoldings(portfolioId);
      // Convert holdings to targets
      const totalValue = data.reduce((sum, h) => sum + (h.currentValue || 0), 0);
      const newTargets: TargetWithHolding[] = data.map((h) => ({
        securityId: h.securityId,
        securityName: h.securityName,
        targetWeight: totalValue > 0 ? ((h.currentValue || 0) / totalValue) * 100 : 0,
        currentWeight: totalValue > 0 ? ((h.currentValue || 0) / totalValue) * 100 : 0,
        currentValue: h.currentValue || 0,
        currentShares: h.shares,
      }));
      setTargets(newTargets);
      setPreview(null);
    } catch (err) {
      console.error('Failed to load holdings:', err);
    } finally {
      setIsLoadingHoldings(false);
    }
  };

  useEffect(() => {
    loadPortfolios();
  }, []);

  useEffect(() => {
    if (selectedPortfolio) {
      loadHoldings(selectedPortfolio);
    }
  }, [selectedPortfolio]);

  // Calculate total target weight
  const totalTargetWeight = useMemo(() => {
    return targets.reduce((sum, t) => sum + t.targetWeight, 0);
  }, [targets]);

  const handlePreviewRebalance = async () => {
    if (!selectedPortfolio || targets.length === 0) return;
    try {
      setIsLoading(true);
      setError(null);
      const rebalanceTargets: RebalanceTarget[] = targets.map((t) => ({
        securityId: t.securityId,
        targetWeight: t.targetWeight,
      }));
      const newCashValue = newCash ? parseFloat(newCash) * 100 : undefined;
      const result = await previewRebalance(selectedPortfolio, rebalanceTargets, newCashValue);
      setPreview(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleExecuteRebalance = async () => {
    if (!selectedPortfolio || !selectedAccount || !preview) return;
    try {
      setIsExecuting(true);
      setError(null);
      const transactionCount = await executeRebalance(
        selectedPortfolio,
        selectedAccount,
        preview.actions
      );
      setExecutionSuccess(true);
      setShowConfirmDialog(false);
      toast.success(`${transactionCount} Transaktionen erstellt`);
      // Reload after execution
      setTimeout(() => {
        setExecutionSuccess(false);
        setPreview(null);
        loadHoldings(selectedPortfolio);
      }, 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsExecuting(false);
    }
  };

  const updateTargetWeight = (index: number, weight: number) => {
    const updated = [...targets];
    updated[index] = { ...updated[index], targetWeight: weight };
    setTargets(updated);
  };

  const formatCurrency = (amount: number, currency: string = 'EUR') => {
    return `${(amount / 100).toLocaleString('de-DE', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    })} ${currency}`;
  };

  const formatPercent = (value: number) => {
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Scale className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Rebalancing</h1>
        </div>
        <button
          onClick={loadPortfolios}
          disabled={isLoading}
          className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
        >
          <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
          Aktualisieren
        </button>
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          {error}
        </div>
      )}

      {executionSuccess && (
        <div className="p-3 bg-green-500/10 border border-green-500/20 rounded-md text-green-600 text-sm flex items-center gap-2">
          <CheckCircle2 size={16} />
          Rebalancing erfolgreich ausgeführt!
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Settings Panel */}
        <div className="lg:col-span-1 space-y-4">
          {/* Portfolio Selection */}
          <div className="bg-card rounded-lg border border-border p-4">
            <h2 className="font-semibold mb-4">Portfolio</h2>
            <select
              value={selectedPortfolio || ''}
              onChange={(e) => setSelectedPortfolio(Number(e.target.value) || null)}
              className="w-full px-3 py-2 border border-border rounded-md bg-background"
            >
              <option value="">Portfolio wählen...</option>
              {portfolios.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </div>

          {/* Account Selection */}
          <div className="bg-card rounded-lg border border-border p-4">
            <h2 className="font-semibold mb-4">Verrechnungskonto</h2>
            <select
              value={selectedAccount || ''}
              onChange={(e) => setSelectedAccount(Number(e.target.value) || null)}
              className="w-full px-3 py-2 border border-border rounded-md bg-background"
            >
              <option value="">Konto wählen...</option>
              {accounts.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name} ({a.currency})
                </option>
              ))}
            </select>
            <p className="text-xs text-muted-foreground mt-2">
              Für Käufe und Verkäufe.
            </p>
          </div>

          {/* New Cash */}
          <div className="bg-card rounded-lg border border-border p-4">
            <h2 className="font-semibold mb-4">Neues Kapital</h2>
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={newCash}
                onChange={(e) => setNewCash(e.target.value)}
                placeholder="0.00"
                step="0.01"
                className="flex-1 px-3 py-2 border border-border rounded-md bg-background"
              />
              <span className="text-muted-foreground">EUR</span>
            </div>
            <p className="text-xs text-muted-foreground mt-2">
              Optional: Zusätzliches Kapital zum Investieren.
            </p>
          </div>
        </div>

        {/* Target Allocation & Results */}
        <div className="lg:col-span-2 space-y-4">
          {/* Target Allocation */}
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="flex items-center justify-between mb-4">
              <h2 className="font-semibold">Ziel-Allokation</h2>
              <div className={`text-sm font-medium ${Math.abs(totalTargetWeight - 100) < 0.01 ? 'text-green-600' : 'text-amber-600'}`}>
                Summe: {totalTargetWeight.toFixed(1)}%
              </div>
            </div>

            {isLoadingHoldings ? (
              <div className="flex items-center justify-center py-8">
                <RefreshCw className="animate-spin text-muted-foreground" size={24} />
              </div>
            ) : targets.length === 0 ? (
              <div className="text-sm text-muted-foreground text-center py-8">
                <Scale className="w-12 h-12 mx-auto mb-3 opacity-50" />
                Keine Positionen im Portfolio.
                <br />
                Wählen Sie ein Portfolio mit Beständen.
              </div>
            ) : (
              <>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-border">
                        <th className="text-left py-2 font-medium">Wertpapier</th>
                        <th className="text-right py-2 font-medium">Aktuell</th>
                        <th className="text-right py-2 font-medium w-28">Ziel %</th>
                        <th className="text-right py-2 font-medium">Diff</th>
                      </tr>
                    </thead>
                    <tbody>
                      {targets.map((target, idx) => {
                        const diff = target.targetWeight - (target.currentWeight || 0);
                        return (
                          <tr key={idx} className="border-b border-border last:border-0">
                            <td className="py-2">
                              <div className="flex items-center gap-2">
                                {target.securityId && <SecurityLogo securityId={target.securityId} logos={logos} size={24} />}
                                <span className="font-medium">{target.securityName}</span>
                              </div>
                            </td>
                            <td className="py-2 text-right text-muted-foreground">
                              {(target.currentWeight || 0).toFixed(1)}%
                            </td>
                            <td className="py-2 text-right">
                              <input
                                type="number"
                                value={target.targetWeight.toFixed(1)}
                                onChange={(e) =>
                                  updateTargetWeight(idx, parseFloat(e.target.value) || 0)
                                }
                                step="0.1"
                                min="0"
                                max="100"
                                className="w-20 px-2 py-1 text-sm border border-border rounded bg-background text-right"
                              />
                            </td>
                            <td className={`py-2 text-right font-medium ${diff > 0 ? 'text-green-600' : diff < 0 ? 'text-red-600' : 'text-muted-foreground'}`}>
                              {diff > 0 ? '+' : ''}{diff.toFixed(1)}%
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>

                <div className="flex gap-2 mt-4">
                  <button
                    onClick={handlePreviewRebalance}
                    disabled={!selectedPortfolio || targets.length === 0 || isLoading}
                    className="flex-1 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
                  >
                    {isLoading ? 'Berechne...' : 'Vorschau berechnen'}
                  </button>
                </div>
              </>
            )}
          </div>

          {/* Preview Results */}
          {preview && (
            <>
              {/* Summary */}
              <div className="bg-card rounded-lg border border-border p-4">
                <h2 className="font-semibold mb-4">Rebalancing Vorschau</h2>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                  <div>
                    <div className="text-sm text-muted-foreground">Portfolio-Wert</div>
                    <div className="text-lg font-bold">{formatCurrency(preview.totalValue)}</div>
                  </div>
                  {preview.newCash && preview.newCash > 0 && (
                    <div>
                      <div className="text-sm text-muted-foreground">Neues Kapital</div>
                      <div className="text-lg font-bold text-green-600">
                        +{formatCurrency(preview.newCash)}
                      </div>
                    </div>
                  )}
                  <div>
                    <div className="text-sm text-muted-foreground">Abweichung vorher</div>
                    <div className="text-lg font-bold text-amber-600">
                      {formatPercent(preview.deviationBefore)}
                    </div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Abweichung nachher</div>
                    <div className="text-lg font-bold text-green-600">
                      {formatPercent(preview.deviationAfter)}
                    </div>
                  </div>
                </div>
              </div>

              {/* Actions */}
              {preview.actions.length > 0 && (
                <div className="bg-card rounded-lg border border-border p-4">
                  <h2 className="font-semibold mb-4">Vorgeschlagene Transaktionen</h2>
                  <div className="overflow-x-auto">
                    <table className="w-full text-sm">
                      <thead>
                        <tr className="border-b border-border">
                          <th className="text-left py-2 font-medium">Wertpapier</th>
                          <th className="text-center py-2 font-medium">Aktion</th>
                          <th className="text-right py-2 font-medium">Stück</th>
                          <th className="text-right py-2 font-medium">Betrag</th>
                          <th className="text-right py-2 font-medium">Gewichtung</th>
                        </tr>
                      </thead>
                      <tbody>
                        {preview.actions.map((action, idx) => (
                          <tr key={idx} className="border-b border-border last:border-0">
                            <td className="py-2">
                              <div className="flex items-center gap-2">
                                <SecurityLogo securityId={action.securityId} logos={logos} size={24} />
                                <span className="font-medium">{action.securityName}</span>
                              </div>
                            </td>
                            <td className="py-2 text-center">
                              <span
                                className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs ${
                                  action.action === 'BUY'
                                    ? 'bg-green-500/10 text-green-600'
                                    : 'bg-red-500/10 text-red-600'
                                }`}
                              >
                                {action.action === 'BUY' ? (
                                  <TrendingUp size={12} />
                                ) : (
                                  <TrendingDown size={12} />
                                )}
                                {action.action === 'BUY' ? 'Kaufen' : 'Verkaufen'}
                              </span>
                            </td>
                            <td className="py-2 text-right">
                              {action.shares.toLocaleString('de-DE', { maximumFractionDigits: 4 })}
                            </td>
                            <td className="py-2 text-right font-medium">
                              {formatCurrency(action.amount)}
                            </td>
                            <td className="py-2 text-right text-muted-foreground">
                              {action.currentWeight.toFixed(1)}% → {action.targetWeight.toFixed(1)}%
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>

                  <div className="flex justify-end gap-3 mt-4 pt-4 border-t border-border">
                    <button
                      onClick={() => setPreview(null)}
                      className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
                    >
                      Abbrechen
                    </button>
                    <button
                      onClick={() => setShowConfirmDialog(true)}
                      disabled={!selectedAccount}
                      className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
                    >
                      <Play size={16} />
                      Alle ausführen
                    </button>
                  </div>
                </div>
              )}

              {preview.actions.length === 0 && (
                <div className="bg-card rounded-lg border border-border p-8 text-center">
                  <CheckCircle2 className="w-12 h-12 mx-auto mb-3 text-green-500" />
                  <p className="font-medium">Portfolio ist bereits ausgeglichen</p>
                  <p className="text-sm text-muted-foreground mt-1">
                    Es sind keine Transaktionen erforderlich.
                  </p>
                </div>
              )}
            </>
          )}

          {/* Empty State */}
          {!preview && targets.length > 0 && (
            <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
              <Download className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Passen Sie die Ziel-Gewichtungen an.</p>
              <p className="text-sm mt-1">
                Klicken Sie dann auf "Vorschau berechnen".
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Confirmation Dialog */}
      {showConfirmDialog && preview && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="absolute inset-0 bg-black/50" onClick={() => setShowConfirmDialog(false)} />
          <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md mx-4 p-6">
            <div className="flex items-start gap-3 mb-4">
              <AlertTriangle className="text-amber-500 mt-0.5" size={24} />
              <div>
                <h3 className="font-semibold text-lg">Rebalancing bestätigen</h3>
                <p className="text-sm text-muted-foreground mt-1">
                  Es werden {preview.actions.length} Transaktionen erstellt.
                </p>
              </div>
            </div>

            <div className="bg-muted rounded-md p-3 mb-4 text-sm">
              <div className="flex justify-between mb-1">
                <span>Käufe:</span>
                <span className="font-medium text-green-600">
                  {preview.actions.filter((a) => a.action === 'BUY').length}
                </span>
              </div>
              <div className="flex justify-between">
                <span>Verkäufe:</span>
                <span className="font-medium text-red-600">
                  {preview.actions.filter((a) => a.action === 'SELL').length}
                </span>
              </div>
            </div>

            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowConfirmDialog(false)}
                className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
              >
                Abbrechen
              </button>
              <button
                onClick={handleExecuteRebalance}
                disabled={isExecuting}
                className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
              >
                {isExecuting ? 'Ausführen...' : 'Bestätigen'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
