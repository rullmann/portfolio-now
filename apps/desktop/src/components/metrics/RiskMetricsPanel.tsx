/**
 * Risk Metrics Panel Component
 *
 * Displays portfolio risk metrics:
 * - Sharpe Ratio
 * - Sortino Ratio
 * - Maximum Drawdown
 * - Volatility
 * - Beta / Alpha (vs benchmark)
 * - Calmar Ratio
 */

import { useState, useEffect } from 'react';
import {
  Activity,
  TrendingDown,
  BarChart3,
  RefreshCw,
  Info,
  AlertTriangle,
  Target,
} from 'lucide-react';
import { calculateRiskMetrics, type RiskMetrics } from '../../lib/api';
import { formatDate } from '../../lib/types';

interface Props {
  portfolioId?: number;
  benchmarkId?: number;
  startDate?: string;
  endDate?: string;
  compact?: boolean;
}

export function RiskMetricsPanel({
  portfolioId,
  benchmarkId,
  startDate,
  endDate,
  compact = false,
}: Props) {
  const [metrics, setMetrics] = useState<RiskMetrics | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadMetrics();
  }, [portfolioId, benchmarkId, startDate, endDate]);

  const loadMetrics = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await calculateRiskMetrics({
        portfolioId,
        benchmarkId,
        startDate,
        endDate,
      });
      setMetrics(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const formatRatio = (value: number, decimals: number = 2) => {
    return value.toFixed(decimals);
  };

  const formatPercent = (value: number) => {
    return `${(value * 100).toFixed(2)}%`;
  };

  const getRatioColor = (value: number, thresholds: { good: number; bad: number }) => {
    if (value >= thresholds.good) return 'text-green-600';
    if (value <= thresholds.bad) return 'text-red-600';
    return 'text-yellow-600';
  };

  const getDrawdownColor = (value: number) => {
    if (value < 0.1) return 'text-green-600';
    if (value < 0.2) return 'text-yellow-600';
    return 'text-red-600';
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-32 text-muted-foreground">
        <RefreshCw className="w-5 h-5 animate-spin mr-2" />
        Berechne Risikometriken...
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 p-4 bg-destructive/10 border border-destructive/20 rounded-md text-destructive">
        <AlertTriangle size={20} />
        {error}
      </div>
    );
  }

  if (!metrics) {
    return null;
  }

  // Compact mode for dashboard
  if (compact) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <MetricCard
          label="Sharpe Ratio"
          value={formatRatio(metrics.sharpeRatio)}
          color={getRatioColor(metrics.sharpeRatio, { good: 1, bad: 0 })}
          icon={<Activity size={16} />}
          tooltip="Rendite/Risiko-Verhältnis. >1 ist gut, >2 ist ausgezeichnet."
        />
        <MetricCard
          label="Volatilität"
          value={formatPercent(metrics.volatility)}
          color={getRatioColor(-metrics.volatility, { good: -0.15, bad: -0.3 })}
          icon={<BarChart3 size={16} />}
          tooltip="Annualisierte Standardabweichung der Renditen."
        />
        <MetricCard
          label="Max Drawdown"
          value={formatPercent(metrics.maxDrawdown)}
          color={getDrawdownColor(metrics.maxDrawdown)}
          icon={<TrendingDown size={16} />}
          tooltip="Größter Wertverlust vom Höchststand."
        />
        <MetricCard
          label="Sortino Ratio"
          value={formatRatio(metrics.sortinoRatio)}
          color={getRatioColor(metrics.sortinoRatio, { good: 1.5, bad: 0.5 })}
          icon={<Target size={16} />}
          tooltip="Wie Sharpe, aber nur Abwärtsrisiko. Höher ist besser."
        />
      </div>
    );
  }

  // Full panel
  return (
    <div className="bg-card rounded-lg border border-border">
      <div className="p-4 border-b border-border flex items-center justify-between">
        <h3 className="font-semibold flex items-center gap-2">
          <Activity size={18} />
          Risikometriken
        </h3>
        <button
          onClick={loadMetrics}
          className="p-1.5 hover:bg-muted rounded-md transition-colors"
          title="Aktualisieren"
        >
          <RefreshCw size={16} />
        </button>
      </div>

      <div className="p-4 space-y-6">
        {/* Main Ratios */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <MetricCard
            label="Sharpe Ratio"
            value={formatRatio(metrics.sharpeRatio)}
            color={getRatioColor(metrics.sharpeRatio, { good: 1, bad: 0 })}
            icon={<Activity size={16} />}
            tooltip="Rendite/Risiko-Verhältnis. Misst wie viel Rendite pro Risikoeinheit erzielt wird. >1 ist gut, >2 ist ausgezeichnet."
          />
          <MetricCard
            label="Sortino Ratio"
            value={formatRatio(metrics.sortinoRatio)}
            color={getRatioColor(metrics.sortinoRatio, { good: 1.5, bad: 0.5 })}
            icon={<Target size={16} />}
            tooltip="Wie Sharpe, aber berücksichtigt nur Abwärtsvolatilität. Höhere Werte sind besser."
          />
          <MetricCard
            label="Volatilität"
            value={formatPercent(metrics.volatility)}
            color={getRatioColor(-metrics.volatility, { good: -0.15, bad: -0.3 })}
            icon={<BarChart3 size={16} />}
            tooltip="Annualisierte Standardabweichung der täglichen Renditen. Niedrigere Werte = weniger Schwankung."
          />
          {metrics.calmarRatio !== null && (
            <MetricCard
              label="Calmar Ratio"
              value={formatRatio(metrics.calmarRatio)}
              color={getRatioColor(metrics.calmarRatio, { good: 1, bad: 0 })}
              icon={<TrendingDown size={16} />}
              tooltip="Annualisierte Rendite / Max Drawdown. Höher ist besser."
            />
          )}
        </div>

        {/* Drawdown Section */}
        <div className="bg-muted/30 rounded-lg p-4">
          <div className="flex items-center gap-2 mb-3">
            <TrendingDown size={18} className="text-red-500" />
            <span className="font-medium">Maximum Drawdown</span>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <div className="text-sm text-muted-foreground">Größter Rückgang</div>
              <div className={`text-2xl font-bold ${getDrawdownColor(metrics.maxDrawdown)}`}>
                -{formatPercent(metrics.maxDrawdown)}
              </div>
            </div>
            {metrics.maxDrawdownStart && (
              <div>
                <div className="text-sm text-muted-foreground">Beginn</div>
                <div className="font-medium">{formatDate(metrics.maxDrawdownStart)}</div>
              </div>
            )}
            {metrics.maxDrawdownEnd && (
              <div>
                <div className="text-sm text-muted-foreground">Tiefpunkt</div>
                <div className="font-medium">{formatDate(metrics.maxDrawdownEnd)}</div>
              </div>
            )}
          </div>
          <DrawdownBar value={metrics.maxDrawdown} />
        </div>

        {/* Beta/Alpha Section (if benchmark provided) */}
        {(metrics.beta !== null || metrics.alpha !== null) && (
          <div className="bg-muted/30 rounded-lg p-4">
            <div className="flex items-center gap-2 mb-3">
              <Target size={18} className="text-blue-500" />
              <span className="font-medium">Benchmark-Vergleich</span>
            </div>
            <div className="grid grid-cols-2 gap-4">
              {metrics.beta !== null && (
                <div>
                  <div className="text-sm text-muted-foreground flex items-center gap-1">
                    Beta
                    <Tooltip text="Maß für Marktrisiko. Beta=1 bedeutet gleiche Schwankung wie der Markt. >1 = volatiler, <1 = defensiver." />
                  </div>
                  <div className={`text-2xl font-bold ${
                    metrics.beta < 0.8 ? 'text-green-600' :
                    metrics.beta > 1.2 ? 'text-red-600' : ''
                  }`}>
                    {formatRatio(metrics.beta)}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {metrics.beta < 0.8 ? 'Defensiv' :
                     metrics.beta > 1.2 ? 'Aggressiv' : 'Marktkonform'}
                  </div>
                </div>
              )}
              {metrics.alpha !== null && (
                <div>
                  <div className="text-sm text-muted-foreground flex items-center gap-1">
                    Alpha (Jensen)
                    <Tooltip text="Überrendite gegenüber Benchmark nach Risikobereinigung. Positiv = Outperformance." />
                  </div>
                  <div className={`text-2xl font-bold ${
                    metrics.alpha > 0 ? 'text-green-600' : 'text-red-600'
                  }`}>
                    {metrics.alpha > 0 ? '+' : ''}{formatPercent(metrics.alpha)}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {metrics.alpha > 0 ? 'Outperformance' : 'Underperformance'}
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Data Info */}
        <div className="text-xs text-muted-foreground flex items-center gap-1">
          <Info size={12} />
          Basierend auf {metrics.dataPoints} Datenpunkten
        </div>
      </div>
    </div>
  );
}

// Helper Components

function MetricCard({
  label,
  value,
  color,
  icon,
  tooltip,
}: {
  label: string;
  value: string;
  color?: string;
  icon: React.ReactNode;
  tooltip?: string;
}) {
  return (
    <div className="bg-muted/30 rounded-lg p-3">
      <div className="text-sm text-muted-foreground flex items-center gap-1 mb-1">
        {icon}
        {label}
        {tooltip && <Tooltip text={tooltip} />}
      </div>
      <div className={`text-xl font-bold ${color || ''}`}>{value}</div>
    </div>
  );
}

function Tooltip({ text }: { text: string }) {
  return (
    <span className="group relative">
      <Info size={12} className="text-muted-foreground cursor-help" />
      <span className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-2 py-1 bg-popover text-popover-foreground text-xs rounded shadow-lg opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap max-w-xs z-50 pointer-events-none">
        {text}
      </span>
    </span>
  );
}

function DrawdownBar({ value }: { value: number }) {
  const percent = Math.min(value * 100, 100);

  return (
    <div className="mt-3">
      <div className="flex justify-between text-xs text-muted-foreground mb-1">
        <span>0%</span>
        <span>-50%</span>
      </div>
      <div className="w-full bg-muted rounded-full h-2">
        <div
          className={`h-2 rounded-full transition-all ${
            percent < 10 ? 'bg-green-500' :
            percent < 20 ? 'bg-yellow-500' : 'bg-red-500'
          }`}
          style={{ width: `${percent * 2}%` }}
        />
      </div>
    </div>
  );
}
