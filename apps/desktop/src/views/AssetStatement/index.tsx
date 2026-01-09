/**
 * Asset Statement View (Vermögensaufstellung)
 * Shows holdings table with PP-style columns and a chart with cost basis line.
 * Clicking on a row opens the SecurityDetailChartModal with price history,
 * FIFO cost basis evolution, and trade markers.
 */

import { useState, useEffect, useMemo, useRef } from 'react';
import { Building2, Table2, LineChart as LineChartIcon, ArrowUpDown, ArrowUp, ArrowDown } from 'lucide-react';
import { createChart, AreaSeries, LineSeries } from 'lightweight-charts';
import type { IChartApi, ISeriesApi, LineData, AreaData, Time } from 'lightweight-charts';
import type { AggregatedHolding, PortfolioData } from '../types';
import { formatNumber } from '../utils';
import { getBaseCurrency, getPortfolioHistory } from '../../lib/api';
import { useCachedLogos } from '../../lib/hooks';
import { useSettingsStore } from '../../store';
import { SecurityDetailChartModal } from '../../components/modals';

interface AssetStatementViewProps {
  dbHoldings: AggregatedHolding[];
  dbPortfolios: PortfolioData[];
}

type SortField = 'name' | 'shares' | 'purchasePrice' | 'costBasis' | 'currentValue' | 'gainLoss' | 'gainLossPercent' | 'dividends';
type SortDir = 'asc' | 'desc';
type DisplayMode = 'table' | 'chart';

// Table header with sort functionality
interface SortableHeaderProps {
  label: string;
  field: SortField;
  currentSort: SortField;
  currentDir: SortDir;
  onSort: (field: SortField) => void;
  align?: 'left' | 'right';
}

const SortableHeader = ({ label, field, currentSort, currentDir, onSort, align = 'right' }: SortableHeaderProps) => (
  <th
    className={`py-2 px-3 text-xs font-medium text-muted-foreground cursor-pointer hover:text-foreground ${
      align === 'left' ? 'text-left' : 'text-right'
    }`}
    onClick={() => onSort(field)}
  >
    <div className={`flex items-center gap-1 ${align === 'right' ? 'justify-end' : ''}`}>
      {label}
      {currentSort === field ? (
        currentDir === 'asc' ? <ArrowUp size={12} /> : <ArrowDown size={12} />
      ) : (
        <ArrowUpDown size={12} className="opacity-30" />
      )}
    </div>
  </th>
);

// Extended holding type with logo URL
interface HoldingWithLogo extends AggregatedHolding {
  logoUrl?: string;
}

export function AssetStatementView({ dbHoldings, dbPortfolios: _dbPortfolios }: AssetStatementViewProps) {
  const [baseCurrency, setBaseCurrency] = useState<string>('EUR');
  const [displayMode, setDisplayMode] = useState<DisplayMode>('table');
  const [sortField, setSortField] = useState<SortField>('currentValue');
  const [sortDir, setSortDir] = useState<SortDir>('desc');
  const [portfolioHistory, setPortfolioHistory] = useState<{ date: string; value: number }[]>([]);
  const [isLoadingChart, setIsLoadingChart] = useState(false);
  const [selectedHolding, setSelectedHolding] = useState<HoldingWithLogo | null>(null);
  const brandfetchApiKey = useSettingsStore((state) => state.brandfetchApiKey);

  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const valueSeriesRef = useRef<ISeriesApi<'Area'> | null>(null);
  const costBasisSeriesRef = useRef<ISeriesApi<'Line'> | null>(null);

  // Prepare securities list for logo loading
  const securitiesForLogos = useMemo(() =>
    dbHoldings.map((h) => ({
      id: h.securityId,
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

  // Fetch portfolio history for chart
  useEffect(() => {
    if (displayMode === 'chart') {
      setIsLoadingChart(true);
      getPortfolioHistory()
        .then((history) => {
          setPortfolioHistory(history || []);
        })
        .catch((err) => {
          console.error('Failed to load portfolio history:', err);
          setPortfolioHistory([]);
        })
        .finally(() => setIsLoadingChart(false));
    }
  }, [displayMode]);

  // Handle sort
  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir(sortDir === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDir('desc');
    }
  };

  // Calculate totals
  const totalValue = useMemo(() => {
    return dbHoldings.reduce((sum, h) => sum + (h.currentValue || 0), 0);
  }, [dbHoldings]);

  const totalCostBasis = useMemo(() => {
    return dbHoldings.reduce((sum, h) => sum + (h.costBasis || 0), 0);
  }, [dbHoldings]);

  const totalDividends = useMemo(() => {
    return dbHoldings.reduce((sum, h) => sum + (h.dividendsTotal || 0), 0);
  }, [dbHoldings]);

  const totalGainLoss = useMemo(() => {
    return dbHoldings.reduce((sum, h) => sum + (h.gainLoss || 0), 0);
  }, [dbHoldings]);

  // Prepare holdings data with logos
  const holdingsWithLogos = useMemo(() => {
    return dbHoldings.map((h) => ({
      ...h,
      logoUrl: h.customLogo || cachedLogos.get(h.securityId)?.url,
    }));
  }, [dbHoldings, cachedLogos]);

  // Sort holdings
  const sortedHoldings = useMemo(() => {
    const sorted = [...holdingsWithLogos].sort((a, b) => {
      let aVal: number | string | null = null;
      let bVal: number | string | null = null;

      switch (sortField) {
        case 'name':
          aVal = a.name;
          bVal = b.name;
          break;
        case 'shares':
          aVal = a.totalShares;
          bVal = b.totalShares;
          break;
        case 'purchasePrice':
          aVal = a.purchasePrice ?? 0;
          bVal = b.purchasePrice ?? 0;
          break;
        case 'costBasis':
          aVal = a.costBasis;
          bVal = b.costBasis;
          break;
        case 'currentValue':
          aVal = a.currentValue ?? 0;
          bVal = b.currentValue ?? 0;
          break;
        case 'gainLoss':
          aVal = a.gainLoss ?? 0;
          bVal = b.gainLoss ?? 0;
          break;
        case 'gainLossPercent':
          aVal = a.gainLossPercent ?? 0;
          bVal = b.gainLossPercent ?? 0;
          break;
        case 'dividends':
          aVal = a.dividendsTotal;
          bVal = b.dividendsTotal;
          break;
      }

      if (typeof aVal === 'string' && typeof bVal === 'string') {
        return sortDir === 'asc' ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
      }

      const numA = aVal as number;
      const numB = bVal as number;
      return sortDir === 'asc' ? numA - numB : numB - numA;
    });

    return sorted;
  }, [holdingsWithLogos, sortField, sortDir]);

  // Initialize chart
  useEffect(() => {
    if (displayMode !== 'chart' || !chartContainerRef.current || portfolioHistory.length === 0) {
      return;
    }

    // Clean up existing chart
    if (chartRef.current) {
      chartRef.current.remove();
      chartRef.current = null;
    }

    const container = chartContainerRef.current;

    // Create chart
    const chart = createChart(container, {
      layout: {
        background: { color: 'transparent' },
        textColor: '#888',
      },
      grid: {
        vertLines: { color: 'rgba(128, 128, 128, 0.1)' },
        horzLines: { color: 'rgba(128, 128, 128, 0.1)' },
      },
      width: container.clientWidth,
      height: container.clientHeight,
      rightPriceScale: {
        borderColor: 'rgba(128, 128, 128, 0.2)',
      },
      timeScale: {
        borderColor: 'rgba(128, 128, 128, 0.2)',
        timeVisible: true,
      },
      crosshair: {
        mode: 1,
      },
    });

    chartRef.current = chart;

    // Portfolio value area series (green gradient)
    const valueSeries = chart.addSeries(AreaSeries, {
      lineColor: '#22c55e',
      topColor: 'rgba(34, 197, 94, 0.4)',
      bottomColor: 'rgba(34, 197, 94, 0.0)',
      lineWidth: 2,
    });
    valueSeriesRef.current = valueSeries;

    // Cost basis line series (orange dashed line)
    const costBasisSeries = chart.addSeries(LineSeries, {
      color: '#f97316',
      lineWidth: 2,
      lineStyle: 2, // Dashed
    });
    costBasisSeriesRef.current = costBasisSeries;

    // Prepare data
    const valueData: AreaData<Time>[] = portfolioHistory.map((point) => ({
      time: point.date as Time,
      value: point.value,
    }));

    // Set value data
    valueSeries.setData(valueData);

    // Create cost basis line (flat line at current total cost basis)
    if (portfolioHistory.length > 0) {
      const costBasisData: LineData<Time>[] = [
        { time: portfolioHistory[0].date as Time, value: totalCostBasis },
        { time: portfolioHistory[portfolioHistory.length - 1].date as Time, value: totalCostBasis },
      ];
      costBasisSeries.setData(costBasisData);
    }

    // Fit content
    chart.timeScale().fitContent();

    // Handle resize
    const handleResize = () => {
      if (chartRef.current && container) {
        chartRef.current.applyOptions({
          width: container.clientWidth,
          height: container.clientHeight,
        });
      }
    };

    const resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
      if (chartRef.current) {
        chartRef.current.remove();
        chartRef.current = null;
      }
    };
  }, [displayMode, portfolioHistory, totalCostBasis]);

  if (dbHoldings.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center">
        <Table2 className="w-16 h-16 text-muted-foreground mb-4" />
        <h2 className="text-2xl font-semibold mb-2">Keine Bestände vorhanden</h2>
        <p className="text-muted-foreground">
          Importieren Sie eine .portfolio Datei, um die Vermögensaufstellung zu sehen.
        </p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col gap-4">
      {/* Header */}
      <div className="flex items-center justify-between flex-shrink-0">
        <div>
          <h1 className="text-2xl font-bold">Vermögensaufstellung</h1>
          <p className="text-muted-foreground">
            {dbHoldings.length} Positionen · Marktwert: {formatNumber(totalValue)} {baseCurrency} ·
            Einstandswert: {formatNumber(totalCostBasis)} {baseCurrency}
          </p>
        </div>

        {/* Display mode toggle */}
        <div className="flex border border-border rounded-md overflow-hidden">
          <button
            className={`px-3 py-1.5 text-sm flex items-center gap-1 ${
              displayMode === 'table' ? 'bg-primary text-primary-foreground' : 'bg-background hover:bg-accent'
            }`}
            onClick={() => setDisplayMode('table')}
          >
            <Table2 size={14} />
            Tabelle
          </button>
          <button
            className={`px-3 py-1.5 text-sm flex items-center gap-1 ${
              displayMode === 'chart' ? 'bg-primary text-primary-foreground' : 'bg-background hover:bg-accent'
            }`}
            onClick={() => setDisplayMode('chart')}
          >
            <LineChartIcon size={14} />
            Chart
          </button>
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-4 gap-3 flex-shrink-0">
        <div className="bg-card border border-border rounded-lg p-3">
          <div className="text-xs text-muted-foreground">Marktwert</div>
          <div className="text-lg font-bold">{formatNumber(totalValue)} {baseCurrency}</div>
        </div>
        <div className="bg-card border border-border rounded-lg p-3">
          <div className="text-xs text-muted-foreground">Einstandswert</div>
          <div className="text-lg font-bold">{formatNumber(totalCostBasis)} {baseCurrency}</div>
        </div>
        <div className="bg-card border border-border rounded-lg p-3">
          <div className="text-xs text-muted-foreground">Gewinn/Verlust</div>
          <div className={`text-lg font-bold ${totalGainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
            {totalGainLoss >= 0 ? '+' : ''}{formatNumber(totalGainLoss)} {baseCurrency}
            <span className="text-sm ml-1">
              ({totalCostBasis > 0 ? ((totalGainLoss / totalCostBasis) * 100).toFixed(2) : 0}%)
            </span>
          </div>
        </div>
        <div className="bg-card border border-border rounded-lg p-3">
          <div className="text-xs text-muted-foreground">Dividenden</div>
          <div className="text-lg font-bold text-blue-600">{formatNumber(totalDividends)} {baseCurrency}</div>
        </div>
      </div>

      {/* Main Content */}
      {displayMode === 'table' ? (
        /* Table View - PP Style Vermögensaufstellung */
        <div className="flex-1 min-h-0 bg-card rounded-lg border border-border overflow-hidden">
          <div className="overflow-auto h-full">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 sticky top-0">
                <tr>
                  <SortableHeader label="Wertpapier" field="name" currentSort={sortField} currentDir={sortDir} onSort={handleSort} align="left" />
                  <SortableHeader label="Anteile" field="shares" currentSort={sortField} currentDir={sortDir} onSort={handleSort} />
                  <SortableHeader label="Einstandskurs" field="purchasePrice" currentSort={sortField} currentDir={sortDir} onSort={handleSort} />
                  <SortableHeader label="Einstandswert" field="costBasis" currentSort={sortField} currentDir={sortDir} onSort={handleSort} />
                  <SortableHeader label="Marktwert" field="currentValue" currentSort={sortField} currentDir={sortDir} onSort={handleSort} />
                  <SortableHeader label="Gewinn/Verlust" field="gainLoss" currentSort={sortField} currentDir={sortDir} onSort={handleSort} />
                  <SortableHeader label="% Seit" field="gainLossPercent" currentSort={sortField} currentDir={sortDir} onSort={handleSort} />
                  <SortableHeader label="Div Seit" field="dividends" currentSort={sortField} currentDir={sortDir} onSort={handleSort} />
                </tr>
              </thead>
              <tbody>
                {sortedHoldings.map((holding) => {
                  const gainLossColor = (holding.gainLoss ?? 0) >= 0 ? 'text-green-600' : 'text-red-600';
                  return (
                    <tr
                      key={holding.securityId}
                      className="border-b border-border hover:bg-accent/50 transition-colors cursor-pointer"
                      onClick={() => setSelectedHolding(holding)}
                      title="Klicken für Detailansicht"
                    >
                      {/* Name with Logo */}
                      <td className="py-2 px-3">
                        <div className="flex items-center gap-2">
                          {holding.logoUrl ? (
                            <img
                              src={holding.logoUrl}
                              alt=""
                              className="w-6 h-6 rounded flex-shrink-0"
                              crossOrigin="anonymous"
                            />
                          ) : (
                            <Building2 size={20} className="text-muted-foreground flex-shrink-0" />
                          )}
                          <span className="font-medium truncate" title={holding.name}>
                            {holding.name}
                          </span>
                        </div>
                      </td>
                      {/* Shares */}
                      <td className="py-2 px-3 text-right tabular-nums">
                        {holding.totalShares.toLocaleString('de-DE', { maximumFractionDigits: 4 })}
                      </td>
                      {/* Purchase Price (Einstandskurs) */}
                      <td className="py-2 px-3 text-right tabular-nums">
                        {holding.purchasePrice != null ? formatNumber(holding.purchasePrice) : '-'}
                      </td>
                      {/* Cost Basis (Einstandswert) */}
                      <td className="py-2 px-3 text-right tabular-nums">
                        {formatNumber(holding.costBasis)}
                      </td>
                      {/* Current Value (Marktwert) */}
                      <td className="py-2 px-3 text-right tabular-nums font-medium">
                        {holding.currentValue != null ? formatNumber(holding.currentValue) : '-'}
                      </td>
                      {/* Gain/Loss */}
                      <td className={`py-2 px-3 text-right tabular-nums font-medium ${gainLossColor}`}>
                        {holding.gainLoss != null ? (
                          <>
                            {holding.gainLoss >= 0 ? '+' : ''}
                            {formatNumber(holding.gainLoss)}
                          </>
                        ) : '-'}
                      </td>
                      {/* Gain/Loss % */}
                      <td className={`py-2 px-3 text-right tabular-nums font-medium ${gainLossColor}`}>
                        {holding.gainLossPercent != null ? (
                          <>
                            {holding.gainLossPercent >= 0 ? '+' : ''}
                            {holding.gainLossPercent.toFixed(2)}%
                          </>
                        ) : '-'}
                      </td>
                      {/* Dividends */}
                      <td className="py-2 px-3 text-right tabular-nums text-blue-600">
                        {holding.dividendsTotal > 0 ? formatNumber(holding.dividendsTotal) : '-'}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
              {/* Footer with totals */}
              <tfoot className="bg-muted/50 font-medium border-t-2 border-border">
                <tr>
                  <td className="py-2 px-3">Summe</td>
                  <td className="py-2 px-3 text-right">-</td>
                  <td className="py-2 px-3 text-right">-</td>
                  <td className="py-2 px-3 text-right tabular-nums">{formatNumber(totalCostBasis)}</td>
                  <td className="py-2 px-3 text-right tabular-nums">{formatNumber(totalValue)}</td>
                  <td className={`py-2 px-3 text-right tabular-nums ${totalGainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {totalGainLoss >= 0 ? '+' : ''}{formatNumber(totalGainLoss)}
                  </td>
                  <td className={`py-2 px-3 text-right tabular-nums ${totalGainLoss >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {totalCostBasis > 0 ? (
                      <>
                        {((totalGainLoss / totalCostBasis) * 100) >= 0 ? '+' : ''}
                        {((totalGainLoss / totalCostBasis) * 100).toFixed(2)}%
                      </>
                    ) : '-'}
                  </td>
                  <td className="py-2 px-3 text-right tabular-nums text-blue-600">
                    {totalDividends > 0 ? formatNumber(totalDividends) : '-'}
                  </td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>
      ) : (
        /* Chart View - Portfolio Value vs Cost Basis */
        <div className="flex-1 min-h-0 bg-card rounded-lg border border-border overflow-hidden">
          {isLoadingChart ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-muted-foreground">Lade Chart-Daten...</div>
            </div>
          ) : portfolioHistory.length === 0 ? (
            <div className="h-full flex items-center justify-center">
              <div className="text-center">
                <LineChartIcon className="w-12 h-12 text-muted-foreground mx-auto mb-2" />
                <p className="text-muted-foreground">Keine historischen Daten verfügbar.</p>
                <p className="text-xs text-muted-foreground mt-1">
                  Aktualisieren Sie die Kurse, um den Verlauf zu sehen.
                </p>
              </div>
            </div>
          ) : (
            <div className="h-full p-4">
              {/* Legend */}
              <div className="flex items-center gap-6 mb-4">
                <div className="flex items-center gap-2">
                  <div className="w-4 h-0.5 bg-green-500"></div>
                  <span className="text-sm text-muted-foreground">Marktwert</span>
                </div>
                <div className="flex items-center gap-2">
                  <div className="w-4 h-0.5 bg-orange-500" style={{ borderStyle: 'dashed', borderWidth: '1px', borderColor: '#f97316' }}></div>
                  <span className="text-sm text-muted-foreground">Einstandswert ({formatNumber(totalCostBasis)} {baseCurrency})</span>
                </div>
              </div>
              {/* Chart container */}
              <div ref={chartContainerRef} className="h-[calc(100%-40px)]" />
            </div>
          )}
        </div>
      )}

      {/* Security Detail Chart Modal */}
      {selectedHolding && (
        <SecurityDetailChartModal
          isOpen={!!selectedHolding}
          onClose={() => setSelectedHolding(null)}
          securityId={selectedHolding.securityId}
          securityName={selectedHolding.name}
          isin={selectedHolding.isin}
          currency={selectedHolding.currency}
          customLogo={selectedHolding.customLogo}
        />
      )}
    </div>
  );
}
