/**
 * Modal for setting allocation targets for securities or classifications.
 * Targets trigger alerts when current weight deviates beyond threshold.
 */

import { useState, useEffect } from 'react';
import { X, Target, AlertTriangle } from 'lucide-react';
import { setAllocationTarget, getSecurities, getPortfolios } from '../../lib/api';
import type { SecurityData, PortfolioData, SetAllocationTargetRequest } from '../../lib/types';
import { useEscapeKey } from '../../lib/hooks';
import { toast } from '../../store';

interface AllocationTargetModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  // Pre-selected values (optional)
  preSelectedSecurityId?: number;
  preSelectedSecurityName?: string;
  preSelectedWeight?: number;
}

export function AllocationTargetModal({
  isOpen,
  onClose,
  onSuccess,
  preSelectedSecurityId,
  preSelectedSecurityName,
  preSelectedWeight,
}: AllocationTargetModalProps) {
  useEscapeKey(isOpen, onClose);

  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Form state
  const [form, setForm] = useState({
    portfolioId: '',
    securityId: preSelectedSecurityId ? String(preSelectedSecurityId) : '',
    targetWeight: preSelectedWeight ? String(preSelectedWeight.toFixed(1)) : '',
    threshold: '5', // Default 5%
  });

  // Load portfolios and securities
  useEffect(() => {
    if (isOpen) {
      setIsLoading(true);
      Promise.all([getPortfolios(), getSecurities()])
        .then(([portfolioData, securityData]) => {
          setPortfolios(portfolioData);
          setSecurities(securityData);
          // Auto-select first portfolio if available
          if (portfolioData.length > 0 && !form.portfolioId) {
            setForm(prev => ({ ...prev, portfolioId: String(portfolioData[0].id) }));
          }
        })
        .catch(err => {
          console.error('Failed to load data:', err);
          setError('Fehler beim Laden der Daten');
        })
        .finally(() => setIsLoading(false));
    }
  }, [isOpen]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setForm({
        portfolioId: portfolios.length > 0 ? String(portfolios[0].id) : '',
        securityId: preSelectedSecurityId ? String(preSelectedSecurityId) : '',
        targetWeight: preSelectedWeight ? String(preSelectedWeight.toFixed(1)) : '',
        threshold: '5',
      });
      setError(null);
    }
  }, [isOpen, preSelectedSecurityId, preSelectedWeight, portfolios]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
    const { name, value } = e.target;
    setForm(prev => ({ ...prev, [name]: value }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!form.portfolioId) {
      setError('Bitte ein Portfolio auswählen');
      return;
    }

    if (!form.securityId) {
      setError('Bitte ein Wertpapier auswählen');
      return;
    }

    const targetWeight = parseFloat(form.targetWeight);
    if (isNaN(targetWeight) || targetWeight < 0 || targetWeight > 100) {
      setError('Zielgewichtung muss zwischen 0 und 100% liegen');
      return;
    }

    const threshold = parseFloat(form.threshold);
    if (isNaN(threshold) || threshold < 0 || threshold > 50) {
      setError('Schwellenwert muss zwischen 0 und 50% liegen');
      return;
    }

    setIsSubmitting(true);

    try {
      const request: SetAllocationTargetRequest = {
        portfolioId: parseInt(form.portfolioId),
        securityId: parseInt(form.securityId),
        targetWeight: targetWeight / 100, // Convert to 0-1 range
        threshold: threshold / 100, // Convert to 0-1 range
      };

      await setAllocationTarget(request);
      toast.success('Zielgewichtung gespeichert');
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
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md mx-4">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2">
            <Target className="w-5 h-5 text-primary" />
            <h2 className="text-lg font-semibold">Zielgewichtung setzen</h2>
          </div>
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

          {isLoading ? (
            <div className="text-center py-8 text-muted-foreground">
              Lade Daten...
            </div>
          ) : (
            <>
              {/* Portfolio Selection */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Portfolio <span className="text-destructive">*</span>
                </label>
                <select
                  name="portfolioId"
                  value={form.portfolioId}
                  onChange={handleChange}
                  required
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                >
                  <option value="">Portfolio auswählen...</option>
                  {portfolios.map(p => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>

              {/* Security Selection */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Wertpapier <span className="text-destructive">*</span>
                </label>
                {preSelectedSecurityName ? (
                  <div className="px-3 py-2 border border-border rounded-md bg-muted">
                    {preSelectedSecurityName}
                  </div>
                ) : (
                  <select
                    name="securityId"
                    value={form.securityId}
                    onChange={handleChange}
                    required
                    className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                  >
                    <option value="">Wertpapier auswählen...</option>
                    {securities.map(s => (
                      <option key={s.id} value={s.id}>
                        {s.name} {s.ticker ? `(${s.ticker})` : ''}
                      </option>
                    ))}
                  </select>
                )}
              </div>

              {/* Target Weight */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Zielgewichtung (%) <span className="text-destructive">*</span>
                </label>
                <input
                  type="number"
                  name="targetWeight"
                  value={form.targetWeight}
                  onChange={handleChange}
                  required
                  step="0.1"
                  min="0"
                  max="100"
                  placeholder="z.B. 10"
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Der gewünschte Anteil dieses Wertpapiers am Portfolio
                </p>
              </div>

              {/* Threshold */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Schwellenwert (%) <span className="text-destructive">*</span>
                </label>
                <input
                  type="number"
                  name="threshold"
                  value={form.threshold}
                  onChange={handleChange}
                  required
                  step="0.5"
                  min="0.5"
                  max="50"
                  placeholder="5"
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Warnung bei Abweichung größer als ±{form.threshold || '5'}%
                </p>
              </div>

              {/* Preview */}
              {form.targetWeight && form.threshold && (
                <div className="p-3 bg-muted/50 rounded-md text-sm">
                  <div className="flex items-center gap-2 mb-2">
                    <AlertTriangle className="w-4 h-4 text-yellow-500" />
                    <span className="font-medium">Warnung wird ausgelöst bei:</span>
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-muted-foreground">
                    <div>
                      Untergewichtet: &lt; {(parseFloat(form.targetWeight) - parseFloat(form.threshold)).toFixed(1)}%
                    </div>
                    <div>
                      Übergewichtet: &gt; {(parseFloat(form.targetWeight) + parseFloat(form.threshold)).toFixed(1)}%
                    </div>
                  </div>
                </div>
              )}
            </>
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
              disabled={isSubmitting || isLoading || !form.portfolioId || !form.securityId || !form.targetWeight}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Speichern...' : 'Speichern'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
