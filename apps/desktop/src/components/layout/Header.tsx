/**
 * Header component with import actions and new transaction button.
 * Simplified: removed legacy file operations (Neu, Öffnen, Speichern).
 */

import { useState } from 'react';
import {
  Plus,
  Database,
  RefreshCw,
  FileDown,
  FileUp,
  FileText,
  Globe,
} from 'lucide-react';
import {
  useUIStore,
  useAppStore,
  useSettingsStore,
  getViewLabel,
} from '../../store';
import { DropdownMenu, DropdownItem } from '../common';
import { AIProviderLogo } from '../common/AIProviderLogo';
import { TransactionFormModal } from '../modals/TransactionFormModal';
import { PdfImportModal } from '../modals/PdfImportModal';
import { PdfExportModal } from '../modals/PdfExportModal';
import { DivvyDiaryExportModal } from '../modals/DivvyDiaryExportModal';
import { DivvyDiaryLogo } from '../common/DivvyDiaryLogo';

interface HeaderProps {
  onImportPP: () => void;
  onRefresh: () => void;
}

// Provider display names
const PROVIDER_NAMES: Record<string, string> = {
  claude: 'Claude',
  openai: 'OpenAI',
  gemini: 'Gemini',
  perplexity: 'Perplexity',
};

// Check if model supports live web search
const supportsWebSearch = (provider: string, model: string): boolean => {
  // Perplexity always has web search
  if (provider === 'perplexity') return true;
  // OpenAI o3, o4 models have web search
  if (provider === 'openai' && (model.startsWith('o3') || model.startsWith('o4'))) return true;
  return false;
};

export function Header({
  onImportPP,
  onRefresh,
}: HeaderProps) {
  const { currentView } = useUIStore();
  const { isLoading } = useAppStore();
  const {
    aiProvider,
    aiModel,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
  } = useSettingsStore();

  const [showTransactionModal, setShowTransactionModal] = useState(false);
  const [showPdfImportModal, setShowPdfImportModal] = useState(false);
  const [showPdfExportModal, setShowPdfExportModal] = useState(false);
  const [showDivvyDiaryModal, setShowDivvyDiaryModal] = useState(false);

  // Check if AI is configured (has API key for selected provider)
  const hasAiApiKey = () => {
    switch (aiProvider) {
      case 'claude': return !!anthropicApiKey;
      case 'openai': return !!openaiApiKey;
      case 'gemini': return !!geminiApiKey;
      case 'perplexity': return !!perplexityApiKey;
      default: return false;
    }
  };

  const aiConfigured = hasAiApiKey();

  return (
    <>
      <header className="h-14 flex items-center justify-between px-6 border-b border-border bg-card">
        {/* Left: View title + AI indicator */}
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-semibold text-foreground">
            {getViewLabel(currentView)}
          </h1>

          {/* AI Provider Indicator */}
          {aiConfigured && (
            <div className="flex items-center gap-2 px-3 py-1 bg-muted/50 rounded-full border border-border/50">
              <AIProviderLogo provider={aiProvider} size={16} />
              <span className="text-xs text-muted-foreground">
                {PROVIDER_NAMES[aiProvider] || aiProvider}
              </span>
              <span className="text-xs text-muted-foreground/60">
                {aiModel.split('-').slice(0, 2).join('-')}
              </span>
              {supportsWebSearch(aiProvider, aiModel) && (
                <span title="Live Web-Suche"><Globe className="w-3.5 h-3.5 text-blue-500" /></span>
              )}
            </div>
          )}
        </div>

        {/* Right: Actions */}
        <div className="flex items-center gap-2">
          {/* Import Menu Dropdown */}
          <DropdownMenu
            trigger={
              <>
                <FileDown className="w-4 h-4" aria-hidden="true" />
                <span>Importieren</span>
              </>
            }
            disabled={isLoading}
          >
            <DropdownItem
              onClick={onImportPP}
              disabled={isLoading}
              icon={<Database className="w-4 h-4" />}
            >
              Portfolio Performance Datei...
            </DropdownItem>
            <DropdownItem
              onClick={() => setShowPdfImportModal(true)}
              disabled={isLoading}
              icon={<FileText className="w-4 h-4" />}
            >
              PDF Kontoauszug...
            </DropdownItem>
          </DropdownMenu>

          {/* Export Menu Dropdown */}
          <DropdownMenu
            trigger={
              <>
                <FileUp className="w-4 h-4" aria-hidden="true" />
                <span>Exportieren</span>
              </>
            }
            disabled={isLoading}
          >
            <DropdownItem
              onClick={() => setShowPdfExportModal(true)}
              disabled={isLoading}
              icon={<FileText className="w-4 h-4" />}
            >
              PDF Bericht...
            </DropdownItem>
            <DropdownItem
              onClick={() => setShowDivvyDiaryModal(true)}
              disabled={isLoading}
              icon={<DivvyDiaryLogo size={16} />}
            >
              DivvyDiary...
            </DropdownItem>
          </DropdownMenu>

          {/* Refresh button */}
          <button
            onClick={onRefresh}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-input rounded-md hover:bg-accent transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            title="Daten aktualisieren"
            aria-label="Daten aktualisieren"
          >
            <RefreshCw
              className={`w-4 h-4 ${isLoading ? 'animate-spin' : ''}`}
              aria-hidden="true"
            />
          </button>

          {/* Primary action: New transaction */}
          <button
            onClick={() => setShowTransactionModal(true)}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            aria-label="Neue Buchung erstellen"
          >
            <Plus className="w-4 h-4" aria-hidden="true" />
            <span>Neue Buchung</span>
          </button>
        </div>
      </header>

      {/* Transaction Form Modal */}
      <TransactionFormModal
        isOpen={showTransactionModal}
        onClose={() => setShowTransactionModal(false)}
        onSuccess={() => {
          setShowTransactionModal(false);
          onRefresh();
        }}
      />

      {/* PDF Import Modal */}
      <PdfImportModal
        isOpen={showPdfImportModal}
        onClose={() => setShowPdfImportModal(false)}
        onSuccess={onRefresh}
      />

      {/* PDF Export Modal */}
      <PdfExportModal
        isOpen={showPdfExportModal}
        onClose={() => setShowPdfExportModal(false)}
      />

      {/* DivvyDiary Export Modal */}
      <DivvyDiaryExportModal
        isOpen={showDivvyDiaryModal}
        onClose={() => setShowDivvyDiaryModal(false)}
      />
    </>
  );
}
