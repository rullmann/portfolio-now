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
  FileSpreadsheet,
  Globe,
  Sparkles,
  MessageSquare,
  ShoppingCart,
  PieChart,
  TrendingUp,
  ChevronDown,
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
import { CsvImportModal } from '../modals/CsvImportModal';
import { PdfExportModal } from '../modals/PdfExportModal';
import { DivvyDiaryExportModal } from '../modals/DivvyDiaryExportModal';
import { DivvyDiaryLogo } from '../common/DivvyDiaryLogo';
import { PortfolioInsightsModal } from '../modals/PortfolioInsightsModal';

interface HeaderProps {
  onImportPP: () => void;
  onRefresh: () => void;
  onOpenChat?: () => void;
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

// Get context-specific AI actions based on current view
const getContextActions = (view: string): { label: string; icon: React.ReactNode; action: string }[] => {
  switch (view) {
    case 'holdings':
      return [
        { label: 'Diversifikation prüfen', icon: <PieChart className="w-4 h-4" />, action: 'diversification' },
      ];
    case 'charts':
      return [
        { label: 'Chart analysieren', icon: <TrendingUp className="w-4 h-4" />, action: 'chart' },
      ];
    default:
      return [];
  }
};

export function Header({
  onImportPP,
  onRefresh,
  onOpenChat,
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
  const [showCsvImportModal, setShowCsvImportModal] = useState(false);
  const [showPdfExportModal, setShowPdfExportModal] = useState(false);
  const [showDivvyDiaryModal, setShowDivvyDiaryModal] = useState(false);
  const [showInsightsModal, setShowInsightsModal] = useState(false);
  const [insightsMode, setInsightsMode] = useState<'insights' | 'opportunities'>('insights');
  const [showAiMenu, setShowAiMenu] = useState(false);

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

          {/* AI Provider Indicator - Clickable */}
          {aiConfigured && (
            <div className="relative">
              <button
                onClick={() => setShowAiMenu(!showAiMenu)}
                className="flex items-center gap-2 px-3 py-1.5 bg-muted/50 rounded-full border border-border/50 hover:bg-muted hover:border-border transition-colors"
              >
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
                <ChevronDown className={`w-3.5 h-3.5 text-muted-foreground transition-transform ${showAiMenu ? 'rotate-180' : ''}`} />
              </button>

              {/* AI Dropdown Menu */}
              {showAiMenu && (
                <>
                  {/* Backdrop */}
                  <div
                    className="fixed inset-0 z-40"
                    onClick={() => setShowAiMenu(false)}
                  />

                  {/* Menu */}
                  <div className="absolute top-full left-0 mt-1 w-56 bg-popover border border-border rounded-lg shadow-lg z-50 py-1">
                    {/* Portfolio Insights */}
                    <button
                      onClick={() => {
                        setInsightsMode('insights');
                        setShowInsightsModal(true);
                        setShowAiMenu(false);
                      }}
                      className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-accent transition-colors"
                    >
                      <Sparkles className="w-4 h-4 text-primary" />
                      <span>Portfolio Insights</span>
                    </button>

                    {/* Buy Opportunities */}
                    <button
                      onClick={() => {
                        setInsightsMode('opportunities');
                        setShowInsightsModal(true);
                        setShowAiMenu(false);
                      }}
                      className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-accent transition-colors"
                    >
                      <ShoppingCart className="w-4 h-4 text-green-600" />
                      <span>Nachkauf-Chancen</span>
                    </button>

                    {/* Chat */}
                    {onOpenChat && (
                      <button
                        onClick={() => {
                          onOpenChat();
                          setShowAiMenu(false);
                        }}
                        className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-accent transition-colors"
                      >
                        <MessageSquare className="w-4 h-4 text-blue-600" />
                        <span>Chat öffnen</span>
                      </button>
                    )}

                    {/* Context-specific actions */}
                    {getContextActions(currentView).length > 0 && (
                      <>
                        <div className="border-t border-border my-1" />
                        <div className="px-3 py-1 text-xs text-muted-foreground font-medium">
                          {getViewLabel(currentView)}
                        </div>
                        {getContextActions(currentView).map((action) => (
                          <button
                            key={action.action}
                            onClick={() => {
                              // TODO: Handle context action
                              setShowAiMenu(false);
                            }}
                            className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-accent transition-colors"
                          >
                            {action.icon}
                            <span>{action.label}</span>
                          </button>
                        ))}
                      </>
                    )}
                  </div>
                </>
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
            <DropdownItem
              onClick={() => setShowCsvImportModal(true)}
              disabled={isLoading}
              icon={<FileSpreadsheet className="w-4 h-4" />}
            >
              CSV Transaktionen...
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

      {/* CSV Import Modal */}
      <CsvImportModal
        isOpen={showCsvImportModal}
        onClose={() => setShowCsvImportModal(false)}
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

      {/* Portfolio Insights Modal */}
      <PortfolioInsightsModal
        isOpen={showInsightsModal}
        onClose={() => setShowInsightsModal(false)}
        initialMode={insightsMode}
      />
    </>
  );
}
