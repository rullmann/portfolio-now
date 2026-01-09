/**
 * Skeleton loading components for various UI elements.
 * Uses Tailwind's animate-pulse for consistent loading animations.
 */

interface SkeletonProps {
  className?: string;
}

/**
 * Base skeleton element with pulse animation.
 */
export function Skeleton({ className = '' }: SkeletonProps) {
  return (
    <div
      className={`animate-pulse bg-muted rounded ${className}`}
      aria-hidden="true"
    />
  );
}

/**
 * Skeleton matching SummaryCard dimensions (p-3, text-xs title, text-lg value).
 */
export function SummaryCardSkeleton() {
  return (
    <div className="bg-card rounded-lg border border-border p-3">
      <Skeleton className="h-3 w-16 mb-2" />
      <Skeleton className="h-6 w-24 mb-1" />
      <Skeleton className="h-3 w-12" />
    </div>
  );
}

/**
 * Single table row skeleton.
 */
export function TableRowSkeleton({ columns = 5 }: { columns?: number }) {
  return (
    <tr className="border-b border-border">
      {Array.from({ length: columns }).map((_, i) => (
        <td key={i} className="py-2 px-4">
          <Skeleton className="h-4 w-full max-w-[120px]" />
        </td>
      ))}
    </tr>
  );
}

/**
 * Complete table skeleton with header and rows.
 */
export function TableSkeleton({
  rows = 5,
  columns = 5,
}: {
  rows?: number;
  columns?: number;
}) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border">
            {Array.from({ length: columns }).map((_, i) => (
              <th key={i} className="text-left py-2 px-4">
                <Skeleton className="h-4 w-20" />
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {Array.from({ length: rows }).map((_, i) => (
            <TableRowSkeleton key={i} columns={columns} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

/**
 * Mini chart skeleton (90x28px matching TradingViewMiniChart).
 */
export function MiniChartSkeleton() {
  return (
    <div className="w-[90px] h-[28px]">
      <Skeleton className="w-full h-full" />
    </div>
  );
}

/**
 * Donut chart skeleton for Holdings view.
 */
export function DonutChartSkeleton({ size = 256 }: { size?: number }) {
  const innerSize = size * 0.5;
  return (
    <div className="flex items-center justify-center h-full">
      <div className="relative" style={{ width: size, height: size }}>
        <Skeleton className="w-full h-full rounded-full" />
        <div
          className="absolute bg-background rounded-full"
          style={{
            width: innerSize,
            height: innerSize,
            top: (size - innerSize) / 2,
            left: (size - innerSize) / 2,
          }}
        />
      </div>
    </div>
  );
}

/**
 * Account card skeleton matching Accounts view layout.
 */
export function AccountCardSkeleton() {
  return (
    <div className="bg-card rounded-lg border border-border p-4">
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-3">
          <Skeleton className="w-10 h-10 rounded-lg" />
          <div>
            <Skeleton className="h-5 w-24 mb-1" />
            <Skeleton className="h-4 w-12" />
          </div>
        </div>
      </div>
      <div className="space-y-2">
        <div className="flex justify-between">
          <Skeleton className="h-4 w-12" />
          <Skeleton className="h-4 w-20" />
        </div>
        <div className="flex justify-between">
          <Skeleton className="h-4 w-16" />
          <Skeleton className="h-4 w-8" />
        </div>
      </div>
    </div>
  );
}

/**
 * Holdings table row skeleton with logo placeholder.
 */
export function HoldingsRowSkeleton() {
  return (
    <tr className="border-b border-border">
      <td className="py-2 px-3">
        <div className="flex items-center gap-2">
          <Skeleton className="w-6 h-6 rounded" />
          <Skeleton className="h-4 w-32" />
        </div>
      </td>
      <td className="py-2 px-3 text-right">
        <Skeleton className="h-4 w-16 ml-auto" />
      </td>
      <td className="py-2 px-3 text-right">
        <Skeleton className="h-4 w-20 ml-auto" />
      </td>
      <td className="py-2 px-3 text-right">
        <Skeleton className="h-4 w-16 ml-auto" />
      </td>
      <td className="py-2 px-3 text-right">
        <Skeleton className="h-4 w-12 ml-auto" />
      </td>
      <td className="py-2 px-3">
        <MiniChartSkeleton />
      </td>
    </tr>
  );
}

/**
 * Dashboard skeleton combining summary cards and holdings table.
 */
export function DashboardSkeleton() {
  return (
    <div className="space-y-4">
      {/* Summary Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <SummaryCardSkeleton />
        <SummaryCardSkeleton />
        <SummaryCardSkeleton />
        <SummaryCardSkeleton />
      </div>

      {/* Holdings Table */}
      <div className="bg-card rounded-lg border border-border p-4">
        <Skeleton className="h-5 w-32 mb-4" />
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border text-xs text-muted-foreground">
              <th className="text-left py-2 px-3">
                <Skeleton className="h-3 w-20" />
              </th>
              <th className="text-right py-2 px-3">
                <Skeleton className="h-3 w-12 ml-auto" />
              </th>
              <th className="text-right py-2 px-3">
                <Skeleton className="h-3 w-16 ml-auto" />
              </th>
              <th className="text-right py-2 px-3">
                <Skeleton className="h-3 w-14 ml-auto" />
              </th>
              <th className="text-right py-2 px-3">
                <Skeleton className="h-3 w-10 ml-auto" />
              </th>
              <th className="py-2 px-3">
                <Skeleton className="h-3 w-12" />
              </th>
            </tr>
          </thead>
          <tbody>
            {Array.from({ length: 8 }).map((_, i) => (
              <HoldingsRowSkeleton key={i} />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
