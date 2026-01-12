/**
 * Modal for creating and editing transactions.
 */

import { useState, useEffect, useMemo } from 'react';
import { X, ChevronRight, ChevronDown } from 'lucide-react';
import type {
  AccountData,
  PortfolioData,
  SecurityData,
  CreateTransactionRequest,
  TransactionData,
} from '../../lib/types';
import { extractDateForInput, extractTimeForInput, combineDateAndTime } from '../../lib/types';
import { createTransaction, updateTransaction, getAccounts, getPortfolios, getSecurities } from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';
import { useSettingsStore } from '../../store';

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
  { value: 'TRANSFER_IN', label: 'Umbuchung (Eingang)' },
  { value: 'TRANSFER_OUT', label: 'Umbuchung (Ausgang)' },
];

const PORTFOLIO_TXN_TYPES = [
  { value: 'BUY', label: 'Kauf' },
  { value: 'SELL', label: 'Verkauf' },
  { value: 'DELIVERY_INBOUND', label: 'Einlieferung' },
  { value: 'DELIVERY_OUTBOUND', label: 'Auslieferung' },
  { value: 'TRANSFER_IN', label: 'Umbuchung (Eingang)' },
  { value: 'TRANSFER_OUT', label: 'Umbuchung (Ausgang)' },
];

// Types that require other account/portfolio selection (transfers)
const TRANSFER_TYPES = ['TRANSFER_IN', 'TRANSFER_OUT'];

// Types that require security selection
const SECURITY_REQUIRED_TYPES = ['BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND', 'DIVIDENDS'];
// Types that require shares input
const SHARES_REQUIRED_TYPES = ['BUY', 'SELL', 'DELIVERY_INBOUND', 'DELIVERY_OUTBOUND'];

interface TransactionFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  defaultSecurityId?: number;
  /** Transaction to edit (if provided, modal is in edit mode) */
  transaction?: TransactionData;
}

export function TransactionFormModal({ isOpen, onClose, onSuccess, defaultSecurityId, transaction }: TransactionFormModalProps) {
  const isEditMode = !!transaction;
  const { deliveryMode } = useSettingsStore();

  // Default txnType depends on deliveryMode setting
  const defaultPortfolioTxnType = deliveryMode ? 'DELIVERY_INBOUND' : 'BUY';

  const [formData, setFormData] = useState({
    ownerType: 'portfolio',
    ownerId: '',
    txnType: defaultPortfolioTxnType,
    date: new Date().toISOString().split('T')[0],
    time: '00:00',
    amount: '',
    currency: 'EUR',
    shares: '',
    securityId: defaultSecurityId ? String(defaultSecurityId) : '',
    note: '',
    feeAmount: '',
    taxAmount: '',
    // Transfer fields
    otherAccountId: '',
    otherPortfolioId: '',
    // Forex fields
    forexAmount: '',
    forexCurrency: '',
    exchangeRate: '',
  });

  const [accounts, setAccounts] = useState<AccountData[]>([]);
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isLoadingData, setIsLoadingData] = useState(false);
  const [forexExpanded, setForexExpanded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dataLoaded, setDataLoaded] = useState(false);

  // ESC key to close
  useEscapeKey(isOpen, onClose);

  // Load data when modal opens
  useEffect(() => {
    if (isOpen) {
      setIsLoadingData(true);
      setDataLoaded(false);
      Promise.all([getAccounts(), getPortfolios(), getSecurities()])
        .then(([acc, port, sec]) => {
          setAccounts(acc);
          setPortfolios(port);
          setSecurities(sec);
          setDataLoaded(true);
        })
        .catch((err) => console.error('Failed to load data:', err))
        .finally(() => setIsLoadingData(false));
    } else {
      setDataLoaded(false);
    }
  }, [isOpen]);

  // Reset/prefill form when data is loaded
  useEffect(() => {
    if (isOpen && dataLoaded) {
      if (isEditMode && transaction) {
        // Edit mode: prefill with transaction data
        const ownerType = transaction.ownerType === 'account' ? 'account' : 'portfolio';

        // Use ownerId directly from transaction
        const ownerId = transaction.ownerId ? String(transaction.ownerId) : '';

        // Use securityId directly from transaction
        const securityId = transaction.securityId ? String(transaction.securityId) : '';

        setFormData({
          ownerType,
          ownerId,
          txnType: transaction.txnType,
          date: extractDateForInput(transaction.date),
          time: extractTimeForInput(transaction.date),
          amount: String(transaction.amount), // Already in euros from API
          currency: transaction.currency,
          shares: transaction.shares ? String(transaction.shares) : '', // Already converted by API
          securityId,
          note: transaction.note || '',
          feeAmount: transaction.fees > 0 ? String(transaction.fees) : '', // Already in euros
          taxAmount: transaction.taxes > 0 ? String(transaction.taxes) : '', // Already in euros
          // Transfer fields
          otherAccountId: transaction.otherAccountId ? String(transaction.otherAccountId) : '',
          otherPortfolioId: transaction.otherPortfolioId ? String(transaction.otherPortfolioId) : '',
          // Forex fields (if available)
          forexAmount: '',
          forexCurrency: '',
          exchangeRate: '',
        });
        setForexExpanded(false);
      } else {
        // Create mode: reset form
        setFormData({
          ownerType: 'portfolio',
          ownerId: portfolios.length > 0 ? String(portfolios[0].id) : '',
          txnType: defaultPortfolioTxnType,
          date: new Date().toISOString().split('T')[0],
          time: '00:00',
          amount: '',
          currency: 'EUR',
          shares: '',
          securityId: defaultSecurityId ? String(defaultSecurityId) : '',
          note: '',
          feeAmount: '',
          taxAmount: '',
          otherAccountId: '',
          otherPortfolioId: '',
          forexAmount: '',
          forexCurrency: '',
          exchangeRate: '',
        });
        setForexExpanded(false);
      }
      setError(null);
    }
  }, [isOpen, dataLoaded, portfolios, accounts, securities, defaultSecurityId, isEditMode, transaction, defaultPortfolioTxnType]);

  // Get available transaction types based on owner type
  const availableTxnTypes = formData.ownerType === 'portfolio'
    ? PORTFOLIO_TXN_TYPES
    : ACCOUNT_TXN_TYPES;

  // Check if security is required
  const requiresSecurity = SECURITY_REQUIRED_TYPES.includes(formData.txnType);
  const requiresShares = SHARES_REQUIRED_TYPES.includes(formData.txnType);
  const isTransferType = TRANSFER_TYPES.includes(formData.txnType);

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

    // Special handling for ownerType change - update related fields
    if (name === 'ownerType') {
      if (value === 'portfolio') {
        setFormData(prev => ({
          ...prev,
          ownerType: value,
          ownerId: portfolios.length > 0 ? String(portfolios[0].id) : '',
          txnType: defaultPortfolioTxnType,
        }));
      } else {
        setFormData(prev => ({
          ...prev,
          ownerType: value,
          ownerId: accounts.length > 0 ? String(accounts[0].id) : '',
          txnType: 'DEPOSIT',
        }));
      }
      return;
    }

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

      if (isEditMode && transaction) {
        // Update existing transaction - send all fields
        const feeAmountCents = formData.feeAmount && parseFloat(formData.feeAmount) > 0
          ? Math.round(parseFloat(formData.feeAmount) * 100)
          : undefined;
        const taxAmountCents = formData.taxAmount && parseFloat(formData.taxAmount) > 0
          ? Math.round(parseFloat(formData.taxAmount) * 100)
          : undefined;

        await updateTransaction(transaction.id, {
          date: combineDateAndTime(formData.date, formData.time),
          amount: amountCents,
          shares: sharesScaled,
          note: formData.note || undefined,
          feeAmount: feeAmountCents,
          taxAmount: taxAmountCents,
          // Full edit support
          ownerType: formData.ownerType,
          ownerId: parseInt(formData.ownerId),
          txnType: formData.txnType,
          securityId: formData.securityId ? parseInt(formData.securityId) : undefined,
          currency: formData.currency || selectedOwnerCurrency,
        });
      } else {
        // Create new transaction
        const data: CreateTransactionRequest = {
          ownerType: formData.ownerType,
          ownerId: parseInt(formData.ownerId),
          txnType: formData.txnType,
          date: combineDateAndTime(formData.date, formData.time),
          amount: amountCents,
          currency: formData.currency || selectedOwnerCurrency,
          shares: sharesScaled,
          securityId: formData.securityId ? parseInt(formData.securityId) : undefined,
          note: formData.note || undefined,
          units: [],
          // Transfer fields
          otherAccountId: formData.otherAccountId ? parseInt(formData.otherAccountId) : undefined,
          otherPortfolioId: formData.otherPortfolioId ? parseInt(formData.otherPortfolioId) : undefined,
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

        // Add forex unit if specified
        if (formData.forexAmount && parseFloat(formData.forexAmount) > 0 && formData.forexCurrency) {
          const forexAmountCents = Math.round(parseFloat(formData.forexAmount) * 100);
          const exchangeRateScaled = formData.exchangeRate
            ? Math.round(parseFloat(formData.exchangeRate) * 100_000_000)
            : undefined;
          data.units!.push({
            unitType: 'FOREX',
            amount: forexAmountCents,
            currency: formData.currency || selectedOwnerCurrency,
            forexAmount: forexAmountCents,
            forexCurrency: formData.forexCurrency,
            exchangeRate: exchangeRateScaled,
          });
        }

        await createTransaction(data);
      }

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
          <h2 className="text-lg font-semibold">{isEditMode ? 'Buchung bearbeiten' : 'Neue Buchung'}</h2>
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
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-60 disabled:cursor-not-allowed"
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
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-60 disabled:cursor-not-allowed"
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
          <div className="grid grid-cols-3 gap-4">
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

            {/* Time */}
            <div>
              <label className="block text-sm font-medium mb-1">Uhrzeit</label>
              <input
                type="time"
                name="time"
                value={formData.time}
                onChange={handleChange}
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
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-60 disabled:cursor-not-allowed"
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

          {/* Transfer Target (for TRANSFER_IN/TRANSFER_OUT) */}
          {isTransferType && (
            <div>
              <label className="block text-sm font-medium mb-1">
                {formData.ownerType === 'portfolio' ? 'Gegenstück-Portfolio' : 'Gegenstück-Konto'}
              </label>
              {formData.ownerType === 'portfolio' ? (
                <select
                  name="otherPortfolioId"
                  value={formData.otherPortfolioId}
                  onChange={handleChange}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                >
                  <option value="">Kein Gegenstück</option>
                  {portfolios
                    .filter(p => !p.isRetired && String(p.id) !== formData.ownerId)
                    .map((p) => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))
                  }
                </select>
              ) : (
                <select
                  name="otherAccountId"
                  value={formData.otherAccountId}
                  onChange={handleChange}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                >
                  <option value="">Kein Gegenstück</option>
                  {accounts
                    .filter(a => !a.isRetired && String(a.id) !== formData.ownerId)
                    .map((a) => (
                      <option key={a.id} value={a.id}>{a.name} ({a.currency})</option>
                    ))
                  }
                </select>
              )}
              <p className="text-xs text-muted-foreground mt-1">
                {formData.txnType === 'TRANSFER_IN'
                  ? 'Das Portfolio/Konto, von dem die Buchung kommt.'
                  : 'Das Portfolio/Konto, zu dem die Buchung geht.'}
              </p>
            </div>
          )}

          {/* Forex Section (Collapsible) */}
          <div className="border border-border rounded-md">
            <button
              type="button"
              onClick={() => setForexExpanded(!forexExpanded)}
              className="w-full flex items-center gap-2 p-3 text-sm font-medium hover:bg-muted/50 transition-colors"
            >
              {forexExpanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
              Fremdwährung
              {formData.forexAmount && formData.forexCurrency && (
                <span className="text-muted-foreground ml-auto">
                  {formData.forexAmount} {formData.forexCurrency}
                </span>
              )}
            </button>
            {forexExpanded && (
              <div className="p-3 pt-0 space-y-3">
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block text-xs font-medium mb-1">Betrag</label>
                    <input
                      type="number"
                      name="forexAmount"
                      value={formData.forexAmount}
                      onChange={handleChange}
                      step="0.01"
                      className="w-full px-2 py-1.5 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                      placeholder="0.00"
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-medium mb-1">Währung</label>
                    <select
                      name="forexCurrency"
                      value={formData.forexCurrency}
                      onChange={handleChange}
                      className="w-full px-2 py-1.5 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                    >
                      <option value="">Auswählen...</option>
                      <option value="USD">USD</option>
                      <option value="EUR">EUR</option>
                      <option value="GBP">GBP</option>
                      <option value="CHF">CHF</option>
                      <option value="JPY">JPY</option>
                      <option value="CAD">CAD</option>
                      <option value="AUD">AUD</option>
                    </select>
                  </div>
                </div>
                <div>
                  <label className="block text-xs font-medium mb-1">Wechselkurs</label>
                  <input
                    type="number"
                    name="exchangeRate"
                    value={formData.exchangeRate}
                    onChange={handleChange}
                    step="0.00000001"
                    className="w-full px-2 py-1.5 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                    placeholder="z.B. 1.08"
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Kurs: 1 {formData.forexCurrency || 'FX'} = {formData.exchangeRate || '?'} {formData.currency || selectedOwnerCurrency}
                  </p>
                </div>
              </div>
            )}
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
              {isSubmitting ? 'Speichern...' : (isEditMode ? 'Speichern' : 'Erstellen')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
