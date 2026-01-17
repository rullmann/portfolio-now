/**
 * AlertBadge - Small indicator showing allocation alert count
 * Can be used in sidebar or header navigation
 */

import { useEffect, useState } from 'react';
import { getAllocationAlertCount } from '../../lib/api';
import type { AllocationAlertCount } from '../../lib/types';

interface AlertBadgeProps {
  portfolioId?: number;
  className?: string;
  showZero?: boolean;
}

export function AlertBadge({ portfolioId, className = '', showZero = false }: AlertBadgeProps) {
  const [alertCount, setAlertCount] = useState<AllocationAlertCount | null>(null);

  useEffect(() => {
    const loadCount = async () => {
      try {
        const count = await getAllocationAlertCount(portfolioId);
        setAlertCount(count);
      } catch {
        // Silently fail - badge is non-critical UI
        setAlertCount(null);
      }
    };

    loadCount();

    // Refresh every 60 seconds
    const interval = setInterval(loadCount, 60000);
    return () => clearInterval(interval);
  }, [portfolioId]);

  if (!alertCount || (alertCount.total === 0 && !showZero)) {
    return null;
  }

  const bgColor = alertCount.critical > 0
    ? 'bg-red-500'
    : alertCount.warning > 0
      ? 'bg-yellow-500'
      : 'bg-muted';

  const textColor = alertCount.critical > 0 || alertCount.warning > 0
    ? 'text-white'
    : 'text-muted-foreground';

  return (
    <span
      className={`inline-flex items-center justify-center min-w-5 h-5 px-1.5 text-xs font-medium rounded-full ${bgColor} ${textColor} ${className}`}
      title={`${alertCount.total} Allokations-Warnung${alertCount.total !== 1 ? 'en' : ''}`}
    >
      {alertCount.total}
    </span>
  );
}
