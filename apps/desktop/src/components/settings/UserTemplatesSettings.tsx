/**
 * User Templates Settings - Configure custom SQL query templates for ChatBot.
 *
 * Allows users to create, edit, and delete their own query templates
 * that can be used by the AI assistant.
 */

import { useState, useEffect, useCallback } from 'react';
import { Plus, Pencil, Trash2, ToggleLeft, ToggleRight, Database, AlertCircle } from 'lucide-react';
import {
  getUserTemplates,
  deleteUserTemplate,
  updateUserTemplate,
} from '../../lib/api';
import type { UserTemplate, UserTemplateInput } from '../../lib/types';
import { UserTemplateModal } from '../modals/UserTemplateModal';
import { toast } from '../../store';

interface UserTemplatesSettingsProps {
  className?: string;
}

export function UserTemplatesSettings({ className }: UserTemplatesSettingsProps) {
  const [templates, setTemplates] = useState<UserTemplate[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<UserTemplate | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  // Load templates
  const loadTemplates = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const data = await getUserTemplates();
      setTemplates(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Fehler beim Laden');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  // Toggle enabled state
  const handleToggleEnabled = async (template: UserTemplate) => {
    try {
      const input: UserTemplateInput = {
        name: template.name,
        description: template.description,
        sqlQuery: template.sqlQuery,
        enabled: !template.enabled,
        parameters: template.parameters,
      };
      await updateUserTemplate(template.id, input);
      toast.success(template.enabled ? 'Template deaktiviert' : 'Template aktiviert');
      loadTemplates();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Fehler beim Aktualisieren');
    }
  };

  // Delete template
  const handleDelete = async (id: number) => {
    try {
      setDeletingId(id);
      await deleteUserTemplate(id);
      toast.success('Template gelöscht');
      loadTemplates();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Fehler beim Löschen');
    } finally {
      setDeletingId(null);
    }
  };

  // Open modal for editing
  const handleEdit = (template: UserTemplate) => {
    setEditingTemplate(template);
    setModalOpen(true);
  };

  // Open modal for creating
  const handleCreate = () => {
    setEditingTemplate(null);
    setModalOpen(true);
  };

  // Close modal and reload
  const handleModalClose = (saved: boolean) => {
    setModalOpen(false);
    setEditingTemplate(null);
    if (saved) {
      loadTemplates();
    }
  };

  if (isLoading) {
    return (
      <div className={`space-y-4 ${className || ''}`}>
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-medium">Eigene Abfragen</h3>
        </div>
        <div className="animate-pulse space-y-3">
          {[1, 2].map((i) => (
            <div key={i} className="h-16 bg-muted rounded-lg" />
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className={`space-y-4 ${className || ''}`}>
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-medium">Eigene Abfragen</h3>
        </div>
        <div className="flex items-center gap-2 text-destructive p-4 border border-destructive/50 rounded-lg">
          <AlertCircle className="h-5 w-5" />
          <span>{error}</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`space-y-4 ${className || ''}`}>
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-medium">Eigene Abfragen</h3>
          <p className="text-sm text-muted-foreground">
            Definiere SQL-Abfragen, die der ChatBot für dich ausführen kann.
          </p>
        </div>
        <button
          onClick={handleCreate}
          className="flex items-center gap-2 px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
        >
          <Plus className="h-4 w-4" />
          Neue Abfrage
        </button>
      </div>

      {templates.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground border border-dashed rounded-lg">
          <Database className="h-10 w-10 mx-auto mb-3 opacity-50" />
          <p>Keine eigenen Abfragen definiert.</p>
          <p className="text-sm mt-1">
            Erstelle eine Abfrage, um dem ChatBot zusätzliche Datenbank-Zugriffe zu ermöglichen.
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {templates.map((template) => (
            <div
              key={template.id}
              className={`flex items-center justify-between p-4 border rounded-lg transition-colors ${
                template.enabled
                  ? 'bg-background'
                  : 'bg-muted/50 opacity-60'
              }`}
            >
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-sm bg-muted px-2 py-0.5 rounded">
                    {template.templateId}
                  </span>
                  {!template.enabled && (
                    <span className="text-xs text-muted-foreground">(deaktiviert)</span>
                  )}
                </div>
                <p className="text-sm font-medium mt-1">{template.name}</p>
                <p className="text-sm text-muted-foreground truncate">{template.description}</p>
                {template.parameters.length > 0 && (
                  <p className="text-xs text-muted-foreground mt-1">
                    Parameter: {template.parameters.map((p) => p.paramName).join(', ')}
                  </p>
                )}
              </div>

              <div className="flex items-center gap-2 ml-4">
                <button
                  onClick={() => handleToggleEnabled(template)}
                  title={template.enabled ? 'Deaktivieren' : 'Aktivieren'}
                  className="p-2 rounded hover:bg-muted transition-colors"
                >
                  {template.enabled ? (
                    <ToggleRight className="h-5 w-5 text-green-600" />
                  ) : (
                    <ToggleLeft className="h-5 w-5 text-muted-foreground" />
                  )}
                </button>
                <button
                  onClick={() => handleEdit(template)}
                  title="Bearbeiten"
                  className="p-2 rounded hover:bg-muted transition-colors"
                >
                  <Pencil className="h-4 w-4" />
                </button>
                <button
                  onClick={() => handleDelete(template.id)}
                  disabled={deletingId === template.id}
                  title="Löschen"
                  className="p-2 rounded hover:bg-muted transition-colors text-destructive hover:text-destructive disabled:opacity-50"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="text-xs text-muted-foreground p-3 bg-amber-500/10 border border-amber-500/30 rounded-lg">
        <p className="font-medium text-amber-700 dark:text-amber-400">Hinweis zur Sicherheit</p>
        <p className="mt-1">
          Nur SELECT-Abfragen sind erlaubt. INSERT, UPDATE, DELETE und andere
          Datenbank-modifizierende Befehle werden automatisch blockiert.
        </p>
      </div>

      <UserTemplateModal
        open={modalOpen}
        onClose={handleModalClose}
        template={editingTemplate}
      />
    </div>
  );
}
