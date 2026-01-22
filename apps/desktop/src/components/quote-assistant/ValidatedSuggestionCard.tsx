/**
 * Card displaying a validated quote suggestion from the AI.
 * Shows validation status, test price, and apply button.
 */

import { Check, AlertTriangle, X, Loader2 } from 'lucide-react';
import type { ValidatedQuoteSuggestion } from '../../lib/types';
import { formatCurrency } from '../../lib/types';

interface ValidatedSuggestionCardProps {
  suggestion: ValidatedQuoteSuggestion;
  onApply: () => void;
  isApplying?: boolean;
}

export function ValidatedSuggestionCard({
  suggestion,
  onApply,
  isApplying,
}: ValidatedSuggestionCardProps) {
  const { suggestion: s, validated, testPrice, testDate, testCurrency, validationError } = suggestion;

  const confidencePercent = Math.round(s.confidence * 100);
  const confidenceColor =
    confidencePercent >= 80
      ? 'text-green-600'
      : confidencePercent >= 60
      ? 'text-yellow-600'
      : 'text-red-600';

  return (
    <div
      className={`rounded-lg border p-4 ${
        validated
          ? 'border-green-500/50 bg-green-500/5'
          : 'border-orange-500/50 bg-orange-500/5'
      }`}
    >
      {/* Header */}
      <div className="flex items-center gap-2 mb-3">
        {validated ? (
          <Check className="h-5 w-5 text-green-600" />
        ) : (
          <AlertTriangle className="h-5 w-5 text-orange-500" />
        )}
        <span className="font-medium">
          {validated ? 'Validierte Konfiguration' : 'Vorschlag (nicht validiert)'}
        </span>
      </div>

      {/* Provider and Symbol */}
      <div className="space-y-2 mb-3">
        <div className="flex justify-between items-center">
          <span className="text-sm text-muted-foreground">Provider:</span>
          <span className="font-mono text-sm">{s.provider}</span>
        </div>
        <div className="flex justify-between items-center">
          <span className="text-sm text-muted-foreground">Symbol:</span>
          <span className="font-mono text-sm">
            {s.ticker}
            {s.feedUrl && <span className="text-muted-foreground">{s.feedUrl}</span>}
          </span>
        </div>
        <div className="flex justify-between items-center">
          <span className="text-sm text-muted-foreground">Konfidenz:</span>
          <span className={`text-sm font-medium ${confidenceColor}`}>
            {confidencePercent}%
          </span>
        </div>
      </div>

      {/* Validation Result */}
      {validated && testPrice !== undefined && (
        <div className="bg-green-500/10 rounded p-2 mb-3">
          <div className="text-sm text-green-700 dark:text-green-400">
            Test erfolgreich!
          </div>
          <div className="text-lg font-semibold">
            {formatCurrency(testPrice, testCurrency || 'EUR')}
          </div>
          {testDate && (
            <div className="text-xs text-muted-foreground">
              Stand: {testDate}
            </div>
          )}
        </div>
      )}

      {/* Validation Error */}
      {!validated && validationError && (
        <div className="bg-orange-500/10 rounded p-2 mb-3 flex items-start gap-2">
          <X className="h-4 w-4 text-orange-500 mt-0.5 flex-shrink-0" />
          <div className="text-sm text-orange-700 dark:text-orange-400">
            {validationError}
          </div>
        </div>
      )}

      {/* Reason */}
      <div className="text-sm text-muted-foreground mb-4">
        <span className="font-medium">Grund:</span> {s.reason}
      </div>

      {/* Apply Button */}
      {validated && (
        <button
          onClick={onApply}
          disabled={isApplying}
          className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-green-600
                     text-white rounded-lg hover:bg-green-700 disabled:opacity-50
                     disabled:cursor-not-allowed transition-colors"
        >
          {isApplying ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Wird angewendet...
            </>
          ) : (
            <>
              <Check className="h-4 w-4" />
              Übernehmen
            </>
          )}
        </button>
      )}
    </div>
  );
}
