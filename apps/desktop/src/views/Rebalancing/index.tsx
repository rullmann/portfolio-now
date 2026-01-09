/**
 * Rebalancing view for portfolio rebalancing.
 */

import { useState, useEffect } from 'react';
import { Scale, RefreshCw, Play, TrendingUp, TrendingDown } from 'lucide-react';
import { getPortfolios, previewRebalance } from '../../lib/api';
import type { PortfolioData, RebalanceTarget, RebalancePreview } from '../../lib/types';

export function RebalancingView() {
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [selectedPortfolio, setSelectedPortfolio] = useState<number | null>(null);
  const [targets, setTargets] = useState<RebalanceTarget[]>([]);
  const [preview, setPreview] = useState<RebalancePreview | null>(null);
  const [newCash, setNewCash] = useState<number>(0);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadPortfolios = async () => {
    try {
      setIsLoading(true);
      const data = await getPortfolios();
      const active = data.filter((p) => !p.isRetired);
      setPortfolios(active);
      if (active.length > 0 && !selectedPortfolio) {
        setSelectedPortfolio(active[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadPortfolios();
  }, []);

  const handlePreviewRebalance = async () => {
    if (!selectedPortfolio || targets.length === 0) return;
    try {
      setIsLoading(true);
      const result = await previewRebalance(selectedPortfolio, targets, newCash > 0 ? newCash : undefined);
      setPreview(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const formatCurrency = (amount: number, currency: string = 'EUR') => {
    return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
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

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Settings Panel */}
        <div className="lg:col-span-1 space-y-4">
          {/* Portfolio Selection */}
          <div className="bg-card rounded-lg border border-border p-4">
            <h2 className="font-semibold mb-4">Portfolio auswählen</h2>
            <select
              value={selectedPortfolio || ''}
              onChange={(e) => setSelectedPortfolio(Number(e.target.value) || null)}
              className="w-full px-3 py-2 border border-border rounded-md bg-background"
            >
              <option value="">Portfolio wählen...</option>
              {portfolios.map(p => (
                <option key={p.id} value={p.id}>{p.name}</option>
              ))}
            </select>
          </div>

          {/* New Cash */}
          <div className="bg-card rounded-lg border border-border p-4">
            <h2 className="font-semibold mb-4">Neues Kapital (optional)</h2>
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={newCash}
                onChange={(e) => setNewCash(Number(e.target.value))}
                placeholder="0.00"
                className="flex-1 px-3 py-2 border border-border rounded-md bg-background"
              />
              <span className="text-muted-foreground">EUR</span>
            </div>
            <p className="text-xs text-muted-foreground mt-2">
              Zusätzliches Kapital, das beim Rebalancing investiert werden soll.
            </p>
          </div>

          {/* Target Allocation */}
          <div className="bg-card rounded-lg border border-border p-4">
            <h2 className="font-semibold mb-4">Ziel-Allokation</h2>

            {targets.length === 0 ? (
              <div className="text-sm text-muted-foreground text-center py-4">
                Noch keine Ziele definiert.
                <br />
                Laden Sie die aktuelle Allokation oder definieren Sie Ziele manuell.
              </div>
            ) : (
              <div className="space-y-2">
                {targets.map((target, idx) => (
                  <div key={idx} className="flex items-center gap-2">
                    <span className="flex-1 text-sm truncate">Security {target.securityId}</span>
                    <input
                      type="number"
                      value={target.targetWeight}
                      onChange={(e) => {
                        const updated = [...targets];
                        updated[idx].targetWeight = Number(e.target.value);
                        setTargets(updated);
                      }}
                      className="w-20 px-2 py-1 text-sm border border-border rounded bg-background text-right"
                    />
                    <span className="text-xs text-muted-foreground">%</span>
                  </div>
                ))}
              </div>
            )}

            <div className="flex gap-2 mt-4">
              <button
                onClick={handlePreviewRebalance}
                disabled={!selectedPortfolio || targets.length === 0 || isLoading}
                className="flex-1 px-3 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
              >
                Vorschau berechnen
              </button>
            </div>
          </div>
        </div>

        {/* Results Panel */}
        <div className="lg:col-span-2 space-y-4">
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
                  {preview.newCash && (
                    <div>
                      <div className="text-sm text-muted-foreground">Neues Kapital</div>
                      <div className="text-lg font-bold text-green-600">+{formatCurrency(preview.newCash)}</div>
                    </div>
                  )}
                  <div>
                    <div className="text-sm text-muted-foreground">Abweichung vorher</div>
                    <div className="text-lg font-bold text-amber-600">{formatPercent(preview.deviationBefore)}</div>
                  </div>
                  <div>
                    <div className="text-sm text-muted-foreground">Abweichung nachher</div>
                    <div className="text-lg font-bold text-green-600">{formatPercent(preview.deviationAfter)}</div>
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
                            <td className="py-2 font-medium">{action.securityName}</td>
                            <td className="py-2 text-center">
                              <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs ${
                                action.action === 'BUY'
                                  ? 'bg-green-500/10 text-green-600'
                                  : 'bg-red-500/10 text-red-600'
                              }`}>
                                {action.action === 'BUY' ? <TrendingUp size={12} /> : <TrendingDown size={12} />}
                                {action.action === 'BUY' ? 'Kaufen' : 'Verkaufen'}
                              </span>
                            </td>
                            <td className="py-2 text-right">{action.shares.toLocaleString('de-DE')}</td>
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

                  <div className="flex justify-end mt-4">
                    <button className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors">
                      <Play size={16} />
                      Alle ausführen
                    </button>
                  </div>
                </div>
              )}
            </>
          )}

          {/* Empty State */}
          {!preview && (
            <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
              <Scale className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Wählen Sie ein Portfolio und definieren Sie Ihre Ziel-Allokation.</p>
              <p className="text-sm mt-1">
                Dann können Sie die Rebalancing-Vorschläge berechnen.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
