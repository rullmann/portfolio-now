/**
 * Accounts view for displaying and managing accounts.
 */

import { useState, useEffect, useCallback } from 'react';
import { Plus, Pencil, Trash2, AlertCircle, RefreshCw, Wallet } from 'lucide-react';
import type { AccountData } from '../../lib/types';
import { getAccounts, deleteAccount } from '../../lib/api';
import { AccountFormModal } from '../../components/modals';
import { formatCurrency } from '../../lib/types';
import { AccountCardSkeleton } from '../../components/common';

export function AccountsView() {
  const [dbAccounts, setDbAccounts] = useState<AccountData[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingAccount, setEditingAccount] = useState<AccountData | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  const loadAccounts = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await getAccounts();
      setDbAccounts(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAccounts();
  }, [loadAccounts]);

  const handleCreate = () => {
    setEditingAccount(null);
    setIsModalOpen(true);
  };

  const handleEdit = (account: AccountData) => {
    setEditingAccount(account);
    setIsModalOpen(true);
  };

  const handleDelete = async (account: AccountData) => {
    if (!confirm(`Konto "${account.name}" wirklich löschen?`)) {
      return;
    }

    setDeletingId(account.id);
    setError(null);

    try {
      await deleteAccount(account.id);
      await loadAccounts();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeletingId(null);
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setEditingAccount(null);
  };

  const handleModalSuccess = () => {
    loadAccounts();
  };

  return (
    <div className="space-y-4">
      {/* Header with actions */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">
          Konten ({dbAccounts.length})
        </h2>
        <div className="flex gap-2">
          <button
            onClick={loadAccounts}
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
      {isLoading && dbAccounts.length === 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          <AccountCardSkeleton />
          <AccountCardSkeleton />
          <AccountCardSkeleton />
          <AccountCardSkeleton />
          <AccountCardSkeleton />
          <AccountCardSkeleton />
        </div>
      ) : dbAccounts.length > 0 ? (
        /* Accounts grid */
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {dbAccounts.map((account) => (
            <div
              key={account.id}
              className="bg-card rounded-lg border border-border p-4 hover:border-primary/50 transition-colors"
            >
              <div className="flex items-start justify-between mb-3">
                <div className="flex items-center gap-3">
                  <div className="p-2 bg-primary/10 rounded-lg">
                    <Wallet size={20} className="text-primary" />
                  </div>
                  <div>
                    <h3 className={`font-medium ${account.isRetired ? 'text-muted-foreground line-through' : ''}`}>
                      {account.name}
                    </h3>
                    <p className="text-sm text-muted-foreground">{account.currency}</p>
                  </div>
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={() => handleEdit(account)}
                    className="p-1.5 hover:bg-muted rounded-md transition-colors"
                    title="Bearbeiten"
                  >
                    <Pencil size={16} className="text-muted-foreground" />
                  </button>
                  <button
                    onClick={() => handleDelete(account)}
                    disabled={deletingId === account.id}
                    className="p-1.5 hover:bg-destructive/10 rounded-md transition-colors disabled:opacity-50"
                    title="Löschen"
                  >
                    <Trash2
                      size={16}
                      className={
                        deletingId === account.id
                          ? 'text-muted-foreground animate-pulse'
                          : 'text-destructive'
                      }
                    />
                  </button>
                </div>
              </div>

              <div className="space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Saldo</span>
                  <span className={`font-medium tabular-nums ${account.balance >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {formatCurrency(account.balance, account.currency)}
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Buchungen</span>
                  <span className="tabular-nums">{account.transactionsCount}</span>
                </div>
                {account.isRetired && (
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
      ) : (
        <div className="bg-card rounded-lg border border-border p-6 text-center text-muted-foreground">
          Keine Konten vorhanden. Importieren Sie eine .portfolio Datei oder erstellen Sie ein neues Konto.
        </div>
      )}

      {/* Account Form Modal */}
      <AccountFormModal
        isOpen={isModalOpen}
        onClose={handleModalClose}
        onSuccess={handleModalSuccess}
        account={editingAccount}
      />
    </div>
  );
}
