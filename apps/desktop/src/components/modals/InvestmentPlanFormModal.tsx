/**
 * Modal for creating and editing investment plans (Sparpläne).
 */

import { useState, useEffect } from 'react';
import { X, Search } from 'lucide-react';
import type {
  InvestmentPlanData,
  CreateInvestmentPlanRequest,
  PlanInterval,
  SecurityData,
  AccountData,
  PortfolioData,
} from '../../lib/types';
import {
  createInvestmentPlan,
  updateInvestmentPlan,
  getSecurities,
  getAccounts,
  getPortfolios,
} from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';

const INTERVAL_OPTIONS: { value: PlanInterval; label: string }[] = [
  { value: 'WEEKLY', label: 'Wöchentlich' },
  { value: 'BIWEEKLY', label: 'Zweiwöchentlich' },
  { value: 'MONTHLY', label: 'Monatlich' },
  { value: 'QUARTERLY', label: 'Quartalsweise' },
  { value: 'YEARLY', label: 'Jährlich' },
];

interface InvestmentPlanFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  plan?: InvestmentPlanData | null;
}

export function InvestmentPlanFormModal({
  isOpen,
  onClose,
  onSuccess,
  plan,
}: InvestmentPlanFormModalProps) {
  useEscapeKey(isOpen, onClose);

  const isEditMode = !!plan;

  const [formData, setFormData] = useState({
    name: '',
    securityId: '',
    accountId: '',
    portfolioId: '',
    interval: 'MONTHLY' as PlanInterval,
    amount: '',
    dayOfMonth: '1',
    startDate: new Date().toISOString().split('T')[0],
    endDate: '',
    isActive: true,
  });

  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [accounts, setAccounts] = useState<AccountData[]>([]);
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [securitySearch, setSecuritySearch] = useState('');
  const [showSecurityDropdown, setShowSecurityDropdown] = useState(false);

  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isLoadingData, setIsLoadingData] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load data when modal opens
  useEffect(() => {
    if (isOpen) {
      setIsLoadingData(true);
      Promise.all([getSecurities(), getAccounts(), getPortfolios()])
        .then(([sec, acc, port]) => {
          setSecurities(sec);
          setAccounts(acc);
          setPortfolios(port);
        })
        .catch((err) => console.error('Failed to load data:', err))
        .finally(() => setIsLoadingData(false));
    }
  }, [isOpen]);

  // Reset form when modal opens or plan changes
  useEffect(() => {
    if (isOpen) {
      if (plan) {
        setFormData({
          name: plan.name || '',
          securityId: String(plan.securityId),
          accountId: String(plan.accountId),
          portfolioId: String(plan.portfolioId),
          interval: plan.interval,
          amount: String(plan.amount),
          dayOfMonth: String(plan.dayOfMonth),
          startDate: plan.startDate,
          endDate: plan.endDate || '',
          isActive: plan.isActive,
        });
        // Set search text to security name
        setSecuritySearch(plan.securityName || '');
      } else {
        setFormData({
          name: '',
          securityId: '',
          accountId: '',
          portfolioId: '',
          interval: 'MONTHLY',
          amount: '',
          dayOfMonth: '1',
          startDate: new Date().toISOString().split('T')[0],
          endDate: '',
          isActive: true,
        });
        setSecuritySearch('');
      }
      setShowSecurityDropdown(false);
      setError(null);
    }
  }, [isOpen, plan]);

  // Filter securities by search
  const filteredSecurities = securities.filter(
    (s) =>
      !s.isRetired &&
      (s.name.toLowerCase().includes(securitySearch.toLowerCase()) ||
        s.isin?.toLowerCase().includes(securitySearch.toLowerCase()) ||
        s.ticker?.toLowerCase().includes(securitySearch.toLowerCase()))
  );

  // Get selected security name
  const selectedSecurity = securities.find((s) => String(s.id) === formData.securityId);

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>
  ) => {
    const { name, value, type } = e.target;
    if (type === 'checkbox') {
      setFormData((prev) => ({
        ...prev,
        [name]: (e.target as HTMLInputElement).checked,
      }));
    } else {
      setFormData((prev) => ({ ...prev, [name]: value }));
    }
  };

  const handleSecuritySelect = (security: SecurityData) => {
    setFormData((prev) => ({ ...prev, securityId: String(security.id) }));
    setSecuritySearch(security.name);
    setShowSecurityDropdown(false);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      const amountCents = Math.round(parseFloat(formData.amount) * 100);

      if (isEditMode && plan) {
        await updateInvestmentPlan(plan.id, {
          name: formData.name,
          securityId: parseInt(formData.securityId),
          accountId: parseInt(formData.accountId),
          portfolioId: parseInt(formData.portfolioId),
          interval: formData.interval,
          amount: amountCents,
          dayOfMonth: parseInt(formData.dayOfMonth),
          startDate: formData.startDate,
          endDate: formData.endDate || undefined,
          isActive: formData.isActive,
        });
      } else {
        const createData: CreateInvestmentPlanRequest = {
          name: formData.name,
          securityId: parseInt(formData.securityId),
          accountId: parseInt(formData.accountId),
          portfolioId: parseInt(formData.portfolioId),
          interval: formData.interval,
          amount: amountCents,
          dayOfMonth: parseInt(formData.dayOfMonth),
          startDate: formData.startDate,
          endDate: formData.endDate || undefined,
        };
        await createInvestmentPlan(createData);
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
          <h2 className="text-lg font-semibold">
            {isEditMode ? 'Sparplan bearbeiten' : 'Neuer Sparplan'}
          </h2>
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

          {/* Name */}
          <div>
            <label className="block text-sm font-medium mb-1">
              Name <span className="text-destructive">*</span>
            </label>
            <input
              type="text"
              name="name"
              value={formData.name}
              onChange={handleChange}
              required
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder="z.B. ETF-Sparplan"
            />
          </div>

          {/* Security Search */}
          <div className="relative">
            <label className="block text-sm font-medium mb-1">
              Wertpapier <span className="text-destructive">*</span>
            </label>
            <div className="relative">
              <Search
                size={16}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"
              />
              <input
                type="text"
                value={securitySearch}
                onChange={(e) => {
                  setSecuritySearch(e.target.value);
                  setShowSecurityDropdown(true);
                }}
                onFocus={() => setShowSecurityDropdown(true)}
                disabled={isLoadingData}
                className="w-full pl-9 pr-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
                placeholder="Nach Wertpapier suchen..."
              />
            </div>
            {showSecurityDropdown && securitySearch && (
              <div className="absolute z-10 w-full mt-1 max-h-48 overflow-y-auto bg-card border border-border rounded-md shadow-lg">
                {filteredSecurities.length > 0 ? (
                  filteredSecurities.slice(0, 10).map((s) => (
                    <button
                      key={s.id}
                      type="button"
                      onClick={() => handleSecuritySelect(s)}
                      className="w-full px-3 py-2 text-left hover:bg-muted transition-colors"
                    >
                      <div className="font-medium text-sm">{s.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {s.isin || s.ticker || ''}
                      </div>
                    </button>
                  ))
                ) : (
                  <div className="px-3 py-2 text-sm text-muted-foreground">
                    Keine Ergebnisse
                  </div>
                )}
              </div>
            )}
            {selectedSecurity && (
              <p className="text-xs text-muted-foreground mt-1">
                Ausgewählt: {selectedSecurity.name} ({selectedSecurity.isin || selectedSecurity.ticker})
              </p>
            )}
          </div>

          {/* Portfolio and Account */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">
                Portfolio <span className="text-destructive">*</span>
              </label>
              <select
                name="portfolioId"
                value={formData.portfolioId}
                onChange={handleChange}
                required
                disabled={isLoadingData}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
              >
                <option value="">Auswählen...</option>
                {portfolios
                  .filter((p) => !p.isRetired)
                  .map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">
                Konto <span className="text-destructive">*</span>
              </label>
              <select
                name="accountId"
                value={formData.accountId}
                onChange={handleChange}
                required
                disabled={isLoadingData}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
              >
                <option value="">Auswählen...</option>
                {accounts
                  .filter((a) => !a.isRetired)
                  .map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.name} ({a.currency})
                    </option>
                  ))}
              </select>
            </div>
          </div>

          {/* Amount and Interval */}
          <div className="grid grid-cols-2 gap-4">
            <div>
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
                placeholder="z.B. 100"
              />
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">
                Intervall <span className="text-destructive">*</span>
              </label>
              <select
                name="interval"
                value={formData.interval}
                onChange={handleChange}
                required
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                {INTERVAL_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {/* Day and Start Date */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-1">
                Tag des Monats
              </label>
              <select
                name="dayOfMonth"
                value={formData.dayOfMonth}
                onChange={handleChange}
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              >
                {Array.from({ length: 28 }, (_, i) => i + 1).map((day) => (
                  <option key={day} value={day}>
                    {day}.
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium mb-1">
                Startdatum <span className="text-destructive">*</span>
              </label>
              <input
                type="date"
                name="startDate"
                value={formData.startDate}
                onChange={handleChange}
                required
                className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              />
            </div>
          </div>

          {/* End Date */}
          <div>
            <label className="block text-sm font-medium mb-1">Enddatum</label>
            <input
              type="date"
              name="endDate"
              value={formData.endDate}
              onChange={handleChange}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
            />
            <p className="text-xs text-muted-foreground mt-1">
              Optional. Leer lassen für unbefristeten Plan.
            </p>
          </div>

          {/* Active Toggle (only in edit mode) */}
          {isEditMode && (
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="isActive"
                name="isActive"
                checked={formData.isActive}
                onChange={handleChange}
                className="h-4 w-4 rounded border-border text-primary focus:ring-primary"
              />
              <label htmlFor="isActive" className="text-sm font-medium">
                Aktiv
              </label>
            </div>
          )}

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
              disabled={
                isSubmitting ||
                !formData.name ||
                !formData.securityId ||
                !formData.accountId ||
                !formData.portfolioId ||
                !formData.amount
              }
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Speichern...' : isEditMode ? 'Speichern' : 'Erstellen'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
