/**
 * Benchmark Widget - Shows portfolio vs benchmark comparison
 */

import { useEffect, useState, useMemo } from 'react';
import { RefreshCw, Target, TrendingUp, TrendingDown } from 'lucide-react';
import { getBenchmarks, compareToBenchmark } from '../../../lib/api';
import type { BenchmarkData, BenchmarkComparison } from '../../../lib/types';
import type { WidgetProps } from '../types';

interface BenchmarkWidgetProps extends WidgetProps {
  currency?: string;
}

export function BenchmarkWidget({ config }: BenchmarkWidgetProps) {
  const portfolioId = config.settings?.portfolioId as number | undefined;
  const benchmarkId = config.settings?.benchmarkId as number | undefined;

  const [benchmarks, setBenchmarks] = useState<BenchmarkData[]>([]);
  const [selectedBenchmark, setSelectedBenchmark] = useState<number | null>(benchmarkId ?? null);
  const [comparison, setComparison] = useState<BenchmarkComparison | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Calculate date range (default: YTD)
  const { startDate, endDate } = useMemo(() => {
    const now = new Date();
    const year = now.getFullYear();
    return {
      startDate: `${year}-01-01`,
      endDate: now.toISOString().split('T')[0],
    };
  }, []);

  const loadBenchmarks = async () => {
    try {
      const data = await getBenchmarks();
      setBenchmarks(data);

      // Select first benchmark if none specified
      if (!selectedBenchmark && data.length > 0) {
        setSelectedBenchmark(data[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    }
  };

  const loadComparison = async () => {
    if (!selectedBenchmark) {
      setComparison(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const data = await compareToBenchmark(
        portfolioId ?? null,
        selectedBenchmark,
        startDate,
        endDate
      );
      setComparison(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadBenchmarks();
  }, []);

  useEffect(() => {
    loadComparison();
  }, [selectedBenchmark, portfolioId, startDate, endDate]);

  const formatPercent = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'percent',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      signDisplay: 'exceptZero',
    }).format(value / 100);
  };

  const formatNumber = (value: number, decimals = 2) => {
    return new Intl.NumberFormat('de-DE', {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }).format(value);
  };

  if (loading && !comparison) {
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
          Benchmark-Vergleich
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center text-sm text-muted-foreground">
            <p>{error}</p>
            <button
              onClick={loadComparison}
              className="mt-2 text-primary hover:underline"
            >
              Erneut versuchen
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (benchmarks.length === 0) {
    return (
      <div className="h-full flex flex-col p-4">
        <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
          Benchmark-Vergleich
        </div>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <Target className="h-8 w-8 text-muted-foreground/50 mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">
              Kein Benchmark definiert
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Erstellen Sie einen Benchmark unter Benchmark
            </p>
          </div>
        </div>
      </div>
    );
  }

  const currentBenchmark = benchmarks.find((b) => b.id === selectedBenchmark);
  const portfolioReturn = comparison?.portfolioReturn ?? 0;
  const benchmarkReturn = comparison?.benchmarkReturn ?? 0;
  const alpha = comparison?.alpha ?? 0;
  const isOutperforming = alpha >= 0;

  return (
    <div className="h-full flex flex-col p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="text-xs text-muted-foreground uppercase tracking-wide">
          Benchmark YTD
        </div>
        {benchmarks.length > 1 && (
          <select
            value={selectedBenchmark ?? ''}
            onChange={(e) => setSelectedBenchmark(Number(e.target.value))}
            className="text-xs bg-transparent border-none focus:outline-none cursor-pointer text-muted-foreground"
          >
            {benchmarks.map((bm) => (
              <option key={bm.id} value={bm.id}>
                {bm.securityName}
              </option>
            ))}
          </select>
        )}
      </div>

      {currentBenchmark && benchmarks.length === 1 && (
        <div className="text-xs text-muted-foreground mb-2">
          vs. {currentBenchmark.securityName}
        </div>
      )}

      {comparison && (
        <div className="flex-1 flex flex-col justify-center">
          {/* Alpha (main metric) */}
          <div className="text-center mb-4">
            <div
              className={`text-2xl font-bold ${
                isOutperforming ? 'text-green-600' : 'text-red-600'
              }`}
            >
              {isOutperforming ? '+' : ''}{formatPercent(alpha)}
            </div>
            <div className="text-xs text-muted-foreground flex items-center justify-center gap-1">
              {isOutperforming ? (
                <TrendingUp className="h-3 w-3 text-green-600" />
              ) : (
                <TrendingDown className="h-3 w-3 text-red-600" />
              )}
              {isOutperforming ? 'Outperformance' : 'Underperformance'}
            </div>
          </div>

          {/* Comparison Details */}
          <div className="grid grid-cols-2 gap-3 text-center">
            <div>
              <div
                className={`text-sm font-medium ${
                  portfolioReturn >= 0 ? 'text-green-600' : 'text-red-600'
                }`}
              >
                {formatPercent(portfolioReturn)}
              </div>
              <div className="text-[10px] text-muted-foreground">Portfolio</div>
            </div>
            <div>
              <div
                className={`text-sm font-medium ${
                  benchmarkReturn >= 0 ? 'text-green-600' : 'text-red-600'
                }`}
              >
                {formatPercent(benchmarkReturn)}
              </div>
              <div className="text-[10px] text-muted-foreground">
                {currentBenchmark?.securityName || 'Benchmark'}
              </div>
            </div>
          </div>

          {/* Additional Metrics */}
          {comparison.sharpeRatio !== undefined && (
            <div className="mt-4 pt-3 border-t grid grid-cols-3 gap-2 text-center text-xs">
              <div>
                <div className="font-medium">{formatNumber(comparison.sharpeRatio)}</div>
                <div className="text-[10px] text-muted-foreground">Sharpe</div>
              </div>
              <div>
                <div className="font-medium">{formatNumber(comparison.beta)}</div>
                <div className="text-[10px] text-muted-foreground">Beta</div>
              </div>
              <div>
                <div className="font-medium">{formatPercent(comparison.correlation * 100)}</div>
                <div className="text-[10px] text-muted-foreground">Korrelation</div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
