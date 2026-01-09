/**
 * Reports view for performance analysis.
 */

import { useState, useEffect } from 'react';
import { BarChart3, RefreshCw, TrendingUp, Coins, FileText, PieChart } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend, PieChart as RechartsPieChart, Pie, Cell } from 'recharts';
import {
  generateDividendReport,
  generateRealizedGainsReport,
  generateTaxReport,
  calculatePerformance,
  getPortfolios,
} from '../../lib/api';
import type {
  DividendReport,
  RealizedGainsReport,
  TaxReport,
  PerformanceResult,
  PortfolioData,
} from '../../lib/types';

type ReportType = 'performance' | 'dividends' | 'gains' | 'tax';

const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

export function ReportsView() {
  const [reportType, setReportType] = useState<ReportType>('performance');
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [selectedPortfolio, setSelectedPortfolio] = useState<number | undefined>(undefined);
  const [year, setYear] = useState<number>(new Date().getFullYear());
  const [startDate, setStartDate] = useState<string>(() => {
    const d = new Date();
    d.setFullYear(d.getFullYear() - 1);
    return d.toISOString().split('T')[0];
  });
  const [endDate, setEndDate] = useState<string>(() => new Date().toISOString().split('T')[0]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Report data
  const [performanceData, setPerformanceData] = useState<PerformanceResult | null>(null);
  const [dividendData, setDividendData] = useState<DividendReport | null>(null);
  const [gainsData, setGainsData] = useState<RealizedGainsReport | null>(null);
  const [taxData, setTaxData] = useState<TaxReport | null>(null);

  useEffect(() => {
    loadPortfolios();
  }, []);

  const loadPortfolios = async () => {
    try {
      const data = await getPortfolios();
      setPortfolios(data.filter(p => !p.isRetired));
    } catch (err) {
      console.error('Failed to load portfolios:', err);
    }
  };

  const loadReport = async () => {
    setIsLoading(true);
    setError(null);

    try {
      switch (reportType) {
        case 'performance':
          const perf = await calculatePerformance({
            portfolioId: selectedPortfolio,
            startDate,
            endDate,
          });
          setPerformanceData(perf);
          break;

        case 'dividends':
          const divs = await generateDividendReport(startDate, endDate, selectedPortfolio);
          setDividendData(divs);
          break;

        case 'gains':
          const gains = await generateRealizedGainsReport(startDate, endDate, selectedPortfolio);
          setGainsData(gains);
          break;

        case 'tax':
          const tax = await generateTaxReport(year);
          setTaxData(tax);
          break;
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const formatCurrency = (amount: number, currency: string = 'EUR') => {
    return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
  };

  const formatPercent = (value: number) => {
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString('de-DE');
  };

  const renderPerformanceReport = () => {
    if (!performanceData) return null;

    return (
      <div className="space-y-6">
        {/* Summary Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">TTWROR</div>
            <div className={`text-2xl font-bold ${performanceData.ttwror >= 0 ? 'text-green-600' : 'text-red-600'}`}>
              {formatPercent(performanceData.ttwror)}
            </div>
            <div className="text-xs text-muted-foreground mt-1">
              Annualisiert: {formatPercent(performanceData.ttwrorAnnualized)}
            </div>
          </div>

          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">IRR</div>
            <div className={`text-2xl font-bold ${performanceData.irr >= 0 ? 'text-green-600' : 'text-red-600'}`}>
              {formatPercent(performanceData.irr)}
            </div>
            <div className="text-xs text-muted-foreground mt-1">
              {performanceData.irrConverged ? 'Konvergiert' : 'Approximiert'}
            </div>
          </div>

          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Aktueller Wert</div>
            <div className="text-2xl font-bold">
              {formatCurrency(performanceData.currentValue)}
            </div>
            <div className="text-xs text-muted-foreground mt-1">
              Investiert: {formatCurrency(performanceData.totalInvested)}
            </div>
          </div>

          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Absoluter Gewinn</div>
            <div className={`text-2xl font-bold ${performanceData.absoluteGain >= 0 ? 'text-green-600' : 'text-red-600'}`}>
              {formatCurrency(performanceData.absoluteGain)}
            </div>
            <div className="text-xs text-muted-foreground mt-1">
              {performanceData.days} Tage
            </div>
          </div>
        </div>

        {/* Period Info */}
        <div className="bg-muted/50 rounded-lg p-4">
          <div className="text-sm text-muted-foreground">
            Zeitraum: {formatDate(performanceData.startDate)} - {formatDate(performanceData.endDate)}
          </div>
        </div>
      </div>
    );
  };

  const renderDividendReport = () => {
    if (!dividendData) return null;

    const monthlyChartData = dividendData.byMonth.map(m => ({
      month: m.month,
      gross: m.totalGross / 100,
      taxes: m.totalTaxes / 100,
      net: m.totalNet / 100,
    }));

    const securityPieData = dividendData.bySecurity.slice(0, 8).map(s => ({
      name: s.securityName,
      value: s.totalNet / 100,
    }));

    return (
      <div className="space-y-6">
        {/* Summary Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Brutto-Dividenden</div>
            <div className="text-2xl font-bold text-green-600">
              {formatCurrency(dividendData.totalGross / 100, dividendData.currency)}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Quellensteuer</div>
            <div className="text-2xl font-bold text-red-600">
              -{formatCurrency(dividendData.totalTaxes / 100, dividendData.currency)}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Netto-Dividenden</div>
            <div className="text-2xl font-bold">
              {formatCurrency(dividendData.totalNet / 100, dividendData.currency)}
            </div>
          </div>
        </div>

        {/* Monthly Chart */}
        {monthlyChartData.length > 0 && (
          <div className="bg-card rounded-lg border border-border p-4">
            <h3 className="font-semibold mb-4">Dividenden pro Monat</h3>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={monthlyChartData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                  <XAxis dataKey="month" tick={{ fontSize: 12 }} />
                  <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => `${v.toFixed(0)}€`} />
                  <Tooltip formatter={(value) => [typeof value === 'number' ? `${value.toFixed(2)}€` : '-']} />
                  <Legend />
                  <Bar dataKey="gross" name="Brutto" fill={COLORS[0]} />
                  <Bar dataKey="net" name="Netto" fill={COLORS[1]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>
        )}

        {/* By Security Pie Chart */}
        {securityPieData.length > 0 && (
          <div className="bg-card rounded-lg border border-border p-4">
            <h3 className="font-semibold mb-4">Dividenden nach Wertpapier</h3>
            <div className="h-64">
              <ResponsiveContainer width="100%" height="100%">
                <RechartsPieChart>
                  <Pie
                    data={securityPieData}
                    dataKey="value"
                    nameKey="name"
                    cx="50%"
                    cy="50%"
                    outerRadius={80}
                    label={({ name, percent }) => `${name} (${((percent ?? 0) * 100).toFixed(0)}%)`}
                  >
                    {securityPieData.map((_, idx) => (
                      <Cell key={`cell-${idx}`} fill={COLORS[idx % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => [typeof value === 'number' ? `${value.toFixed(2)}€` : '-']} />
                </RechartsPieChart>
              </ResponsiveContainer>
            </div>
          </div>
        )}

        {/* Entries Table */}
        {dividendData.entries.length > 0 && (
          <div className="bg-card rounded-lg border border-border">
            <div className="p-4 border-b border-border">
              <h3 className="font-semibold">Dividendenzahlungen ({dividendData.entries.length})</h3>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border bg-muted/50">
                    <th className="text-left py-3 px-4 font-medium">Datum</th>
                    <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
                    <th className="text-right py-3 px-4 font-medium">Brutto</th>
                    <th className="text-right py-3 px-4 font-medium">Steuer</th>
                    <th className="text-right py-3 px-4 font-medium">Netto</th>
                  </tr>
                </thead>
                <tbody>
                  {dividendData.entries.slice(0, 20).map((entry, idx) => (
                    <tr key={idx} className="border-b border-border last:border-0 hover:bg-muted/30">
                      <td className="py-3 px-4">{formatDate(entry.date)}</td>
                      <td className="py-3 px-4 font-medium">{entry.securityName}</td>
                      <td className="py-3 px-4 text-right text-green-600">
                        {formatCurrency(entry.grossAmount / 100, entry.currency)}
                      </td>
                      <td className="py-3 px-4 text-right text-red-600">
                        -{formatCurrency(entry.taxes / 100, entry.currency)}
                      </td>
                      <td className="py-3 px-4 text-right font-medium">
                        {formatCurrency(entry.netAmount / 100, entry.currency)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    );
  };

  const renderGainsReport = () => {
    if (!gainsData) return null;

    const gainLoss = gainsData.totalGain >= 0;

    return (
      <div className="space-y-6">
        {/* Summary Cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Veräußerungserlöse</div>
            <div className="text-2xl font-bold">
              {formatCurrency(gainsData.totalProceeds / 100, gainsData.currency)}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Anschaffungskosten</div>
            <div className="text-2xl font-bold">
              {formatCurrency(gainsData.totalCostBasis / 100, gainsData.currency)}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Realisierter Gewinn/Verlust</div>
            <div className={`text-2xl font-bold ${gainLoss ? 'text-green-600' : 'text-red-600'}`}>
              {formatCurrency(gainsData.totalGain / 100, gainsData.currency)}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Gebühren & Steuern</div>
            <div className="text-2xl font-bold text-red-600">
              -{formatCurrency((gainsData.totalFees + gainsData.totalTaxes) / 100, gainsData.currency)}
            </div>
          </div>
        </div>

        {/* Short vs Long Term */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Kurzfristige Gewinne (&lt;1 Jahr)</div>
            <div className={`text-xl font-bold ${gainsData.shortTermGain >= 0 ? 'text-green-600' : 'text-red-600'}`}>
              {formatCurrency(gainsData.shortTermGain / 100, gainsData.currency)}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="text-sm text-muted-foreground">Langfristige Gewinne (&gt;1 Jahr)</div>
            <div className={`text-xl font-bold ${gainsData.longTermGain >= 0 ? 'text-green-600' : 'text-red-600'}`}>
              {formatCurrency(gainsData.longTermGain / 100, gainsData.currency)}
            </div>
          </div>
        </div>

        {/* By Security */}
        {gainsData.bySecurity.length > 0 && (
          <div className="bg-card rounded-lg border border-border">
            <div className="p-4 border-b border-border">
              <h3 className="font-semibold">Gewinne nach Wertpapier</h3>
            </div>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border bg-muted/50">
                    <th className="text-left py-3 px-4 font-medium">Wertpapier</th>
                    <th className="text-right py-3 px-4 font-medium">Verkäufe</th>
                    <th className="text-right py-3 px-4 font-medium">Erlös</th>
                    <th className="text-right py-3 px-4 font-medium">Kosten</th>
                    <th className="text-right py-3 px-4 font-medium">Gewinn</th>
                  </tr>
                </thead>
                <tbody>
                  {gainsData.bySecurity.map((item, idx) => (
                    <tr key={idx} className="border-b border-border last:border-0 hover:bg-muted/30">
                      <td className="py-3 px-4">
                        <div className="font-medium">{item.securityName}</div>
                        {item.securityIsin && (
                          <div className="text-xs text-muted-foreground">{item.securityIsin}</div>
                        )}
                      </td>
                      <td className="py-3 px-4 text-right">{item.saleCount}</td>
                      <td className="py-3 px-4 text-right">
                        {formatCurrency(item.totalProceeds / 100)}
                      </td>
                      <td className="py-3 px-4 text-right">
                        {formatCurrency(item.totalCostBasis / 100)}
                      </td>
                      <td className={`py-3 px-4 text-right font-medium ${item.totalGain >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                        {formatCurrency(item.totalGain / 100)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    );
  };

  const renderTaxReport = () => {
    if (!taxData) return null;

    return (
      <div className="space-y-6">
        {/* Summary */}
        <div className="bg-card rounded-lg border border-border p-6">
          <h3 className="font-semibold text-lg mb-4">Steuerbericht {taxData.year}</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <h4 className="font-medium text-muted-foreground mb-2">Dividendeneinkünfte</h4>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <span>Brutto-Dividenden:</span>
                  <span className="font-medium">{formatCurrency(taxData.dividendIncome / 100, taxData.currency)}</span>
                </div>
                <div className="flex justify-between">
                  <span>Einbehaltene Quellensteuer:</span>
                  <span className="font-medium text-red-600">-{formatCurrency(taxData.dividendTaxesWithheld / 100, taxData.currency)}</span>
                </div>
              </div>
            </div>
            <div>
              <h4 className="font-medium text-muted-foreground mb-2">Kapitalerträge</h4>
              <div className="space-y-2">
                <div className="flex justify-between">
                  <span>Kurzfristige Gewinne:</span>
                  <span className={`font-medium ${taxData.shortTermGains >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {formatCurrency(taxData.shortTermGains / 100, taxData.currency)}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span>Langfristige Gewinne:</span>
                  <span className={`font-medium ${taxData.longTermGains >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {formatCurrency(taxData.longTermGains / 100, taxData.currency)}
                  </span>
                </div>
                <div className="flex justify-between border-t border-border pt-2">
                  <span className="font-medium">Gesamte Kapitalerträge:</span>
                  <span className={`font-bold ${taxData.totalCapitalGains >= 0 ? 'text-green-600' : 'text-red-600'}`}>
                    {formatCurrency(taxData.totalCapitalGains / 100, taxData.currency)}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Detailed Reports */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="bg-card rounded-lg border border-border p-4">
            <h4 className="font-medium mb-2">Gebühren</h4>
            <div className="text-2xl font-bold text-red-600">
              -{formatCurrency(taxData.totalFees / 100, taxData.currency)}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-4">
            <h4 className="font-medium mb-2">Gezahlte Kapitalertragssteuer</h4>
            <div className="text-2xl font-bold text-red-600">
              -{formatCurrency(taxData.capitalGainsTaxes / 100, taxData.currency)}
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <BarChart3 className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Berichte</h1>
        </div>
        <button
          onClick={loadReport}
          disabled={isLoading}
          className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
        >
          <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
          Bericht generieren
        </button>
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          {error}
        </div>
      )}

      {/* Report Type Selection */}
      <div className="bg-card rounded-lg border border-border p-4">
        <div className="flex flex-wrap gap-4">
          <button
            onClick={() => setReportType('performance')}
            className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
              reportType === 'performance'
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted hover:bg-muted/80'
            }`}
          >
            <TrendingUp size={18} />
            Performance
          </button>
          <button
            onClick={() => setReportType('dividends')}
            className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
              reportType === 'dividends'
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted hover:bg-muted/80'
            }`}
          >
            <Coins size={18} />
            Dividenden
          </button>
          <button
            onClick={() => setReportType('gains')}
            className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
              reportType === 'gains'
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted hover:bg-muted/80'
            }`}
          >
            <PieChart size={18} />
            Realisierte Gewinne
          </button>
          <button
            onClick={() => setReportType('tax')}
            className={`flex items-center gap-2 px-4 py-2 rounded-md transition-colors ${
              reportType === 'tax'
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted hover:bg-muted/80'
            }`}
          >
            <FileText size={18} />
            Steuerbericht
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="bg-card rounded-lg border border-border p-4">
        <div className="flex flex-wrap items-end gap-4">
          {reportType !== 'tax' && (
            <>
              <div>
                <label className="block text-sm font-medium mb-1">Portfolio</label>
                <select
                  value={selectedPortfolio || ''}
                  onChange={(e) => setSelectedPortfolio(e.target.value ? Number(e.target.value) : undefined)}
                  className="px-3 py-2 border border-border rounded-md bg-background min-w-[200px]"
                >
                  <option value="">Alle Portfolios</option>
                  {portfolios.map(p => (
                    <option key={p.id} value={p.id}>{p.name}</option>
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
            </>
          )}
          {reportType === 'tax' && (
            <div>
              <label className="block text-sm font-medium mb-1">Steuerjahr</label>
              <select
                value={year}
                onChange={(e) => setYear(Number(e.target.value))}
                className="px-3 py-2 border border-border rounded-md bg-background"
              >
                {Array.from({ length: 10 }, (_, i) => new Date().getFullYear() - i).map(y => (
                  <option key={y} value={y}>{y}</option>
                ))}
              </select>
            </div>
          )}
        </div>
      </div>

      {/* Report Content */}
      {reportType === 'performance' && performanceData && renderPerformanceReport()}
      {reportType === 'dividends' && dividendData && renderDividendReport()}
      {reportType === 'gains' && gainsData && renderGainsReport()}
      {reportType === 'tax' && taxData && renderTaxReport()}

      {/* Empty State */}
      {!isLoading && !performanceData && !dividendData && !gainsData && !taxData && (
        <div className="bg-card rounded-lg border border-border p-8 text-center text-muted-foreground">
          <BarChart3 className="w-12 h-12 mx-auto mb-3 opacity-50" />
          <p>Wählen Sie einen Berichtstyp und klicken Sie auf "Bericht generieren".</p>
        </div>
      )}
    </div>
  );
}
