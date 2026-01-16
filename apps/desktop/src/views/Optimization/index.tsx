/**
 * Portfolio Optimization View
 *
 * Implements Modern Portfolio Theory (Markowitz):
 * - Efficient Frontier visualization
 * - Correlation Matrix heatmap
 * - Optimal portfolio suggestions
 */

import { useState, useEffect, useMemo } from 'react';
import {
  Target,
  TrendingUp,
  RefreshCw,
  AlertTriangle,
  Info,
  Sparkles,
  Grid3X3,
} from 'lucide-react';
import {
  ScatterChart,
  Scatter,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import {
  calculateCorrelationMatrix,
  calculateEfficientFrontier,
  type CorrelationMatrix,
  type EfficientFrontier,
} from '../../lib/api';

type TabType = 'frontier' | 'correlation';

export function OptimizationView() {
  const [activeTab, setActiveTab] = useState<TabType>('frontier');
  const [frontier, setFrontier] = useState<EfficientFrontier | null>(null);
  const [correlation, setCorrelation] = useState<CorrelationMatrix | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setIsLoading(true);
    setError(null);

    try {
      const [frontierData, correlationData] = await Promise.all([
        calculateEfficientFrontier({ numPoints: 50 }),
        calculateCorrelationMatrix(),
      ]);
      setFrontier(frontierData);
      setCorrelation(correlationData);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="h-full flex flex-col p-4 space-y-4 overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Target className="w-6 h-6 text-primary" />
          <h1 className="text-xl font-semibold">Portfolio-Optimierung</h1>
        </div>
        <button
          onClick={loadData}
          disabled={isLoading}
          className="flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 disabled:opacity-50"
        >
          <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
          Aktualisieren
        </button>
      </div>

      {/* Tabs */}
      <div className="flex gap-2">
        <button
          onClick={() => setActiveTab('frontier')}
          className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
            activeTab === 'frontier'
              ? 'bg-primary text-primary-foreground'
              : 'bg-muted hover:bg-muted/80'
          }`}
        >
          <TrendingUp size={18} />
          Efficient Frontier
        </button>
        <button
          onClick={() => setActiveTab('correlation')}
          className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
            activeTab === 'correlation'
              ? 'bg-primary text-primary-foreground'
              : 'bg-muted hover:bg-muted/80'
          }`}
        >
          <Grid3X3 size={18} />
          Korrelationsmatrix
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="flex items-center gap-2 p-4 bg-destructive/10 border border-destructive/20 rounded-md text-destructive">
          <AlertTriangle size={20} />
          {error}
        </div>
      )}

      {/* Loading */}
      {isLoading && (
        <div className="flex-1 flex items-center justify-center">
          <RefreshCw className="w-8 h-8 animate-spin text-primary" />
        </div>
      )}

      {/* Content */}
      {!isLoading && !error && (
        <div className="flex-1 min-h-0 overflow-auto">
          {activeTab === 'frontier' && frontier && (
            <EfficientFrontierChart data={frontier} />
          )}
          {activeTab === 'correlation' && correlation && (
            <CorrelationMatrixView data={correlation} />
          )}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Efficient Frontier Chart
// ============================================================================

function EfficientFrontierChart({ data }: { data: EfficientFrontier }) {
  const chartData = useMemo(() => {
    return data.points.map((p, i) => ({
      id: i,
      volatility: p.volatility * 100,
      return: p.expectedReturn * 100,
      sharpe: p.sharpeRatio,
      type: 'frontier',
    }));
  }, [data]);

  const currentPortfolio = {
    volatility: data.currentPortfolio.volatility * 100,
    return: data.currentPortfolio.expectedReturn * 100,
    sharpe: data.currentPortfolio.sharpeRatio,
    type: 'current',
  };

  const minVariance = {
    volatility: data.minVariancePortfolio.volatility * 100,
    return: data.minVariancePortfolio.expectedReturn * 100,
    sharpe: data.minVariancePortfolio.sharpeRatio,
    type: 'minVar',
  };

  const maxSharpe = {
    volatility: data.maxSharpePortfolio.volatility * 100,
    return: data.maxSharpePortfolio.expectedReturn * 100,
    sharpe: data.maxSharpePortfolio.sharpeRatio,
    type: 'maxSharpe',
  };

  const CustomTooltip = ({ active, payload }: { active?: boolean; payload?: Array<{ payload: { volatility: number; return: number; sharpe: number; type: string } }> }) => {
    if (!active || !payload || !payload[0]) return null;

    const p = payload[0].payload;
    const labels: Record<string, string> = {
      frontier: 'Efficient Frontier',
      current: 'Aktuelles Portfolio',
      minVar: 'Minimum Varianz',
      maxSharpe: 'Maximum Sharpe',
    };

    return (
      <div className="bg-popover border border-border rounded-lg shadow-lg p-3">
        <div className="font-medium mb-2">{labels[p.type] || 'Portfolio'}</div>
        <div className="space-y-1 text-sm">
          <div className="flex justify-between gap-4">
            <span className="text-muted-foreground">Rendite:</span>
            <span className={p.return >= 0 ? 'text-green-600' : 'text-red-600'}>
              {p.return.toFixed(2)}%
            </span>
          </div>
          <div className="flex justify-between gap-4">
            <span className="text-muted-foreground">Volatilität:</span>
            <span>{p.volatility.toFixed(2)}%</span>
          </div>
          <div className="flex justify-between gap-4">
            <span className="text-muted-foreground">Sharpe Ratio:</span>
            <span>{p.sharpe.toFixed(2)}</span>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="space-y-4">
      {/* Info Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <PortfolioCard
          title="Aktuelles Portfolio"
          icon={<Target size={18} className="text-blue-500" />}
          returnValue={currentPortfolio.return}
          volatility={currentPortfolio.volatility}
          sharpe={currentPortfolio.sharpe}
          weights={data.currentPortfolio.weights}
          securities={data.securities}
        />
        <PortfolioCard
          title="Minimum Varianz"
          icon={<Sparkles size={18} className="text-green-500" />}
          returnValue={minVariance.return}
          volatility={minVariance.volatility}
          sharpe={minVariance.sharpe}
          weights={data.minVariancePortfolio.weights}
          securities={data.securities}
          highlight="volatility"
        />
        <PortfolioCard
          title="Maximum Sharpe"
          icon={<TrendingUp size={18} className="text-orange-500" />}
          returnValue={maxSharpe.return}
          volatility={maxSharpe.volatility}
          sharpe={maxSharpe.sharpe}
          weights={data.maxSharpePortfolio.weights}
          securities={data.securities}
          highlight="sharpe"
        />
      </div>

      {/* Chart */}
      <div className="bg-card rounded-lg border border-border p-4">
        <h3 className="font-semibold mb-4">Efficient Frontier</h3>
        <div className="h-[400px]">
          <ResponsiveContainer width="100%" height="100%">
            <ScatterChart margin={{ top: 20, right: 20, bottom: 40, left: 40 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
              <XAxis
                type="number"
                dataKey="volatility"
                name="Volatilität"
                unit="%"
                tick={{ fontSize: 12 }}
                label={{
                  value: 'Volatilität (%)',
                  position: 'bottom',
                  offset: 20,
                  style: { fontSize: 12 },
                }}
              />
              <YAxis
                type="number"
                dataKey="return"
                name="Rendite"
                unit="%"
                tick={{ fontSize: 12 }}
                label={{
                  value: 'Erwartete Rendite (%)',
                  angle: -90,
                  position: 'left',
                  offset: 10,
                  style: { fontSize: 12 },
                }}
              />
              <Tooltip content={<CustomTooltip />} />
              <Legend />

              {/* Efficient Frontier Line */}
              <Scatter
                name="Efficient Frontier"
                data={chartData}
                fill="#3b82f6"
                line={{ stroke: '#3b82f6', strokeWidth: 2 }}
                shape="circle"
              />

              {/* Current Portfolio */}
              <Scatter
                name="Aktuelles Portfolio"
                data={[currentPortfolio]}
                fill="#ef4444"
                shape="star"
              />

              {/* Min Variance */}
              <Scatter
                name="Min. Varianz"
                data={[minVariance]}
                fill="#10b981"
                shape="diamond"
              />

              {/* Max Sharpe */}
              <Scatter
                name="Max. Sharpe"
                data={[maxSharpe]}
                fill="#f59e0b"
                shape="triangle"
              />
            </ScatterChart>
          </ResponsiveContainer>
        </div>
      </div>

      {/* Explanation */}
      <div className="bg-muted/30 rounded-lg p-4 text-sm text-muted-foreground">
        <div className="flex items-start gap-2">
          <Info size={16} className="mt-0.5 flex-shrink-0" />
          <div>
            <strong>Efficient Frontier:</strong> Die Kurve zeigt alle optimalen Portfolios,
            die für ein gegebenes Risiko die maximale Rendite erzielen. Portfolios unterhalb
            der Kurve sind suboptimal. Das &quot;Maximum Sharpe&quot; Portfolio bietet das beste
            Risiko/Rendite-Verhältnis.
          </div>
        </div>
      </div>
    </div>
  );
}

function PortfolioCard({
  title,
  icon,
  returnValue,
  volatility,
  sharpe,
  weights,
  securities,
  highlight,
}: {
  title: string;
  icon: React.ReactNode;
  returnValue: number;
  volatility: number;
  sharpe: number;
  weights: Record<number, number>;
  securities: { id: number; name: string }[];
  highlight?: 'volatility' | 'sharpe';
}) {
  const [showWeights, setShowWeights] = useState(false);

  const sortedWeights = Object.entries(weights)
    .map(([id, weight]) => ({
      id: Number(id),
      name: securities.find((s) => s.id === Number(id))?.name || 'Unknown',
      weight,
    }))
    .sort((a, b) => b.weight - a.weight);

  return (
    <div className="bg-card rounded-lg border border-border p-4">
      <div className="flex items-center gap-2 mb-3">
        {icon}
        <span className="font-medium">{title}</span>
      </div>

      <div className="grid grid-cols-3 gap-2 text-center">
        <div>
          <div className="text-xs text-muted-foreground">Rendite</div>
          <div className={`font-semibold ${returnValue >= 0 ? 'text-green-600' : 'text-red-600'}`}>
            {returnValue.toFixed(1)}%
          </div>
        </div>
        <div>
          <div className="text-xs text-muted-foreground">Volatilität</div>
          <div className={`font-semibold ${highlight === 'volatility' ? 'text-green-600' : ''}`}>
            {volatility.toFixed(1)}%
          </div>
        </div>
        <div>
          <div className="text-xs text-muted-foreground">Sharpe</div>
          <div className={`font-semibold ${highlight === 'sharpe' ? 'text-orange-500' : ''}`}>
            {sharpe.toFixed(2)}
          </div>
        </div>
      </div>

      <button
        onClick={() => setShowWeights(!showWeights)}
        className="mt-3 w-full text-xs text-muted-foreground hover:text-foreground"
      >
        {showWeights ? 'Gewichtung ausblenden' : 'Gewichtung anzeigen'}
      </button>

      {showWeights && (
        <div className="mt-2 space-y-1 text-xs">
          {sortedWeights.slice(0, 5).map((w) => (
            <div key={w.id} className="flex justify-between">
              <span className="truncate mr-2">{w.name}</span>
              <span className="font-medium">{(w.weight * 100).toFixed(1)}%</span>
            </div>
          ))}
          {sortedWeights.length > 5 && (
            <div className="text-muted-foreground">+{sortedWeights.length - 5} weitere</div>
          )}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Correlation Matrix View
// ============================================================================

function CorrelationMatrixView({ data }: { data: CorrelationMatrix }) {
  if (data.securities.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        Keine Daten für Korrelationsmatrix verfügbar.
      </div>
    );
  }

  const getCorrelationColor = (value: number) => {
    if (value >= 0.7) return 'bg-red-500';
    if (value >= 0.3) return 'bg-orange-400';
    if (value >= -0.3) return 'bg-gray-300';
    if (value >= -0.7) return 'bg-blue-400';
    return 'bg-blue-600';
  };

  const getCorrelationTextColor = (value: number) => {
    if (Math.abs(value) >= 0.5) return 'text-white';
    return 'text-foreground';
  };

  return (
    <div className="space-y-6">
      {/* Matrix */}
      <div className="bg-card rounded-lg border border-border p-4 overflow-x-auto">
        <h3 className="font-semibold mb-4">Korrelationsmatrix</h3>
        <div className="inline-block">
          <table className="border-collapse">
            <thead>
              <tr>
                <th className="p-2 text-xs"></th>
                {data.securities.map((s) => (
                  <th
                    key={s.id}
                    className="p-2 text-xs font-medium transform -rotate-45 origin-left whitespace-nowrap"
                    style={{ minWidth: '60px' }}
                  >
                    {s.ticker || s.name.slice(0, 10)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data.securities.map((rowSec, i) => (
                <tr key={rowSec.id}>
                  <td className="p-2 text-xs font-medium whitespace-nowrap">
                    {rowSec.ticker || rowSec.name.slice(0, 15)}
                  </td>
                  {data.matrix[i].map((value, j) => (
                    <td
                      key={j}
                      className={`p-2 text-center text-xs font-medium ${getCorrelationColor(value)} ${getCorrelationTextColor(value)}`}
                      style={{ minWidth: '50px', minHeight: '50px' }}
                      title={`${rowSec.name} <-> ${data.securities[j].name}: ${value.toFixed(2)}`}
                    >
                      {i === j ? '1.00' : value.toFixed(2)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Legend */}
        <div className="flex items-center gap-4 mt-4 text-xs">
          <span className="text-muted-foreground">Korrelation:</span>
          <div className="flex items-center gap-1">
            <div className="w-4 h-4 bg-blue-600 rounded"></div>
            <span>-1.0</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-4 h-4 bg-blue-400 rounded"></div>
            <span>-0.5</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-4 h-4 bg-gray-300 rounded"></div>
            <span>0</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-4 h-4 bg-orange-400 rounded"></div>
            <span>+0.5</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-4 h-4 bg-red-500 rounded"></div>
            <span>+1.0</span>
          </div>
        </div>
      </div>

      {/* Top Correlations */}
      <div className="bg-card rounded-lg border border-border p-4">
        <h3 className="font-semibold mb-4">Stärkste Korrelationen</h3>
        <div className="space-y-2">
          {data.pairs.slice(0, 10).map((pair, i) => (
            <div
              key={i}
              className="flex items-center justify-between p-2 bg-muted/30 rounded-md"
            >
              <div className="flex items-center gap-2 text-sm">
                <span className="font-medium">{pair.security1Name}</span>
                <span className="text-muted-foreground">↔</span>
                <span className="font-medium">{pair.security2Name}</span>
              </div>
              <div
                className={`px-2 py-1 rounded text-sm font-medium ${
                  pair.correlation >= 0.5
                    ? 'bg-red-500/20 text-red-600'
                    : pair.correlation <= -0.5
                    ? 'bg-blue-500/20 text-blue-600'
                    : 'bg-gray-500/20 text-gray-600'
                }`}
              >
                {pair.correlation >= 0 ? '+' : ''}
                {pair.correlation.toFixed(2)}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Explanation */}
      <div className="bg-muted/30 rounded-lg p-4 text-sm text-muted-foreground">
        <div className="flex items-start gap-2">
          <Info size={16} className="mt-0.5 flex-shrink-0" />
          <div>
            <strong>Korrelation:</strong> Zeigt wie stark Wertpapiere zusammen schwanken.
            <ul className="mt-2 space-y-1">
              <li>
                <span className="text-red-500 font-medium">+0.7 bis +1.0:</span> Stark positiv
                korreliert - bewegen sich gemeinsam
              </li>
              <li>
                <span className="text-blue-500 font-medium">-0.7 bis -1.0:</span> Stark negativ
                korreliert - bewegen sich gegensätzlich (gut für Diversifikation)
              </li>
              <li>
                <span className="text-gray-500 font-medium">-0.3 bis +0.3:</span> Schwach
                korreliert - unabhängig (ideal für Diversifikation)
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
