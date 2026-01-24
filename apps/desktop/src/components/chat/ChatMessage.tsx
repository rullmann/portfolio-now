/**
 * Chat Message Component - Displays individual chat messages.
 *
 * Supports user and assistant messages with markdown rendering.
 * Shows image attachments if present.
 */

import { useState } from 'react';
import { User, Bot, X, ImageIcon } from 'lucide-react';
import { SafeMarkdown } from '../common/SafeMarkdown';
import { cn } from '../../lib/utils';
import { useSettingsStore } from '../../store';

export interface StoredChatAttachment {
  data: string;      // Base64 encoded image data
  mimeType: string;  // e.g., "image/png", "image/jpeg"
  filename?: string;
}

export interface ChatMessageData {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  attachments?: StoredChatAttachment[];
  isDuplicate?: boolean; // For duplicate transaction messages (shown with amber border)
  isError?: boolean; // For error messages (shown with red border)
}

interface ChatMessageProps {
  message: ChatMessageData;
  onDelete?: (id: string) => void;
}

export function ChatMessage({ message, onDelete }: ChatMessageProps) {
  const isUser = message.role === 'user';
  const [expandedImage, setExpandedImage] = useState<string | null>(null);
  const hasAttachments = message.attachments && message.attachments.length > 0;
  const isDuplicate = message.isDuplicate;
  const isError = message.isError;
  const { profilePicture } = useSettingsStore();

  return (
    <div
      className={cn(
        'flex gap-3 p-3 rounded-lg border group relative',
        isError
          ? 'bg-red-500/5 border-red-500/50 border-2' // Red border for errors
          : isDuplicate
            ? 'bg-amber-500/5 border-amber-500/50 border-2' // Amber border for duplicates
            : isUser
              ? 'bg-blue-500/5 border-blue-500/20'
              : 'bg-orange-500/5 border-orange-500/20'
      )}
    >
      {/* Delete button */}
      {onDelete && (
        <button
          onClick={() => onDelete(message.id)}
          className="absolute top-2 right-2 p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-black/10 dark:hover:bg-white/10 transition-opacity"
          title="Nachricht löschen"
        >
          <X className="h-3 w-3 text-muted-foreground" />
        </button>
      )}

      {/* Avatar */}
      {isUser && profilePicture ? (
        <img
          src={profilePicture}
          alt="Du"
          className="flex-shrink-0 w-8 h-8 rounded-lg object-cover"
        />
      ) : (
        <div
          className={cn(
            'flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center',
            isUser
              ? 'bg-blue-500/10 text-blue-600 dark:text-blue-400'
              : 'bg-orange-500/10 text-orange-600 dark:text-orange-400'
          )}
        >
          {isUser ? <User className="h-4 w-4" /> : <Bot className="h-4 w-4" />}
        </div>
      )}

      <div className="flex-1 min-w-0 pr-6">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-medium text-sm">
            {isUser ? 'Du' : 'Portfolio-Assistent'}
          </span>
          <span className="text-xs text-muted-foreground">
            {formatTime(message.timestamp)}
          </span>
          {hasAttachments && (
            <span className="flex items-center gap-1 text-xs text-muted-foreground">
              <ImageIcon className="h-3 w-3" />
              {message.attachments!.length}
            </span>
          )}
        </div>

        {/* Image attachments */}
        {hasAttachments && (
          <div className="flex flex-wrap gap-2 mb-2">
            {message.attachments!.map((attachment, idx) => (
              <button
                key={idx}
                onClick={() => setExpandedImage(`data:${attachment.mimeType};base64,${attachment.data}`)}
                className="relative group/thumb"
              >
                <img
                  src={`data:${attachment.mimeType};base64,${attachment.data}`}
                  alt={attachment.filename || `Bild ${idx + 1}`}
                  className="h-16 w-16 object-cover rounded-lg border border-border hover:border-primary transition-colors"
                />
              </button>
            ))}
          </div>
        )}

        <div className="prose prose-sm dark:prose-invert max-w-none text-[13px] leading-relaxed prose-p:my-1 prose-ul:my-1 prose-li:my-0.5 [&_table]:text-xs [&_table]:w-full [&_table]:border-collapse [&_table]:border [&_table]:border-border/30 [&_table]:rounded-md [&_table]:overflow-hidden [&_thead]:bg-muted/60 [&_th]:px-3 [&_th]:py-2 [&_th]:text-left [&_th]:font-semibold [&_th]:border-b [&_th]:border-border/50 [&_td]:px-3 [&_td]:py-2 [&_td]:border-b [&_td]:border-border/20 [&_tr:last-child_td]:border-b-0 [&_tbody_tr:hover]:bg-muted/30 [&_tbody_tr:nth-child(even)]:bg-muted/10 overflow-x-auto">
          {isUser ? (
            <p className="mb-0">{message.content}</p>
          ) : (
            <SafeMarkdown>{message.content}</SafeMarkdown>
          )}
        </div>
      </div>

      {/* Expanded image modal */}
      {expandedImage && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80"
          onClick={() => setExpandedImage(null)}
        >
          <button
            className="absolute top-4 right-4 p-2 rounded-full bg-white/10 hover:bg-white/20 transition-colors"
            onClick={() => setExpandedImage(null)}
          >
            <X className="h-6 w-6 text-white" />
          </button>
          <img
            src={expandedImage}
            alt="Vergrößertes Bild"
            className="max-w-[90vw] max-h-[90vh] object-contain rounded-lg"
            onClick={(e) => e.stopPropagation()}
          />
        </div>
      )}
    </div>
  );
}

function formatTime(date: Date): string {
  return new Intl.DateTimeFormat('de-DE', {
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}
