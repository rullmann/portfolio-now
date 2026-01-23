/**
 * Pattern Statistics Panel
 * Displays candlestick pattern success rates and evaluation controls.
 */

import { useState, useEffect, useCallback } from 'react';
import {
  BarChart3,
  RefreshCw,
  ChevronDown,
  ChevronUp,
  TrendingUp,
  TrendingDown,
  Clock,
  CheckCircle2,
  XCircle,
} from 'lucide-react';
import {
  getPatternStatistics,
  evaluatePatternOutcomes,
  type PatternStatistics,
  type PatternEvaluationResult,
} from '../../lib/api';

// Pattern name translations
const PATTERN_NAMES: Record<string, string> = {
  hammer: 'Hammer',
  inverted_hammer: 'Umgekehrter Hammer',
  bullish_engulfing: 'Bullish Engulfing',
  bearish_engulfing: 'Bearish Engulfing',
  morning_star: 'Morgenstern',
  evening_star: 'Abendstern',
  doji: 'Doji',
  dragonfly_doji: 'Libellen-Doji',
  gravestone_doji: 'Grabstein-Doji',
  spinning_top: 'Kreisel',
  marubozu: 'Marubozu',
  three_white_soldiers: 'Drei weiße Soldaten',
  three_black_crows: 'Drei schwarze Krähen',
  piercing_line: 'Durchstoßungslinie',
  dark_cloud_cover: 'Dunkle Wolkendecke',
  hanging_man: 'Hängender Mann',
  shooting_star: 'Sternschnuppe',
  harami: 'Harami',
  tweezer_top: 'Pinzetten-Top',
  tweezer_bottom: 'Pinzetten-Boden',
  rising_three: 'Rising Three Methods',
  falling_three: 'Falling Three Methods',
};

interface PatternStatisticsPanelProps {
  securityId?: number | null;
}

export function PatternStatisticsPanel({ securityId: _securityId }: PatternStatisticsPanelProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const [statistics, setStatistics] = useState<PatternStatistics[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isEvaluating, setIsEvaluating] = useState(false);
  const [lastEvaluation, setLastEvaluation] = useState<PatternEvaluationResult | null>(null);

  // Load statistics
  const loadStatistics = useCallback(async () => {
    setIsLoading(true);
    try {
      const stats = await getPatternStatistics();
      // Sort by total count descending
      stats.sort((a, b) => b.totalCount - a.totalCount);
      setStatistics(stats);
    } catch (err) {
      console.error('Failed to load pattern statistics:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStatistics();
  }, [loadStatistics]);

  // Evaluate pending patterns
  const handleEvaluate = async () => {
    setIsEvaluating(true);
    try {
      const result = await evaluatePatternOutcomes();
      setLastEvaluation(result);
      // Reload statistics after evaluation
      await loadStatistics();
    } catch (err) {
      console.error('Failed to evaluate patterns:', err);
    } finally {
      setIsEvaluating(false);
    }
  };

  // Calculate totals
  const totals = statistics.reduce(
    (acc, s) => ({
      total: acc.total + s.totalCount,
      success: acc.success + s.successCount,
      failure: acc.failure + s.failureCount,
      pending: acc.pending + s.pendingCount,
    }),
    { total: 0, success: 0, failure: 0, pending: 0 }
  );

  const overallSuccessRate =
    totals.success + totals.failure > 0
      ? (totals.success / (totals.success + totals.failure)) * 100
      : 0;

  const getPatternName = (type: string): string => {
    return PATTERN_NAMES[type] || type.replace(/_/g, ' ');
  };

  return (
    <div className="bg-card border border-border rounded-lg overflow-hidden">
      {/* Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center justify-between p-3 hover:bg-muted/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          <BarChart3 size={16} className="text-primary" />
          <span className="font-medium text-sm">Pattern-Statistiken</span>
          {totals.total > 0 && (
            <span className="text-xs text-muted-foreground">
              ({totals.total} erkannt)
            </span>
          )}
        </div>
        {isExpanded ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
      </button>

      {isExpanded && (
        <div className="p-3 pt-0 space-y-3">
          {/* Summary */}
          {totals.total > 0 && (
            <div className="grid grid-cols-4 gap-2 text-xs">
              <div className="bg-muted/50 rounded p-2 text-center">
                <div className="text-muted-foreground mb-0.5">Gesamt</div>
                <div className="font-medium">{totals.total}</div>
              </div>
              <div className="bg-green-500/10 rounded p-2 text-center">
                <div className="text-green-600 dark:text-green-400 mb-0.5 flex items-center justify-center gap-1">
                  <CheckCircle2 size={10} />
                  Erfolg
                </div>
                <div className="font-medium text-green-600 dark:text-green-400">
                  {totals.success}
                </div>
              </div>
              <div className="bg-red-500/10 rounded p-2 text-center">
                <div className="text-red-600 dark:text-red-400 mb-0.5 flex items-center justify-center gap-1">
                  <XCircle size={10} />
                  Fehler
                </div>
                <div className="font-medium text-red-600 dark:text-red-400">
                  {totals.failure}
                </div>
              </div>
              <div className="bg-amber-500/10 rounded p-2 text-center">
                <div className="text-amber-600 dark:text-amber-400 mb-0.5 flex items-center justify-center gap-1">
                  <Clock size={10} />
                  Offen
                </div>
                <div className="font-medium text-amber-600 dark:text-amber-400">
                  {totals.pending}
                </div>
              </div>
            </div>
          )}

          {/* Overall Success Rate */}
          {totals.success + totals.failure > 0 && (
            <div className="flex items-center justify-between p-2 bg-muted/30 rounded">
              <span className="text-xs text-muted-foreground">Gesamt-Erfolgsrate</span>
              <span
                className={`text-sm font-medium ${
                  overallSuccessRate >= 50
                    ? 'text-green-600 dark:text-green-400'
                    : 'text-red-600 dark:text-red-400'
                }`}
              >
                {overallSuccessRate.toFixed(1)}%
              </span>
            </div>
          )}

          {/* Evaluate Button */}
          {totals.pending > 0 && (
            <button
              onClick={handleEvaluate}
              disabled={isEvaluating}
              className="w-full flex items-center justify-center gap-2 px-3 py-2 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              <RefreshCw size={14} className={isEvaluating ? 'animate-spin' : ''} />
              {isEvaluating ? 'Auswertung...' : 'Patterns auswerten'}
            </button>
          )}

          {/* Last Evaluation Result */}
          {lastEvaluation && (
            <div className="text-xs text-muted-foreground text-center">
              Zuletzt ausgewertet: {lastEvaluation.patternsEvaluated} Patterns
              ({lastEvaluation.successes} Erfolge, {lastEvaluation.failures} Fehler)
            </div>
          )}

          {/* Pattern List */}
          {isLoading ? (
            <div className="flex items-center justify-center py-4">
              <RefreshCw size={16} className="animate-spin text-muted-foreground" />
            </div>
          ) : statistics.length === 0 ? (
            <div className="text-xs text-muted-foreground text-center py-4">
              Keine Pattern-Daten vorhanden.
              <br />
              Patterns werden automatisch erkannt, wenn die Chart-Analyse aktiv ist.
            </div>
          ) : (
            <div className="space-y-1 max-h-64 overflow-auto">
              {statistics.map((stat) => {
                const evaluated = stat.successCount + stat.failureCount;
                const successRate = evaluated > 0 ? (stat.successCount / evaluated) * 100 : 0;

                return (
                  <div
                    key={stat.patternType}
                    className="flex items-center justify-between p-2 bg-muted/30 rounded text-xs hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="truncate font-medium">
                        {getPatternName(stat.patternType)}
                      </span>
                      <span className="text-muted-foreground flex-shrink-0">
                        ({stat.totalCount})
                      </span>
                    </div>

                    <div className="flex items-center gap-3 flex-shrink-0">
                      {evaluated > 0 ? (
                        <>
                          <div className="flex items-center gap-1">
                            {successRate >= 50 ? (
                              <TrendingUp
                                size={12}
                                className="text-green-600 dark:text-green-400"
                              />
                            ) : (
                              <TrendingDown
                                size={12}
                                className="text-red-600 dark:text-red-400"
                              />
                            )}
                            <span
                              className={
                                successRate >= 50
                                  ? 'text-green-600 dark:text-green-400'
                                  : 'text-red-600 dark:text-red-400'
                              }
                            >
                              {successRate.toFixed(0)}%
                            </span>
                          </div>
                          {stat.avgGainOnSuccess !== undefined && stat.avgGainOnSuccess > 0 && (
                            <span className="text-green-600 dark:text-green-400">
                              +{stat.avgGainOnSuccess.toFixed(1)}%
                            </span>
                          )}
                          {stat.avgLossOnFailure !== undefined && stat.avgLossOnFailure < 0 && (
                            <span className="text-red-600 dark:text-red-400">
                              {stat.avgLossOnFailure.toFixed(1)}%
                            </span>
                          )}
                        </>
                      ) : (
                        <span className="text-muted-foreground">
                          {stat.pendingCount} offen
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default PatternStatisticsPanel;
