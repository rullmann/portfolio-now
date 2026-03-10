/**
 * Reusable Security Logo component with caching support.
 * Uses the global logo cache to display security logos efficiently.
 */

import { useState } from 'react';
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
  /** Custom logo URL (from DB), takes priority over cached logos */
  customLogoUrl?: string | null;
}

/**
 * Display a security logo with fallback icon.
 * Uses the global logo cache (useCachedLogos) for logo lookup.
 */
export function SecurityLogo({ securityId, logos, size = 28, className = '', customLogoUrl }: SecurityLogoProps) {
  // Track which URL triggered the error so reset happens automatically when URL changes
  const [errorUrl, setErrorUrl] = useState<string | undefined>(undefined);
  const logoData = logos.get(securityId);
  const effectiveUrl = customLogoUrl || logoData?.url;
  const hasError = errorUrl !== undefined && errorUrl === effectiveUrl;

  if (effectiveUrl && !hasError) {
    return (
      <img
        src={effectiveUrl}
        alt=""
        className={`rounded-md object-contain bg-white flex-shrink-0 ${className}`}
        style={{ width: size, height: size }}
        crossOrigin="anonymous"
        onError={() => setErrorUrl(effectiveUrl)}
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

/**
 * Simple logo image with error fallback.
 * For views that have a direct URL string instead of the cachedLogos Map.
 */
export function LogoImage({ src, size = 28, className = '' }: {
  src?: string | null;
  size?: number;
  className?: string;
}) {
  // Track which URL triggered the error so reset happens automatically when URL changes
  const [errorUrl, setErrorUrl] = useState<string | undefined>(undefined);
  const hasError = errorUrl !== undefined && errorUrl === src;

  if (src && !hasError) {
    return (
      <img
        src={src}
        alt=""
        className={`rounded-md object-contain bg-white flex-shrink-0 ${className}`}
        style={{ width: size, height: size }}
        crossOrigin="anonymous"
        onError={() => setErrorUrl(src)}
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
