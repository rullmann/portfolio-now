/**
 * Component for managing custom attribute type definitions.
 * Used in Settings to define attribute types that can be used on securities.
 */

import { useState, useEffect, useCallback } from 'react';
import { Plus, Trash2, Edit2, Save, X, ChevronDown, ChevronRight, Tag } from 'lucide-react';
import type { AttributeType, CreateAttributeTypeRequest } from '../../lib/types';
import {
  getAttributeTypes,
  createAttributeType,
  updateAttributeType,
  deleteAttributeType,
} from '../../lib/api';
import { toast } from '../../store';

const DATA_TYPES = [
  { value: 'STRING', label: 'Text' },
  { value: 'LONG_NUMBER', label: 'Ganzzahl' },
  { value: 'DOUBLE_NUMBER', label: 'Dezimalzahl' },
  { value: 'DATE', label: 'Datum' },
  { value: 'BOOLEAN', label: 'Ja/Nein' },
  { value: 'LIMIT_PRICE', label: 'Limitpreis' },
  { value: 'SHARE', label: 'Anteil' },
];

const TARGET_TYPES = [
  { value: 'security', label: 'Wertpapier' },
  { value: 'account', label: 'Konto' },
  { value: 'portfolio', label: 'Depot' },
];

interface AttributeTypeRowProps {
  attribute: AttributeType;
  onUpdate: () => void;
  onDelete: () => void;
}

function AttributeTypeRow({ attribute, onUpdate, onDelete }: AttributeTypeRowProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [editForm, setEditForm] = useState({
    name: attribute.name,
    columnLabel: attribute.columnLabel || '',
    dataType: attribute.dataType,
  });
  const [isDeleting, setIsDeleting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await updateAttributeType(attribute.id, {
        name: editForm.name || undefined,
        columnLabel: editForm.columnLabel || undefined,
        dataType: editForm.dataType || undefined,
      });
      toast.success('Attribut-Typ aktualisiert');
      setIsEditing(false);
      onUpdate();
    } catch (err) {
      toast.error(`Fehler: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDelete = async () => {
    setIsDeleting(true);
    try {
      await deleteAttributeType(attribute.id);
      toast.success('Attribut-Typ gelöscht');
      onDelete();
    } catch (err) {
      toast.error(`Fehler: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsDeleting(false);
    }
  };

  const dataTypeLabel = DATA_TYPES.find(dt => dt.value === attribute.dataType)?.label || attribute.dataType;
  const targetLabel = TARGET_TYPES.find(t => t.value === attribute.target)?.label || attribute.target;

  if (isEditing) {
    return (
      <div className="p-3 border border-primary/50 rounded-md bg-primary/5 space-y-3">
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-xs font-medium text-muted-foreground">Name</label>
            <input
              type="text"
              value={editForm.name}
              onChange={(e) => setEditForm(prev => ({ ...prev, name: e.target.value }))}
              className="w-full mt-1 px-2 py-1.5 text-sm border border-border rounded bg-background"
              placeholder="Attribut-Name"
            />
          </div>
          <div>
            <label className="text-xs font-medium text-muted-foreground">Spaltenbezeichnung</label>
            <input
              type="text"
              value={editForm.columnLabel}
              onChange={(e) => setEditForm(prev => ({ ...prev, columnLabel: e.target.value }))}
              className="w-full mt-1 px-2 py-1.5 text-sm border border-border rounded bg-background"
              placeholder="Optional"
            />
          </div>
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground">Datentyp</label>
          <select
            value={editForm.dataType}
            onChange={(e) => setEditForm(prev => ({ ...prev, dataType: e.target.value as typeof prev.dataType }))}
            className="w-full mt-1 px-2 py-1.5 text-sm border border-border rounded bg-background"
          >
            {DATA_TYPES.map(dt => (
              <option key={dt.value} value={dt.value}>{dt.label}</option>
            ))}
          </select>
        </div>
        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={() => setIsEditing(false)}
            className="px-3 py-1.5 text-sm border border-border rounded hover:bg-muted"
          >
            <X size={14} />
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={isSaving || !editForm.name.trim()}
            className="px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50 flex items-center gap-1"
          >
            <Save size={14} />
            Speichern
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center justify-between p-3 border border-border rounded-md hover:bg-muted/30 group">
      <div className="flex items-center gap-3 flex-1 min-w-0">
        <Tag size={16} className="text-muted-foreground shrink-0" />
        <div className="min-w-0">
          <div className="font-medium truncate">{attribute.name}</div>
          <div className="text-xs text-muted-foreground flex items-center gap-2">
            <span>{dataTypeLabel}</span>
            <span className="text-muted-foreground/50">|</span>
            <span>{targetLabel}</span>
            {attribute.columnLabel && (
              <>
                <span className="text-muted-foreground/50">|</span>
                <span>Spalte: {attribute.columnLabel}</span>
              </>
            )}
          </div>
        </div>
      </div>
      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <button
          type="button"
          onClick={() => setIsEditing(true)}
          className="p-1.5 text-muted-foreground hover:text-foreground rounded hover:bg-muted"
          title="Bearbeiten"
        >
          <Edit2 size={14} />
        </button>
        <button
          type="button"
          onClick={handleDelete}
          disabled={isDeleting}
          className="p-1.5 text-muted-foreground hover:text-destructive rounded hover:bg-muted disabled:opacity-50"
          title="Löschen"
        >
          <Trash2 size={14} />
        </button>
      </div>
    </div>
  );
}

interface AttributeTypeManagerProps {
  expanded?: boolean;
  onToggleExpand?: () => void;
}

export function AttributeTypeManager({ expanded = false, onToggleExpand }: AttributeTypeManagerProps) {
  const [attributeTypes, setAttributeTypes] = useState<AttributeType[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [createForm, setCreateForm] = useState<CreateAttributeTypeRequest>({
    name: '',
    columnLabel: '',
    target: 'security',
    dataType: 'STRING',
  });
  const [isCreating, setIsCreating] = useState(false);

  const loadAttributeTypes = useCallback(async () => {
    setIsLoading(true);
    try {
      const types = await getAttributeTypes();
      setAttributeTypes(types);
    } catch (err) {
      toast.error(`Fehler beim Laden: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (expanded) {
      loadAttributeTypes();
    }
  }, [expanded, loadAttributeTypes]);

  const handleCreate = async () => {
    if (!createForm.name.trim()) {
      toast.error('Name ist erforderlich');
      return;
    }

    setIsCreating(true);
    try {
      await createAttributeType(createForm);
      toast.success('Attribut-Typ erstellt');
      setShowCreateForm(false);
      setCreateForm({
        name: '',
        columnLabel: '',
        target: 'security',
        dataType: 'STRING',
      });
      loadAttributeTypes();
    } catch (err) {
      toast.error(`Fehler: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsCreating(false);
    }
  };

  // Group by target
  const groupedTypes = attributeTypes.reduce((acc, type) => {
    const target = type.target;
    if (!acc[target]) acc[target] = [];
    acc[target].push(type);
    return acc;
  }, {} as Record<string, AttributeType[]>);

  return (
    <div className="border border-border rounded-md overflow-hidden">
      <button
        type="button"
        onClick={onToggleExpand}
        className="w-full flex items-center justify-between p-4 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
      >
        <div className="flex items-center gap-2">
          {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          <Tag size={18} className="text-primary" />
          <span className="font-medium">Benutzerdefinierte Attribute</span>
          <span className="text-xs text-muted-foreground">
            ({attributeTypes.length})
          </span>
        </div>
      </button>

      {expanded && (
        <div className="p-4 border-t border-border space-y-4">
          <p className="text-sm text-muted-foreground">
            Erstelle eigene Attribute für Wertpapiere, Konten oder Depots.
            Diese können z.B. für Kategorisierung, Notizen oder zusätzliche Kennzahlen verwendet werden.
          </p>

          {isLoading ? (
            <div className="text-center py-4 text-muted-foreground">
              Lade Attribute...
            </div>
          ) : (
            <>
              {/* Grouped list */}
              {Object.entries(groupedTypes).map(([target, types]) => {
                const targetLabel = TARGET_TYPES.find(t => t.value === target)?.label || target;
                return (
                  <div key={target}>
                    <h4 className="text-sm font-medium text-muted-foreground mb-2">
                      {targetLabel}
                    </h4>
                    <div className="space-y-2">
                      {types.map(attr => (
                        <AttributeTypeRow
                          key={attr.id}
                          attribute={attr}
                          onUpdate={loadAttributeTypes}
                          onDelete={loadAttributeTypes}
                        />
                      ))}
                    </div>
                  </div>
                );
              })}

              {attributeTypes.length === 0 && !showCreateForm && (
                <div className="text-center py-6 text-muted-foreground">
                  Keine Attribute definiert
                </div>
              )}

              {/* Create form */}
              {showCreateForm ? (
                <div className="p-4 border border-primary/50 rounded-md bg-primary/5 space-y-3">
                  <h4 className="font-medium">Neues Attribut erstellen</h4>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="text-xs font-medium text-muted-foreground">Name *</label>
                      <input
                        type="text"
                        value={createForm.name}
                        onChange={(e) => setCreateForm(prev => ({ ...prev, name: e.target.value }))}
                        className="w-full mt-1 px-2 py-1.5 text-sm border border-border rounded bg-background"
                        placeholder="z.B. Sektor"
                      />
                    </div>
                    <div>
                      <label className="text-xs font-medium text-muted-foreground">Spaltenbezeichnung</label>
                      <input
                        type="text"
                        value={createForm.columnLabel || ''}
                        onChange={(e) => setCreateForm(prev => ({ ...prev, columnLabel: e.target.value || undefined }))}
                        className="w-full mt-1 px-2 py-1.5 text-sm border border-border rounded bg-background"
                        placeholder="Optional"
                      />
                    </div>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <label className="text-xs font-medium text-muted-foreground">Ziel</label>
                      <select
                        value={createForm.target || 'security'}
                        onChange={(e) => setCreateForm(prev => ({ ...prev, target: e.target.value }))}
                        className="w-full mt-1 px-2 py-1.5 text-sm border border-border rounded bg-background"
                      >
                        {TARGET_TYPES.map(t => (
                          <option key={t.value} value={t.value}>{t.label}</option>
                        ))}
                      </select>
                    </div>
                    <div>
                      <label className="text-xs font-medium text-muted-foreground">Datentyp</label>
                      <select
                        value={createForm.dataType || 'STRING'}
                        onChange={(e) => setCreateForm(prev => ({ ...prev, dataType: e.target.value }))}
                        className="w-full mt-1 px-2 py-1.5 text-sm border border-border rounded bg-background"
                      >
                        {DATA_TYPES.map(dt => (
                          <option key={dt.value} value={dt.value}>{dt.label}</option>
                        ))}
                      </select>
                    </div>
                  </div>
                  <div className="flex justify-end gap-2 pt-2">
                    <button
                      type="button"
                      onClick={() => setShowCreateForm(false)}
                      className="px-3 py-1.5 text-sm border border-border rounded hover:bg-muted"
                    >
                      Abbrechen
                    </button>
                    <button
                      type="button"
                      onClick={handleCreate}
                      disabled={isCreating || !createForm.name.trim()}
                      className="px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50"
                    >
                      {isCreating ? 'Erstelle...' : 'Erstellen'}
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => setShowCreateForm(true)}
                  className="flex items-center gap-2 text-sm text-primary hover:text-primary/80 transition-colors"
                >
                  <Plus size={16} />
                  Neues Attribut erstellen
                </button>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}

export default AttributeTypeManager;
