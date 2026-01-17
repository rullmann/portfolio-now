/**
 * Modal for auditing existing quote configurations.
 * Shows securities with missing or stale prices.
 */

import { useState, useEffect, useCallback } from 'react';
import { ClipboardCheck, Check, RefreshCw, AlertCircle, Clock, XCircle } from 'lucide-react';
import { useEscapeKey } from '../../lib/hooks';
import { auditQuoteConfigurations, type QuoteAuditSummary, type QuoteConfigAuditResult } from '../../lib/api';

interface QuoteAuditModalProps {
  isOpen: boolean;
  onClose: () => void;
  onComplete: () => void;
}

function StatusBadge({ status, days }: { status: string; days?: number }) {
  if (status === 'missing') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-red-100 text-red-700">
        <XCircle size={12} />
        Keine Kurse
      </span>
    );
  }
  if (status === 'stale') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-yellow-100 text-yellow-700">
        <Clock size={12} />
        {days} Tage alt
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium rounded bg-green-100 text-green-700">
      <Check size={12} />
      OK
    </span>
  );
}

function SecurityRow({ result }: { result: QuoteConfigAuditResult }) {
  return (
    <div className="flex items-center justify-between p-3 border border-border rounded-lg bg-card">
      <div className="flex-1 min-w-0">
        <div className="font-medium truncate">{result.securityName}</div>
        <div className="text-xs text-muted-foreground flex items-center gap-2">
          <span>{result.feed}</span>
          {result.ticker && <span className="font-mono">{result.ticker}</span>}
          {result.lastPriceDate && (
            <span className="text-muted-foreground/70">Letzter Kurs: {result.lastPriceDate}</span>
          )}
        </div>
      </div>
      <StatusBadge status={result.status} days={result.daysSinceLastPrice} />
    </div>
  );
}

export function QuoteAuditModal({
  isOpen,
  onClose,
  onComplete: _onComplete,
}: QuoteAuditModalProps) {
  const [auditData, setAuditData] = useState<QuoteAuditSummary | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(isOpen, onClose);

  useEffect(() => {
    if (isOpen) {
      loadAuditData();
    }
  }, [isOpen]);

  const loadAuditData = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const result = await auditQuoteConfigurations(true);
      setAuditData(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, []);

  const handleClose = () => {
    onClose();
  };

  if (!isOpen) return null;

  const hasIssues = auditData && (auditData.staleCount > 0 || auditData.missingCount > 0);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={handleClose}
      />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-lg w-full max-w-xl mx-4 max-h-[85vh] flex flex-col overflow-hidden">
        {/* Header */}
        <div className="bg-primary/10 p-6 text-center flex-shrink-0">
          <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <ClipboardCheck size={32} className="text-primary" />
          </div>
          <h2 className="text-xl font-semibold">Kursquellen-Prüfung</h2>
          {auditData && (
            <p className="text-muted-foreground mt-2">
              {auditData.totalAudited} Wertpapiere geprüft
            </p>
          )}
        </div>

        {/* Content */}
        <div className="p-6 flex-1 overflow-y-auto">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-8">
              <RefreshCw size={32} className="text-primary animate-spin mb-4" />
              <p className="text-muted-foreground">Prüfe Kursdaten...</p>
            </div>
          ) : error ? (
            <div className="flex items-start gap-3 p-4 bg-destructive/10 border border-destructive/20 rounded-md">
              <AlertCircle className="text-destructive flex-shrink-0 mt-0.5" size={18} />
              <div className="text-sm">{error}</div>
            </div>
          ) : auditData && auditData.totalAudited === 0 ? (
            <div className="text-center py-8">
              <AlertCircle size={48} className="text-muted-foreground mx-auto mb-4" />
              <p className="font-medium">Keine konfigurierten Kursquellen gefunden</p>
              <p className="text-muted-foreground mt-2 text-sm">
                Konfiguriere zuerst Kursquellen für deine Wertpapiere.
              </p>
            </div>
          ) : auditData ? (
            <div className="space-y-4">
              {/* Summary Stats */}
              <div className="grid grid-cols-3 gap-3 text-center">
                <div className="bg-green-50 border border-green-200 rounded-lg p-3">
                  <div className="text-2xl font-bold text-green-700">{auditData.okCount}</div>
                  <div className="text-xs text-green-600">Aktuell</div>
                </div>
                <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-3">
                  <div className="text-2xl font-bold text-yellow-700">{auditData.staleCount}</div>
                  <div className="text-xs text-yellow-600">Veraltet (&gt;7 Tage)</div>
                </div>
                <div className="bg-red-50 border border-red-200 rounded-lg p-3">
                  <div className="text-2xl font-bold text-red-700">{auditData.missingCount}</div>
                  <div className="text-xs text-red-600">Keine Kurse</div>
                </div>
              </div>

              {/* Results List - only shows issues */}
              {hasIssues ? (
                <div className="space-y-2">
                  <div className="text-sm font-medium text-muted-foreground">
                    {auditData.results.length} Wertpapiere mit Problemen:
                  </div>
                  {auditData.results.map((result) => (
                    <SecurityRow key={result.securityId} result={result} />
                  ))}
                </div>
              ) : (
                <div className="text-center py-8 bg-green-50 border border-green-200 rounded-lg">
                  <Check size={48} className="text-green-600 mx-auto mb-4" />
                  <p className="font-medium text-green-700">Alle Kurse aktuell</p>
                  <p className="text-green-600 mt-2 text-sm">
                    Keine fehlenden oder veralteten Kurse gefunden.
                  </p>
                </div>
              )}
            </div>
          ) : null}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-border flex-shrink-0">
          <div className="flex gap-3">
            <button
              type="button"
              onClick={loadAuditData}
              disabled={isLoading}
              className="px-4 py-2 border border-border rounded-lg hover:bg-muted transition-colors disabled:opacity-50"
            >
              <RefreshCw size={16} className={`inline mr-1 ${isLoading ? 'animate-spin' : ''}`} />
              Aktualisieren
            </button>
            <button
              type="button"
              onClick={handleClose}
              className="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
            >
              Schließen
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
