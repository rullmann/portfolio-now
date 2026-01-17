/**
 * German Tax Report (Steuerbericht DE)
 *
 * Comprehensive tax calculation for German residents:
 * - Abgeltungssteuer (25% + Soli)
 * - Kirchensteuer (optional)
 * - Freistellungsauftrag tracking
 * - Anlage KAP data export
 */

import { useState, useEffect } from 'react';
import {
  FileText,
  RefreshCw,
  AlertCircle,
  Settings,
  Info,
} from 'lucide-react';
import {
  generateGermanTaxReport,
  getTaxSettings,
  saveTaxSettings,
  type GermanTaxReport,
  type TaxSettings,
} from '../../lib/api';
import { formatDate } from '../../lib/types';

interface Props {
  year: number;
}

const BUNDESLAENDER = [
  { value: '', label: 'Keine Kirchensteuer' },
  { value: 'bayern', label: 'Bayern (8%)' },
  { value: 'baden-wuerttemberg', label: 'Baden-Württemberg (8%)' },
  { value: 'berlin', label: 'Berlin (9%)' },
  { value: 'brandenburg', label: 'Brandenburg (9%)' },
  { value: 'bremen', label: 'Bremen (9%)' },
  { value: 'hamburg', label: 'Hamburg (9%)' },
  { value: 'hessen', label: 'Hessen (9%)' },
  { value: 'mecklenburg-vorpommern', label: 'Mecklenburg-Vorpommern (9%)' },
  { value: 'niedersachsen', label: 'Niedersachsen (9%)' },
  { value: 'nordrhein-westfalen', label: 'Nordrhein-Westfalen (9%)' },
  { value: 'rheinland-pfalz', label: 'Rheinland-Pfalz (9%)' },
  { value: 'saarland', label: 'Saarland (9%)' },
  { value: 'sachsen', label: 'Sachsen (9%)' },
  { value: 'sachsen-anhalt', label: 'Sachsen-Anhalt (9%)' },
  { value: 'schleswig-holstein', label: 'Schleswig-Holstein (9%)' },
  { value: 'thueringen', label: 'Thüringen (9%)' },
];

export function GermanTaxReportView({ year }: Props) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<GermanTaxReport | null>(null);
  const [settings, setSettings] = useState<TaxSettings | null>(null);
  const [showSettings, setShowSettings] = useState(false);

  // Load data
  useEffect(() => {
    loadData();
  }, [year]);

  const loadData = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [reportData, settingsData] = await Promise.all([
        generateGermanTaxReport(year),
        getTaxSettings(year),
      ]);
      setReport(reportData);
      setSettings(settingsData);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleSaveSettings = async () => {
    if (!settings) return;
    try {
      await saveTaxSettings(settings);
      setShowSettings(false);
      loadData(); // Reload with new settings
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleBundeslandChange = (bundesland: string) => {
    if (!settings) return;
    let rate: number | undefined = undefined;
    if (bundesland === 'bayern' || bundesland === 'baden-wuerttemberg') {
      rate = 0.08;
    } else if (bundesland) {
      rate = 0.09;
    }
    setSettings({
      ...settings,
      bundesland: bundesland || undefined,
      kirchensteuerRate: rate,
    });
  };

  const formatCurrency = (amount: number, currency: string = 'EUR') => {
    return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 })} ${currency}`;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        <RefreshCw className="w-6 h-6 animate-spin mr-2" />
        Lade Steuerbericht...
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 p-4 bg-destructive/10 border border-destructive/20 rounded-md text-destructive">
        <AlertCircle size={20} />
        {error}
      </div>
    );
  }

  if (!report || !settings) return null;

  const freistellungPercent = settings.freistellungLimit > 0
    ? (report.freistellungUsed / settings.freistellungLimit) * 100
    : 0;

  return (
    <div className="space-y-6">
      {/* Header with Settings Button */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold flex items-center gap-2">
          <FileText size={20} />
          Steuerbericht {year} (Deutschland)
        </h2>
        <button
          onClick={() => setShowSettings(!showSettings)}
          className="flex items-center gap-2 px-3 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
        >
          <Settings size={16} />
          Einstellungen
        </button>
      </div>

      {/* Settings Panel */}
      {showSettings && (
        <div className="bg-card rounded-lg border border-border p-4 space-y-4">
          <h3 className="font-medium">Steuereinstellungen {year}</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-muted-foreground mb-1">Familienstand</label>
              <select
                value={settings.isMarried ? 'married' : 'single'}
                onChange={(e) => setSettings({ ...settings, isMarried: e.target.value === 'married' })}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              >
                <option value="single">Ledig / Einzelveranlagung</option>
                <option value="married">Verheiratet / Zusammenveranlagung</option>
              </select>
            </div>
            <div>
              <label className="block text-sm text-muted-foreground mb-1">Kirchensteuer</label>
              <select
                value={settings.bundesland || ''}
                onChange={(e) => handleBundeslandChange(e.target.value)}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              >
                {BUNDESLAENDER.map((b) => (
                  <option key={b.value} value={b.value}>{b.label}</option>
                ))}
              </select>
            </div>
          </div>
          <div className="flex items-center gap-2 p-3 bg-muted/50 rounded-md text-sm">
            <Info size={16} className="text-muted-foreground flex-shrink-0" />
            <span>
              Freistellungsauftrag {year}: {formatCurrency(settings.freistellungLimit)}
              {settings.isMarried ? ' (Zusammenveranlagung)' : ' (Einzelveranlagung)'}
            </span>
          </div>
          <div className="flex justify-end gap-2">
            <button
              onClick={() => setShowSettings(false)}
              className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted"
            >
              Abbrechen
            </button>
            <button
              onClick={handleSaveSettings}
              className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90"
            >
              Speichern
            </button>
          </div>
        </div>
      )}

      {/* Freistellung Status */}
      <div className="bg-card rounded-lg border border-border p-4">
        <div className="flex items-center justify-between mb-3">
          <h3 className="font-medium">Freistellungsauftrag {year}</h3>
          <span className={`text-sm px-2 py-1 rounded ${
            freistellungPercent >= 100 ? 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400' :
            freistellungPercent >= 80 ? 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400' :
            'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
          }`}>
            {freistellungPercent.toFixed(0)}% ausgeschöpft
          </span>
        </div>
        <div className="grid grid-cols-3 gap-4 mb-3">
          <div>
            <div className="text-sm text-muted-foreground">Verfügbar</div>
            <div className="text-lg font-medium">{formatCurrency(settings.freistellungLimit)}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground">Verwendet</div>
            <div className="text-lg font-medium text-red-600">{formatCurrency(report.freistellungUsed)}</div>
          </div>
          <div>
            <div className="text-sm text-muted-foreground">Verbleibend</div>
            <div className="text-lg font-medium text-green-600">
              {formatCurrency(report.freistellungAvailable)}
            </div>
          </div>
        </div>
        <div className="w-full bg-muted rounded-full h-3">
          <div
            className={`h-3 rounded-full transition-all ${
              freistellungPercent >= 100 ? 'bg-red-500' :
              freistellungPercent >= 80 ? 'bg-yellow-500' : 'bg-green-500'
            }`}
            style={{ width: `${Math.min(freistellungPercent, 100)}%` }}
          />
        </div>
      </div>

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground mb-1">Dividenden (brutto)</div>
          <div className="text-xl font-bold">{formatCurrency(report.dividendIncomeGross)}</div>
          <div className="text-xs text-muted-foreground mt-1">
            {report.dividendDetails.length} Zahlungen
          </div>
        </div>
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground mb-1">Realisierte Gewinne</div>
          <div className="text-xl font-bold text-green-600">
            {formatCurrency(report.realizedGains)}
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            {report.gainsDetails.length} Verkäufe
          </div>
        </div>
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground mb-1">Realisierte Verluste</div>
          <div className="text-xl font-bold text-red-600">
            -{formatCurrency(report.realizedLosses)}
          </div>
          <div className="text-xs text-muted-foreground mt-1">
            {report.lossesDetails.length} Verlustpositionen
          </div>
        </div>
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="text-sm text-muted-foreground mb-1">Zu versteuern</div>
          <div className="text-xl font-bold">{formatCurrency(report.taxableAfterDeductions)}</div>
          <div className="text-xs text-muted-foreground mt-1">
            nach Freibetrag & Verlustverrechnung
          </div>
        </div>
      </div>

      {/* Tax Calculation */}
      <div className="bg-card rounded-lg border border-border">
        <div className="p-4 border-b border-border">
          <h3 className="font-medium">Steuerberechnung</h3>
        </div>
        <div className="p-4">
          <table className="w-full text-sm">
            <tbody>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Kapitalerträge gesamt</td>
                <td className="py-2 text-right">{formatCurrency(report.totalTaxableIncome)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">- Freistellungsauftrag</td>
                <td className="py-2 text-right text-green-600">-{formatCurrency(report.freistellungUsed)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">- Verlustverrechnung</td>
                <td className="py-2 text-right text-green-600">-{formatCurrency(report.realizedLosses)}</td>
              </tr>
              <tr className="border-b border-border font-medium">
                <td className="py-2">= Steuerpflichtige Erträge</td>
                <td className="py-2 text-right">{formatCurrency(report.taxableAfterDeductions)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Abgeltungssteuer (25%)</td>
                <td className="py-2 text-right">{formatCurrency(report.abgeltungssteuer)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Solidaritätszuschlag (5,5%)</td>
                <td className="py-2 text-right">{formatCurrency(report.solidaritaetszuschlag)}</td>
              </tr>
              {report.kirchensteuer > 0 && (
                <tr className="border-b border-border">
                  <td className="py-2 text-muted-foreground">
                    Kirchensteuer ({(settings.kirchensteuerRate || 0) * 100}%)
                  </td>
                  <td className="py-2 text-right">{formatCurrency(report.kirchensteuer)}</td>
                </tr>
              )}
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">- Anrechenbare Quellensteuer</td>
                <td className="py-2 text-right text-green-600">-{formatCurrency(report.creditableForeignTax)}</td>
              </tr>
              <tr className="font-bold text-lg">
                <td className="py-3">Gesamt Steuerlast</td>
                <td className="py-3 text-right text-red-600">{formatCurrency(report.totalGermanTax)}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Anlage KAP */}
      <div className="bg-card rounded-lg border border-border">
        <div className="p-4 border-b border-border flex items-center justify-between">
          <h3 className="font-medium">Anlage KAP {year}</h3>
          <span className="text-xs text-muted-foreground bg-muted px-2 py-1 rounded">
            Werte für Steuererklärung
          </span>
        </div>
        <div className="p-4">
          <table className="w-full text-sm">
            <tbody>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 7: Inländische Dividenden</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile7InlandDividenden)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 8: Ausländische Dividenden</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile8AuslandDividenden)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 14: Zinsen</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile14Zinsen)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 15: Veräußerungsgewinne</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile15Veraeusserungsgewinne)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 16: Veräußerungsverluste</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile16Veraeusserungsverluste)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 47: Anrechenbare ausl. Steuern</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile47AuslaendischeSteuern)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 48: Kapitalertragsteuer</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile48Kapest)}</td>
              </tr>
              <tr className="border-b border-border">
                <td className="py-2 text-muted-foreground">Zeile 49: Solidaritätszuschlag</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile49Soli)}</td>
              </tr>
              <tr>
                <td className="py-2 text-muted-foreground">Zeile 50: Kirchensteuer</td>
                <td className="py-2 text-right font-mono">{formatCurrency(report.anlageKap.zeile50Kist)}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Dividend Details */}
      {report.dividendDetails.length > 0 && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-4 border-b border-border">
            <h3 className="font-medium">Dividenden-Details</h3>
          </div>
          <div className="overflow-x-auto max-h-64 overflow-y-auto">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 sticky top-0">
                <tr>
                  <th className="text-left py-2 px-4 font-medium">Datum</th>
                  <th className="text-left py-2 px-4 font-medium">Wertpapier</th>
                  <th className="text-right py-2 px-4 font-medium">Brutto</th>
                  <th className="text-right py-2 px-4 font-medium">Quellensteuer</th>
                  <th className="text-right py-2 px-4 font-medium">Netto</th>
                </tr>
              </thead>
              <tbody>
                {report.dividendDetails.map((item, idx) => (
                  <tr key={idx} className="border-b border-border last:border-0">
                    <td className="py-2 px-4">{formatDate(item.date)}</td>
                    <td className="py-2 px-4">{item.securityName}</td>
                    <td className="py-2 px-4 text-right">{formatCurrency(item.grossAmount)}</td>
                    <td className="py-2 px-4 text-right text-red-600">
                      -{formatCurrency(item.withholdingTax)}
                    </td>
                    <td className="py-2 px-4 text-right font-medium">{formatCurrency(item.netAmount)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Gains/Losses Details */}
      {(report.gainsDetails.length > 0 || report.lossesDetails.length > 0) && (
        <div className="bg-card rounded-lg border border-border">
          <div className="p-4 border-b border-border">
            <h3 className="font-medium">Realisierte Gewinne/Verluste</h3>
          </div>
          <div className="overflow-x-auto max-h-64 overflow-y-auto">
            <table className="w-full text-sm">
              <thead className="bg-muted/50 sticky top-0">
                <tr>
                  <th className="text-left py-2 px-4 font-medium">Datum</th>
                  <th className="text-left py-2 px-4 font-medium">Wertpapier</th>
                  <th className="text-right py-2 px-4 font-medium">Verkaufserlös</th>
                  <th className="text-right py-2 px-4 font-medium">Gewinn/Verlust</th>
                </tr>
              </thead>
              <tbody>
                {[...report.gainsDetails, ...report.lossesDetails]
                  .sort((a, b) => b.date.localeCompare(a.date))
                  .map((item, idx) => (
                    <tr key={idx} className="border-b border-border last:border-0">
                      <td className="py-2 px-4">{formatDate(item.date)}</td>
                      <td className="py-2 px-4">{item.securityName}</td>
                      <td className="py-2 px-4 text-right">{formatCurrency(item.grossAmount)}</td>
                      <td className={`py-2 px-4 text-right font-medium ${
                        item.netAmount >= 0 ? 'text-green-600' : 'text-red-600'
                      }`}>
                        {item.netAmount >= 0 ? '+' : ''}{formatCurrency(item.netAmount)}
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
}
