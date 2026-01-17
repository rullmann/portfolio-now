/**
 * Component for editing custom attribute values on a security.
 * Shows attribute types defined in the system and allows setting values.
 */

import { useState, useEffect, useCallback } from 'react';
import { ChevronRight, Tag, Loader2, Check, X, Calendar } from 'lucide-react';
import type { AttributeValue } from '../../lib/types';
import {
  getAttributeTypes,
  getSecurityAttributes,
  setSecurityAttribute,
  removeSecurityAttribute,
} from '../../lib/api';
import { toast } from '../../store';

const DATA_TYPE_LABELS: Record<string, string> = {
  STRING: 'Text',
  LONG_NUMBER: 'Ganzzahl',
  DOUBLE_NUMBER: 'Dezimalzahl',
  DATE: 'Datum',
  BOOLEAN: 'Ja/Nein',
  LIMIT_PRICE: 'Limitpreis',
  SHARE: 'Anteil',
};

interface AttributeInputProps {
  attribute: AttributeValue;
  securityId: number;
  onSaved: () => void;
}

function AttributeInput({ attribute, securityId, onSaved }: AttributeInputProps) {
  const [value, setValue] = useState(attribute.value || '');
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    setValue(attribute.value || '');
    setHasChanges(false);
  }, [attribute.value]);

  const handleChange = (newValue: string) => {
    setValue(newValue);
    setHasChanges(newValue !== (attribute.value || ''));
  };

  const handleSave = async () => {
    if (!hasChanges) return;

    setIsSaving(true);
    try {
      if (value.trim()) {
        await setSecurityAttribute({
          securityId,
          attributeTypeId: attribute.attributeTypeId,
          value: value.trim(),
        });
      } else {
        await removeSecurityAttribute(securityId, attribute.attributeTypeId);
      }
      setHasChanges(false);
      onSaved();
    } catch (err) {
      toast.error(`Fehler: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleClear = async () => {
    if (!attribute.value) return;

    setIsSaving(true);
    try {
      await removeSecurityAttribute(securityId, attribute.attributeTypeId);
      setValue('');
      setHasChanges(false);
      onSaved();
    } catch (err) {
      toast.error(`Fehler: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const renderInput = () => {
    switch (attribute.dataType) {
      case 'BOOLEAN':
        return (
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => handleChange(value === 'true' ? 'false' : 'true')}
              className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors ${
                value === 'true' ? 'bg-primary' : 'bg-muted'
              }`}
            >
              <span
                className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow transition ${
                  value === 'true' ? 'translate-x-5' : 'translate-x-0'
                }`}
              />
            </button>
            <span className="text-sm">{value === 'true' ? 'Ja' : 'Nein'}</span>
          </div>
        );

      case 'DATE':
        return (
          <div className="relative">
            <input
              type="date"
              value={value}
              onChange={(e) => handleChange(e.target.value)}
              className="w-full px-2 py-1.5 text-sm border border-border rounded bg-background pr-8"
            />
            <Calendar size={14} className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none" />
          </div>
        );

      case 'LONG_NUMBER':
        return (
          <input
            type="number"
            value={value}
            onChange={(e) => handleChange(e.target.value)}
            step="1"
            className="w-full px-2 py-1.5 text-sm border border-border rounded bg-background font-mono"
            placeholder="0"
          />
        );

      case 'DOUBLE_NUMBER':
      case 'LIMIT_PRICE':
      case 'SHARE':
        return (
          <input
            type="number"
            value={value}
            onChange={(e) => handleChange(e.target.value)}
            step="0.01"
            className="w-full px-2 py-1.5 text-sm border border-border rounded bg-background font-mono"
            placeholder="0.00"
          />
        );

      default: // STRING
        return (
          <input
            type="text"
            value={value}
            onChange={(e) => handleChange(e.target.value)}
            className="w-full px-2 py-1.5 text-sm border border-border rounded bg-background"
            placeholder="Wert eingeben..."
          />
        );
    }
  };

  return (
    <div className="flex items-start gap-2 group">
      <div className="flex-1">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-sm font-medium">{attribute.attributeTypeName}</span>
          <span className="text-xs text-muted-foreground">
            ({DATA_TYPE_LABELS[attribute.dataType] || attribute.dataType})
          </span>
        </div>
        {renderInput()}
      </div>
      <div className="flex items-center gap-1 pt-6">
        {hasChanges && (
          <button
            type="button"
            onClick={handleSave}
            disabled={isSaving}
            className="p-1.5 text-green-600 hover:bg-green-100 dark:hover:bg-green-900/30 rounded disabled:opacity-50"
            title="Speichern"
          >
            {isSaving ? <Loader2 size={14} className="animate-spin" /> : <Check size={14} />}
          </button>
        )}
        {attribute.value && (
          <button
            type="button"
            onClick={handleClear}
            disabled={isSaving}
            className="p-1.5 text-muted-foreground hover:text-destructive hover:bg-muted rounded opacity-0 group-hover:opacity-100 transition-opacity disabled:opacity-50"
            title="Wert löschen"
          >
            <X size={14} />
          </button>
        )}
      </div>
    </div>
  );
}

interface SecurityAttributesEditorProps {
  securityId: number;
  expanded?: boolean;
  onToggleExpand?: () => void;
}

export function SecurityAttributesEditor({
  securityId,
  expanded = false,
  onToggleExpand,
}: SecurityAttributesEditorProps) {
  const [attributes, setAttributes] = useState<AttributeValue[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [hasTypes, setHasTypes] = useState<boolean | null>(null);

  const loadAttributes = useCallback(async () => {
    setIsLoading(true);
    try {
      // First check if any attribute types exist
      const types = await getAttributeTypes('security');
      setHasTypes(types.length > 0);

      if (types.length > 0) {
        const attrs = await getSecurityAttributes(securityId);
        setAttributes(attrs);
      }
    } catch (err) {
      toast.error(`Fehler beim Laden: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsLoading(false);
    }
  }, [securityId]);

  useEffect(() => {
    if (expanded) {
      loadAttributes();
    }
  }, [expanded, securityId, loadAttributes]);

  const filledCount = attributes.filter(a => a.value).length;

  return (
    <div className="border border-border rounded-md overflow-hidden">
      <button
        type="button"
        onClick={onToggleExpand}
        className="w-full flex items-center justify-between p-3 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
      >
        <div className="flex items-center gap-2">
          <ChevronRight
            size={16}
            className={`transition-transform ${expanded ? 'rotate-90' : ''}`}
          />
          <Tag size={16} className="text-primary" />
          <span className="text-sm font-medium">Benutzerdefinierte Attribute</span>
          {attributes.length > 0 && (
            <span className="text-xs text-muted-foreground">
              ({filledCount}/{attributes.length})
            </span>
          )}
        </div>
      </button>

      {expanded && (
        <div className="p-3 border-t border-border bg-card space-y-4">
          {isLoading ? (
            <div className="flex items-center justify-center py-4 text-muted-foreground">
              <Loader2 size={16} className="animate-spin mr-2" />
              Lade Attribute...
            </div>
          ) : hasTypes === false ? (
            <div className="text-center py-4 text-sm text-muted-foreground">
              <p>Keine Attribut-Typen definiert.</p>
              <p className="mt-1 text-xs">
                Erstelle Attribut-Typen in den Einstellungen unter "Benutzerdefinierte Attribute".
              </p>
            </div>
          ) : attributes.length === 0 ? (
            <div className="text-center py-4 text-sm text-muted-foreground">
              Keine Attribute verfügbar
            </div>
          ) : (
            <div className="space-y-3">
              {attributes.map((attr) => (
                <AttributeInput
                  key={attr.attributeTypeId}
                  attribute={attr}
                  securityId={securityId}
                  onSaved={loadAttributes}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default SecurityAttributesEditor;
