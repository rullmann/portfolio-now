/**
 * Holdings view - Bestand (Donut Chart)
 * Donut chart with legend showing portfolio allocation.
 */

import { useState, useEffect, useMemo } from 'react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Customized } from 'recharts';
import { Building2, PieChart as PieChartIcon } from 'lucide-react';
import type { AggregatedHolding, PortfolioData } from '../types';
import { formatNumber } from '../utils';
import { getBaseCurrency } from '../../lib/api';
import { useCachedLogos } from '../../lib/hooks';
import { useSettingsStore } from '../../store';

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
  [key: string]: string | number | undefined;
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
  const brandfetchApiKey = useSettingsStore((state) => state.brandfetchApiKey);

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
  const { logos: cachedLogos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

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
          {data.logoUrl ? (
            <img src={data.logoUrl} alt="" className="w-6 h-6 rounded" crossOrigin="anonymous" />
          ) : (
            <Building2 size={24} className="text-muted-foreground" />
          )}
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
          Importieren Sie eine .portfolio Datei, um Ihre Bestände zu sehen.
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

      {/* Main Content: Chart + Legend side by side */}
      <div className="flex-1 min-h-0 flex gap-4">
        {/* Chart Container */}
        <div
          className="bg-card rounded-lg border border-border flex-1 min-w-0 relative p-2"
          role="img"
          aria-label={`Donut-Diagramm zeigt ${chartData.length} Positionen mit Gesamtwert ${formatNumber(displayedTotal)} ${baseCurrency}`}
        >
          {/* Screen reader summary */}
          <div className="sr-only">
            <h2>Vermögensverteilung</h2>
            <p>Gesamtwert: {formatNumber(displayedTotal)} {baseCurrency}</p>
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
                outerRadius="95%"
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
                  const outerRadius = minDim * 0.95 / 2;
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
              <div className="text-sm text-muted-foreground">Gesamt</div>
              <div className="text-2xl font-bold">{formatNumber(displayedTotal)}</div>
              <div className="text-sm text-muted-foreground">{baseCurrency}</div>
            </div>
          </div>
        </div>

        {/* Legend */}
        <div className="w-80 bg-card rounded-lg border border-border flex flex-col">
          <div className="p-3 border-b border-border">
            <h3 className="font-semibold text-sm">Positionen</h3>
          </div>
          <div className="flex-1 overflow-y-auto p-2">
            {chartData.map((item, index) => (
              <div
                key={item.securityId}
                className={`flex items-center gap-3 p-2 rounded-md cursor-pointer transition-colors ${
                  hoveredIndex === index ? 'bg-accent' : 'hover:bg-accent/50'
                }`}
                onMouseEnter={() => setHoveredIndex(index)}
                onMouseLeave={() => setHoveredIndex(null)}
              >
                {/* Color indicator */}
                <div
                  className="w-3 h-3 rounded-sm flex-shrink-0"
                  style={{ backgroundColor: item.color }}
                />

                {/* Logo */}
                {item.logoUrl ? (
                  <img
                    src={item.logoUrl}
                    alt=""
                    className="w-6 h-6 rounded flex-shrink-0"
                    crossOrigin="anonymous"
                  />
                ) : (
                  <Building2 size={24} className="text-muted-foreground flex-shrink-0" />
                )}

                {/* Name and details */}
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-sm truncate" title={item.name}>
                    {item.name}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {formatNumber(item.value)} {baseCurrency}
                  </div>
                </div>

                {/* Percentage */}
                <div className="text-sm font-medium tabular-nums">
                  {item.percentValue.toFixed(1)}%
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
