/**
 * Risk Analysis Panel
 * Shows VaR/CVaR, Drawdown analysis, Rolling Returns, Monte Carlo, and GARCH
 * Collapsible sections with AI summaries
 */

import { useState, useCallback, useMemo } from 'react';
import {
  Shield,
  ChevronDown,
  ChevronRight,
  TrendingDown,
  Dice5,
  Activity,
  BarChart3,
  Loader2,
  Brain,
} from 'lucide-react';
import type { OHLCData } from '../../lib/indicators';
import type {
  VaRResult,
  DrawdownResult,
  MonteCarloResult,
  GarchResult,
  AllRollingReturns,
  PricePoint,
} from '../../lib/api';
import {
  calculateVarCvar,
  calculateDrawdowns,
  calculateRollingReturns,
  runMonteCarlo,
  calculateGarch,
} from '../../lib/api';

interface RiskAnalysisPanelProps {
  data: OHLCData[];
  currency?: string;
}

function ohlcToPricePoints(data: OHLCData[]): PricePoint[] {
  return data.map(d => ({ date: d.time as string, close: d.close }));
}

function formatPct(value: number, decimals = 1): string {
  const sign = value >= 0 ? '+' : '';
  return `${sign}${value.toFixed(decimals)}%`;
}

function formatNumber(value: number): string {
  if (Math.abs(value) >= 1e9) return `${(value / 1e9).toFixed(1)}B`;
  if (Math.abs(value) >= 1e6) return `${(value / 1e6).toFixed(1)}M`;
  if (Math.abs(value) >= 1e3) return `${(value / 1e3).toFixed(1)}K`;
  return value.toFixed(2);
}

// Collapsible section component
function Section({ title, icon: Icon, children, defaultOpen = false }: {
  title: string;
  icon: React.ComponentType<{ size?: number; className?: string }>;
  children: React.ReactNode;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="border-b border-border/50 last:border-b-0">
      <button
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 w-full p-2 text-xs font-medium hover:bg-muted/50 transition-colors"
      >
        {open ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        <Icon size={12} className="text-muted-foreground" />
        <span>{title}</span>
      </button>
      {open && <div className="px-2 pb-2">{children}</div>}
    </div>
  );
}

// Metric row component
function Metric({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div className="flex justify-between text-xs">
      <span className="text-muted-foreground">{label}</span>
      <span className={`font-mono tabular-nums ${color || ''}`}>{value}</span>
    </div>
  );
}

export function RiskAnalysisPanel({ data, currency }: RiskAnalysisPanelProps) {
  const [expanded, setExpanded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Results
  const [varResult, setVarResult] = useState<VaRResult | null>(null);
  const [drawdownResult, setDrawdownResult] = useState<DrawdownResult | null>(null);
  const [rollingResult, setRollingResult] = useState<AllRollingReturns | null>(null);
  const [monteCarloResult, setMonteCarloResult] = useState<MonteCarloResult | null>(null);
  const [garchResult, setGarchResult] = useState<GarchResult | null>(null);
  const [showAiSummary, setShowAiSummary] = useState<string | null>(null);

  const prices = useMemo(() => ohlcToPricePoints(data), [data]);

  const runAnalysis = useCallback(async () => {
    if (prices.length < 30) return;
    setLoading(true);
    setError(null);

    try {
      const [varRes, ddRes, rollRes, mcRes, garchRes] = await Promise.all([
        calculateVarCvar(prices),
        calculateDrawdowns(prices),
        calculateRollingReturns(prices),
        runMonteCarlo(prices, 5000, 252),
        calculateGarch(prices, 30),
      ]);

      setVarResult(varRes);
      setDrawdownResult(ddRes);
      setRollingResult(rollRes);
      setMonteCarloResult(mcRes);
      setGarchResult(garchRes);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [prices]);

  if (data.length < 30) return null;

  return (
    <div className="card-surface">
      <button
        onClick={() => {
          setExpanded(!expanded);
          if (!expanded && !varResult && !loading) {
            runAnalysis();
          }
        }}
        className="flex items-center gap-2 w-full p-3 text-sm font-medium hover:bg-muted/30 transition-colors"
      >
        <Shield size={14} className="text-blue-500" />
        <span>Risikoanalyse</span>
        {loading && <Loader2 size={12} className="animate-spin ml-auto" />}
        {!loading && (expanded ? <ChevronDown size={14} className="ml-auto" /> : <ChevronRight size={14} className="ml-auto" />)}
      </button>

      {expanded && (
        <div className="border-t border-border/50">
          {error && (
            <div className="p-2 text-xs text-red-500">{error}</div>
          )}

          {loading && !varResult && (
            <div className="p-4 flex items-center justify-center gap-2 text-xs text-muted-foreground">
              <Loader2 size={14} className="animate-spin" />
              Berechne Risikoanalyse...
            </div>
          )}

          {/* VaR / CVaR */}
          {varResult && (
            <Section title="VaR / CVaR" icon={Shield} defaultOpen>
              <div className="space-y-1">
                <Metric label="VaR (95%) täglich" value={formatPct(varResult.var95)} color="text-red-500" />
                <Metric label="CVaR (95%) täglich" value={formatPct(varResult.cvar95)} color="text-red-500" />
                <Metric label="VaR (99%) täglich" value={formatPct(varResult.var99)} color="text-red-600" />
                <Metric label="CVaR (99%) täglich" value={formatPct(varResult.cvar99)} color="text-red-600" />
                <div className="border-t border-border/30 mt-1 pt-1" />
                <Metric label="VaR (95%) annualisiert" value={formatPct(varResult.var95Annual)} color="text-red-500" />
                <Metric label="CVaR (95%) annualisiert" value={formatPct(varResult.cvar95Annual)} color="text-red-500" />
                <Metric label="Beobachtungen" value={String(varResult.observations)} />
              </div>
              <button
                onClick={() => setShowAiSummary(showAiSummary === 'var' ? null : 'var')}
                className="mt-1 flex items-center gap-1 text-[10px] text-blue-400 hover:text-blue-300"
              >
                <Brain size={10} /> KI-Erklärung
              </button>
              {showAiSummary === 'var' && (
                <p className="mt-1 text-[10px] text-muted-foreground leading-relaxed">{varResult.aiSummary}</p>
              )}
            </Section>
          )}

          {/* Drawdown */}
          {drawdownResult && (
            <Section title="Drawdown-Analyse" icon={TrendingDown}>
              <div className="space-y-1">
                <Metric
                  label="Aktueller Drawdown"
                  value={formatPct(drawdownResult.currentDrawdown)}
                  color={drawdownResult.currentDrawdown < -5 ? 'text-red-500' : 'text-muted-foreground'}
                />
                <Metric label="Durchschnittl. DD" value={formatPct(drawdownResult.averageDrawdown)} />
                {drawdownResult.worstDrawdowns.slice(0, 3).map((dd, i) => (
                  <div key={i} className="mt-1 p-1.5 bg-red-500/5 rounded text-[10px]">
                    <div className="flex justify-between">
                      <span className="text-red-500 font-medium">{formatPct(dd.depth)}</span>
                      <span className="text-muted-foreground">{dd.durationDays}d</span>
                    </div>
                    <div className="text-muted-foreground">
                      {dd.startDate} → {dd.troughDate}
                      {dd.recoveryDate ? ` ↑ ${dd.recoveryDate}` : ' (offen)'}
                    </div>
                  </div>
                ))}
              </div>
              <button
                onClick={() => setShowAiSummary(showAiSummary === 'dd' ? null : 'dd')}
                className="mt-1 flex items-center gap-1 text-[10px] text-blue-400 hover:text-blue-300"
              >
                <Brain size={10} /> KI-Erklärung
              </button>
              {showAiSummary === 'dd' && (
                <p className="mt-1 text-[10px] text-muted-foreground leading-relaxed">{drawdownResult.aiSummary}</p>
              )}
            </Section>
          )}

          {/* Rolling Returns */}
          {rollingResult && (
            <Section title="Rolling Returns" icon={BarChart3}>
              <div className="space-y-2">
                {rollingResult.windows.map(w => (
                  <div key={w.windowLabel} className="space-y-0.5">
                    <div className="text-[10px] font-medium text-muted-foreground">{w.windowLabel}</div>
                    <Metric label="Beste" value={formatPct(w.best)} color="text-green-500" />
                    <Metric label="Schlechteste" value={formatPct(w.worst)} color="text-red-500" />
                    <Metric label="Durchschnitt" value={formatPct(w.average)} />
                    <Metric label="Positiv" value={formatPct(w.positivePct, 0)} />
                  </div>
                ))}
              </div>
            </Section>
          )}

          {/* Monte Carlo */}
          {monteCarloResult && (
            <Section title="Monte Carlo Simulation" icon={Dice5}>
              <div className="space-y-1">
                <Metric label="Simulationen" value={formatNumber(monteCarloResult.numSimulations)} />
                <Metric label="Zeitraum" value={`${monteCarloResult.daysForward} Tage`} />
                <div className="border-t border-border/30 mt-1 pt-1" />
                <Metric
                  label="Erwartungswert"
                  value={`${formatNumber(monteCarloResult.finalMean)} ${currency || ''}`}
                />
                <Metric
                  label="Median"
                  value={`${formatNumber(monteCarloResult.finalMedian)} ${currency || ''}`}
                />
                <Metric
                  label="Verlustwahrscheinlichkeit"
                  value={formatPct(monteCarloResult.probLoss, 0)}
                  color="text-red-500"
                />
                <Metric
                  label="P(>10% Gewinn)"
                  value={formatPct(monteCarloResult.probGain10, 0)}
                  color="text-green-500"
                />
                <div className="mt-1 p-1.5 bg-muted/30 rounded text-[10px] space-y-0.5">
                  <div className="flex justify-between">
                    <span className="text-red-400">5%-Szenario</span>
                    <span className="font-mono">{formatNumber(monteCarloResult.percentile5[monteCarloResult.percentile5.length - 1]?.value || 0)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">25%-Szenario</span>
                    <span className="font-mono">{formatNumber(monteCarloResult.percentile25[monteCarloResult.percentile25.length - 1]?.value || 0)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Median</span>
                    <span className="font-mono">{formatNumber(monteCarloResult.percentile50[monteCarloResult.percentile50.length - 1]?.value || 0)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">75%-Szenario</span>
                    <span className="font-mono">{formatNumber(monteCarloResult.percentile75[monteCarloResult.percentile75.length - 1]?.value || 0)}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-green-400">95%-Szenario</span>
                    <span className="font-mono">{formatNumber(monteCarloResult.percentile95[monteCarloResult.percentile95.length - 1]?.value || 0)}</span>
                  </div>
                </div>
              </div>
              <button
                onClick={() => setShowAiSummary(showAiSummary === 'mc' ? null : 'mc')}
                className="mt-1 flex items-center gap-1 text-[10px] text-blue-400 hover:text-blue-300"
              >
                <Brain size={10} /> KI-Erklärung
              </button>
              {showAiSummary === 'mc' && (
                <p className="mt-1 text-[10px] text-muted-foreground leading-relaxed">{monteCarloResult.aiSummary}</p>
              )}
            </Section>
          )}

          {/* GARCH Volatility */}
          {garchResult && (
            <Section title="GARCH Volatilität" icon={Activity}>
              <div className="space-y-1">
                <Metric
                  label="Aktuelle Volatilität"
                  value={formatPct(garchResult.currentVolatility)}
                  color={garchResult.currentVolatility > 30 ? 'text-red-500' : garchResult.currentVolatility > 20 ? 'text-yellow-500' : 'text-green-500'}
                />
                <Metric label="Langfristige Vol." value={formatPct(garchResult.longRunVolatility)} />
                <Metric label="Persistenz (α+β)" value={garchResult.persistence.toFixed(3)} />
                <Metric label="Konvergiert" value={garchResult.converged ? 'Ja' : 'Nein'} />
                <div className="border-t border-border/30 mt-1 pt-1" />
                <div className="text-[10px] text-muted-foreground">
                  GARCH(1,1): ω={garchResult.omega.toFixed(6)}, α={garchResult.alpha.toFixed(3)}, β={garchResult.beta.toFixed(3)}
                </div>
              </div>
              <button
                onClick={() => setShowAiSummary(showAiSummary === 'garch' ? null : 'garch')}
                className="mt-1 flex items-center gap-1 text-[10px] text-blue-400 hover:text-blue-300"
              >
                <Brain size={10} /> KI-Erklärung
              </button>
              {showAiSummary === 'garch' && (
                <p className="mt-1 text-[10px] text-muted-foreground leading-relaxed">{garchResult.aiSummary}</p>
              )}
            </Section>
          )}

          {/* Refresh button */}
          {varResult && (
            <div className="p-2 border-t border-border/50">
              <button
                onClick={runAnalysis}
                disabled={loading}
                className="w-full text-[10px] text-center text-muted-foreground hover:text-foreground transition-colors"
              >
                {loading ? 'Berechne...' : 'Aktualisieren'}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default RiskAnalysisPanel;
