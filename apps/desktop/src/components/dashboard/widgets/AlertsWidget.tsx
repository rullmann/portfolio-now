/**
 * Alerts Widget - Shows allocation warnings from backend
 */

import { useEffect, useState } from 'react';
import { AlertTriangle, CheckCircle, RefreshCw } from 'lucide-react';
import { getAllocationAlerts } from '../../../lib/api';
import type { AllocationAlert } from '../../../lib/types';
import type { WidgetProps } from '../types';

export function AlertsWidget({ config }: WidgetProps) {
  // Extract portfolioId from widget settings if provided
  const portfolioId = config.settings?.portfolioId as number | undefined;
  const [alerts, setAlerts] = useState<AllocationAlert[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadAlerts = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getAllocationAlerts(portfolioId);
      setAlerts(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAlerts();
  }, [portfolioId]);

  const criticalCount = alerts.filter(a => a.severity === 'critical').length;
  const warningCount = alerts.filter(a => a.severity === 'warning').length;

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
          Warnungen
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center text-sm text-muted-foreground">
            <p>{error}</p>
            <button
              onClick={loadAlerts}
              className="mt-2 text-primary hover:underline"
            >
              Erneut versuchen
            </button>
          </div>
        </div>
      </div>
    );
  }

  const hasAlerts = alerts.length > 0;

  return (
    <div className="h-full flex flex-col p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="text-xs text-muted-foreground uppercase tracking-wide">
          Warnungen
        </div>
        {hasAlerts && (
          <div className="flex items-center gap-2 text-xs">
            {criticalCount > 0 && (
              <span className="px-1.5 py-0.5 rounded bg-red-500/10 text-red-600">
                {criticalCount} kritisch
              </span>
            )}
            {warningCount > 0 && (
              <span className="px-1.5 py-0.5 rounded bg-yellow-500/10 text-yellow-600">
                {warningCount} Warnung{warningCount > 1 ? 'en' : ''}
              </span>
            )}
          </div>
        )}
      </div>

      {hasAlerts ? (
        <div className="flex-1 overflow-auto space-y-2">
          {alerts.map((alert, index) => (
            <div
              key={`${alert.entityName}-${index}`}
              className={`flex items-start gap-2 p-2 rounded text-sm ${
                alert.severity === 'critical'
                  ? 'bg-red-500/10 text-red-600'
                  : 'bg-yellow-500/10 text-yellow-600'
              }`}
            >
              <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
              <div className="min-w-0 flex-1">
                <div className="font-medium truncate">{alert.entityName}</div>
                <div className="text-xs opacity-75">
                  {alert.alertType === 'over_weight' ? 'Übergewichtet' : 'Untergewichtet'}
                  {' • '}
                  Ziel: {(alert.targetWeight * 100).toFixed(1)}%
                  {' • '}
                  Aktuell: {(alert.currentWeight * 100).toFixed(1)}%
                </div>
                <div className="text-xs opacity-75">
                  Abweichung: {alert.deviation > 0 ? '+' : ''}{(alert.deviation * 100).toFixed(1)}%
                </div>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <CheckCircle className="h-8 w-8 text-green-500 mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">
              Keine Warnungen
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Zielgewichtungen in Rebalancing setzen
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
