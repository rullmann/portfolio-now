/**
 * Portfolio view for displaying and managing portfolios.
 * Extended with performance metrics and quick actions.
 */

import { useState, useEffect, useCallback } from 'react';
import { Plus, Pencil, Trash2, AlertCircle, RefreshCw, Briefcase, PieChart, ArrowRightLeft, BarChart3 } from 'lucide-react';
import type { PortfolioData } from '../../lib/types';
import { formatCurrency, formatNumber } from '../../lib/types';
import { getPortfolios, deletePPPortfolio, getHoldings } from '../../lib/api';
import { PortfolioFormModal } from '../../components/modals';
import { PortfolioCardSkeleton } from '../../components/common/Skeleton';
import { useUIStore, useSettingsStore } from '../../store';

/** Extended portfolio data with calculated metrics */
interface PortfolioWithMetrics extends PortfolioData {
  totalValue: number;
  totalCost: number;
  gainLoss: number;
  gainLossPercent: number;
  ttwror: number | null;
  currency: string;
  metricsLoaded: boolean;
  metricsError?: string;
}

interface PortfolioViewProps {
  dbPortfolios?: PortfolioData[];
}

export function PortfolioView({ dbPortfolios: _initialDbPortfolios }: PortfolioViewProps) {
  const [portfolios, setPortfolios] = useState<PortfolioWithMetrics[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingPortfolio, setEditingPortfolio] = useState<PortfolioData | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  const setCurrentView = useUIStore((s) => s.setCurrentView);
  const setScrollTarget = useUIStore((s) => s.setScrollTarget);
  const baseCurrency = useSettingsStore((s) => s.baseCurrency);

  /** Load metrics for a single portfolio - only holdings, no TTWROR for speed */
  const loadPortfolioMetrics = async (portfolio: PortfolioData): Promise<PortfolioWithMetrics> => {
    try {
      // Only load holdings - skip expensive TTWROR calculation for overview
      const holdings = await getHoldings(portfolio.id);

      const totalValue = holdings.reduce((sum, h) => sum + (h.currentValue ?? 0), 0);
      const totalCost = holdings.reduce((sum, h) => sum + h.costBasis, 0);
      const gainLoss = totalValue - totalCost;
      const gainLossPercent = totalCost > 0 ? ((totalValue - totalCost) / totalCost) * 100 : 0;

      return {
        ...portfolio,
        totalValue,
        totalCost,
        gainLoss,
        gainLossPercent,
        ttwror: null, // Skip TTWROR for faster loading - use G/V% instead
        currency: baseCurrency,
        metricsLoaded: true,
      };
    } catch (err) {
      // Return portfolio without metrics on error
      return {
        ...portfolio,
        totalValue: 0,
        totalCost: 0,
        gainLoss: 0,
        gainLossPercent: 0,
        ttwror: null,
        currency: baseCurrency,
        metricsLoaded: true,
        metricsError: err instanceof Error ? err.message : String(err),
      };
    }
  };

  const loadPortfolios = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getPortfolios();

      // Initialize portfolios without metrics
      const initialPortfolios: PortfolioWithMetrics[] = data.map((p) => ({
        ...p,
        totalValue: 0,
        totalCost: 0,
        gainLoss: 0,
        gainLossPercent: 0,
        ttwror: null,
        currency: baseCurrency,
        metricsLoaded: false,
      }));
      setPortfolios(initialPortfolios);
      setIsLoading(false);

      // Load metrics sequentially to avoid SQLite contention
      // Each portfolio updates immediately after loading
      const results: PortfolioWithMetrics[] = [...initialPortfolios];
      for (let i = 0; i < data.length; i++) {
        const withMetrics = await loadPortfolioMetrics(data[i]);
        results[i] = withMetrics;
        setPortfolios([...results]); // Progressive update
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setIsLoading(false);
    }
  }, [baseCurrency]);

  useEffect(() => {
    loadPortfolios();
  }, [loadPortfolios]);

  const handleCreate = () => {
    setEditingPortfolio(null);
    setIsModalOpen(true);
  };

  const handleEdit = (portfolio: PortfolioData) => {
    setEditingPortfolio(portfolio);
    setIsModalOpen(true);
  };

  const handleDelete = async (portfolio: PortfolioData) => {
    if (!confirm(`Portfolio "${portfolio.name}" wirklich löschen?`)) {
      return;
    }

    setDeletingId(portfolio.id);
    setError(null);

    try {
      await deletePPPortfolio(portfolio.id);
      await loadPortfolios();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeletingId(null);
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setEditingPortfolio(null);
  };

  const handleModalSuccess = () => {
    loadPortfolios();
  };

  /** Navigate to another view with portfolio filter */
  const handleQuickAction = (action: 'holdings' | 'transactions' | 'reports', portfolioId: number) => {
    switch (action) {
      case 'holdings':
        setCurrentView('holdings');
        setScrollTarget(`portfolio:${portfolioId}`);
        break;
      case 'transactions':
        setCurrentView('transactions');
        setScrollTarget(`portfolio:${portfolioId}`);
        break;
      case 'reports':
        setCurrentView('reports');
        setScrollTarget(`portfolio:${portfolioId}`);
        break;
    }
  };

  return (
    <div className="space-y-4">
      {/* Header with actions */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">
          Portfolios ({portfolios.length})
        </h2>
        <div className="flex gap-2">
          <button
            onClick={loadPortfolios}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
          <button
            onClick={handleCreate}
            className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
          >
            <Plus size={16} />
            Neu
          </button>
        </div>
      </div>

      {/* Error message */}
      {error && (
        <div className="flex items-center gap-2 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          <AlertCircle size={16} />
          {error}
        </div>
      )}

      {/* Main content */}
      {isLoading && portfolios.length === 0 ? (
        /* Loading skeleton */
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          <PortfolioCardSkeleton />
          <PortfolioCardSkeleton />
          <PortfolioCardSkeleton />
        </div>
      ) : portfolios.length > 0 ? (
        /* Portfolios grid */
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {portfolios.map((portfolio) => (
            <div
              key={portfolio.id}
              className="bg-card rounded-lg border border-border p-4 hover:border-primary/50 transition-colors"
            >
              {/* Header */}
              <div className="flex items-start justify-between mb-3">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-primary/10 rounded-lg">
                    <Briefcase size={20} className="text-primary" />
                  </div>
                  <div>
                    <h3 className={`font-medium ${portfolio.isRetired ? 'text-muted-foreground line-through' : ''}`}>
                      {portfolio.name}
                    </h3>
                    {portfolio.referenceAccountName && (
                      <p className="text-sm text-muted-foreground">
                        Ref: {portfolio.referenceAccountName}
                      </p>
                    )}
                  </div>
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={() => handleEdit(portfolio)}
                    className="p-1.5 hover:bg-muted rounded-md transition-colors"
                    title="Bearbeiten"
                  >
                    <Pencil size={16} className="text-muted-foreground" />
                  </button>
                  <button
                    onClick={() => handleDelete(portfolio)}
                    disabled={deletingId === portfolio.id}
                    className="p-1.5 hover:bg-destructive/10 rounded-md transition-colors disabled:opacity-50"
                    title="Löschen"
                  >
                    <Trash2
                      size={16}
                      className={
                        deletingId === portfolio.id
                          ? 'text-muted-foreground animate-pulse'
                          : 'text-destructive'
                      }
                    />
                  </button>
                </div>
              </div>

              {/* Metrics section */}
              {portfolio.metricsLoaded && !portfolio.metricsError ? (
                <div className="space-y-2 py-3 border-t border-border">
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Wert</span>
                    <span className="font-medium tabular-nums">
                      {formatCurrency(portfolio.totalValue, portfolio.currency)}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Einstand</span>
                    <span className="tabular-nums">
                      {formatCurrency(portfolio.totalCost, portfolio.currency)}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">G/V</span>
                    <span className={`font-medium tabular-nums ${
                      portfolio.gainLoss >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}>
                      {portfolio.gainLoss >= 0 ? '+' : ''}{formatCurrency(portfolio.gainLoss, portfolio.currency)}
                      {' '}
                      <span className="text-xs">
                        ({portfolio.gainLossPercent >= 0 ? '+' : ''}{formatNumber(portfolio.gainLossPercent, 1)}%)
                      </span>
                    </span>
                  </div>
                  {portfolio.ttwror !== null && (
                    <div className="flex justify-between text-sm">
                      <span className="text-muted-foreground">TTWROR</span>
                      <span className={`font-medium tabular-nums ${
                        portfolio.ttwror >= 0 ? 'text-green-600' : 'text-red-600'
                      }`}>
                        {portfolio.ttwror >= 0 ? '+' : ''}{formatNumber(portfolio.ttwror, 2)}%
                      </span>
                    </div>
                  )}
                </div>
              ) : portfolio.metricsError ? (
                <div className="py-3 border-t border-border">
                  <p className="text-xs text-muted-foreground">
                    Kennzahlen nicht verfügbar
                  </p>
                </div>
              ) : (
                /* Metrics loading */
                <div className="space-y-2 py-3 border-t border-border">
                  <div className="flex justify-between">
                    <div className="h-4 w-12 bg-muted rounded animate-pulse" />
                    <div className="h-4 w-24 bg-muted rounded animate-pulse" />
                  </div>
                  <div className="flex justify-between">
                    <div className="h-4 w-14 bg-muted rounded animate-pulse" />
                    <div className="h-4 w-20 bg-muted rounded animate-pulse" />
                  </div>
                  <div className="flex justify-between">
                    <div className="h-4 w-8 bg-muted rounded animate-pulse" />
                    <div className="h-4 w-28 bg-muted rounded animate-pulse" />
                  </div>
                </div>
              )}

              {/* Position/Transaction counts */}
              <div className="py-3 border-t border-border">
                <p className="text-sm text-muted-foreground">
                  {portfolio.holdingsCount} {portfolio.holdingsCount === 1 ? 'Position' : 'Positionen'} · {portfolio.transactionsCount} {portfolio.transactionsCount === 1 ? 'Transaktion' : 'Transaktionen'}
                </p>
                {portfolio.isRetired && (
                  <span className="inline-block mt-2 px-2 py-0.5 text-xs bg-muted rounded-full text-muted-foreground">
                    Inaktiv
                  </span>
                )}
              </div>

              {/* Quick Actions */}
              <div className="flex gap-2 pt-3 border-t border-border">
                <button
                  onClick={() => handleQuickAction('holdings', portfolio.id)}
                  className="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs font-medium border border-border rounded hover:bg-muted transition-colors"
                  title="Bestand anzeigen"
                >
                  <PieChart size={14} />
                  Bestand
                </button>
                <button
                  onClick={() => handleQuickAction('transactions', portfolio.id)}
                  className="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs font-medium border border-border rounded hover:bg-muted transition-colors"
                  title="Transaktionen anzeigen"
                >
                  <ArrowRightLeft size={14} />
                  Buchungen
                </button>
                <button
                  onClick={() => handleQuickAction('reports', portfolio.id)}
                  className="flex-1 flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs font-medium border border-border rounded hover:bg-muted transition-colors"
                  title="Berichte anzeigen"
                >
                  <BarChart3 size={14} />
                  Reports
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="bg-card rounded-lg border border-border p-6 text-center text-muted-foreground">
          Keine Portfolios vorhanden. Erstellen Sie ein neues Portfolio oder importieren Sie eine PP-Datei.
        </div>
      )}

      {/* Portfolio Form Modal */}
      <PortfolioFormModal
        isOpen={isModalOpen}
        onClose={handleModalClose}
        onSuccess={handleModalSuccess}
        portfolio={editingPortfolio}
      />
    </div>
  );
}
