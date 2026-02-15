interface SetupScoreBadgeProps {
  score: number;
  size?: 'sm' | 'md';
}

function getScoreConfig(score: number): { bg: string; text: string; label: string } {
  if (score >= 80) return { bg: 'bg-emerald-500/15', text: 'text-emerald-600 dark:text-emerald-400', label: 'Exzellent' };
  if (score >= 60) return { bg: 'bg-green-500/15', text: 'text-green-600 dark:text-green-400', label: 'Gut' };
  if (score >= 30) return { bg: 'bg-yellow-500/15', text: 'text-yellow-600 dark:text-yellow-400', label: 'Moderat' };
  return { bg: 'bg-red-500/15', text: 'text-red-600 dark:text-red-400', label: 'Schwach' };
}

export function SetupScoreBadge({ score, size = 'sm' }: SetupScoreBadgeProps) {
  const config = getScoreConfig(score);
  const sizeClass = size === 'md'
    ? 'px-2.5 py-1 text-sm font-semibold'
    : 'px-1.5 py-0.5 text-xs font-medium';

  return (
    <span
      className={`inline-flex items-center rounded ${config.bg} ${config.text} ${sizeClass}`}
      title={config.label}
    >
      {Math.round(score)}
    </span>
  );
}
