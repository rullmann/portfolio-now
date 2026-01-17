/**
 * Signals Panel
 * Displays detected technical signals, divergences, and candlestick patterns
 */

import { useState, useMemo } from 'react';
import {
  ChevronDown,
  ChevronUp,
  TrendingUp,
  TrendingDown,
  AlertTriangle,
  Activity,
  CandlestickChart,
  Signal,
  Filter,
} from 'lucide-react';
import type { OHLCData } from '../../lib/indicators';
import { getAllSignals, type TechnicalSignal, type SignalDirection, type SignalStrength } from '../../lib/signals';
import { detectCandlestickPatterns, type PatternMatch } from '../../lib/patterns';
import { formatDate } from '../../lib/types';

// ============================================================================
// Types
// ============================================================================

interface SignalsPanelProps {
  data: OHLCData[];
  onSignalClick?: (date: string) => void;
}

type FilterType = 'all' | 'bullish' | 'bearish' | 'patterns';

// ============================================================================
// Helper Components
// ============================================================================

function SignalIcon({ direction }: { direction: SignalDirection }) {
  switch (direction) {
    case 'bullish':
      return <TrendingUp size={14} className="text-green-500" />;
    case 'bearish':
      return <TrendingDown size={14} className="text-red-500" />;
    default:
      return <AlertTriangle size={14} className="text-amber-500" />;
  }
}

function StrengthBadge({ strength }: { strength: SignalStrength }) {
  const colors = {
    strong: 'bg-green-500/20 text-green-600 dark:text-green-400',
    moderate: 'bg-amber-500/20 text-amber-600 dark:text-amber-400',
    weak: 'bg-gray-500/20 text-gray-600 dark:text-gray-400',
  };

  const labels = {
    strong: 'Stark',
    moderate: 'Mittel',
    weak: 'Schwach',
  };

  return (
    <span className={`text-[10px] px-1.5 py-0.5 rounded ${colors[strength]}`}>
      {labels[strength]}
    </span>
  );
}

function DirectionBadge({ direction }: { direction: SignalDirection }) {
  const colors = {
    bullish: 'bg-green-500/20 text-green-600 dark:text-green-400',
    bearish: 'bg-red-500/20 text-red-600 dark:text-red-400',
    neutral: 'bg-gray-500/20 text-gray-600 dark:text-gray-400',
  };

  const labels = {
    bullish: 'Bullish',
    bearish: 'Bearish',
    neutral: 'Neutral',
  };

  return (
    <span className={`text-[10px] px-1.5 py-0.5 rounded ${colors[direction]}`}>
      {labels[direction]}
    </span>
  );
}

// ============================================================================
// Signal Item Component
// ============================================================================

function SignalItem({
  signal,
  onClick,
}: {
  signal: TechnicalSignal;
  onClick?: () => void;
}) {
  const borderColor = {
    bullish: 'border-l-green-500',
    bearish: 'border-l-red-500',
    neutral: 'border-l-amber-500',
  };

  return (
    <button
      onClick={onClick}
      className={`w-full text-left p-2 rounded-lg border border-border hover:bg-muted/50 transition-colors border-l-2 ${borderColor[signal.direction]}`}
    >
      <div className="flex items-start gap-2">
        <SignalIcon direction={signal.direction} />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-0.5">
            <span className="text-xs font-medium">{signal.indicator}</span>
            <StrengthBadge strength={signal.strength} />
          </div>
          <p className="text-xs text-muted-foreground line-clamp-2">
            {signal.description}
          </p>
          <div className="flex items-center gap-2 mt-1">
            <span className="text-[10px] text-muted-foreground">
              {formatDate(signal.date)}
            </span>
            {signal.value !== undefined && (
              <span className="text-[10px] font-mono text-muted-foreground">
                {signal.value.toFixed(1)}
              </span>
            )}
          </div>
        </div>
      </div>
    </button>
  );
}

// ============================================================================
// Pattern Item Component
// ============================================================================

function PatternItem({
  pattern,
  data,
  onClick,
}: {
  pattern: PatternMatch;
  data: OHLCData[];
  onClick?: () => void;
}) {
  const borderColor = {
    bullish: 'border-l-green-500',
    bearish: 'border-l-red-500',
    neutral: 'border-l-amber-500',
  };

  const reliabilityColors = {
    high: 'bg-green-500/20 text-green-600 dark:text-green-400',
    medium: 'bg-amber-500/20 text-amber-600 dark:text-amber-400',
    low: 'bg-gray-500/20 text-gray-600 dark:text-gray-400',
  };

  const reliabilityLabels = {
    high: 'Hoch',
    medium: 'Mittel',
    low: 'Niedrig',
  };

  const endDate = data[pattern.endIndex]?.time || '';

  return (
    <button
      onClick={onClick}
      className={`w-full text-left p-2 rounded-lg border border-border hover:bg-muted/50 transition-colors border-l-2 ${borderColor[pattern.direction]}`}
    >
      <div className="flex items-start gap-2">
        <CandlestickChart size={14} className="text-purple-500 mt-0.5" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-0.5">
            <span className="text-xs font-medium">{pattern.name}</span>
            <DirectionBadge direction={pattern.direction} />
            <span className={`text-[10px] px-1.5 py-0.5 rounded ${reliabilityColors[pattern.reliability]}`}>
              {reliabilityLabels[pattern.reliability]}
            </span>
          </div>
          <p className="text-xs text-muted-foreground line-clamp-2">
            {pattern.description}
          </p>
          <span className="text-[10px] text-muted-foreground">
            {endDate ? formatDate(endDate) : ''}
          </span>
        </div>
      </div>
    </button>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function SignalsPanel({ data, onSignalClick }: SignalsPanelProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [filter, setFilter] = useState<FilterType>('all');

  // Detect signals and patterns
  const signals = useMemo(() => {
    if (data.length < 30) return [];
    return getAllSignals(data);
  }, [data]);

  const patterns = useMemo(() => {
    if (data.length < 10) return [];
    return detectCandlestickPatterns(data);
  }, [data]);

  // Filter signals
  const filteredSignals = useMemo(() => {
    if (filter === 'patterns') return [];
    if (filter === 'all') return signals;
    return signals.filter(s => s.direction === filter);
  }, [signals, filter]);

  const filteredPatterns = useMemo(() => {
    if (filter === 'all' || filter === 'patterns') return patterns;
    return patterns.filter(p => p.direction === filter);
  }, [patterns, filter]);

  // Count by direction
  const counts = useMemo(() => ({
    bullish: signals.filter(s => s.direction === 'bullish').length +
             patterns.filter(p => p.direction === 'bullish').length,
    bearish: signals.filter(s => s.direction === 'bearish').length +
             patterns.filter(p => p.direction === 'bearish').length,
    patterns: patterns.length,
    total: signals.length + patterns.length,
  }), [signals, patterns]);

  const handleSignalClick = (date: string) => {
    onSignalClick?.(date);
  };

  const handlePatternClick = (pattern: PatternMatch) => {
    const date = data[pattern.endIndex]?.time;
    if (date) {
      onSignalClick?.(date);
    }
  };

  return (
    <div className="bg-card border border-border rounded-lg overflow-hidden">
      {/* Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center justify-between px-4 py-3 hover:bg-muted/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          <Signal size={16} className="text-muted-foreground" />
          <span className="font-medium">Signale</span>
          {counts.total > 0 && (
            <div className="flex items-center gap-1">
              {counts.bullish > 0 && (
                <span className="text-xs bg-green-500/20 text-green-600 dark:text-green-400 px-1.5 py-0.5 rounded">
                  {counts.bullish} bullish
                </span>
              )}
              {counts.bearish > 0 && (
                <span className="text-xs bg-red-500/20 text-red-600 dark:text-red-400 px-1.5 py-0.5 rounded">
                  {counts.bearish} bearish
                </span>
              )}
            </div>
          )}
        </div>
        {isExpanded ? (
          <ChevronUp size={16} className="text-muted-foreground" />
        ) : (
          <ChevronDown size={16} className="text-muted-foreground" />
        )}
      </button>

      {isExpanded && (
        <div className="border-t border-border">
          {/* Filter Buttons */}
          <div className="p-2 flex gap-1 border-b border-border">
            <button
              onClick={() => setFilter('all')}
              className={`flex items-center gap-1 px-2 py-1 text-xs rounded transition-colors ${
                filter === 'all' ? 'bg-primary text-primary-foreground' : 'bg-muted hover:bg-muted/80'
              }`}
            >
              <Filter size={12} />
              Alle ({counts.total})
            </button>
            <button
              onClick={() => setFilter('bullish')}
              className={`flex items-center gap-1 px-2 py-1 text-xs rounded transition-colors ${
                filter === 'bullish' ? 'bg-green-500 text-white' : 'bg-muted hover:bg-muted/80'
              }`}
            >
              <TrendingUp size={12} />
              Bullish ({counts.bullish})
            </button>
            <button
              onClick={() => setFilter('bearish')}
              className={`flex items-center gap-1 px-2 py-1 text-xs rounded transition-colors ${
                filter === 'bearish' ? 'bg-red-500 text-white' : 'bg-muted hover:bg-muted/80'
              }`}
            >
              <TrendingDown size={12} />
              Bearish ({counts.bearish})
            </button>
            <button
              onClick={() => setFilter('patterns')}
              className={`flex items-center gap-1 px-2 py-1 text-xs rounded transition-colors ${
                filter === 'patterns' ? 'bg-purple-500 text-white' : 'bg-muted hover:bg-muted/80'
              }`}
            >
              <CandlestickChart size={12} />
              Patterns ({counts.patterns})
            </button>
          </div>

          {/* Signals List */}
          <div className="p-2 space-y-2 max-h-80 overflow-y-auto">
            {/* Technical Signals */}
            {filteredSignals.length > 0 && filter !== 'patterns' && (
              <div className="space-y-1.5">
                <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground font-medium uppercase tracking-wider px-1">
                  <Activity size={10} />
                  Technische Signale
                </div>
                {filteredSignals.slice(0, 10).map((signal, i) => (
                  <SignalItem
                    key={`${signal.type}-${signal.date}-${i}`}
                    signal={signal}
                    onClick={() => handleSignalClick(signal.date)}
                  />
                ))}
              </div>
            )}

            {/* Candlestick Patterns */}
            {filteredPatterns.length > 0 && (
              <div className="space-y-1.5">
                <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground font-medium uppercase tracking-wider px-1">
                  <CandlestickChart size={10} />
                  Candlestick Patterns
                </div>
                {filteredPatterns.slice(0, 10).map((pattern, i) => (
                  <PatternItem
                    key={`${pattern.pattern}-${pattern.endIndex}-${i}`}
                    pattern={pattern}
                    data={data}
                    onClick={() => handlePatternClick(pattern)}
                  />
                ))}
              </div>
            )}

            {/* Empty State */}
            {filteredSignals.length === 0 && filteredPatterns.length === 0 && (
              <div className="text-center py-6 text-muted-foreground">
                <Signal size={24} className="mx-auto mb-2 opacity-30" />
                <p className="text-sm">Keine Signale erkannt</p>
                <p className="text-xs">Signale werden automatisch basierend auf technischen Indikatoren generiert</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export default SignalsPanel;
