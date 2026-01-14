/**
 * Chat Panel - Slide-in chat interface for portfolio questions.
 *
 * Provides a conversational interface to ask questions about the portfolio.
 * The AI is restricted to finance/portfolio topics only.
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import { X, Send, Loader2, Trash2, MessageSquare, GripVertical, CheckCircle, XCircle, AlertTriangle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '../../store';
import { AIProviderLogo } from '../common/AIProviderLogo';
import { ChatMessage, type ChatMessageData } from './ChatMessage';
import { cn } from '../../lib/utils';

const MIN_WIDTH = 320;
const MAX_WIDTH = 800;
const DEFAULT_WIDTH = 420;
const STORAGE_KEY_WIDTH = 'portfolio-chat-width';

interface ChatPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

interface SuggestedAction {
  actionType: string;
  description: string;
  payload: string;
}

interface PortfolioChatResponse {
  response: string;
  provider: string;
  model: string;
  tokensUsed: number | null;
  suggestions?: SuggestedAction[];
}

const EXAMPLE_QUESTIONS = [
  'Wie war meine Rendite dieses Jahr?',
  'Welche Aktien zahlen Dividende?',
  'Zeige meine Top-Performer',
  'Wie ist mein Portfolio diversifiziert?',
];

const STORAGE_KEY = 'portfolio-chat-history';

export function ChatPanel({ isOpen, onClose }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessageData[]>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        return parsed.map((m: ChatMessageData) => ({
          ...m,
          timestamp: new Date(m.timestamp),
        }));
      }
    } catch {
      // Ignore parse errors
    }
    return [];
  });
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingSuggestions, setPendingSuggestions] = useState<SuggestedAction[]>([]);
  const [executingSuggestion, setExecutingSuggestion] = useState<string | null>(null);
  const [panelWidth, setPanelWidth] = useState(() => {
    const saved = localStorage.getItem(STORAGE_KEY_WIDTH);
    return saved ? Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, parseInt(saved, 10))) : DEFAULT_WIDTH;
  });
  const [isResizing, setIsResizing] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const {
    aiProvider,
    aiModel,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
    baseCurrency,
    alphaVantageApiKey,
    userName,
  } = useSettingsStore();

  const getApiKey = () => {
    switch (aiProvider) {
      case 'claude':
        return anthropicApiKey;
      case 'openai':
        return openaiApiKey;
      case 'gemini':
        return geminiApiKey;
      case 'perplexity':
        return perplexityApiKey;
      default:
        return '';
    }
  };

  const hasApiKey = () => {
    const key = getApiKey();
    return key && key.trim().length > 0;
  };

  // Save messages to localStorage
  useEffect(() => {
    if (messages.length > 0) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(messages));
    }
  }, [messages]);

  // Scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Focus input when panel opens
  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [isOpen]);

  // Save width to localStorage
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_WIDTH, String(panelWidth));
  }, [panelWidth]);

  // Handle resize
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = window.innerWidth - e.clientX;
      setPanelWidth(Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth)));
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizing]);

  const sendMessage = async (content: string) => {
    if (!content.trim() || isLoading) return;

    const userMessage: ChatMessageData = {
      id: crypto.randomUUID(),
      role: 'user',
      content: content.trim(),
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);
    setError(null);

    try {
      // Build message history for API
      const apiMessages = [...messages, userMessage].map((m) => ({
        role: m.role,
        content: m.content,
      }));

      const response = await invoke<PortfolioChatResponse>('chat_with_portfolio_assistant', {
        request: {
          messages: apiMessages,
          provider: aiProvider,
          model: aiModel,
          apiKey: getApiKey(),
          baseCurrency: baseCurrency || 'EUR',
          userName: userName || null,
        },
      });

      const assistantMessage: ChatMessageData = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: response.response,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);

      // Store any suggestions that need user confirmation
      if (response.suggestions && response.suggestions.length > 0) {
        setPendingSuggestions(response.suggestions);
      }
    } catch (err) {
      const errorMessage = typeof err === 'string' ? err : String(err);

      // Try to parse structured error
      try {
        const parsed = JSON.parse(errorMessage);
        setError(parsed.message || errorMessage);
      } catch {
        setError(errorMessage);
      }
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage(input);
    }
  };

  const clearHistory = () => {
    setMessages([]);
    localStorage.removeItem(STORAGE_KEY);
  };

  const deleteMessage = (id: string) => {
    setMessages((prev) => prev.filter((m) => m.id !== id));
  };

  // Execute a confirmed suggestion
  const executeSuggestion = async (suggestion: SuggestedAction) => {
    setExecutingSuggestion(suggestion.payload);
    try {
      const result = await invoke<string>('execute_confirmed_ai_action', {
        actionType: suggestion.actionType,
        payload: suggestion.payload,
        alphaVantageApiKey: alphaVantageApiKey || null,
      });

      // Add success message
      const successMessage: ChatMessageData = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: `✓ ${result}`,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, successMessage]);

      // Remove executed suggestion
      setPendingSuggestions((prev) => prev.filter((s) => s.payload !== suggestion.payload));
    } catch (err) {
      setError(typeof err === 'string' ? err : String(err));
    } finally {
      setExecutingSuggestion(null);
    }
  };

  // Decline a suggestion
  const declineSuggestion = (suggestion: SuggestedAction) => {
    setPendingSuggestions((prev) => prev.filter((s) => s.payload !== suggestion.payload));
  };

  // Decline all suggestions
  const declineAllSuggestions = () => {
    setPendingSuggestions([]);
  };

  if (!isOpen) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-40 bg-black/20 backdrop-blur-sm md:bg-transparent md:backdrop-blur-none"
        onClick={onClose}
      />

      {/* Panel */}
      <div
        ref={panelRef}
        style={{ width: panelWidth }}
        className={cn(
          'fixed right-0 top-0 z-50 h-full',
          'bg-background border-l border-border shadow-xl',
          'flex flex-col',
          'animate-in slide-in-from-right duration-300',
          isResizing && 'select-none'
        )}
      >
        {/* Resize Handle */}
        <div
          onMouseDown={handleMouseDown}
          className={cn(
            'absolute left-0 top-0 bottom-0 w-1 cursor-ew-resize',
            'hover:bg-primary/30 active:bg-primary/50 transition-colors',
            'group flex items-center justify-center',
            isResizing && 'bg-primary/50'
          )}
        >
          <div className="absolute left-0 w-4 h-full" /> {/* Larger hit area */}
          <GripVertical className="h-6 w-3 text-muted-foreground/50 group-hover:text-primary/70 absolute -left-1" />
        </div>
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2">
            <MessageSquare className="h-5 w-5 text-primary" />
            <h2 className="font-semibold">Portfolio-Assistent</h2>
          </div>
          <div className="flex items-center gap-1">
            {messages.length > 0 && (
              <button
                onClick={clearHistory}
                className="p-2 rounded hover:bg-muted transition-colors"
                title="Verlauf löschen"
              >
                <Trash2 className="h-4 w-4" />
              </button>
            )}
            <button
              onClick={onClose}
              className="p-2 rounded hover:bg-muted transition-colors"
            >
              <X className="h-5 w-5" />
            </button>
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {messages.length === 0 ? (
            <div className="text-center py-8">
              <MessageSquare className="h-12 w-12 mx-auto mb-4 text-muted-foreground/50" />
              <p className="text-muted-foreground mb-4">
                Stelle Fragen zu deinem Portfolio
              </p>
              <div className="space-y-2">
                {EXAMPLE_QUESTIONS.map((question) => (
                  <button
                    key={question}
                    onClick={() => sendMessage(question)}
                    disabled={isLoading || !hasApiKey()}
                    className="block w-full text-left px-3 py-2 text-sm rounded-lg bg-muted/50 hover:bg-muted transition-colors disabled:opacity-50"
                  >
                    {question}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            messages.map((message) => (
              <ChatMessage key={message.id} message={message} onDelete={deleteMessage} />
            ))
          )}

          {isLoading && (
            <div className="flex items-center gap-2 p-3 rounded-lg bg-muted/50">
              <Loader2 className="h-4 w-4 animate-spin text-primary" />
              <span className="text-sm text-muted-foreground">Denke nach...</span>
            </div>
          )}

          {error && (
            <div className="p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-sm text-destructive">
              {error}
            </div>
          )}

          {/* Pending Suggestions - Require user confirmation */}
          {pendingSuggestions.length > 0 && (
            <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 space-y-3">
              <div className="flex items-center gap-2 text-amber-600">
                <AlertTriangle className="h-4 w-4" />
                <span className="text-sm font-medium">
                  {pendingSuggestions.length === 1 ? 'Aktion erfordert Bestätigung' : `${pendingSuggestions.length} Aktionen erfordern Bestätigung`}
                </span>
                {pendingSuggestions.length > 1 && (
                  <button
                    onClick={declineAllSuggestions}
                    className="ml-auto text-xs text-muted-foreground hover:text-foreground"
                  >
                    Alle ablehnen
                  </button>
                )}
              </div>
              {pendingSuggestions.map((suggestion, idx) => (
                <div key={idx} className="flex items-center gap-2 bg-background/50 rounded-md p-2">
                  <span className="flex-1 text-sm">{suggestion.description}</span>
                  <button
                    onClick={() => executeSuggestion(suggestion)}
                    disabled={executingSuggestion !== null}
                    className="p-1.5 rounded bg-green-500/20 text-green-600 hover:bg-green-500/30 disabled:opacity-50"
                    title="Bestätigen"
                  >
                    {executingSuggestion === suggestion.payload ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <CheckCircle className="h-4 w-4" />
                    )}
                  </button>
                  <button
                    onClick={() => declineSuggestion(suggestion)}
                    disabled={executingSuggestion !== null}
                    className="p-1.5 rounded bg-red-500/20 text-red-600 hover:bg-red-500/30 disabled:opacity-50"
                    title="Ablehnen"
                  >
                    <XCircle className="h-4 w-4" />
                  </button>
                </div>
              ))}
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        <div className="p-4 border-t border-border">
          {!hasApiKey() ? (
            <div className="text-center text-sm text-muted-foreground p-2">
              Bitte konfiguriere deinen {aiProvider.toUpperCase()} API-Key in den Einstellungen.
            </div>
          ) : (
            <div className="flex gap-2">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Nachricht eingeben..."
                rows={1}
                className="flex-1 resize-none rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
                disabled={isLoading}
              />
              <button
                onClick={() => sendMessage(input)}
                disabled={!input.trim() || isLoading}
                className="p-2 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                <Send className="h-5 w-5" />
              </button>
            </div>
          )}

          {/* Provider info */}
          <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
            <AIProviderLogo provider={aiProvider} size={14} />
            <span>{aiModel}</span>
          </div>
        </div>
      </div>
    </>
  );
}
