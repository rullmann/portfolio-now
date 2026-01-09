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
    <div className="bg-card rounded-lg border border-border p-3">
      <div className="text-xs text-muted-foreground mb-0.5">{title}</div>
      <div className="text-lg font-semibold">{value}</div>
      {change && (
        <div className={`text-xs ${positive ? 'text-green-500' : 'text-red-500'}`}>
          {change}
        </div>
      )}
    </div>
  );
}
