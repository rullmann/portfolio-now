/**
 * Benchmark view for performance comparison.
 */

import { useState, useEffect } from 'react';
import { Target, Plus, Trash2, RefreshCw } from 'lucide-react';
import { getBenchmarks, compareToBenchmark, getBenchmarkComparisonData, removeBenchmark } from '../../lib/api';
import { TradingViewBenchmarkChart } from '../../components/charts';
import { formatDate, type BenchmarkData, type BenchmarkComparison, type BenchmarkDataPoint } from '../../lib/types';

export function BenchmarkView() {
  const [benchmarks, setBenchmarks] = useState<BenchmarkData[]>([]);
  const [selectedBenchmark, setSelectedBenchmark] = useState<number | null>(null);
  const [comparison, setComparison] = useState<BenchmarkComparison | null>(null);
  const [chartData, setChartData] = useState<BenchmarkDataPoint[]>([]);
  const [startDate, setStartDate] = useState<string>(() => {
    const d = new Date();
    d.setFullYear(d.getFullYear() - 1);
    return d.toISOString().split('T')[0];
  });
  const [endDate, setEndDate] = useState<string>(() => new Date().toISOString().split('T')[0]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadBenchmarks = async () => {
    try {
      setIsLoading(true);
      const data = await getBenchmarks();
      setBenchmarks(data);
      if (data.length > 0 && !selectedBenchmark) {
        setSelectedBenchmark(data[0].securityId);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const loadComparison = async () => {
    if (!selectedBenchmark) return;
    try {
      setIsLoading(true);
      const [comparisonData, chartPoints] = await Promise.all([
        compareToBenchmark(null, selectedBenchmark, startDate, endDate),
        getBenchmarkComparisonData(null, selectedBenchmark, startDate, endDate),
      ]);
      setComparison(comparisonData);
      setChartData(chartPoints);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadBenchmarks();
  }, []);

  useEffect(() => {
    if (selectedBenchmark) {
      loadComparison();
    }
  }, [selectedBenchmark, startDate, endDate]);

  const handleRemoveBenchmark = async (benchmarkId: number, securityId: number) => {
    try {
      await removeBenchmark(benchmarkId);
      if (selectedBenchmark === securityId) {
        setSelectedBenchmark(null);
        setComparison(null);
        setChartData([]);
      }
      await loadBenchmarks();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const formatPercent = (value: number | null | undefined) => {
    if (value === null || value === undefined) return '-';
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  };

  const formatNumber = (value: number | null | undefined, decimals: number = 2) => {
    if (value === null || value === undefined) return '-';
    return value.toFixed(decimals);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Target className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Benchmark-Vergleich</h1>
        </div>
        <div className="flex gap-2">
          <button
            onClick={loadBenchmarks}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
          <button className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors">
            <Plus size={16} />
            Benchmark hinzufügen
          </button>
        </div>
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          {error}
        </div>
      )}

      {/* Date Range & Benchmark Selection */}
      <div className="bg-card rounded-lg border border-border p-4">
        <div className="flex flex-wrap items-end gap-4">
          <div>
            <label className="block text-sm font-medium mb-1">Benchmark</label>
            <select
              value={selectedBenchmark || ''}
              onChange={(e) => setSelectedBenchmark(Number(e.target.value) || null)}
              className="px-3 py-2 border border-border rounded-md bg-background min-w-[200px]"
            >
              <option value="">Benchmark wählen...</option>
              {benchmarks.map(b => (
                <option key={b.id} value={b.securityId}>{b.securityName}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Von</label>
            <input
              type="date"
              value={startDate}
              onChange={(e) => setStartDate(e.target.value)}
              className="px-3 py-2 border border-border rounded-md bg-background"
            />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Bis</label>
            <input
              type="date"
              value={endDate}
              onChange={(e) => setEndDate(e.target.value)}
              className="px-3 py-2 border border-border rounded-md bg-background"
            />
          </div>
          <button
            onClick={loadComparison}
            disabled={!selectedBenchmark || isLoading}
            className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            Vergleichen
          </button>
        </div>
      </div>

      {comparison && (
        <>
          {/* Performance Cards */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <div className="bg-card rounded-lg border border-border p-4">
              <div className="text-sm text-muted-foreground">Portfolio-Rendite</div>
              <div className={`text-2xl font-bold ${comparison.portfolioReturn >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {formatPercent(comparison.portfolioReturn)}
              </div>
            </div>
            <div className="bg-card rounded-lg border border-border p-4">
              <div className="text-sm text-muted-foreground">Benchmark-Rendite</div>
              <div className={`text-2xl font-bold ${comparison.benchmarkReturn >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {formatPercent(comparison.benchmarkReturn)}
              </div>
            </div>
            <div className="bg-card rounded-lg border border-border p-4">
              <div className="text-sm text-muted-foreground">Alpha</div>
              <div className={`text-2xl font-bold ${comparison.alpha >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {formatPercent(comparison.alpha)}
              </div>
            </div>
            <div className="bg-card rounded-lg border border-border p-4">
              <div className="text-sm text-muted-foreground">Sharpe Ratio</div>
              <div className="text-2xl font-bold">
                {formatNumber(comparison.sharpeRatio)}
              </div>
            </div>
          </div>

          {/* Chart */}
          {chartData.length > 0 && (
            <div className="bg-card rounded-lg border border-border p-4">
              <h2 className="font-semibold mb-4">Performance-Vergleich (normalisiert)</h2>
              <TradingViewBenchmarkChart
                data={chartData}
                height={320}
                portfolioName="Portfolio"
                benchmarkName={benchmarks.find(b => b.securityId === selectedBenchmark)?.securityName || 'Benchmark'}
              />
            </div>
          )}

          {/* Risk Metrics Table */}
          <div className="bg-card rounded-lg border border-border p-4">
            <h2 className="font-semibold mb-4">Risiko-Kennzahlen</h2>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left py-2 font-medium">Kennzahl</th>
                    <th className="text-right py-2 font-medium">Wert</th>
                    <th className="text-left py-2 pl-4 font-medium">Beschreibung</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b border-border">
                    <td className="py-2 font-medium">Beta</td>
                    <td className="py-2 text-right">{formatNumber(comparison.beta)}</td>
                    <td className="py-2 pl-4 text-muted-foreground">Systematisches Risiko relativ zum Benchmark</td>
                  </tr>
                  <tr className="border-b border-border">
                    <td className="py-2 font-medium">Korrelation</td>
                    <td className="py-2 text-right">{formatNumber(comparison.correlation)}</td>
                    <td className="py-2 pl-4 text-muted-foreground">Stärke des linearen Zusammenhangs</td>
                  </tr>
                  <tr>
                    <td className="py-2 font-medium">Tracking Error</td>
                    <td className="py-2 text-right">{formatPercent(comparison.trackingError)}</td>
                    <td className="py-2 pl-4 text-muted-foreground">Standardabweichung der Überrendite</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </>
      )}

      {/* Benchmarks List */}
      <div className="bg-card rounded-lg border border-border p-4">
        <h2 className="font-semibold mb-4">Verfügbare Benchmarks ({benchmarks.length})</h2>
        {benchmarks.length > 0 ? (
          <div className="space-y-2">
            {benchmarks.map(b => (
              <div
                key={b.id}
                className={`flex items-center justify-between p-3 rounded-md ${
                  selectedBenchmark === b.securityId ? 'bg-primary/10 border border-primary/20' : 'bg-muted/50'
                }`}
              >
                <div>
                  <div className="font-medium">{b.securityName}</div>
                  <div className="text-xs text-muted-foreground">
                    {b.isin || 'Keine ISIN'}
                    <span className="mx-2">·</span>
                    Start: {formatDate(b.startDate)}
                  </div>
                </div>
                <button
                  onClick={() => handleRemoveBenchmark(b.id, b.securityId)}
                  className="p-1.5 hover:bg-destructive/10 rounded-md text-destructive"
                  title="Entfernen"
                >
                  <Trash2 size={16} />
                </button>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center text-muted-foreground py-4">
            <Target className="w-12 h-12 mx-auto mb-3 opacity-50" />
            <p>Keine Benchmarks definiert.</p>
            <p className="text-sm mt-1">Fügen Sie einen Benchmark hinzu, um Ihre Performance zu vergleichen.</p>
          </div>
        )}
      </div>
    </div>
  );
}
