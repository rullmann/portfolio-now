/**
 * DivvyDiary Export Modal for uploading portfolio to DivvyDiary.
 *
 * API-Key management is handled exclusively in Settings.
 * This modal only USES the key from secure storage — never edits it.
 */

import { useState, useEffect } from 'react';
import { X, Upload, Loader2, CheckCircle, AlertCircle, Info, Settings } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getPortfolios } from '../../lib/api';
import type { PortfolioData } from '../../lib/types';
import { useEscapeKey } from '../../lib/hooks';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { useUIStore } from '../../store';
import { DivvyDiaryLogo } from '../common/DivvyDiaryLogo';

interface DivvyDiaryExportModalProps {
  isOpen: boolean;
  onClose: () => void;
}

interface DivvyDiaryPortfolio {
  id: string;
  name: string;
}

interface ExportResult {
  success: boolean;
  message: string;
  securitiesCount: number;
  activitiesCount: number;
}

export function DivvyDiaryExportModal({ isOpen, onClose }: DivvyDiaryExportModalProps) {
  useEscapeKey(isOpen, onClose);

  const { keys: secureKeys } = useSecureApiKeys();
  const apiKey = secureKeys.divvyDiaryApiKey;

  const [localPortfolios, setLocalPortfolios] = useState<PortfolioData[]>([]);
  const [selectedLocalPortfolio, setSelectedLocalPortfolio] = useState<'all' | number>('all');
  const [divvyDiaryPortfolios, setDivvyDiaryPortfolios] = useState<DivvyDiaryPortfolio[]>([]);
  const [selectedDivvyDiaryPortfolio, setSelectedDivvyDiaryPortfolio] = useState<string>('');
  const [includeTransactions, setIncludeTransactions] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<ExportResult | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadLocalPortfolios();
      setError(null);
      setSuccess(null);
      setDivvyDiaryPortfolios([]);
      setSelectedDivvyDiaryPortfolio('');
    }
  }, [isOpen]);

  // Auto-load DivvyDiary portfolios when key is available
  useEffect(() => {
    if (isOpen && apiKey) {
      loadDivvyDiaryPortfolios();
    }
  }, [isOpen, apiKey]);

  const loadLocalPortfolios = async () => {
    try {
      const data = await getPortfolios();
      const activePortfoliosWithHoldings = data.filter(p => !p.isRetired && (p.holdingsCount ?? 0) > 0);
      setLocalPortfolios(activePortfoliosWithHoldings);
    } catch (err) {
      console.error('Failed to load portfolios:', err);
    }
  };

  const loadDivvyDiaryPortfolios = async () => {
    if (!apiKey) return;

    setIsLoading(true);
    setError(null);

    try {
      const portfolios = await invoke<DivvyDiaryPortfolio[]>('get_divvydiary_portfolios', {
        apiKey,
      });
      setDivvyDiaryPortfolios(portfolios);
      if (portfolios.length > 0 && !selectedDivvyDiaryPortfolio) {
        setSelectedDivvyDiaryPortfolio(portfolios[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setDivvyDiaryPortfolios([]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleExport = async () => {
    if (!apiKey || !selectedDivvyDiaryPortfolio) return;

    setIsExporting(true);
    setError(null);
    setSuccess(null);

    try {
      const result = await invoke<ExportResult>('upload_to_divvydiary', {
        apiKey,
        divvydiaryPortfolioId: selectedDivvyDiaryPortfolio,
        portfolioId: selectedLocalPortfolio === 'all' ? null : selectedLocalPortfolio,
        includeTransactions,
      });

      setSuccess(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsExporting(false);
    }
  };

  const goToSettings = () => {
    onClose();
    useUIStore.getState().setCurrentView('settings');
    useUIStore.getState().setScrollTarget('services');
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg shadow-lg w-full max-w-lg max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-3">
            <DivvyDiaryLogo size={28} />
            <div>
              <h2 className="text-lg font-semibold">DivvyDiary Export</h2>
              <p className="text-xs text-muted-foreground">Portfolio an DivvyDiary übertragen</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-5 overflow-y-auto flex-1">
          {error && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm flex items-center gap-2">
              <AlertCircle size={16} className="flex-shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {success && (
            <div className="p-3 bg-green-500/10 border border-green-500/20 rounded-md text-green-600 text-sm flex items-center gap-2">
              <CheckCircle size={16} className="flex-shrink-0" />
              <span>{success.message}</span>
            </div>
          )}

          {/* No API key → redirect to Settings */}
          {!apiKey ? (
            <div className="p-4 bg-amber-500/10 border border-amber-500/20 rounded-md space-y-3">
              <div className="flex items-start gap-2">
                <AlertCircle size={16} className="flex-shrink-0 mt-0.5 text-amber-600" />
                <div className="text-sm text-muted-foreground">
                  <span className="font-medium text-foreground">Kein API-Key hinterlegt.</span>{' '}
                  Bitte hinterlege deinen DivvyDiary API-Key in den Einstellungen.
                </div>
              </div>
              <button
                onClick={goToSettings}
                className="flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-md bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
              >
                <Settings size={16} />
                Zu den Einstellungen
              </button>
            </div>
          ) : (
            <>
              {/* Info Banner */}
              <div className="p-3 bg-blue-500/10 border border-blue-500/20 rounded-md text-sm flex items-start gap-2">
                <Info size={16} className="flex-shrink-0 mt-0.5 text-blue-600" />
                <div className="text-muted-foreground">
                  <span className="font-medium text-foreground">Hinweis:</span> Beim Import werden deine
                  bestehenden Bestände in DivvyDiary überschrieben. Nur Wertpapiere mit ISIN werden exportiert.
                </div>
              </div>

              {/* DivvyDiary Portfolio Selection */}
              {isLoading ? (
                <div className="flex items-center gap-2 py-2">
                  <Loader2 size={16} className="text-muted-foreground animate-spin" />
                  <span className="text-sm text-muted-foreground">Lade DivvyDiary-Portfolios...</span>
                </div>
              ) : divvyDiaryPortfolios.length > 0 ? (
                <div>
                  <label className="block text-sm font-medium mb-1">Ziel-Portfolio in DivvyDiary</label>
                  <select
                    value={selectedDivvyDiaryPortfolio}
                    onChange={(e) => setSelectedDivvyDiaryPortfolio(e.target.value)}
                    className="w-full px-3 py-2 border border-border rounded-md bg-background text-sm"
                  >
                    {divvyDiaryPortfolios.map(p => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))}
                  </select>
                </div>
              ) : null}

              {/* Local Portfolio Selection */}
              <div>
                <label className="block text-sm font-medium mb-2">Portfolio zum Exportieren auswählen</label>
                <div className="space-y-1 max-h-40 overflow-y-auto border border-border rounded-md p-2">
                  {localPortfolios.length === 0 ? (
                    <p className="text-sm text-muted-foreground p-2">Keine Portfolios vorhanden</p>
                  ) : (
                    <>
                      <label
                        className={`flex items-center gap-2 p-2 rounded cursor-pointer transition-colors ${
                          selectedLocalPortfolio === 'all'
                            ? 'bg-primary/10'
                            : 'hover:bg-muted/50'
                        }`}
                      >
                        <input
                          type="radio"
                          name="localPortfolio"
                          checked={selectedLocalPortfolio === 'all'}
                          onChange={() => setSelectedLocalPortfolio('all')}
                          className="text-primary"
                        />
                        <span className="text-sm font-medium">Alle Portfolios</span>
                        <span className="text-xs text-muted-foreground ml-auto">
                          {localPortfolios.reduce((sum, p) => sum + (p.holdingsCount ?? 0), 0)} Positionen
                        </span>
                      </label>
                      {localPortfolios.map(portfolio => (
                        <label
                          key={portfolio.id}
                          className={`flex items-center gap-2 p-2 rounded cursor-pointer transition-colors ${
                            selectedLocalPortfolio === portfolio.id
                              ? 'bg-primary/10'
                              : 'hover:bg-muted/50'
                          }`}
                        >
                          <input
                            type="radio"
                            name="localPortfolio"
                            checked={selectedLocalPortfolio === portfolio.id}
                            onChange={() => setSelectedLocalPortfolio(portfolio.id)}
                            className="text-primary"
                          />
                          <span className="text-sm">{portfolio.name}</span>
                          <span className="text-xs text-muted-foreground ml-auto">
                            {portfolio.holdingsCount} Positionen
                          </span>
                        </label>
                      ))}
                    </>
                  )}
                </div>
              </div>

              {/* Include Transactions Toggle */}
              <label className="flex items-center gap-3 p-3 border border-border rounded-md cursor-pointer hover:bg-muted/50 transition-colors">
                <input
                  type="checkbox"
                  checked={includeTransactions}
                  onChange={(e) => setIncludeTransactions(e.target.checked)}
                  className="rounded"
                />
                <div>
                  <div className="text-sm font-medium">Transaktionen übertragen</div>
                  <div className="text-xs text-muted-foreground">
                    Kauf- und Verkaufshistorie mit übertragen (empfohlen)
                  </div>
                </div>
              </label>
            </>
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
            disabled={isExporting || !apiKey || !selectedDivvyDiaryPortfolio}
            className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isExporting ? (
              <>
                <Loader2 size={16} className="animate-spin" />
                Exportiere...
              </>
            ) : (
              <>
                <Upload size={16} />
                Exportieren
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
