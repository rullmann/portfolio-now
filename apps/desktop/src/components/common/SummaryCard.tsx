/**
 * Summary card component for displaying key metrics.
 */

interface SummaryCardProps {
  title: string;
  value: string;
  change?: string;
  positive?: boolean;
}

export function SummaryCard({ title, value, change, positive = true }: SummaryCardProps) {
  return (
    <div className="card-surface p-3">
      <div className="text-xs text-muted-foreground mb-0.5">{title}</div>
      <div className="text-lg font-semibold font-mono tabular-nums">{value}</div>
      {change && (
        <div className={`text-xs font-mono tabular-nums ${positive ? 'text-positive' : 'text-negative'}`}>
          {change}
        </div>
      )}
    </div>
  );
}
