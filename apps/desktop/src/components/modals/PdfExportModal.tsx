/**
 * PDF Export Modal for exporting reports.
 */

import { useState, useEffect } from 'react';
import { X, Download, FileText, Loader2, CheckCircle, AlertCircle } from 'lucide-react';
import { save } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getPortfolios } from '../../lib/api';
import type { PortfolioData } from '../../lib/types';
import { useEscapeKey } from '../../lib/hooks';

interface PdfExportModalProps {
  isOpen: boolean;
  onClose: () => void;
}

type ExportType = 'summary' | 'holdings' | 'performance' | 'dividends' | 'tax';

interface ExportOption {
  id: ExportType;
  name: string;
  description: string;
}

const exportOptions: ExportOption[] = [
  {
    id: 'summary',
    name: 'Portfolio-Zusammenfassung',
    description: 'Übersicht aller Portfolios mit Kennzahlen',
  },
  {
    id: 'holdings',
    name: 'Vermögensaufstellung',
    description: 'Detaillierte Liste aller gehaltenen Wertpapiere',
  },
  {
    id: 'performance',
    name: 'Performance-Bericht',
    description: 'TTWROR, IRR und Wertentwicklung',
  },
  {
    id: 'dividends',
    name: 'Dividenden-Bericht',
    description: 'Übersicht aller erhaltenen Dividenden',
  },
  {
    id: 'tax',
    name: 'Steuerbericht',
    description: 'Kapitalerträge und Quellensteuer für die Steuererklärung',
  },
];

export function PdfExportModal({ isOpen, onClose }: PdfExportModalProps) {
  useEscapeKey(isOpen, onClose);

  const [selectedType, setSelectedType] = useState<ExportType>('summary');
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [selectedPortfolio, setSelectedPortfolio] = useState<number | undefined>(undefined);
  const [year, setYear] = useState<number>(new Date().getFullYear());
  const [startDate, setStartDate] = useState<string>(() => {
    const d = new Date();
    d.setFullYear(d.getFullYear() - 1);
    return d.toISOString().split('T')[0];
  });
  const [endDate, setEndDate] = useState<string>(() => new Date().toISOString().split('T')[0]);
  const [isExporting, setIsExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadPortfolios();
      setError(null);
      setSuccess(null);
    }
  }, [isOpen]);

  const loadPortfolios = async () => {
    try {
      const data = await getPortfolios();
      setPortfolios(data.filter(p => !p.isRetired));
    } catch (err) {
      console.error('Failed to load portfolios:', err);
    }
  };

  const handleExport = async () => {
    try {
      // Select save location
      const defaultFileName = `${selectedType}-${new Date().toISOString().split('T')[0]}.pdf`;
      const savePath = await save({
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
        defaultPath: defaultFileName,
      });

      if (!savePath) return;

      setIsExporting(true);
      setError(null);

      // Call appropriate export command
      let command: string;
      let params: Record<string, unknown> = { path: savePath };

      switch (selectedType) {
        case 'summary':
          command = 'export_portfolio_summary_pdf';
          params.portfolioId = selectedPortfolio ?? null;
          break;
        case 'holdings':
          command = 'export_holdings_pdf';
          params.date = endDate;
          params.portfolioId = selectedPortfolio ?? null;
          break;
        case 'performance':
          command = 'export_performance_pdf';
          params.portfolioId = selectedPortfolio ?? null;
          params.startDate = startDate;
          params.endDate = endDate;
          break;
        case 'dividends':
          command = 'export_dividend_pdf';
          params.year = year;
          params.portfolioId = selectedPortfolio ?? null;
          break;
        case 'tax':
          command = 'export_tax_report_pdf';
          params.year = year;
          break;
        default:
          throw new Error('Unbekannter Export-Typ');
      }

      await invoke(command, params);
      setSuccess(`PDF wurde erfolgreich exportiert nach: ${savePath}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsExporting(false);
    }
  };

  const needsDateRange = selectedType === 'performance';
  const needsYear = ['tax', 'dividends'].includes(selectedType);
  const needsPortfolio = ['summary', 'holdings', 'performance', 'dividends'].includes(selectedType);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg shadow-lg w-full max-w-lg">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2">
            <Download className="w-5 h-5 text-primary" />
            <h2 className="text-lg font-semibold">PDF Export</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-6">
          {error && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm flex items-center gap-2">
              <AlertCircle size={16} />
              {error}
            </div>
          )}

          {success && (
            <div className="p-3 bg-green-500/10 border border-green-500/20 rounded-md text-green-600 text-sm flex items-center gap-2">
              <CheckCircle size={16} />
              {success}
            </div>
          )}

          {/* Export Type Selection */}
          <div>
            <label className="block text-sm font-medium mb-2">Berichtstyp</label>
            <div className="space-y-2">
              {exportOptions.map((option) => (
                <label
                  key={option.id}
                  className={`flex items-start gap-3 p-3 rounded-md border cursor-pointer transition-colors ${
                    selectedType === option.id
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  <input
                    type="radio"
                    name="exportType"
                    value={option.id}
                    checked={selectedType === option.id}
                    onChange={() => setSelectedType(option.id)}
                    className="mt-1"
                  />
                  <div>
                    <div className="font-medium">{option.name}</div>
                    <div className="text-sm text-muted-foreground">{option.description}</div>
                  </div>
                </label>
              ))}
            </div>
          </div>

          {/* Portfolio Selection */}
          {needsPortfolio && (
            <div>
              <label className="block text-sm font-medium mb-1">Portfolio</label>
              <select
                value={selectedPortfolio || ''}
                onChange={(e) => setSelectedPortfolio(e.target.value ? Number(e.target.value) : undefined)}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              >
                <option value="">Alle Portfolios</option>
                {portfolios.map(p => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
            </div>
          )}

          {/* Date Range */}
          {needsDateRange && (
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium mb-1">Von</label>
                <input
                  type="date"
                  value={startDate}
                  onChange={(e) => setStartDate(e.target.value)}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">Bis</label>
                <input
                  type="date"
                  value={endDate}
                  onChange={(e) => setEndDate(e.target.value)}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background"
                />
              </div>
            </div>
          )}

          {/* Year Selection */}
          {needsYear && (
            <div>
              <label className="block text-sm font-medium mb-1">Jahr</label>
              <select
                value={year}
                onChange={(e) => setYear(Number(e.target.value))}
                className="w-full px-3 py-2 border border-border rounded-md bg-background"
              >
                {Array.from({ length: 10 }, (_, i) => new Date().getFullYear() - i).map(y => (
                  <option key={y} value={y}>{y}</option>
                ))}
              </select>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 p-4 border-t border-border">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            Abbrechen
          </button>
          <button
            onClick={handleExport}
            disabled={isExporting}
            className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            {isExporting ? (
              <>
                <Loader2 size={16} className="animate-spin" />
                Exportiere...
              </>
            ) : (
              <>
                <FileText size={16} />
                PDF exportieren
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
