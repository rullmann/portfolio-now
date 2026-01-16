/**
 * CSV Import Modal for importing transactions from broker CSV exports.
 * Supports manual column mapping and broker template auto-detection.
 */

import { useState, useEffect } from 'react';
import {
  X,
  Upload,
  FileSpreadsheet,
  AlertCircle,
  CheckCircle,
  Loader2,
  ChevronRight,
  ChevronLeft,
  Info,
  Sparkles,
} from 'lucide-react';
import { open } from '@tauri-apps/plugin-dialog';
import { ErrorBoundary } from '../common/ErrorBoundary';
import {
  previewCsv,
  importTransactionsCsv,
  getPortfolios,
  detectCsvBroker,
  getBrokerTemplates,
  importCsvWithTemplate,
  analyzeCsvWithAi,
} from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';
import { useSettingsStore } from '../../store';
import type {
  CsvPreview,
  CsvColumnMapping,
  CsvImportResult,
  PortfolioData,
  BrokerDetectionResult,
  BrokerTemplateSummary,
  AiCsvAnalysisResponse,
} from '../../lib/types';

interface CsvImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

type Step = 'select' | 'map' | 'preview' | 'importing' | 'done';

// Fields that can be mapped
const MAPPING_FIELDS = [
  { key: 'date', label: 'Datum', required: true },
  { key: 'txnType', label: 'Typ', required: false },
  { key: 'isin', label: 'ISIN', required: false },
  { key: 'securityName', label: 'Wertpapier', required: false },
  { key: 'shares', label: 'Stück', required: false },
  { key: 'amount', label: 'Betrag', required: true },
  { key: 'currency', label: 'Währung', required: false },
  { key: 'fees', label: 'Gebühren', required: false },
  { key: 'taxes', label: 'Steuern', required: false },
  { key: 'note', label: 'Notiz', required: false },
] as const;

type MappingKey = (typeof MAPPING_FIELDS)[number]['key'];

export function CsvImportModal({ isOpen, onClose, onSuccess }: CsvImportModalProps) {
  // ESC key to close
  useEscapeKey(isOpen, onClose);

  // AI settings from store
  const { aiProvider, aiModel, anthropicApiKey, openaiApiKey, geminiApiKey, perplexityApiKey } = useSettingsStore();

  const [step, setStep] = useState<Step>('select');
  const [filePath, setFilePath] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string>('');
  const [csvContent, setCsvContent] = useState<string>('');
  const [preview, setPreview] = useState<CsvPreview | null>(null);
  const [mapping, setMapping] = useState<CsvColumnMapping>({});
  const [portfolios, setPortfolios] = useState<PortfolioData[]>([]);
  const [selectedPortfolio, setSelectedPortfolio] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [importResult, setImportResult] = useState<CsvImportResult | null>(null);

  // Broker template detection
  const [brokerTemplates, setBrokerTemplates] = useState<BrokerTemplateSummary[]>([]);
  const [detectedBroker, setDetectedBroker] = useState<BrokerDetectionResult | null>(null);
  const [selectedTemplate, setSelectedTemplate] = useState<string>('manual');
  const [useTemplate, setUseTemplate] = useState(false);

  // AI fallback (Code-first, AI as helper)
  const [isAiAnalyzing, setIsAiAnalyzing] = useState(false);
  const [aiAnalysis, setAiAnalysis] = useState<AiCsvAnalysisResponse | null>(null);

  // Get API key for current provider
  const getApiKey = () => {
    switch (aiProvider) {
      case 'claude': return anthropicApiKey;
      case 'openai': return openaiApiKey;
      case 'gemini': return geminiApiKey;
      case 'perplexity': return perplexityApiKey;
      default: return '';
    }
  };

  const hasAiConfigured = !!getApiKey();

  useEffect(() => {
    if (isOpen) {
      loadInitialData();
    } else {
      resetState();
    }
  }, [isOpen]);

  const loadInitialData = async () => {
    try {
      const [portfolioList, templates] = await Promise.all([
        getPortfolios(),
        getBrokerTemplates().catch(() => []),
      ]);
      setPortfolios(portfolioList.filter(p => !p.isRetired));
      setBrokerTemplates(templates);

      // Set default portfolio
      const defaultPortfolio = portfolioList.find(p => !p.isRetired);
      if (defaultPortfolio) {
        setSelectedPortfolio(defaultPortfolio.id);
      }
    } catch (err) {
      console.error('Failed to load initial data:', err);
    }
  };

  const resetState = () => {
    setStep('select');
    setFilePath(null);
    setFileName('');
    setCsvContent('');
    setPreview(null);
    setMapping({});
    setError(null);
    setImportResult(null);
    setDetectedBroker(null);
    setSelectedTemplate('manual');
    setUseTemplate(false);
    setAiAnalysis(null);
    setIsAiAnalyzing(false);
  };

  // Extract filename from path
  const getFileName = (path: string): string => {
    const parts = path.split(/[/\\]/);
    return parts[parts.length - 1] || path;
  };

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'CSV', extensions: ['csv', 'txt'] }],
      });

      if (selected && typeof selected === 'string') {
        setFilePath(selected);
        setFileName(getFileName(selected));
        setIsLoading(true);
        setError(null);

        // Preview the CSV
        const previewData = await previewCsv(selected);
        setPreview(previewData);

        // Build CSV content for AI analysis (header + first rows)
        const headerLine = previewData.columns.map(c => c.name).join(previewData.delimiter);
        const dataLines = Array.from({ length: Math.min(10, previewData.columns[0]?.sampleValues.length || 0) })
          .map((_, rowIdx) => previewData.columns.map(c => c.sampleValues[rowIdx] || '').join(previewData.delimiter));
        setCsvContent([headerLine, ...dataLines].join('\n'));

        // Try to detect broker
        try {
          const detection = await detectCsvBroker(selected);
          setDetectedBroker(detection);

          if (detection.confidence >= 0.8 && detection.templateId) {
            setSelectedTemplate(detection.templateId);
            setUseTemplate(true);
          }
        } catch {
          // Broker detection not available or failed - continue with manual mapping
          setDetectedBroker(null);
        }

        // Auto-map columns based on header names
        autoMapColumns(previewData);

        setStep('map');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  // Try to auto-map columns based on common header names
  const autoMapColumns = (previewData: CsvPreview) => {
    const newMapping: CsvColumnMapping = {};
    const headerPatterns: Record<MappingKey, RegExp[]> = {
      date: [/datum/i, /date/i, /buchung/i, /valuta/i],
      txnType: [/typ/i, /type/i, /art/i, /aktion/i, /action/i],
      isin: [/isin/i],
      securityName: [/wertpapier/i, /security/i, /name/i, /produkt/i, /product/i, /titel/i],
      shares: [/stück/i, /anzahl/i, /shares/i, /quantity/i, /menge/i],
      amount: [/betrag/i, /amount/i, /wert/i, /value/i, /summe/i],
      currency: [/währung/i, /currency/i],
      fees: [/gebühr/i, /fee/i, /provision/i, /kosten/i],
      taxes: [/steuer/i, /tax/i],
      note: [/notiz/i, /note/i, /bemerkung/i, /comment/i],
    };

    previewData.columns.forEach((col, index) => {
      const headerLower = col.name.toLowerCase();

      for (const [field, patterns] of Object.entries(headerPatterns)) {
        if (patterns.some(p => p.test(headerLower))) {
          // Don't override if already mapped
          if (newMapping[field as MappingKey] === undefined) {
            newMapping[field as MappingKey] = index;
          }
        }
      }
    });

    setMapping(newMapping);
  };

  const handleMappingChange = (field: MappingKey, value: string) => {
    setMapping(prev => ({
      ...prev,
      [field]: value === '' ? undefined : parseInt(value, 10),
    }));
  };

  // AI-assisted analysis (Code-first, AI as fallback)
  const handleAiAnalysis = async () => {
    if (!csvContent || !hasAiConfigured) return;

    setIsAiAnalyzing(true);
    setError(null);

    try {
      const result = await analyzeCsvWithAi(csvContent, aiProvider, aiModel, getApiKey());
      setAiAnalysis(result);

      // Apply AI suggestions if we have them
      if (result.mappingSuggestions.length > 0) {
        applyAiSuggestions(result);
      }
    } catch (err) {
      setError(`KI-Analyse fehlgeschlagen: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsAiAnalyzing(false);
    }
  };

  // Apply AI mapping suggestions
  const applyAiSuggestions = (analysis: AiCsvAnalysisResponse) => {
    const newMapping: CsvColumnMapping = { ...mapping };

    for (const suggestion of analysis.mappingSuggestions) {
      if (suggestion.columnIndex !== undefined && suggestion.confidence >= 0.5) {
        const field = suggestion.field as MappingKey;
        if (MAPPING_FIELDS.some(f => f.key === field)) {
          newMapping[field] = suggestion.columnIndex;
        }
      }
    }

    setMapping(newMapping);
  };

  const handleImport = async () => {
    if (!filePath || !selectedPortfolio) return;

    setStep('importing');
    setError(null);

    try {
      let result: CsvImportResult;

      if (useTemplate && selectedTemplate !== 'manual') {
        // Use template-based import
        result = await importCsvWithTemplate(filePath, selectedTemplate, selectedPortfolio);
      } else {
        // Use manual mapping
        result = await importTransactionsCsv(filePath, mapping, selectedPortfolio);
      }

      setImportResult(result);
      setStep('done');

      if (result.rowsImported > 0 && onSuccess) {
        onSuccess();
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStep('map');
    }
  };

  // Check if required fields are mapped
  const hasRequiredMappings = () => {
    if (useTemplate && selectedTemplate !== 'manual') {
      return true; // Template handles mapping
    }
    return mapping.date !== undefined && mapping.amount !== undefined;
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg shadow-lg w-full max-w-4xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2">
            <FileSpreadsheet className="w-5 h-5 text-primary" />
            <h2 className="text-lg font-semibold">CSV Import</h2>
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
          <ErrorBoundary onError={(err) => console.error('CsvImportModal ErrorBoundary caught:', err)}>
            {error && (
              <div className="mb-4 p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm flex items-center gap-2">
                <AlertCircle size={16} />
                {error}
              </div>
            )}

            {/* Step: Select File */}
            {step === 'select' && (
              <div className="space-y-4">
                <div
                  onClick={handleSelectFile}
                  className="border-2 border-dashed border-border rounded-lg p-8 text-center cursor-pointer hover:border-primary hover:bg-muted/50 transition-colors"
                >
                  {isLoading ? (
                    <div className="flex items-center justify-center gap-3">
                      <Loader2 className="w-5 h-5 text-primary animate-spin" />
                      <span className="text-sm font-medium">CSV wird analysiert...</span>
                    </div>
                  ) : (
                    <>
                      <Upload className="w-10 h-10 mx-auto mb-3 text-muted-foreground" />
                      <p className="font-medium mb-1">CSV-Datei auswählen</p>
                      <p className="text-sm text-muted-foreground">
                        Klicken, um eine CSV-Datei auszuwählen
                      </p>
                    </>
                  )}
                </div>

                {/* Supported Brokers */}
                {brokerTemplates.length > 0 && (
                  <details className="border border-border rounded-lg">
                    <summary className="px-4 py-2 cursor-pointer text-sm font-medium">
                      Unterstützte Broker-Formate
                    </summary>
                    <div className="px-4 pb-4">
                      <div className="flex flex-wrap gap-2 mt-2">
                        {brokerTemplates.map(template => (
                          <span
                            key={template.id}
                            className="px-2 py-1 bg-muted rounded text-xs"
                          >
                            {template.name}
                          </span>
                        ))}
                      </div>
                    </div>
                  </details>
                )}
              </div>
            )}

            {/* Step: Map Columns */}
            {step === 'map' && preview && (
              <div className="space-y-4">
                {/* File Info */}
                <div className="flex items-center gap-2 text-sm">
                  <FileSpreadsheet size={16} className="text-muted-foreground" />
                  <span className="font-medium">{fileName}</span>
                  <span className="text-muted-foreground">
                    ({preview.rowCount} Zeilen, Trennzeichen: {preview.delimiter === ';' ? 'Semikolon' : preview.delimiter === ',' ? 'Komma' : 'Tab'})
                  </span>
                </div>

                {/* Broker Detection Result */}
                {detectedBroker && detectedBroker.confidence >= 0.5 && (
                  <div className={`p-3 rounded-lg border ${detectedBroker.confidence >= 0.8 ? 'bg-green-500/10 border-green-500/20' : 'bg-blue-500/10 border-blue-500/20'}`}>
                    <div className="flex items-center gap-2">
                      <Info size={16} className={detectedBroker.confidence >= 0.8 ? 'text-green-600' : 'text-blue-600'} />
                      <span className={`font-medium ${detectedBroker.confidence >= 0.8 ? 'text-green-600' : 'text-blue-600'}`}>
                        {detectedBroker.brokerName} erkannt ({Math.round(detectedBroker.confidence * 100)}% Konfidenz)
                      </span>
                    </div>
                    <div className="mt-2 flex items-center gap-4">
                      <label className="flex items-center gap-2 cursor-pointer text-sm">
                        <input
                          type="radio"
                          checked={useTemplate}
                          onChange={() => setUseTemplate(true)}
                          className="rounded border-border"
                        />
                        <span>Template verwenden</span>
                      </label>
                      <label className="flex items-center gap-2 cursor-pointer text-sm">
                        <input
                          type="radio"
                          checked={!useTemplate}
                          onChange={() => setUseTemplate(false)}
                          className="rounded border-border"
                        />
                        <span>Manuell zuordnen</span>
                      </label>
                    </div>
                  </div>
                )}

                {/* Template Selection (if available but not auto-detected) */}
                {brokerTemplates.length > 0 && (!detectedBroker || detectedBroker.confidence < 0.5) && (
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-muted-foreground">Broker-Template:</span>
                    <select
                      value={selectedTemplate}
                      onChange={(e) => {
                        setSelectedTemplate(e.target.value);
                        setUseTemplate(e.target.value !== 'manual');
                      }}
                      className="px-2 py-1 text-sm border border-border rounded bg-background"
                    >
                      <option value="manual">Manuelles Mapping</option>
                      <optgroup label="Broker-Templates">
                        {brokerTemplates.map(t => (
                          <option key={t.id} value={t.id}>{t.name}</option>
                        ))}
                      </optgroup>
                    </select>
                  </div>
                )}

                {/* AI Fallback - Code first, AI as helper */}
                {!useTemplate && hasAiConfigured && (!detectedBroker || detectedBroker.confidence < 0.8) && (
                  <div className="p-3 rounded-lg border border-purple-500/20 bg-purple-500/10">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <Sparkles size={16} className="text-purple-600" />
                        <span className="text-sm font-medium text-purple-600">
                          KI-Unterstützung
                        </span>
                        <span className="text-xs text-muted-foreground">
                          (Format nicht automatisch erkannt)
                        </span>
                      </div>
                      <button
                        onClick={handleAiAnalysis}
                        disabled={isAiAnalyzing}
                        className="flex items-center gap-2 px-3 py-1 text-xs bg-purple-600 text-white rounded-md hover:bg-purple-700 transition-colors disabled:opacity-50"
                      >
                        {isAiAnalyzing ? (
                          <>
                            <Loader2 size={12} className="animate-spin" />
                            Analysiere...
                          </>
                        ) : (
                          <>
                            <Sparkles size={12} />
                            KI analysieren lassen
                          </>
                        )}
                      </button>
                    </div>

                    {/* AI Analysis Results */}
                    {aiAnalysis && (
                      <div className="mt-3 space-y-2">
                        {aiAnalysis.detectedBroker && (
                          <div className="text-sm">
                            <span className="text-muted-foreground">Erkannter Broker: </span>
                            <span className="font-medium">{aiAnalysis.detectedBroker}</span>
                            <span className="text-xs text-muted-foreground ml-1">
                              ({Math.round(aiAnalysis.brokerConfidence * 100)}%)
                            </span>
                          </div>
                        )}
                        {aiAnalysis.analysisNotes && (
                          <div className="text-xs text-muted-foreground">
                            {aiAnalysis.analysisNotes}
                          </div>
                        )}
                        {aiAnalysis.mappingSuggestions.length > 0 && (
                          <div className="text-xs text-green-600">
                            {aiAnalysis.mappingSuggestions.length} Spalten-Zuordnungen vorgeschlagen und angewendet
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                )}

                {/* Manual Column Mapping */}
                {!useTemplate && (
                  <div className="border border-border rounded-lg p-4">
                    <h3 className="font-medium mb-3">Spalten zuordnen</h3>
                    <div className="grid grid-cols-2 gap-3">
                      {MAPPING_FIELDS.map(field => (
                        <div key={field.key} className="flex items-center gap-2">
                          <label className="text-sm w-24">
                            {field.label}
                            {field.required && <span className="text-destructive">*</span>}
                          </label>
                          <select
                            value={mapping[field.key] ?? ''}
                            onChange={(e) => handleMappingChange(field.key, e.target.value)}
                            className="flex-1 px-2 py-1 text-sm border border-border rounded bg-background"
                          >
                            <option value="">-- Nicht zuordnen --</option>
                            {preview.columns.map(col => (
                              <option key={col.index} value={col.index}>
                                {col.name}
                                {col.sampleValues[0] && ` (z.B. ${col.sampleValues[0].substring(0, 20)})`}
                              </option>
                            ))}
                          </select>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Portfolio Selection */}
                <div className="flex items-center gap-2">
                  <span className="text-sm text-muted-foreground">Ziel-Portfolio:</span>
                  <select
                    value={selectedPortfolio || ''}
                    onChange={(e) => setSelectedPortfolio(Number(e.target.value))}
                    className="px-2 py-1 text-sm border border-border rounded bg-background"
                  >
                    <option value="">-- Portfolio wählen --</option>
                    {portfolios.map(p => (
                      <option key={p.id} value={p.id}>{p.name}</option>
                    ))}
                  </select>
                </div>

                {/* Preview Table */}
                <div className="border border-border rounded-lg overflow-hidden">
                  <div className="bg-muted px-3 py-2 text-sm font-medium flex items-center justify-between">
                    <span>Vorschau (erste 5 Zeilen)</span>
                  </div>
                  <div className="overflow-x-auto">
                    <table className="w-full text-sm">
                      <thead className="bg-muted/50">
                        <tr>
                          {preview.columns.map(col => {
                            // Find which field this column is mapped to
                            const mappedTo = Object.entries(mapping).find(([, v]) => v === col.index)?.[0];
                            const fieldInfo = MAPPING_FIELDS.find(f => f.key === mappedTo);

                            return (
                              <th key={col.index} className="text-left py-2 px-3 font-medium border-b border-border">
                                <div>{col.name}</div>
                                {fieldInfo && !useTemplate && (
                                  <div className="text-xs text-primary font-normal">
                                    → {fieldInfo.label}
                                  </div>
                                )}
                              </th>
                            );
                          })}
                        </tr>
                      </thead>
                      <tbody>
                        {Array.from({ length: Math.min(5, preview.columns[0]?.sampleValues.length || 0) }).map((_, rowIdx) => (
                          <tr key={rowIdx} className="border-b border-border last:border-0">
                            {preview.columns.map(col => (
                              <td key={col.index} className="py-2 px-3">
                                {col.sampleValues[rowIdx] || ''}
                              </td>
                            ))}
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </div>
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
                {importResult.rowsImported > 0 ? (
                  <>
                    <CheckCircle className="w-16 h-16 mx-auto mb-4 text-green-600" />
                    <h3 className="text-xl font-bold mb-2">Import erfolgreich!</h3>
                    <p className="text-muted-foreground mb-6">
                      {importResult.rowsImported} Transaktionen wurden importiert.
                      {importResult.rowsSkipped > 0 && (
                        <> {importResult.rowsSkipped} Zeilen wurden übersprungen.</>
                      )}
                    </p>
                  </>
                ) : (
                  <>
                    <AlertCircle className="w-16 h-16 mx-auto mb-4 text-destructive" />
                    <h3 className="text-xl font-bold mb-2">Import fehlgeschlagen</h3>
                    <p className="text-muted-foreground mb-6">
                      Keine Transaktionen konnten importiert werden.
                    </p>
                  </>
                )}

                {importResult.errors.length > 0 && (
                  <div className="text-left bg-destructive/10 border border-destructive/20 rounded-lg p-4 mt-4 max-h-48 overflow-y-auto">
                    <h4 className="font-medium text-destructive mb-2">Fehler ({importResult.errors.length})</h4>
                    <ul className="text-sm text-destructive space-y-1">
                      {importResult.errors.slice(0, 20).map((err, idx) => (
                        <li key={idx}>{err}</li>
                      ))}
                      {importResult.errors.length > 20 && (
                        <li>... und {importResult.errors.length - 20} weitere Fehler</li>
                      )}
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
            onClick={step === 'map' ? () => { setStep('select'); setFilePath(null); setPreview(null); } : onClose}
            className="flex items-center gap-2 px-4 py-2 text-sm border border-border rounded-md hover:bg-muted transition-colors"
          >
            {step === 'map' ? (
              <>
                <ChevronLeft size={16} />
                Zurück
              </>
            ) : (
              step === 'done' ? 'Schließen' : 'Abbrechen'
            )}
          </button>

          {step === 'map' && (
            <button
              onClick={handleImport}
              disabled={!hasRequiredMappings() || !selectedPortfolio}
              className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              Import starten
              <ChevronRight size={16} />
            </button>
          )}

          {step === 'done' && importResult?.rowsImported && importResult.rowsImported > 0 && (
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
