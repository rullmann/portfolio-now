/**
 * Recent Transactions Widget - Shows last N transactions
 */

import { useEffect, useState } from 'react';
import { ArrowUpRight, ArrowDownRight, RefreshCw, ArrowLeftRight } from 'lucide-react';
import { getTransactions } from '../../../lib/api';
import type { TransactionData } from '../../../lib/types';
import type { WidgetProps } from '../types';

interface RecentTransactionsWidgetProps extends WidgetProps {
  currency?: string;
}

const TXN_TYPE_LABELS: Record<string, string> = {
  BUY: 'Kauf',
  SELL: 'Verkauf',
  DEPOSIT: 'Einzahlung',
  REMOVAL: 'Auszahlung',
  DIVIDENDS: 'Dividende',
  INTEREST: 'Zinsen',
  INTEREST_CHARGE: 'Zinsbelastung',
  FEES: 'Gebühren',
  FEES_REFUND: 'Gebühren-Erstattung',
  TAXES: 'Steuern',
  TAX_REFUND: 'Steuer-Erstattung',
  TRANSFER_IN: 'Umbuchung (ein)',
  TRANSFER_OUT: 'Umbuchung (aus)',
  DELIVERY_INBOUND: 'Einlieferung',
  DELIVERY_OUTBOUND: 'Auslieferung',
};

const TXN_TYPE_ICONS: Record<string, React.ElementType> = {
  BUY: ArrowDownRight,
  SELL: ArrowUpRight,
  DEPOSIT: ArrowDownRight,
  REMOVAL: ArrowUpRight,
  DIVIDENDS: ArrowDownRight,
  TRANSFER_IN: ArrowLeftRight,
  TRANSFER_OUT: ArrowLeftRight,
  DELIVERY_INBOUND: ArrowDownRight,
  DELIVERY_OUTBOUND: ArrowUpRight,
};

export function RecentTransactionsWidget({
  config,
}: RecentTransactionsWidgetProps) {
  const limit = (config.settings.limit as number) || 10;
  const [transactions, setTransactions] = useState<TransactionData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadTransactions = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getTransactions({ limit });
      setTransactions(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTransactions();
  }, [limit]);

  const formatCurrency = (value: number, curr: string) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'currency',
      currency: curr,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value);
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString('de-DE', {
      day: '2-digit',
      month: '2-digit',
      year: '2-digit',
    });
  };

  const getTypeColor = (txnType: string) => {
    if (['BUY', 'DEPOSIT', 'DIVIDENDS', 'INTEREST', 'DELIVERY_INBOUND', 'TRANSFER_IN'].includes(txnType)) {
      return 'text-green-600';
    }
    if (['SELL', 'REMOVAL', 'FEES', 'TAXES', 'INTEREST_CHARGE', 'DELIVERY_OUTBOUND', 'TRANSFER_OUT'].includes(txnType)) {
      return 'text-red-600';
    }
    return 'text-muted-foreground';
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col p-4">
        <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
          Letzte Buchungen
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center text-sm text-muted-foreground">
            <p>{error}</p>
            <button
              onClick={loadTransactions}
              className="mt-2 text-primary hover:underline"
            >
              Erneut versuchen
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
        Letzte Buchungen
      </div>
      <div className="flex-1 overflow-auto space-y-1">
        {transactions.length === 0 ? (
          <div className="text-center text-muted-foreground py-4 text-sm">
            Keine Buchungen vorhanden
          </div>
        ) : (
          transactions.map((txn) => {
            const Icon = TXN_TYPE_ICONS[txn.txnType] || ArrowLeftRight;
            const colorClass = getTypeColor(txn.txnType);

            return (
              <div
                key={txn.id}
                className="flex items-center gap-2 py-1.5 px-2 rounded hover:bg-muted/30"
              >
                <Icon className={`h-3.5 w-3.5 shrink-0 ${colorClass}`} />
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-medium truncate">
                    {txn.securityName || txn.ownerName}
                  </div>
                  <div className="text-[10px] text-muted-foreground">
                    {TXN_TYPE_LABELS[txn.txnType] || txn.txnType} &middot; {formatDate(txn.date)}
                  </div>
                </div>
                <div className={`text-xs font-medium text-right ${colorClass}`}>
                  {formatCurrency(txn.amount, txn.currency)}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
