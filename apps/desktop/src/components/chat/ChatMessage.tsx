/**
 * Chat Message Component - Displays individual chat messages.
 *
 * Supports user and assistant messages with markdown rendering.
 */

import { User, Bot, X } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import { cn } from '../../lib/utils';

export interface ChatMessageData {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

interface ChatMessageProps {
  message: ChatMessageData;
  onDelete?: (id: string) => void;
}

export function ChatMessage({ message, onDelete }: ChatMessageProps) {
  const isUser = message.role === 'user';

  return (
    <div
      className={cn(
        'flex gap-3 p-3 rounded-lg border group relative',
        isUser
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

      <div className="flex-1 min-w-0 pr-6">
        <div className="flex items-center gap-2 mb-1">
          <span className="font-medium text-sm">
            {isUser ? 'Du' : 'Portfolio-Assistent'}
          </span>
          <span className="text-xs text-muted-foreground">
            {formatTime(message.timestamp)}
          </span>
        </div>

        <div className="prose prose-sm dark:prose-invert max-w-none text-[13px] leading-relaxed prose-p:my-1 prose-ul:my-1 prose-li:my-0.5">
          {isUser ? (
            <p className="mb-0">{message.content}</p>
          ) : (
            <ReactMarkdown>{message.content}</ReactMarkdown>
          )}
        </div>
      </div>
    </div>
  );
}

function formatTime(date: Date): string {
  return new Intl.DateTimeFormat('de-DE', {
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}
