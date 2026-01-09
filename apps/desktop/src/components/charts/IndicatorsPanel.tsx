/**
 * Indicators Panel
 * UI for adding, removing, and configuring technical indicators
 */

import { useState } from 'react';
import {
  ChevronDown,
  ChevronUp,
  Plus,
  Trash2,
  Settings2,
  TrendingUp,
  Activity,
  BarChart3,
} from 'lucide-react';
import type { IndicatorConfig, IndicatorType } from '../../lib/indicators';
import { defaultIndicatorConfigs } from '../../lib/indicators';

// ============================================================================
// Types
// ============================================================================

interface IndicatorsPanelProps {
  indicators: IndicatorConfig[];
  onIndicatorsChange: (indicators: IndicatorConfig[]) => void;
}

interface IndicatorInfo {
  type: IndicatorType;
  name: string;
  description: string;
  icon: typeof TrendingUp;
  category: 'overlay' | 'oscillator' | 'volatility';
  params: { key: string; label: string; min: number; max: number; step: number }[];
}

// ============================================================================
// Indicator Definitions
// ============================================================================

const indicatorInfo: IndicatorInfo[] = [
  {
    type: 'sma',
    name: 'SMA',
    description: 'Simple Moving Average',
    icon: TrendingUp,
    category: 'overlay',
    params: [{ key: 'period', label: 'Periode', min: 2, max: 200, step: 1 }],
  },
  {
    type: 'ema',
    name: 'EMA',
    description: 'Exponential Moving Average',
    icon: TrendingUp,
    category: 'overlay',
    params: [{ key: 'period', label: 'Periode', min: 2, max: 200, step: 1 }],
  },
  {
    type: 'bollinger',
    name: 'Bollinger Bands',
    description: 'Volatilitätsbänder',
    icon: Activity,
    category: 'volatility',
    params: [
      { key: 'period', label: 'Periode', min: 2, max: 50, step: 1 },
      { key: 'stdDev', label: 'Std. Abw.', min: 1, max: 4, step: 0.5 },
    ],
  },
  {
    type: 'rsi',
    name: 'RSI',
    description: 'Relative Strength Index',
    icon: BarChart3,
    category: 'oscillator',
    params: [{ key: 'period', label: 'Periode', min: 2, max: 50, step: 1 }],
  },
  {
    type: 'macd',
    name: 'MACD',
    description: 'Moving Average Convergence Divergence',
    icon: Activity,
    category: 'oscillator',
    params: [
      { key: 'fast', label: 'Schnell', min: 2, max: 50, step: 1 },
      { key: 'slow', label: 'Langsam', min: 2, max: 100, step: 1 },
      { key: 'signal', label: 'Signal', min: 2, max: 50, step: 1 },
    ],
  },
  {
    type: 'atr',
    name: 'ATR',
    description: 'Average True Range',
    icon: Activity,
    category: 'volatility',
    params: [{ key: 'period', label: 'Periode', min: 2, max: 50, step: 1 }],
  },
];

// ============================================================================
// Colors for Overlays
// ============================================================================

const overlayColors = [
  '#2196f3', // Blue
  '#ff9800', // Orange
  '#4caf50', // Green
  '#e91e63', // Pink
  '#9c27b0', // Purple
  '#00bcd4', // Cyan
  '#ffeb3b', // Yellow
  '#795548', // Brown
];

// ============================================================================
// Main Component
// ============================================================================

export function IndicatorsPanel({ indicators, onIndicatorsChange }: IndicatorsPanelProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);

  // Get next available color for overlay indicators
  const getNextColor = () => {
    const usedColors = indicators
      .filter(i => ['sma', 'ema'].includes(i.type))
      .map(i => i.color);
    return overlayColors.find(c => !usedColors.includes(c)) || overlayColors[0];
  };

  // Add new indicator
  const handleAddIndicator = (type: IndicatorType) => {
    const config = defaultIndicatorConfigs[type];
    const newIndicator: IndicatorConfig = {
      ...config,
      id: `${type}-${Date.now()}`,
      enabled: true,
      color: ['sma', 'ema'].includes(type) ? getNextColor() : config.color,
    };
    onIndicatorsChange([...indicators, newIndicator]);
  };

  // Remove indicator
  const handleRemoveIndicator = (id: string) => {
    onIndicatorsChange(indicators.filter(i => i.id !== id));
  };

  // Toggle indicator
  const handleToggleIndicator = (id: string) => {
    onIndicatorsChange(
      indicators.map(i => (i.id === id ? { ...i, enabled: !i.enabled } : i))
    );
  };

  // Update indicator params
  const handleParamChange = (id: string, key: string, value: number) => {
    onIndicatorsChange(
      indicators.map(i =>
        i.id === id ? { ...i, params: { ...i.params, [key]: value } } : i
      )
    );
  };

  // Group indicators by category
  const groupedIndicators = {
    overlay: indicatorInfo.filter(i => i.category === 'overlay'),
    oscillator: indicatorInfo.filter(i => i.category === 'oscillator'),
    volatility: indicatorInfo.filter(i => i.category === 'volatility'),
  };

  return (
    <div className="bg-card border border-border rounded-lg overflow-hidden">
      {/* Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-muted/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Settings2 size={16} className="text-muted-foreground" />
          <span className="font-medium">Indikatoren</span>
          <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
            {indicators.filter(i => i.enabled).length} aktiv
          </span>
        </div>
        {isExpanded ? (
          <ChevronUp size={16} className="text-muted-foreground" />
        ) : (
          <ChevronDown size={16} className="text-muted-foreground" />
        )}
      </button>

      {isExpanded && (
        <div className="border-t border-border">
          {/* Active Indicators */}
          {indicators.length > 0 && (
            <div className="p-3 space-y-2 border-b border-border">
              <div className="text-xs text-muted-foreground font-medium mb-2">Aktive Indikatoren</div>
              {indicators.map(indicator => {
                const info = indicatorInfo.find(i => i.type === indicator.type);
                if (!info) return null;

                return (
                  <div
                    key={indicator.id}
                    className={`rounded-lg border ${
                      indicator.enabled ? 'border-primary/30 bg-primary/5' : 'border-border bg-muted/30'
                    }`}
                  >
                    <div className="flex items-center justify-between px-3 py-2">
                      <div className="flex items-center gap-2">
                        {indicator.color && (
                          <div
                            className="w-3 h-3 rounded-full"
                            style={{ backgroundColor: indicator.color }}
                          />
                        )}
                        <button
                          onClick={() => handleToggleIndicator(indicator.id)}
                          className={`font-medium text-sm ${
                            indicator.enabled ? 'text-foreground' : 'text-muted-foreground'
                          }`}
                        >
                          {info.name}
                        </button>
                        <span className="text-xs text-muted-foreground">
                          ({Object.values(indicator.params).join(',')})
                        </span>
                      </div>
                      <div className="flex items-center gap-1">
                        <button
                          onClick={() =>
                            setEditingId(editingId === indicator.id ? null : indicator.id)
                          }
                          className="p-1 hover:bg-muted rounded"
                          title="Einstellungen"
                        >
                          <Settings2 size={14} className="text-muted-foreground" />
                        </button>
                        <button
                          onClick={() => handleRemoveIndicator(indicator.id)}
                          className="p-1 hover:bg-red-500/10 rounded"
                          title="Entfernen"
                        >
                          <Trash2 size={14} className="text-red-500" />
                        </button>
                      </div>
                    </div>

                    {/* Params Editor */}
                    {editingId === indicator.id && (
                      <div className="px-3 pb-3 pt-1 border-t border-border/50 space-y-2">
                        {info.params.map(param => (
                          <div key={param.key} className="flex items-center gap-2">
                            <label className="text-xs text-muted-foreground w-16">
                              {param.label}
                            </label>
                            <input
                              type="range"
                              min={param.min}
                              max={param.max}
                              step={param.step}
                              value={indicator.params[param.key]}
                              onChange={e =>
                                handleParamChange(
                                  indicator.id,
                                  param.key,
                                  parseFloat(e.target.value)
                                )
                              }
                              className="flex-1 h-1.5 bg-muted rounded-full appearance-none cursor-pointer"
                            />
                            <span className="text-xs font-mono w-8 text-right">
                              {indicator.params[param.key]}
                            </span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          {/* Add Indicator */}
          <div className="p-3">
            <div className="text-xs text-muted-foreground font-medium mb-2">Indikator hinzufügen</div>

            {/* Overlay Indicators */}
            <div className="mb-3">
              <div className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1.5">
                Overlay
              </div>
              <div className="flex flex-wrap gap-1">
                {groupedIndicators.overlay.map(info => (
                  <button
                    key={info.type}
                    onClick={() => handleAddIndicator(info.type)}
                    className="flex items-center gap-1.5 px-2 py-1 text-xs bg-muted hover:bg-muted/80 rounded transition-colors"
                    title={info.description}
                  >
                    <Plus size={12} />
                    {info.name}
                  </button>
                ))}
              </div>
            </div>

            {/* Oscillator Indicators */}
            <div className="mb-3">
              <div className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1.5">
                Oszillatoren
              </div>
              <div className="flex flex-wrap gap-1">
                {groupedIndicators.oscillator.map(info => (
                  <button
                    key={info.type}
                    onClick={() => handleAddIndicator(info.type)}
                    className="flex items-center gap-1.5 px-2 py-1 text-xs bg-muted hover:bg-muted/80 rounded transition-colors"
                    title={info.description}
                    disabled={indicators.some(i => i.type === info.type && i.enabled)}
                  >
                    <Plus size={12} />
                    {info.name}
                  </button>
                ))}
              </div>
            </div>

            {/* Volatility Indicators */}
            <div>
              <div className="text-[10px] text-muted-foreground uppercase tracking-wider mb-1.5">
                Volatilität
              </div>
              <div className="flex flex-wrap gap-1">
                {groupedIndicators.volatility.map(info => (
                  <button
                    key={info.type}
                    onClick={() => handleAddIndicator(info.type)}
                    className="flex items-center gap-1.5 px-2 py-1 text-xs bg-muted hover:bg-muted/80 rounded transition-colors"
                    title={info.description}
                    disabled={
                      info.type !== 'bollinger' &&
                      indicators.some(i => i.type === info.type && i.enabled)
                    }
                  >
                    <Plus size={12} />
                    {info.name}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default IndicatorsPanel;
