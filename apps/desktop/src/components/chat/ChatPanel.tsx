/**
 * Chat Panel - Slide-in chat interface for portfolio questions.
 *
 * Provides a conversational interface to ask questions about the portfolio.
 * The AI is restricted to finance/portfolio topics only.
 *
 * Chat history is persisted in SQLite and uses a sliding window
 * (chatContextSize setting) to limit tokens sent to the AI.
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import { X, Send, Loader2, Trash2, MessageSquare, GripVertical, CheckCircle, XCircle, AlertTriangle, Receipt } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore } from '../../store';
import { AIProviderLogo } from '../common/AIProviderLogo';
import { ChatMessage, type ChatMessageData } from './ChatMessage';
import { cn } from '../../lib/utils';
import type { ChatHistoryMessage, TransactionCreateCommand, PortfolioTransferCommand } from '../../lib/types';
import { formatSharesFromScaled, formatAmountFromScaled, getTransactionTypeLabel, formatDate } from '../../lib/types';

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

// ============================================================================
// Transaction Confirmation Component
// ============================================================================

interface TransactionConfirmationProps {
  suggestion: SuggestedAction;
  onConfirm: () => void;
  onDecline: () => void;
  isExecuting: boolean;
}

function TransactionConfirmation({ suggestion, onConfirm, onDecline, isExecuting }: TransactionConfirmationProps) {
  const isTransaction = suggestion.actionType === 'transaction_create';
  const isTransfer = suggestion.actionType === 'portfolio_transfer';

  if (!isTransaction && !isTransfer) {
    return null;
  }

  // Parse the payload
  let preview: TransactionCreateCommand | PortfolioTransferCommand | null = null;
  try {
    preview = JSON.parse(suggestion.payload);
  } catch {
    return null;
  }

  if (!preview) return null;

  // Render transaction preview
  if (isTransaction) {
    const txn = preview as TransactionCreateCommand;
    return (
      <div className="p-4 rounded-lg border-2 border-primary/50 bg-primary/5">
        <div className="flex items-center gap-2 mb-3">
          <Receipt className="h-5 w-5 text-primary" />
          <span className="font-semibold">Transaktion bestätigen</span>
        </div>

        <table className="w-full text-sm mb-4">
          <tbody className="divide-y divide-border">
            <tr>
              <td className="py-1.5 text-muted-foreground">Typ</td>
              <td className="py-1.5 font-medium">{getTransactionTypeLabel(txn.type)}</td>
            </tr>
            {txn.securityName && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Wertpapier</td>
                <td className="py-1.5">{txn.securityName}</td>
              </tr>
            )}
            {txn.portfolioId && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Depot</td>
                <td className="py-1.5">ID: {txn.portfolioId}</td>
              </tr>
            )}
            {txn.accountId && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Konto</td>
                <td className="py-1.5">ID: {txn.accountId}</td>
              </tr>
            )}
            {txn.shares !== undefined && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Stückzahl</td>
                <td className="py-1.5">{formatSharesFromScaled(txn.shares)}</td>
              </tr>
            )}
            {txn.amount !== undefined && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Betrag</td>
                <td className="py-1.5">{formatAmountFromScaled(txn.amount, txn.currency)}</td>
              </tr>
            )}
            <tr>
              <td className="py-1.5 text-muted-foreground">Datum</td>
              <td className="py-1.5">{formatDate(txn.date)}</td>
            </tr>
            {txn.fees !== undefined && txn.fees > 0 && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Gebühren</td>
                <td className="py-1.5">{formatAmountFromScaled(txn.fees, txn.currency)}</td>
              </tr>
            )}
            {txn.taxes !== undefined && txn.taxes > 0 && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Steuern</td>
                <td className="py-1.5">{formatAmountFromScaled(txn.taxes, txn.currency)}</td>
              </tr>
            )}
            {txn.note && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Notiz</td>
                <td className="py-1.5 text-xs">{txn.note}</td>
              </tr>
            )}
          </tbody>
        </table>

        <div className="flex gap-2">
          <button
            onClick={onConfirm}
            disabled={isExecuting}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
          >
            {isExecuting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <CheckCircle className="h-4 w-4" />
            )}
            Bestätigen
          </button>
          <button
            onClick={onDecline}
            disabled={isExecuting}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 transition-colors"
          >
            <XCircle className="h-4 w-4" />
            Abbrechen
          </button>
        </div>
      </div>
    );
  }

  // Render transfer preview
  if (isTransfer) {
    const transfer = preview as PortfolioTransferCommand;
    return (
      <div className="p-4 rounded-lg border-2 border-primary/50 bg-primary/5">
        <div className="flex items-center gap-2 mb-3">
          <Receipt className="h-5 w-5 text-primary" />
          <span className="font-semibold">Depotwechsel bestätigen</span>
        </div>

        <table className="w-full text-sm mb-4">
          <tbody className="divide-y divide-border">
            <tr>
              <td className="py-1.5 text-muted-foreground">Wertpapier</td>
              <td className="py-1.5">ID: {transfer.securityId}</td>
            </tr>
            <tr>
              <td className="py-1.5 text-muted-foreground">Stückzahl</td>
              <td className="py-1.5">{formatSharesFromScaled(transfer.shares)}</td>
            </tr>
            <tr>
              <td className="py-1.5 text-muted-foreground">Von Depot</td>
              <td className="py-1.5">ID: {transfer.fromPortfolioId}</td>
            </tr>
            <tr>
              <td className="py-1.5 text-muted-foreground">Nach Depot</td>
              <td className="py-1.5">ID: {transfer.toPortfolioId}</td>
            </tr>
            <tr>
              <td className="py-1.5 text-muted-foreground">Datum</td>
              <td className="py-1.5">{formatDate(transfer.date)}</td>
            </tr>
            {transfer.note && (
              <tr>
                <td className="py-1.5 text-muted-foreground">Notiz</td>
                <td className="py-1.5 text-xs">{transfer.note}</td>
              </tr>
            )}
          </tbody>
        </table>

        <div className="flex gap-2">
          <button
            onClick={onConfirm}
            disabled={isExecuting}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
          >
            {isExecuting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <CheckCircle className="h-4 w-4" />
            )}
            Bestätigen
          </button>
          <button
            onClick={onDecline}
            disabled={isExecuting}
            className="flex-1 flex items-center justify-center gap-2 px-3 py-2 rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 transition-colors"
          >
            <XCircle className="h-4 w-4" />
            Abbrechen
          </button>
        </div>
      </div>
    );
  }

  return null;
}

export function ChatPanel({ isOpen, onClose }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessageData[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState(true);
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
    aiFeatureSettings,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
    baseCurrency,
    alphaVantageApiKey,
    userName,
    chatContextSize,
  } = useSettingsStore();

  // Get feature-specific provider and model for Chat Assistant
  const { provider: aiProvider, model: aiModel } = aiFeatureSettings.chatAssistant;

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

  // Load chat history from database on mount
  useEffect(() => {
    const loadHistory = async () => {
      try {
        setIsLoadingHistory(true);
        const history = await invoke<ChatHistoryMessage[]>('get_chat_history', { limit: null });
        const loaded: ChatMessageData[] = history.map((m) => ({
          id: String(m.id),
          role: m.role as 'user' | 'assistant',
          content: m.content,
          timestamp: new Date(m.createdAt),
        }));
        setMessages(loaded);
      } catch (err) {
        console.error('Failed to load chat history:', err);
      } finally {
        setIsLoadingHistory(false);
      }
    };
    loadHistory();
  }, []);

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

    const trimmedContent = content.trim();
    setInput('');
    setIsLoading(true);
    setError(null);

    try {
      // Save user message to database first
      const userMsgId = await invoke<number>('save_chat_message', {
        role: 'user',
        content: trimmedContent,
      });

      const userMessage: ChatMessageData = {
        id: String(userMsgId),
        role: 'user',
        content: trimmedContent,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, userMessage]);

      // Build message history for API with sliding window
      const allMessages = [...messages, userMessage];
      const contextMessages = allMessages.slice(-chatContextSize);
      const apiMessages = contextMessages.map((m) => ({
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

      // Save assistant response to database
      const assistantMsgId = await invoke<number>('save_chat_message', {
        role: 'assistant',
        content: response.response,
      });

      const assistantMessage: ChatMessageData = {
        id: String(assistantMsgId),
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

  const clearHistory = async () => {
    try {
      await invoke('clear_chat_history');
      setMessages([]);
    } catch (err) {
      console.error('Failed to clear chat history:', err);
      setError('Fehler beim Löschen des Verlaufs');
    }
  };

  const deleteMessage = async (id: string) => {
    try {
      await invoke('delete_chat_message', { id: parseInt(id, 10) });
      setMessages((prev) => prev.filter((m) => m.id !== id));
    } catch (err) {
      console.error('Failed to delete chat message:', err);
      setError('Fehler beim Löschen der Nachricht');
    }
  };

  // Execute a confirmed suggestion
  const executeSuggestion = async (suggestion: SuggestedAction) => {
    setExecutingSuggestion(suggestion.payload);
    try {
      let result: string;

      // Handle different action types
      if (suggestion.actionType === 'transaction_create') {
        result = await invoke<string>('execute_confirmed_transaction', {
          payload: suggestion.payload,
        });
      } else if (suggestion.actionType === 'portfolio_transfer') {
        result = await invoke<string>('execute_confirmed_portfolio_transfer', {
          payload: suggestion.payload,
        });
      } else {
        // Default: watchlist actions
        result = await invoke<string>('execute_confirmed_ai_action', {
          actionType: suggestion.actionType,
          payload: suggestion.payload,
          alphaVantageApiKey: alphaVantageApiKey || null,
        });
      }

      // Save success message to database
      const successContent = `✓ ${result}`;
      const msgId = await invoke<number>('save_chat_message', {
        role: 'assistant',
        content: successContent,
      });

      const successMessage: ChatMessageData = {
        id: String(msgId),
        role: 'assistant',
        content: successContent,
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
          {isLoadingHistory ? (
            <div className="flex items-center justify-center py-8">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : messages.length === 0 ? (
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
            <div className="space-y-3">
              {/* Transaction suggestions get special preview treatment */}
              {pendingSuggestions
                .filter((s) => s.actionType === 'transaction_create' || s.actionType === 'portfolio_transfer')
                .map((suggestion, idx) => (
                  <TransactionConfirmation
                    key={`txn-${idx}`}
                    suggestion={suggestion}
                    onConfirm={() => executeSuggestion(suggestion)}
                    onDecline={() => declineSuggestion(suggestion)}
                    isExecuting={executingSuggestion === suggestion.payload}
                  />
                ))}

              {/* Other suggestions (watchlist, etc.) */}
              {pendingSuggestions.filter((s) => s.actionType !== 'transaction_create' && s.actionType !== 'portfolio_transfer').length > 0 && (
                <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/30 space-y-3">
                  <div className="flex items-center gap-2 text-amber-600">
                    <AlertTriangle className="h-4 w-4" />
                    <span className="text-sm font-medium">
                      {pendingSuggestions.filter((s) => s.actionType !== 'transaction_create' && s.actionType !== 'portfolio_transfer').length === 1
                        ? 'Aktion erfordert Bestätigung'
                        : `${pendingSuggestions.filter((s) => s.actionType !== 'transaction_create' && s.actionType !== 'portfolio_transfer').length} Aktionen erfordern Bestätigung`}
                    </span>
                    {pendingSuggestions.filter((s) => s.actionType !== 'transaction_create' && s.actionType !== 'portfolio_transfer').length > 1 && (
                      <button
                        onClick={declineAllSuggestions}
                        className="ml-auto text-xs text-muted-foreground hover:text-foreground"
                      >
                        Alle ablehnen
                      </button>
                    )}
                  </div>
                  {pendingSuggestions
                    .filter((s) => s.actionType !== 'transaction_create' && s.actionType !== 'portfolio_transfer')
                    .map((suggestion, idx) => (
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
            <div className="flex gap-2 items-end">
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Nachricht eingeben..."
                rows={3}
                className="flex-1 resize-y min-h-[76px] max-h-[200px] rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
                disabled={isLoading}
              />
              <button
                onClick={() => sendMessage(input)}
                disabled={!input.trim() || isLoading}
                className="p-2 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shrink-0"
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
