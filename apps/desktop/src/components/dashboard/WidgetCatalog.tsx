/**
 * Widget Catalog Modal - Select widgets to add to dashboard
 */

import { useState } from 'react';
import {
  X,
  LineChart,
  PieChart,
  Table2,
  TrendingUp,
  Bell,
  Eye,
  Calendar,
  Target,
  LayoutGrid,
} from 'lucide-react';
import type { WidgetDefinition, WidgetType } from './types';

interface WidgetCatalogProps {
  isOpen: boolean;
  onClose: () => void;
  widgets: WidgetDefinition[];
  onAddWidget: (widgetType: WidgetType) => void;
}

const WIDGET_ICONS: Record<string, React.ElementType> = {
  portfolio_value: TrendingUp,
  performance: LineChart,
  holdings_table: Table2,
  holdings_pie: PieChart,
  recent_transactions: Table2,
  dividends: Calendar,
  watchlist: Eye,
  heatmap: LayoutGrid,
  year_returns: Calendar,
  alerts: Bell,
  chart: LineChart,
  benchmark: Target,
};

export function WidgetCatalog({
  isOpen,
  onClose,
  widgets,
  onAddWidget,
}: WidgetCatalogProps) {
  const [searchTerm, setSearchTerm] = useState('');

  if (!isOpen) return null;

  const filteredWidgets = widgets.filter(
    (w) =>
      w.label.toLowerCase().includes(searchTerm.toLowerCase()) ||
      w.description.toLowerCase().includes(searchTerm.toLowerCase())
  );

  const handleAdd = (widgetType: WidgetType) => {
    onAddWidget(widgetType);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-background rounded-lg shadow-xl w-full max-w-2xl max-h-[80vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <h2 className="text-lg font-semibold">Widget hinzufügen</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-accent rounded"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {/* Search */}
        <div className="px-6 py-4 border-b">
          <input
            type="text"
            placeholder="Widgets durchsuchen..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full px-3 py-2 border rounded-md bg-background focus:outline-none focus:ring-2 focus:ring-primary"
          />
        </div>

        {/* Widget Grid */}
        <div className="p-6 overflow-auto max-h-[50vh]">
          <div className="grid grid-cols-2 gap-4">
            {filteredWidgets.map((widget) => {
              const Icon = WIDGET_ICONS[widget.widget_type] || LayoutGrid;

              return (
                <button
                  key={widget.widget_type}
                  data-testid={`widget-${widget.widget_type}`}
                  onClick={() => handleAdd(widget.widget_type)}
                  className="flex items-start gap-4 p-4 border rounded-lg hover:bg-accent text-left transition-colors"
                >
                  <div className="p-2 bg-primary/10 rounded-lg">
                    <Icon className="h-6 w-6 text-primary" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium">{widget.label}</div>
                    <div className="text-sm text-muted-foreground line-clamp-2">
                      {widget.description}
                    </div>
                    <div className="text-xs text-muted-foreground mt-1">
                      {widget.default_width}×{widget.default_height} Felder
                    </div>
                  </div>
                </button>
              );
            })}
          </div>

          {filteredWidgets.length === 0 && (
            <div className="text-center py-8 text-muted-foreground">
              Keine Widgets gefunden
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
