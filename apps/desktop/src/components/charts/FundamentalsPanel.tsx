/**
 * Fundamentals & Earnings Panel
 * Shows fundamental financial data and earnings history for a security
 * Data fetched from Yahoo Finance (free)
 */

import { useState, useCallback } from 'react';
import {
  BarChart3,
  ChevronDown,
  ChevronRight,
  Loader2,
  TrendingUp,
  DollarSign,
  PieChart,
  Target,
  Calendar,
  Brain,
} from 'lucide-react';
import type { YahooFundamentals, YahooEarnings } from '../../lib/api';
import { fetchSecurityFundamentals, fetchSecurityEarnings } from '../../lib/api';

interface FundamentalsPanelProps {
  securityId: number | null;
  securityName?: string;
  currency?: string;
}

function Metric({ label, value, color, suffix }: {
  label: string;
  value: string | number | undefined | null;
  color?: string;
  suffix?: string;
}) {
  if (value === undefined || value === null) return null;
  const displayValue = typeof value === 'number'
    ? (Math.abs(value) >= 1e9 ? `${(value / 1e9).toFixed(1)}B`
      : Math.abs(value) >= 1e6 ? `${(value / 1e6).toFixed(1)}M`
      : Math.abs(value) >= 1e3 ? `${(value / 1e3).toFixed(1)}K`
      : value.toFixed(2))
    : value;

  return (
    <div className="flex justify-between text-xs">
      <span className="text-muted-foreground">{label}</span>
      <span className={`font-mono tabular-nums ${color || ''}`}>
        {displayValue}{suffix || ''}
      </span>
    </div>
  );
}

function PctMetric({ label, value }: { label: string; value: number | undefined | null }) {
  if (value === undefined || value === null) return null;
  const pct = value * 100;
  const color = pct > 0 ? 'text-green-500' : pct < 0 ? 'text-red-500' : '';
  return <Metric label={label} value={`${pct >= 0 ? '+' : ''}${pct.toFixed(1)}%`} color={color} />;
}

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

function RecommendationBadge({ value, label }: { value?: number; label?: string }) {
  if (!value && !label) return null;

  let color = 'bg-gray-500';
  let text = label || '';

  if (value) {
    if (value <= 1.5) { color = 'bg-green-600'; text = text || 'Strong Buy'; }
    else if (value <= 2.5) { color = 'bg-green-500'; text = text || 'Buy'; }
    else if (value <= 3.5) { color = 'bg-yellow-500'; text = text || 'Hold'; }
    else if (value <= 4.5) { color = 'bg-red-400'; text = text || 'Sell'; }
    else { color = 'bg-red-600'; text = text || 'Strong Sell'; }
  }

  return (
    <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] text-white font-medium ${color}`}>
      {text} {value ? `(${value.toFixed(1)})` : ''}
    </span>
  );
}

export function FundamentalsPanel({ securityId, securityName, currency }: FundamentalsPanelProps) {
  const [expanded, setExpanded] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fundamentals, setFundamentals] = useState<YahooFundamentals | null>(null);
  const [earnings, setEarnings] = useState<YahooEarnings | null>(null);

  const fetchData = useCallback(async () => {
    if (!securityId) return;
    setLoading(true);
    setError(null);

    try {
      const [fund, earn] = await Promise.allSettled([
        fetchSecurityFundamentals(securityId),
        fetchSecurityEarnings(securityId),
      ]);

      if (fund.status === 'fulfilled') setFundamentals(fund.value);
      if (earn.status === 'fulfilled') setEarnings(earn.value);

      if (fund.status === 'rejected' && earn.status === 'rejected') {
        setError('Keine Fundamentaldaten verfügbar');
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [securityId]);

  if (!securityId) return null;

  return (
    <div className="card-surface">
      <button
        onClick={() => {
          setExpanded(!expanded);
          if (!expanded && !fundamentals && !loading) {
            fetchData();
          }
        }}
        className="flex items-center gap-2 w-full p-3 text-sm font-medium hover:bg-muted/30 transition-colors"
      >
        <BarChart3 size={14} className="text-purple-500" />
        <span>Fundamentaldaten</span>
        {loading && <Loader2 size={12} className="animate-spin ml-auto" />}
        {!loading && (expanded ? <ChevronDown size={14} className="ml-auto" /> : <ChevronRight size={14} className="ml-auto" />)}
      </button>

      {expanded && (
        <div className="border-t border-border/50">
          {error && (
            <div className="p-2 text-xs text-red-500">{error}</div>
          )}

          {loading && !fundamentals && (
            <div className="p-4 flex items-center justify-center gap-2 text-xs text-muted-foreground">
              <Loader2 size={14} className="animate-spin" />
              Lade Fundamentaldaten...
            </div>
          )}

          {/* Analyst Recommendation */}
          {fundamentals && (fundamentals.recommendationMean || fundamentals.recommendationKey) && (
            <div className="p-2 border-b border-border/50">
              <div className="flex items-center justify-between">
                <span className="text-xs text-muted-foreground">Analysten-Rating</span>
                <RecommendationBadge
                  value={fundamentals.recommendationMean}
                  label={fundamentals.recommendationKey}
                />
              </div>
              {fundamentals.numberOfAnalystOpinions && (
                <div className="text-[10px] text-muted-foreground mt-0.5">
                  {fundamentals.numberOfAnalystOpinions} Analysten
                </div>
              )}
              {fundamentals.targetMeanPrice && (
                <div className="flex justify-between text-xs mt-1">
                  <span className="text-muted-foreground">Kursziel</span>
                  <span className="font-mono">
                    {fundamentals.targetLowPrice?.toFixed(0)} - <strong>{fundamentals.targetMeanPrice?.toFixed(0)}</strong> - {fundamentals.targetHighPrice?.toFixed(0)} {currency}
                  </span>
                </div>
              )}
            </div>
          )}

          {/* Valuation */}
          {fundamentals && (
            <Section title="Bewertung" icon={DollarSign} defaultOpen>
              <div className="space-y-1">
                <Metric label="KGV (trailing)" value={fundamentals.trailingPe?.toFixed(1)} />
                <Metric label="KGV (forward)" value={fundamentals.forwardPe?.toFixed(1)} />
                <Metric label="KBV" value={fundamentals.priceToBook?.toFixed(2)} />
                <Metric label="KUV" value={fundamentals.priceToSales?.toFixed(2)} />
                <Metric label="EV/EBITDA" value={fundamentals.enterpriseToEbitda?.toFixed(1)} />
                <Metric label="PEG" value={fundamentals.pegRatio?.toFixed(2)} />
              </div>
            </Section>
          )}

          {/* Profitability */}
          {fundamentals && (
            <Section title="Profitabilität" icon={TrendingUp}>
              <div className="space-y-1">
                <PctMetric label="ROE" value={fundamentals.returnOnEquity} />
                <PctMetric label="ROA" value={fundamentals.returnOnAssets} />
                <PctMetric label="Gewinnmarge" value={fundamentals.profitMargin} />
                <PctMetric label="Operative Marge" value={fundamentals.operatingMargin} />
                <PctMetric label="Bruttomarge" value={fundamentals.grossMargin} />
              </div>
            </Section>
          )}

          {/* Financial Health */}
          {fundamentals && (
            <Section title="Finanzgesundheit" icon={PieChart}>
              <div className="space-y-1">
                <Metric label="Verschuldungsgrad" value={fundamentals.debtToEquity?.toFixed(1)} />
                <Metric label="Current Ratio" value={fundamentals.currentRatio?.toFixed(2)} />
                <Metric label="Quick Ratio" value={fundamentals.quickRatio?.toFixed(2)} />
                <Metric label="Free Cash Flow" value={fundamentals.freeCashFlow} suffix={` ${currency || ''}`} />
              </div>
            </Section>
          )}

          {/* Growth */}
          {fundamentals && (
            <Section title="Wachstum" icon={TrendingUp}>
              <div className="space-y-1">
                <PctMetric label="Umsatzwachstum" value={fundamentals.revenueGrowth} />
                <PctMetric label="Gewinnwachstum" value={fundamentals.earningsGrowth} />
                <Metric label="EPS" value={fundamentals.earningsPerShare?.toFixed(2)} suffix={` ${currency || ''}`} />
                <Metric label="Umsatz" value={fundamentals.revenue} suffix={` ${currency || ''}`} />
              </div>
            </Section>
          )}

          {/* Dividend */}
          {fundamentals && fundamentals.dividendYield != null && (
            <Section title="Dividende" icon={DollarSign}>
              <div className="space-y-1">
                <PctMetric label="Dividendenrendite" value={fundamentals.dividendYield} />
                <Metric label="Dividende p.a." value={fundamentals.dividendRate?.toFixed(2)} suffix={` ${currency || ''}`} />
                <PctMetric label="Ausschüttungsquote" value={fundamentals.payoutRatio} />
              </div>
            </Section>
          )}

          {/* Earnings */}
          {earnings && (
            <Section title="Earnings" icon={Calendar}>
              <div className="space-y-1">
                {earnings.nextEarningsDate && (
                  <div className="p-1.5 bg-blue-500/10 rounded text-[10px] mb-1">
                    <span className="text-blue-400">Nächste Earnings:</span>{' '}
                    <span className="font-mono">{earnings.nextEarningsDate}</span>
                  </div>
                )}

                {/* EPS Estimates */}
                {earnings.currentYearEstimate != null && (
                  <Metric label="EPS Schätzung (akt. Jahr)" value={earnings.currentYearEstimate.toFixed(2)} suffix={` ${currency || ''}`} />
                )}
                {earnings.nextYearEstimate != null && (
                  <Metric label="EPS Schätzung (nächst. Jahr)" value={earnings.nextYearEstimate.toFixed(2)} suffix={` ${currency || ''}`} />
                )}
                <PctMetric label="EPS-Wachstum (Schätzung)" value={earnings.epsGrowthEstimate} />

                {/* Quarterly History */}
                {earnings.quarterlyEarnings.length > 0 && (
                  <div className="mt-2">
                    <div className="text-[10px] font-medium text-muted-foreground mb-1">Quartals-EPS</div>
                    {earnings.quarterlyEarnings.slice(-4).reverse().map((q, i) => (
                      <div key={i} className="flex items-center justify-between text-[10px] py-0.5">
                        <span className="text-muted-foreground w-14">{q.quarter || q.date}</span>
                        <span className="font-mono">
                          {q.epsActual?.toFixed(2) || '–'}
                        </span>
                        {q.epsEstimate != null && (
                          <span className="text-muted-foreground font-mono">
                            Est: {q.epsEstimate.toFixed(2)}
                          </span>
                        )}
                        {q.epsSurprisePct != null && (
                          <span className={`font-mono ${q.epsSurprisePct >= 0 ? 'text-green-500' : 'text-red-500'}`}>
                            {q.epsSurprisePct >= 0 ? '+' : ''}{(q.epsSurprisePct * 100).toFixed(1)}%
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </Section>
          )}

          {/* Refresh */}
          {(fundamentals || earnings) && (
            <div className="p-2 border-t border-border/50">
              <button
                onClick={fetchData}
                disabled={loading}
                className="w-full text-[10px] text-center text-muted-foreground hover:text-foreground transition-colors"
              >
                {loading ? 'Lade...' : 'Aktualisieren'}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default FundamentalsPanel;
