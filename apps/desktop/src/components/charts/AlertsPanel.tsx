/**
 * Alerts Panel
 * UI for managing price alerts for a security
 */

import { useState, useEffect, useCallback } from 'react';
import {
  ChevronDown,
  ChevronUp,
  Bell,
  BellRing,
  Plus,
  Trash2,
  Power,
  TrendingUp,
  TrendingDown,
  AlertTriangle,
} from 'lucide-react';
import {
  getPriceAlerts,
  createPriceAlert,
  deletePriceAlert,
  togglePriceAlert,
  resetAlertTrigger,
} from '../../lib/api';
import { formatDateTime } from '../../lib/types';
import type { PriceAlert, AlertType, CreateAlertRequest } from '../../lib/types';

// ============================================================================
// Types
// ============================================================================

interface AlertsPanelProps {
  securityId: number | null;
  currentPrice?: number;
  currency?: string;
}

interface AlertTypeInfo {
  type: AlertType;
  label: string;
  description: string;
  icon: typeof TrendingUp;
  direction: 'bullish' | 'bearish' | 'neutral';
}

// ============================================================================
// Alert Type Definitions
// ============================================================================

const alertTypes: AlertTypeInfo[] = [
  {
    type: 'price_above',
    label: 'Kurs über',
    description: 'Alert wenn Kurs über Zielwert steigt',
    icon: TrendingUp,
    direction: 'bullish',
  },
  {
    type: 'price_below',
    label: 'Kurs unter',
    description: 'Alert wenn Kurs unter Zielwert fällt',
    icon: TrendingDown,
    direction: 'bearish',
  },
  {
    type: 'resistance_break',
    label: 'Widerstand durchbrochen',
    description: 'Alert bei Ausbruch über Widerstandslevel',
    icon: TrendingUp,
    direction: 'bullish',
  },
  {
    type: 'support_break',
    label: 'Support gebrochen',
    description: 'Alert bei Durchbruch unter Supportlevel',
    icon: TrendingDown,
    direction: 'bearish',
  },
];

// ============================================================================
// Alert Item Component
// ============================================================================

function AlertItem({
  alert,
  currency,
  onToggle,
  onDelete,
  onReset,
}: {
  alert: PriceAlert;
  currency?: string;
  onToggle: () => void;
  onDelete: () => void;
  onReset: () => void;
}) {
  const typeInfo = alertTypes.find(t => t.type === alert.alertType);
  const Icon = typeInfo?.icon || AlertTriangle;

  const directionColors = {
    bullish: 'text-green-500',
    bearish: 'text-red-500',
    neutral: 'text-amber-500',
  };

  return (
    <div
      className={`p-2 rounded-lg border ${
        alert.isActive
          ? alert.isTriggered
            ? 'border-amber-500/50 bg-amber-500/10'
            : 'border-border bg-card'
          : 'border-border/50 bg-muted/30 opacity-60'
      }`}
    >
      <div className="flex items-start gap-2">
        <div className={`mt-0.5 ${directionColors[typeInfo?.direction || 'neutral']}`}>
          {alert.isTriggered ? (
            <BellRing size={14} className="animate-pulse" />
          ) : (
            <Icon size={14} />
          )}
        </div>

        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-0.5">
            <span className="text-xs font-medium">{typeInfo?.label || alert.alertType}</span>
            <span className="text-xs font-mono font-semibold">
              {currency} {alert.targetValue.toFixed(2)}
            </span>
          </div>

          {alert.isTriggered && alert.lastTriggeredAt && (
            <div className="text-[10px] text-amber-600 dark:text-amber-400 mb-1">
              Ausgelöst: {formatDateTime(alert.lastTriggeredAt)}
              {alert.lastTriggeredPrice && ` bei ${currency} ${alert.lastTriggeredPrice.toFixed(2)}`}
            </div>
          )}

          {alert.note && (
            <p className="text-[10px] text-muted-foreground truncate">{alert.note}</p>
          )}

          <div className="flex items-center gap-1 mt-1.5">
            <button
              onClick={onToggle}
              className={`p-1 rounded hover:bg-muted transition-colors ${
                alert.isActive ? 'text-green-500' : 'text-muted-foreground'
              }`}
              title={alert.isActive ? 'Deaktivieren' : 'Aktivieren'}
            >
              <Power size={12} />
            </button>

            {alert.isTriggered && (
              <button
                onClick={onReset}
                className="p-1 rounded hover:bg-muted transition-colors text-amber-500"
                title="Zurücksetzen"
              >
                <Bell size={12} />
              </button>
            )}

            <button
              onClick={onDelete}
              className="p-1 rounded hover:bg-red-500/10 transition-colors text-red-500"
              title="Löschen"
            >
              <Trash2 size={12} />
            </button>

            <span className="text-[10px] text-muted-foreground ml-auto">
              {alert.triggerCount}x ausgelöst
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Create Alert Form
// ============================================================================

function CreateAlertForm({
  securityId,
  currentPrice,
  currency,
  onCreated,
  onCancel,
}: {
  securityId: number;
  currentPrice?: number;
  currency?: string;
  onCreated: (alert: PriceAlert) => void;
  onCancel: () => void;
}) {
  const [alertType, setAlertType] = useState<AlertType>('price_above');
  const [targetValue, setTargetValue] = useState(currentPrice?.toString() || '');
  const [note, setNote] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!targetValue) return;

    setIsLoading(true);
    try {
      const request: CreateAlertRequest = {
        securityId,
        alertType,
        targetValue: parseFloat(targetValue),
        note: note || undefined,
      };

      const alert = await createPriceAlert(request);
      onCreated(alert);
    } catch (err) {
      console.error('Failed to create alert:', err);
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="p-2 space-y-2 border-t border-border">
      <div className="text-xs font-medium text-muted-foreground mb-2">Neuen Alert erstellen</div>

      {/* Alert Type */}
      <select
        value={alertType}
        onChange={e => setAlertType(e.target.value as AlertType)}
        className="w-full px-2 py-1.5 text-xs bg-muted border-none rounded focus:outline-none focus:ring-2 focus:ring-primary"
      >
        {alertTypes.map(type => (
          <option key={type.type} value={type.type}>
            {type.label}
          </option>
        ))}
      </select>

      {/* Target Value */}
      <div className="flex items-center gap-2">
        <input
          type="number"
          value={targetValue}
          onChange={e => setTargetValue(e.target.value)}
          placeholder="Zielwert"
          step="0.01"
          className="flex-1 px-2 py-1.5 text-xs bg-muted border-none rounded focus:outline-none focus:ring-2 focus:ring-primary"
        />
        <span className="text-xs text-muted-foreground">{currency}</span>
      </div>

      {/* Quick Buttons */}
      {currentPrice && (
        <div className="flex gap-1">
          <button
            type="button"
            onClick={() => setTargetValue((currentPrice * 1.05).toFixed(2))}
            className="px-2 py-0.5 text-[10px] bg-muted rounded hover:bg-muted/80"
          >
            +5%
          </button>
          <button
            type="button"
            onClick={() => setTargetValue((currentPrice * 1.1).toFixed(2))}
            className="px-2 py-0.5 text-[10px] bg-muted rounded hover:bg-muted/80"
          >
            +10%
          </button>
          <button
            type="button"
            onClick={() => setTargetValue((currentPrice * 0.95).toFixed(2))}
            className="px-2 py-0.5 text-[10px] bg-muted rounded hover:bg-muted/80"
          >
            -5%
          </button>
          <button
            type="button"
            onClick={() => setTargetValue((currentPrice * 0.9).toFixed(2))}
            className="px-2 py-0.5 text-[10px] bg-muted rounded hover:bg-muted/80"
          >
            -10%
          </button>
        </div>
      )}

      {/* Note */}
      <input
        type="text"
        value={note}
        onChange={e => setNote(e.target.value)}
        placeholder="Notiz (optional)"
        className="w-full px-2 py-1.5 text-xs bg-muted border-none rounded focus:outline-none focus:ring-2 focus:ring-primary"
      />

      {/* Actions */}
      <div className="flex gap-2">
        <button
          type="submit"
          disabled={isLoading || !targetValue}
          className="flex-1 px-2 py-1.5 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors disabled:opacity-50"
        >
          {isLoading ? 'Erstelle...' : 'Alert erstellen'}
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="px-2 py-1.5 text-xs bg-muted rounded hover:bg-muted/80 transition-colors"
        >
          Abbrechen
        </button>
      </div>
    </form>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function AlertsPanel({ securityId, currentPrice, currency }: AlertsPanelProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [alerts, setAlerts] = useState<PriceAlert[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  // Load alerts for security
  const loadAlerts = useCallback(async () => {
    if (!securityId) {
      setAlerts([]);
      return;
    }

    setIsLoading(true);
    try {
      const data = await getPriceAlerts(securityId);
      setAlerts(data);
    } catch (err) {
      console.error('Failed to load alerts:', err);
    } finally {
      setIsLoading(false);
    }
  }, [securityId]);

  useEffect(() => {
    loadAlerts();
  }, [loadAlerts]);

  const handleToggle = async (alertId: number) => {
    try {
      const updated = await togglePriceAlert(alertId);
      setAlerts(prev => prev.map(a => (a.id === alertId ? updated : a)));
    } catch (err) {
      console.error('Failed to toggle alert:', err);
    }
  };

  const handleDelete = async (alertId: number) => {
    try {
      await deletePriceAlert(alertId);
      setAlerts(prev => prev.filter(a => a.id !== alertId));
    } catch (err) {
      console.error('Failed to delete alert:', err);
    }
  };

  const handleReset = async (alertId: number) => {
    try {
      await resetAlertTrigger(alertId);
      setAlerts(prev =>
        prev.map(a => (a.id === alertId ? { ...a, isTriggered: false } : a))
      );
    } catch (err) {
      console.error('Failed to reset alert:', err);
    }
  };

  const handleCreated = (alert: PriceAlert) => {
    setAlerts(prev => [alert, ...prev]);
    setIsCreating(false);
  };

  const activeCount = alerts.filter(a => a.isActive).length;
  const triggeredCount = alerts.filter(a => a.isTriggered).length;

  return (
    <div className="bg-card border border-border rounded-lg overflow-hidden">
      {/* Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-muted/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          {triggeredCount > 0 ? (
            <BellRing size={16} className="text-amber-500 animate-pulse" />
          ) : (
            <Bell size={16} className="text-muted-foreground" />
          )}
          <span className="font-medium">Price Alerts</span>
          <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
            {activeCount} aktiv
          </span>
          {triggeredCount > 0 && (
            <span className="text-xs text-amber-600 bg-amber-500/20 px-1.5 py-0.5 rounded">
              {triggeredCount} ausgelöst
            </span>
          )}
        </div>
        {isExpanded ? (
          <ChevronUp size={16} className="text-muted-foreground" />
        ) : (
          <ChevronDown size={16} className="text-muted-foreground" />
        )}
      </button>

      {isExpanded && (
        <div className="border-t border-border">
          {/* Alerts List */}
          {alerts.length > 0 && (
            <div className="p-2 space-y-2 max-h-60 overflow-y-auto">
              {alerts.map(alert => (
                <AlertItem
                  key={alert.id}
                  alert={alert}
                  currency={currency}
                  onToggle={() => handleToggle(alert.id)}
                  onDelete={() => handleDelete(alert.id)}
                  onReset={() => handleReset(alert.id)}
                />
              ))}
            </div>
          )}

          {/* Empty State */}
          {alerts.length === 0 && !isCreating && !isLoading && (
            <div className="p-4 text-center text-muted-foreground">
              <Bell size={24} className="mx-auto mb-2 opacity-30" />
              <p className="text-xs">Keine Alerts für dieses Wertpapier</p>
            </div>
          )}

          {/* Create Form */}
          {isCreating && securityId ? (
            <CreateAlertForm
              securityId={securityId}
              currentPrice={currentPrice}
              currency={currency}
              onCreated={handleCreated}
              onCancel={() => setIsCreating(false)}
            />
          ) : (
            <div className="p-2 border-t border-border">
              <button
                onClick={() => setIsCreating(true)}
                disabled={!securityId}
                className="w-full flex items-center justify-center gap-1.5 px-2 py-1.5 text-xs bg-muted hover:bg-muted/80 rounded transition-colors disabled:opacity-50"
              >
                <Plus size={12} />
                Alert hinzufügen
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default AlertsPanel;
