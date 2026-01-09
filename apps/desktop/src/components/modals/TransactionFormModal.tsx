/**
 * Modal for creating transactions.
 */

import { useState, useEffect, useMemo } from 'react';
import { X } from 'lucide-react';
import type {
  AccountData,
  PortfolioData,
  SecurityData,
  CreateTransactionRequest,
} from '../../lib/types';
import { createTransaction, getAccounts, getPortfolios, getSecurities } from '../../lib/api';

// Transaction types by owner
const ACCOUNT_TXN_TYPES = [
  { value: 'DEPOSIT', label: 'Einlage' },
  { value: 'REMOVAL', label: 'Entnahme' },
  { value: 'INTEREST', label: 'Zinsen' },
  { value: 'INTEREST_CHARGE', label: 'Zinsbelastung' },
  { value: 'DIVIDENDS', label: 'Dividende' },
  { value: 'FEES', label: 'Gebühren' },
  { value: 'FEES_REFUND', label: 'Gebührenerstattung' },
  { value: 'TAXES', label: 'Steuern' },
  { value: 'TAX_REFUND', label: 'Steuererstattung' },
];

const PORTFOLIO_TXN_TYPES = [
  { value: 'BUY', label: 'Kauf' },
  { value: 'SELL', label: 'Verkauf' },
  { value: 'DELIVERY_INBOUND', label: 'Einlieferung' },
  { value: 'DELIVERY_OUTBOUND', label: 'Auslieferung' },
];

// Types that require security selection
const SECURITY_REQUIRED_TYPES = ['BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND', 'DIVIDENDS'];
// Types that require shares input
const SHARES_REQUIRED_TYPES = ['BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND'];

interface TransactionFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export function TransactionFormModal({ isOpen, onClose, onSuccess }: TransactionFormModalProps) {
  const [formData, setFormData] = useState({
    ownerType: 'portfolio',
    ownerId: '',
    txnType: 'BUY',
    date: new Date().toISOString().split('T')[0],
    amount: '',
    currency: 'EUR',
    shares: '',
    securityId: '',
    note: '',
    feeAmount: '',
    taxAmount: '',
  });

  const [accounts, setAccounts] = useState<AccountData[]>([]);
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isLoadingData, setIsLoadingData] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load data when modal opens
  useEffect(() => {
    if (isOpen) {
      setIsLoadingData(true);
      Promise.all([getAccounts(), getPortfolios(), getSecurities()])
        .then(([acc, port, sec]) => {
          setAccounts(acc);
          setPortfolios(port);
          setSecurities(sec);
          // Set default owner if available
          if (port.length > 0 && formData.ownerType === 'portfolio') {
            setFormData(prev => ({ ...prev, ownerId: String(port[0].id) }));
          } else if (acc.length > 0 && formData.ownerType === 'account') {
            setFormData(prev => ({ ...prev, ownerId: String(acc[0].id) }));
          }
        })
        .catch((err) => console.error('Failed to load data:', err))
        .finally(() => setIsLoadingData(false));
    }
  }, [isOpen]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setFormData({
        ownerType: 'portfolio',
        ownerId: portfolios.length > 0 ? String(portfolios[0].id) : '',
        txnType: 'BUY',
        date: new Date().toISOString().split('T')[0],
        amount: '',
        currency: 'EUR',
        shares: '',
        securityId: '',
        note: '',
        feeAmount: '',
        taxAmount: '',
      });
      setError(null);
    }
  }, [isOpen, portfolios]);

  // Update ownerId and txnType when ownerType changes
  useEffect(() => {
    if (formData.ownerType === 'portfolio') {
      setFormData(prev => ({
        ...prev,
        ownerId: portfolios.length > 0 ? String(portfolios[0].id) : '',
        txnType: 'BUY',
      }));
    } else {
      setFormData(prev => ({
        ...prev,
        ownerId: accounts.length > 0 ? String(accounts[0].id) : '',
        txnType: 'DEPOSIT',
      }));
    }
  }, [formData.ownerType, portfolios, accounts]);

  // Get available transaction types based on owner type
  const availableTxnTypes = formData.ownerType === 'portfolio'
    ? PORTFOLIO_TXN_TYPES
    : ACCOUNT_TXN_TYPES;

  // Check if security is required
  const requiresSecurity = SECURITY_REQUIRED_TYPES.includes(formData.txnType);
  const requiresShares = SHARES_REQUIRED_TYPES.includes(formData.txnType);

  // Get selected owner's currency
  const selectedOwnerCurrency = useMemo(() => {
    if (formData.ownerType === 'portfolio') {
      const portfolio = portfolios.find(p => String(p.id) === formData.ownerId);
      // Get currency from reference account
      if (portfolio) {
        const refAccount = accounts.find(a => a.name === portfolio.referenceAccountName);
        return refAccount?.currency || 'EUR';
      }
    } else {
      const account = accounts.find(a => String(a.id) === formData.ownerId);
      return account?.currency || 'EUR';
    }
    return 'EUR';
  }, [formData.ownerType, formData.ownerId, portfolios, accounts]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      // Prepare request data
      const amountCents = Math.round(parseFloat(formData.amount) * 100);
      const sharesScaled = formData.shares
        ? Math.round(parseFloat(formData.shares) * 100_000_000)
        : undefined;

      const data: CreateTransactionRequest = {
        ownerType: formData.ownerType,
        ownerId: parseInt(formData.ownerId),
        txnType: formData.txnType,
        date: formData.date,
        amount: amountCents,
        currency: formData.currency || selectedOwnerCurrency,
        shares: sharesScaled,
        securityId: formData.securityId ? parseInt(formData.securityId) : undefined,
        note: formData.note || undefined,
        units: [],
      };

      // Add fee unit if specified
      if (formData.feeAmount && parseFloat(formData.feeAmount) > 0) {
        data.units!.push({
          unitType: 'FEE',
          amount: Math.round(parseFloat(formData.feeAmount) * 100),
          currency: formData.currency || selectedOwnerCurrency,
        });
      }

      // Add tax unit if specified
      if (formData.taxAmount && parseFloat(formData.taxAmount) > 0) {
        data.units!.push({
          unitType: 'TAX',
          amount: Math.round(parseFloat(formData.taxAmount) * 100),
          currency: formData.currency || selectedOwnerCurrency,
        });
      }

      await createTransaction(data);
      onSuccess();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border sticky top-0 bg-card">
          <h2 className="text-lg font-semibold">Neue Buchung</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          {error && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
              {error}
            </div>
          )}

          {/* Owner Type */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">Bereich</label>
              <select
                name="ownerType"
                value={formData.ownerType}
                onChange={handleChange}
                disabled={isLoadingData}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                <option value="portfolio">Depot</option>
                <option value="account">Konto</option>
              </select>
            </div>

            {/* Owner Selection */}
            <div>
              <label className="block text-sm font-medium mb-1">
                {formData.ownerType === 'portfolio' ? 'Depot' : 'Konto'}
              </label>
              <select
                name="ownerId"
                value={formData.ownerId}
                onChange={handleChange}
                disabled={isLoadingData}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                {formData.ownerType === 'portfolio'
                  ? portfolios.filter(p => !p.isRetired).map((p) => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))
                  : accounts.filter(a => !a.isRetired).map((a) => (
                      <option key={a.id} value={a.id}>{a.name} ({a.currency})</option>
                    ))
                }
              </select>
            </div>
          </div>

          {/* Transaction Type */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">Buchungsart</label>
              <select
                name="txnType"
                value={formData.txnType}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                {availableTxnTypes.map((type) => (
                  <option key={type.value} value={type.value}>{type.label}</option>
                ))}
              </select>
            </div>

            {/* Date */}
            <div>
              <label className="block text-sm font-medium mb-1">Datum</label>
              <input
                type="date"
                name="date"
                value={formData.date}
                onChange={handleChange}
                required
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              />
            </div>
          </div>

          {/* Security (if required) */}
          {requiresSecurity && (
            <div>
              <label className="block text-sm font-medium mb-1">
                Wertpapier <span className="text-destructive">*</span>
              </label>
              <select
                name="securityId"
                value={formData.securityId}
                onChange={handleChange}
                required={requiresSecurity}
                disabled={isLoadingData}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                <option value="">Wertpapier auswählen...</option>
                {securities.filter(s => !s.isRetired).map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name} {s.isin ? `(${s.isin})` : ''}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Shares (if required) */}
          {requiresShares && (
            <div>
              <label className="block text-sm font-medium mb-1">
                Stück <span className="text-destructive">*</span>
              </label>
              <input
                type="number"
                name="shares"
                value={formData.shares}
                onChange={handleChange}
                required={requiresShares}
                step="0.00000001"
                min="0"
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="z.B. 10"
              />
            </div>
          )}

          {/* Amount and Currency */}
          <div className="grid grid-cols-3 gap-4">
            <div className="col-span-2">
              <label className="block text-sm font-medium mb-1">
                Betrag <span className="text-destructive">*</span>
              </label>
              <input
                type="number"
                name="amount"
                value={formData.amount}
                onChange={handleChange}
                required
                step="0.01"
                min="0"
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="z.B. 1000.00"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Währung</label>
              <select
                name="currency"
                value={formData.currency || selectedOwnerCurrency}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                <option value="EUR">EUR</option>
                <option value="USD">USD</option>
                <option value="CHF">CHF</option>
                <option value="GBP">GBP</option>
              </select>
            </div>
          </div>

          {/* Fees and Taxes */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">Gebühren</label>
              <input
                type="number"
                name="feeAmount"
                value={formData.feeAmount}
                onChange={handleChange}
                step="0.01"
                min="0"
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="0.00"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">Steuern</label>
              <input
                type="number"
                name="taxAmount"
                value={formData.taxAmount}
                onChange={handleChange}
                step="0.01"
                min="0"
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="0.00"
              />
            </div>
          </div>

          {/* Note */}
          <div>
            <label className="block text-sm font-medium mb-1">Notiz</label>
            <textarea
              name="note"
              value={formData.note}
              onChange={handleChange}
              rows={2}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
              placeholder="Optionale Notizen..."
            />
          </div>

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4 border-t border-border">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors"
            >
              Abbrechen
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !formData.ownerId || !formData.amount}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Speichern...' : 'Erstellen'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
