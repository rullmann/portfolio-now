/**
 * Holdings view - Bestand (Donut Chart)
 * Donut chart with legend showing portfolio allocation.
 */

import { useState, useEffect, useMemo } from 'react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Customized } from 'recharts';
import { PieChart as PieChartIcon, Newspaper, TrendingUp, TrendingDown } from 'lucide-react';
import type { AggregatedHolding, PortfolioData } from '../types';
import { formatNumber } from '../utils';
import { getBaseCurrency } from '../../lib/api';
import { useCachedLogos } from '../../lib/hooks';
import { useSettingsStore } from '../../store';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { LogoImage } from '../../components/common/SecurityLogo';
import { NewsResearchModal } from '../../components/modals/NewsResearchModal';

// Color palette similar to Portfolio Performance
const COLORS = [
  '#FF6B6B', // Coral red
  '#4ECDC4', // Teal
  '#45B7D1', // Sky blue
  '#96CEB4', // Sage green
  '#FFEAA7', // Pale yellow
  '#DDA0DD', // Plum
  '#98D8C8', // Mint
  '#F7DC6F', // Mustard
  '#BB8FCE', // Lavender
  '#85C1E9', // Light blue
  '#F8B500', // Gold
  '#82E0AA', // Light green
  '#F1948A', // Salmon
  '#85929E', // Steel
  '#D7BDE2', // Light purple
  '#A3E4D7', // Aquamarine
  '#FAD7A0', // Peach
  '#AED6F1', // Powder blue
  '#D5DBDB', // Silver
  '#FADBD8', // Blush
];

interface HoldingsViewProps {
  dbHoldings: AggregatedHolding[];
  dbPortfolios: PortfolioData[];
}

interface ChartDataItem {
  name: string;
  value: number;
  percentValue: number;
  securityId: number;
  color: string;
  currency: string;
  shares: number;
  logoUrl?: string;
  gainLoss: number | null;
  gainLossPercent: number | null;
  costBasis: number;
  currentPrice: number | null;
  purchasePrice: number | null;
  dividendsTotal: number;
  [key: string]: string | number | null | undefined;
}

type ViewMode = 'total' | 'byPortfolio';

const RADIAN = Math.PI / 180;

// Minimum segment size to show logo (5%)
const MIN_LOGO_PERCENT = 0.05;

// Custom component to render logos on pie segments (separate layer, not affected by hover)
interface LogoLayerProps {
  cx: number;
  cy: number;
  innerRadius: number;
  outerRadius: number;
  data: ChartDataItem[];
}

const LogoLayer = ({ cx, cy, innerRadius, outerRadius, data }: LogoLayerProps) => {
  // Don't render if no data or chart not yet sized
  if (!data || data.length === 0 || cx === 0 || cy === 0 || outerRadius === 0) return null;

  // Calculate total value for percent calculation
  const totalValue = data.reduce((sum, item) => sum + item.value, 0);

  // Calculate cumulative angles for each segment
  let currentAngle = 90; // Start from top (90 degrees in Recharts coordinate system)

  return (
    <g className="logo-layer" style={{ pointerEvents: 'none' }}>
      {data.map((item, index) => {
        const percent = totalValue > 0 ? item.value / totalValue : 0;
        const segmentAngle = percent * 360;
        const midAngle = currentAngle - segmentAngle / 2;

        // Update for next segment
        currentAngle -= segmentAngle;

        // Skip if segment is too small or no logo URL
        if (percent < MIN_LOGO_PERCENT || !item.logoUrl || item.logoUrl.length === 0) {
          return null;
        }

        // Calculate position in the middle of the segment
        const radius = innerRadius + (outerRadius - innerRadius) * 0.5;
        const x = cx + radius * Math.cos(-midAngle * RADIAN);
        const y = cy + radius * Math.sin(-midAngle * RADIAN);

        // Logo size based on segment size (between 20 and 48 pixels)
        const logoSize = Math.min(48, Math.max(20, Math.floor(percent * 350)));

        return (
          <image
            key={`logo-${index}`}
            x={x - logoSize / 2}
            y={y - logoSize / 2}
            width={logoSize}
            height={logoSize}
            href={item.logoUrl}
            style={{ pointerEvents: 'none' }}
            clipPath="inset(0% round 4px)"
            // Hide on error - SVG doesn't support onError well, so we use CSS
            onError={(e) => { (e.target as SVGImageElement).style.display = 'none'; }}
          />
        );
      })}
    </g>
  );
};

export function HoldingsView({ dbHoldings, dbPortfolios }: HoldingsViewProps) {
  const [baseCurrency, setBaseCurrency] = useState<string>('EUR');
  const [viewMode, setViewMode] = useState<ViewMode>('total');
  const [selectedPortfolio, setSelectedPortfolio] = useState<number | null>(null);
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const [newsModalHolding, setNewsModalHolding] = useState<AggregatedHolding | null>(null);
  const { keys: secureKeys } = useSecureApiKeys();
  const aiEnabled = useSettingsStore((state) => state.aiEnabled);

  // Prepare securities list for logo loading
  const securitiesForLogos = useMemo(() =>
    dbHoldings.map((h) => ({
      id: h.securityIds[0],
      ticker: undefined,
      name: h.name || '',
    })),
    [dbHoldings]
  );

  // Use cached logos hook
  const { logos: cachedLogos } = useCachedLogos(securitiesForLogos, secureKeys.brandfetchApiKey);

  // Fetch base currency
  useEffect(() => {
    getBaseCurrency()
      .then(setBaseCurrency)
      .catch(() => setBaseCurrency('EUR'));
  }, []);

  // Calculate total value
  const totalValue = useMemo(() => {
    return dbHoldings.reduce((sum, h) => sum + (h.currentValue || 0), 0);
  }, [dbHoldings]);

  // Prepare holdings data with logos
  const holdingsWithLogos = useMemo(() => {
    return dbHoldings.map((h) => ({
      ...h,
      logoUrl: h.customLogo || cachedLogos.get(h.securityIds[0])?.url,
    }));
  }, [dbHoldings, cachedLogos]);

  // Prepare chart data based on view mode
  const chartData = useMemo((): ChartDataItem[] => {
    if (dbHoldings.length === 0) return [];

    let data: ChartDataItem[];

    if (viewMode === 'total') {
      data = holdingsWithLogos
        .filter((h) => (h.currentValue || 0) > 0)
        .sort((a, b) => (b.currentValue || 0) - (a.currentValue || 0))
        .map((holding, index) => ({
          name: holding.name,
          value: holding.currentValue || 0,
          percentValue: totalValue > 0 ? ((holding.currentValue || 0) / totalValue) * 100 : 0,
          securityId: holding.securityIds[0],
          color: COLORS[index % COLORS.length],
          currency: holding.currency,
          shares: holding.totalShares,
          logoUrl: holding.logoUrl,
          gainLoss: holding.gainLoss ?? null,
          gainLossPercent: holding.gainLossPercent ?? null,
          costBasis: holding.costBasis,
          currentPrice: holding.currentPrice,
          purchasePrice: holding.purchasePrice ?? null,
          dividendsTotal: holding.dividendsTotal,
        }));
    } else if (selectedPortfolio !== null) {
      const portfolioHoldings = holdingsWithLogos
        .map((holding) => {
          const portfolioEntry = holding.portfolios.find(
            (p) => dbPortfolios.find((dp) => dp.name === p.portfolioName)?.id === selectedPortfolio
          );
          if (!portfolioEntry || (portfolioEntry.value || 0) <= 0) return null;
          return {
            ...holding,
            currentValue: portfolioEntry.value || 0,
            totalShares: portfolioEntry.shares,
          };
        })
        .filter((h): h is NonNullable<typeof h> => h !== null);

      const portfolioTotal = portfolioHoldings.reduce((sum, h) => sum + (h.currentValue || 0), 0);

      data = portfolioHoldings
        .sort((a, b) => (b.currentValue || 0) - (a.currentValue || 0))
        .map((holding, index) => ({
          name: holding.name,
          value: holding.currentValue || 0,
          percentValue: portfolioTotal > 0 ? ((holding.currentValue || 0) / portfolioTotal) * 100 : 0,
          securityId: holding.securityIds[0],
          color: COLORS[index % COLORS.length],
          currency: holding.currency,
          shares: holding.totalShares,
          logoUrl: holding.logoUrl,
          gainLoss: holding.gainLoss ?? null,
          gainLossPercent: holding.gainLossPercent ?? null,
          costBasis: holding.costBasis,
          currentPrice: holding.currentPrice,
          purchasePrice: holding.purchasePrice ?? null,
          dividendsTotal: holding.dividendsTotal,
        }));
    } else {
      return [];
    }

    return data;
  }, [dbHoldings, dbPortfolios, holdingsWithLogos, viewMode, selectedPortfolio, totalValue]);

  // Calculate displayed total
  const displayedTotal = useMemo(() => {
    return chartData.reduce((sum, d) => sum + d.value, 0);
  }, [chartData]);

  // Custom tooltip
  const CustomTooltip = ({ active, payload }: { active?: boolean; payload?: Array<{ payload: ChartDataItem }> }) => {
    if (!active || !payload || payload.length === 0) return null;

    const data = payload[0].payload;
    return (
      <div className="bg-card border border-border rounded-lg p-3 shadow-lg z-50">
        <div className="flex items-center gap-2 mb-2">
          <LogoImage src={data.logoUrl} size={24} />
          <span className="font-medium">{data.name}</span>
        </div>
        <div className="space-y-1 text-sm">
          <div className="flex justify-between gap-4">
            <span className="text-muted-foreground">Wert:</span>
            <span className="font-medium">{formatNumber(data.value)} {baseCurrency}</span>
          </div>
          <div className="flex justify-between gap-4">
            <span className="text-muted-foreground">Anteil:</span>
            <span className="font-medium">{data.percentValue.toFixed(2)}%</span>
          </div>
          <div className="flex justify-between gap-4">
            <span className="text-muted-foreground">Bestand:</span>
            <span className="font-medium">{data.shares.toLocaleString('de-DE', { maximumFractionDigits: 4 })}</span>
          </div>
        </div>
      </div>
    );
  };

  if (dbHoldings.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center">
        <PieChartIcon className="w-16 h-16 text-muted-foreground mb-4" />
        <h2 className="text-2xl font-semibold mb-2">Keine Bestände vorhanden</h2>
        <p className="text-muted-foreground">
          Importiere eine .portfolio Datei, um deine Bestände zu sehen.
        </p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col gap-4">
      {/* Header */}
      <div className="flex items-center justify-between flex-shrink-0">
        <div>
          <h1 className="text-2xl font-bold">Bestand</h1>
          <p className="text-muted-foreground">
            {chartData.length} Positionen · {formatNumber(displayedTotal)} {baseCurrency}
          </p>
        </div>

        {/* View Mode Selector */}
        <div className="flex items-center gap-2">
          <select
            value={viewMode}
            onChange={(e) => {
              setViewMode(e.target.value as ViewMode);
              if (e.target.value === 'total') {
                setSelectedPortfolio(null);
              } else if (dbPortfolios.length > 0) {
                setSelectedPortfolio(dbPortfolios.filter(p => !p.isRetired)[0]?.id || null);
              }
            }}
            className="px-3 py-2 text-sm border border-border rounded-md bg-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <option value="total">Gesamt</option>
            <option value="byPortfolio">Nach Depot</option>
          </select>

          {viewMode === 'byPortfolio' && (
            <select
              value={selectedPortfolio || ''}
              onChange={(e) => setSelectedPortfolio(Number(e.target.value))}
              className="px-3 py-2 text-sm border border-border rounded-md bg-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              {dbPortfolios
                .filter((p) => !p.isRetired)
                .map((portfolio) => (
                  <option key={portfolio.id} value={portfolio.id}>
                    {portfolio.name}
                  </option>
                ))}
            </select>
          )}
        </div>
      </div>

      {/* Summary Cards */}
      {(() => {
        const totalGainLoss = chartData.reduce((sum, d) => sum + (d.gainLoss || 0), 0);
        const totalCostBasis = chartData.reduce((sum, d) => sum + d.costBasis, 0);
        const totalGainPercent = totalCostBasis > 0 ? (totalGainLoss / totalCostBasis) * 100 : 0;
        const totalDividends = chartData.reduce((sum, d) => sum + d.dividendsTotal, 0);
        return (
          <div className="grid grid-cols-4 gap-3 flex-shrink-0">
            <div className="bg-card rounded-lg border border-border p-3">
              <div className="text-xs text-muted-foreground mb-1">Marktwert</div>
              <div className="text-lg font-semibold tabular-nums">{formatNumber(displayedTotal)} {baseCurrency}</div>
            </div>
            <div className="bg-card rounded-lg border border-border p-3">
              <div className="text-xs text-muted-foreground mb-1">Einstandswert</div>
              <div className="text-lg font-semibold tabular-nums">{formatNumber(totalCostBasis)} {baseCurrency}</div>
            </div>
            <div className="bg-card rounded-lg border border-border p-3">
              <div className="text-xs text-muted-foreground mb-1">Gewinn / Verlust</div>
              <div className={`text-lg font-semibold tabular-nums ${totalGainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                {totalGainLoss >= 0 ? '+' : ''}{formatNumber(totalGainLoss)} {baseCurrency}
                <span className="text-sm ml-1">({totalGainPercent >= 0 ? '+' : ''}{totalGainPercent.toFixed(1)}%)</span>
              </div>
            </div>
            <div className="bg-card rounded-lg border border-border p-3">
              <div className="text-xs text-muted-foreground mb-1">Dividenden</div>
              <div className="text-lg font-semibold tabular-nums">{formatNumber(totalDividends)} {baseCurrency}</div>
            </div>
          </div>
        );
      })()}

      {/* Main Content: Chart + Table */}
      <div className="flex-1 min-h-0 flex gap-4">
        {/* Chart Container - compact */}
        <div
          className="bg-card rounded-lg border border-border w-[320px] flex-shrink-0 relative p-2"
          role="img"
          aria-label={`Donut-Diagramm zeigt ${chartData.length} Positionen`}
        >
          <div className="sr-only">
            <h2>Vermögensverteilung</h2>
            <ul>
              {chartData.map((item) => (
                <li key={item.securityId}>
                  {item.name}: {formatNumber(item.value)} {baseCurrency} ({item.percentValue.toFixed(2)}%)
                </li>
              ))}
            </ul>
          </div>

          <ResponsiveContainer width="100%" height="100%">
            <PieChart>
              <Pie
                data={chartData}
                cx="50%"
                cy="50%"
                innerRadius="55%"
                outerRadius="92%"
                paddingAngle={1}
                dataKey="value"
                onMouseEnter={(_, index) => setHoveredIndex(index)}
                onMouseLeave={() => setHoveredIndex(null)}
              >
                {chartData.map((entry, index) => (
                  <Cell
                    key={`cell-${index}`}
                    fill={entry.color}
                    stroke={hoveredIndex === index ? 'hsl(var(--foreground))' : 'hsl(var(--background))'}
                    strokeWidth={hoveredIndex === index ? 3 : 2}
                    style={{
                      filter: hoveredIndex === index ? 'brightness(1.1)' : 'none',
                      cursor: 'pointer',
                    }}
                  />
                ))}
              </Pie>
              <Customized
                component={(props: { width?: number; height?: number }) => {
                  const { width = 0, height = 0 } = props;
                  const cx = width / 2;
                  const cy = height / 2;
                  const minDim = Math.min(width, height);
                  const outerRadius = minDim * 0.92 / 2;
                  const innerRadius = outerRadius * 0.55;
                  return (
                    <LogoLayer
                      cx={cx}
                      cy={cy}
                      innerRadius={innerRadius}
                      outerRadius={outerRadius}
                      data={chartData}
                    />
                  );
                }}
              />
              <Tooltip content={<CustomTooltip />} />
            </PieChart>
          </ResponsiveContainer>

          {/* Center Total */}
          <div className="absolute inset-0 flex items-center justify-center pointer-events-none" aria-hidden="true">
            <div className="text-center">
              <div className="text-xs text-muted-foreground">Gesamt</div>
              <div className="text-xl font-bold tabular-nums">{formatNumber(displayedTotal)}</div>
              <div className="text-xs text-muted-foreground">{baseCurrency}</div>
            </div>
          </div>
        </div>

        {/* Holdings Table */}
        <div className="flex-1 min-w-0 bg-card rounded-lg border border-border flex flex-col">
          {/* Table Header */}
          <div className="grid grid-cols-[minmax(0,2.5fr)_repeat(5,minmax(0,1fr))] gap-2 px-4 py-2.5 border-b border-border text-xs text-muted-foreground font-medium">
            <div>Position</div>
            <div className="text-right">Kurs</div>
            <div className="text-right">Marktwert</div>
            <div className="text-right">Einstand</div>
            <div className="text-right">Gewinn/Verlust</div>
            <div className="text-right">Anteil</div>
          </div>

          {/* Table Body */}
          <div className="flex-1 overflow-y-auto">
            {chartData.map((item, index) => {
              const isPositive = (item.gainLoss ?? 0) >= 0;
              return (
                <div
                  key={item.securityId}
                  className={`group grid grid-cols-[minmax(0,2.5fr)_repeat(5,minmax(0,1fr))] gap-2 px-4 py-2.5 items-center cursor-pointer transition-colors border-b border-border/50 last:border-b-0 ${
                    hoveredIndex === index ? 'bg-accent' : 'hover:bg-accent/50'
                  }`}
                  onMouseEnter={() => setHoveredIndex(index)}
                  onMouseLeave={() => setHoveredIndex(null)}
                >
                  {/* Position: Color + Logo + Name + Shares */}
                  <div className="flex items-center gap-2.5 min-w-0">
                    <div
                      className="w-2.5 h-2.5 rounded-sm flex-shrink-0"
                      style={{ backgroundColor: item.color }}
                    />
                    <LogoImage src={item.logoUrl} size={28} />
                    <div className="min-w-0 flex-1">
                      <div className="font-medium text-sm truncate" title={item.name}>
                        {item.name}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {item.shares.toLocaleString('de-DE', { maximumFractionDigits: 2 })} Stk. · {item.currency}
                      </div>
                    </div>
                    {aiEnabled && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          const holding = dbHoldings.find(h => h.securityIds.includes(item.securityId));
                          if (holding) setNewsModalHolding(holding);
                        }}
                        className="p-1 hover:bg-muted rounded-md transition-colors opacity-0 group-hover:opacity-100 flex-shrink-0"
                        title="Nachrichten recherchieren"
                      >
                        <Newspaper size={14} className="text-muted-foreground" />
                      </button>
                    )}
                  </div>

                  {/* Current Price */}
                  <div className="text-right text-sm tabular-nums">
                    {item.currentPrice != null ? formatNumber(item.currentPrice) : '–'}
                  </div>

                  {/* Market Value */}
                  <div className="text-right text-sm font-medium tabular-nums">
                    {formatNumber(item.value)}
                  </div>

                  {/* Cost Basis */}
                  <div className="text-right text-sm tabular-nums text-muted-foreground">
                    {formatNumber(item.costBasis)}
                  </div>

                  {/* Gain/Loss */}
                  <div className="text-right">
                    {item.gainLoss != null ? (
                      <div className={`text-sm tabular-nums ${isPositive ? 'text-green-600' : 'text-red-600'}`}>
                        <div className="flex items-center justify-end gap-1">
                          {isPositive ? <TrendingUp size={12} /> : <TrendingDown size={12} />}
                          <span>{isPositive ? '+' : ''}{formatNumber(item.gainLoss)}</span>
                        </div>
                        <div className="text-xs">
                          {isPositive ? '+' : ''}{(item.gainLossPercent ?? 0).toFixed(1)}%
                        </div>
                      </div>
                    ) : (
                      <span className="text-sm text-muted-foreground">–</span>
                    )}
                  </div>

                  {/* Allocation % with bar */}
                  <div className="text-right">
                    <div className="text-sm font-medium tabular-nums">{item.percentValue.toFixed(1)}%</div>
                    <div className="mt-1 h-1.5 bg-muted rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all"
                        style={{ width: `${Math.min(item.percentValue, 100)}%`, backgroundColor: item.color }}
                      />
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* News Research Modal */}
      {newsModalHolding && (
        <NewsResearchModal
          isOpen={!!newsModalHolding}
          onClose={() => setNewsModalHolding(null)}
          security={{
            id: newsModalHolding.securityIds[0],
            name: newsModalHolding.name,
            isin: newsModalHolding.isin,
            currency: newsModalHolding.currency,
          }}
          currentPrice={newsModalHolding.currentPrice ?? undefined}
        />
      )}
    </div>
  );
}
