/**
 * Widget Dashboard - Customizable dashboard with drag-and-drop widgets
 * Users can freely configure their own dashboard layout here.
 */

import { useState, useCallback, useMemo, useEffect } from 'react';
import {
  Plus,
  RotateCcw,
  Pencil,
  Check,
  X,
  Loader2,
  LayoutGrid,
} from 'lucide-react';
import { useDashboardLayout } from '../../hooks/useDashboardLayout';
import { WidgetContainer, WidgetCatalog } from '../../components/dashboard';
import { getBaseCurrency, calculatePerformance } from '../../lib/api';
import type { PerformanceResult } from '../../lib/types';

interface WidgetDashboardViewProps {
  dbHoldings: Array<{
    securityIds: number[];
    name?: string;
    currentValue?: number | null;
    costBasis: number;
    gainLossPercent?: number | null;
  }>;
  dbPortfolioHistory: Array<{ date: string; value: number }>;
}

export function WidgetDashboardView({
  dbHoldings,
  dbPortfolioHistory,
}: WidgetDashboardViewProps) {
  const {
    layout,
    widgets,
    availableWidgets,
    isLoading: isLayoutLoading,
    isEditing,
    setIsEditing,
    addWidget,
    removeWidget,
    moveWidget,
    saveLayout,
    resetLayout,
  } = useDashboardLayout();

  const [showWidgetCatalog, setShowWidgetCatalog] = useState(false);
  const [baseCurrency, setBaseCurrency] = useState<string>('EUR');
  const [performance, setPerformance] = useState<PerformanceResult | null>(null);

  // Load base currency
  useEffect(() => {
    getBaseCurrency()
      .then(setBaseCurrency)
      .catch(() => setBaseCurrency('EUR'));
  }, []);

  // Load performance data
  useEffect(() => {
    if (dbHoldings.length > 0) {
      calculatePerformance({})
        .then(setPerformance)
        .catch(() => setPerformance(null));
    }
  }, [dbHoldings]);

  // Calculate derived values for widgets
  const totalValue = dbHoldings.reduce((sum, h) => sum + (h.currentValue || 0), 0);
  const totalCostBasis = dbHoldings.reduce((sum, h) => sum + h.costBasis, 0);
  const totalGainLoss = totalValue - totalCostBasis;
  const totalGainLossPercent = totalCostBasis > 0 ? (totalGainLoss / totalCostBasis) * 100 : 0;

  // Prepare holdings data for widgets
  const holdingsForWidgets = useMemo(() => {
    return dbHoldings.map((h) => ({
      name: h.name || '',
      value: h.currentValue || 0,
      weight: totalValue > 0 ? ((h.currentValue || 0) / totalValue) * 100 : 0,
      gainLossPercent: h.gainLossPercent || 0,
    }));
  }, [dbHoldings, totalValue]);

  // Handle cancel editing
  const handleCancelEdit = useCallback(() => {
    setIsEditing(false);
  }, [setIsEditing]);

  // Handle save layout
  const handleSaveLayout = useCallback(async () => {
    await saveLayout();
    setShowWidgetCatalog(false);
  }, [saveLayout]);

  // Handle add widget from catalog
  const handleAddWidget = useCallback((widgetType: string) => {
    addWidget(widgetType);
    setShowWidgetCatalog(false);
  }, [addWidget]);

  // Empty state - no widgets yet
  if (!isLayoutLoading && widgets.length === 0 && !isEditing) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center max-w-md">
          <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary/20 to-primary/5 flex items-center justify-center mx-auto mb-6 backdrop-blur-sm border border-primary/10">
            <LayoutGrid className="w-8 h-8 text-primary" />
          </div>
          <h2 className="text-xl font-light mb-2">Mein Dashboard</h2>
          <p className="text-sm text-muted-foreground mb-6">
            Gestalten Sie Ihr persönliches Dashboard mit den Widgets, die Sie am meisten interessieren.
          </p>
          <button
            onClick={() => setIsEditing(true)}
            className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-xl hover:bg-primary/90 transition-colors mx-auto"
          >
            <Pencil size={16} />
            Dashboard einrichten
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col overflow-hidden -m-4">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b bg-card/50">
        <div className="flex items-center gap-3">
          <LayoutGrid className="h-5 w-5 text-primary" />
          <h2 className="text-sm font-medium">
            {isEditing ? 'Dashboard bearbeiten' : 'Mein Dashboard'}
          </h2>
          <span className="text-xs text-muted-foreground">
            {widgets.length} Widget{widgets.length !== 1 ? 's' : ''}
          </span>
        </div>

        {isEditing ? (
          <div className="flex items-center gap-2">
            <button
              onClick={() => setShowWidgetCatalog(true)}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
            >
              <Plus size={14} />
              Widget hinzufügen
            </button>
            <button
              onClick={resetLayout}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium border rounded-md hover:bg-muted transition-colors"
              title="Auf Standard zurücksetzen"
            >
              <RotateCcw size={14} />
              Zurücksetzen
            </button>
            <div className="w-px h-6 bg-border" />
            <button
              onClick={handleCancelEdit}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium border rounded-md hover:bg-muted transition-colors"
            >
              <X size={14} />
              Abbrechen
            </button>
            <button
              onClick={handleSaveLayout}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors"
            >
              <Check size={14} />
              Speichern
            </button>
          </div>
        ) : (
          <button
            onClick={() => setIsEditing(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium border rounded-md hover:bg-muted transition-colors"
          >
            <Pencil size={14} />
            Bearbeiten
          </button>
        )}
      </div>

      {/* Widget Container */}
      <div className="flex-1 overflow-auto">
        {isLayoutLoading ? (
          <div className="h-full flex items-center justify-center">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <WidgetContainer
            widgets={widgets}
            columns={layout?.columns ?? 6}
            isEditing={isEditing}
            onRemoveWidget={removeWidget}
            onMoveWidget={moveWidget}
            portfolioValue={totalValue}
            costBasis={totalCostBasis}
            gainLoss={totalGainLoss}
            gainLossPercent={totalGainLossPercent}
            ttwror={performance?.ttwror}
            irr={performance?.irr}
            holdings={holdingsForWidgets}
            portfolioHistory={dbPortfolioHistory}
            currency={baseCurrency}
          />
        )}
      </div>

      {/* Widget Catalog Modal */}
      <WidgetCatalog
        isOpen={showWidgetCatalog}
        onClose={() => setShowWidgetCatalog(false)}
        widgets={availableWidgets}
        onAddWidget={handleAddWidget}
      />
    </div>
  );
}
