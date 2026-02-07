import { Search, TrendingUp, Database, CheckCircle, XCircle, Loader2, ShieldCheck } from 'lucide-react';
import { cn } from '../../lib/utils';
import { type PendingQuery, type QueryType, getQueryTypeLabel, getQueryTypeDescription } from '../../lib/types';

interface QueryApprovalCardProps {
  query: PendingQuery;
  onApproveOnce: () => void;
  onApproveSession: () => void;
  onDecline: () => void;
  isExecuting?: boolean;
  disabled?: boolean;
}

/**
 * Get the appropriate icon for a query type
 */
function getQueryTypeIcon(queryType: QueryType) {
  switch (queryType) {
    case 'sql_query':
      return Database;
    case 'transaction_query':
      return Search;
    case 'portfolio_value_query':
      return TrendingUp;
    case 'database_query':
      return Database;
    default:
      return Database;
  }
}

/**
 * Card component for approving or declining pending database queries.
 *
 * SECURITY: This component is critical for the query approval security feature.
 * Users must explicitly approve queries before they can access their data.
 *
 * Options:
 * - "Einmal erlauben": Execute query once without session approval
 * - "Für Sitzung erlauben": Approve this query type for the entire session
 * - "Ablehnen": Decline the query, no data is exposed
 */
export function QueryApprovalCard({
  query,
  onApproveOnce,
  onApproveSession,
  onDecline,
  isExecuting = false,
  disabled = false,
}: QueryApprovalCardProps) {
  const Icon = getQueryTypeIcon(query.queryType);
  const label = getQueryTypeLabel(query.queryType);
  const typeDescription = getQueryTypeDescription(query.queryType);

  return (
    <div className="p-4 rounded-lg border-2 border-amber-500/50 bg-amber-500/5">
      <div className="flex items-start gap-3">
        <div className="p-2 rounded-lg bg-amber-500/10">
          <Icon className="h-5 w-5 text-amber-600" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span className="font-medium text-sm">{label}</span>
            <span className="text-xs px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-700 dark:text-amber-400">
              Genehmigung erforderlich
            </span>
          </div>
          <p className="text-sm text-muted-foreground mb-2">{query.description}</p>
          <p className="text-xs text-muted-foreground/70 mb-3">{typeDescription}</p>

          {/* Action buttons */}
          <div className="flex flex-wrap gap-2">
            <button
              onClick={onApproveOnce}
              disabled={disabled || isExecuting}
              className={cn(
                'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
                'bg-green-600 text-white hover:bg-green-700',
                'disabled:opacity-50 disabled:cursor-not-allowed'
              )}
            >
              {isExecuting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <CheckCircle className="h-3.5 w-3.5" />
              )}
              Einmal erlauben
            </button>

            <button
              onClick={onApproveSession}
              disabled={disabled || isExecuting}
              className={cn(
                'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
                'bg-primary text-primary-foreground hover:bg-primary/90',
                'disabled:opacity-50 disabled:cursor-not-allowed'
              )}
            >
              <ShieldCheck className="h-3.5 w-3.5" />
              Für Sitzung erlauben
            </button>

            <button
              onClick={onDecline}
              disabled={disabled || isExecuting}
              className={cn(
                'inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md transition-colors',
                'bg-muted hover:bg-muted/80',
                'disabled:opacity-50 disabled:cursor-not-allowed'
              )}
            >
              <XCircle className="h-3.5 w-3.5" />
              Ablehnen
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
