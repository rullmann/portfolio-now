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
  Sparkles,
  MessageSquare,
  ShoppingCart,
  PieChart,
  TrendingUp,
  ChevronDown,
  BarChart3,
} from 'lucide-react';
import {
  useUIStore,
  useAppStore,
  useSettingsStore,
  getViewLabel,
} from '../../store';
import { DropdownMenu, DropdownItem } from '../common';
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
  const { currentView, setCurrentView } = useUIStore();
  const { isLoading } = useAppStore();
  const {
    aiEnabled,
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

  // Check if AI is configured (has at least one API key)
  const hasAnyAiApiKey = !!(anthropicApiKey || openaiApiKey || geminiApiKey || perplexityApiKey);
  const aiConfigured = aiEnabled && hasAnyAiApiKey;

  return (
    <>
      <header className="h-14 flex items-center justify-between px-6 border-b border-border bg-card">
        {/* Left: View title + AI indicator */}
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-semibold text-foreground">
            {getViewLabel(currentView)}
          </h1>

          {/* AI Indicator - Simple "KI" badge with dropdown */}
          {aiConfigured && (
            <div className="relative">
              <button
                onClick={() => setShowAiMenu(!showAiMenu)}
                className="flex items-center gap-1.5 px-2.5 py-1 bg-primary/10 text-primary rounded-full border border-primary/20 hover:bg-primary/20 hover:border-primary/30 transition-colors"
                title="KI-Funktionen"
              >
                <Sparkles className="w-3.5 h-3.5" />
                <span className="text-xs font-medium">KI</span>
                <ChevronDown className={`w-3 h-3 transition-transform ${showAiMenu ? 'rotate-180' : ''}`} />
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

                    <div className="border-t border-border my-1" />

                    {/* Chart Analysis */}
                    <button
                      onClick={() => {
                        setCurrentView('charts');
                        setShowAiMenu(false);
                      }}
                      className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-accent transition-colors"
                    >
                      <BarChart3 className="w-4 h-4 text-purple-600" />
                      <span>Chart-Analyse</span>
                    </button>

                    {/* PDF OCR */}
                    <button
                      onClick={() => {
                        setShowPdfImportModal(true);
                        setShowAiMenu(false);
                      }}
                      className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-accent transition-colors"
                    >
                      <FileText className="w-4 h-4 text-orange-600" />
                      <span>PDF OCR</span>
                    </button>

                    {/* CSV Import */}
                    <button
                      onClick={() => {
                        setShowCsvImportModal(true);
                        setShowAiMenu(false);
                      }}
                      className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-accent transition-colors"
                    >
                      <FileSpreadsheet className="w-4 h-4 text-teal-600" />
                      <span>CSV-Import</span>
                    </button>

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
