/**
 * Image Attachment Preview - Shows attached images before sending.
 *
 * Displays thumbnails of attached images with remove buttons.
 * Images can be removed individually before sending.
 */

import { X, Image as ImageIcon } from 'lucide-react';
import { cn } from '../../lib/utils';

export interface ChatImageAttachment {
  data: string;       // Base64-encoded image data
  mimeType: string;   // image/png, image/jpeg, etc.
  filename?: string;  // Optional filename for display
}

interface ImageAttachmentPreviewProps {
  attachments: ChatImageAttachment[];
  onRemove: (index: number) => void;
  className?: string;
  disabled?: boolean;
}

export function ImageAttachmentPreview({
  attachments,
  onRemove,
  className,
  disabled = false,
}: ImageAttachmentPreviewProps) {
  if (attachments.length === 0) {
    return null;
  }

  return (
    <div className={cn('flex flex-wrap gap-2 p-2 border-b border-border bg-muted/30', className)}>
      {attachments.map((attachment, index) => (
        <div
          key={index}
          className="relative group"
        >
          <div className="w-16 h-16 rounded-md overflow-hidden border border-border bg-background">
            {attachment.data ? (
              <img
                src={`data:${attachment.mimeType};base64,${attachment.data}`}
                alt={attachment.filename || `Bild ${index + 1}`}
                className="w-full h-full object-cover"
              />
            ) : (
              <div className="w-full h-full flex items-center justify-center bg-muted">
                <ImageIcon className="h-6 w-6 text-muted-foreground" />
              </div>
            )}
          </div>

          {/* Remove button */}
          <button
            type="button"
            onClick={() => onRemove(index)}
            disabled={disabled}
            className={cn(
              'absolute -top-1.5 -right-1.5 p-0.5 rounded-full',
              'bg-destructive text-destructive-foreground',
              'opacity-0 group-hover:opacity-100 transition-opacity',
              'hover:bg-destructive/90 focus:opacity-100',
              'disabled:opacity-50 disabled:cursor-not-allowed'
            )}
            title="Bild entfernen"
          >
            <X className="h-3 w-3" />
          </button>

          {/* Filename tooltip */}
          {attachment.filename && (
            <div className="absolute bottom-0 left-0 right-0 px-1 py-0.5 bg-black/60 text-white text-[10px] truncate rounded-b-md">
              {attachment.filename}
            </div>
          )}
        </div>
      ))}

      {/* Info text */}
      <div className="flex items-center text-xs text-muted-foreground ml-2">
        {attachments.length === 1
          ? '1 Bild angehängt'
          : `${attachments.length} Bilder angehängt`}
      </div>
    </div>
  );
}

export default ImageAttachmentPreview;
