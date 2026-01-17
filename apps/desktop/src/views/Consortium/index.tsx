/**
 * Consortium View - Portfolio Groups
 *
 * Allows combining multiple portfolios into a "virtual portfolio" for
 * consolidated performance analysis and comparison.
 */

import { useState, useEffect, useMemo } from 'react';
import {
  FolderKanban,
  Plus,
  Trash2,
  Edit2,
  TrendingUp,
  RefreshCw,
  PieChart,
  BarChart3,
  X,
  Check,
} from 'lucide-react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import {
  getConsortiums,
  createConsortium,
  updateConsortium,
  deleteConsortium,
  getConsortiumPerformance,
  getConsortiumHistory,
  getPortfolios,
} from '../../lib/api';
import type {
  Consortium,
  ConsortiumPerformance,
  ConsortiumHistory,
  PortfolioData,
  PortfolioPerformanceSummary,
} from '../../lib/types';
import { toast, useSettingsStore } from '../../store';
import { formatCurrency } from '../utils';

export function ConsortiumView() {
  const [consortiums, setConsortiums] = useState<Consortium[]>([]);
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [selectedConsortiumId, setSelectedConsortiumId] = useState<number | null>(null);
  const [performance, setPerformance] = useState<ConsortiumPerformance | null>(null);
  const [history, setHistory] = useState<ConsortiumHistory | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingPerformance, setIsLoadingPerformance] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editingConsortium, setEditingConsortium] = useState<Consortium | null>(null);
  const [formName, setFormName] = useState('');
  const [formPortfolioIds, setFormPortfolioIds] = useState<number[]>([]);
  const [error, setError] = useState<string | null>(null);

  const { baseCurrency: _baseCurrency } = useSettingsStore();
  void _baseCurrency; // Reserved for future use

  // Load consortiums and portfolios
  useEffect(() => {
    loadData();
  }, []);

  // Load performance when consortium selected
  useEffect(() => {
    if (selectedConsortiumId) {
      loadPerformance(selectedConsortiumId);
    } else {
      setPerformance(null);
      setHistory(null);
    }
  }, [selectedConsortiumId]);

  const loadData = async () => {
    setIsLoading(true);
    try {
      const [consortiumsData, portfoliosData] = await Promise.all([
        getConsortiums(),
        getPortfolios(),
      ]);
      setConsortiums(consortiumsData);
      setPortfolios(portfoliosData);

      // Auto-select first consortium if available
      if (consortiumsData.length > 0 && !selectedConsortiumId) {
        setSelectedConsortiumId(consortiumsData[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load data');
      toast.error('Fehler beim Laden der Daten');
    } finally {
      setIsLoading(false);
    }
  };

  const loadPerformance = async (consortiumId: number) => {
    setIsLoadingPerformance(true);
    try {
      const [perf, hist] = await Promise.all([
        getConsortiumPerformance(consortiumId),
        getConsortiumHistory(consortiumId),
      ]);
      setPerformance(perf);
      setHistory(hist);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load performance');
      toast.error('Fehler beim Laden der Performance-Daten');
    } finally {
      setIsLoadingPerformance(false);
    }
  };

  const handleCreateOrUpdate = async () => {
    if (!formName.trim() || formPortfolioIds.length === 0) {
      toast.error('Bitte Name und mindestens ein Portfolio angeben');
      return;
    }

    try {
      if (editingConsortium) {
        await updateConsortium(editingConsortium.id, {
          name: formName,
          portfolioIds: formPortfolioIds,
        });
        toast.success('Konsortium aktualisiert');
      } else {
        const created = await createConsortium({
          name: formName,
          portfolioIds: formPortfolioIds,
        });
        setSelectedConsortiumId(created.id);
        toast.success('Konsortium erstellt');
      }
      await loadData();
      closeForm();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Fehler beim Speichern');
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('Konsortium wirklich löschen?')) return;

    try {
      await deleteConsortium(id);
      if (selectedConsortiumId === id) {
        setSelectedConsortiumId(null);
      }
      await loadData();
      toast.success('Konsortium gelöscht');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Fehler beim Löschen');
    }
  };

  const openCreateForm = () => {
    setEditingConsortium(null);
    setFormName('');
    setFormPortfolioIds([]);
    setShowForm(true);
  };

  const openEditForm = (consortium: Consortium) => {
    setEditingConsortium(consortium);
    setFormName(consortium.name);
    setFormPortfolioIds(consortium.portfolioIds);
    setShowForm(true);
  };

  const closeForm = () => {
    setShowForm(false);
    setEditingConsortium(null);
    setFormName('');
    setFormPortfolioIds([]);
  };

  const togglePortfolio = (portfolioId: number) => {
    setFormPortfolioIds((prev) =>
      prev.includes(portfolioId)
        ? prev.filter((id) => id !== portfolioId)
        : [...prev, portfolioId]
    );
  };

  // Prepare chart data
  const chartData = useMemo(() => {
    if (!history) return [];

    return history.combined.map((point, i) => {
      const entry: Record<string, string | number> = {
        date: point.date,
        Gesamt: point.cumulativeReturn,
      };

      history.byPortfolio.forEach((portfolio) => {
        const portfolioPoint = portfolio.data[i];
        if (portfolioPoint) {
          entry[portfolio.portfolioName] = portfolioPoint.cumulativeReturn;
        }
      });

      return entry;
    });
  }, [history]);

  const formatPercent = (value: number) => `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  const formatPercentShort = (value: number) => `${value.toFixed(1)}%`;

  if (isLoading) {
    return (
      <div className="p-6 flex items-center justify-center h-64">
        <RefreshCw className="w-6 h-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <FolderKanban className="w-6 h-6 text-blue-600" />
          <h1 className="text-2xl font-bold">Portfolio-Gruppen</h1>
        </div>
        <button
          onClick={openCreateForm}
          className="flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        >
          <Plus className="w-4 h-4" />
          Neue Gruppe
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-800 rounded-lg p-4 text-red-700 dark:text-red-300">
          {error}
        </div>
      )}

      {/* Main Content */}
      <div className="grid grid-cols-4 gap-6">
        {/* Sidebar - Consortium List */}
        <div className="col-span-1 space-y-2">
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
            Gruppen
          </h2>

          {consortiums.length === 0 ? (
            <div className="text-sm text-muted-foreground p-4 bg-muted/50 rounded-lg text-center">
              Noch keine Gruppen erstellt.
              <br />
              Erstelle eine Gruppe, um mehrere Portfolios zusammenzufassen.
            </div>
          ) : (
            consortiums.map((consortium) => (
              <div
                key={consortium.id}
                onClick={() => setSelectedConsortiumId(consortium.id)}
                className={`p-3 rounded-lg cursor-pointer transition-colors ${
                  selectedConsortiumId === consortium.id
                    ? 'bg-blue-100 dark:bg-blue-900/40 border-2 border-blue-500'
                    : 'bg-card hover:bg-muted border border-border'
                }`}
              >
                <div className="flex items-center justify-between">
                  <span className="font-medium">{consortium.name}</span>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        openEditForm(consortium);
                      }}
                      className="p-1 hover:bg-muted-foreground/20 rounded"
                      title="Bearbeiten"
                    >
                      <Edit2 className="w-3.5 h-3.5 text-muted-foreground" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDelete(consortium.id);
                      }}
                      className="p-1 hover:bg-red-100 dark:hover:bg-red-900/40 rounded"
                      title="Löschen"
                    >
                      <Trash2 className="w-3.5 h-3.5 text-red-500" />
                    </button>
                  </div>
                </div>
                <div className="text-xs text-muted-foreground mt-1">
                  {consortium.portfolioIds.length} Portfolio{consortium.portfolioIds.length !== 1 ? 's' : ''}
                </div>
              </div>
            ))
          )}
        </div>

        {/* Main Content - Performance */}
        <div className="col-span-3 space-y-6">
          {!selectedConsortiumId || !performance ? (
            <div className="bg-card rounded-lg p-8 text-center text-muted-foreground">
              <FolderKanban className="w-12 h-12 mx-auto mb-4 opacity-50" />
              <p>Wähle eine Gruppe aus, um die kombinierte Performance zu sehen.</p>
            </div>
          ) : isLoadingPerformance ? (
            <div className="flex items-center justify-center h-64">
              <RefreshCw className="w-6 h-6 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <>
              {/* Performance Summary Cards */}
              <div className="grid grid-cols-4 gap-4">
                {/* Total Value */}
                <div className="bg-card rounded-lg p-4 border border-border">
                  <div className="text-sm text-muted-foreground mb-1">Gesamtwert</div>
                  <div className="text-xl font-bold">
                    {formatCurrency(performance.totalValue, performance.currency)}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    Einstand: {formatCurrency(performance.totalCostBasis, performance.currency)}
                  </div>
                </div>

                {/* Gain/Loss */}
                <div className="bg-card rounded-lg p-4 border border-border">
                  <div className="text-sm text-muted-foreground mb-1">Gewinn/Verlust</div>
                  <div
                    className={`text-xl font-bold ${
                      performance.totalGainLoss >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}
                  >
                    {performance.totalGainLoss >= 0 ? '+' : ''}
                    {formatCurrency(performance.totalGainLoss, performance.currency)}
                  </div>
                  <div
                    className={`text-xs mt-1 ${
                      performance.totalGainLossPercent >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}
                  >
                    {formatPercent(performance.totalGainLossPercent)}
                  </div>
                </div>

                {/* TTWROR */}
                <div className="bg-card rounded-lg p-4 border border-border">
                  <div className="text-sm text-muted-foreground mb-1">TTWROR</div>
                  <div
                    className={`text-xl font-bold ${
                      performance.ttwror >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}
                  >
                    {formatPercent(performance.ttwror)}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    p.a.: {formatPercent(performance.ttwrorAnnualized)}
                  </div>
                </div>

                {/* IRR */}
                <div className="bg-card rounded-lg p-4 border border-border">
                  <div className="text-sm text-muted-foreground mb-1">IRR</div>
                  <div
                    className={`text-xl font-bold ${
                      performance.irr >= 0 ? 'text-green-600' : 'text-red-600'
                    }`}
                  >
                    {formatPercent(performance.irr)}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {performance.days} Tage
                  </div>
                </div>
              </div>

              {/* Risk Metrics */}
              {performance.riskMetrics && (
                <div className="bg-card rounded-lg p-4 border border-border">
                  <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                    <BarChart3 className="w-4 h-4" />
                    Risiko-Metriken
                  </h3>
                  <div className="grid grid-cols-4 gap-4">
                    <div>
                      <div className="text-xs text-muted-foreground">Volatilität</div>
                      <div className="font-medium">{formatPercentShort(performance.riskMetrics.volatility)}</div>
                    </div>
                    <div>
                      <div className="text-xs text-muted-foreground">Sharpe Ratio</div>
                      <div className="font-medium">{performance.riskMetrics.sharpeRatio.toFixed(2)}</div>
                    </div>
                    <div>
                      <div className="text-xs text-muted-foreground">Sortino Ratio</div>
                      <div className="font-medium">{performance.riskMetrics.sortinoRatio.toFixed(2)}</div>
                    </div>
                    <div>
                      <div className="text-xs text-muted-foreground">Max. Drawdown</div>
                      <div className="font-medium text-red-600">
                        -{formatPercentShort(performance.riskMetrics.maxDrawdown)}
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* Performance Chart */}
              {chartData.length > 0 && (
                <div className="bg-card rounded-lg p-4 border border-border">
                  <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                    <TrendingUp className="w-4 h-4" />
                    Performance-Verlauf
                  </h3>
                  <div className="h-64">
                    <ResponsiveContainer width="100%" height="100%">
                      <LineChart data={chartData}>
                        <XAxis
                          dataKey="date"
                          tick={{ fontSize: 10 }}
                          tickFormatter={(date) => {
                            const d = new Date(date);
                            return `${d.getDate()}.${d.getMonth() + 1}`;
                          }}
                        />
                        <YAxis
                          tick={{ fontSize: 10 }}
                          tickFormatter={(v) => `${v.toFixed(0)}%`}
                        />
                        <Tooltip
                          formatter={(value) => [`${(value as number).toFixed(2)}%`, '']}
                          labelFormatter={(date) => new Date(date as string).toLocaleDateString('de-DE')}
                        />
                        <Legend />
                        <Line
                          type="monotone"
                          dataKey="Gesamt"
                          stroke="#2563eb"
                          strokeWidth={2}
                          dot={false}
                        />
                        {history?.byPortfolio.map((portfolio) => (
                          <Line
                            key={portfolio.portfolioId}
                            type="monotone"
                            dataKey={portfolio.portfolioName}
                            stroke={portfolio.color}
                            strokeWidth={1.5}
                            strokeDasharray="4 4"
                            dot={false}
                          />
                        ))}
                      </LineChart>
                    </ResponsiveContainer>
                  </div>
                </div>
              )}

              {/* Portfolio Breakdown */}
              <div className="bg-card rounded-lg p-4 border border-border">
                <h3 className="text-sm font-semibold mb-3 flex items-center gap-2">
                  <PieChart className="w-4 h-4" />
                  Portfolio-Aufschlüsselung
                </h3>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-border">
                        <th className="text-left py-2 px-2 font-medium">Portfolio</th>
                        <th className="text-right py-2 px-2 font-medium">Wert</th>
                        <th className="text-right py-2 px-2 font-medium">Anteil</th>
                        <th className="text-right py-2 px-2 font-medium">Einstand</th>
                        <th className="text-right py-2 px-2 font-medium">G/V</th>
                        <th className="text-right py-2 px-2 font-medium">TTWROR</th>
                        <th className="text-right py-2 px-2 font-medium">IRR</th>
                      </tr>
                    </thead>
                    <tbody>
                      {performance.byPortfolio.map((portfolio: PortfolioPerformanceSummary) => (
                        <tr key={portfolio.portfolioId} className="border-b border-border last:border-0">
                          <td className="py-2 px-2 font-medium">{portfolio.portfolioName}</td>
                          <td className="py-2 px-2 text-right">
                            {formatCurrency(portfolio.value, performance.currency)}
                          </td>
                          <td className="py-2 px-2 text-right">
                            {formatPercentShort(portfolio.weight)}
                          </td>
                          <td className="py-2 px-2 text-right">
                            {formatCurrency(portfolio.costBasis, performance.currency)}
                          </td>
                          <td
                            className={`py-2 px-2 text-right ${
                              portfolio.gainLoss >= 0 ? 'text-green-600' : 'text-red-600'
                            }`}
                          >
                            {formatPercent(portfolio.gainLossPercent)}
                          </td>
                          <td
                            className={`py-2 px-2 text-right ${
                              portfolio.ttwror >= 0 ? 'text-green-600' : 'text-red-600'
                            }`}
                          >
                            {formatPercent(portfolio.ttwror)}
                          </td>
                          <td
                            className={`py-2 px-2 text-right ${
                              portfolio.irr >= 0 ? 'text-green-600' : 'text-red-600'
                            }`}
                          >
                            {formatPercent(portfolio.irr)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </>
          )}
        </div>
      </div>

      {/* Create/Edit Form Modal */}
      {showForm && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
          <div className="bg-card rounded-lg shadow-lg w-full max-w-md mx-4">
            <div className="flex items-center justify-between p-4 border-b border-border">
              <h2 className="text-lg font-semibold">
                {editingConsortium ? 'Gruppe bearbeiten' : 'Neue Gruppe erstellen'}
              </h2>
              <button onClick={closeForm} className="p-1 hover:bg-muted rounded">
                <X className="w-5 h-5" />
              </button>
            </div>

            <div className="p-4 space-y-4">
              {/* Name */}
              <div>
                <label className="block text-sm font-medium mb-1">Name</label>
                <input
                  type="text"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                  placeholder="z.B. Familie Gesamt"
                  className="w-full px-3 py-2 border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              {/* Portfolio Selection */}
              <div>
                <label className="block text-sm font-medium mb-2">Portfolios</label>
                <div className="space-y-2 max-h-48 overflow-y-auto">
                  {portfolios.map((portfolio) => (
                    <label
                      key={portfolio.id}
                      className={`flex items-center gap-3 p-2 rounded-lg cursor-pointer transition-colors ${
                        formPortfolioIds.includes(portfolio.id)
                          ? 'bg-blue-100 dark:bg-blue-900/40'
                          : 'hover:bg-muted'
                      }`}
                    >
                      <input
                        type="checkbox"
                        checked={formPortfolioIds.includes(portfolio.id)}
                        onChange={() => togglePortfolio(portfolio.id)}
                        className="w-4 h-4 rounded border-border"
                      />
                      <span className="flex-1">{portfolio.name}</span>
                      {formPortfolioIds.includes(portfolio.id) && (
                        <Check className="w-4 h-4 text-blue-600" />
                      )}
                    </label>
                  ))}
                </div>
                {portfolios.length === 0 && (
                  <p className="text-sm text-muted-foreground text-center py-4">
                    Keine Portfolios vorhanden.
                  </p>
                )}
              </div>
            </div>

            <div className="flex justify-end gap-2 p-4 border-t border-border">
              <button
                onClick={closeForm}
                className="px-4 py-2 text-muted-foreground hover:bg-muted rounded-lg transition-colors"
              >
                Abbrechen
              </button>
              <button
                onClick={handleCreateOrUpdate}
                disabled={!formName.trim() || formPortfolioIds.length === 0}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {editingConsortium ? 'Speichern' : 'Erstellen'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
