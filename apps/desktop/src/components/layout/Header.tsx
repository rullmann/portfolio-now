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
  FileText,
} from 'lucide-react';
import {
  useUIStore,
  useAppStore,
  getViewLabel,
} from '../../store';
import { DropdownMenu, DropdownItem } from '../common';
import { TransactionFormModal } from '../modals/TransactionFormModal';
import { PdfImportModal } from '../modals/PdfImportModal';

interface HeaderProps {
  onImportPP: () => void;
  onRefresh: () => void;
}

export function Header({
  onImportPP,
  onRefresh,
}: HeaderProps) {
  const { currentView } = useUIStore();
  const { isLoading } = useAppStore();

  const [showTransactionModal, setShowTransactionModal] = useState(false);
  const [showPdfImportModal, setShowPdfImportModal] = useState(false);

  return (
    <>
      <header className="h-14 flex items-center justify-between px-6 border-b border-border bg-card">
        {/* Left: View title */}
        <div className="flex items-center gap-4">
          <h1 className="text-lg font-semibold text-foreground">
            {getViewLabel(currentView)}
          </h1>
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
    </>
  );
}
