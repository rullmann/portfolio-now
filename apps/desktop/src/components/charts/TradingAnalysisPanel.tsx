/**
 * Trading Analysis Panel
 * Shows regime detection, setup scoring, and risk calculator.
 */

import { useState, useEffect, useCallback } from 'react';
import {
  ChevronDown,
  ChevronUp,
  RefreshCw,
  TrendingUp,
  TrendingDown,
  Minus,
  Calculator,
  Activity,
  Shield,
} from 'lucide-react';
import { RegimeBadge } from '../common/RegimeBadge';
import { SetupScoreBadge } from '../common/SetupScoreBadge';
import { fullTradingAnalysis, calculateRisk } from '../../lib/indicators-rust';
import type { OHLCData, TradingAnalysis, RiskAnalysis } from '../../lib/indicators';

interface TradingAnalysisPanelProps {
  data: OHLCData[];
  currency?: string;
  currentPrice?: number;
}

export function TradingAnalysisPanel({
  data,
  currency = 'EUR',
  currentPrice,
}: TradingAnalysisPanelProps) {
  const [analysis, setAnalysis] = useState<TradingAnalysis | null>(null);
  const [loading, setLoading] = useState(false);
  const [riskExpanded, setRiskExpanded] = useState(false);

  // Risk calculator inputs
  const [accountSize, setAccountSize] = useState('100000');
  const [riskPercent, setRiskPercent] = useState('1');
  const [entryPrice, setEntryPrice] = useState('');
  const [customRisk, setCustomRisk] = useState<RiskAnalysis | null>(null);

  const loadAnalysis = useCallback(async () => {
    if (data.length < 20) return;
    setLoading(true);
    try {
      const result = await fullTradingAnalysis(data);
      setAnalysis(result);
    } catch (err) {
      console.error('Trading analysis failed:', err);
    } finally {
      setLoading(false);
    }
  }, [data]);

  useEffect(() => {
    loadAnalysis();
  }, [loadAnalysis]);

  // Recalculate risk when inputs change
  const handleRiskCalculate = async () => {
    const size = parseFloat(accountSize);
    const pct = parseFloat(riskPercent);
    const entry = entryPrice ? parseFloat(entryPrice) : undefined;
    if (isNaN(size) || size <= 0) return;

    try {
      const result = await calculateRisk(data, size, isNaN(pct) ? 1 : pct, entry);
      setCustomRisk(result);
    } catch (err) {
      console.error('Risk calculation failed:', err);
    }
  };

  if (loading) {
    return (
      <div className="p-4 flex items-center justify-center">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!analysis) {
    return (
      <div className="p-4 text-center text-sm text-muted-foreground">
        Nicht genügend Daten für Trading-Analyse
      </div>
    );
  }

  const risk = customRisk || analysis.risk;
  const directionIcon = analysis.setup.direction === 'long'
    ? <TrendingUp size={14} className="text-green-500" />
    : analysis.setup.direction === 'short'
      ? <TrendingDown size={14} className="text-red-500" />
      : <Minus size={14} className="text-muted-foreground" />;

  return (
    <div className="space-y-3">
      {/* Regime Section */}
      <div className="bg-card rounded-lg border border-border p-3">
        <div className="flex items-center gap-2 mb-2">
          <Activity size={14} className="text-primary" />
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Regime</span>
        </div>
        <div className="flex items-center gap-2 mb-2">
          <RegimeBadge regime={analysis.regime.regime} size="md" />
          <span className="text-xs text-muted-foreground">
            ADX: {analysis.regime.trendStrength.toFixed(1)}
          </span>
        </div>
        {/* Confidence bar */}
        <div className="mb-2">
          <div className="flex items-center justify-between text-xs text-muted-foreground mb-1">
            <span>Konfidenz</span>
            <span>{(analysis.regime.confidence * 100).toFixed(0)}%</span>
          </div>
          <div className="h-1.5 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full bg-primary rounded-full transition-all"
              style={{ width: `${analysis.regime.confidence * 100}%` }}
            />
          </div>
        </div>
        {/* Evidence */}
        <div className="space-y-0.5">
          {analysis.regime.supportingEvidence.slice(0, 4).map((ev, i) => (
            <div key={i} className="text-xs text-muted-foreground truncate" title={ev}>
              {ev}
            </div>
          ))}
        </div>
      </div>

      {/* Setup Score Section */}
      <div className="bg-card rounded-lg border border-border p-3">
        <div className="flex items-center gap-2 mb-2">
          <TrendingUp size={14} className="text-primary" />
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Setup Score</span>
        </div>
        <div className="flex items-center gap-3 mb-2">
          <span className="text-2xl font-bold">{Math.round(analysis.setup.totalScore)}</span>
          <span className="text-sm text-muted-foreground">/ 100</span>
          <SetupScoreBadge score={analysis.setup.totalScore} size="md" />
        </div>
        <div className="flex items-center gap-2 mb-3 text-sm">
          {directionIcon}
          <span>{analysis.setup.setupLabel}</span>
        </div>
        {/* Factor bars */}
        <div className="space-y-1.5">
          {analysis.setup.factors.map((f) => (
            <div key={f.name}>
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground">{f.name} ({(f.weight * 100).toFixed(0)}%)</span>
                <span>{Math.round(f.score)}</span>
              </div>
              <div className="h-1 bg-muted rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full transition-all"
                  style={{
                    width: `${Math.min(f.score, 100)}%`,
                    backgroundColor: f.score >= 70 ? '#22c55e' : f.score >= 40 ? '#eab308' : '#ef4444',
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Risk Calculator (collapsible) */}
      <div className="bg-card rounded-lg border border-border">
        <button
          onClick={() => setRiskExpanded(!riskExpanded)}
          className="w-full flex items-center justify-between p-3 hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-2">
            <Shield size={14} className="text-primary" />
            <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">Risiko-Rechner</span>
          </div>
          {riskExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        </button>
        {riskExpanded && (
          <div className="px-3 pb-3 space-y-2">
            <div className="grid grid-cols-2 gap-2">
              <div>
                <label className="text-xs text-muted-foreground">Kontowert ({currency})</label>
                <input
                  type="number"
                  value={accountSize}
                  onChange={(e) => setAccountSize(e.target.value)}
                  className="w-full px-2 py-1 text-sm border rounded bg-background"
                />
              </div>
              <div>
                <label className="text-xs text-muted-foreground">Risiko (%)</label>
                <input
                  type="number"
                  value={riskPercent}
                  onChange={(e) => setRiskPercent(e.target.value)}
                  className="w-full px-2 py-1 text-sm border rounded bg-background"
                  step="0.5"
                />
              </div>
            </div>
            <div>
              <label className="text-xs text-muted-foreground">Entry-Preis (optional)</label>
              <input
                type="number"
                value={entryPrice}
                onChange={(e) => setEntryPrice(e.target.value)}
                placeholder={currentPrice?.toFixed(2) || 'Aktueller Kurs'}
                className="w-full px-2 py-1 text-sm border rounded bg-background"
              />
            </div>
            <button
              onClick={handleRiskCalculate}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors"
            >
              <Calculator size={12} />
              Berechnen
            </button>
            {risk && (
              <div className="space-y-1 pt-1 border-t border-border">
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">ATR (14)</span>
                  <span>{risk.atrValue.toFixed(2)} {currency}</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">Stop-Loss</span>
                  <span className="text-red-500">
                    {risk.stopPrice.toFixed(2)} {currency} ({risk.atrStopPercent.toFixed(1)}%)
                  </span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">Kursziel</span>
                  <span className="text-green-500">{risk.targetPrice.toFixed(2)} {currency}</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">Position</span>
                  <span>{risk.suggestedPositionSize.toFixed(0)} Stück ({risk.suggestedPositionValue.toFixed(0)} {currency})</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">R:R</span>
                  <span>1:{risk.riskRewardRatio.toFixed(1)}</span>
                </div>
                <div className="flex justify-between text-xs">
                  <span className="text-muted-foreground">Max. Verlust</span>
                  <span className="text-red-500">{risk.riskPerTrade.toFixed(0)} {currency} ({risk.maxLossPercent.toFixed(1)}%)</span>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
