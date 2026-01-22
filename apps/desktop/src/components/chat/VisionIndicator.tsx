/**
 * Vision Indicator - Shows whether the current model supports image input.
 *
 * Displays a camera icon with a color indicator:
 * - Green: Model supports images (Vision-capable)
 * - Gray: Model does not support images
 */

import { useState, useEffect } from 'react';
import { Camera, CameraOff } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { cn } from '../../lib/utils';

interface VisionIndicatorProps {
  model: string;
  className?: string;
}

export function VisionIndicator({ model, className }: VisionIndicatorProps) {
  const [hasVision, setHasVision] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const checkVision = async () => {
      setIsLoading(true);
      try {
        const result = await invoke<boolean>('check_vision_support', { model });
        setHasVision(result);
      } catch (error) {
        console.error('Failed to check vision support:', error);
        setHasVision(false);
      } finally {
        setIsLoading(false);
      }
    };

    if (model) {
      checkVision();
    } else {
      setHasVision(false);
      setIsLoading(false);
    }
  }, [model]);

  if (isLoading) {
    return (
      <div className={cn('flex items-center gap-1 text-xs text-muted-foreground', className)}>
        <Camera size={12} className="animate-pulse" />
        <span>...</span>
      </div>
    );
  }

  return (
    <div
      className={cn(
        'flex items-center gap-1 text-xs',
        hasVision ? 'text-green-600' : 'text-muted-foreground',
        className
      )}
      title={hasVision ? 'Modell unterstützt Bilder' : 'Modell unterstützt keine Bilder'}
    >
      {hasVision ? (
        <>
          <Camera size={12} />
          <span>Bilder möglich</span>
        </>
      ) : (
        <>
          <CameraOff size={12} />
          <span>Keine Bilder</span>
        </>
      )}
    </div>
  );
}

export default VisionIndicator;
