/**
 * Loading indicator component.
 */

import { useAppStore } from '../../store';

export function LoadingIndicator() {
  const { isLoading } = useAppStore();

  if (!isLoading) return null;

  return (
    <div className="px-6 py-2 bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-200 text-sm">
      Lädt...
    </div>
  );
}
