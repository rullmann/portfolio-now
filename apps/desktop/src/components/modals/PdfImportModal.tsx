/**
 * PDF Import Modal for importing bank statements.
 */

import { useState, useEffect } from 'react';
import { X, Upload, FileText, AlertCircle, CheckCircle, Loader2 } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import {
  getSupportedBanks,
  previewPdfImport,
  importPdfTransactions,
  getPortfolios,
  getAccounts,
} from '../../lib/api';
import type {
  SupportedBank,
  PdfImportPreview,
  PortfolioData,
  AccountData,
} from '../../lib/types';

interface PdfImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

type Step = 'select' | 'preview' | 'configure' | 'importing' | 'done';

export function PdfImportModal({ isOpen, onClose, onSuccess }: PdfImportModalProps) {
  const [step, setStep] = useState<Step>('select');
  const [supportedBanks, setSupportedBanks] = useState<SupportedBank[]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [preview, setPreview] = useState<PdfImportPreview | null>(null);
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [accounts, setAccounts] = useState<AccountData[]>([]);
  const [selectedPortfolio, setSelectedPortfolio] = useState<number | null>(null);
  const [selectedAccount, setSelectedAccount] = useState<number | null>(null);
  const [createMissingSecurities, setCreateMissingSecurities] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [importResult, setImportResult] = useState<{
    success: boolean;
    transactionsImported: number;
    securitiesCreated: number;
    errors: string[];
  } | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadInitialData();
    } else {
      resetState();
    }
  }, [isOpen]);

  const loadInitialData = async () => {
    try {
      const [banks, portfolioList, accountList] = await Promise.all([
        getSupportedBanks(),
        getPortfolios(),
        getAccounts(),
      ]);
      setSupportedBanks(banks);
      setPortfolios(portfolioList.filter(p => !p.isRetired));
      setAccounts(accountList.filter(a => !a.isRetired));

      // Set defaults
      if (portfolioList.length > 0) {
        setSelectedPortfolio(portfolioList.find(p => !p.isRetired)?.id ?? null);
      }
      if (accountList.length > 0) {
        setSelectedAccount(accountList.find(a => !a.isRetired)?.id ?? null);
      }
    } catch (err) {
      console.error('Failed to load initial data:', err);
    }
  };

  const resetState = () => {
    setStep('select');
    setSelectedFile(null);
    setPreview(null);
    setError(null);
    setImportResult(null);
  };

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });

      if (selected) {
        setSelectedFile(selected as string);
        setIsLoading(true);
        setError(null);

        // Get preview (includes detected bank)
        const previewData = await previewPdfImport(selected as string);
        setPreview(previewData);
        setStep('preview');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleProceedToConfig = () => {
    setStep('configure');
  };

  const handleImport = async () => {
    if (!selectedFile || !selectedPortfolio || !selectedAccount) return;

    setStep('importing');
    setError(null);

    try {
      const result = await importPdfTransactions(
        selectedFile,
        selectedPortfolio,
        selectedAccount,
        createMissingSecurities
      );

      setImportResult({
        success: result.success,
        transactionsImported: result.transactionsImported,
        securitiesCreated: result.securitiesCreated,
        errors: result.errors,
      });
      setStep('done');

      if (result.success && onSuccess) {
        onSuccess();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStep('configure');
    }
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString('de-DE');
  };

  const formatCurrency = (amount: number, currency: string) => {
    return `${(amount / 100).toLocaleString('de-DE', { minimumFractionDigits: 2 })} ${currency}`;
  };

  const getTxnTypeLabel = (type: string): string => {
    const labels: Record<string, string> = {
      Buy: 'Kauf',
      Sell: 'Verkauf',
      Dividend: 'Dividende',
      Interest: 'Zinsen',
      Deposit: 'Einzahlung',
      Withdrawal: 'Auszahlung',
      Fee: 'Gebühr',
      TaxRefund: 'Steuererstattung',
      TransferIn: 'Eingang',
      TransferOut: 'Ausgang',
      Unknown: 'Unbekannt',
    };
    return labels[type] || type;
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg shadow-lg w-full max-w-4xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2">
            <FileText className="w-5 h-5 text-primary" />
            <h2 className="text-lg font-semibold">PDF Import</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-6">
          {error && (
            <div className="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm flex items-center gap-2">
              <AlertCircle size={16} />
              {error}
            </div>
          )}

          {/* Step: Select File */}
          {step === 'select' && (
            <div className="space-y-6">
              <div
                onClick={handleSelectFile}
                className="border-2 border-dashed border-border rounded-lg p-12 text-center cursor-pointer hover:border-primary hover:bg-muted/50 transition-colors"
              >
                {isLoading ? (
                  <Loader2 className="w-12 h-12 mx-auto mb-4 text-primary animate-spin" />
                ) : (
                  <Upload className="w-12 h-12 mx-auto mb-4 text-muted-foreground" />
                )}
                <p className="text-lg font-medium mb-2">
                  {isLoading ? 'PDF wird analysiert...' : 'PDF-Datei auswählen'}
                </p>
                <p className="text-sm text-muted-foreground">
                  Klicken Sie hier oder ziehen Sie eine PDF-Datei hierher
                </p>
              </div>

              {/* Supported Banks */}
              <div>
                <h3 className="font-medium mb-3">Unterstützte Banken</h3>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-2">
                  {supportedBanks.map((bank) => (
                    <div
                      key={bank.id}
                      className="p-3 bg-muted rounded-md"
                    >
                      <div className="font-medium text-sm">{bank.name}</div>
                      <div className="text-xs text-muted-foreground">{bank.description}</div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Step: Preview */}
          {step === 'preview' && preview && (
            <div className="space-y-6">
              {/* Summary */}
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="bg-muted rounded-lg p-4">
                  <div className="text-sm text-muted-foreground">Erkannte Bank</div>
                  <div className="text-lg font-bold">{preview.bank || 'Unbekannt'}</div>
                </div>
                <div className="bg-muted rounded-lg p-4">
                  <div className="text-sm text-muted-foreground">Transaktionen</div>
                  <div className="text-lg font-bold">{preview.transactions.length}</div>
                </div>
                <div className="bg-muted rounded-lg p-4">
                  <div className="text-sm text-muted-foreground">Neue Wertpapiere</div>
                  <div className="text-lg font-bold">{preview.newSecurities.length}</div>
                </div>
                <div className="bg-muted rounded-lg p-4">
                  <div className="text-sm text-muted-foreground">Warnungen</div>
                  <div className="text-lg font-bold">{preview.warnings.length}</div>
                </div>
              </div>

              {/* Warnings */}
              {preview.warnings.length > 0 && (
                <div className="bg-amber-500/10 border border-amber-500/20 rounded-lg p-4">
                  <h4 className="font-medium text-amber-600 mb-2">Warnungen</h4>
                  <ul className="text-sm space-y-1">
                    {preview.warnings.map((warning, idx) => (
                      <li key={idx} className="text-amber-600">{warning}</li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Transactions Preview */}
              <div>
                <h3 className="font-medium mb-3">Erkannte Transaktionen</h3>
                <div className="border border-border rounded-lg overflow-hidden">
                  <div className="overflow-x-auto max-h-64">
                    <table className="w-full text-sm">
                      <thead className="bg-muted sticky top-0">
                        <tr>
                          <th className="text-left py-2 px-3 font-medium">Datum</th>
                          <th className="text-left py-2 px-3 font-medium">Typ</th>
                          <th className="text-left py-2 px-3 font-medium">Wertpapier</th>
                          <th className="text-right py-2 px-3 font-medium">Stück</th>
                          <th className="text-right py-2 px-3 font-medium">Betrag</th>
                        </tr>
                      </thead>
                      <tbody>
                        {preview.transactions.map((txn, idx) => (
                          <tr key={idx} className="border-t border-border">
                            <td className="py-2 px-3">{formatDate(txn.date)}</td>
                            <td className="py-2 px-3">
                              <span className={`px-2 py-0.5 rounded text-xs ${
                                txn.txnType === 'Buy' ? 'bg-green-500/10 text-green-600' :
                                txn.txnType === 'Sell' ? 'bg-red-500/10 text-red-600' :
                                txn.txnType === 'Dividend' ? 'bg-blue-500/10 text-blue-600' :
                                'bg-muted text-muted-foreground'
                              }`}>
                                {getTxnTypeLabel(txn.txnType)}
                              </span>
                            </td>
                            <td className="py-2 px-3">
                              <div className="font-medium">{txn.securityName || '-'}</div>
                              <div className="text-xs text-muted-foreground">{txn.isin || txn.wkn || ''}</div>
                            </td>
                            <td className="py-2 px-3 text-right">{txn.shares?.toLocaleString('de-DE') || '-'}</td>
                            <td className="py-2 px-3 text-right font-medium">
                              {formatCurrency(txn.netAmount * 100, txn.currency)}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>

              {/* New Securities */}
              {preview.newSecurities.length > 0 && (
                <div>
                  <h3 className="font-medium mb-3">Neue Wertpapiere (werden angelegt)</h3>
                  <div className="space-y-2">
                    {preview.newSecurities.map((sec, idx) => (
                      <div key={idx} className="flex items-center gap-4 p-3 bg-muted rounded-lg">
                        <div className="flex-1">
                          <div className="font-medium">{sec.name}</div>
                          <div className="text-xs text-muted-foreground">
                            {sec.isin && `ISIN: ${sec.isin}`}
                            {sec.wkn && ` WKN: ${sec.wkn}`}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Step: Configure */}
          {step === 'configure' && (
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-4">Import-Einstellungen</h3>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium mb-1">Portfolio</label>
                    <select
                      value={selectedPortfolio || ''}
                      onChange={(e) => setSelectedPortfolio(Number(e.target.value))}
                      className="w-full px-3 py-2 border border-border rounded-md bg-background"
                    >
                      <option value="">Portfolio wählen...</option>
                      {portfolios.map(p => (
                        <option key={p.id} value={p.id}>{p.name}</option>
                      ))}
                    </select>
                    <p className="text-xs text-muted-foreground mt-1">
                      Für Kauf-/Verkauf-Transaktionen
                    </p>
                  </div>

                  <div>
                    <label className="block text-sm font-medium mb-1">Verrechnungskonto</label>
                    <select
                      value={selectedAccount || ''}
                      onChange={(e) => setSelectedAccount(Number(e.target.value))}
                      className="w-full px-3 py-2 border border-border rounded-md bg-background"
                    >
                      <option value="">Konto wählen...</option>
                      {accounts.map(a => (
                        <option key={a.id} value={a.id}>{a.name} ({a.currency})</option>
                      ))}
                    </select>
                    <p className="text-xs text-muted-foreground mt-1">
                      Für Buchungen und Dividenden
                    </p>
                  </div>
                </div>

                <div className="mt-4">
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={createMissingSecurities}
                      onChange={(e) => setCreateMissingSecurities(e.target.checked)}
                      className="rounded border-border"
                    />
                    <span className="text-sm">Fehlende Wertpapiere automatisch anlegen</span>
                  </label>
                </div>
              </div>

              {/* Summary */}
              {preview && (
                <div className="bg-muted rounded-lg p-4">
                  <h4 className="font-medium mb-2">Zusammenfassung</h4>
                  <ul className="text-sm space-y-1">
                    <li>{preview.transactions.length} Transaktionen werden importiert</li>
                    {preview.newSecurities.length > 0 && (
                      <li>{preview.newSecurities.length} neue Wertpapiere werden angelegt</li>
                    )}
                  </ul>
                </div>
              )}
            </div>
          )}

          {/* Step: Importing */}
          {step === 'importing' && (
            <div className="text-center py-12">
              <Loader2 className="w-12 h-12 mx-auto mb-4 text-primary animate-spin" />
              <p className="text-lg font-medium">Transaktionen werden importiert...</p>
              <p className="text-sm text-muted-foreground mt-2">Bitte warten Sie einen Moment.</p>
            </div>
          )}

          {/* Step: Done */}
          {step === 'done' && importResult && (
            <div className="text-center py-8">
              {importResult.success ? (
                <>
                  <CheckCircle className="w-16 h-16 mx-auto mb-4 text-green-600" />
                  <h3 className="text-xl font-bold mb-2">Import erfolgreich!</h3>
                  <p className="text-muted-foreground mb-6">
                    {importResult.transactionsImported} Transaktionen wurden importiert.
                    {importResult.securitiesCreated > 0 && (
                      <> {importResult.securitiesCreated} neue Wertpapiere wurden angelegt.</>
                    )}
                  </p>
                </>
              ) : (
                <>
                  <AlertCircle className="w-16 h-16 mx-auto mb-4 text-destructive" />
                  <h3 className="text-xl font-bold mb-2">Import fehlgeschlagen</h3>
                </>
              )}

              {importResult.errors.length > 0 && (
                <div className="text-left bg-destructive/10 border border-destructive/20 rounded-lg p-4 mt-4">
                  <h4 className="font-medium text-destructive mb-2">Fehler</h4>
                  <ul className="text-sm text-destructive space-y-1">
                    {importResult.errors.map((err, idx) => (
                      <li key={idx}>{err}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-between p-4 border-t border-border">
          <button
            onClick={step === 'done' ? onClose : () => setStep('select')}
            className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            {step === 'done' ? 'Schließen' : 'Zurück'}
          </button>

          {step === 'preview' && (
            <button
              onClick={handleProceedToConfig}
              className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
            >
              Weiter zur Konfiguration
            </button>
          )}

          {step === 'configure' && (
            <button
              onClick={handleImport}
              disabled={!selectedPortfolio || !selectedAccount}
              className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              Import starten
            </button>
          )}

          {step === 'done' && importResult?.success && (
            <button
              onClick={onClose}
              className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
            >
              Fertig
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
