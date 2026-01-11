/**
 * Chat Button - Floating button to open the chat panel.
 *
 * Fixed position in the bottom right corner.
 */

import { MessageSquare } from 'lucide-react';
import { cn } from '../../lib/utils';

interface ChatButtonProps {
  onClick: () => void;
  hasMessages?: boolean;
}

export function ChatButton({ onClick, hasMessages }: ChatButtonProps) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'fixed bottom-6 right-6 z-30',
        'w-14 h-14 rounded-full',
        'bg-primary text-primary-foreground',
        'shadow-lg hover:shadow-xl',
        'flex items-center justify-center',
        'transition-all duration-200',
        'hover:scale-105 active:scale-95'
      )}
      title="Portfolio-Assistent"
    >
      <MessageSquare className="h-6 w-6" />
      {hasMessages && (
        <span className="absolute top-0 right-0 w-3 h-3 bg-green-500 rounded-full border-2 border-background" />
      )}
    </button>
  );
}
