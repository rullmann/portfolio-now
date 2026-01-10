/**
 * Image optimization utilities for AI API requests.
 *
 * Best practices for AI vision APIs:
 * - Claude: Max 1568x1568 recommended, supports PNG/JPEG/GIF/WebP
 * - OpenAI: Auto-scales, but smaller = faster + cheaper
 * - Gemini: Max 3072x3072, but smaller is better
 *
 * For chart analysis, 1200x800 is typically sufficient.
 */

export interface ImageOptimizationOptions {
  /** Maximum width in pixels (default: 1200) */
  maxWidth?: number;
  /** Maximum height in pixels (default: 800) */
  maxHeight?: number;
  /** JPEG quality 0-1 (default: 0.85) */
  quality?: number;
  /** Output format (default: 'jpeg') */
  format?: 'jpeg' | 'png' | 'webp';
}

const DEFAULT_OPTIONS: Required<ImageOptimizationOptions> = {
  maxWidth: 1200,
  maxHeight: 800,
  quality: 0.85,
  format: 'jpeg',
};

/**
 * Optimize a canvas for AI API submission.
 * - Resizes if larger than max dimensions (maintains aspect ratio)
 * - Converts to JPEG for smaller file size
 * - Returns base64 string (without data URL prefix)
 */
export function optimizeCanvasForAI(
  canvas: HTMLCanvasElement,
  options: ImageOptimizationOptions = {}
): { base64: string; width: number; height: number; originalSize: number; optimizedSize: number } {
  const opts = { ...DEFAULT_OPTIONS, ...options };

  const originalWidth = canvas.width;
  const originalHeight = canvas.height;

  // Calculate new dimensions maintaining aspect ratio
  let newWidth = originalWidth;
  let newHeight = originalHeight;

  if (originalWidth > opts.maxWidth || originalHeight > opts.maxHeight) {
    const widthRatio = opts.maxWidth / originalWidth;
    const heightRatio = opts.maxHeight / originalHeight;
    const ratio = Math.min(widthRatio, heightRatio);

    newWidth = Math.round(originalWidth * ratio);
    newHeight = Math.round(originalHeight * ratio);
  }

  // Create optimized canvas
  const optimizedCanvas = document.createElement('canvas');
  optimizedCanvas.width = newWidth;
  optimizedCanvas.height = newHeight;

  const ctx = optimizedCanvas.getContext('2d');
  if (!ctx) {
    throw new Error('Failed to get canvas context');
  }

  // Use high-quality image smoothing for downscaling
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';

  // Draw the original canvas scaled down
  ctx.drawImage(canvas, 0, 0, newWidth, newHeight);

  // Get original size estimate (PNG)
  const originalDataUrl = canvas.toDataURL('image/png');
  const originalSize = Math.round((originalDataUrl.length - 22) * 0.75); // Approximate bytes

  // Convert to optimized format
  const mimeType = `image/${opts.format}`;
  const optimizedDataUrl = optimizedCanvas.toDataURL(mimeType, opts.quality);
  const base64 = optimizedDataUrl.split(',')[1];
  const optimizedSize = Math.round((base64.length) * 0.75); // Approximate bytes

  return {
    base64,
    width: newWidth,
    height: newHeight,
    originalSize,
    optimizedSize,
  };
}

/**
 * Capture and optimize a chart container for AI analysis.
 * Combines all canvas layers and optimizes the result.
 */
export function captureAndOptimizeChart(
  container: HTMLElement,
  options: ImageOptimizationOptions = {}
): { base64: string; width: number; height: number; savings: string } {
  const canvases = container.querySelectorAll('canvas');
  if (canvases.length === 0) {
    throw new Error('No canvas elements found in container');
  }

  const containerRect = container.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;

  // Create combined canvas at device pixel ratio
  const combinedCanvas = document.createElement('canvas');
  combinedCanvas.width = containerRect.width * dpr;
  combinedCanvas.height = containerRect.height * dpr;

  const ctx = combinedCanvas.getContext('2d');
  if (!ctx) {
    throw new Error('Failed to get canvas context');
  }

  ctx.scale(dpr, dpr);

  // Fill background
  const isDark = document.documentElement.classList.contains('dark');
  ctx.fillStyle = isDark ? '#1f2937' : '#ffffff';
  ctx.fillRect(0, 0, containerRect.width, containerRect.height);

  // Draw each canvas layer
  canvases.forEach((canvas) => {
    const rect = canvas.getBoundingClientRect();
    const x = rect.left - containerRect.left;
    const y = rect.top - containerRect.top;

    ctx.drawImage(
      canvas,
      0, 0, canvas.width, canvas.height,
      x, y, rect.width, rect.height
    );
  });

  // Optimize the combined canvas
  const result = optimizeCanvasForAI(combinedCanvas, options);

  // Calculate savings percentage
  const savingsPercent = Math.round((1 - result.optimizedSize / result.originalSize) * 100);
  const savings = `${formatBytes(result.originalSize)} → ${formatBytes(result.optimizedSize)} (-${savingsPercent}%)`;

  return {
    base64: result.base64,
    width: result.width,
    height: result.height,
    savings,
  };
}

/**
 * Format bytes to human-readable string
 */
function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

/**
 * Debounce function to prevent rapid API calls
 */
export function createDebouncer<T extends (...args: unknown[]) => unknown>(
  fn: T,
  delay: number
): { call: (...args: Parameters<T>) => void; cancel: () => void } {
  let timeoutId: number | null = null;

  return {
    call: (...args: Parameters<T>) => {
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
      }
      timeoutId = window.setTimeout(() => {
        fn(...args);
        timeoutId = null;
      }, delay);
    },
    cancel: () => {
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
        timeoutId = null;
      }
    },
  };
}

/**
 * Rate limiter to prevent too frequent API calls
 */
export class RateLimiter {
  private lastCallTime = 0;
  private readonly minInterval: number;

  constructor(minIntervalMs: number = 3000) {
    this.minInterval = minIntervalMs;
  }

  canCall(): boolean {
    return Date.now() - this.lastCallTime >= this.minInterval;
  }

  markCalled(): void {
    this.lastCallTime = Date.now();
  }

  timeUntilNextCall(): number {
    const elapsed = Date.now() - this.lastCallTime;
    return Math.max(0, this.minInterval - elapsed);
  }
}
