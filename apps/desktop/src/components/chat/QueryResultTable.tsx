/**
 * QueryResultTable - Sortable table component for displaying database query results.
 *
 * Features:
 * - Sortable columns (click header to sort)
 * - Compact display
 * - CSV export button
 * - Horizontal scroll for wide tables
 */

import { useState, useMemo } from 'react';
import { ArrowUpDown, ArrowUp, ArrowDown, Download } from 'lucide-react';
import { cn } from '../../lib/utils';

export interface QueryResultData {
  template_id: string;
  columns: string[];
  rows: Array<Record<string, unknown>>;
  row_count: number;
  formatted_markdown?: string;
}

interface QueryResultTableProps {
  data: QueryResultData;
  className?: string;
}

type SortDirection = 'asc' | 'desc' | null;

export function QueryResultTable({ data, className }: QueryResultTableProps) {
  const [sortColumn, setSortColumn] = useState<string | null>(null);
  const [sortDirection, setSortDirection] = useState<SortDirection>(null);

  // Sort rows based on current sort state
  const sortedRows = useMemo(() => {
    if (!sortColumn || !sortDirection) {
      return data.rows;
    }

    return [...data.rows].sort((a, b) => {
      const aVal = a[sortColumn];
      const bVal = b[sortColumn];

      // Handle null/undefined
      if (aVal == null && bVal == null) return 0;
      if (aVal == null) return sortDirection === 'asc' ? -1 : 1;
      if (bVal == null) return sortDirection === 'asc' ? 1 : -1;

      // Numeric comparison
      if (typeof aVal === 'number' && typeof bVal === 'number') {
        return sortDirection === 'asc' ? aVal - bVal : bVal - aVal;
      }

      // String comparison
      const aStr = String(aVal).toLowerCase();
      const bStr = String(bVal).toLowerCase();
      const comparison = aStr.localeCompare(bStr, 'de');
      return sortDirection === 'asc' ? comparison : -comparison;
    });
  }, [data.rows, sortColumn, sortDirection]);

  // Toggle sort on column header click
  const handleSort = (column: string) => {
    if (sortColumn === column) {
      // Cycle through: asc -> desc -> null
      if (sortDirection === 'asc') {
        setSortDirection('desc');
      } else if (sortDirection === 'desc') {
        setSortColumn(null);
        setSortDirection(null);
      }
    } else {
      setSortColumn(column);
      setSortDirection('asc');
    }
  };

  // Export to CSV
  const exportToCsv = () => {
    const headers = data.columns.join(';');
    const rows = sortedRows.map(row =>
      data.columns.map(col => {
        const val = row[col];
        if (val == null) return '';
        // Quote strings that contain semicolons or quotes
        const str = String(val);
        if (str.includes(';') || str.includes('"') || str.includes('\n')) {
          return `"${str.replace(/"/g, '""')}"`;
        }
        return str;
      }).join(';')
    );

    const csv = [headers, ...rows].join('\n');
    const blob = new Blob(['\ufeff' + csv], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `${data.template_id}_${new Date().toISOString().split('T')[0]}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  // Format cell value for display
  const formatValue = (value: unknown, column: string): string => {
    if (value == null) return '-';

    if (typeof value === 'number') {
      // Detect if it's a percentage
      if (column.includes('pct') || column.includes('percent') || column.includes('allocation')) {
        return `${value.toFixed(2)}%`;
      }
      // Detect if it's currency/amount
      if (column.includes('amount') || column.includes('value') || column.includes('cost') ||
          column.includes('price') || column.includes('gain') || column.includes('loss') ||
          column.includes('dividend') || column.includes('basis')) {
        return value.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
      }
      // Shares
      if (column.includes('shares') || column.includes('count')) {
        return value.toLocaleString('de-DE', { minimumFractionDigits: 0, maximumFractionDigits: 4 });
      }
      return value.toLocaleString('de-DE');
    }

    return String(value);
  };

  // Get display name for column
  const getColumnDisplayName = (column: string): string => {
    const translations: Record<string, string> = {
      security_name: 'Wertpapier',
      ticker: 'Ticker',
      isin: 'ISIN',
      currency: 'Währung',
      shares: 'Stück',
      current_price: 'Kurs',
      current_value: 'Wert',
      cost_basis: 'Einstand',
      gain_loss: 'G/V',
      gain_loss_pct: 'G/V %',
      allocation_pct: 'Anteil %',
      total_value: 'Gesamtwert',
      total_cost_basis: 'Ges. Einstand',
      unrealized_return_pct: 'Unrealisiert %',
      unrealized_gain_loss: 'Unrealisiert',
      total_dividends: 'Dividenden',
      realized_gains: 'Realisiert',
      position_count: 'Positionen',
      year: 'Jahr',
      sale_date: 'Verkauf',
      purchase_date: 'Kauf',
      holding_days: 'Haltetage',
      category: 'Kategorie',
      asset_type: 'Assetklasse',
      period_label: 'Zeitraum',
    };
    return translations[column] || column.replace(/_/g, ' ');
  };

  if (data.rows.length === 0) {
    return (
      <div className={cn('text-sm text-muted-foreground p-3 bg-muted/50 rounded', className)}>
        Keine Ergebnisse gefunden.
      </div>
    );
  }

  return (
    <div className={cn('space-y-2', className)}>
      {/* Header with export button */}
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {data.row_count} Ergebnis{data.row_count !== 1 ? 'se' : ''}
        </span>
        <button
          onClick={exportToCsv}
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors px-2 py-1 rounded hover:bg-muted"
          title="Als CSV exportieren"
        >
          <Download className="h-3 w-3" />
          CSV
        </button>
      </div>

      {/* Table with horizontal scroll */}
      <div className="overflow-x-auto rounded border border-border">
        <table className="w-full text-xs">
          <thead>
            <tr className="bg-muted/50">
              {data.columns.map(column => (
                <th
                  key={column}
                  className="text-left px-2 py-1.5 font-medium cursor-pointer hover:bg-muted transition-colors select-none whitespace-nowrap"
                  onClick={() => handleSort(column)}
                >
                  <div className="flex items-center gap-1">
                    <span>{getColumnDisplayName(column)}</span>
                    {sortColumn === column ? (
                      sortDirection === 'asc' ? (
                        <ArrowUp className="h-3 w-3" />
                      ) : (
                        <ArrowDown className="h-3 w-3" />
                      )
                    ) : (
                      <ArrowUpDown className="h-3 w-3 opacity-30" />
                    )}
                  </div>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sortedRows.map((row, rowIndex) => (
              <tr
                key={rowIndex}
                className={cn(
                  'border-t border-border',
                  rowIndex % 2 === 0 ? 'bg-background' : 'bg-muted/20'
                )}
              >
                {data.columns.map(column => {
                  const value = row[column];
                  const formatted = formatValue(value, column);
                  const isNegative = typeof value === 'number' && value < 0;
                  const isPositive = typeof value === 'number' && value > 0 &&
                    (column.includes('gain') || column.includes('return') || column.includes('pct'));

                  return (
                    <td
                      key={column}
                      className={cn(
                        'px-2 py-1.5 whitespace-nowrap',
                        isNegative && 'text-red-600 dark:text-red-400',
                        isPositive && 'text-green-600 dark:text-green-400'
                      )}
                    >
                      {formatted}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
