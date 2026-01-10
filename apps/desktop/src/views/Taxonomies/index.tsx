/**
 * Taxonomies view for classification management.
 */

import { useState, useEffect } from 'react';
import { FolderTree, Plus, ChevronRight, ChevronDown, Edit2, Trash2, RefreshCw } from 'lucide-react';
import { getTaxonomies, getClassificationTree, createStandardTaxonomies, deleteTaxonomy, deleteClassification } from '../../lib/api';
import type { TaxonomyData, ClassificationData } from '../../lib/types';
import { TaxonomyFormModal } from '../../components/modals';
import { toast } from '../../store';

export function TaxonomiesView() {
  const [taxonomies, setTaxonomies] = useState<TaxonomyData[]>([]);
  const [selectedTaxonomy, setSelectedTaxonomy] = useState<number | null>(null);
  const [classificationTree, setClassificationTree] = useState<ClassificationData[]>([]);
  const [expandedNodes, setExpandedNodes] = useState<Set<number>>(new Set());
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Modal state
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [modalMode, setModalMode] = useState<'taxonomy' | 'classification'>('taxonomy');
  const [editingTaxonomy, setEditingTaxonomy] = useState<TaxonomyData | undefined>(undefined);
  const [editingClassification, setEditingClassification] = useState<ClassificationData | undefined>(undefined);
  const [parentClassificationId, setParentClassificationId] = useState<number | undefined>(undefined);

  const loadTaxonomies = async () => {
    try {
      setIsLoading(true);
      const data = await getTaxonomies();
      setTaxonomies(data);
      if (data.length > 0 && !selectedTaxonomy) {
        setSelectedTaxonomy(data[0].id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const loadClassifications = async (taxonomyId: number) => {
    try {
      const data = await getClassificationTree(taxonomyId);
      setClassificationTree(data);
    } catch (err) {
      console.error('Error loading classification tree:', err);
    }
  };

  useEffect(() => {
    loadTaxonomies();
  }, []);

  useEffect(() => {
    if (selectedTaxonomy) {
      loadClassifications(selectedTaxonomy);
    }
  }, [selectedTaxonomy]);

  const handleCreateStandardTaxonomies = async () => {
    try {
      setIsLoading(true);
      await createStandardTaxonomies();
      await loadTaxonomies();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const toggleNode = (nodeId: number) => {
    setExpandedNodes(prev => {
      const next = new Set(prev);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  };

  // Modal handlers
  const handleCreateTaxonomy = () => {
    setModalMode('taxonomy');
    setEditingTaxonomy(undefined);
    setEditingClassification(undefined);
    setParentClassificationId(undefined);
    setIsModalOpen(true);
  };

  const handleEditTaxonomy = (taxonomy: TaxonomyData) => {
    setModalMode('taxonomy');
    setEditingTaxonomy(taxonomy);
    setEditingClassification(undefined);
    setParentClassificationId(undefined);
    setIsModalOpen(true);
  };

  const handleCreateClassification = (parentId?: number) => {
    if (!selectedTaxonomy) return;
    setModalMode('classification');
    setEditingTaxonomy(undefined);
    setEditingClassification(undefined);
    setParentClassificationId(parentId);
    setIsModalOpen(true);
  };

  const handleEditClassification = (classification: ClassificationData, e: React.MouseEvent) => {
    e.stopPropagation();
    setModalMode('classification');
    setEditingTaxonomy(undefined);
    setEditingClassification(classification);
    setParentClassificationId(classification.parentId);
    setIsModalOpen(true);
  };

  const handleDeleteTaxonomy = async (taxonomy: TaxonomyData, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm(`Taxonomie "${taxonomy.name}" wirklich löschen?`)) return;
    try {
      await deleteTaxonomy(taxonomy.id);
      toast.success('Taxonomie gelöscht');
      if (selectedTaxonomy === taxonomy.id) {
        setSelectedTaxonomy(null);
        setClassificationTree([]);
      }
      await loadTaxonomies();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleDeleteClassification = async (classification: ClassificationData, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm(`Klassifikation "${classification.name}" wirklich löschen?`)) return;
    try {
      await deleteClassification(classification.id);
      toast.success('Klassifikation gelöscht');
      if (selectedTaxonomy) {
        await loadClassifications(selectedTaxonomy);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setEditingTaxonomy(undefined);
    setEditingClassification(undefined);
    setParentClassificationId(undefined);
  };

  const handleModalSuccess = async () => {
    handleModalClose();
    await loadTaxonomies();
    if (selectedTaxonomy) {
      await loadClassifications(selectedTaxonomy);
    }
    toast.success(
      editingTaxonomy || editingClassification
        ? 'Erfolgreich aktualisiert'
        : 'Erfolgreich erstellt'
    );
  };

  const renderClassificationNode = (node: ClassificationData, depth: number = 0) => {
    const hasChildren = node.children && node.children.length > 0;
    const isExpanded = expandedNodes.has(node.id);
    const weightPercent = node.weight !== undefined ? node.weight / 100 : 0;

    return (
      <div key={node.id}>
        <div
          className={`group flex items-center gap-2 py-2 px-3 hover:bg-muted/50 rounded-md cursor-pointer`}
          style={{ paddingLeft: `${depth * 20 + 12}px` }}
          onClick={() => hasChildren && toggleNode(node.id)}
        >
          {hasChildren ? (
            isExpanded ? (
              <ChevronDown size={16} className="text-muted-foreground" />
            ) : (
              <ChevronRight size={16} className="text-muted-foreground" />
            )
          ) : (
            <div className="w-4" />
          )}

          {node.color && (
            <div
              className="w-3 h-3 rounded-full"
              style={{ backgroundColor: node.color }}
            />
          )}

          <span className="flex-1 font-medium text-sm">{node.name}</span>

          {weightPercent > 0 && (
            <span className="text-xs text-muted-foreground">
              {weightPercent.toFixed(1)}%
            </span>
          )}

          {node.assignmentsCount > 0 && (
            <span className="text-xs bg-muted px-1.5 py-0.5 rounded">
              {node.assignmentsCount}
            </span>
          )}

          <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleCreateClassification(node.id);
              }}
              className="p-1 hover:bg-background rounded-md"
              title="Unterklassifikation hinzufügen"
            >
              <Plus size={14} className="text-muted-foreground" />
            </button>
            <button
              onClick={(e) => handleEditClassification(node, e)}
              className="p-1 hover:bg-background rounded-md"
              title="Bearbeiten"
            >
              <Edit2 size={14} className="text-muted-foreground" />
            </button>
            <button
              onClick={(e) => handleDeleteClassification(node, e)}
              className="p-1 hover:bg-destructive/10 rounded-md"
              title="Löschen"
            >
              <Trash2 size={14} className="text-destructive" />
            </button>
          </div>
        </div>

        {hasChildren && isExpanded && (
          <div>
            {node.children.map(child => renderClassificationNode(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <FolderTree className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">Klassifizierung</h1>
        </div>
        <div className="flex gap-2">
          <button
            onClick={loadTaxonomies}
            disabled={isLoading}
            className="flex items-center gap-2 px-3 py-2 border border-border rounded-md hover:bg-muted transition-colors"
          >
            <RefreshCw size={16} className={isLoading ? 'animate-spin' : ''} />
            Aktualisieren
          </button>
          {taxonomies.length === 0 ? (
            <button
              onClick={handleCreateStandardTaxonomies}
              disabled={isLoading}
              className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
            >
              <Plus size={16} />
              Standard-Taxonomien erstellen
            </button>
          ) : (
            <button
              onClick={handleCreateTaxonomy}
              className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
            >
              <Plus size={16} />
              Neue Taxonomie
            </button>
          )}
        </div>
      </div>

      {error && (
        <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-md text-destructive text-sm">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-6">
        {/* Taxonomy sidebar */}
        <div className="lg:col-span-1">
          <div className="bg-card rounded-lg border border-border p-4">
            <div className="flex items-center justify-between mb-4">
              <h2 className="font-semibold">Taxonomien</h2>
              <button
                className="p-1 hover:bg-muted rounded-md transition-colors"
                title="Neue Taxonomie"
              >
                <Plus size={18} />
              </button>
            </div>

            <div className="space-y-1">
              {taxonomies.map((taxonomy) => (
                <div
                  key={taxonomy.id}
                  className={`group p-3 rounded-md cursor-pointer transition-colors ${
                    selectedTaxonomy === taxonomy.id
                      ? 'bg-primary text-primary-foreground'
                      : 'hover:bg-muted'
                  }`}
                  onClick={() => setSelectedTaxonomy(taxonomy.id)}
                >
                  <div className="flex items-center justify-between">
                    <div className="font-medium text-sm">{taxonomy.name}</div>
                    <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleEditTaxonomy(taxonomy);
                        }}
                        className={`p-1 rounded-md ${
                          selectedTaxonomy === taxonomy.id
                            ? 'hover:bg-primary-foreground/10'
                            : 'hover:bg-muted'
                        }`}
                        title="Bearbeiten"
                      >
                        <Edit2 size={12} />
                      </button>
                      <button
                        onClick={(e) => handleDeleteTaxonomy(taxonomy, e)}
                        className={`p-1 rounded-md ${
                          selectedTaxonomy === taxonomy.id
                            ? 'hover:bg-primary-foreground/10'
                            : 'hover:bg-destructive/10'
                        }`}
                        title="Löschen"
                      >
                        <Trash2 size={12} className={selectedTaxonomy === taxonomy.id ? '' : 'text-destructive'} />
                      </button>
                    </div>
                  </div>
                  <div className={`text-xs ${selectedTaxonomy === taxonomy.id ? 'text-primary-foreground/70' : 'text-muted-foreground'}`}>
                    {taxonomy.classificationsCount} Klassifikationen
                    {taxonomy.source && ` · ${taxonomy.source}`}
                  </div>
                </div>
              ))}

              {taxonomies.length === 0 && !isLoading && (
                <div className="text-sm text-muted-foreground text-center py-4">
                  Keine Taxonomien vorhanden.
                  <br />
                  Erstellen Sie Standard-Taxonomien.
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Classification tree */}
        <div className="lg:col-span-3">
          <div className="bg-card rounded-lg border border-border">
            {selectedTaxonomy ? (
              classificationTree.length > 0 ? (
                <div className="p-4">
                  <div className="flex items-center justify-between mb-4">
                    <h3 className="font-semibold">
                      {taxonomies.find(t => t.id === selectedTaxonomy)?.name}
                    </h3>
                    <div className="flex gap-2">
                      <button
                        onClick={() => handleCreateClassification()}
                        className="flex items-center gap-1 px-2 py-1 text-sm hover:bg-muted rounded-md transition-colors"
                      >
                        <Plus size={14} />
                        Klassifikation
                      </button>
                    </div>
                  </div>
                  <div className="border border-border rounded-md">
                    {classificationTree.map(node => renderClassificationNode(node))}
                  </div>
                </div>
              ) : (
                <div className="p-8 text-center text-muted-foreground">
                  <FolderTree className="w-12 h-12 mx-auto mb-3 opacity-50" />
                  <p>Diese Taxonomie hat noch keine Klassifikationen.</p>
                  <button
                    onClick={() => handleCreateClassification()}
                    className="mt-4 flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors mx-auto"
                  >
                    <Plus size={16} />
                    Klassifikation erstellen
                  </button>
                </div>
              )
            ) : (
              <div className="p-8 text-center text-muted-foreground">
                <FolderTree className="w-12 h-12 mx-auto mb-3 opacity-50" />
                <p>Wählen Sie eine Taxonomie aus.</p>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Taxonomy/Classification Form Modal */}
      <TaxonomyFormModal
        isOpen={isModalOpen}
        onClose={handleModalClose}
        onSuccess={handleModalSuccess}
        mode={modalMode}
        taxonomy={editingTaxonomy}
        classification={editingClassification}
        taxonomyId={selectedTaxonomy || undefined}
        parentId={parentClassificationId}
      />
    </div>
  );
}
