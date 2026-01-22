/**
 * List of securities with quote problems.
 * Displays securities that have no provider, fetch errors, or stale quotes.
 */

import { AlertCircle, Clock, XCircle } from 'lucide-react';
import type { ProblematicSecurity } from '../../lib/types';

interface ProblematicSecurityListProps {
  securities: ProblematicSecurity[];
  selectedId: number | null;
  onSelect: (security: ProblematicSecurity) => void;
  isLoading?: boolean;
}

export function ProblematicSecurityList({
  securities,
  selectedId,
  onSelect,
  isLoading,
}: ProblematicSecurityListProps) {
  if (isLoading) {
    return (
      <div className="space-y-2">
        {[1, 2, 3].map((i) => (
          <div
            key={i}
            className="h-16 bg-muted/50 rounded-lg animate-pulse"
          />
        ))}
      </div>
    );
  }

  if (securities.length === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <AlertCircle className="h-8 w-8 mx-auto mb-2 opacity-50" />
        <p>Keine problematischen Wertpapiere gefunden</p>
      </div>
    );
  }

  const getProblemIcon = (type: string) => {
    switch (type) {
      case 'no_provider':
        return <XCircle className="h-4 w-4 text-red-500" />;
      case 'fetch_error':
        return <AlertCircle className="h-4 w-4 text-orange-500" />;
      case 'stale':
        return <Clock className="h-4 w-4 text-yellow-500" />;
      default:
        return <AlertCircle className="h-4 w-4 text-muted-foreground" />;
    }
  };

  return (
    <div className="space-y-2">
      <div className="text-sm font-medium text-muted-foreground mb-3">
        {securities.length} Wertpapier{securities.length !== 1 ? 'e' : ''} mit Problemen
      </div>
      {securities.map((security) => (
        <button
          key={security.id}
          onClick={() => onSelect(security)}
          className={`w-full text-left p-3 rounded-lg border transition-colors ${
            selectedId === security.id
              ? 'border-primary bg-primary/5'
              : 'border-border hover:border-primary/50 hover:bg-muted/50'
          }`}
        >
          <div className="flex items-start gap-2">
            <div className="mt-0.5">{getProblemIcon(security.problemType)}</div>
            <div className="flex-1 min-w-0">
              <div className="font-medium truncate">{security.name}</div>
              <div className="text-xs text-muted-foreground">
                {security.isin && <span>{security.isin}</span>}
                {security.isin && security.ticker && <span> · </span>}
                {security.ticker && <span>{security.ticker}</span>}
              </div>
              <div className="text-xs text-muted-foreground mt-1">
                {security.problemDescription}
              </div>
            </div>
          </div>
        </button>
      ))}
    </div>
  );
}
