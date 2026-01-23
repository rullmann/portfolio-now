/**
 * Modal for creating and editing user-defined query templates.
 *
 * Provides a form for entering template name, description, SQL query,
 * and optional parameters. Includes SQL validation and preview functionality.
 */

import { useState, useEffect } from 'react';
import { X, Plus, Trash2, Play, AlertCircle, CheckCircle2, Loader2 } from 'lucide-react';
import type { UserTemplate, UserTemplateInput, UserTemplateParam } from '../../lib/types';
import { createUserTemplate, updateUserTemplate, testUserTemplate } from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';
import { toast } from '../../store';

interface UserTemplateModalProps {
  open: boolean;
  onClose: (saved: boolean) => void;
  template?: UserTemplate | null;
}

const PARAM_TYPES = [
  { value: 'string', label: 'Text' },
  { value: 'number', label: 'Zahl' },
  { value: 'date', label: 'Datum (YYYY-MM-DD)' },
  { value: 'year', label: 'Jahr' },
] as const;

export function UserTemplateModal({ open, onClose, template }: UserTemplateModalProps) {
  const isEditing = !!template;

  // Form state
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [sqlQuery, setSqlQuery] = useState('');
  const [parameters, setParameters] = useState<UserTemplateParam[]>([]);

  // UI state
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
    rowCount?: number;
  } | null>(null);

  // Reset form when modal opens/closes or template changes
  useEffect(() => {
    if (open) {
      if (template) {
        setName(template.name);
        setDescription(template.description);
        setSqlQuery(template.sqlQuery);
        setParameters(template.parameters.map(p => ({ ...p })));
      } else {
        setName('');
        setDescription('');
        setSqlQuery('SELECT \n  s.name,\n  s.ticker\nFROM pp_security s\nLIMIT 10');
        setParameters([]);
      }
      setTestResult(null);
    }
  }, [open, template]);

  // Escape key to close
  useEscapeKey(open, () => onClose(false));

  // Generate template ID preview
  const templateIdPreview = name
    ? `user_${name.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/_+/g, '_').replace(/^_|_$/g, '')}`
    : 'user_...';

  // Add parameter
  const addParameter = () => {
    setParameters([
      ...parameters,
      {
        paramName: '',
        paramType: 'string',
        required: false,
        description: '',
        defaultValue: undefined,
      },
    ]);
  };

  // Remove parameter
  const removeParameter = (index: number) => {
    setParameters(parameters.filter((_, i) => i !== index));
  };

  // Update parameter
  const updateParameter = (index: number, field: keyof UserTemplateParam, value: unknown) => {
    const updated = [...parameters];
    updated[index] = { ...updated[index], [field]: value };
    setParameters(updated);
  };

  // Validate form
  const validateForm = (): string | null => {
    if (!name.trim()) return 'Name ist erforderlich';
    if (!description.trim()) return 'Beschreibung ist erforderlich';
    if (!sqlQuery.trim()) return 'SQL-Abfrage ist erforderlich';

    // Check for unnamed parameters
    const unnamedParam = parameters.find(p => !p.paramName.trim());
    if (unnamedParam) return 'Alle Parameter benötigen einen Namen';

    // Check for duplicate parameter names
    const paramNames = parameters.map(p => p.paramName.toLowerCase());
    const hasDuplicates = paramNames.length !== new Set(paramNames).size;
    if (hasDuplicates) return 'Parameter-Namen müssen eindeutig sein';

    return null;
  };

  // Test the query
  const handleTest = async () => {
    const error = validateForm();
    if (error) {
      setTestResult({ success: false, message: error });
      return;
    }

    setIsTesting(true);
    setTestResult(null);

    try {
      const input: UserTemplateInput = {
        name: name.trim(),
        description: description.trim(),
        sqlQuery: sqlQuery.trim(),
        parameters: parameters.filter(p => p.paramName.trim()),
      };

      const result = await testUserTemplate(input);

      if (result.success) {
        setTestResult({
          success: true,
          message: `Erfolgreich! ${result.rowCount} Ergebnisse.`,
          rowCount: result.rowCount,
        });
      } else {
        setTestResult({
          success: false,
          message: result.error || 'Unbekannter Fehler',
        });
      }
    } catch (err) {
      setTestResult({
        success: false,
        message: err instanceof Error ? err.message : 'Fehler beim Testen',
      });
    } finally {
      setIsTesting(false);
    }
  };

  // Save the template
  const handleSave = async () => {
    const error = validateForm();
    if (error) {
      toast.error(error);
      return;
    }

    setIsSaving(true);

    try {
      const input: UserTemplateInput = {
        name: name.trim(),
        description: description.trim(),
        sqlQuery: sqlQuery.trim(),
        enabled: true,
        parameters: parameters.filter(p => p.paramName.trim()),
      };

      if (isEditing && template) {
        await updateUserTemplate(template.id, input);
        toast.success('Template aktualisiert');
      } else {
        await createUserTemplate(input);
        toast.success('Template erstellt');
      }

      onClose(true);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Fehler beim Speichern');
    } finally {
      setIsSaving(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background border border-border rounded-lg shadow-lg w-full max-w-2xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <h2 className="text-lg font-semibold">
            {isEditing ? 'Abfrage bearbeiten' : 'Neue Abfrage erstellen'}
          </h2>
          <button
            onClick={() => onClose(false)}
            className="p-1 rounded hover:bg-muted transition-colors"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-4">
          {/* Name */}
          <div>
            <label className="block text-sm font-medium mb-1">Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="z.B. Top Performer"
              className="w-full px-3 py-2 border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
            />
            <p className="text-xs text-muted-foreground mt-1">
              Template-ID: <code className="bg-muted px-1 rounded">{templateIdPreview}</code>
            </p>
          </div>

          {/* Description */}
          <div>
            <label className="block text-sm font-medium mb-1">Beschreibung</label>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Was zeigt diese Abfrage an?"
              className="w-full px-3 py-2 border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
            />
          </div>

          {/* SQL Query */}
          <div>
            <label className="block text-sm font-medium mb-1">SQL-Abfrage</label>
            <textarea
              value={sqlQuery}
              onChange={(e) => {
                setSqlQuery(e.target.value);
                setTestResult(null);
              }}
              placeholder="SELECT ..."
              rows={8}
              className="w-full px-3 py-2 font-mono text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 resize-y"
            />
            <p className="text-xs text-muted-foreground mt-1">
              Nur SELECT-Abfragen sind erlaubt. Verwende :param_name für Parameter.
            </p>
          </div>

          {/* Test Result */}
          {testResult && (
            <div
              className={`flex items-start gap-2 p-3 rounded-lg ${
                testResult.success
                  ? 'bg-green-500/10 border border-green-500/30 text-green-700 dark:text-green-400'
                  : 'bg-red-500/10 border border-red-500/30 text-red-700 dark:text-red-400'
              }`}
            >
              {testResult.success ? (
                <CheckCircle2 className="h-5 w-5 flex-shrink-0" />
              ) : (
                <AlertCircle className="h-5 w-5 flex-shrink-0" />
              )}
              <span className="text-sm">{testResult.message}</span>
            </div>
          )}

          {/* Parameters */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className="block text-sm font-medium">Parameter (optional)</label>
              <button
                type="button"
                onClick={addParameter}
                className="flex items-center gap-1 text-sm text-primary hover:text-primary/80 transition-colors"
              >
                <Plus className="h-4 w-4" />
                Hinzufügen
              </button>
            </div>

            {parameters.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                Keine Parameter definiert. Parameter erlauben dynamische Werte in der Abfrage.
              </p>
            ) : (
              <div className="space-y-3">
                {parameters.map((param, index) => (
                  <div
                    key={index}
                    className="grid grid-cols-[1fr_1fr_auto_auto] gap-2 p-3 bg-muted/30 rounded-lg"
                  >
                    <input
                      type="text"
                      value={param.paramName}
                      onChange={(e) => updateParameter(index, 'paramName', e.target.value)}
                      placeholder="Name (z.B. year)"
                      className="px-2 py-1 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                    />
                    <select
                      value={param.paramType}
                      onChange={(e) => updateParameter(index, 'paramType', e.target.value)}
                      className="px-2 py-1 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                    >
                      {PARAM_TYPES.map((type) => (
                        <option key={type.value} value={type.value}>
                          {type.label}
                        </option>
                      ))}
                    </select>
                    <label className="flex items-center gap-1 text-sm">
                      <input
                        type="checkbox"
                        checked={param.required}
                        onChange={(e) => updateParameter(index, 'required', e.target.checked)}
                        className="rounded"
                      />
                      Pflicht
                    </label>
                    <button
                      type="button"
                      onClick={() => removeParameter(index)}
                      className="p-1 text-muted-foreground hover:text-destructive transition-colors"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                    <input
                      type="text"
                      value={param.description}
                      onChange={(e) => updateParameter(index, 'description', e.target.value)}
                      placeholder="Beschreibung"
                      className="col-span-2 px-2 py-1 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                    />
                    <input
                      type="text"
                      value={param.defaultValue || ''}
                      onChange={(e) => updateParameter(index, 'defaultValue', e.target.value || undefined)}
                      placeholder="Default"
                      className="col-span-2 px-2 py-1 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
                    />
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-6 py-4 border-t bg-muted/30">
          <button
            type="button"
            onClick={handleTest}
            disabled={isTesting || isSaving}
            className="flex items-center gap-2 px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isTesting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Play className="h-4 w-4" />
            )}
            Testen
          </button>

          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => onClose(false)}
              disabled={isSaving}
              className="px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors"
            >
              Abbrechen
            </button>
            <button
              type="button"
              onClick={handleSave}
              disabled={isSaving}
              className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSaving && <Loader2 className="h-4 w-4 animate-spin" />}
              {isEditing ? 'Speichern' : 'Erstellen'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
