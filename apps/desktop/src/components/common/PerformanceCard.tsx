/**
 * Performance metrics display card.
 * Shows TTWROR, IRR, and other key performance indicators.
 */

import { useState, useEffect } from 'react';
import { TrendingUp, TrendingDown, RefreshCw, AlertCircle, Calendar, Percent } from 'lucide-react';
import { calculatePerformance, getBaseCurrency } from '../../lib/api';
import type { PerformanceResult } from '../../lib/types';
import { formatCurrency, formatNumber } from '../../lib/types';

interface PerformanceCardProps {
  portfolioId?: number;
  className?: string;
}

export function PerformanceCard({ portfolioId, className = '' }: PerformanceCardProps) {
  const [performance, setPerformance] = useState<PerformanceResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [baseCurrency, setBaseCurrency] = useState<string>('EUR');

  const loadPerformance = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [result, currency] = await Promise.all([
        calculatePerformance({ portfolioId }),
        getBaseCurrency().catch(() => 'EUR'),
      ]);
      setPerformance(result);
      setBaseCurrency(currency);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadPerformance();
  }, [portfolioId]);

  if (isLoading && !performance) {
    return (
      <div className={`bg-card rounded-lg border border-border p-6 ${className}`}>
        <div className="flex items-center gap-2 text-muted-foreground">
          <RefreshCw size={16} className="animate-spin" />
          Lade Performance...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={`bg-card rounded-lg border border-border p-6 ${className}`}>
        <div className="flex items-center gap-2 text-destructive">
          <AlertCircle size={16} />
          {error}
        </div>
      </div>
    );
  }

  if (!performance) {
    return (
      <div className={`bg-card rounded-lg border border-border p-6 ${className}`}>
        <p className="text-muted-foreground">Keine Performance-Daten verfügbar.</p>
      </div>
    );
  }

  const isPositive = performance.ttwror >= 0;
  const irrIsPositive = performance.irr >= 0;

  return (
    <div className={`bg-card rounded-lg border border-border p-6 ${className}`}>
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold">Performance</h3>
        <button
          onClick={loadPerformance}
          disabled={isLoading}
          className="p-1.5 hover:bg-muted rounded-md transition-colors"
          title="Aktualisieren"
        >
          <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
        </button>
      </div>

      {/* Main metrics */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
        {/* TTWROR */}
        <div className="space-y-1">
          <p className="text-sm text-muted-foreground">TTWROR</p>
          <div className="flex items-center gap-2">
            {isPositive ? (
              <TrendingUp size={20} className="text-green-600" />
            ) : (
              <TrendingDown size={20} className="text-red-600" />
            )}
            <span className={`text-2xl font-bold tabular-nums ${isPositive ? 'text-green-600' : 'text-red-600'}`}>
              {performance.ttwror >= 0 ? '+' : ''}{formatNumber(performance.ttwror, 2)}%
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            {formatNumber(performance.ttwrorAnnualized, 2)}% p.a.
          </p>
        </div>

        {/* IRR */}
        <div className="space-y-1">
          <p className="text-sm text-muted-foreground">
            IRR
            {!performance.irrConverged && (
              <span className="ml-1 text-yellow-600" title="Berechnung nicht konvergiert">~</span>
            )}
          </p>
          <div className="flex items-center gap-2">
            <Percent size={20} className={irrIsPositive ? 'text-green-600' : 'text-red-600'} />
            <span className={`text-2xl font-bold tabular-nums ${irrIsPositive ? 'text-green-600' : 'text-red-600'}`}>
              {performance.irr >= 0 ? '+' : ''}{formatNumber(performance.irr, 2)}%
            </span>
          </div>
          <p className="text-xs text-muted-foreground">Money-Weighted</p>
        </div>

        {/* Current Value */}
        <div className="space-y-1">
          <p className="text-sm text-muted-foreground">Depotwert</p>
          <p className="text-2xl font-bold tabular-nums">
            {formatCurrency(performance.currentValue, baseCurrency)}
          </p>
          <p className="text-xs text-muted-foreground">
            Investiert: {formatCurrency(performance.totalInvested, baseCurrency)}
          </p>
        </div>

        {/* Absolute Gain */}
        <div className="space-y-1">
          <p className="text-sm text-muted-foreground">Gewinn/Verlust</p>
          <p className={`text-2xl font-bold tabular-nums ${performance.absoluteGain >= 0 ? 'text-green-600' : 'text-red-600'}`}>
            {performance.absoluteGain >= 0 ? '+' : ''}{formatCurrency(performance.absoluteGain, baseCurrency)}
          </p>
          <p className="text-xs text-muted-foreground flex items-center gap-1">
            <Calendar size={12} />
            {performance.days} Tage
          </p>
        </div>
      </div>

      {/* Period info */}
      <div className="pt-4 border-t border-border">
        <p className="text-sm text-muted-foreground">
          Zeitraum: {performance.startDate} bis {performance.endDate}
        </p>
      </div>
    </div>
  );
}
