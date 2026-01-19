/**
 * PDF Import Modal for importing bank statements.
 */

import { useState, useEffect } from 'react';
import { X, Upload, FileText, AlertCircle, CheckCircle, Loader2, ScanText, AlertTriangle } from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { ErrorBoundary } from '../common/ErrorBoundary';
import { AIProviderLogo } from '../common/AIProviderLogo';
import {
  getSupportedBanks,
  previewPdfImport,
  importPdfTransactions,
  getPortfolios,
  getAccounts,
} from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';
import { useSettingsStore, getVisionModels, type AiProvider } from '../../store';
import type {
  SupportedBank,
  PdfImportPreview,
  PortfolioData,
  AccountData,
} from '../../lib/types';
import { formatDate } from '../../lib/types';

interface PdfImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

type Step = 'select' | 'preview' | 'importing' | 'done';

// Transaction type options for override
const TXN_TYPE_OPTIONS = [
  { value: 'Buy', label: 'Kauf' },
  { value: 'Sell', label: 'Verkauf' },
  { value: 'TransferIn', label: 'Einlieferung' },
  { value: 'TransferOut', label: 'Auslieferung' },
  { value: 'Dividend', label: 'Dividende' },
  { value: 'Interest', label: 'Zinsen' },
  { value: 'Deposit', label: 'Einzahlung' },
  { value: 'Withdrawal', label: 'Auszahlung' },
  { value: 'Fee', label: 'Gebühr' },
] as const;

export function PdfImportModal({ isOpen, onClose, onSuccess }: PdfImportModalProps) {
  const {
    deliveryMode,
    aiEnabled,
    aiFeatureSettings,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
  } = useSettingsStore();

  // Get feature-specific provider and model for PDF OCR
  const { provider: aiProvider, model: aiModel } = aiFeatureSettings.pdfOcr;

  // ESC key to close
  useEscapeKey(isOpen, onClose);

  const [step, setStep] = useState<Step>('select');
  const [supportedBanks, setSupportedBanks] = useState<SupportedBank[]>([]);
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [previews, setPreviews] = useState<Array<PdfImportPreview & { filePath: string; fileName: string }>>([]);
  // Combined preview for display
  const [combinedPreview, setCombinedPreview] = useState<PdfImportPreview | null>(null);
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [accounts, setAccounts] = useState<AccountData[]>([]);
  // Portfolio selection per file (file index -> portfolio id)
  const [portfolioPerFile, setPortfolioPerFile] = useState<Record<number, number>>({});
  const [selectedAccount, setSelectedAccount] = useState<number | null>(null);
  const [createMissingSecurities, setCreateMissingSecurities] = useState(true);
  const [skipDuplicates, setSkipDuplicates] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [importResult, setImportResult] = useState<{
    success: boolean;
    transactionsImported: number;
    transactionsSkipped: number;
    securitiesCreated: number;
    errors: string[];
    warnings: string[];
  } | null>(null);
  // Transaction type overrides (index -> new type)
  const [txnTypeOverrides, setTxnTypeOverrides] = useState<Record<number, string>>({});
  // Fee overrides (index -> new fee value)
  const [feeOverrides, setFeeOverrides] = useState<Record<number, number>>({});

  // OCR state
  const [useOcrFallback, setUseOcrFallback] = useState(false);
  const [isOcrAvailable, setIsOcrAvailable] = useState(false);
  const [ocrStatus, setOcrStatus] = useState<string | null>(null);
  const [showOcrConsentDialog, setShowOcrConsentDialog] = useState(false);
  const [ocrConsentGiven, setOcrConsentGiven] = useState(false);
  const [modelHasVision, setModelHasVision] = useState(true);
  const [isCheckingVision, setIsCheckingVision] = useState(false);
  const [isOcrActive, setIsOcrActive] = useState(false);
  const [ocrProgress, setOcrProgress] = useState<{ current: number; total: number } | null>(null);

  // Check if provider supports direct PDF upload (no Poppler needed)
  const providerSupportsDirectPdf = aiProvider === 'claude' || aiProvider === 'gemini';
  // Poppler only required for OpenAI/Perplexity
  const needsPoppler = !providerSupportsDirectPdf;

  // Get the current AI API key based on provider
  const getOcrApiKey = () => {
    switch (aiProvider) {
      case 'claude':
        return anthropicApiKey;
      case 'openai':
        return openaiApiKey;
      case 'gemini':
        return geminiApiKey;
      case 'perplexity':
        return perplexityApiKey;
      default:
        return '';
    }
  };

  const hasOcrApiKey = () => {
    const key = getOcrApiKey();
    return key && key.trim().length > 0;
  };

  useEffect(() => {
    if (isOpen) {
      loadInitialData();
    } else {
      resetState();
    }
  }, [isOpen]);

  // Check if configured model supports vision when provider/model changes
  useEffect(() => {
    const checkVision = async () => {
      if (!aiProvider || !aiModel) {
        setModelHasVision(false);
        return;
      }

      setIsCheckingVision(true);
      try {
        const visionModels = await getVisionModels(aiProvider);
        const hasVision = visionModels.some(m => m.id === aiModel);
        setModelHasVision(hasVision);
      } catch (err) {
        console.warn('Failed to check vision support:', err);
        // Assume vision is supported if check fails
        setModelHasVision(true);
      } finally {
        setIsCheckingVision(false);
      }
    };

    if (isOpen && aiEnabled) {
      checkVision();
    }
  }, [isOpen, aiProvider, aiModel, aiEnabled]);

  const loadInitialData = async () => {
    try {
      const [banks, portfolioList, accountList, ocrAvailable] = await Promise.all([
        getSupportedBanks(),
        getPortfolios(),
        getAccounts(),
        invoke<boolean>('is_ocr_available').catch(() => false),
      ]);
      setSupportedBanks(banks);
      setPortfolios(portfolioList.filter(p => !p.isRetired));
      setAccounts(accountList.filter(a => !a.isRetired));
      setIsOcrAvailable(ocrAvailable);

      // Set defaults
      if (accountList.length > 0) {
        setSelectedAccount(accountList.find(a => !a.isRetired)?.id ?? null);
      }
    } catch (err) {
      console.error('Failed to load initial data:', err);
    }
  };

  const resetState = () => {
    setStep('select');
    setSelectedFiles([]);
    setPreviews([]);
    setCombinedPreview(null);
    setPortfolioPerFile({});
    setError(null);
    setImportResult(null);
    setTxnTypeOverrides({});
    setFeeOverrides({});
    setUseOcrFallback(false);
    setOcrStatus(null);
    setShowOcrConsentDialog(false);
    setOcrConsentGiven(false);
    setModelHasVision(true);
    setIsCheckingVision(false);
    setIsOcrActive(false);
    setOcrProgress(null);
  };

  // Handle OCR checkbox change - show consent dialog if enabling
  const handleOcrToggle = (enabled: boolean) => {
    if (enabled && !ocrConsentGiven) {
      setShowOcrConsentDialog(true);
    } else if (!enabled) {
      setUseOcrFallback(false);
      setOcrConsentGiven(false);
    }
  };

  // Handle OCR consent confirmation
  const handleOcrConsentConfirm = () => {
    setOcrConsentGiven(true);
    setUseOcrFallback(true);
    setShowOcrConsentDialog(false);
  };

  // Handle OCR consent cancel
  const handleOcrConsentCancel = () => {
    setShowOcrConsentDialog(false);
    setUseOcrFallback(false);
  };

  // Get effective transaction type (with override)
  const getEffectiveTxnType = (idx: number, originalType: string) => {
    return txnTypeOverrides[idx] ?? originalType;
  };

  // Handle transaction type change
  const handleTxnTypeChange = (idx: number, newType: string) => {
    setTxnTypeOverrides(prev => ({
      ...prev,
      [idx]: newType,
    }));
  };

  // Change all transaction types at once
  const handleChangeAllTypes = (newType: string) => {
    if (!combinedPreview) return;
    const newOverrides: Record<number, string> = {};
    combinedPreview.transactions.forEach((_, idx) => {
      newOverrides[idx] = newType;
    });
    setTxnTypeOverrides(newOverrides);
  };

  // Get effective fee (with override)
  const getEffectiveFee = (idx: number, originalFee: number) => {
    return feeOverrides[idx] ?? originalFee;
  };

  // Handle fee change
  const handleFeeChange = (idx: number, newFee: number) => {
    setFeeOverrides(prev => ({
      ...prev,
      [idx]: newFee,
    }));
  };

  // Extract filename from path
  const getFileName = (filePath: string): string => {
    const parts = filePath.split(/[/\\]/);
    return parts[parts.length - 1] || filePath;
  };

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });

      if (selected && (Array.isArray(selected) ? selected.length > 0 : selected)) {
        const files = Array.isArray(selected) ? selected : [selected];
        setSelectedFiles(files);
        setIsLoading(true);
        setError(null);
        setOcrStatus(null);

        const allPreviews: Array<PdfImportPreview & { filePath: string; fileName: string }> = [];

        // Process each PDF
        const errors: string[] = [];
        for (let i = 0; i < files.length; i++) {
          const filePath = files[i];
          const fileName = getFileName(filePath);
          console.log(`[PDF Import] Processing ${i + 1}/${files.length}: ${fileName}`);
          setOcrStatus(`Analysiere ${fileName} (${i + 1}/${files.length})...`);

          // Allow UI to update
          await new Promise(resolve => setTimeout(resolve, 100));

          try {
            let previewData: PdfImportPreview;

            // Use OCR-enabled preview if option is selected and consent given
            if (useOcrFallback && hasOcrApiKey() && ocrConsentGiven) {
              console.log(`[PDF Import] Using OCR for ${fileName}`);
              setIsOcrActive(true);
              setOcrProgress({ current: i + 1, total: files.length });
              previewData = await invoke<PdfImportPreview>('preview_pdf_import_with_ocr', {
                pdfPath: filePath,
                useOcr: true,
                ocrProvider: aiProvider,
                ocrModel: aiModel,
                ocrApiKey: getOcrApiKey(),
                ocrConsentGiven: true,
              });
            } else {
              // Regular preview
              console.log(`[PDF Import] Using regular preview for ${fileName}`);
              previewData = await previewPdfImport(filePath);
              console.log(`[PDF Import] Received preview for ${fileName}:`, previewData.transactions.length, 'transactions');
            }

            allPreviews.push({
              ...previewData,
              filePath,
              fileName,
            });
          } catch (err) {
            console.error(`Failed to parse ${fileName}:`, err);
            errors.push(`${fileName}: ${err instanceof Error ? err.message : String(err)}`);
          }
        }

        // Show errors if any files failed
        if (errors.length > 0 && allPreviews.length === 0) {
          throw new Error(`Keine PDFs konnten verarbeitet werden:\n${errors.join('\n')}`);
        }

        // Add errors as warnings to show in preview
        if (errors.length > 0) {
          allPreviews[0] = {
            ...allPreviews[0],
            warnings: [...(allPreviews[0].warnings || []), ...errors.map(e => `Fehler: ${e}`)],
          };
        }

        setOcrStatus(null);
        setIsOcrActive(false);
        setOcrProgress(null);
        setPreviews(allPreviews);

        // Initialize portfolio selection per file with first available portfolio
        const defaultPortfolio = portfolios.find(p => !p.isRetired)?.id;
        if (defaultPortfolio) {
          const initialPortfolios: Record<number, number> = {};
          allPreviews.forEach((_, idx) => {
            initialPortfolios[idx] = defaultPortfolio;
          });
          setPortfolioPerFile(initialPortfolios);
        }

        // Combine all previews into one for display
        const combined: PdfImportPreview = {
          bank: allPreviews.map(p => p.bank).filter(Boolean).join(', ') || 'Unbekannt',
          transactions: allPreviews.flatMap(p =>
            p.transactions.map(txn => ({
              ...txn,
              // Add source info to security name for display
              _sourceFile: p.fileName,
              _sourceBank: p.bank,
            }))
          ),
          newSecurities: allPreviews.flatMap(p => p.newSecurities),
          matchedSecurities: allPreviews.flatMap(p => p.matchedSecurities || []),
          warnings: allPreviews.flatMap(p =>
            p.warnings.map(w => `[${p.fileName}] ${w}`)
          ),
          potentialDuplicates: allPreviews.flatMap(p => p.potentialDuplicates || []),
        };

        // Remove duplicate securities (by ISIN)
        const seenIsins = new Set<string>();
        combined.newSecurities = combined.newSecurities.filter(sec => {
          if (sec.isin && seenIsins.has(sec.isin)) return false;
          if (sec.isin) seenIsins.add(sec.isin);
          return true;
        });

        setCombinedPreview(combined);

        // If deliveryMode is active, automatically convert Buy → TransferIn
        if (deliveryMode && combined.transactions.length > 0) {
          const autoOverrides: Record<number, string> = {};
          combined.transactions.forEach((txn, idx) => {
            if (txn.txnType === 'Buy') {
              autoOverrides[idx] = 'TransferIn';
            }
          });
          if (Object.keys(autoOverrides).length > 0) {
            setTxnTypeOverrides(autoOverrides);
          }
        }

        // No step change - preview appears inline below upload zone
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setOcrStatus(null);
      setIsOcrActive(false);
      setOcrProgress(null);
    } finally {
      setIsLoading(false);
    }
  };

  const handleImport = async () => {
    // Check if all files have a portfolio selected
    const allFilesHavePortfolio = previews.every((_, idx) => portfolioPerFile[idx]);
    if (selectedFiles.length === 0 || !allFilesHavePortfolio || !selectedAccount) return;

    setStep('importing');
    setError(null);

    try {
      let totalTransactions = 0;
      let totalSkipped = 0;
      let totalSecurities = 0;
      const allErrors: string[] = [];
      const allWarnings: string[] = [];

      // Track transaction index offset for overrides
      let txnIndexOffset = 0;

      // Import each PDF
      for (let i = 0; i < previews.length; i++) {
        const preview = previews[i];
        const filePortfolio = portfolioPerFile[i];

        if (!filePortfolio) {
          allErrors.push(`[${preview.fileName}] Kein Portfolio ausgewählt`);
          continue;
        }

        setOcrStatus(`Importiere ${preview.fileName} (${i + 1}/${previews.length})...`);

        // Allow UI to update
        await new Promise(resolve => setTimeout(resolve, 50));

        // Get overrides for this file's transactions
        const fileTypeOverrides: Record<number, string> = {};
        const fileFeesOverrides: Record<number, number> = {};

        for (let j = 0; j < preview.transactions.length; j++) {
          const globalIdx = txnIndexOffset + j;
          if (txnTypeOverrides[globalIdx] !== undefined) {
            fileTypeOverrides[j] = txnTypeOverrides[globalIdx];
          }
          if (feeOverrides[globalIdx] !== undefined) {
            fileFeesOverrides[j] = feeOverrides[globalIdx];
          }
        }

        const typeOverrides = Object.keys(fileTypeOverrides).length > 0 ? fileTypeOverrides : undefined;
        const feesOverrides = Object.keys(fileFeesOverrides).length > 0 ? fileFeesOverrides : undefined;

        try {
          const result = await importPdfTransactions(
            preview.filePath,
            filePortfolio,
            selectedAccount,
            createMissingSecurities,
            skipDuplicates,
            typeOverrides,
            feesOverrides
          );

          totalTransactions += result.transactionsImported;
          totalSkipped += result.transactionsSkipped;
          totalSecurities += result.securitiesCreated;
          if (result.errors.length > 0) {
            allErrors.push(...result.errors.map(e => `[${preview.fileName}] ${e}`));
          }
          if (result.warnings.length > 0) {
            allWarnings.push(...result.warnings.map(w => `[${preview.fileName}] ${w}`));
          }
        } catch (err) {
          allErrors.push(`[${preview.fileName}] ${err instanceof Error ? err.message : String(err)}`);
        }

        txnIndexOffset += preview.transactions.length;
      }

      setOcrStatus(null);
      setImportResult({
        success: allErrors.length === 0 || totalTransactions > 0,
        transactionsImported: totalTransactions,
        transactionsSkipped: totalSkipped,
        securitiesCreated: totalSecurities,
        errors: allErrors,
        warnings: allWarnings,
      });
      setStep('done');

      if (totalTransactions > 0 && onSuccess) {
        onSuccess();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setOcrStatus(null);
      setStep('preview');
    }
  };

  const formatCurrency = (amount: number | undefined | null, currency: string) => {
    try {
      if (amount === undefined || amount === null || isNaN(amount)) {
        console.warn('PdfImportModal: Invalid amount:', amount);
        return `- ${currency}`;
      }
      return `${amount.toLocaleString('de-DE', { minimumFractionDigits: 2 })} ${currency}`;
    } catch (err) {
      console.error('PdfImportModal: Currency formatting error:', err);
      return `${amount} ${currency}`;
    }
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
          <ErrorBoundary onError={(err) => console.error('PdfImportModal ErrorBoundary caught:', err)}>
          {error && (
            <div className="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm flex items-center gap-2">
              <AlertCircle size={16} />
              {error}
            </div>
          )}

          {/* Single-Page Layout: Upload + Preview */}
          {(step === 'select' || step === 'preview') && (
            <div className="space-y-2">
              {/* Compact Upload Zone */}
              <div
                onClick={handleSelectFile}
                className={`border-2 border-dashed border-border rounded-lg text-center cursor-pointer hover:border-primary hover:bg-muted/50 transition-colors ${
                  combinedPreview ? 'py-2 px-4' : 'p-8'
                }`}
              >
                {isLoading ? (
                  <div className="flex flex-col items-center justify-center gap-2 py-2">
                    {isOcrActive ? (
                      <>
                        {/* KI-Aktivitätsanzeige */}
                        <div className="flex items-center gap-3">
                          <div className="relative">
                            <AIProviderLogo provider={aiProvider as AiProvider} size={24} />
                            <div className="absolute -bottom-1 -right-1 w-3 h-3 bg-primary rounded-full animate-pulse" />
                          </div>
                          <div className="text-left">
                            <div className="text-sm font-medium flex items-center gap-2">
                              <span>KI analysiert PDF</span>
                              <Loader2 className="w-4 h-4 animate-spin text-primary" />
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {aiProvider === 'claude' ? 'Claude' :
                               aiProvider === 'openai' ? 'OpenAI' :
                               aiProvider === 'gemini' ? 'Gemini' : 'Perplexity'} • {aiModel}
                            </div>
                          </div>
                        </div>
                        {/* Fortschrittsanzeige */}
                        {ocrProgress && ocrProgress.total > 1 && (
                          <div className="w-full max-w-xs">
                            <div className="flex justify-between text-xs text-muted-foreground mb-1">
                              <span>Datei {ocrProgress.current} von {ocrProgress.total}</span>
                              <span>{Math.round((ocrProgress.current / ocrProgress.total) * 100)}%</span>
                            </div>
                            <div className="h-1.5 bg-muted rounded-full overflow-hidden">
                              <div
                                className="h-full bg-primary rounded-full transition-all duration-300"
                                style={{ width: `${(ocrProgress.current / ocrProgress.total) * 100}%` }}
                              />
                            </div>
                          </div>
                        )}
                        {/* Provider-spezifischer Hinweis */}
                        <div className="text-xs text-muted-foreground">
                          {providerSupportsDirectPdf
                            ? 'Direkter PDF-Upload (schneller)'
                            : 'PDF → Bilder → Vision API'}
                        </div>
                      </>
                    ) : (
                      <div className="flex items-center gap-3">
                        <Loader2 className="w-5 h-5 text-primary animate-spin" />
                        <span className="text-sm font-medium">{ocrStatus || 'PDF wird analysiert...'}</span>
                      </div>
                    )}
                  </div>
                ) : combinedPreview ? (
                  <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
                    <Upload size={16} />
                    <span>Weitere PDFs hinzufügen oder neu auswählen</span>
                  </div>
                ) : (
                  <>
                    <Upload className="w-10 h-10 mx-auto mb-3 text-muted-foreground" />
                    <p className="font-medium mb-1">PDF-Dateien auswählen</p>
                    <p className="text-sm text-muted-foreground">
                      Klicken oder Dateien hierher ziehen
                    </p>
                  </>
                )}
              </div>

              {/* OCR Options & Banks - Only show before PDFs are loaded */}
              {!combinedPreview && !isLoading && (
                <details className="border border-border rounded-lg">
                  <summary className="px-4 py-2 cursor-pointer text-sm font-medium flex items-center gap-2">
                    <ScanText size={14} />
                    Erweiterte Optionen
                  </summary>
                  <div className="px-4 pb-4 space-y-4">
                    {/* OCR Option */}
                    <div className="space-y-2">
                      <label className="flex items-center justify-between cursor-pointer text-sm">
                        <div className="flex-1">
                          <span className="font-medium">OCR für gescannte PDFs</span>
                          <span className="text-xs text-muted-foreground ml-2">
                            (KI-Texterkennung)
                          </span>
                        </div>
                        <input
                          type="checkbox"
                          checked={useOcrFallback}
                          onChange={(e) => handleOcrToggle(e.target.checked)}
                          disabled={!aiEnabled || !hasOcrApiKey() || (needsPoppler && !isOcrAvailable) || !modelHasVision}
                          className="rounded border-border"
                        />
                      </label>

                      {/* OCR Status & Warnings */}
                      {aiEnabled && hasOcrApiKey() && (
                        <div className="pl-0 space-y-1.5">
                          {/* Current Configuration */}
                          <div className="flex items-center gap-2 text-xs text-muted-foreground">
                            <AIProviderLogo provider={aiProvider as AiProvider} size={14} />
                            <span>{aiModel}</span>
                            {ocrConsentGiven && (
                              <span className="text-green-600 font-medium">✓ Zustimmung erteilt</span>
                            )}
                          </div>

                          {/* Vision Warning */}
                          {!modelHasVision && !isCheckingVision && (
                            <div className="flex items-start gap-2 p-2 bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-800 rounded text-xs">
                              <AlertTriangle size={14} className="text-amber-600 shrink-0 mt-0.5" />
                              <div>
                                <span className="font-medium text-amber-800 dark:text-amber-200">
                                  Modell unterstützt keine Bilder
                                </span>
                                <p className="text-amber-700 dark:text-amber-300 mt-0.5">
                                  {aiModel} hat keine Vision-Unterstützung. Wähle in den Einstellungen ein Vision-fähiges Modell für PDF OCR.
                                </p>
                              </div>
                            </div>
                          )}

                          {/* Poppler Warning for OpenAI/Perplexity */}
                          {needsPoppler && !isOcrAvailable && modelHasVision && (
                            <div className="flex items-start gap-2 p-2 bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-800 rounded text-xs">
                              <AlertTriangle size={14} className="text-amber-600 shrink-0 mt-0.5" />
                              <div>
                                <span className="font-medium text-amber-800 dark:text-amber-200">
                                  Poppler nicht installiert
                                </span>
                                <p className="text-amber-700 dark:text-amber-300 mt-0.5">
                                  {aiProvider === 'openai' ? 'OpenAI' : 'Perplexity'} benötigt Poppler für PDF-zu-Bild-Konvertierung:
                                </p>
                                <code className="block mt-1 p-1.5 bg-amber-100 dark:bg-amber-900/50 rounded text-amber-800 dark:text-amber-200 font-mono">
                                  {navigator.platform.includes('Mac') ? 'brew install poppler' :
                                   navigator.platform.includes('Win') ? 'choco install poppler' :
                                   'sudo apt install poppler-utils'}
                                </code>
                                <p className="text-amber-700 dark:text-amber-300 mt-1">
                                  <strong>Tipp:</strong> Claude oder Gemini unterstützen direkten PDF-Upload ohne Poppler.
                                </p>
                              </div>
                            </div>
                          )}

                          {/* Direct PDF Upload Info for Claude/Gemini */}
                          {providerSupportsDirectPdf && modelHasVision && (
                            <div className="flex items-center gap-2 text-xs text-green-600">
                              <CheckCircle size={12} />
                              <span>Direkter PDF-Upload (kein Poppler nötig)</span>
                            </div>
                          )}
                        </div>
                      )}

                      {/* KI Disabled Warning */}
                      {!aiEnabled && (
                        <div className="text-xs text-amber-600">
                          KI ist deaktiviert. Aktiviere KI in den Einstellungen.
                        </div>
                      )}

                      {/* No API Key Warning */}
                      {aiEnabled && !hasOcrApiKey() && (
                        <div className="text-xs text-amber-600">
                          Kein API-Key für {aiProvider} hinterlegt. Konfiguriere den Key in den Einstellungen.
                        </div>
                      )}
                    </div>
                    {/* Supported Banks - Grouped by Region */}
                    <div>
                      <div className="text-xs text-muted-foreground mb-2">
                        {supportedBanks.length} unterstützte Banken & Broker:
                      </div>
                      <div className="grid grid-cols-2 gap-3 text-xs">
                        <div>
                          <div className="font-medium text-muted-foreground mb-1">🇩🇪 Deutschland</div>
                          <div className="text-muted-foreground leading-relaxed">
                            {supportedBanks.slice(0, 24).map(b => b.name).join(', ')}
                          </div>
                        </div>
                        <div className="space-y-2">
                          <div>
                            <div className="font-medium text-muted-foreground mb-1">🇨🇭 Schweiz</div>
                            <div className="text-muted-foreground">
                              {supportedBanks.slice(24, 30).map(b => b.name).join(', ')}
                            </div>
                          </div>
                          <div>
                            <div className="font-medium text-muted-foreground mb-1">🇦🇹 Österreich</div>
                            <div className="text-muted-foreground">
                              {supportedBanks.slice(30, 32).map(b => b.name).join(', ')}
                            </div>
                          </div>
                          <div>
                            <div className="font-medium text-muted-foreground mb-1">🌍 International</div>
                            <div className="text-muted-foreground">
                              {supportedBanks.slice(32).map(b => b.name).join(', ')}
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </details>
              )}

              {/* Preview Content - Shows immediately after PDFs are loaded */}
              {combinedPreview && (
                <>
              {/* Compact Summary - single line */}
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">{selectedFiles.length} Dateien</span>
                <span>•</span>
                <span>{combinedPreview.bank}</span>
                <span>•</span>
                <span>{combinedPreview.transactions.length} Transaktionen</span>
                {combinedPreview.potentialDuplicates && combinedPreview.potentialDuplicates.length > 0 && (
                  <span className="text-orange-600 font-medium">• {combinedPreview.potentialDuplicates.length} Duplikate</span>
                )}
                {combinedPreview.newSecurities.length > 0 && (
                  <span>• {combinedPreview.newSecurities.length} neue</span>
                )}
              </div>

              {/* Files with Portfolio Selection - Compact */}
              <div className="space-y-1">
                {previews.map((p, idx) => (
                  <div key={idx} className="flex items-center gap-2 text-sm">
                    <FileText size={14} className="text-muted-foreground flex-shrink-0" />
                    <span className="truncate flex-1 min-w-0" title={p.fileName}>{p.fileName}</span>
                    <span className="text-xs text-muted-foreground flex-shrink-0">
                      {p.bank && `${p.bank} •`} {p.transactions.length}
                    </span>
                    <span className="text-muted-foreground">→</span>
                    <select
                      value={portfolioPerFile[idx] || ''}
                      onChange={(e) => setPortfolioPerFile(prev => ({ ...prev, [idx]: Number(e.target.value) }))}
                      className="px-2 py-0.5 text-xs border border-border rounded bg-background w-28"
                    >
                      <option value="">Portfolio...</option>
                      {portfolios.map(p => (
                        <option key={p.id} value={p.id}>{p.name}</option>
                      ))}
                    </select>
                  </div>
                ))}
              </div>

              {/* Account & Options - Single row */}
              <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
                <div className="flex items-center gap-1.5">
                  <span>Konto:</span>
                  <select
                    value={selectedAccount || ''}
                    onChange={(e) => setSelectedAccount(Number(e.target.value))}
                    className="px-1.5 py-0.5 text-xs border border-border rounded bg-background"
                  >
                    <option value="">Wählen...</option>
                    {accounts.map(a => (
                      <option key={a.id} value={a.id}>{a.name}</option>
                    ))}
                  </select>
                </div>
                <label className="flex items-center gap-1 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={createMissingSecurities}
                    onChange={(e) => setCreateMissingSecurities(e.target.checked)}
                    className="rounded border-border w-3 h-3"
                  />
                  <span>Wertpapiere anlegen</span>
                </label>
                {combinedPreview && combinedPreview.potentialDuplicates && combinedPreview.potentialDuplicates.length > 0 && (
                  <label className="flex items-center gap-1 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={skipDuplicates}
                      onChange={(e) => setSkipDuplicates(e.target.checked)}
                      className="rounded border-border w-3 h-3"
                    />
                    <span>Duplikate überspringen</span>
                  </label>
                )}
              </div>

              {/* Warnings - Compact inline */}
              {combinedPreview.warnings.length > 0 && (
                <div className="flex items-start gap-2 text-xs bg-amber-500/5 border border-amber-500/20 rounded-lg px-3 py-2">
                  <AlertTriangle size={14} className="text-amber-600 flex-shrink-0 mt-0.5" />
                  <span className="text-amber-600">
                    {combinedPreview.warnings.slice(0, 3).join(' · ')}
                    {combinedPreview.warnings.length > 3 && ` (+${combinedPreview.warnings.length - 3} weitere)`}
                  </span>
                </div>
              )}

              {/* Potential Duplicates - Compact inline */}
              {combinedPreview.potentialDuplicates && combinedPreview.potentialDuplicates.length > 0 && (
                <div className="flex items-start gap-2 text-xs bg-orange-500/5 border border-orange-500/20 rounded-lg px-3 py-2">
                  <AlertCircle size={14} className="text-orange-600 flex-shrink-0 mt-0.5" />
                  <div className="flex-1 min-w-0">
                    <span className="text-orange-600 font-medium">{combinedPreview.potentialDuplicates.length} mögliche Duplikate: </span>
                    <span className="text-orange-600">
                      {combinedPreview.potentialDuplicates.map((dup, idx) => (
                        <span key={idx}>
                          {idx > 0 && ' · '}
                          {formatDate(dup.date)} {getTxnTypeLabel(dup.txnType)} {dup.securityName || 'Unbekannt'} {formatCurrency(dup.amount, 'EUR')}
                        </span>
                      ))}
                    </span>
                  </div>
                </div>
              )}

              {/* Transactions Preview - Compact */}
              <div>
                <div className="flex items-center justify-between mb-1">
                  <span className="font-medium text-xs">Transaktionen</span>
                  <select
                    onChange={(e) => {
                      if (e.target.value) {
                        handleChangeAllTypes(e.target.value);
                        e.target.value = '';
                      }
                    }}
                    className="px-1.5 py-0.5 text-xs border border-border rounded bg-background"
                    defaultValue=""
                  >
                    <option value="" disabled>Alle: Typ...</option>
                    {TXN_TYPE_OPTIONS.map(opt => (
                      <option key={opt.value} value={opt.value}>{opt.label}</option>
                    ))}
                  </select>
                </div>
                <div className="border border-border rounded-lg overflow-hidden">
                  <div className="overflow-y-auto max-h-48">
                    <table className="w-full text-sm">
                      <thead className="bg-muted sticky top-0">
                        <tr>
                          <th className="text-left py-1.5 px-3 font-medium text-xs">Datum</th>
                          <th className="text-left py-1.5 px-3 font-medium text-xs">Typ</th>
                          <th className="text-left py-1.5 px-3 font-medium text-xs">Wertpapier</th>
                          <th className="text-right py-1.5 px-3 font-medium text-xs">Betrag</th>
                          <th className="text-right py-1.5 px-3 font-medium text-xs w-16">Gebühr</th>
                        </tr>
                      </thead>
                      <tbody>
                        {combinedPreview.transactions.map((txn, idx) => {
                          const effectiveType = getEffectiveTxnType(idx, txn.txnType);
                          return (
                          <tr key={idx} className="border-t border-border">
                            <td className="py-1.5 px-3 text-xs">{formatDate(txn.date)}</td>
                            <td className="py-1.5 px-3">
                              <select
                                value={effectiveType}
                                onChange={(e) => handleTxnTypeChange(idx, e.target.value)}
                                className={`px-1.5 py-0.5 rounded text-xs border-0 cursor-pointer ${
                                  effectiveType === 'Buy' || effectiveType === 'TransferIn' ? 'bg-green-500/10 text-green-600' :
                                  effectiveType === 'Sell' || effectiveType === 'TransferOut' ? 'bg-red-500/10 text-red-600' :
                                  effectiveType === 'Dividend' ? 'bg-blue-500/10 text-blue-600' :
                                  'bg-muted text-muted-foreground'
                                }`}
                              >
                                {TXN_TYPE_OPTIONS.map(opt => (
                                  <option key={opt.value} value={opt.value}>{opt.label}</option>
                                ))}
                              </select>
                            </td>
                            <td className="py-1.5 px-3">
                              <span className="font-medium text-sm">{txn.securityName || '-'}</span>
                              {txn.isin && <span className="text-xs text-muted-foreground ml-1">{txn.isin}</span>}
                            </td>
                            <td className="py-1.5 px-3 text-right font-medium text-sm">
                              {formatCurrency(txn.netAmount, txn.currency)}
                            </td>
                            <td className="py-1.5 px-3 text-right">
                              <input
                                type="number"
                                step="0.01"
                                min="0"
                                value={getEffectiveFee(idx, txn.fees)}
                                onChange={(e) => handleFeeChange(idx, parseFloat(e.target.value) || 0)}
                                className="w-14 px-1 py-0.5 text-right text-xs border border-border rounded bg-background"
                              />
                            </td>
                          </tr>
                        )})}
                      </tbody>
                    </table>
                  </div>
                </div>
              </div>

              {/* New Securities - Compact inline */}
              {combinedPreview.newSecurities.length > 0 && (
                <div className="text-xs text-muted-foreground">
                  <span className="font-medium">{combinedPreview.newSecurities.length} neue Wertpapiere:</span>{' '}
                  {combinedPreview.newSecurities.map((sec, idx) => (
                    <span key={idx}>
                      {idx > 0 && ', '}
                      {sec.name} <span className="opacity-60">({sec.isin || sec.wkn})</span>
                    </span>
                  ))}
                </div>
              )}
              </>
              )}
            </div>
          )}

          {/* Step: Importing */}
          {step === 'importing' && (
            <div className="text-center py-12">
              <Loader2 className="w-12 h-12 mx-auto mb-4 text-primary animate-spin" />
              <p className="text-lg font-medium">
                {ocrStatus || 'Transaktionen werden importiert...'}
              </p>
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
                    {selectedFiles.length > 1 && `${selectedFiles.length} PDFs verarbeitet. `}
                    {importResult.transactionsImported} Transaktionen wurden importiert.
                    {importResult.transactionsSkipped > 0 && (
                      <> {importResult.transactionsSkipped} Duplikate wurden übersprungen.</>
                    )}
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

              {importResult.warnings.length > 0 && (
                <div className="text-left bg-yellow-500/10 border border-yellow-500/20 rounded-lg p-4 mt-4">
                  <h4 className="font-medium text-yellow-600 dark:text-yellow-500 mb-2">Hinweise</h4>
                  <ul className="text-sm text-yellow-600 dark:text-yellow-500 space-y-1 max-h-32 overflow-y-auto">
                    {importResult.warnings.map((warning, idx) => (
                      <li key={idx}>{warning}</li>
                    ))}
                  </ul>
                </div>
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
          </ErrorBoundary>
        </div>

        {/* Footer */}
        <div className="flex justify-between p-4 border-t border-border">
          <button
            onClick={step === 'done' ? onClose : combinedPreview ? () => { setPreviews([]); setCombinedPreview(null); setSelectedFiles([]); } : onClose}
            className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            {step === 'done' ? 'Schließen' : combinedPreview ? 'Neu starten' : 'Abbrechen'}
          </button>

          {/* Show import button when preview is available */}
          {combinedPreview && step !== 'importing' && step !== 'done' && (
            <button
              onClick={handleImport}
              disabled={!previews.every((_, idx) => portfolioPerFile[idx]) || !selectedAccount}
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

      {/* OCR Consent Dialog */}
      {showOcrConsentDialog && (
        <div className="fixed inset-0 bg-black/70 flex items-center justify-center z-[60]">
          <div className="bg-background rounded-lg shadow-xl w-full max-w-md p-6">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 rounded-full bg-amber-500/10">
                <AlertTriangle className="w-6 h-6 text-amber-600" />
              </div>
              <h3 className="text-lg font-semibold">Datenschutz-Hinweis</h3>
            </div>

            <div className="space-y-3 text-sm text-muted-foreground mb-6">
              <p>
                Bei Aktivierung der OCR-Funktion werden Ihre PDF-Dokumente an einen
                <strong className="text-foreground"> externen KI-Dienst ({aiProvider === 'claude' ? 'Anthropic' : aiProvider === 'openai' ? 'OpenAI' : aiProvider === 'gemini' ? 'Google' : 'Perplexity'})</strong> übermittelt.
              </p>
              <p>
                Ihre PDFs können <strong className="text-foreground">sensible Finanzdaten</strong> enthalten wie:
              </p>
              <ul className="list-disc list-inside pl-2 space-y-1">
                <li>Kontonummern und Depotnummern</li>
                <li>Transaktionsdetails und Beträge</li>
                <li>Persönliche Angaben (Name, Adresse)</li>
              </ul>
              <p className="text-amber-600 font-medium">
                Diese Daten werden zur Texterkennung an den KI-Anbieter übertragen.
              </p>
            </div>

            <div className="flex gap-3 justify-end">
              <button
                onClick={handleOcrConsentCancel}
                className="px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
              >
                Abbrechen
              </button>
              <button
                onClick={handleOcrConsentConfirm}
                className="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
              >
                Zustimmen & OCR aktivieren
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
