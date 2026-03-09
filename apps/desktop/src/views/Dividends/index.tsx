/**
 * Dividends view - comprehensive overview of dividend income.
 * Redesigned with DividendStatsBar, YearChart, Allocation, Matrix, Timeline.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import { Coins, RefreshCw, AlertCircle, LayoutGrid, CalendarDays, LineChart } from 'lucide-react';
import {
  generateDividendReport,
  getSecurities,
  fetchLogosBatch,
  getCachedLogoData,
  getPortfolioDividendYield,
  estimateAnnualDividends,
  syncExDividends,
  getAllHoldings,
} from '../../lib/api';
import type { DividendReport, SecurityData } from '../../lib/types';
import type { DividendForecast } from '../../lib/api';
import { useSettingsStore, toast } from '../../store';
import type { CachedLogo } from '../../lib/hooks';
import { DividendCalendar } from './DividendCalendar';
import { DividendForecast as DividendForecastView } from './DividendForecast';
import { DividendStatsBar } from './DividendStatsBar';
import { DividendYearChart } from './DividendYearChart';
import { DividendAllocation } from './DividendAllocation';
import { DividendMatrix } from './DividendMatrix';
import { DividendTimeline } from './DividendTimeline';

type TabType = 'overview' | 'calendar' | 'forecast';

// Load reports for a range of years
const HISTORY_YEARS = 6;

export function DividendsView() {
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [yearReports, setYearReports] = useState<Map<number, DividendReport>>(new Map());
  const [selectedYear, setSelectedYear] = useState<number>(new Date().getFullYear());
  const [logos, setLogos] = useState<Map<number, CachedLogo>>(new Map());
  const [activeTab, setActiveTab] = useState<TabType>('overview');
  const [dividendYield, setDividendYield] = useState<number | null>(null);
  const [fwdForecast, setFwdForecast] = useState<DividendForecast | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);
  const [heldSecurityIds, setHeldSecurityIds] = useState<Set<number>>(new Set());

  const { brandfetchApiKey } = useSettingsStore();

  const loadData = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const currentYear = new Date().getFullYear();
      const startYear = currentYear - HISTORY_YEARS + 1;

      // Build year range
      const yearRange = Array.from({ length: HISTORY_YEARS }, (_, i) => startYear + i);

      // Load all year reports + yield + forecast + securities + holdings in parallel
      const [yieldResult, forecastResult, allSecurities, holdings, ...reports] = await Promise.all([
        getPortfolioDividendYield().catch(() => null),
        estimateAnnualDividends(currentYear).catch(() => null),
        getSecurities(),
        getAllHoldings().catch(() => []),
        ...yearRange.map((y) =>
          generateDividendReport(`${y}-01-01`, `${y}-12-31`, undefined).catch(() => null)
        ),
      ]);

      setDividendYield(yieldResult);
      setFwdForecast(forecastResult);

      // Build set of security IDs that are currently held
      const heldIds = new Set<number>();
      for (const h of holdings) {
        if (h.totalShares > 0) {
          for (const id of h.securityIds) {
            heldIds.add(id);
          }
        }
      }
      setHeldSecurityIds(heldIds);

      // Build year reports map
      const reportsMap = new Map<number, DividendReport>();
      yearRange.forEach((y, idx) => {
        const report = reports[idx];
        if (report && (report.entries.length > 0 || report.totalNet > 0)) {
          reportsMap.set(y, report);
        }
      });
      setYearReports(reportsMap);

      // Collect all security IDs for logos
      const securityIds = new Set<number>();
      for (const report of reportsMap.values()) {
        report.bySecurity.forEach((s) => securityIds.add(s.securityId));
        report.entries.forEach((e) => securityIds.add(e.securityId));
      }

      // Fetch logos
      if (securityIds.size > 0) {
        const secMap = new Map<number, SecurityData>();
        allSecurities.forEach((s) => secMap.set(s.id, s));

        const toFetch = Array.from(securityIds)
          .map((id) => secMap.get(id))
          .filter((s): s is SecurityData => !!s)
          .map((s) => ({ id: s.id, ticker: s.ticker || undefined, name: s.name || '' }));

        if (toFetch.length > 0) {
          const results = await fetchLogosBatch(brandfetchApiKey || '', toFetch);
          const newLogos = new Map<number, CachedLogo>();
          for (const result of results) {
            if (result.domain) {
              const cachedData = await getCachedLogoData(result.domain);
              if (cachedData) {
                newLogos.set(result.securityId, { url: cachedData, isLocal: true, domain: result.domain });
              } else if (result.logoUrl) {
                newLogos.set(result.securityId, { url: result.logoUrl, isLocal: false, domain: result.domain });
              }
            }
          }
          setLogos(newLogos);
        }
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  }, [brandfetchApiKey]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Current year report
  const currentReport = useMemo(() => yearReports.get(selectedYear) || null, [yearReports, selectedYear]);
  const currency = currentReport?.currency || fwdForecast?.currency || 'EUR';

  // Filter reports to only include currently held securities (for Matrix, Allocation, Timeline)
  const filterReport = useCallback((report: DividendReport): DividendReport => {
    if (heldSecurityIds.size === 0) return report;
    const filteredBySecurity = report.bySecurity.filter((s) => heldSecurityIds.has(s.securityId));
    const filteredEntries = report.entries.filter((e) => heldSecurityIds.has(e.securityId));
    const totalNet = filteredBySecurity.reduce((sum, s) => sum + s.totalNet, 0);
    const totalGross = filteredBySecurity.reduce((sum, s) => sum + s.totalGross, 0);
    const totalTaxes = filteredBySecurity.reduce((sum, s) => sum + s.totalTaxes, 0);
    return { ...report, bySecurity: filteredBySecurity, entries: filteredEntries, totalNet, totalGross, totalTaxes };
  }, [heldSecurityIds]);

  const filteredYearReports = useMemo(() => {
    const filtered = new Map<number, DividendReport>();
    for (const [year, report] of yearReports) {
      const fr = filterReport(report);
      if (fr.bySecurity.length > 0) {
        filtered.set(year, fr);
      }
    }
    return filtered;
  }, [yearReports, filterReport]);

  const filteredCurrentReport = useMemo(
    () => (currentReport ? filterReport(currentReport) : null),
    [currentReport, filterReport]
  );

  // YTD net
  const ytdNet = useMemo(() => {
    const thisYearReport = yearReports.get(new Date().getFullYear());
    return thisYearReport?.totalNet || 0;
  }, [yearReports]);

  // FWD annual
  const fwdAnnual = useMemo(() => {
    if (!fwdForecast) return 0;
    return fwdForecast.totalEstimated + fwdForecast.totalReceived;
  }, [fwdForecast]);

  const handleSyncDividends = async () => {
    setIsSyncing(true);
    try {
      const result = await syncExDividends();
      if (result.newDividends > 0 || result.updatedDividends > 0) {
        toast.success(`${result.newDividends} neue, ${result.updatedDividends} aktualisierte Dividendentermine geladen`);
      } else {
        toast.info('Keine neuen Dividendentermine gefunden');
      }
      if (result.errors.length > 0) {
        toast.warning(`${result.failed} von ${result.totalSecurities} Wertpapieren fehlgeschlagen`);
      }
    } catch (err) {
      toast.error('Fehler beim Synchronisieren der Dividendentermine');
      console.error(err);
    } finally {
      setIsSyncing(false);
    }
  };

  const years = useMemo(() => {
    const currentYear = new Date().getFullYear();
    return Array.from({ length: 10 }, (_, i) => currentYear - i);
  }, []);

  const tabs: { id: TabType; label: string; icon: typeof LayoutGrid }[] = [
    { id: 'overview', label: 'Ubersicht', icon: LayoutGrid },
    { id: 'calendar', label: 'Kalender', icon: CalendarDays },
    { id: 'forecast', label: 'Prognose', icon: LineChart },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-emerald-100 dark:bg-emerald-900/30 rounded-lg">
            <Coins className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
          </div>
          <h1 className="text-2xl font-bold">Dividenden</h1>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={handleSyncDividends}
            disabled={isSyncing}
            className="flex items-center gap-2 px-3 py-2 border border-border rounded-lg text-sm hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            title="Dividendentermine von DivvyDiary laden"
          >
            <RefreshCw size={15} className={isSyncing ? 'animate-spin' : ''} />
            {isSyncing ? 'Synchronisiere...' : 'Termine laden'}
          </button>
          <select
            value={selectedYear}
            onChange={(e) => setSelectedYear(Number(e.target.value))}
            className="px-3 py-2 border border-border rounded-lg bg-background text-sm"
          >
            {years.map((y) => (
              <option key={y} value={y}>{y}</option>
            ))}
          </select>
          {activeTab === 'overview' && (
            <button
              onClick={loadData}
              disabled={isLoading}
              className="flex items-center gap-2 px-3 py-2 border border-border rounded-lg hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed text-sm"
            >
              <RefreshCw size={15} className={isLoading ? 'animate-spin' : ''} />
              Aktualisieren
            </button>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 p-1 bg-muted rounded-lg w-fit">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                activeTab === tab.id
                  ? 'bg-background text-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              <Icon size={16} />
              {tab.label}
            </button>
          );
        })}
      </div>

      {/* Error */}
      {error && activeTab === 'overview' && (
        <div className="flex items-center gap-2 p-3 bg-destructive/10 border border-destructive/20 rounded-lg text-destructive text-sm">
          <AlertCircle size={16} />
          {error}
        </div>
      )}

      {/* Calendar Tab */}
      {activeTab === 'calendar' && (
        <DividendCalendar selectedYear={selectedYear} onYearChange={setSelectedYear} />
      )}

      {/* Forecast Tab */}
      {activeTab === 'forecast' && (
        <DividendForecastView selectedYear={selectedYear} />
      )}

      {/* Overview Tab */}
      {activeTab === 'overview' && (
        <>
          {/* Stats Bar */}
          <DividendStatsBar
            dividendYield={dividendYield}
            ytdNet={ytdNet}
            fwdAnnual={fwdAnnual}
            currency={currency}
            isLoading={isLoading}
          />

          {/* Year Chart + Allocation */}
          <div className="grid grid-cols-1 lg:grid-cols-5 gap-4">
            <div className="lg:col-span-3">
              <DividendYearChart
                yearReports={yearReports}
                selectedYear={selectedYear}
                currency={currency}
                isLoading={isLoading}
              />
            </div>
            <div className="lg:col-span-2">
              <DividendAllocation
                bySecurity={filteredCurrentReport?.bySecurity || []}
                totalNet={filteredCurrentReport?.totalNet || 0}
                currency={currency}
                logos={logos}
                isLoading={isLoading}
              />
            </div>
          </div>

          {/* Matrix */}
          <DividendMatrix
            yearReports={filteredYearReports}
            logos={logos}
            currency={currency}
            isLoading={isLoading}
          />

          {/* Timeline */}
          <DividendTimeline
            report={filteredCurrentReport}
            selectedYear={selectedYear}
            logos={logos}
            currency={currency}
            isLoading={isLoading}
          />

          {/* Empty State */}
          {!isLoading && yearReports.size === 0 && (
            <div className="bg-card rounded-xl border border-border p-8 text-center text-muted-foreground">
              <Coins className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Keine Dividenden gefunden.</p>
            </div>
          )}
        </>
      )}
    </div>
  );
}
