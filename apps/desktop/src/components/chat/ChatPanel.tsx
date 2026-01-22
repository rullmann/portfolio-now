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
import { X, Send, Loader2, Trash2, MessageSquare, GripVertical, CheckCircle, XCircle, AlertTriangle, Receipt, Plus, Check, Image as ImageIcon } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { readFile } from '@tauri-apps/plugin-fs';
import { useSettingsStore, type AiProvider } from '../../store';
import { AIModelSelector } from '../common/AIModelSelector';
import { ChatMessage, type ChatMessageData } from './ChatMessage';
import { VisionIndicator } from './VisionIndicator';
import { ImageAttachmentPreview, type ChatImageAttachment } from './ImageAttachmentPreview';
import { ImageUploadConsentDialog } from './ImageUploadConsentDialog';
import { ExtractedTransactionsPreview, type ExtractedTransaction, type ExtractedTransactionsPayload, type Portfolio } from './ExtractedTransactionsPreview';
import { cn } from '../../lib/utils';
import type { ChatHistoryMessage, TransactionCreateCommand, PortfolioTransferCommand, Conversation, ImageImportTransactionsResult } from '../../lib/types';
import { formatSharesFromScaled, formatAmountFromScaled, getTransactionTypeLabel, formatDate } from '../../lib/types';
import { DropdownMenu, DropdownItem } from '../common/DropdownMenu';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';

// Image upload constants
const MAX_IMAGE_SIZE_MB = 10;
const MAX_IMAGE_SIZE_BYTES = MAX_IMAGE_SIZE_MB * 1024 * 1024;
const ALLOWED_IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];
const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp'];

const MIN_WIDTH = 320;
const MAX_WIDTH = 800;
const DEFAULT_WIDTH = 420;
const STORAGE_KEY_WIDTH = 'portfolio-chat-width';

interface ChatPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

interface SuggestedAction {
  id?: number; // DB ID (undefined for new suggestions)
  messageId?: number; // Associated message ID
  actionType: string;
  description: string;
  payload: string;
  status: 'pending' | 'confirmed' | 'declined';
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
  const isPending = suggestion.status === 'pending';

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
      <div className="p-3 rounded-lg border border-primary/30 bg-primary/5">
        <div className="flex items-center gap-2 mb-2">
          <Receipt className="h-4 w-4 text-primary" />
          <span className="font-medium text-sm">Transaktion bestätigen</span>
        </div>

        <table className="w-full text-sm mb-2">
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

        {isPending && (
          <div className="flex gap-2 mt-3">
            <button
              onClick={onConfirm}
              disabled={isExecuting}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
            >
              {isExecuting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <CheckCircle className="h-3.5 w-3.5" />
              )}
              Bestätigen
            </button>
            <button
              onClick={onDecline}
              disabled={isExecuting}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 transition-colors"
            >
              <XCircle className="h-3.5 w-3.5" />
              Abbrechen
            </button>
          </div>
        )}
      </div>
    );
  }

  // Render transfer preview
  if (isTransfer) {
    const transfer = preview as PortfolioTransferCommand;
    return (
      <div className="p-3 rounded-lg border border-primary/30 bg-primary/5">
        <div className="flex items-center gap-2 mb-2">
          <Receipt className="h-4 w-4 text-primary" />
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

        {isPending && (
          <div className="flex gap-2 mt-3">
            <button
              onClick={onConfirm}
              disabled={isExecuting}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
            >
              {isExecuting ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <CheckCircle className="h-3.5 w-3.5" />
              )}
              Bestätigen
            </button>
            <button
              onClick={onDecline}
              disabled={isExecuting}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 transition-colors"
            >
              <XCircle className="h-3.5 w-3.5" />
              Abbrechen
            </button>
          </div>
        )}
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
  const [suggestions, setSuggestions] = useState<SuggestedAction[]>([]);
  const [executingSuggestion, setExecutingSuggestion] = useState<string | null>(null);
  const [importingTransactions, setImportingTransactions] = useState<string | null>(null);
  const [panelWidth, setPanelWidth] = useState(() => {
    const saved = localStorage.getItem(STORAGE_KEY_WIDTH);
    return saved ? Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, parseInt(saved, 10))) : DEFAULT_WIDTH;
  });
  const [isResizing, setIsResizing] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // Image attachment state
  const [attachments, setAttachments] = useState<ChatImageAttachment[]>([]);
  const [hasVisionSupport, setHasVisionSupport] = useState(false);
  const [showImageConsent, setShowImageConsent] = useState(false);
  const [imageConsentGiven, setImageConsentGiven] = useState(false);
  const [pendingImageUpload, setPendingImageUpload] = useState<File[] | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  // Conversation state
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<number | null>(null);
  const [isFirstMessage, setIsFirstMessage] = useState(true);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Portfolio state for extracted transactions
  const [portfolios, setPortfolios] = useState<Portfolio[]>([]);

  const {
    aiFeatureSettings,
    baseCurrency,
    alphaVantageApiKey,
    userName,
    chatContextSize,
    deliveryMode,
  } = useSettingsStore();

  const { keys } = useSecureApiKeys();

  // Get feature-specific provider and model for Chat Assistant (default)
  const { provider: defaultProvider, model: defaultModel } = aiFeatureSettings.chatAssistant;

  // Temporary model override (not persisted unless "save as default" is checked)
  const [tempSelection, setTempSelection] = useState<{ provider: AiProvider; model: string } | null>(null);

  // Effective provider/model (temp selection or default)
  const aiProvider = tempSelection?.provider ?? defaultProvider;
  const aiModel = tempSelection?.model ?? defaultModel;

  const getApiKey = (provider: AiProvider) => {
    switch (provider) {
      case 'claude':
        return keys.anthropicApiKey;
      case 'openai':
        return keys.openaiApiKey;
      case 'gemini':
        return keys.geminiApiKey;
      case 'perplexity':
        return keys.perplexityApiKey;
      default:
        return '';
    }
  };

  const hasApiKey = () => {
    const key = getApiKey(aiProvider);
    return key && key.trim().length > 0;
  };

  // Check vision support when model changes
  useEffect(() => {
    const checkVision = async () => {
      try {
        const result = await invoke<boolean>('check_vision_support', { model: aiModel });
        setHasVisionSupport(result);
        // Clear attachments if switching to non-vision model
        if (!result && attachments.length > 0) {
          setAttachments([]);
        }
      } catch {
        setHasVisionSupport(false);
      }
    };
    if (aiModel) {
      checkVision();
    }
  }, [aiModel]);

  // Load portfolios for transaction import
  useEffect(() => {
    const loadPortfolios = async () => {
      try {
        const result = await invoke<Array<{ id: number; name: string }>>('get_pp_portfolios', {});
        // Filter out retired portfolios if needed (backend should already do this)
        setPortfolios(result.filter((p) => p.name).map((p) => ({ id: p.id, name: p.name })));
      } catch (e) {
        console.error('Failed to load portfolios:', e);
      }
    };
    loadPortfolios();
  }, []);

  // Tauri native drag-drop handler (external files)
  useEffect(() => {
    if (!isOpen) return;

    const window = getCurrentWindow();
    let unlistenFn: (() => void) | null = null;

    const setupListener = async () => {
      unlistenFn = await window.onDragDropEvent(async (event) => {
        if (event.payload.type === 'drop' && event.payload.paths.length > 0) {
          // Filter for image files by extension
          const imagePaths = event.payload.paths.filter((path: string) => {
            const ext = path.split('.').pop()?.toLowerCase() || '';
            return IMAGE_EXTENSIONS.includes(ext);
          });

          if (imagePaths.length === 0) return;

          if (!hasVisionSupport) {
            setError('Das aktuelle Modell unterstützt keine Bilder. Bitte wähle ein Vision-fähiges Modell.');
            return;
          }

          // Check consent first
          if (!imageConsentGiven) {
            // Store paths for later processing after consent
            const files: File[] = [];
            for (const path of imagePaths) {
              try {
                const data = await readFile(path);
                const ext = path.split('.').pop()?.toLowerCase() || 'png';
                const mimeType = ext === 'jpg' ? 'image/jpeg' : `image/${ext}`;
                const filename = path.split('/').pop() || path.split('\\').pop() || 'image';
                const blob = new Blob([data], { type: mimeType });
                files.push(new File([blob], filename, { type: mimeType }));
              } catch (err) {
                console.error('Failed to read dropped file:', path, err);
              }
            }
            if (files.length > 0) {
              setPendingImageUpload(files);
              setShowImageConsent(true);
            }
            return;
          }

          // Process dropped files
          setError(null);
          const newAttachments: ChatImageAttachment[] = [];

          for (const path of imagePaths) {
            try {
              const data = await readFile(path);
              const ext = path.split('.').pop()?.toLowerCase() || 'png';
              const mimeType = ext === 'jpg' ? 'image/jpeg' : `image/${ext}`;
              const filename = path.split('/').pop() || path.split('\\').pop() || 'image';

              // Check file size
              if (data.byteLength > MAX_IMAGE_SIZE_BYTES) {
                setError(`Bild zu groß: ${filename} (max. ${MAX_IMAGE_SIZE_MB} MB)`);
                continue;
              }

              // Convert to base64
              const base64 = btoa(
                new Uint8Array(data).reduce((data, byte) => data + String.fromCharCode(byte), '')
              );

              newAttachments.push({
                data: base64,
                mimeType,
                filename,
              });
            } catch (err) {
              console.error('Failed to read dropped file:', path, err);
            }
          }

          if (newAttachments.length > 0) {
            setAttachments((prev) => [...prev, ...newAttachments]);
          }
        }
      });
    };

    setupListener();

    return () => {
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [isOpen, hasVisionSupport, imageConsentGiven]);

  // Get provider display name for consent dialog
  const getProviderDisplayName = (provider: AiProvider) => {
    switch (provider) {
      case 'claude': return 'Anthropic Claude';
      case 'openai': return 'OpenAI';
      case 'gemini': return 'Google Gemini';
      case 'perplexity': return 'Perplexity';
      default: return provider;
    }
  };

  // Image upload handlers
  const processImageFile = async (file: File): Promise<ChatImageAttachment | null> => {
    // Validate file size
    if (file.size > MAX_IMAGE_SIZE_BYTES) {
      setError(`Bild zu groß: ${file.name} (max. ${MAX_IMAGE_SIZE_MB} MB)`);
      return null;
    }

    // Validate MIME type
    if (!ALLOWED_IMAGE_TYPES.includes(file.type)) {
      setError(`Ungültiger Dateityp: ${file.name}. Erlaubt: PNG, JPEG, GIF, WebP`);
      return null;
    }

    // Read file and convert to base64
    return new Promise((resolve) => {
      const reader = new FileReader();
      reader.onload = () => {
        const base64 = (reader.result as string).split(',')[1];
        resolve({
          data: base64,
          mimeType: file.type,
          filename: file.name,
        });
      };
      reader.onerror = () => {
        setError(`Fehler beim Lesen: ${file.name}`);
        resolve(null);
      };
      reader.readAsDataURL(file);
    });
  };

  const handleImageFiles = async (files: File[]) => {
    // Check consent first
    if (!imageConsentGiven) {
      setPendingImageUpload(files);
      setShowImageConsent(true);
      return;
    }

    // Process files
    setError(null);
    const newAttachments: ChatImageAttachment[] = [];

    for (const file of files) {
      const attachment = await processImageFile(file);
      if (attachment) {
        newAttachments.push(attachment);
      }
    }

    if (newAttachments.length > 0) {
      setAttachments((prev) => [...prev, ...newAttachments]);
    }
  };

  const handleImageConsentConfirm = async () => {
    setImageConsentGiven(true);
    setShowImageConsent(false);

    // Process pending uploads directly (don't call handleImageFiles as state isn't updated yet)
    if (pendingImageUpload) {
      const filesToProcess = pendingImageUpload;
      setPendingImageUpload(null);

      setError(null);
      const newAttachments: ChatImageAttachment[] = [];

      for (const file of filesToProcess) {
        const attachment = await processImageFile(file);
        if (attachment) {
          newAttachments.push(attachment);
        }
      }

      if (newAttachments.length > 0) {
        setAttachments((prev) => [...prev, ...newAttachments]);
      }
    }
  };

  const handleImageConsentCancel = () => {
    setShowImageConsent(false);
    setPendingImageUpload(null);
  };

  const handleImageUploadClick = async () => {
    // Check vision support first
    if (!hasVisionSupport) {
      setError('Das aktuelle Modell unterstützt keine Bilder. Bitte wähle ein Vision-fähiges Modell.');
      return;
    }

    // Open file dialog via Tauri
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: 'Bilder',
          extensions: IMAGE_EXTENSIONS,
        }],
      });

      if (selected && selected.length > 0) {
        // Read files and convert to attachments
        const files: File[] = [];
        for (const path of selected) {
          try {
            const data = await readFile(path);
            const ext = path.split('.').pop()?.toLowerCase() || 'png';
            const mimeType = ext === 'jpg' ? 'image/jpeg' : `image/${ext}`;
            const filename = path.split('/').pop() || path.split('\\').pop() || 'image';

            // Create a File-like object
            const blob = new Blob([data], { type: mimeType });
            const file = new File([blob], filename, { type: mimeType });
            files.push(file);
          } catch (err) {
            console.error('Failed to read file:', path, err);
          }
        }

        if (files.length > 0) {
          await handleImageFiles(files);
        }
      }
    } catch (err) {
      console.error('File dialog error:', err);
    }
  };

  const handlePaste = async (e: React.ClipboardEvent) => {
    const items = e.clipboardData?.items;
    if (!items) return;

    const imageFiles: File[] = [];
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.type.startsWith('image/')) {
        const file = item.getAsFile();
        if (file) {
          imageFiles.push(file);
        }
      }
    }

    if (imageFiles.length > 0) {
      e.preventDefault();
      if (!hasVisionSupport) {
        setError('Das aktuelle Modell unterstützt keine Bilder. Bitte wähle ein Vision-fähiges Modell.');
        return;
      }
      await handleImageFiles(imageFiles);
    }
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    if (!hasVisionSupport) {
      setError('Das aktuelle Modell unterstützt keine Bilder. Bitte wähle ein Vision-fähiges Modell.');
      return;
    }

    const files = Array.from(e.dataTransfer.files).filter(
      (file) => file.type.startsWith('image/')
    );

    if (files.length > 0) {
      await handleImageFiles(files);
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    // Only show drag indicator if dragging files (not text)
    if (e.dataTransfer.types.includes('Files')) {
      setIsDragging(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    // Only reset if leaving the panel (not entering a child element)
    const rect = panelRef.current?.getBoundingClientRect();
    if (rect) {
      const { clientX, clientY } = e;
      if (
        clientX < rect.left ||
        clientX > rect.right ||
        clientY < rect.top ||
        clientY > rect.bottom
      ) {
        setIsDragging(false);
      }
    }
  };

  const removeAttachment = (index: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== index));
  };

  // Load conversations and select the most recent one on mount
  useEffect(() => {
    const loadConversations = async () => {
      try {
        setIsLoadingHistory(true);
        const convs = await invoke<Conversation[]>('get_conversations');
        setConversations(convs);

        if (convs.length > 0) {
          // Select the most recent conversation (first in list, sorted by updated_at DESC)
          setCurrentConversationId(convs[0].id);
          setIsFirstMessage(convs[0].messageCount === 0);
        } else {
          // Create a new conversation if none exist
          const newConv = await invoke<Conversation>('create_conversation', { title: null });
          setConversations([newConv]);
          setCurrentConversationId(newConv.id);
          setIsFirstMessage(true);
        }
      } catch (err) {
        console.error('Failed to load conversations:', err);
      } finally {
        setIsLoadingHistory(false);
      }
    };
    loadConversations();
  }, []);

  // Load messages and suggestions when conversation changes
  useEffect(() => {
    const loadMessagesForConversation = async () => {
      if (!currentConversationId) return;

      try {
        const history = await invoke<ChatHistoryMessage[]>('get_chat_history', {
          conversationId: currentConversationId,
          limit: null,
        });
        const loaded: ChatMessageData[] = history.map((m) => ({
          id: String(m.id),
          role: m.role as 'user' | 'assistant',
          content: m.content,
          timestamp: new Date(m.createdAt),
          attachments: m.attachments && m.attachments.length > 0
            ? m.attachments.map(a => ({ data: a.data, mimeType: a.mimeType, filename: a.filename }))
            : undefined,
        }));
        setMessages(loaded);
        setIsFirstMessage(loaded.length === 0);

        // Load pending suggestions for this conversation
        interface DbSuggestion {
          id: number;
          messageId: number;
          actionType: string;
          description: string;
          payload: string;
          status: string;
        }
        const dbSuggestions = await invoke<DbSuggestion[]>('get_pending_suggestions', {
          conversationId: currentConversationId,
        });
        const loadedSuggestions: SuggestedAction[] = dbSuggestions.map((s) => ({
          id: s.id,
          messageId: s.messageId,
          actionType: s.actionType,
          description: s.description,
          payload: s.payload,
          status: s.status as 'pending' | 'confirmed' | 'declined',
        }));
        setSuggestions(loadedSuggestions);

        // Scroll to bottom after loading messages
        setTimeout(() => {
          messagesEndRef.current?.scrollIntoView({ behavior: 'instant' });
        }, 50);
      } catch (err) {
        console.error('Failed to load messages for conversation:', err);
      }
    };
    loadMessagesForConversation();
  }, [currentConversationId]);

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
    // Allow sending with just attachments (no text required if images present)
    const hasContent = content.trim().length > 0;
    const hasAttachments = attachments.length > 0;

    if ((!hasContent && !hasAttachments) || isLoading || !currentConversationId) return;

    const trimmedContent = content.trim();
    const currentAttachments = [...attachments]; // Copy for sending

    setInput('');
    setAttachments([]); // Clear attachments immediately
    setIsLoading(true);
    setError(null);

    try {
      // Build user message content (include attachment indicator if images sent)
      const displayContent = hasAttachments && !hasContent
        ? `[${currentAttachments.length} Bild${currentAttachments.length > 1 ? 'er' : ''} gesendet]`
        : hasAttachments
        ? `${trimmedContent}\n\n[${currentAttachments.length} Bild${currentAttachments.length > 1 ? 'er' : ''} angehängt]`
        : trimmedContent;

      // Save user message to database first (with attachments)
      const userMsgId = await invoke<number>('save_chat_message', {
        role: 'user',
        content: displayContent,
        conversationId: currentConversationId,
        attachments: currentAttachments.length > 0
          ? currentAttachments.map(a => ({
              data: a.data,
              mimeType: a.mimeType,
              filename: a.filename,
            }))
          : null,
      });

      const userMessage: ChatMessageData = {
        id: String(userMsgId),
        role: 'user',
        content: displayContent,
        timestamp: new Date(),
        attachments: currentAttachments.length > 0 ? currentAttachments : undefined,
      };

      setMessages((prev) => [...prev, userMessage]);

      // Update conversation title on first user message
      if (isFirstMessage) {
        const newTitle = displayContent.substring(0, 40) + (displayContent.length > 40 ? '...' : '');
        await invoke('update_conversation_title', {
          id: currentConversationId,
          title: newTitle,
        });
        setConversations((prev) =>
          prev.map((c) => (c.id === currentConversationId ? { ...c, title: newTitle } : c))
        );
        setIsFirstMessage(false);
      }

      // Build message history for API with sliding window
      const allMessages = [...messages, userMessage];
      const contextMessages = allMessages.slice(-chatContextSize);

      // Format messages for API - include attachments only on the last (current) message
      const apiMessages = contextMessages.map((m, idx) => {
        const isLastMessage = idx === contextMessages.length - 1;
        return {
          role: m.role,
          content: m.content,
          // Add attachments only to the current user message being sent
          attachments: isLastMessage && m.role === 'user' && currentAttachments.length > 0
            ? currentAttachments
            : [],
        };
      });

      const response = await invoke<PortfolioChatResponse>('chat_with_portfolio_assistant', {
        request: {
          messages: apiMessages,
          provider: aiProvider,
          model: aiModel,
          apiKey: getApiKey(aiProvider),
          baseCurrency: baseCurrency || 'EUR',
          userName: userName || null,
        },
      });

      // Save assistant response to database
      const assistantMsgId = await invoke<number>('save_chat_message', {
        role: 'assistant',
        content: response.response,
        conversationId: currentConversationId,
      });

      const assistantMessage: ChatMessageData = {
        id: String(assistantMsgId),
        role: 'assistant',
        content: response.response,
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);

      // Update conversation's messageCount and updatedAt in local state
      setConversations((prev) =>
        prev.map((c) =>
          c.id === currentConversationId
            ? { ...c, messageCount: c.messageCount + 2, updatedAt: new Date().toISOString() }
            : c
        )
      );

      // Store any suggestions that need user confirmation
      if (response.suggestions && response.suggestions.length > 0) {
        // Save suggestions to database and update local state
        const savedSuggestions: SuggestedAction[] = [];
        for (const suggestion of response.suggestions) {
          try {
            const suggestionId = await invoke<number>('save_chat_suggestion', {
              messageId: Number(assistantMsgId),
              conversationId: currentConversationId,
              actionType: suggestion.actionType,
              description: suggestion.description,
              payload: suggestion.payload,
            });
            savedSuggestions.push({
              ...suggestion,
              id: suggestionId,
              messageId: Number(assistantMsgId),
              status: 'pending',
            });
          } catch (err) {
            console.error('Failed to save suggestion:', err);
          }
        }
        setSuggestions((prev) => [...prev, ...savedSuggestions]);
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

  // Create a new conversation
  const handleNewConversation = async () => {
    try {
      const newConv = await invoke<Conversation>('create_conversation', { title: null });
      setConversations((prev) => [newConv, ...prev]);
      setCurrentConversationId(newConv.id);
      setMessages([]);
      setSuggestions([]);
      setIsFirstMessage(true);
      setError(null);
    } catch (err) {
      console.error('Failed to create conversation:', err);
      setError('Fehler beim Erstellen der Conversation');
    }
  };

  // Switch to a different conversation
  const switchConversation = (convId: number) => {
    if (convId !== currentConversationId) {
      setCurrentConversationId(convId);
      setError(null);
    }
  };

  // Delete the current conversation
  const handleDeleteConversation = async () => {
    if (!currentConversationId) return;

    try {
      await invoke('delete_conversation', { id: currentConversationId });

      // Remove from local state
      const remaining = conversations.filter((c) => c.id !== currentConversationId);
      setConversations(remaining);

      // Switch to next conversation or create new one
      if (remaining.length > 0) {
        setCurrentConversationId(remaining[0].id);
      } else {
        const newConv = await invoke<Conversation>('create_conversation', { title: null });
        setConversations([newConv]);
        setCurrentConversationId(newConv.id);
        setIsFirstMessage(true);
      }

      setShowDeleteConfirm(false);
    } catch (err) {
      console.error('Failed to delete conversation:', err);
      setError('Fehler beim Löschen der Conversation');
    }
  };

  // Get current conversation
  const currentConversation = conversations.find((c) => c.id === currentConversationId);

  // Format relative time for dropdown
  const formatRelativeTime = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return 'Gerade eben';
    if (diffMins < 60) return `Vor ${diffMins} Min.`;
    if (diffHours < 24) return `Vor ${diffHours} Std.`;
    if (diffDays === 1) return 'Gestern';
    if (diffDays < 7) return `Vor ${diffDays} Tagen`;
    return formatDate(dateStr);
  };

  const deleteMessage = async (id: string) => {
    try {
      const messageId = parseInt(id, 10);
      await invoke('delete_chat_message', { id: messageId });
      setMessages((prev) => prev.filter((m) => m.id !== id));
      // Also remove any suggestions associated with this message (DB cascade handles the rest)
      setSuggestions((prev) => prev.filter((s) => s.messageId !== messageId));
    } catch (err) {
      console.error('Failed to delete chat message:', err);
      setError('Fehler beim Löschen der Nachricht');
    }
  };

  // Execute a confirmed suggestion
  const executeSuggestion = async (suggestion: SuggestedAction) => {
    if (!currentConversationId) return;

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
      } else if (suggestion.actionType === 'transaction_delete') {
        result = await invoke<string>('execute_confirmed_transaction_delete', {
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
        conversationId: currentConversationId,
      });

      const successMessage: ChatMessageData = {
        id: String(msgId),
        role: 'assistant',
        content: successContent,
        timestamp: new Date(),
      };
      setMessages((prev) => [...prev, successMessage]);

      // Update suggestion status to confirmed in DB and local state
      if (suggestion.id) {
        await invoke('update_suggestion_status', { id: suggestion.id, status: 'confirmed' });
      }
      setSuggestions((prev) =>
        prev.map((s) => (s.payload === suggestion.payload ? { ...s, status: 'confirmed' as const } : s))
      );
    } catch (err) {
      setError(typeof err === 'string' ? err : String(err));
    } finally {
      setExecutingSuggestion(null);
    }
  };

  // Decline a suggestion
  const declineSuggestion = async (suggestion: SuggestedAction) => {
    // Update suggestion status to declined in DB and local state
    if (suggestion.id) {
      try {
        await invoke('update_suggestion_status', { id: suggestion.id, status: 'declined' });
      } catch (err) {
        console.error('Failed to update suggestion status:', err);
      }
    }
    setSuggestions((prev) =>
      prev.map((s) => (s.payload === suggestion.payload ? { ...s, status: 'declined' as const } : s))
    );
  };

  // Handle extracted transactions import
  const handleImportExtractedTransactions = async (
    suggestion: SuggestedAction,
    transactions: ExtractedTransaction[],
    portfolioId: number | null
  ) => {
    if (!currentConversationId) return;

    setImportingTransactions(suggestion.payload);
    try {
      // Call backend to import the transactions (with delivery mode from settings - SSOT)
        const result = await invoke<ImageImportTransactionsResult>('import_extracted_transactions', {
        transactions: transactions.map((t) => ({
          date: t.date,
          txn_type: t.txnType,
          security_name: t.securityName || null,
          isin: t.isin || null,
          shares: t.shares || null,
          gross_amount: t.grossAmount || null,
          gross_currency: t.grossCurrency || null,
          amount: t.amount || null,
          currency: t.currency,
          fees: t.fees || null,
          fees_foreign: t.feesForeign || null,
          fees_foreign_currency: t.feesForeignCurrency || null,
          exchange_rate: t.exchangeRate || null,
          taxes: t.taxes || null,
          note: t.note || null,
        })),
        portfolioId: portfolioId,
        deliveryMode: deliveryMode,
      });

      // Build success message
      let successContent = '';
      if (result.importedCount > 0) {
        successContent = `✓ ${result.importedCount} Transaktion${result.importedCount === 1 ? '' : 'en'} erfolgreich importiert`;
      }

      // Add errors to success message if any
      if (result.errors.length > 0) {
        if (successContent) successContent += '\n\n';
        successContent += `⚠️ Fehler: ${result.errors.join('; ')}`;
      }

      // Save success message if there's any content
      if (successContent) {
        const msgId = await invoke<number>('save_chat_message', {
          role: 'assistant',
          content: successContent,
          conversationId: currentConversationId,
        });

        setMessages((prev) => [
          ...prev,
          {
            id: String(msgId),
            role: 'assistant',
            content: successContent,
            timestamp: new Date(),
          },
        ]);
      }

      // Show duplicates as a separate message with special styling (red border)
      if (result.duplicates.length > 0) {
        const duplicateContent = `🔄 Duplikate übersprungen:\n${result.duplicates.map(d => `• ${d}`).join('\n')}`;
        const dupMsgId = await invoke<number>('save_chat_message', {
          role: 'assistant',
          content: duplicateContent,
          conversationId: currentConversationId,
        });

        setMessages((prev) => [
          ...prev,
          {
            id: String(dupMsgId),
            role: 'assistant',
            content: duplicateContent,
            timestamp: new Date(),
            isDuplicate: true, // Mark as duplicate for special styling
          },
        ]);
      }

      // Update suggestion status
      if (suggestion.id) {
        await invoke('update_suggestion_status', { id: suggestion.id, status: 'confirmed' });
      }
      setSuggestions((prev) =>
        prev.map((s) => (s.payload === suggestion.payload ? { ...s, status: 'confirmed' as const } : s))
      );
    } catch (err) {
      setError(typeof err === 'string' ? err : String(err));
    } finally {
      setImportingTransactions(null);
    }
  };

  // Discard extracted transactions
  const handleDiscardExtractedTransactions = async (suggestion: SuggestedAction) => {
    if (suggestion.id) {
      try {
        await invoke('update_suggestion_status', { id: suggestion.id, status: 'declined' });
      } catch (err) {
        console.error('Failed to update suggestion status:', err);
      }
    }
    setSuggestions((prev) =>
      prev.map((s) => (s.payload === suggestion.payload ? { ...s, status: 'declined' as const } : s))
    );
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
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragEnter={handleDragEnter}
        onDragLeave={handleDragLeave}
      >
        {/* Drag overlay */}
        {isDragging && hasVisionSupport && (
          <div className="absolute inset-0 z-50 bg-primary/10 border-2 border-dashed border-primary rounded-lg flex items-center justify-center pointer-events-none">
            <div className="bg-background/90 rounded-lg p-4 shadow-lg text-center">
              <ImageIcon className="h-8 w-8 mx-auto mb-2 text-primary" />
              <p className="text-sm font-medium">Bild hier ablegen</p>
            </div>
          </div>
        )}
        {isDragging && !hasVisionSupport && (
          <div className="absolute inset-0 z-50 bg-destructive/10 border-2 border-dashed border-destructive rounded-lg flex items-center justify-center pointer-events-none">
            <div className="bg-background/90 rounded-lg p-4 shadow-lg text-center">
              <ImageIcon className="h-8 w-8 mx-auto mb-2 text-destructive" />
              <p className="text-sm font-medium text-destructive">Modell unterstützt keine Bilder</p>
            </div>
          </div>
        )}

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
          <div className="flex items-center gap-2 min-w-0 flex-1">
            {/* New Conversation Button */}
            <button
              onClick={handleNewConversation}
              className="p-1.5 rounded hover:bg-muted transition-colors shrink-0"
              title="Neuer Chat"
            >
              <Plus className="h-4 w-4" />
            </button>

            {/* Conversation Dropdown */}
            <DropdownMenu
              trigger={
                <span className="truncate max-w-[180px]">
                  {currentConversation?.title || 'Neuer Chat'}
                </span>
              }
              align="left"
            >
              {conversations.map((conv) => (
                <DropdownItem
                  key={conv.id}
                  onClick={() => switchConversation(conv.id)}
                  icon={conv.id === currentConversationId ? <Check className="h-4 w-4 text-primary" /> : <span className="h-4 w-4" />}
                >
                  <div className="min-w-0 flex-1">
                    <div className="truncate font-medium">{conv.title}</div>
                    <div className="text-xs text-muted-foreground">
                      {formatRelativeTime(conv.updatedAt)} · {conv.messageCount} Nachrichten
                    </div>
                  </div>
                </DropdownItem>
              ))}
              {conversations.length === 0 && (
                <div className="px-2 py-3 text-sm text-muted-foreground text-center">
                  Keine Conversations
                </div>
              )}
            </DropdownMenu>
          </div>

          <div className="flex items-center gap-1 shrink-0">
            {/* Delete Conversation Button */}
            {conversations.length > 0 && (
              <button
                onClick={() => setShowDeleteConfirm(true)}
                className="p-2 rounded hover:bg-muted transition-colors"
                title="Chat löschen"
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

        {/* Delete Confirmation Dialog */}
        {showDeleteConfirm && (
          <div className="p-4 border-b border-border bg-destructive/5">
            <div className="flex items-center gap-2 mb-2">
              <AlertTriangle className="h-4 w-4 text-destructive" />
              <span className="font-medium text-sm">Chat löschen?</span>
            </div>
            <p className="text-sm text-muted-foreground mb-3">
              "{currentConversation?.title}" und alle Nachrichten werden gelöscht.
            </p>
            <div className="flex gap-2">
              <button
                onClick={handleDeleteConversation}
                className="px-3 py-1.5 text-sm font-medium rounded-md bg-destructive text-destructive-foreground hover:bg-destructive/90 transition-colors"
              >
                Löschen
              </button>
              <button
                onClick={() => setShowDeleteConfirm(false)}
                className="px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 transition-colors"
              >
                Abbrechen
              </button>
            </div>
          </div>
        )}

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

          {/* Suggestions - Pending require confirmation, completed are shown grayed */}
          {suggestions.length > 0 && (
            <div className="space-y-3">
              {/* Transaction suggestions get special preview treatment */}
              {suggestions
                .filter((s) => s.actionType === 'transaction_create' || s.actionType === 'portfolio_transfer')
                .map((suggestion, idx) => (
                  <div key={`txn-${idx}`} className={cn(suggestion.status !== 'pending' && 'opacity-60')}>
                    <TransactionConfirmation
                      suggestion={suggestion}
                      onConfirm={() => executeSuggestion(suggestion)}
                      onDecline={() => declineSuggestion(suggestion)}
                      isExecuting={executingSuggestion === suggestion.payload}
                    />
                    {suggestion.status !== 'pending' && (
                      <div className="mt-2 text-xs text-muted-foreground flex items-center gap-1.5">
                        {suggestion.status === 'confirmed' ? (
                          <><CheckCircle className="h-3.5 w-3.5 text-green-600" /> Bestätigt</>
                        ) : (
                          <><XCircle className="h-3.5 w-3.5 text-muted-foreground" /> Abgebrochen</>
                        )}
                      </div>
                    )}
                  </div>
                ))}

              {/* Delete suggestions - red/destructive UI */}
              {suggestions
                .filter((s) => s.actionType === 'transaction_delete')
                .map((suggestion, idx) => (
                  <div key={`delete-${idx}`} className={cn('p-3 rounded-lg border border-red-500/30 bg-red-500/5', suggestion.status !== 'pending' && 'opacity-60')}>
                    <div className="flex items-start gap-2">
                      <Trash2 className="h-4 w-4 text-red-500 mt-0.5 shrink-0" />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm font-medium text-red-600">Transaktion löschen</p>
                        <p className="text-sm text-muted-foreground">{suggestion.description}</p>
                        {suggestion.status === 'pending' ? (
                          <div className="flex gap-2 mt-2">
                            <button
                              onClick={() => executeSuggestion(suggestion)}
                              disabled={executingSuggestion !== null}
                              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-red-600 text-white hover:bg-red-700 disabled:opacity-50 transition-colors"
                            >
                              {executingSuggestion === suggestion.payload ? (
                                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                              ) : (
                                <Trash2 className="h-3.5 w-3.5" />
                              )}
                              Löschen
                            </button>
                            <button
                              onClick={() => declineSuggestion(suggestion)}
                              disabled={executingSuggestion !== null}
                              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 transition-colors"
                            >
                              <XCircle className="h-3.5 w-3.5" />
                              Abbrechen
                            </button>
                          </div>
                        ) : (
                          <div className="mt-2 text-xs text-muted-foreground flex items-center gap-1.5">
                            {suggestion.status === 'confirmed' ? (
                              <><CheckCircle className="h-3.5 w-3.5 text-green-600" /> Gelöscht</>
                            ) : (
                              <><XCircle className="h-3.5 w-3.5 text-muted-foreground" /> Abgebrochen</>
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ))}

              {/* Extracted transactions from images */}
              {suggestions
                .filter((s) => s.actionType === 'extracted_transactions')
                .map((suggestion, idx) => {
                  // Parse the payload to get transactions
                  let payload: ExtractedTransactionsPayload | null = null;
                  try {
                    payload = JSON.parse(suggestion.payload);
                  } catch {
                    return null;
                  }
                  if (!payload) return null;

                  return (
                    <div
                      key={`extracted-${idx}`}
                      className={cn(suggestion.status !== 'pending' && 'opacity-60')}
                    >
                      {suggestion.status === 'pending' ? (
                        <ExtractedTransactionsPreview
                          payload={payload}
                          portfolios={portfolios}
                          onConfirm={(txns, portfolioId) => handleImportExtractedTransactions(suggestion, txns, portfolioId)}
                          onDiscard={() => handleDiscardExtractedTransactions(suggestion)}
                          isImporting={importingTransactions === suggestion.payload}
                        />
                      ) : (
                        <div className="p-3 rounded-lg border border-amber-500/30 bg-amber-500/5">
                          <div className="flex items-center gap-2 text-sm">
                            <Receipt className="h-4 w-4 text-amber-600" />
                            <span>
                              {payload.transactions.length} Transaktion{payload.transactions.length !== 1 ? 'en' : ''}
                            </span>
                            {suggestion.status === 'confirmed' ? (
                              <span className="text-green-600 flex items-center gap-1">
                                <CheckCircle className="h-3.5 w-3.5" /> Importiert
                              </span>
                            ) : (
                              <span className="text-muted-foreground flex items-center gap-1">
                                <XCircle className="h-3.5 w-3.5" /> Verworfen
                              </span>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}

              {/* Other suggestions (watchlist, etc.) - compact UI */}
              {suggestions
                .filter((s) => s.actionType !== 'transaction_create' && s.actionType !== 'portfolio_transfer' && s.actionType !== 'transaction_delete' && s.actionType !== 'extracted_transactions')
                .map((suggestion, idx) => (
                  <div key={`other-${idx}`} className={cn('p-3 rounded-lg border border-amber-500/30 bg-amber-500/5', suggestion.status !== 'pending' && 'opacity-60')}>
                    <div className="flex items-start gap-2">
                      <AlertTriangle className="h-4 w-4 text-amber-500 mt-0.5 shrink-0" />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm">{suggestion.description}</p>
                        {suggestion.status === 'pending' ? (
                          <div className="flex gap-2 mt-2">
                            <button
                              onClick={() => executeSuggestion(suggestion)}
                              disabled={executingSuggestion !== null}
                              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
                            >
                              {executingSuggestion === suggestion.payload ? (
                                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                              ) : (
                                <CheckCircle className="h-3.5 w-3.5" />
                              )}
                              Bestätigen
                            </button>
                            <button
                              onClick={() => declineSuggestion(suggestion)}
                              disabled={executingSuggestion !== null}
                              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 transition-colors"
                            >
                              <XCircle className="h-3.5 w-3.5" />
                              Abbrechen
                            </button>
                          </div>
                        ) : (
                          <div className="mt-2 text-xs text-muted-foreground flex items-center gap-1.5">
                            {suggestion.status === 'confirmed' ? (
                              <><CheckCircle className="h-3.5 w-3.5 text-green-600" /> Bestätigt</>
                            ) : (
                              <><XCircle className="h-3.5 w-3.5 text-muted-foreground" /> Abgebrochen</>
                            )}
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* Image Attachments Preview */}
        {attachments.length > 0 && (
          <ImageAttachmentPreview
            attachments={attachments}
            onRemove={removeAttachment}
            disabled={isLoading}
          />
        )}

        {/* Input */}
        <div className="p-4 border-t border-border">
          {!hasApiKey() ? (
            <div className="text-center text-sm text-muted-foreground p-2">
              Bitte konfiguriere deinen {aiProvider.toUpperCase()} API-Key in den Einstellungen.
            </div>
          ) : (
            <div className="flex gap-2 items-end">
              {/* Image upload button */}
              <button
                type="button"
                onClick={handleImageUploadClick}
                disabled={isLoading || !hasVisionSupport}
                className={cn(
                  'p-2 rounded-lg transition-colors shrink-0',
                  hasVisionSupport
                    ? 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    : 'text-muted-foreground/50 cursor-not-allowed'
                )}
                title={hasVisionSupport ? 'Bild anhängen' : 'Modell unterstützt keine Bilder'}
              >
                <ImageIcon className="h-5 w-5" />
              </button>

              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                onPaste={handlePaste}
                placeholder={attachments.length > 0 ? 'Beschreibung hinzufügen (optional)...' : 'Nachricht eingeben...'}
                rows={3}
                className="flex-1 resize-y min-h-[76px] max-h-[200px] rounded-lg border border-input bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-primary"
                disabled={isLoading}
              />
              <button
                onClick={() => sendMessage(input)}
                disabled={(!input.trim() && attachments.length === 0) || isLoading}
                className="p-2 rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shrink-0"
              >
                <Send className="h-5 w-5" />
              </button>
            </div>
          )}

          {/* Model selector with Vision indicator */}
          <div className="mt-2 flex items-center justify-between">
            <AIModelSelector
              featureId="chatAssistant"
              value={{ provider: aiProvider, model: aiModel }}
              onChange={setTempSelection}
              compact
              disabled={isLoading}
            />
            <VisionIndicator model={aiModel} className="ml-2" />
          </div>
        </div>
      </div>

      {/* Image Upload Consent Dialog */}
      <ImageUploadConsentDialog
        isOpen={showImageConsent}
        providerName={getProviderDisplayName(aiProvider)}
        onConfirm={handleImageConsentConfirm}
        onCancel={handleImageConsentCancel}
      />
    </>
  );
}
