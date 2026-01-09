/**
 * Transactions view for displaying transaction history.
 */

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Database, Plus, Trash2, RefreshCw, AlertCircle } from 'lucide-react';
import { useDataModeStore } from '../../store';
import { getTransactionTypeLabel } from '../../lib/types';
import { deleteTransaction } from '../../lib/api';
import { TransactionFormModal } from '../../components/modals';
import { TableSkeleton } from '../../components/common';

// Types
interface TransactionData {
  id: number;
  uuid: string;
  ownerType: string;
  ownerName: string;
  txnType: string;
  date: string;
  amount: number;
  currency: string;
  shares: number | null;
  securityName: string | null;
  securityUuid: string | null;
  note: string | null;
  fees: number;
  taxes: number;
  hasForex: boolean;
}

interface AccountTransaction {
  uuid: string;
  date: string;
  transactionType: string;
  amount: { amount: number; currency: string };
  shares?: number | null;
}

interface Account {
  uuid: string;
  name: string;
  currency: string;
  transactions: AccountTransaction[];
}

interface PortfolioFile {
  accounts?: Account[];
}

interface TransactionsViewProps {
  portfolioFile: PortfolioFile | null;
}

const POSITIVE_TYPES = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN', 'DEPOSIT', 'DIVIDENDS', 'INTEREST', 'FEES_REFUND', 'TAX_REFUND'];
const NEGATIVE_TYPES = ['SELL', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT', 'REMOVAL', 'FEES', 'TAXES', 'INTEREST_CHARGE'];

export function TransactionsView({ portfolioFile }: TransactionsViewProps) {
  const { useDbData } = useDataModeStore();
  const [dbTransactions, setDbTransactions] = useState<TransactionData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filterOwnerType, setFilterOwnerType] = useState<string>('all');
  const [filterTxnType, setFilterTxnType] = useState<string>('all');
  const [displayLimit, setDisplayLimit] = useState(100);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  // Load transactions from database
  const loadTransactions = useCallback(async () => {
    if (!useDbData) return;

    setIsLoading(true);
    setError(null);
    try {
      const transactions = await invoke<TransactionData[]>('get_transactions', {
        ownerType: null,
        ownerId: null,
        securityId: null,
        limit: 2000,
        offset: null,
      });
      setDbTransactions(transactions);
    } catch (err) {
      console.error('Failed to load transactions:', err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [useDbData]);

  useEffect(() => {
    loadTransactions();
  }, [loadTransactions]);

  const handleCreate = () => {
    setIsModalOpen(true);
  };

  const handleDelete = async (tx: TransactionData) => {
    const message = tx.securityName
      ? `Buchung "${getTransactionTypeLabel(tx.txnType)}" für ${tx.securityName} wirklich löschen?`
      : `Buchung "${getTransactionTypeLabel(tx.txnType)}" wirklich löschen?`;

    if (!confirm(message)) {
      return;
    }

    setDeletingId(tx.id);
    setError(null);

    try {
      await deleteTransaction(tx.id);
      await loadTransactions();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeletingId(null);
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
  };

  const handleModalSuccess = () => {
    loadTransactions();
  };

  // Show DB-based transactions
  if (useDbData) {
    // Filter transactions by type
    const filteredTransactions = dbTransactions.filter(tx => {
      if (filterOwnerType !== 'all' && tx.ownerType !== filterOwnerType) return false;
      if (filterTxnType !== 'all' && tx.txnType !== filterTxnType) return false;
      return true;
    });

    // Get unique transaction types for filter
    const uniqueTxnTypes = [...new Set(dbTransactions.map(tx => tx.txnType))].sort();

    return (
      <div className="space-y-4">
        {/* Header with actions */}
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">
            Buchungen ({filteredTransactions.length})
          </h2>
          <div className="flex gap-2">
            <button
              onClick={loadTransactions}
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

        {/* Filters */}
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="flex flex-wrap gap-4 items-center">
            <div className="flex items-center gap-2">
              <Database className="w-4 h-4 text-green-600" />
              <span className="text-sm text-green-600 font-medium">Aus Datenbank</span>
            </div>
            <div className="flex items-center gap-2">
              <label className="text-sm text-muted-foreground">Bereich:</label>
              <select
                value={filterOwnerType}
                onChange={(e) => setFilterOwnerType(e.target.value)}
                className="text-sm rounded-md border border-input bg-background px-2 py-1"
              >
                <option value="all">Alle</option>
                <option value="account">Konten</option>
                <option value="portfolio">Depots</option>
              </select>
            </div>
            <div className="flex items-center gap-2">
              <label className="text-sm text-muted-foreground">Typ:</label>
              <select
                value={filterTxnType}
                onChange={(e) => setFilterTxnType(e.target.value)}
                className="text-sm rounded-md border border-input bg-background px-2 py-1"
              >
                <option value="all">Alle</option>
                {uniqueTxnTypes.map(type => (
                  <option key={type} value={type}>{getTransactionTypeLabel(type)}</option>
                ))}
              </select>
            </div>
          </div>
        </div>

        {/* Transactions Table */}
        <div className="bg-card rounded-lg border border-border p-6">
          {isLoading && dbTransactions.length === 0 ? (
            <TableSkeleton rows={10} columns={9} />
          ) : filteredTransactions.length > 0 ? (
            <>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border">
                      <th className="text-left py-2 font-medium">Datum</th>
                      <th className="text-left py-2 font-medium">Typ</th>
                      <th className="text-left py-2 font-medium">Konto/Depot</th>
                      <th className="text-left py-2 font-medium">Wertpapier</th>
                      <th className="text-right py-2 font-medium">Stück</th>
                      <th className="text-right py-2 font-medium">Betrag</th>
                      <th className="text-right py-2 font-medium">Gebühren</th>
                      <th className="text-right py-2 font-medium">Steuern</th>
                      <th className="w-10"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredTransactions.slice(0, displayLimit).map((tx) => {
                      const isPositive = POSITIVE_TYPES.includes(tx.txnType);
                      const isNegative = NEGATIVE_TYPES.includes(tx.txnType);

                      return (
                        <tr key={tx.uuid} className="border-b border-border last:border-0 hover:bg-accent/30 group">
                          <td className="py-2 whitespace-nowrap">{tx.date}</td>
                          <td className="py-2">
                            <span className={`inline-block px-2 py-0.5 rounded text-xs ${
                              tx.ownerType === 'portfolio' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300' : 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'
                            }`}>
                              {getTransactionTypeLabel(tx.txnType)}
                            </span>
                          </td>
                          <td className="py-2 text-muted-foreground">{tx.ownerName}</td>
                          <td className="py-2">
                            {tx.securityName ? (
                              <span className="font-medium">{tx.securityName}</span>
                            ) : (
                              <span className="text-muted-foreground">-</span>
                            )}
                          </td>
                          <td className="py-2 text-right font-mono">
                            {tx.shares !== null ? tx.shares.toLocaleString('de-DE', { maximumFractionDigits: 6 }) : '-'}
                          </td>
                          <td className={`py-2 text-right font-mono ${isPositive ? 'text-green-600' : isNegative ? 'text-red-600' : ''}`}>
                            {tx.amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} {tx.currency}
                          </td>
                          <td className="py-2 text-right font-mono text-muted-foreground">
                            {tx.fees > 0 ? tx.fees.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '-'}
                          </td>
                          <td className="py-2 text-right font-mono text-muted-foreground">
                            {tx.taxes > 0 ? tx.taxes.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : '-'}
                          </td>
                          <td className="py-2">
                            <button
                              onClick={() => handleDelete(tx)}
                              disabled={deletingId === tx.id}
                              className="p-1.5 opacity-0 group-hover:opacity-100 hover:bg-destructive/10 rounded-md transition-all disabled:opacity-50"
                              title="Löschen"
                            >
                              <Trash2
                                size={14}
                                className={deletingId === tx.id ? 'text-muted-foreground animate-pulse' : 'text-destructive'}
                              />
                            </button>
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              {filteredTransactions.length > displayLimit && (
                <div className="text-center pt-4">
                  <button
                    onClick={() => setDisplayLimit(prev => prev + 100)}
                    className="text-sm text-primary hover:underline"
                  >
                    Mehr anzeigen ({displayLimit} von {filteredTransactions.length})
                  </button>
                </div>
              )}
            </>
          ) : (
            <p className="text-muted-foreground">Keine Buchungen gefunden.</p>
          )}
        </div>

        {/* Transaction Form Modal */}
        <TransactionFormModal
          isOpen={isModalOpen}
          onClose={handleModalClose}
          onSuccess={handleModalSuccess}
        />
      </div>
    );
  }

  if (!portfolioFile) {
    return (
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Buchungen</h2>
        <p className="text-muted-foreground">
          Öffnen Sie eine .portfolio Datei oder importieren Sie sie in die Datenbank, um Buchungen anzuzeigen.
        </p>
      </div>
    );
  }

  // Collect all transactions from all accounts
  const allTransactions: Array<AccountTransaction & { accountName: string; index: number }> = [];
  let txIndex = 0;
  for (const account of portfolioFile.accounts || []) {
    for (const tx of account.transactions || []) {
      allTransactions.push({ ...tx, accountName: account.name || 'Unbenannt', index: txIndex++ });
    }
  }

  // Sort by date descending
  allTransactions.sort((a, b) => (b.date || '').localeCompare(a.date || ''));

  return (
    <div className="bg-card rounded-lg border border-border p-6">
      <h2 className="text-lg font-semibold mb-4">Buchungen ({allTransactions.length})</h2>
      {allTransactions.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left py-2 font-medium">Datum</th>
                <th className="text-left py-2 font-medium">Typ</th>
                <th className="text-left py-2 font-medium">Konto</th>
                <th className="text-right py-2 font-medium">Betrag</th>
              </tr>
            </thead>
            <tbody>
              {allTransactions.slice(0, 50).map((tx) => (
                <tr key={tx.uuid || `tx-${tx.index}`} className="border-b border-border last:border-0">
                  <td className="py-2">{tx.date || '-'}</td>
                  <td className="py-2">{tx.transactionType || '-'}</td>
                  <td className="py-2 text-muted-foreground">{tx.accountName}</td>
                  <td className="py-2 text-right">{(tx.amount.amount / 100).toFixed(2)} {tx.amount.currency}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {allTransactions.length > 50 && (
            <div className="text-sm text-muted-foreground text-center pt-4">
              Zeige 50 von {allTransactions.length} Buchungen
            </div>
          )}
        </div>
      ) : (
        <p className="text-muted-foreground">Keine Buchungen vorhanden.</p>
      )}
    </div>
  );
}
