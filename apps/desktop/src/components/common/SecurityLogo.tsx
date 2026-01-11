/**
 * Reusable Security Logo component with caching support.
 * Uses the global logo cache to display security logos efficiently.
 */

import { Building2 } from 'lucide-react';
import type { CachedLogo } from '../../lib/hooks';

interface SecurityLogoProps {
  /** Security ID for logo lookup */
  securityId: number;
  /** Logo data from useCachedLogos hook */
  logos: Map<number, CachedLogo>;
  /** Size in pixels (default: 28) */
  size?: number;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Display a security logo with fallback icon.
 *
 * Usage:
 * ```tsx
 * const { logos } = useCachedLogos(securities, brandfetchApiKey);
 * <SecurityLogo securityId={123} logos={logos} size={32} />
 * ```
 */
export function SecurityLogo({ securityId, logos, size = 28, className = '' }: SecurityLogoProps) {
  const logoData = logos.get(securityId);

  if (logoData?.url) {
    return (
      <img
        src={logoData.url}
        alt=""
        className={`rounded-md object-contain bg-white flex-shrink-0 ${className}`}
        style={{ width: size, height: size }}
        crossOrigin="anonymous"
        onError={(e) => {
          e.currentTarget.style.display = 'none';
        }}
      />
    );
  }

  return (
    <div
      className={`rounded-md bg-muted flex items-center justify-center flex-shrink-0 ${className}`}
      style={{ width: size, height: size }}
    >
      <Building2 size={size * 0.5} className="text-muted-foreground" />
    </div>
  );
}
