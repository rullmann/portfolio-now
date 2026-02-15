/**
 * Trading Briefing Widget - Top securities by setup score
 */

import { useEffect, useState } from 'react';
import { Activity, RefreshCw } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { RegimeBadge } from '../../common/RegimeBadge';
import { SetupScoreBadge } from '../../common/SetupScoreBadge';
import { SecurityLogo } from '../../common/SecurityLogo';
import { getPriceHistory } from '../../../lib/api';
import { convertToOHLC } from '../../../lib/indicators';
import type { RegimeAnalysis, SetupScore } from '../../../lib/indicators';
import { detectRegime, scoreSetup } from '../../../lib/indicators-rust';
import { useCachedLogos } from '../../../lib/hooks';
import { useSettingsStore, useUIStore } from '../../../store';
import type { WidgetProps } from '../types';

interface BriefingEntry {
  securityId: number;
  name: string;
  ticker: string | null;
  regime: RegimeAnalysis;
  setup: SetupScore;
}

export function TradingBriefingWidget({ config: _config }: WidgetProps) {
  const [entries, setEntries] = useState<BriefingEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const { brandfetchApiKey } = useSettingsStore();
  const { setCurrentView, setScrollTarget } = useUIStore();

  const securitiesForLogos = entries.map(e => ({
    id: e.securityId,
    ticker: e.ticker || undefined,
    name: e.name,
  }));
  const { logos } = useCachedLogos(securitiesForLogos, brandfetchApiKey);

  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        // Get holdings + watchlist securities
        const holdings = await invoke<Array<{ securityIds: number[]; name?: string }>>('get_all_holdings');
        const holdingIds = new Set(holdings.flatMap(h => h.securityIds));

        const allSecurities = await invoke<Array<{ id: number; name: string; ticker: string | null }>>('get_securities', { importId: null });
        const candidates = allSecurities.filter(s => holdingIds.has(s.id));

        const sixMonthsAgo = new Date();
        sixMonthsAgo.setMonth(sixMonthsAgo.getMonth() - 6);
        const from = sixMonthsAgo.toISOString().split('T')[0];
        const to = new Date().toISOString().split('T')[0];

        const results: BriefingEntry[] = [];

        await Promise.all(
          candidates.slice(0, 20).map(async (sec) => {
            try {
              const prices = await getPriceHistory(sec.id, from, to);
              if (prices.length >= 50) {
                const ohlc = convertToOHLC(prices, 1.5);
                const [regime, setup] = await Promise.all([
                  detectRegime(ohlc),
                  scoreSetup(ohlc),
                ]);
                results.push({
                  securityId: sec.id,
                  name: sec.name,
                  ticker: sec.ticker,
                  regime,
                  setup,
                });
              }
            } catch { /* skip */ }
          })
        );

        // Sort by setup score descending, take top 5
        results.sort((a, b) => b.setup.totalScore - a.setup.totalScore);
        setEntries(results.slice(0, 5));
      } catch (err) {
        console.error('Trading briefing failed:', err);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  const handleClick = (securityId: number) => {
    setScrollTarget(securityId.toString());
    setCurrentView('charts');
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center">
        <RefreshCw className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col p-4">
      <div className="flex items-center gap-2 mb-3">
        <Activity size={14} className="text-primary" />
        <div className="text-xs text-muted-foreground uppercase tracking-wide">
          Trading Briefing
        </div>
      </div>

      {entries.length === 0 ? (
        <div className="flex-1 flex items-center justify-center text-sm text-muted-foreground">
          Keine Daten verfügbar
        </div>
      ) : (
        <div className="flex-1 overflow-auto space-y-1.5">
          {entries.map((entry) => (
            <button
              key={entry.securityId}
              onClick={() => handleClick(entry.securityId)}
              className="w-full flex items-center gap-2 p-2 rounded hover:bg-muted/50 transition-colors text-left"
            >
              <SecurityLogo securityId={entry.securityId} logos={logos} size={24} />
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium truncate">{entry.name}</div>
                <div className="text-xs text-muted-foreground">{entry.setup.setupLabel}</div>
              </div>
              <RegimeBadge regime={entry.regime.regime} />
              <SetupScoreBadge score={entry.setup.totalScore} />
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
