/**
 * Dashboard Widget Types
 */

export type WidgetType =
  | 'portfolio_value'
  | 'performance'
  | 'holdings_table'
  | 'holdings_pie'
  | 'recent_transactions'
  | 'dividends'
  | 'watchlist'
  | 'heatmap'
  | 'year_returns'
  | 'alerts'
  | 'chart'
  | 'benchmark';

export interface Position {
  x: number;
  y: number;
}

export interface Size {
  width: number;
  height: number;
}

export interface WidgetConfig {
  id: string;
  widget_type: WidgetType;
  title?: string;
  position: Position;
  size: Size;
  settings: Record<string, unknown>;
}

export interface DashboardLayout {
  id: number;
  name: string;
  columns: number;
  widgets: WidgetConfig[];
  is_default: boolean;
}

export interface WidgetDefinition {
  widget_type: WidgetType;
  label: string;
  description: string;
  default_width: number;
  default_height: number;
  min_width: number;
  min_height: number;
  max_width: number;
  max_height: number;
  configurable: boolean;
}

export interface WidgetProps {
  config: WidgetConfig;
  isEditing?: boolean;
  onConfigChange?: (config: WidgetConfig) => void;
  onRemove?: () => void;
}
