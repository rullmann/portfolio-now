/**
 * Modal for creating and editing taxonomies and classifications.
 */

import { useState, useEffect } from 'react';
import { X, Palette } from 'lucide-react';
import type {
  TaxonomyData,
  ClassificationData,
  CreateTaxonomyRequest,
  UpdateTaxonomyRequest,
  CreateClassificationRequest,
  UpdateClassificationRequest,
} from '../../lib/types';
import {
  createTaxonomy,
  updateTaxonomy,
  createClassification,
  updateClassification,
  getClassifications,
} from '../../lib/api';

// Preset colors for classifications
const PRESET_COLORS = [
  '#3B82F6', // blue
  '#10B981', // emerald
  '#F59E0B', // amber
  '#EF4444', // red
  '#8B5CF6', // violet
  '#EC4899', // pink
  '#14B8A6', // teal
  '#F97316', // orange
  '#6366F1', // indigo
  '#84CC16', // lime
  '#06B6D4', // cyan
  '#A855F7', // purple
];

type ModalMode = 'taxonomy' | 'classification';

interface TaxonomyFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  mode: ModalMode;
  // For taxonomy mode
  taxonomy?: TaxonomyData | null;
  // For classification mode
  classification?: ClassificationData | null;
  taxonomyId?: number;
  parentId?: number;
}

export function TaxonomyFormModal({
  isOpen,
  onClose,
  onSuccess,
  mode,
  taxonomy,
  classification,
  taxonomyId,
  parentId,
}: TaxonomyFormModalProps) {
  const isEditMode = mode === 'taxonomy' ? !!taxonomy : !!classification;

  // Taxonomy form state
  const [taxonomyForm, setTaxonomyForm] = useState({
    name: '',
    source: '',
  });

  // Classification form state
  const [classificationForm, setClassificationForm] = useState({
    name: '',
    color: PRESET_COLORS[0],
    weight: '',
    parentId: '',
  });

  const [availableParents, setAvailableParents] = useState<ClassificationData[]>([]);
  const [showColorPicker, setShowColorPicker] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isLoadingParents, setIsLoadingParents] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load available parent classifications when in classification mode
  useEffect(() => {
    if (isOpen && mode === 'classification' && taxonomyId) {
      setIsLoadingParents(true);
      getClassifications(taxonomyId)
        .then((data) => {
          // Filter out the current classification (can't be its own parent)
          const filtered = classification
            ? data.filter((c) => c.id !== classification.id)
            : data;
          setAvailableParents(filtered);
        })
        .catch((err) => console.error('Failed to load classifications:', err))
        .finally(() => setIsLoadingParents(false));
    }
  }, [isOpen, mode, taxonomyId, classification]);

  // Reset form when modal opens
  useEffect(() => {
    if (isOpen) {
      if (mode === 'taxonomy') {
        if (taxonomy) {
          setTaxonomyForm({
            name: taxonomy.name || '',
            source: taxonomy.source || '',
          });
        } else {
          setTaxonomyForm({ name: '', source: '' });
        }
      } else {
        if (classification) {
          setClassificationForm({
            name: classification.name || '',
            color: classification.color || PRESET_COLORS[0],
            weight: classification.weight ? String(classification.weight / 100) : '',
            parentId: classification.parentId ? String(classification.parentId) : '',
          });
        } else {
          setClassificationForm({
            name: '',
            color: PRESET_COLORS[Math.floor(Math.random() * PRESET_COLORS.length)],
            weight: '',
            parentId: parentId ? String(parentId) : '',
          });
        }
      }
      setShowColorPicker(false);
      setError(null);
    }
  }, [isOpen, mode, taxonomy, classification, parentId]);

  const handleTaxonomyChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    setTaxonomyForm((prev) => ({ ...prev, [name]: value }));
  };

  const handleClassificationChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>
  ) => {
    const { name, value } = e.target;
    setClassificationForm((prev) => ({ ...prev, [name]: value }));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      if (mode === 'taxonomy') {
        if (isEditMode && taxonomy) {
          const updateData: UpdateTaxonomyRequest = {
            name: taxonomyForm.name || undefined,
            source: taxonomyForm.source || undefined,
          };
          await updateTaxonomy(taxonomy.id, updateData);
        } else {
          const createData: CreateTaxonomyRequest = {
            name: taxonomyForm.name,
            source: taxonomyForm.source || undefined,
          };
          await createTaxonomy(createData);
        }
      } else {
        // Classification mode
        if (isEditMode && classification) {
          const updateData: UpdateClassificationRequest = {
            name: classificationForm.name || undefined,
            color: classificationForm.color || undefined,
            weight: classificationForm.weight
              ? Math.round(parseFloat(classificationForm.weight) * 100)
              : undefined,
            parentId: classificationForm.parentId
              ? parseInt(classificationForm.parentId)
              : undefined,
          };
          await updateClassification(classification.id, updateData);
        } else {
          if (!taxonomyId) {
            throw new Error('Taxonomy ID is required');
          }
          const createData: CreateClassificationRequest = {
            taxonomyId,
            parentId: classificationForm.parentId
              ? parseInt(classificationForm.parentId)
              : undefined,
            name: classificationForm.name,
            color: classificationForm.color || undefined,
            weight: classificationForm.weight
              ? Math.round(parseFloat(classificationForm.weight) * 100)
              : undefined,
          };
          await createClassification(createData);
        }
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

  const title =
    mode === 'taxonomy'
      ? isEditMode
        ? 'Taxonomie bearbeiten'
        : 'Neue Taxonomie'
      : isEditMode
        ? 'Klassifikation bearbeiten'
        : 'Neue Klassifikation';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-xl w-full max-w-md mx-4">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <h2 className="text-lg font-semibold">{title}</h2>
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

          {mode === 'taxonomy' ? (
            <>
              {/* Taxonomy Name */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Name <span className="text-destructive">*</span>
                </label>
                <input
                  type="text"
                  name="name"
                  value={taxonomyForm.name}
                  onChange={handleTaxonomyChange}
                  required
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                  placeholder="z.B. Asset Allocation"
                />
              </div>

              {/* Source */}
              <div>
                <label className="block text-sm font-medium mb-1">Quelle</label>
                <input
                  type="text"
                  name="source"
                  value={taxonomyForm.source}
                  onChange={handleTaxonomyChange}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                  placeholder="z.B. MSCI, eigene"
                />
              </div>
            </>
          ) : (
            <>
              {/* Classification Name */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Name <span className="text-destructive">*</span>
                </label>
                <input
                  type="text"
                  name="name"
                  value={classificationForm.name}
                  onChange={handleClassificationChange}
                  required
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                  placeholder="z.B. Aktien"
                />
              </div>

              {/* Parent Classification */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Übergeordnete Klassifikation
                </label>
                <select
                  name="parentId"
                  value={classificationForm.parentId}
                  onChange={handleClassificationChange}
                  disabled={isLoadingParents}
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50"
                >
                  <option value="">Keine (Root-Ebene)</option>
                  {availableParents.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>

              {/* Color */}
              <div>
                <label className="block text-sm font-medium mb-1">Farbe</label>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => setShowColorPicker(!showColorPicker)}
                    className="flex items-center gap-2 px-3 py-2 border border-border rounded-md hover:bg-muted transition-colors"
                  >
                    <div
                      className="w-5 h-5 rounded"
                      style={{ backgroundColor: classificationForm.color }}
                    />
                    <Palette size={16} className="text-muted-foreground" />
                    <span className="text-sm">{classificationForm.color}</span>
                  </button>
                </div>
                {showColorPicker && (
                  <div className="mt-2 p-2 border border-border rounded-md bg-background">
                    <div className="grid grid-cols-6 gap-2">
                      {PRESET_COLORS.map((color) => (
                        <button
                          key={color}
                          type="button"
                          onClick={() => {
                            setClassificationForm((prev) => ({ ...prev, color }));
                            setShowColorPicker(false);
                          }}
                          className={`w-8 h-8 rounded-md border-2 transition-all ${
                            classificationForm.color === color
                              ? 'border-foreground scale-110'
                              : 'border-transparent hover:scale-105'
                          }`}
                          style={{ backgroundColor: color }}
                        />
                      ))}
                    </div>
                    <div className="mt-2">
                      <input
                        type="color"
                        value={classificationForm.color}
                        onChange={(e) =>
                          setClassificationForm((prev) => ({
                            ...prev,
                            color: e.target.value,
                          }))
                        }
                        className="w-full h-8 cursor-pointer"
                      />
                    </div>
                  </div>
                )}
              </div>

              {/* Weight */}
              <div>
                <label className="block text-sm font-medium mb-1">
                  Gewichtung (%)
                </label>
                <input
                  type="number"
                  name="weight"
                  value={classificationForm.weight}
                  onChange={handleClassificationChange}
                  step="0.01"
                  min="0"
                  max="100"
                  className="w-full px-3 py-2 border border-border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
                  placeholder="z.B. 60"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Optionale Zielgewichtung in Prozent
                </p>
              </div>
            </>
          )}

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
              disabled={
                isSubmitting ||
                (mode === 'taxonomy' ? !taxonomyForm.name : !classificationForm.name)
              }
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
