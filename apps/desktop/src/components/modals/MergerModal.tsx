/**
 * Modal for recording mergers and acquisitions.
 */

import { useState, useEffect } from 'react';
import { X, AlertTriangle, CheckCircle2, Loader2, GitMerge } from 'lucide-react';
import { formatDate, type SecurityData } from '../../lib/types';
import type { MergerPreview, ApplyMergerRequest } from '../../lib/api';
import { getSecurities, previewMerger, applyMerger } from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';

interface MergerModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  defaultSourceSecurityId?: number;
}

export function MergerModal({
  isOpen,
  onClose,
  onSuccess,
  defaultSourceSecurityId,
}: MergerModalProps) {
  useEscapeKey(isOpen, onClose);

  const [formData, setFormData] = useState({
    sourceSecurityId: '',
    targetSecurityId: '',
    effectiveDate: new Date().toISOString().split('T')[0],
    shareRatio: '1',
    cashPerShare: '0',
    cashCurrency: 'EUR',
    note: '',
  });

  const [securities, setSecurities] = useState<SecurityData[]>([]);
  const [preview, setPreview] = useState<MergerPreview | null>(null);
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
        sourceSecurityId: defaultSourceSecurityId ? String(defaultSourceSecurityId) : '',
        targetSecurityId: '',
        effectiveDate: new Date().toISOString().split('T')[0],
        shareRatio: '1',
        cashPerShare: '0',
        cashCurrency: 'EUR',
        note: '',
      });
      setPreview(null);
      setStep('form');
      setError(null);
    }
  }, [isOpen, defaultSourceSecurityId]);

  // Filter to only show securities
  const availableSecurities = securities.filter((s) => !s.isRetired);

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>
  ) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
  };

  const handlePreview = async () => {
    setError(null);
    setIsLoadingPreview(true);

    try {
      const cashInCents = Math.round(parseFloat(formData.cashPerShare) * 100);
      const result = await previewMerger(
        parseInt(formData.sourceSecurityId),
        parseInt(formData.targetSecurityId),
        formData.effectiveDate,
        parseFloat(formData.shareRatio),
        cashInCents,
        formData.cashCurrency || undefined
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
      const cashInCents = Math.round(parseFloat(formData.cashPerShare) * 100);
      const request: ApplyMergerRequest = {
        sourceSecurityId: parseInt(formData.sourceSecurityId),
        targetSecurityId: parseInt(formData.targetSecurityId),
        effectiveDate: formData.effectiveDate,
        shareRatio: parseFloat(formData.shareRatio),
        cashPerShare: cashInCents,
        cashCurrency: formData.cashCurrency || undefined,
        note: formData.note || undefined,
      };

      await applyMerger(request);
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

  const formatCurrency = (cents: number, currency: string) => {
    return (cents / 100).toLocaleString('de-DE', {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  };

  const formatShares = (shares: number) => {
    return (shares / 100_000_000).toLocaleString('de-DE', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 8,
    });
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
          <div className="flex items-center gap-2">
            <GitMerge className="w-5 h-5 text-primary" />
            <h2 className="text-lg font-semibold">Fusion / Übernahme</h2>
          </div>
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
              {/* Source Security */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Ursprungswertpapier <span className="text-destructive">*</span>
                </label>
                <select
                  name="sourceSecurityId"
                  value={formData.sourceSecurityId}
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
                <p className="text-xs text-muted-foreground mt-1">
                  Das Wertpapier, das übernommen wird
                </p>
              </div>

              {/* Target Security */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Zielwertpapier <span className="text-destructive">*</span>
                </label>
                <select
                  name="targetSecurityId"
                  value={formData.targetSecurityId}
                  onChange={handleChange}
                  required
                  disabled={isLoadingSecurities}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
                >
                  <option value="">Auswählen...</option>
                  {availableSecurities
                    .filter((s) => String(s.id) !== formData.sourceSecurityId)
                    .map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.name} {s.isin ? `(${s.isin})` : ''}
                      </option>
                    ))}
                </select>
                <p className="text-xs text-muted-foreground mt-1">
                  Das Wertpapier, das Sie erhalten
                </p>
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

              {/* Share Ratio */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Umtauschverhältnis <span className="text-destructive">*</span>
                </label>
                <div className="flex items-center gap-2">
                  <span className="text-sm text-muted-foreground">1 Ursprungsaktie =</span>
                  <input
                    type="number"
                    name="shareRatio"
                    value={formData.shareRatio}
                    onChange={handleChange}
                    required
                    min="0.0001"
                    step="0.0001"
                    className="w-28 px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary text-center"
                  />
                  <span className="text-sm text-muted-foreground">Zielaktien</span>
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  Beispiel: 0.5 bedeutet 2 Ursprungsaktien = 1 Zielaktie
                </p>
              </div>

              {/* Cash Component */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Barabfindung pro Aktie
                </label>
                <div className="flex items-center gap-2">
                  <input
                    type="number"
                    name="cashPerShare"
                    value={formData.cashPerShare}
                    onChange={handleChange}
                    min="0"
                    step="0.01"
                    className="w-28 px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary text-right"
                  />
                  <select
                    name="cashCurrency"
                    value={formData.cashCurrency}
                    onChange={handleChange}
                    className="w-20 px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                  >
                    <option value="EUR">EUR</option>
                    <option value="USD">USD</option>
                    <option value="CHF">CHF</option>
                    <option value="GBP">GBP</option>
                  </select>
                </div>
                <p className="text-xs text-muted-foreground mt-1">
                  Zusätzliche Barzahlung pro Ursprungsaktie (falls vorhanden)
                </p>
              </div>

              {/* Note */}
              <div>
                <label className="block text-sm font-medium mb-1">Notiz</label>
                <textarea
                  name="note"
                  value={formData.note}
                  onChange={handleChange}
                  rows={2}
                  placeholder="Optionale Notiz zur Transaktion..."
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
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
                  type="button"
                  onClick={handlePreview}
                  disabled={
                    isLoadingPreview ||
                    !formData.sourceSecurityId ||
                    !formData.targetSecurityId ||
                    !formData.shareRatio ||
                    formData.sourceSecurityId === formData.targetSecurityId
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
                    Diese Aktion erstellt Auslieferungs- und Einlieferungstransaktionen.
                  </p>
                </div>
              </div>

              {/* Preview Details */}
              <div className="space-y-3">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Ursprung:</span>
                  <span className="font-medium">{preview.sourceSecurityName}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Ziel:</span>
                  <span className="font-medium">{preview.targetSecurityName}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Stichtag:</span>
                  <span className="font-medium">
                    {formatDate(preview.effectiveDate)}
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Verhältnis:</span>
                  <span className="font-medium">
                    1 : {preview.shareRatio.toLocaleString('de-DE', { maximumFractionDigits: 4 })}
                  </span>
                </div>
                {preview.cashPerShare > 0 && (
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Barabfindung/Aktie:</span>
                    <span className="font-medium">
                      {formatCurrency(preview.cashPerShare, preview.cashCurrency)}
                    </span>
                  </div>
                )}
                <div className="h-px bg-border" />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Ursprungsaktien:</span>
                  <span className="font-medium text-red-600">
                    - {formatShares(preview.totalSourceShares)}
                  </span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Zielaktien:</span>
                  <span className="font-medium text-green-600">
                    + {formatShares(preview.totalTargetShares)}
                  </span>
                </div>
                {preview.totalCashAmount > 0 && (
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Barabfindung gesamt:</span>
                    <span className="font-medium text-green-600">
                      + {formatCurrency(preview.totalCashAmount, preview.cashCurrency)}
                    </span>
                  </div>
                )}
                <div className="h-px bg-border" />
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Einstandswert übertragen:</span>
                  <span className="font-medium">
                    {formatCurrency(preview.totalCostBasisTransferred, 'EUR')}
                  </span>
                </div>
              </div>

              {/* Affected Portfolios */}
              {preview.affectedPortfolios.length > 0 && (
                <div className="mt-4">
                  <h4 className="text-sm font-medium mb-2">Betroffene Portfolios:</h4>
                  <div className="space-y-2">
                    {preview.affectedPortfolios.map((p) => (
                      <div
                        key={p.portfolioId}
                        className="text-xs p-2 bg-muted rounded space-y-1"
                      >
                        <div className="font-medium">{p.portfolioName}</div>
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">Ursprung:</span>
                          <span className="text-red-600">- {formatShares(p.sourceShares)}</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">Ziel:</span>
                          <span className="text-green-600">+ {formatShares(p.targetShares)}</span>
                        </div>
                        {p.cashAmount > 0 && (
                          <div className="flex justify-between">
                            <span className="text-muted-foreground">Bar:</span>
                            <span className="text-green-600">
                              + {formatCurrency(p.cashAmount, preview.cashCurrency)}
                            </span>
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {preview.affectedPortfolios.length === 0 && (
                <div className="p-3 bg-muted rounded-md text-sm text-muted-foreground text-center">
                  Keine Positionen im Ursprungswertpapier gefunden.
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
                  disabled={isSubmitting || preview.affectedPortfolios.length === 0}
                  className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {isSubmitting ? (
                    <span className="flex items-center gap-2">
                      <Loader2 className="animate-spin" size={16} />
                      Anwenden...
                    </span>
                  ) : (
                    'Fusion anwenden'
                  )}
                </button>
              </div>
            </div>
          )}

          {step === 'success' && (
            <div className="py-8 text-center">
              <CheckCircle2 className="w-16 h-16 text-green-500 mx-auto mb-4" />
              <h3 className="text-lg font-semibold mb-2">Fusion erfasst</h3>
              <p className="text-muted-foreground">
                Die Fusion wurde erfolgreich angewendet.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
