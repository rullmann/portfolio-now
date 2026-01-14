/**
 * Confirmation modal for bulk transaction deletion.
 * Shows summary of transactions to be deleted with warnings about linked transactions.
 */

import { useMemo } from 'react';
import { AlertTriangle, Trash2 } from 'lucide-react';
import { useEscapeKey } from '../../lib/hooks';
import { getTransactionTypeLabel } from '../../lib/types';

interface TransactionData {
  id: number;
  ownerType: string;
  ownerName: string;
  txnType: string;
  securityName?: string;
}

interface BulkDeleteConfirmModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  transactions: TransactionData[];
  isDeleting: boolean;
}

export function BulkDeleteConfirmModal({
  isOpen,
  onClose,
  onConfirm,
  transactions,
  isDeleting,
}: BulkDeleteConfirmModalProps) {
  useEscapeKey(isOpen, onClose);

  // Calculate summary statistics
  const summary = useMemo(() => {
    const byType = new Map<string, number>();
    const affectedOwners = new Set<string>();
    let hasLinkedTransactions = false;

    transactions.forEach((tx) => {
      // Count by transaction type
      byType.set(tx.txnType, (byType.get(tx.txnType) || 0) + 1);

      // Track affected accounts/portfolios
      affectedOwners.add(`${tx.ownerType}:${tx.ownerName}`);

      // Check if BUY/SELL which have cross-entries (linked account transactions)
      if (['BUY', 'SELL'].includes(tx.txnType) && tx.ownerType === 'portfolio') {
        hasLinkedTransactions = true;
      }
    });

    return { byType, affectedOwners, hasLinkedTransactions };
  }, [transactions]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={isDeleting ? undefined : onClose}
      />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-lg w-full max-w-md mx-4 overflow-hidden">
        {/* Header */}
        <div className="bg-destructive/10 p-6 text-center">
          <div className="w-16 h-16 bg-destructive/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <Trash2 size={32} className="text-destructive" />
          </div>
          <h2 className="text-xl font-semibold">Transaktionen löschen</h2>
          <p className="text-destructive font-medium mt-2">
            {transactions.length} Buchung{transactions.length !== 1 ? 'en' : ''} werden gelöscht
          </p>
        </div>

        {/* Content */}
        <div className="p-6 space-y-4">
          {/* Summary by type */}
          <div className="bg-muted p-3 rounded-md">
            <h4 className="font-medium mb-2 text-sm">Zusammenfassung:</h4>
            <ul className="text-sm space-y-1">
              {Array.from(summary.byType)
                .sort((a, b) => b[1] - a[1])
                .map(([type, count]) => (
                  <li key={type} className="flex justify-between">
                    <span>{getTransactionTypeLabel(type)}</span>
                    <span className="text-muted-foreground">{count}x</span>
                  </li>
                ))}
            </ul>
          </div>

          {/* Affected accounts/portfolios */}
          <div className="text-sm text-muted-foreground">
            Betroffene Konten/Depots: {summary.affectedOwners.size}
          </div>

          {/* Warning for linked transactions */}
          {summary.hasLinkedTransactions && (
            <div className="flex items-start gap-3 p-3 bg-warning/10 border border-warning/20 rounded-md">
              <AlertTriangle className="text-warning flex-shrink-0 mt-0.5" size={18} />
              <div className="text-sm">
                <strong>Hinweis:</strong> Verknüpfte Konto-Buchungen werden ebenfalls
                gelöscht (z.B. Kauf/Verkauf mit zugehöriger Belastung).
              </div>
            </div>
          )}

          {/* FIFO recalculation notice */}
          <div className="text-sm text-muted-foreground">
            Der Einstandswert (FIFO) wird automatisch neu berechnet.
          </div>

          {/* Buttons */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              disabled={isDeleting}
              className="flex-1 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Abbrechen
            </button>
            <button
              type="button"
              onClick={onConfirm}
              disabled={isDeleting}
              className="flex-1 px-4 py-2.5 bg-destructive text-destructive-foreground rounded-lg hover:bg-destructive/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isDeleting ? 'Lösche...' : 'Endgültig löschen'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
