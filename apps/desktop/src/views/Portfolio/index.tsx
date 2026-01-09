/**
 * Portfolio view for displaying and managing portfolios.
 */

import { useState, useEffect, useCallback } from 'react';
import { Plus, Pencil, Trash2, AlertCircle, RefreshCw, Briefcase } from 'lucide-react';
import type { PortfolioData } from '../../lib/types';
import { getPortfolios, deletePPPortfolio, getPortfolioHistory } from '../../lib/api';
import { PortfolioFormModal } from '../../components/modals';
import { TradingViewPortfolioChart } from '../../components/charts';

// Legacy types for direct file viewing
interface LegacyPortfolioTransaction {
  uuid: string;
  date: string;
  transactionType: string;
  amount: { amount: number; currency: string };
  shares: number;
  securityUuid?: string | null;
}

interface LegacyPortfolio {
  uuid: string;
  name: string;
  referenceAccountUuid?: string | null;
  transactions: LegacyPortfolioTransaction[];
}

interface PortfolioFile {
  portfolios?: LegacyPortfolio[];
}

interface PortfolioViewProps {
  portfolioFile: PortfolioFile | null;
  dbPortfolios?: PortfolioData[]; // Optional for backward compatibility
}

export function PortfolioView({ portfolioFile, dbPortfolios: initialDbPortfolios }: PortfolioViewProps) {
  const [dbPortfolios, setDbPortfolios] = useState<PortfolioData[]>(initialDbPortfolios || []);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingPortfolio, setEditingPortfolio] = useState<PortfolioData | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [portfolioHistory, setPortfolioHistory] = useState<Array<{ date: string; value: number }>>([]);

  const loadPortfolios = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [data, history] = await Promise.all([
        getPortfolios(),
        getPortfolioHistory().catch(() => []),
      ]);
      setDbPortfolios(data);
      setPortfolioHistory(history);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

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

  // Use DB data if available, otherwise fall back to legacy file data
  const hasDbData = dbPortfolios.length > 0;
  const legacyPortfolios = portfolioFile?.portfolios || [];

  return (
    <div className="space-y-4">
      {/* Header with actions */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">
          Portfolios ({hasDbData ? dbPortfolios.length : legacyPortfolios.length})
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

      {/* Portfolio Value Chart */}
      {portfolioHistory.length > 0 && (
        <div className="bg-card rounded-lg border border-border p-6">
          <TradingViewPortfolioChart data={portfolioHistory} height={280} />
        </div>
      )}

      {/* Main content */}
      {isLoading && dbPortfolios.length === 0 ? (
        <div className="bg-card rounded-lg border border-border p-6 text-center text-muted-foreground">
          Lade Portfolios...
        </div>
      ) : hasDbData ? (
        /* Database portfolios grid */
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {dbPortfolios.map((portfolio) => (
            <div
              key={portfolio.id}
              className="bg-card rounded-lg border border-border p-4 hover:border-primary/50 transition-colors"
            >
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

              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Positionen</span>
                  <span className="font-medium tabular-nums">{portfolio.holdingsCount}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Transaktionen</span>
                  <span className="tabular-nums">{portfolio.transactionsCount}</span>
                </div>
                {portfolio.isRetired && (
                  <div className="pt-2">
                    <span className="px-2 py-0.5 text-xs bg-muted rounded-full text-muted-foreground">
                      Inaktiv
                    </span>
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : legacyPortfolios.length > 0 ? (
        /* Legacy file portfolios */
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {legacyPortfolios.map((portfolio, index) => (
            <div
              key={portfolio.uuid || `portfolio-${index}`}
              className="bg-card rounded-lg border border-border p-4"
            >
              <div className="flex items-center gap-3 mb-3">
                <div className="p-2 bg-primary/10 rounded-lg">
                  <Briefcase size={20} className="text-primary" />
                </div>
                <div>
                  <h3 className="font-medium">{portfolio.name || 'Unbenannt'}</h3>
                </div>
              </div>
              <div className="text-sm text-muted-foreground">
                {portfolio.transactions?.length || 0} Transaktionen
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="bg-card rounded-lg border border-border p-6 text-center text-muted-foreground">
          Keine Portfolios vorhanden. Importieren Sie eine .portfolio Datei oder erstellen Sie ein neues Portfolio.
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
