import { useState } from 'react';
import type { TrendInfo, RiskRewardAnalysis } from '../../lib/types';
import type { TechnicalSignal } from '../../lib/signals';
import { ShareToXModal } from './ShareToXModal';

/** Minimal security info needed for sharing */
export interface ShareSecurityInfo {
  id: number;
  name: string;
  ticker?: string | null;
  currency: string;
}

/** X logo SVG (current brand mark) */
function XLogo({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
    </svg>
  );
}

interface ShareToXButtonProps {
  chartRef: React.RefObject<HTMLDivElement>;
  security: ShareSecurityInfo | null;
  currentPrice: number;
  analysis?: string;
  trendInfo?: TrendInfo;
  riskReward?: RiskRewardAnalysis;
  signals?: TechnicalSignal[];
  patterns?: string[];
  variant: 'icon' | 'button';
}

export function ShareToXButton({
  chartRef,
  security,
  currentPrice,
  analysis,
  trendInfo,
  riskReward,
  signals,
  patterns,
  variant,
}: ShareToXButtonProps) {
  const [isModalOpen, setIsModalOpen] = useState(false);

  if (!security) return null;

  return (
    <>
      {variant === 'icon' ? (
        <button
          onClick={() => setIsModalOpen(true)}
          className="p-1.5 hover:bg-muted rounded-md transition-colors text-muted-foreground hover:text-foreground"
          title="Auf X teilen"
        >
          <XLogo size={14} />
        </button>
      ) : (
        <button
          onClick={() => setIsModalOpen(true)}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-card border border-border rounded-lg hover:bg-muted transition-colors"
        >
          <XLogo size={12} />
          <span>Auf X teilen</span>
        </button>
      )}

      {isModalOpen && (
        <ShareToXModal
          chartRef={chartRef}
          security={security}
          currentPrice={currentPrice}
          analysis={analysis}
          trendInfo={trendInfo}
          riskReward={riskReward}
          signals={signals}
          patterns={patterns}
          onClose={() => setIsModalOpen(false)}
        />
      )}
    </>
  );
}
