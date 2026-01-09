/**
 * Error banner component for displaying error messages.
 */

import { useAppStore } from '../../store';

export function ErrorBanner() {
  const { error, clearError } = useAppStore();

  if (!error) return null;

  return (
    <div className="px-6 py-3 bg-red-100 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800 text-red-800 dark:text-red-200 text-sm flex items-center justify-between">
      <span>{error}</span>
      <button onClick={clearError} className="text-red-600 hover:text-red-800">
        Schließen
      </button>
    </div>
  );
}
