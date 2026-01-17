/**
 * Widget Container - Grid-based layout for dashboard widgets
 */

import { useMemo } from 'react';
import { GripVertical, X, Settings } from 'lucide-react';
import type { WidgetConfig, WidgetProps } from './types';

// Widget Components
import { PortfolioValueWidget } from './widgets/PortfolioValueWidget';
import { PerformanceWidget } from './widgets/PerformanceWidget';
import { HoldingsTableWidget } from './widgets/HoldingsTableWidget';
import { ChartWidget } from './widgets/ChartWidget';
import { AlertsWidget } from './widgets/AlertsWidget';
import { HeatmapWidget } from './widgets/HeatmapWidget';
import { YearReturnsWidget } from './widgets/YearReturnsWidget';

interface WidgetContainerProps {
  widgets: WidgetConfig[];
  columns: number;
  isEditing: boolean;
  onRemoveWidget: (widgetId: string) => void;
  onMoveWidget: (widgetId: string, newPosition: { x: number; y: number }) => void;
  onConfigureWidget?: (widget: WidgetConfig) => void;
  // Data props passed to widgets
  portfolioValue?: number;
  costBasis?: number;
  gainLoss?: number;
  gainLossPercent?: number;
  ttwror?: number;
  irr?: number;
  holdings?: Array<{ name: string; value: number; weight: number; gainLossPercent: number }>;
  portfolioHistory?: Array<{ date: string; value: number }>;
}

// Map widget type to component
const widgetComponents: Record<string, React.ComponentType<WidgetProps & Record<string, unknown>>> = {
  portfolio_value: PortfolioValueWidget,
  performance: PerformanceWidget,
  holdings_table: HoldingsTableWidget,
  chart: ChartWidget,
  alerts: AlertsWidget,
  heatmap: HeatmapWidget,
  year_returns: YearReturnsWidget,
};

export function WidgetContainer({
  widgets,
  columns,
  isEditing,
  onRemoveWidget,
  onMoveWidget,
  onConfigureWidget,
  ...dataProps
}: WidgetContainerProps) {
  // Calculate grid rows needed
  const gridRows = useMemo(() => {
    if (widgets.length === 0) return 1;
    return Math.max(
      ...widgets.map((w) => w.position.y + w.size.height)
    );
  }, [widgets]);

  // Handle drag start
  const handleDragStart = (e: React.DragEvent, widgetId: string) => {
    if (!isEditing) return;
    e.dataTransfer.setData('widgetId', widgetId);
    e.dataTransfer.effectAllowed = 'move';
  };

  // Handle drag over
  const handleDragOver = (e: React.DragEvent) => {
    if (!isEditing) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
  };

  // Handle drop
  const handleDrop = (e: React.DragEvent, targetX: number, targetY: number) => {
    if (!isEditing) return;
    e.preventDefault();

    const widgetId = e.dataTransfer.getData('widgetId');
    if (widgetId) {
      onMoveWidget(widgetId, { x: targetX, y: targetY });
    }
  };

  return (
    <div
      className="grid gap-4 p-4"
      style={{
        gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
        gridTemplateRows: `repeat(${gridRows}, minmax(120px, auto))`,
      }}
      onDragOver={handleDragOver}
    >
      {widgets.map((widget) => {
        const WidgetComponent = widgetComponents[widget.widget_type];

        if (!WidgetComponent) {
          console.warn(`Unknown widget type: ${widget.widget_type}`);
          return null;
        }

        return (
          <div
            key={widget.id}
            data-testid={`widget-${widget.widget_type}-instance`}
            className={`
              relative bg-card rounded-lg border shadow-sm overflow-hidden
              ${isEditing ? 'ring-2 ring-primary/20 cursor-move' : ''}
            `}
            style={{
              gridColumn: `${widget.position.x + 1} / span ${widget.size.width}`,
              gridRow: `${widget.position.y + 1} / span ${widget.size.height}`,
            }}
            draggable={isEditing}
            onDragStart={(e) => handleDragStart(e, widget.id)}
            onDrop={(e) => handleDrop(e, widget.position.x, widget.position.y)}
          >
            {/* Edit Mode Header */}
            {isEditing && (
              <div className="absolute top-0 left-0 right-0 z-10 flex items-center justify-between px-2 py-1 bg-muted/90 border-b">
                <div className="flex items-center gap-1 text-xs text-muted-foreground">
                  <GripVertical className="h-3 w-3" />
                  <span>{widget.title || widget.widget_type}</span>
                </div>
                <div className="flex items-center gap-1">
                  {onConfigureWidget && (
                    <button
                      onClick={() => onConfigureWidget(widget)}
                      className="p-1 hover:bg-accent rounded"
                      title="Einstellungen"
                    >
                      <Settings className="h-3 w-3" />
                    </button>
                  )}
                  <button
                    onClick={() => onRemoveWidget(widget.id)}
                    className="p-1 hover:bg-destructive/20 hover:text-destructive rounded"
                    title="Entfernen"
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              </div>
            )}

            {/* Widget Content */}
            <div className={`h-full ${isEditing ? 'pt-8' : ''}`}>
              <WidgetComponent
                config={widget}
                isEditing={isEditing}
                {...dataProps}
              />
            </div>
          </div>
        );
      })}

      {/* Empty state */}
      {widgets.length === 0 && (
        <div
          className="col-span-full flex items-center justify-center h-64 border-2 border-dashed rounded-lg"
        >
          <p className="text-muted-foreground">
            Keine Widgets vorhanden. Klicken Sie auf "Widget hinzufügen" um zu beginnen.
          </p>
        </div>
      )}
    </div>
  );
}
