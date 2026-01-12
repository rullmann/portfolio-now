/**
 * Modal for creating and editing portfolios.
 */

import { useState, useEffect } from 'react';
import { X, ChevronRight, ChevronDown, Plus, Trash2 } from 'lucide-react';
import type { PortfolioData, AccountData, CreatePortfolioRequest, UpdatePortfolioRequest } from '../../lib/types';
import { createPPPortfolio, updatePPPortfolio, getAccounts } from '../../lib/api';
import { useEscapeKey } from '../../lib/hooks';

// Key-Value entry for attributes
interface KeyValueEntry {
  key: string;
  value: string;
}

// Helper functions for converting between Record and array of entries
function recordToEntries(record: Record<string, string> | undefined): KeyValueEntry[] {
  if (!record) return [];
  return Object.entries(record).map(([key, value]) => ({ key, value }));
}

function entriesToRecord(entries: KeyValueEntry[]): Record<string, string> | undefined {
  const filtered = entries.filter((e) => e.key.trim() !== '');
  if (filtered.length === 0) return undefined;
  return Object.fromEntries(filtered.map((e) => [e.key, e.value]));
}

// Collapsible Key-Value Editor component
function KeyValueEditor({
  title,
  entries,
  onChange,
  expanded,
  onToggleExpand,
}: {
  title: string;
  entries: KeyValueEntry[];
  onChange: (entries: KeyValueEntry[]) => void;
  expanded: boolean;
  onToggleExpand: () => void;
}) {
  const addEntry = () => {
    onChange([...entries, { key: '', value: '' }]);
  };

  const removeEntry = (index: number) => {
    onChange(entries.filter((_, i) => i !== index));
  };

  const updateEntry = (index: number, field: 'key' | 'value', value: string) => {
    const newEntries = [...entries];
    newEntries[index] = { ...newEntries[index], [field]: value };
    onChange(newEntries);
  };

  return (
    <div className="border border-border rounded-md">
      <button
        type="button"
        onClick={onToggleExpand}
        className="w-full flex items-center gap-2 p-3 text-sm font-medium hover:bg-muted/50 transition-colors"
      >
        {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
        {title} ({entries.length})
      </button>
      {expanded && (
        <div className="p-3 pt-0 space-y-2">
          {entries.map((entry, index) => (
            <div key={index} className="flex gap-2">
              <input
                type="text"
                value={entry.key}
                onChange={(e) => updateEntry(index, 'key', e.target.value)}
                placeholder="Schlüssel"
                className="flex-1 px-2 py-1 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
              />
              <input
                type="text"
                value={entry.value}
                onChange={(e) => updateEntry(index, 'value', e.target.value)}
                placeholder="Wert"
                className="flex-1 px-2 py-1 text-sm border border-border rounded bg-background focus:outline-none focus:ring-1 focus:ring-primary"
              />
              <button
                type="button"
                onClick={() => removeEntry(index)}
                className="p-1 text-muted-foreground hover:text-destructive transition-colors"
              >
                <Trash2 size={16} />
              </button>
            </div>
          ))}
          <button
            type="button"
            onClick={addEntry}
            className="flex items-center gap-1 text-sm text-primary hover:text-primary/80 transition-colors"
          >
            <Plus size={14} />
            Attribut hinzufügen
          </button>
        </div>
      )}
    </div>
  );
}

interface PortfolioFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  portfolio?: PortfolioData | null; // null = create mode, PortfolioData = edit mode
}

export function PortfolioFormModal({ isOpen, onClose, onSuccess, portfolio }: PortfolioFormModalProps) {
  useEscapeKey(isOpen, onClose);

  const isEditMode = !!portfolio;

  const [formData, setFormData] = useState({
    name: '',
    referenceAccountId: '',
    note: '',
    isRetired: false,
  });

  const [attributes, setAttributes] = useState<KeyValueEntry[]>([]);
  const [attributesExpanded, setAttributesExpanded] = useState(false);
  const [accounts, setAccounts] = useState<AccountData[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isLoadingAccounts, setIsLoadingAccounts] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load accounts when modal opens
  useEffect(() => {
    if (isOpen) {
      setIsLoadingAccounts(true);
      getAccounts()
        .then(setAccounts)
        .catch((err) => console.error('Failed to load accounts:', err))
        .finally(() => setIsLoadingAccounts(false));
    }
  }, [isOpen]);

  // Reset form when modal opens or portfolio changes
  useEffect(() => {
    if (isOpen) {
      if (portfolio) {
        setFormData({
          name: portfolio.name || '',
          referenceAccountId: portfolio.referenceAccountId?.toString() || '',
          note: portfolio.note || '',
          isRetired: portfolio.isRetired || false,
        });
        setAttributes(recordToEntries(portfolio.attributes));
      } else {
        setFormData({
          name: '',
          referenceAccountId: '',
          note: '',
          isRetired: false,
        });
        setAttributes([]);
      }
      setAttributesExpanded(false);
      setError(null);
    }
  }, [isOpen, portfolio]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      if (isEditMode && portfolio) {
        const updateData: UpdatePortfolioRequest = {
          name: formData.name || undefined,
          referenceAccountId: formData.referenceAccountId ? parseInt(formData.referenceAccountId) : undefined,
          note: formData.note || undefined,
          isRetired: formData.isRetired,
          attributes: entriesToRecord(attributes),
        };
        await updatePPPortfolio(portfolio.id, updateData);
      } else {
        const createData: CreatePortfolioRequest = {
          name: formData.name,
          referenceAccountId: formData.referenceAccountId ? parseInt(formData.referenceAccountId) : undefined,
          note: formData.note || undefined,
          attributes: entriesToRecord(attributes),
        };
        await createPPPortfolio(createData);
      }
      onSuccess();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md mx-4">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-lg font-semibold">
            {isEditMode ? 'Portfolio bearbeiten' : 'Neues Portfolio'}
          </h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-4 space-y-4">
          {error && (
            <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
              {error}
            </div>
          )}

          {/* Name */}
          <div>
            <label className="block text-sm font-medium mb-1">
              Name <span className="text-destructive">*</span>
            </label>
            <input
              type="text"
              name="name"
              value={formData.name}
              onChange={handleChange}
              required
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder="z.B. Depot"
            />
          </div>

          {/* Reference Account */}
          <div>
            <label className="block text-sm font-medium mb-1">Referenzkonto</label>
            <select
              name="referenceAccountId"
              value={formData.referenceAccountId}
              onChange={handleChange}
              disabled={isLoadingAccounts}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
            >
              <option value="">Kein Referenzkonto</option>
              {accounts
                .filter((a) => !a.isRetired)
                .map((account) => (
                  <option key={account.id} value={account.id}>
                    {account.name} ({account.currency})
                  </option>
                ))}
            </select>
            <p className="text-xs text-muted-foreground mt-1">
              Das Referenzkonto wird für Käufe und Verkäufe verwendet.
            </p>
          </div>

          {/* Note */}
          <div>
            <label className="block text-sm font-medium mb-1">Notiz</label>
            <textarea
              name="note"
              value={formData.note}
              onChange={handleChange}
              rows={2}
              className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary resize-none"
              placeholder="Optionale Notizen..."
            />
          </div>

          {/* Is Retired (only in edit mode) */}
          {isEditMode && (
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="isRetired"
                checked={formData.isRetired}
                onChange={(e) => setFormData((prev) => ({ ...prev, isRetired: e.target.checked }))}
                className="h-4 w-4 rounded border-border text-primary focus:ring-primary"
              />
              <label htmlFor="isRetired" className="text-sm font-medium">
                Inaktiv (Retired)
              </label>
            </div>
          )}

          {/* Attributes */}
          <KeyValueEditor
            title="Attribute"
            entries={attributes}
            onChange={setAttributes}
            expanded={attributesExpanded}
            onToggleExpand={() => setAttributesExpanded(!attributesExpanded)}
          />

          {/* Actions */}
          <div className="flex justify-end gap-3 pt-4 border-t border-border">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 border border-border rounded-md hover:bg-muted transition-colors"
            >
              Abbrechen
            </button>
            <button
              type="submit"
              disabled={isSubmitting || !formData.name}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? 'Speichern...' : isEditMode ? 'Speichern' : 'Erstellen'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
