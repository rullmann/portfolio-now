/**
 * Mini chat component for the quote assistant.
 * Specialized for quote source discussions only.
 */

import { useState, useRef, useEffect } from 'react';
import { Send, Loader2, Bot, User } from 'lucide-react';
import { SafeMarkdown } from '../common/SafeMarkdown';
import type { ProblematicSecurity, ValidatedQuoteSuggestion } from '../../lib/types';
import { ValidatedSuggestionCard } from './ValidatedSuggestionCard';

interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  suggestion?: ValidatedQuoteSuggestion;
}

interface QuoteChatProps {
  security: ProblematicSecurity | null;
  messages: ChatMessage[];
  onSendMessage: (message: string) => void;
  onApplySuggestion: (suggestion: ValidatedQuoteSuggestion) => void;
  isLoading?: boolean;
  isApplying?: boolean;
}

export function QuoteChat({
  security,
  messages,
  onSendMessage,
  onApplySuggestion,
  isLoading,
  isApplying,
}: QuoteChatProps) {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Scroll to bottom when messages change
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Focus input when security changes
  useEffect(() => {
    if (security) {
      inputRef.current?.focus();
    }
  }, [security?.id]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (input.trim() && !isLoading) {
      onSendMessage(input.trim());
      setInput('');
    }
  };

  if (!security) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-4">
        <Bot className="h-12 w-12 mb-4 opacity-30" />
        <p className="text-center">
          Wähle ein Wertpapier aus der Liste, um die KI-Analyse zu starten.
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Chat Header */}
      <div className="px-4 py-3 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <Bot className="h-5 w-5 text-primary" />
          <span className="font-medium">Kursquellen-Experte</span>
        </div>
        <div className="text-sm text-muted-foreground mt-1">
          Analysiere: {security.name}
        </div>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {messages.map((msg, idx) => (
          <div key={idx}>
            {msg.role === 'system' ? (
              <div className="text-center text-sm text-muted-foreground py-2">
                {msg.content}
              </div>
            ) : (
              <div
                className={`flex gap-3 ${
                  msg.role === 'user' ? 'flex-row-reverse' : 'flex-row'
                }`}
              >
                <div
                  className={`w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 ${
                    msg.role === 'user'
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-muted'
                  }`}
                >
                  {msg.role === 'user' ? (
                    <User className="h-4 w-4" />
                  ) : (
                    <Bot className="h-4 w-4" />
                  )}
                </div>
                <div
                  className={`flex-1 max-w-[85%] ${
                    msg.role === 'user' ? 'text-right' : 'text-left'
                  }`}
                >
                  <div
                    className={`inline-block rounded-lg px-4 py-2 ${
                      msg.role === 'user'
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted'
                    }`}
                  >
                    <div className="prose prose-sm dark:prose-invert max-w-none">
                      <SafeMarkdown>{msg.content}</SafeMarkdown>
                    </div>
                  </div>

                  {/* Suggestion Card */}
                  {msg.suggestion && (
                    <div className="mt-3">
                      <ValidatedSuggestionCard
                        suggestion={msg.suggestion}
                        onApply={() => onApplySuggestion(msg.suggestion!)}
                        isApplying={isApplying}
                      />
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        ))}

        {/* Loading indicator */}
        {isLoading && (
          <div className="flex gap-3">
            <div className="w-8 h-8 rounded-full bg-muted flex items-center justify-center">
              <Loader2 className="h-4 w-4 animate-spin" />
            </div>
            <div className="bg-muted rounded-lg px-4 py-2">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="h-3 w-3 animate-spin" />
                Analysiere Kursquelle...
              </div>
            </div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <form onSubmit={handleSubmit} className="p-4 border-t">
        <div className="flex gap-2">
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Frage eingeben..."
            disabled={isLoading}
            className="flex-1 px-3 py-2 border rounded-lg bg-background
                       focus:outline-none focus:ring-2 focus:ring-primary/50
                       disabled:opacity-50 disabled:cursor-not-allowed"
          />
          <button
            type="submit"
            disabled={!input.trim() || isLoading}
            className="px-4 py-2 bg-primary text-primary-foreground rounded-lg
                       hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed
                       transition-colors"
          >
            <Send className="h-4 w-4" />
          </button>
        </div>
      </form>
    </div>
  );
}
