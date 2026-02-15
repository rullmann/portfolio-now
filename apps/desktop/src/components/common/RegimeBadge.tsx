import type { RegimeType } from '../../lib/indicators';

const REGIME_CONFIG: Record<RegimeType, { label: string; bg: string; text: string }> = {
  strong_uptrend: { label: 'Stark \u2191', bg: 'bg-green-700/15', text: 'text-green-700 dark:text-green-400' },
  uptrend: { label: 'Aufwärts', bg: 'bg-green-500/15', text: 'text-green-600 dark:text-green-400' },
  sideways: { label: 'Seitwärts', bg: 'bg-yellow-500/15', text: 'text-yellow-600 dark:text-yellow-400' },
  downtrend: { label: 'Abwärts', bg: 'bg-red-500/15', text: 'text-red-600 dark:text-red-400' },
  strong_downtrend: { label: 'Stark \u2193', bg: 'bg-red-700/15', text: 'text-red-700 dark:text-red-400' },
  squeeze: { label: 'Squeeze', bg: 'bg-purple-500/15', text: 'text-purple-600 dark:text-purple-400' },
};

interface RegimeBadgeProps {
  regime: RegimeType;
  size?: 'sm' | 'md';
}

export function RegimeBadge({ regime, size = 'sm' }: RegimeBadgeProps) {
  const config = REGIME_CONFIG[regime] || REGIME_CONFIG.sideways;
  const sizeClass = size === 'md'
    ? 'px-2.5 py-1 text-sm font-medium'
    : 'px-1.5 py-0.5 text-xs';

  return (
    <span className={`inline-flex items-center rounded ${config.bg} ${config.text} ${sizeClass}`}>
      {config.label}
    </span>
  );
}
