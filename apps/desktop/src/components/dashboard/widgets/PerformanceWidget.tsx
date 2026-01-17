/**
 * Performance Widget - Shows TTWROR and IRR metrics
 */

import type { WidgetProps } from '../types';

interface PerformanceWidgetProps extends WidgetProps {
  ttwror?: number;
  irr?: number;
  days?: number;
}

export function PerformanceWidget({
  ttwror = 0,
  irr = 0,
  days = 0,
}: PerformanceWidgetProps) {
  const formatPercent = (value: number) => {
    return new Intl.NumberFormat('de-DE', {
      style: 'percent',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
      signDisplay: 'exceptZero',
    }).format(value / 100);
  };

  const getColorClass = (value: number) => {
    if (value > 0) return 'text-green-600';
    if (value < 0) return 'text-red-600';
    return 'text-muted-foreground';
  };

  return (
    <div className="h-full flex flex-col justify-center p-4">
      <div className="text-xs text-muted-foreground uppercase tracking-wide mb-3">
        Performance
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div>
          <div className="text-xs text-muted-foreground">TTWROR</div>
          <div className={`text-xl font-semibold ${getColorClass(ttwror)}`}>
            {formatPercent(ttwror)}
          </div>
        </div>
        <div>
          <div className="text-xs text-muted-foreground">IRR</div>
          <div className={`text-xl font-semibold ${getColorClass(irr)}`}>
            {formatPercent(irr)}
          </div>
        </div>
      </div>
      {days > 0 && (
        <div className="text-xs text-muted-foreground mt-2">
          {days} Tage
        </div>
      )}
    </div>
  );
}
