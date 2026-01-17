/**
 * Hook for managing dashboard layouts and widgets
 */

import { useState, useEffect, useCallback } from 'react';
import { toast } from '../store';
import {
  getDashboardLayout,
  saveDashboardLayout,
  getAvailableWidgets,
  createDefaultDashboardLayout,
} from '../lib/api';
import type {
  DashboardLayout,
  WidgetConfig,
  WidgetDefinition,
} from '../components/dashboard/types';

interface UseDashboardLayoutResult {
  layout: DashboardLayout | null;
  widgets: WidgetConfig[];
  availableWidgets: WidgetDefinition[];
  isLoading: boolean;
  isEditing: boolean;
  setIsEditing: (editing: boolean) => void;
  addWidget: (widgetType: string) => void;
  removeWidget: (widgetId: string) => void;
  updateWidget: (widgetId: string, updates: Partial<WidgetConfig>) => void;
  moveWidget: (widgetId: string, newPosition: { x: number; y: number }) => void;
  resizeWidget: (widgetId: string, newSize: { width: number; height: number }) => void;
  saveLayout: () => Promise<void>;
  resetLayout: () => Promise<void>;
}

export function useDashboardLayout(): UseDashboardLayoutResult {
  const [layout, setLayout] = useState<DashboardLayout | null>(null);
  const [availableWidgets, setAvailableWidgets] = useState<WidgetDefinition[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isEditing, setIsEditing] = useState(false);

  // Load layout and available widgets on mount
  useEffect(() => {
    async function loadData() {
      try {
        setIsLoading(true);

        // Load available widgets
        const widgets = await getAvailableWidgets();
        setAvailableWidgets(widgets);

        // Load current layout (or create default)
        let currentLayout = await getDashboardLayout();

        if (!currentLayout) {
          // Create default layout if none exists
          currentLayout = await createDefaultDashboardLayout();
        }

        setLayout(currentLayout);
      } catch (error) {
        console.error('Failed to load dashboard layout:', error);
        toast.error('Dashboard-Layout konnte nicht geladen werden');
      } finally {
        setIsLoading(false);
      }
    }

    loadData();
  }, []);

  // Generate unique widget ID
  const generateWidgetId = useCallback(() => {
    return `widget-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
  }, []);

  // Find next available position for a new widget
  const findNextPosition = useCallback(
    (width: number, height: number): { x: number; y: number } => {
      if (!layout) return { x: 0, y: 0 };

      const columns = layout.columns;
      const occupiedCells = new Set<string>();

      // Mark occupied cells
      for (const widget of layout.widgets) {
        for (let x = widget.position.x; x < widget.position.x + widget.size.width; x++) {
          for (let y = widget.position.y; y < widget.position.y + widget.size.height; y++) {
            occupiedCells.add(`${x},${y}`);
          }
        }
      }

      // Find first available position
      for (let y = 0; y < 100; y++) {
        for (let x = 0; x <= columns - width; x++) {
          let canPlace = true;

          for (let dx = 0; dx < width && canPlace; dx++) {
            for (let dy = 0; dy < height && canPlace; dy++) {
              if (occupiedCells.has(`${x + dx},${y + dy}`)) {
                canPlace = false;
              }
            }
          }

          if (canPlace) {
            return { x, y };
          }
        }
      }

      // Fallback: place at bottom
      const maxY = Math.max(0, ...layout.widgets.map((w) => w.position.y + w.size.height));
      return { x: 0, y: maxY };
    },
    [layout]
  );

  // Add a new widget
  const addWidget = useCallback(
    (widgetType: string) => {
      if (!layout) return;

      const definition = availableWidgets.find((w) => w.widget_type === widgetType);
      if (!definition) {
        toast.error('Unbekannter Widget-Typ');
        return;
      }

      const position = findNextPosition(definition.default_width, definition.default_height);

      const newWidget: WidgetConfig = {
        id: generateWidgetId(),
        widget_type: definition.widget_type,
        title: definition.label,
        position,
        size: {
          width: definition.default_width,
          height: definition.default_height,
        },
        settings: {},
      };

      setLayout({
        ...layout,
        widgets: [...layout.widgets, newWidget],
      });
    },
    [layout, availableWidgets, findNextPosition, generateWidgetId]
  );

  // Remove a widget
  const removeWidget = useCallback(
    (widgetId: string) => {
      if (!layout) return;

      setLayout({
        ...layout,
        widgets: layout.widgets.filter((w) => w.id !== widgetId),
      });
    },
    [layout]
  );

  // Update a widget
  const updateWidget = useCallback(
    (widgetId: string, updates: Partial<WidgetConfig>) => {
      if (!layout) return;

      setLayout({
        ...layout,
        widgets: layout.widgets.map((w) =>
          w.id === widgetId ? { ...w, ...updates } : w
        ),
      });
    },
    [layout]
  );

  // Move a widget
  const moveWidget = useCallback(
    (widgetId: string, newPosition: { x: number; y: number }) => {
      updateWidget(widgetId, { position: newPosition });
    },
    [updateWidget]
  );

  // Resize a widget
  const resizeWidget = useCallback(
    (widgetId: string, newSize: { width: number; height: number }) => {
      const widget = layout?.widgets.find((w) => w.id === widgetId);
      if (!widget) return;

      const definition = availableWidgets.find(
        (d) => d.widget_type === widget.widget_type
      );

      if (definition) {
        // Clamp size to min/max
        newSize.width = Math.max(definition.min_width, Math.min(definition.max_width, newSize.width));
        newSize.height = Math.max(definition.min_height, Math.min(definition.max_height, newSize.height));
      }

      updateWidget(widgetId, { size: newSize });
    },
    [layout, availableWidgets, updateWidget]
  );

  // Save current layout
  const saveLayout = useCallback(async () => {
    if (!layout) return;

    try {
      const id = await saveDashboardLayout(layout);
      setLayout({ ...layout, id });
      setIsEditing(false);
      toast.success('Layout gespeichert');
    } catch (error) {
      console.error('Failed to save layout:', error);
      toast.error('Layout konnte nicht gespeichert werden');
    }
  }, [layout]);

  // Reset to default layout
  const resetLayout = useCallback(async () => {
    try {
      const defaultLayout = await createDefaultDashboardLayout();
      setLayout(defaultLayout);
      toast.info('Layout zurückgesetzt');
    } catch (error) {
      console.error('Failed to reset layout:', error);
      toast.error('Layout konnte nicht zurückgesetzt werden');
    }
  }, []);

  return {
    layout,
    widgets: layout?.widgets ?? [],
    availableWidgets,
    isLoading,
    isEditing,
    setIsEditing,
    addWidget,
    removeWidget,
    updateWidget,
    moveWidget,
    resizeWidget,
    saveLayout,
    resetLayout,
  };
}
