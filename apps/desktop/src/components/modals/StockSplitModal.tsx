/**
 * Modal for recording stock splits.
 */

import { useState, useEffect } from 'react';
import { X, AlertTriangle, CheckCircle2, Loader2 } from 'lucide-react';
import { formatDate, type SecurityData, type StockSplitPreview, type ApplyStockSplitRequest } from '../../lib/types';
import { getSecurities, previewStockSplit, applyStockSplit } from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';

interface StockSplitModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  defaultSecurityId?: number;
}

export function StockSplitModal({
  isOpen,
  onClose,
  onSuccess,
  defaultSecurityId,
}: StockSplitModalProps) {
  useEscapeKey(isOpen, onClose);

  const [formData, setFormData] = useState({
    securityId: '',
    effectiveDate: new Date().toISOString().split('T')[0],
    ratioFrom: '1',
    ratioTo: '1',
    adjustPrices: true,
    adjustFifo: true,
  });

  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [preview, setPreview] = useState<StockSplitPreview | null>(null);
  const [step, setStep] = useState<'form' | 'preview' | 'success'>('form');

  const [isLoadingSecurities, setIsLoadingSecurities] = useState(false);
  const [isLoadingPreview, setIsLoadingPreview] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load securities when modal opens
  useEffect(() => {
    if (isOpen) {
      setIsLoadingSecurities(true);
      getSecurities()
        .then(setSecurities)
        .catch((err) => console.error('Failed to load securities:', err))
        .finally(() => setIsLoadingSecurities(false));
    }
  }, [isOpen]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      setFormData({
        securityId: defaultSecurityId ? String(defaultSecurityId) : '',
        effectiveDate: new Date().toISOString().split('T')[0],
        ratioFrom: '1',
        ratioTo: '1',
        adjustPrices: true,
        adjustFifo: true,
      });
      setPreview(null);
      setStep('form');
      setError(null);
    }
  }, [isOpen, defaultSecurityId]);

  // Filter to only show securities with holdings
  const availableSecurities = securities.filter((s) => !s.isRetired);

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

  const handlePreview = async () => {
    setError(null);
    setIsLoadingPreview(true);

    try {
      const result = await previewStockSplit(
        parseInt(formData.securityId),
        formData.effectiveDate,
        parseInt(formData.ratioFrom),
        parseInt(formData.ratioTo)
      );
      setPreview(result);
      setStep('preview');
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoadingPreview(false);
    }
  };

  const handleSubmit = async () => {
    setError(null);
    setIsSubmitting(true);

    try {
      const request: ApplyStockSplitRequest = {
        securityId: parseInt(formData.securityId),
        effectiveDate: formData.effectiveDate,
        ratioFrom: parseInt(formData.ratioFrom),
        ratioTo: parseInt(formData.ratioTo),
        adjustPrices: formData.adjustPrices,
        adjustFifo: formData.adjustFifo,
      };

      await applyStockSplit(request);
      setStep('success');
      setTimeout(() => {
        onSuccess();
        onClose();
      }, 1500);
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
          <h2 className="text-lg font-semibold">Aktiensplit erfassen</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-4">
          {error && (
            <div className="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
              {error}
            </div>
          )}

          {step === 'form' && (
            <div className="space-y-4">
              {/* Security Selection */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Wertpapier <span className="text-destructive">*</span>
                </label>
                <select
                  name="securityId"
                  value={formData.securityId}
                  onChange={handleChange}
                  required
                  disabled={isLoadingSecurities}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
                >
                  <option value="">Auswählen...</option>
                  {availableSecurities.map((s) => (
                    <option key={s.id} value={s.id}>
                      {s.name} {s.isin ? `(${s.isin})` : ''}
                    </option>
                  ))}
                </select>
              </div>

              {/* Effective Date */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Stichtag <span className="text-destructive">*</span>
                </label>
                <input
                  type="date"
                  name="effectiveDate"
                  value={formData.effectiveDate}
                  onChange={handleChange}
                  required
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                />
              </div>

              {/* Split Ratio */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Split-Verhältnis <span className="text-destructive">*</span>
                </label>
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    name="ratioFrom"
                    value={formData.ratioFrom}
                    onChange={handleChange}
                    required
                    min="1"
                    className="w-20 px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary text-center"
                  />
                  <span className="text-muted-foreground">:</span>
                  <input
                    type="number"
                    name="ratioTo"
                    value={formData.ratioTo}
                    onChange={handleChange}
                    required
                    min="1"
                    className="w-20 px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary text-center"
                  />
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  {parseInt(formData.ratioFrom)} alte Aktie(n) = {parseInt(formData.ratioTo)} neue Aktie(n)
                </p>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Beispiel: 1:4 Split (1 alte = 4 neue), 10:1 Reverse Split (10 alte = 1 neue)
                </p>
              </div>

              {/* Options */}
              <div className="space-y-2 pt-2">
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    id="adjustPrices"
                    name="adjustPrices"
                    checked={formData.adjustPrices}
                    onChange={handleChange}
                    className="h-4 w-4 rounded border-border text-primary focus:ring-primary"
                  />
                  <label htmlFor="adjustPrices" className="text-sm">
                    Historische Kurse anpassen
                  </label>
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    id="adjustFifo"
                    name="adjustFifo"
                    checked={formData.adjustFifo}
                    onChange={handleChange}
                    className="h-4 w-4 rounded border-border text-primary focus:ring-primary"
                  />
                  <label htmlFor="adjustFifo" className="text-sm">
                    FIFO-Lots anpassen
                  </label>
                </div>
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
                  type="button"
                  onClick={handlePreview}
                  disabled={
                    isLoadingPreview ||
                    !formData.securityId ||
                    !formData.ratioFrom ||
                    !formData.ratioTo
                  }
                  className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {isLoadingPreview ? (
                    <span className="flex items-center gap-2">
                      <Loader2 className="animate-spin" size={16} />
                      Vorschau...
                    </span>
                  ) : (
                    'Vorschau'
                  )}
                </button>
              </div>
            </div>
          )}

          {step === 'preview' && preview && (
            <div className="space-y-4">
              {/* Preview Header */}
              <div className="flex items-start gap-3 p-3 bg-amber-50 dark:bg-amber-900/20 rounded-md border border-amber-200 dark:border-amber-900/50">
                <AlertTriangle className="text-amber-600 mt-0.5" size={20} />
                <div>
                  <p className="font-medium text-amber-900 dark:text-amber-200">
                    Bitte überprüfen Sie die Änderungen
                  </p>
                  <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                    Diese Aktion kann nicht rückgängig gemacht werden.
                  </p>
                </div>
              </div>

              {/* Preview Details */}
              <div className="space-y-3">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Wertpapier:</span>
                  <span className="font-medium">{preview.securityName}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Stichtag:</span>
                  <span className="font-medium">
                    {formatDate(preview.effectiveDate)}
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Verhältnis:</span>
                  <span className="font-medium">{preview.ratioDisplay}</span>
                </div>
                <div className="h-px bg-border" />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Stück vorher:</span>
                  <span className="font-medium">
                    {preview.totalSharesBefore.toLocaleString('de-DE', {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 8,
                    })}
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Stück nachher:</span>
                  <span className="font-medium text-primary">
                    {preview.totalSharesAfter.toLocaleString('de-DE', {
                      minimumFractionDigits: 2,
                      maximumFractionDigits: 8,
                    })}
                  </span>
                </div>
                <div className="h-px bg-border" />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Betroffene Portfolios:</span>
                  <span className="font-medium">{preview.affectedPortfolios.length}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">FIFO-Lots:</span>
                  <span className="font-medium">{preview.fifoLotsCount}</span>
                </div>
                {formData.adjustPrices && (
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Kurse anzupassen:</span>
                    <span className="font-medium">{preview.pricesCount}</span>
                  </div>
                )}
              </div>

              {/* Affected Portfolios */}
              {preview.affectedPortfolios.length > 0 && (
                <div className="mt-4">
                  <h4 className="text-sm font-medium mb-2">Betroffene Portfolios:</h4>
                  <div className="space-y-1">
                    {preview.affectedPortfolios.map((p, i) => (
                      <div
                        key={i}
                        className="flex justify-between text-xs p-2 bg-muted rounded"
                      >
                        <span>{p.portfolioName}</span>
                        <span>
                          {p.sharesBefore.toFixed(4)} → {p.sharesAfter.toFixed(4)}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Actions */}
              <div className="flex justify-end gap-3 pt-4 border-t border-border">
                <button
                  type="button"
                  onClick={() => setStep('form')}
                  className="px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors"
                >
                  Zurück
                </button>
                <button
                  type="button"
                  onClick={handleSubmit}
                  disabled={isSubmitting}
                  className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {isSubmitting ? (
                    <span className="flex items-center gap-2">
                      <Loader2 className="animate-spin" size={16} />
                      Anwenden...
                    </span>
                  ) : (
                    'Split anwenden'
                  )}
                </button>
              </div>
            </div>
          )}

          {step === 'success' && (
            <div className="py-8 text-center">
              <CheckCircle2 className="w-16 h-16 text-green-500 mx-auto mb-4" />
              <h3 className="text-lg font-semibold mb-2">Aktiensplit erfasst</h3>
              <p className="text-muted-foreground">
                Der Aktiensplit wurde erfolgreich angewendet.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
