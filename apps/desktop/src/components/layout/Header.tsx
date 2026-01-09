/**
 * Header component with file operations and actions.
 * Redesigned with dropdown menu for cleaner UI.
 */

import {
  FileText,
  Plus,
  FolderOpen,
  Save,
  FilePlus,
  Database,
  RefreshCw,
  File,
} from 'lucide-react';
import {
  useUIStore,
  useAppStore,
  usePortfolioFileStore,
  useDataModeStore,
  getViewLabel,
} from '../../store';
import { DropdownMenu, DropdownItem, DropdownDivider } from '../common';

interface HeaderProps {
  onNewFile: () => void;
  onOpenFile: () => void;
  onSaveFile: () => void;
  onSaveAsFile: () => void;
  onImportToDb: () => void;
  onRefresh: () => void;
  hasPortfolioFile: boolean;
}

export function Header({
  onNewFile,
  onOpenFile,
  onSaveFile,
  onSaveAsFile,
  onImportToDb,
  onRefresh,
  hasPortfolioFile,
}: HeaderProps) {
  const { currentView } = useUIStore();
  const { isLoading } = useAppStore();
  const { currentFilePath, hasUnsavedChanges } = usePortfolioFileStore();
  const { useDbData } = useDataModeStore();

  const fileName = currentFilePath
    ? currentFilePath.split('/').pop() || 'Portfolio'
    : hasPortfolioFile
      ? 'Neues Portfolio'
      : null;

  return (
    <header className="h-14 flex items-center justify-between px-6 border-b border-border bg-card">
      {/* Left: View title and file info */}
      <div className="flex items-center gap-4">
        <h1 className="text-lg font-semibold text-foreground">
          {getViewLabel(currentView)}
        </h1>
        {fileName && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <FileText className="w-4 h-4" aria-hidden="true" />
            <span>{fileName}</span>
            {hasUnsavedChanges && (
              <span className="text-yellow-500" aria-label="Ungespeicherte Änderungen">
                *
              </span>
            )}
          </div>
        )}
      </div>

      {/* Right: Actions */}
      <div className="flex items-center gap-2">
        {/* File Menu Dropdown */}
        <DropdownMenu
          trigger={
            <>
              <File className="w-4 h-4" aria-hidden="true" />
              <span>Datei</span>
            </>
          }
          disabled={isLoading}
        >
          <DropdownItem
            onClick={onNewFile}
            disabled={isLoading}
            icon={<FilePlus className="w-4 h-4" />}
          >
            Neu
          </DropdownItem>
          <DropdownItem
            onClick={onOpenFile}
            disabled={isLoading}
            icon={<FolderOpen className="w-4 h-4" />}
          >
            Öffnen...
          </DropdownItem>
          <DropdownDivider />
          <DropdownItem
            onClick={onSaveFile}
            disabled={isLoading || !hasPortfolioFile || !hasUnsavedChanges}
            icon={<Save className="w-4 h-4" />}
          >
            Speichern
          </DropdownItem>
          <DropdownItem
            onClick={onSaveAsFile}
            disabled={isLoading || !hasPortfolioFile}
            icon={<Save className="w-4 h-4" />}
          >
            Speichern unter...
          </DropdownItem>
          <DropdownDivider />
          <DropdownItem
            onClick={onImportToDb}
            disabled={isLoading}
            icon={<Database className="w-4 h-4" />}
          >
            In DB importieren
          </DropdownItem>
        </DropdownMenu>

        {/* Refresh button (when using DB) */}
        {useDbData && (
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
        )}

        {/* Primary action: New transaction */}
        <button
          disabled={!hasPortfolioFile && !useDbData}
          className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label="Neue Buchung erstellen"
        >
          <Plus className="w-4 h-4" aria-hidden="true" />
          <span>Neue Buchung</span>
        </button>
      </div>
    </header>
  );
}
