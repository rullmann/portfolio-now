/**
 * AlertsPanel - Full panel for viewing and managing allocation alerts
 * Shows current alerts and allows setting allocation targets
 */

import { useEffect, useState } from 'react';
import {
  AlertTriangle,
  CheckCircle,
  RefreshCw,
  Trash2,
  Plus,
  Settings2,
  Target,
} from 'lucide-react';
import {
  getAllocationAlerts,
  getAllocationTargets,
  deleteAllocationTarget,
} from '../../lib/api';
import type { AllocationAlert, AllocationTarget } from '../../lib/types';

interface AlertsPanelProps {
  portfolioId?: number;
  onAddTarget?: () => void;
  className?: string;
}

export function AlertsPanel({ portfolioId, onAddTarget, className = '' }: AlertsPanelProps) {
  const [alerts, setAlerts] = useState<AllocationAlert[]>([]);
  const [targets, setTargets] = useState<AllocationTarget[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'alerts' | 'targets'>('alerts');

  const loadData = async () => {
    setLoading(true);
    setError(null);
    try {
      const [alertsData, targetsData] = await Promise.all([
        getAllocationAlerts(portfolioId),
        portfolioId ? getAllocationTargets(portfolioId) : Promise.resolve([]),
      ]);
      setAlerts(alertsData);
      setTargets(targetsData);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, [portfolioId]);

  const handleDeleteTarget = async (targetId: number) => {
    try {
      await deleteAllocationTarget(targetId);
      await loadData();
    } catch (err) {
      console.error('Failed to delete target:', err);
    }
  };

  const criticalCount = alerts.filter((a) => a.severity === 'critical').length;
  const warningCount = alerts.filter((a) => a.severity === 'warning').length;

  if (loading) {
    return (
      <div className={`flex items-center justify-center p-8 ${className}`}>
        <RefreshCw className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className={`p-4 ${className}`}>
        <div className="text-center text-sm text-muted-foreground">
          <p className="text-red-500">{error}</p>
          <button onClick={loadData} className="mt-2 text-primary hover:underline">
            Erneut versuchen
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex flex-col ${className}`}>
      {/* Header with tabs */}
      <div className="flex items-center justify-between border-b border-border pb-2 mb-4">
        <div className="flex items-center gap-4">
          <button
            onClick={() => setActiveTab('alerts')}
            className={`text-sm font-medium pb-2 border-b-2 transition-colors ${
              activeTab === 'alerts'
                ? 'border-primary text-foreground'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            Warnungen
            {alerts.length > 0 && (
              <span className="ml-2 px-1.5 py-0.5 text-xs rounded bg-muted">
                {alerts.length}
              </span>
            )}
          </button>
          <button
            onClick={() => setActiveTab('targets')}
            className={`text-sm font-medium pb-2 border-b-2 transition-colors ${
              activeTab === 'targets'
                ? 'border-primary text-foreground'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            Zielgewichtungen
            {targets.length > 0 && (
              <span className="ml-2 px-1.5 py-0.5 text-xs rounded bg-muted">
                {targets.length}
              </span>
            )}
          </button>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={loadData}
            className="p-1.5 rounded hover:bg-accent"
            title="Aktualisieren"
          >
            <RefreshCw className="h-4 w-4 text-muted-foreground" />
          </button>
          {onAddTarget && (
            <button
              onClick={onAddTarget}
              className="flex items-center gap-1 px-2 py-1 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90"
            >
              <Plus className="h-4 w-4" />
              Ziel
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      {activeTab === 'alerts' ? (
        <div className="flex-1 overflow-auto">
          {alerts.length > 0 ? (
            <>
              {/* Summary */}
              <div className="flex items-center gap-2 mb-4 text-sm">
                {criticalCount > 0 && (
                  <span className="px-2 py-1 rounded bg-red-500/10 text-red-600">
                    {criticalCount} kritisch
                  </span>
                )}
                {warningCount > 0 && (
                  <span className="px-2 py-1 rounded bg-yellow-500/10 text-yellow-600">
                    {warningCount} Warnung{warningCount > 1 ? 'en' : ''}
                  </span>
                )}
              </div>

              {/* Alert list */}
              <div className="space-y-3">
                {alerts.map((alert, index) => (
                  <div
                    key={`${alert.entityName}-${index}`}
                    className={`flex items-start gap-3 p-3 rounded-lg border ${
                      alert.severity === 'critical'
                        ? 'border-red-500/30 bg-red-500/5'
                        : 'border-yellow-500/30 bg-yellow-500/5'
                    }`}
                  >
                    <AlertTriangle
                      className={`h-5 w-5 shrink-0 mt-0.5 ${
                        alert.severity === 'critical' ? 'text-red-500' : 'text-yellow-500'
                      }`}
                    />
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">{alert.entityName}</div>
                      <div className="mt-1 grid grid-cols-3 gap-2 text-sm text-muted-foreground">
                        <div>
                          <span className="text-xs uppercase tracking-wide">Typ</span>
                          <div className={alert.alertType === 'over_weight' ? 'text-red-500' : 'text-yellow-500'}>
                            {alert.alertType === 'over_weight' ? 'Übergewichtet' : 'Untergewichtet'}
                          </div>
                        </div>
                        <div>
                          <span className="text-xs uppercase tracking-wide">Ziel</span>
                          <div>{(alert.targetWeight * 100).toFixed(1)}%</div>
                        </div>
                        <div>
                          <span className="text-xs uppercase tracking-wide">Aktuell</span>
                          <div>{(alert.currentWeight * 100).toFixed(1)}%</div>
                        </div>
                      </div>
                      <div className="mt-2 text-sm">
                        Abweichung:{' '}
                        <span className={alert.deviation > 0 ? 'text-red-500' : 'text-yellow-500'}>
                          {alert.deviation > 0 ? '+' : ''}
                          {(alert.deviation * 100).toFixed(1)} Prozentpunkte
                        </span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <CheckCircle className="h-12 w-12 text-green-500 mb-4" />
              <h3 className="font-medium">Keine Warnungen</h3>
              <p className="text-sm text-muted-foreground mt-1">
                Alle Allokationen sind im Zielbereich
              </p>
            </div>
          )}
        </div>
      ) : (
        <div className="flex-1 overflow-auto">
          {targets.length > 0 ? (
            <div className="space-y-2">
              {targets.map((target) => (
                <div
                  key={target.id}
                  className="flex items-center justify-between p-3 rounded-lg border border-border bg-card"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <Target className="h-5 w-5 text-primary shrink-0" />
                    <div className="min-w-0">
                      <div className="font-medium truncate">
                        {target.securityName || target.classificationName || 'Unbekannt'}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        Ziel: {(target.targetWeight * 100).toFixed(1)}% ±{' '}
                        {(target.threshold * 100).toFixed(1)}%
                      </div>
                    </div>
                  </div>
                  <button
                    onClick={() => handleDeleteTarget(target.id)}
                    className="p-1.5 rounded hover:bg-accent text-muted-foreground hover:text-red-500"
                    title="Zielgewichtung löschen"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <Settings2 className="h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="font-medium">Keine Zielgewichtungen</h3>
              <p className="text-sm text-muted-foreground mt-1">
                Definieren Sie Zielgewichtungen für Securities oder Klassifikationen
              </p>
              {onAddTarget && (
                <button
                  onClick={onAddTarget}
                  className="mt-4 flex items-center gap-2 px-3 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
                >
                  <Plus className="h-4 w-4" />
                  Zielgewichtung hinzufügen
                </button>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
