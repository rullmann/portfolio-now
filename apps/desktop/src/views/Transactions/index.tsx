/**
 * Transactions view for displaying transaction history.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Database, Plus, Trash2, AlertCircle, FileText, Pencil, CheckSquare, X } from 'lucide-react';
import { getTransactionTypeLabel, formatDate } from '../../lib/types';
import { deleteTransaction, deleteTransactionsBulk, getSecurities } from '../../lib/api';
import { TransactionFormModal, PdfImportModal, BulkDeleteConfirmModal } from '../../components/modals';
import { TableSkeleton, SecurityLogo } from '../../components/common';
import { useCachedLogos } from '../../lib/hooks';
import { useSettingsStore } from '../../store';

// Types
interface TransactionData {
  id: number;
  uuid: string;
  ownerType: string;
  ownerId: number;
  ownerName: string;
  txnType: string;
  date: string;
  amount: number;
  currency: string;
  shares?: number;
  securityId?: number;
  securityName?: string;
  securityUuid?: string;
  note?: string;
  fees: number;
  taxes: number;
  hasForex: boolean;
}

const POSITIVE_TYPES = ['BUY', 'DELIVERY_INBOUND', 'TRANSFER_IN', 'DEPOSIT', 'DIVIDENDS', 'INTEREST', 'FEES_REFUND', 'TAX_REFUND'];
const NEGATIVE_TYPES = ['SELL', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT', 'REMOVAL', 'FEES', 'TAXES', 'INTEREST_CHARGE'];

interface SecurityInfo {
  id: number;
  uuid: string;
  name: string;
  ticker: string | null | undefined;
}

export function TransactionsView() {
  const [dbTransactions, setDbTransactions] = useState<TransactionData[]>([]);
  const [securities, setSecurities] = useState<SecurityInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filterOwnerType, setFilterOwnerType] = useState<string>('all');
  const [filterTxnType, setFilterTxnType] = useState<string>('all');
  const [displayLimit, setDisplayLimit] = useState(100);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isPdfImportOpen, setIsPdfImportOpen] = useState(false);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [editingTransaction, setEditingTransaction] = useState<TransactionData | null>(null);
  const { brandfetchApiKey } = useSettingsStore();

  // Selection mode state
  const [isSelectionMode, setIsSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());
  const [showBulkDeleteModal, setShowBulkDeleteModal] = useState(false);
  const [isBulkDeleting, setIsBulkDeleting] = useState(false);

  // Map UUID to security ID for logo lookup
  const securityUuidToId = useMemo(() => {
    const map = new Map<string, number>();
    securities.forEach((s) => map.set(s.uuid, s.id));
    return map;
  }, [securities]);

  // Prepare securities for logo loading
  const securitiesForLogos = useMemo(() =>
    securities.map((s) => ({
      id: s.id,
      ticker: s.ticker || undefined,
      name: s.name,
    })),
    [securities]
  );

  // Load logos
  const { logos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

  // Load transactions and securities from database
  const loadTransactions = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [transactions, secs] = await Promise.all([
        invoke<TransactionData[]>('get_transactions', {
          ownerType: null,
          ownerId: null,
          securityId: null,
          limit: 2000,
          offset: null,
        }),
        getSecurities(),
      ]);
      setDbTransactions(transactions);
      setSecurities(secs.map((s) => ({ id: s.id, uuid: s.uuid, name: s.name, ticker: s.ticker })));
    } catch (err) {
      console.error('Failed to load transactions:', err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadTransactions();
  }, [loadTransactions]);

  // Listen for data_changed events to auto-refresh transactions
  useEffect(() => {
    const unlisten = listen<{ entity: string; action: string }>('data_changed', (event) => {
      // Refresh when transactions or related data changes
      const relevantEntities = ['transaction', 'transactions', 'import', 'rebalance', 'investment_plan'];
      if (relevantEntities.some((e) => event.payload.entity?.includes(e))) {
        loadTransactions();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadTransactions]);

  const handleCreate = () => {
    setEditingTransaction(null);
    setIsModalOpen(true);
  };

  const handleEdit = (tx: TransactionData) => {
    setEditingTransaction(tx);
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
    setEditingTransaction(null);
  };

  const handleModalSuccess = () => {
    loadTransactions();
  };

  // Selection mode handlers
  const toggleSelectionMode = () => {
    setIsSelectionMode(!isSelectionMode);
    setSelectedIds(new Set());
  };

  const handleToggleSelection = (id: number) => {
    const newSelected = new Set(selectedIds);
    if (newSelected.has(id)) {
      newSelected.delete(id);
    } else {
      newSelected.add(id);
    }
    setSelectedIds(newSelected);
  };

  const handleSelectAll = () => {
    const visibleIds = filteredTransactions.slice(0, displayLimit).map((tx) => tx.id);
    setSelectedIds(new Set(visibleIds));
  };

  const handleDeselectAll = () => {
    setSelectedIds(new Set());
  };

  const handleBulkDelete = async () => {
    if (selectedIds.size === 0) return;

    setIsBulkDeleting(true);
    setError(null);

    try {
      const result = await deleteTransactionsBulk(Array.from(selectedIds));
      console.log('Bulk delete result:', result);
      setShowBulkDeleteModal(false);
      setSelectedIds(new Set());
      setIsSelectionMode(false);
      await loadTransactions();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsBulkDeleting(false);
    }
  };

  // Filter transactions by type
  const filteredTransactions = useMemo(() => {
    return dbTransactions.filter(tx => {
      if (filterOwnerType !== 'all' && tx.ownerType !== filterOwnerType) return false;
      if (filterTxnType !== 'all' && tx.txnType !== filterTxnType) return false;
      return true;
    });
  }, [dbTransactions, filterOwnerType, filterTxnType]);

  // Get unique transaction types for filter
  const uniqueTxnTypes = useMemo(() => {
    return [...new Set(dbTransactions.map(tx => tx.txnType))].sort();
  }, [dbTransactions]);

  // Get selected transactions for the modal
  const selectedTransactions = useMemo(() => {
    return filteredTransactions.filter((tx) => selectedIds.has(tx.id));
  }, [filteredTransactions, selectedIds]);

  // Check if all visible transactions are selected
  const allVisibleSelected = useMemo(() => {
    const visibleIds = filteredTransactions.slice(0, displayLimit).map((tx) => tx.id);
    return visibleIds.length > 0 && visibleIds.every((id) => selectedIds.has(id));
  }, [filteredTransactions, displayLimit, selectedIds]);

    return (
      <div className="space-y-4">
        {/* Header with actions */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h2 className="text-lg font-semibold">
              Buchungen ({filteredTransactions.length})
            </h2>
            {/* Selection mode controls */}
            {isSelectionMode && (
              <div className="flex items-center gap-2">
                <button
                  onClick={allVisibleSelected ? handleDeselectAll : handleSelectAll}
                  className="text-sm text-primary hover:underline"
                >
                  {allVisibleSelected ? 'Auswahl aufheben' : 'Alle auswählen'}
                </button>
                <span className="text-sm text-muted-foreground">
                  {selectedIds.size} ausgewählt
                </span>
                {selectedIds.size > 0 && (
                  <button
                    onClick={() => setShowBulkDeleteModal(true)}
                    className="flex items-center gap-1 px-3 py-1.5 text-sm bg-destructive text-destructive-foreground rounded-md hover:bg-destructive/90 transition-colors"
                  >
                    <Trash2 size={14} />
                    Löschen ({selectedIds.size})
                  </button>
                )}
              </div>
            )}
          </div>
          <div className="flex gap-2">
            {/* Selection mode toggle */}
            <button
              onClick={toggleSelectionMode}
              className={`flex items-center gap-2 px-3 py-1.5 text-sm border rounded-md transition-colors ${
                isSelectionMode
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'border-border hover:bg-muted'
              }`}
            >
              {isSelectionMode ? <X size={16} /> : <CheckSquare size={16} />}
              {isSelectionMode ? 'Abbrechen' : 'Auswählen'}
            </button>
            <button
              onClick={() => setIsPdfImportOpen(true)}
              className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
              title="Bankdokumente (PDF) importieren"
            >
              <FileText size={16} />
              PDF Import
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
                      {isSelectionMode && (
                        <th className="w-10 py-2">
                          <input
                            type="checkbox"
                            checked={allVisibleSelected}
                            onChange={allVisibleSelected ? handleDeselectAll : handleSelectAll}
                            className="w-4 h-4 rounded border-border"
                          />
                        </th>
                      )}
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
                      const isSelected = selectedIds.has(tx.id);

                      return (
                        <tr
                          key={tx.uuid}
                          className={`border-b border-border last:border-0 hover:bg-accent/30 group ${
                            isSelected ? 'bg-primary/5' : ''
                          }`}
                        >
                          {isSelectionMode && (
                            <td className="py-2">
                              <input
                                type="checkbox"
                                checked={isSelected}
                                onChange={() => handleToggleSelection(tx.id)}
                                className="w-4 h-4 rounded border-border"
                              />
                            </td>
                          )}
                          <td className="py-2 whitespace-nowrap">{formatDate(tx.date)}</td>
                          <td className="py-2">
                            <span className={`inline-block px-2 py-0.5 rounded text-xs ${
                              tx.ownerType === 'portfolio' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300' : 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300'
                            }`}>
                              {getTransactionTypeLabel(tx.txnType)}
                            </span>
                          </td>
                          <td className="py-2 text-muted-foreground">{tx.ownerName}</td>
                          <td className="py-2">
                            {tx.securityName && tx.securityUuid ? (
                              <div className="flex items-center gap-2">
                                <SecurityLogo
                                  securityId={securityUuidToId.get(tx.securityUuid) || 0}
                                  logos={logos}
                                  size={24}
                                />
                                <span className="font-medium">{tx.securityName}</span>
                              </div>
                            ) : tx.securityName ? (
                              <span className="font-medium">{tx.securityName}</span>
                            ) : (
                              <span className="text-muted-foreground">-</span>
                            )}
                          </td>
                          <td className="py-2 text-right font-mono">
                            {tx.shares != null ? tx.shares.toLocaleString('de-DE', { maximumFractionDigits: 6 }) : '-'}
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
                            <div className="flex items-center gap-1">
                              <button
                                onClick={() => handleEdit(tx)}
                                className="p-1.5 opacity-0 group-hover:opacity-100 hover:bg-muted rounded-md transition-all"
                                title="Bearbeiten"
                              >
                                <Pencil size={14} className="text-muted-foreground" />
                              </button>
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
                            </div>
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
          transaction={editingTransaction || undefined}
        />

        {/* PDF Import Modal */}
        <PdfImportModal
          isOpen={isPdfImportOpen}
          onClose={() => setIsPdfImportOpen(false)}
          onSuccess={() => {
            setIsPdfImportOpen(false);
            loadTransactions();
          }}
        />

        {/* Bulk Delete Confirmation Modal */}
        <BulkDeleteConfirmModal
          isOpen={showBulkDeleteModal}
          onClose={() => setShowBulkDeleteModal(false)}
          onConfirm={handleBulkDelete}
          transactions={selectedTransactions}
          isDeleting={isBulkDeleting}
        />
      </div>
    );
}
