/**
 * QueryApprovalCard - Displays pending queries that require user approval.
 *
 * SECURITY: Read-only queries (transactions, portfolio values, database queries)
 * can expose sensitive portfolio data. This component allows users to:
 * - Review what data the query will access
 * - Approve once (execute this query only)
 * - Approve for session (auto-execute this query type for the session)
 * - Decline (don't execute, remove the card)
 *
 * This prevents prompt injection attacks from exfiltrating data without consent.
 */

import { Search, TrendingUp, Database, CheckCircle, XCircle, Loader2, ShieldAlert } from 'lucide-react';
import type { PendingQuery, QueryType } from '../../lib/types';
import { getQueryTypeLabel, getQueryTypeDescription } from '../../lib/types';
import { cn } from '../../lib/utils';

interface QueryApprovalCardProps {
  /** The pending query that needs approval */
  query: PendingQuery;
  /** Called when user approves to execute once */
  onApproveOnce: () => void;
  /** Called when user approves for the entire session */
  onApproveSession: () => void;
  /** Called when user declines the query */
  onDecline: () => void;
  /** Whether the query is currently being executed */
  isExecuting: boolean;
}

/**
 * Get the appropriate icon for a query type
 */
function getQueryIcon(queryType: QueryType) {
  switch (queryType) {
    case 'transaction_query':
      return Search;
    case 'portfolio_value_query':
      return TrendingUp;
    case 'database_query':
      return Database;
    default:
      return Search;
  }
}

export function QueryApprovalCard({
  query,
  onApproveOnce,
  onApproveSession,
  onDecline,
  isExecuting,
}: QueryApprovalCardProps) {
  const Icon = getQueryIcon(query.queryType);

  return (
    <div className="p-4 rounded-lg border border-amber-500/50 bg-amber-500/5">
      {/* Header */}
      <div className="flex items-start gap-3 mb-3">
        <div className="p-2 rounded-lg bg-amber-500/10">
          <ShieldAlert className="h-5 w-5 text-amber-600" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="font-medium text-sm">Datenabfrage bestätigen</p>
          <p className="text-xs text-muted-foreground mt-0.5">
            Die KI möchte auf deine Portfolio-Daten zugreifen
          </p>
        </div>
      </div>

      {/* Query details */}
      <div className="mb-4 p-3 rounded-md bg-background/80 border border-border">
        <div className="flex items-center gap-2 mb-2">
          <Icon className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">{getQueryTypeLabel(query.queryType)}</span>
        </div>
        <p className="text-sm text-muted-foreground mb-2">
          {query.description}
        </p>
        <p className="text-xs text-muted-foreground/70">
          {getQueryTypeDescription(query.queryType)}
        </p>
      </div>

      {/* Action buttons */}
      <div className="flex flex-col gap-2">
        {/* Approve once */}
        <button
          onClick={onApproveOnce}
          disabled={isExecuting}
          className={cn(
            'w-full inline-flex items-center justify-center gap-2 px-4 py-2',
            'text-sm font-medium rounded-md',
            'bg-green-600 text-white hover:bg-green-700',
            'disabled:opacity-50 disabled:cursor-not-allowed',
            'transition-colors'
          )}
        >
          {isExecuting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <CheckCircle className="h-4 w-4" />
          )}
          Einmal erlauben
        </button>

        {/* Approve for session */}
        <button
          onClick={onApproveSession}
          disabled={isExecuting}
          className={cn(
            'w-full inline-flex items-center justify-center gap-2 px-4 py-2',
            'text-sm font-medium rounded-md',
            'bg-primary text-primary-foreground hover:bg-primary/90',
            'disabled:opacity-50 disabled:cursor-not-allowed',
            'transition-colors'
          )}
        >
          {isExecuting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <CheckCircle className="h-4 w-4" />
          )}
          Für Sitzung erlauben
        </button>

        {/* Decline */}
        <button
          onClick={onDecline}
          disabled={isExecuting}
          className={cn(
            'w-full inline-flex items-center justify-center gap-2 px-4 py-2',
            'text-sm font-medium rounded-md',
            'bg-muted hover:bg-muted/80',
            'disabled:opacity-50 disabled:cursor-not-allowed',
            'transition-colors'
          )}
        >
          <XCircle className="h-4 w-4" />
          Ablehnen
        </button>
      </div>

      {/* Security hint */}
      <p className="mt-3 text-xs text-muted-foreground/70 text-center">
        "Für Sitzung erlauben" gilt bis zum Neustart der App
      </p>
    </div>
  );
}
