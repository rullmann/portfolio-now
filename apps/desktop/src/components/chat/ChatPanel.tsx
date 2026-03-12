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
import { X, Send, Loader2, Trash2, MessageSquare, GripVertical, CheckCircle, XCircle, AlertTriangle, Receipt, Plus, Check, Image as ImageIcon, Mic, Square, SlidersHorizontal, CandlestickChart, PanelRightClose, PanelRightOpen, Maximize2, Minimize2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';

// Result type for read_image_as_base64 command (matches Rust's camelCase serialization)
interface FileBase64Result {
  data: string;
  mimeType: string;  // Rust uses rename_all = "camelCase"
  filename: string;
}
import { useSettingsStore, useUIStore, toast, type AiProvider } from '../../store';
import { AIModelSelector } from '../common/AIModelSelector';
import { ChatMessage, type ChatMessageData } from './ChatMessage';
import { VisionIndicator } from './VisionIndicator';
import { ImageAttachmentPreview, type ChatImageAttachment } from './ImageAttachmentPreview';
import { ImageUploadConsentDialog } from './ImageUploadConsentDialog';
import { ExtractedTransactionsPreview, type ExtractedTransaction, type ExtractedTransactionsPayload, type Portfolio } from './ExtractedTransactionsPreview';
import { QueryApprovalCard } from './QueryApprovalCard';
import { cn } from '../../lib/utils';
import type { ChatHistoryMessage, TransactionCreateCommand, PortfolioTransferCommand, Conversation, ImageImportTransactionsResult, DuplicateCheckResponse, PendingQuery } from '../../lib/types';
import { formatSharesFromScaled, formatAmountFromScaled, getTransactionTypeLabel, formatDate } from '../../lib/types';
import { DropdownMenu, DropdownItem } from '../common/DropdownMenu';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { calculateAiCost } from '../../lib/ai-cost';

// Image upload constants
const MAX_IMAGE_SIZE_MB = 10;
const MAX_IMAGE_SIZE_BYTES = MAX_IMAGE_SIZE_MB * 1024 * 1024;
const ALLOWED_IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];
const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp'];
const PDF_EXTENSIONS = ['pdf'];

const MIN_WIDTH = 320;
const MAX_WIDTH = 800;
const DEFAULT_WIDTH = 420;
const COLLAPSED_WIDTH = 44;
const STORAGE_KEY_WIDTH = 'portfolio-chat-width';
const STORAGE_KEY_COLLAPSED = 'portfolio-chat-collapsed';

interface ChatPanelProps {
  isOpen: boolean;
  onClose: () => void;
  overlay?: boolean;
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
  inputTokens?: number;
  outputTokens?: number;
  suggestions?: SuggestedAction[];
  pendingQueries?: PendingQuery[];
}

const EXAMPLE_QUESTIONS = [
  'Wie war meine Rendite dieses Jahr?',
  'Welche Aktien zahlen Dividende?',
  'Zeige meine Top-Performer',
  'Scanne den DAX nach überverkauften Aktien',
  'Finde Ausbruchs-Kandidaten im S&P 500',
  'Welche meiner Aktien haben starkes Momentum?',
  'Suche Kaufgelegenheiten im NASDAQ 100',
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
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
              className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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

export function ChatPanel({ isOpen, onClose, overlay = false }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessageData[]>([]);
  const [isLoadingHistory, setIsLoadingHistory] = useState(true);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastFailedInput, setLastFailedInput] = useState<string | null>(null);
  const [suggestions, setSuggestions] = useState<SuggestedAction[]>([]);
  const [executingSuggestion, setExecutingSuggestion] = useState<string | null>(null);
  const [importingTransactions, setImportingTransactions] = useState<string | null>(null);
  const [panelWidth, setPanelWidth] = useState(() => {
    const saved = localStorage.getItem(STORAGE_KEY_WIDTH);
    return saved ? Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, parseInt(saved, 10))) : DEFAULT_WIDTH;
  });
  const [isResizing, setIsResizing] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(() => {
    return localStorage.getItem(STORAGE_KEY_COLLAPSED) === 'true';
  });
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [lastResponseCost, setLastResponseCost] = useState<string>('');
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

  // Speech-to-text state
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  // PDF import modal access
  const { openPdfImportModal } = useUIStore();

  // Conversation state
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<number | null>(null);
  const [isFirstMessage, setIsFirstMessage] = useState(true);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Portfolio state for extracted transactions
  const [portfolios, setPortfolios] = useState<Portfolio[]>([]);

  // Query approval state (security feature)
  // SECURITY: Tracks pending queries that need user approval before execution
  const [pendingQueries, setPendingQueries] = useState<Array<PendingQuery & { messageId: number; id?: number; status?: 'pending' | 'confirmed' | 'declined' }>>([]);
  const [executingQuery, setExecutingQuery] = useState<string | null>(null);

  const {
    aiFeatureSettings,
    baseCurrency,
    language,
    alphaVantageApiKey,
    userName,
    chatContextSize,
    deliveryMode,
    sqlApprovalMode,
    sqlErrorHandling,
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
      case 'openrouter':
        return keys.openrouterApiKey;
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
        const result = await invoke<boolean>('check_vision_support', { model: aiModel, provider: aiProvider });
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
  }, [aiModel, aiProvider, attachments.length]);

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

  // Ref for stable access to messages in async sendMessage
  const messagesRef = useRef<ChatMessageData[]>(messages);
  useEffect(() => { messagesRef.current = messages; }, [messages]);

  // Refs for stable access in Tauri drag-drop handler
  const hasVisionSupportRef = useRef(hasVisionSupport);
  const imageConsentGivenRef = useRef(imageConsentGiven);
  const openPdfImportModalRef = useRef(openPdfImportModal);
  const onCloseRef = useRef(onClose);

  // Keep refs in sync
  useEffect(() => {
    hasVisionSupportRef.current = hasVisionSupport;
  }, [hasVisionSupport]);
  useEffect(() => {
    imageConsentGivenRef.current = imageConsentGiven;
  }, [imageConsentGiven]);
  useEffect(() => {
    openPdfImportModalRef.current = openPdfImportModal;
  }, [openPdfImportModal]);
  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  // Tauri native drag-drop handler (external files)
  // Uses refs for mutable values to avoid re-registering listener on every state change
  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const window = getCurrentWindow();
    let unlistenFn: (() => void) | null = null;
    let aborted = false;

    const setupListener = async () => {
      try {
        const unlisten = await window.onDragDropEvent(async (event) => {
          if (aborted) return;

          // Handle enter for visual feedback (external files from Finder)
          if (event.payload.type === 'enter') {
            setIsDragging(true);
            return;
          }

          // Handle leave
          if (event.payload.type === 'leave') {
            setIsDragging(false);
            return;
          }

          if (event.payload.type === 'drop') {
            setIsDragging(false); // Reset drag state on drop

            if (event.payload.paths.length === 0) {
              return;
            }

            // Check for PDF files first - open import modal directly
            const pdfPaths = event.payload.paths.filter((path: string) => {
              const ext = path.split('.').pop()?.toLowerCase() || '';
              return PDF_EXTENSIONS.includes(ext);
            });

            if (pdfPaths.length > 0) {
              openPdfImportModalRef.current(pdfPaths[0]);
              onCloseRef.current(); // Close chat panel
              return;
            }

            // Filter for image files by extension
            const imagePaths = event.payload.paths.filter((path: string) => {
              const ext = path.split('.').pop()?.toLowerCase() || '';
              return IMAGE_EXTENSIONS.includes(ext);
            });

            if (imagePaths.length === 0) {
              return;
            }

            if (!hasVisionSupportRef.current) {
              setError('Das aktuelle Modell unterstützt keine Bilder. Bitte wähle ein Vision-fähiges Modell.');
              return;
            }

            // Check consent first
            if (!imageConsentGivenRef.current) {
              // Read and prepare attachments, but show consent first
              const pendingAttachments: ChatImageAttachment[] = [];
              for (const path of imagePaths) {
                try {
                  // Use Tauri command (security: validated path, returns base64 directly)
                  const result = await invoke<FileBase64Result>('read_image_as_base64', { path });
                  pendingAttachments.push({
                    data: result.data,
                    mimeType: result.mimeType,
                    filename: result.filename,
                  });
                } catch (err) {
                  console.error('Failed to read dropped file:', path, err);
                }
              }
              if (pendingAttachments.length > 0) {
                // Store attachments directly instead of File objects
                // After consent, we'll add them directly without re-processing
                setPendingImageUpload(pendingAttachments as unknown as File[]);
                setShowImageConsent(true);
              }
              return;
            }

            // Process dropped files
            setError(null);
            const newAttachments: ChatImageAttachment[] = [];

            for (const path of imagePaths) {
              try {
                // Use Tauri command instead of fs plugin (security: validated path)
                const result = await invoke<FileBase64Result>('read_image_as_base64', { path });

                newAttachments.push({
                  data: result.data,
                  mimeType: result.mimeType,
                  filename: result.filename,
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

        if (!aborted) {
          unlistenFn = unlisten;
        } else {
          // Already cleaned up, unlisten immediately
          unlisten();
        }
      } catch (err) {
        console.error('[ChatPanel D&D] Failed to register listener:', err);
      }
    };

    setupListener();

    return () => {
      aborted = true;
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [isOpen]); // Only depends on isOpen - uses refs for other values

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
      const pending = pendingImageUpload;
      setPendingImageUpload(null);
      setError(null);

      // Check if these are already ChatImageAttachment objects (from Tauri D&D)
      // or File objects (from browser D&D / paste)
      if (pending.length > 0 && 'mimeType' in pending[0]) {
        // Already processed attachments from Tauri D&D - add directly
        setAttachments((prev) => [...prev, ...(pending as unknown as ChatImageAttachment[])]);
      } else {
        // File objects from browser - process them
        const newAttachments: ChatImageAttachment[] = [];
        for (const file of pending as File[]) {
          const attachment = await processImageFile(file);
          if (attachment) {
            newAttachments.push(attachment);
          }
        }
        if (newAttachments.length > 0) {
          setAttachments((prev) => [...prev, ...newAttachments]);
        }
      }
    }
  };

  const handleImageConsentCancel = () => {
    setShowImageConsent(false);
    setPendingImageUpload(null);
  };

  // ============================================================================
  // Speech-to-Text (Whisper) - Only available when ChatBot uses OpenAI
  // ============================================================================

  const isOpenAiProvider = aiProvider === 'openai';
  const hasSpeechToText = isOpenAiProvider && !!keys.openaiApiKey;

  // Classic oscilloscope waveform visualization
  const drawWaveform = useCallback(() => {
    const canvas = canvasRef.current;
    const analyser = analyserRef.current;
    if (!canvas || !analyser) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Ensure canvas is properly sized for HiDPI
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    if (rect.width > 0 && rect.height > 0) {
      if (canvas.width !== rect.width * dpr || canvas.height !== rect.height * dpr) {
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
      }
    }

    const bufferLength = analyser.fftSize;
    const dataArray = new Uint8Array(bufferLength);
    analyser.getByteTimeDomainData(dataArray);

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Get CSS variable for primary color (fallback to green for oldschool look)
    const computedStyle = getComputedStyle(document.documentElement);
    const primaryHsl = computedStyle.getPropertyValue('--primary').trim();
    const strokeColor = primaryHsl ? `hsl(${primaryHsl})` : '#22c55e';

    // Draw waveform line
    ctx.lineWidth = 1.5 * dpr;
    ctx.strokeStyle = strokeColor;
    ctx.beginPath();

    const sliceWidth = width / bufferLength;
    let x = 0;

    for (let i = 0; i < bufferLength; i++) {
      // Normalize to -1 to 1 (128 is center/silence)
      const v = (dataArray[i] - 128) / 128.0;
      // Amplify the signal for more visible waveform
      const gain = 3.0;
      const amplified = Math.max(-1, Math.min(1, v * gain)); // Clamp to prevent overflow
      const y = height / 2 - amplified * (height / 2) * 0.9; // 0.9 = margin from edges

      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }

      x += sliceWidth;
    }

    ctx.lineTo(width, height / 2);
    ctx.stroke();

    // Draw center line (subtle)
    ctx.strokeStyle = strokeColor;
    ctx.globalAlpha = 0.2;
    ctx.lineWidth = 1 * dpr;
    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();
    ctx.globalAlpha = 1;

    animationFrameRef.current = requestAnimationFrame(drawWaveform);
  }, []);

  // Start waveform animation when recording begins and canvas is ready
  useEffect(() => {
    if (isRecording && analyserRef.current) {
      // Use requestAnimationFrame to ensure canvas is rendered
      const frameId = requestAnimationFrame(() => {
        if (canvasRef.current) {
          drawWaveform();
        }
      });
      return () => cancelAnimationFrame(frameId);
    }
  }, [isRecording, drawWaveform]);

  const startRecording = async () => {
    if (!hasSpeechToText) {
      toast.error('OpenAI API-Key erforderlich für Spracheingabe. Bitte in den Einstellungen konfigurieren.');
      return;
    }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mediaRecorder = new MediaRecorder(stream, { mimeType: 'audio/webm' });

      // Set up audio visualization
      const audioContext = new AudioContext();
      const source = audioContext.createMediaStreamSource(stream);
      const analyser = audioContext.createAnalyser();
      analyser.fftSize = 256;
      source.connect(analyser);

      audioContextRef.current = audioContext;
      analyserRef.current = analyser;
      audioChunksRef.current = [];

      mediaRecorder.ondataavailable = (event) => {
        if (event.data.size > 0) {
          audioChunksRef.current.push(event.data);
        }
      };

      mediaRecorder.onstop = async () => {
        // Stop visualization
        if (animationFrameRef.current) {
          cancelAnimationFrame(animationFrameRef.current);
          animationFrameRef.current = null;
        }
        if (audioContextRef.current) {
          audioContextRef.current.close();
          audioContextRef.current = null;
        }
        analyserRef.current = null;

        // Stop all tracks to release the microphone
        stream.getTracks().forEach(track => track.stop());

        if (audioChunksRef.current.length === 0) {
          return;
        }

        setIsTranscribing(true);

        try {
          // Convert audio chunks to base64
          const audioBlob = new Blob(audioChunksRef.current, { type: 'audio/webm' });
          const arrayBuffer = await audioBlob.arrayBuffer();
          const base64 = btoa(
            new Uint8Array(arrayBuffer).reduce((data, byte) => data + String.fromCharCode(byte), '')
          );

          // Call Whisper API via Tauri
          const transcribedText = await invoke<string>('transcribe_audio', {
            audioBase64: base64,
            apiKey: keys.openaiApiKey,
            language: 'de', // German as default, could be made configurable
          });

          // Send transcribed text directly as message
          if (transcribedText.trim()) {
            sendMessage(transcribedText.trim());
          }
        } catch (err) {
          console.error('Transcription error:', err);
          toast.error(`Transkription fehlgeschlagen: ${err}`);
        } finally {
          setIsTranscribing(false);
        }
      };

      mediaRecorderRef.current = mediaRecorder;
      mediaRecorder.start();
      setIsRecording(true);
      // Animation is started by useEffect when isRecording becomes true
    } catch (err) {
      console.error('Failed to start recording:', err);
      toast.error('Mikrofon-Zugriff fehlgeschlagen. Bitte Berechtigungen prüfen.');
    }
  };

  const stopRecording = () => {
    if (mediaRecorderRef.current && isRecording) {
      mediaRecorderRef.current.stop();
      setIsRecording(false);
    }
  };

  const toggleRecording = () => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
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
        // Read files and convert to attachments using Tauri command
        const files: File[] = [];
        for (const path of selected) {
          try {
            const result = await invoke<FileBase64Result>('read_image_as_base64', { path });
            // Convert base64 back to blob for File object
            const binaryString = atob(result.data);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
              bytes[i] = binaryString.charCodeAt(i);
            }
            const blob = new Blob([bytes], { type: result.mimeType });
            const file = new File([blob], result.filename, { type: result.mimeType });
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
    // Note: No stopPropagation() - allows Tauri native D&D to work alongside React
    setIsDragging(false);

    // Tauri native D&D handles actual file drops from Finder
    // React handler is only for browser-based D&D (e.g. images from websites)
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
    // Note: No stopPropagation() - allows Tauri events to pass through
  };

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    // Note: No stopPropagation() - Tauri handler sets isDragging for external files
    // Only show drag indicator if dragging files (not text) - for browser D&D
    if (e.dataTransfer.types.includes('Files')) {
      setIsDragging(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    // Note: No stopPropagation() - allows Tauri events to pass through
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
        const loadedSuggestions: SuggestedAction[] = [];
        const loadedPendingQueries: Array<PendingQuery & { messageId: number; id?: number; status?: 'pending' | 'confirmed' | 'declined' }> = [];

        for (const s of dbSuggestions) {
          if (s.actionType === 'sql_query') {
            loadedPendingQueries.push({
              queryType: 'sql_query',
              description: s.description,
              payload: s.payload,
              messageId: s.messageId,
              id: s.id,
              status: s.status as 'pending' | 'confirmed' | 'declined',
            });
          } else {
            loadedSuggestions.push({
              id: s.id,
              messageId: s.messageId,
              actionType: s.actionType,
              description: s.description,
              payload: s.payload,
              status: s.status as 'pending' | 'confirmed' | 'declined',
            });
          }
        }

        setSuggestions(loadedSuggestions);
        setPendingQueries(loadedPendingQueries);

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

  // Scroll to bottom when new messages or suggestions arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, suggestions, pendingQueries]);

  // Focus input and scroll to bottom when panel opens
  useEffect(() => {
    if (isOpen) {
      setTimeout(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: 'instant' });
        inputRef.current?.focus();
      }, 100);
    }
  }, [isOpen]);

  // Save width to localStorage
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_WIDTH, String(panelWidth));
  }, [panelWidth]);

  // Save collapsed state to localStorage
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_COLLAPSED, String(isCollapsed));
  }, [isCollapsed]);

  const toggleCollapsed = useCallback(() => {
    setIsCollapsed(prev => !prev);
  }, []);

  // Keyboard shortcut: Cmd+. to toggle collapse
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isFullscreen) {
        setIsFullscreen(false);
        return;
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '.' && !isFullscreen) {
        e.preventDefault();
        toggleCollapsed();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [toggleCollapsed, isFullscreen]);

  // Handle resize
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    if (isCollapsed) return; // Don't resize when collapsed
    setIsResizing(true);
  }, [isCollapsed]);

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

  const sendMessage = async (content: string, overrideAttachments?: ChatImageAttachment[]) => {
    // Allow sending with just attachments (no text required if images present)
    const hasContent = content.trim().length > 0;
    // Use override attachments if provided, otherwise use state
    const effectiveAttachments = overrideAttachments ?? attachments;
    const hasAttachments = effectiveAttachments.length > 0;

    if ((!hasContent && !hasAttachments) || isLoading || !currentConversationId) return;

    const trimmedContent = content.trim();
    const currentAttachments = [...effectiveAttachments]; // Copy for sending

    setInput('');
    if (!overrideAttachments) {
      setAttachments([]); // Clear attachments only if not using override
    }
    setIsLoading(true);
    setError(null);
    setLastFailedInput(null);

    try {
      // Build user message content (include attachment indicator)
      const hasPdf = currentAttachments.some(a => a.mimeType === 'application/pdf');
      const attachmentLabel = hasPdf
        ? `${currentAttachments.length} PDF${currentAttachments.length > 1 ? 's' : ''}`
        : `${currentAttachments.length} Bild${currentAttachments.length > 1 ? 'er' : ''}`;
      const displayContent = hasAttachments && !hasContent
        ? `[${attachmentLabel} gesendet]`
        : hasAttachments
        ? `${trimmedContent}\n\n[${attachmentLabel} angehängt]`
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

      // Build message history for API with sliding window (use ref for latest state)
      const allMessages = [...messagesRef.current, userMessage];
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
          language: language || 'de',
          sqlApprovalMode: sqlApprovalMode || 'session',
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

      // Calculate and display cost
      calculateAiCost(response.provider, response.model, response.inputTokens, response.outputTokens, response.tokensUsed, baseCurrency)
        .then(c => {
          if (c?.totalTokens) {
            const parts = [`${c.totalTokens.toLocaleString()} Tokens`];
            if (c.costDisplay) parts.push(c.costDisplay);
            setLastResponseCost(parts.join(' · '));
          }
        });

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
            // For extracted_transactions: check for duplicates BEFORE showing preview
            if (suggestion.actionType === 'extracted_transactions') {
              let payload: ExtractedTransactionsPayload;
              try {
                payload = JSON.parse(suggestion.payload);
              } catch {
                console.error('Failed to parse extracted transactions payload');
                continue;
              }

              // Check all transactions for duplicates
              const duplicateCheck = await invoke<DuplicateCheckResponse>(
                'check_extracted_transactions_for_duplicates',
                {
                  transactions: payload.transactions.map((t) => ({
                    date: t.date,
                    txn_type: t.txnType,
                    security_name: t.securityName || null,
                    isin: t.isin || null,
                    shares: t.shares ?? null,
                    gross_amount: t.grossAmount ?? null,
                    gross_currency: t.grossCurrency || null,
                    amount: t.amount ?? null,
                    currency: t.currency,
                    fees: t.fees ?? null,
                    fees_foreign: t.feesForeign ?? null,
                    fees_foreign_currency: t.feesForeignCurrency || null,
                    exchange_rate: t.exchangeRate ?? null,
                    taxes: t.taxes ?? null,
                    note: t.note || null,
                  })),
                }
              );

              // If ALL transactions are duplicates, show message instead of preview
              if (duplicateCheck.allDuplicates) {
                const duplicateMessages = duplicateCheck.results
                  .filter((r) => r.isDuplicate && r.message)
                  .map((r) => r.message);

                const duplicateContent =
                  duplicateCheck.duplicateCount === 1
                    ? `🔄 Diese Transaktion ist bereits vorhanden:\n• ${duplicateMessages[0]}`
                    : `🔄 Alle ${duplicateCheck.duplicateCount} Transaktionen sind bereits vorhanden:\n${duplicateMessages.map((m) => `• ${m}`).join('\n')}`;

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
                    isDuplicate: true,
                  },
                ]);

                // Don't save this suggestion - all duplicates
                continue;
              }

              // If SOME transactions are duplicates, filter them out
              if (duplicateCheck.duplicateCount > 0) {
                const nonDuplicateIndices = duplicateCheck.results
                  .filter((r) => !r.isDuplicate)
                  .map((r) => r.index);
                const filteredTransactions = payload.transactions.filter((_, idx) =>
                  nonDuplicateIndices.includes(idx)
                );

                // Show message about skipped duplicates
                const duplicateMessages = duplicateCheck.results
                  .filter((r) => r.isDuplicate && r.message)
                  .map((r) => r.message);

                const partialDupContent = `🔄 ${duplicateCheck.duplicateCount === 1 ? 'Duplikat' : 'Duplikate'} übersprungen:\n${duplicateMessages.map((m) => `• ${m}`).join('\n')}`;

                const partialDupMsgId = await invoke<number>('save_chat_message', {
                  role: 'assistant',
                  content: partialDupContent,
                  conversationId: currentConversationId,
                });

                setMessages((prev) => [
                  ...prev,
                  {
                    id: String(partialDupMsgId),
                    role: 'assistant',
                    content: partialDupContent,
                    timestamp: new Date(),
                    isDuplicate: true,
                  },
                ]);

                // Update payload with filtered transactions
                const filteredPayload: ExtractedTransactionsPayload = {
                  ...payload,
                  transactions: filteredTransactions,
                };
                suggestion.payload = JSON.stringify(filteredPayload);
                suggestion.description = `${filteredTransactions.length} Transaktion${filteredTransactions.length !== 1 ? 'en' : ''} aus Bild erkannt`;
              }
            }

            // Pre-validate watchlist_remove: check if security is actually in watchlist
            if (suggestion.actionType === 'watchlist_remove') {
              try {
                const cmd = JSON.parse(suggestion.payload);
                const watchlists = await invoke<Array<{ id: number; name: string; securities: Array<{ name: string; ticker: string | null }> }>>('ai_list_watchlists');
                const wl = watchlists.find(w => w.name.toLowerCase() === (cmd.watchlist || '').toLowerCase());
                if (wl) {
                  const query = (cmd.security || '').toLowerCase().split('(')[0].trim();
                  const found = wl.securities.some(s =>
                    s.name.toLowerCase().includes(query) ||
                    (s.ticker && s.ticker.toLowerCase().includes(query))
                  );
                  if (!found) {
                    // Security not in watchlist — skip suggestion, show info message
                    const infoContent = `"${cmd.security}" ist nicht in der Watchlist "${cmd.watchlist}".`;
                    const infoMsgId = await invoke<number>('save_chat_message', {
                      role: 'assistant',
                      content: infoContent,
                      conversationId: currentConversationId,
                    });
                    setMessages(prev => [...prev, {
                      id: String(infoMsgId),
                      role: 'assistant',
                      content: infoContent,
                      timestamp: new Date(),
                    }]);
                    continue;
                  }
                }
              } catch (checkErr) {
                console.warn('Watchlist pre-check failed, showing suggestion anyway:', checkErr);
              }
            }

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
            // Still show the suggestion in UI even if DB save failed
            savedSuggestions.push({
              ...suggestion,
              messageId: Number(assistantMsgId),
              status: 'pending',
            });
          }
        }
        setSuggestions((prev) => [...prev, ...savedSuggestions]);
      }

      // SECURITY: Handle pending queries that need user approval
      // These are database queries that were NOT auto-executed because approval wasn't given yet
      if (response.pendingQueries && response.pendingQueries.length > 0) {
        const savedQueries: Array<PendingQuery & { messageId: number; id?: number; status?: 'pending' | 'confirmed' | 'declined' }> = [];
        for (const q of response.pendingQueries) {
          try {
            const queryId = await invoke<number>('save_chat_suggestion', {
              messageId: Number(assistantMsgId),
              conversationId: currentConversationId,
              actionType: 'sql_query',
              description: q.description,
              payload: q.payload,
            });
            savedQueries.push({
              ...q,
              messageId: Number(assistantMsgId),
              id: queryId,
              status: 'pending',
            });
          } catch (err) {
            console.error('Failed to save pending query:', err);
            savedQueries.push({
              ...q,
              messageId: Number(assistantMsgId),
            });
          }
        }
        setPendingQueries((prev) => [...prev, ...savedQueries]);
      }

      // Auto-retry SQL errors: when sqlErrorHandling is 'auto_retry' and the response
      // contains SQL errors, automatically send a follow-up to let the AI correct itself
      const hasSqlError = response.response.includes('SQL-Fehler:');
      if (hasSqlError && sqlErrorHandling === 'auto_retry') {
        // Extract the error for context
        const errorMatch = response.response.match(/SQL-Fehler:\s*(.+)/);
        const errorDetail = errorMatch ? errorMatch[1].trim() : 'unbekannter Fehler';
        // Schedule a follow-up message (non-blocking, after current render)
        setTimeout(() => {
          sendMessage(`Die SQL-Abfrage hatte einen Fehler: "${errorDetail}". Bitte korrigiere die Abfrage und versuche es erneut.`);
        }, 500);
      }

      // UX: Warn user if image was sent but no transactions were extracted
      const hasImageAttachments = currentAttachments.some(a => a.mimeType.startsWith('image/'));
      const hasExtractedTransactions = response.suggestions?.some(s => s.actionType === 'extracted_transactions');
      if (hasImageAttachments && !hasExtractedTransactions && response.response.length < 200) {
        toast.warning('Keine Transaktionen im Bild erkannt. Versuche ein klareres Bild oder beschreibe den Inhalt.');
      }
    } catch (err) {
      const errorMessage = typeof err === 'string' ? err : String(err);

      // Try to parse structured error
      let displayError = errorMessage;
      try {
        const parsed = JSON.parse(errorMessage);
        displayError = parsed.message || errorMessage;
      } catch {
        // Keep original errorMessage
      }

      // Show toast for API errors
      toast.error(displayError);
      setError(displayError);
      setLastFailedInput(trimmedContent);
      // Restore input so user can edit and retry (not just blind-retry)
      setInput(trimmedContent);
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

  // Consume pending chat analysis from Screener Analyse button
  const pendingChatAnalysis = useUIStore((s) => s.pendingChatAnalysis);
  const clearPendingChatAnalysis = useUIStore((s) => s.clearPendingChatAnalysis);
  useEffect(() => {
    if (pendingChatAnalysis && !isLoading && currentConversationId) {
      const { message } = pendingChatAnalysis;
      clearPendingChatAnalysis();
      sendMessage(message);
    }
  }, [pendingChatAnalysis, isLoading, currentConversationId]); // eslint-disable-line react-hooks/exhaustive-deps

  // Create a new conversation
  const handleNewConversation = async () => {
    try {
      // If current conversation is empty (no messages), just reset state instead of creating a new one
      const currentConv = conversations.find((c) => c.id === currentConversationId);
      if (currentConv && currentConv.messageCount === 0) {
        setMessages([]);
        setSuggestions([]);
        setPendingQueries([]);
        setIsFirstMessage(true);
        setError(null);
        return;
      }

      const newConv = await invoke<Conversation>('create_conversation', { title: null });
      // Clean up any empty conversations (except the new one)
      const emptyConvIds = conversations
        .filter((c) => c.messageCount === 0 && c.id !== currentConversationId)
        .map((c) => c.id);
      for (const id of emptyConvIds) {
        invoke('delete_conversation', { id }).catch(() => {});
      }

      setConversations((prev) => [
        newConv,
        ...prev.filter((c) => !emptyConvIds.includes(c.id)),
      ]);
      setCurrentConversationId(newConv.id);
      setMessages([]);
      setSuggestions([]);
      setPendingQueries([]);
      setIsFirstMessage(true);
      setError(null);
    } catch (err) {
      console.error('Failed to create conversation:', err);
      setError('Fehler beim Erstellen der Conversation');
    }
  };

  // Switch to a different conversation
  const switchConversation = async (convId: number) => {
    if (convId !== currentConversationId) {
      setCurrentConversationId(convId);
      setError(null);
      // Refresh conversations list from DB to get accurate message counts
      try {
        const convs = await invoke<Conversation[]>('get_conversations');
        setConversations(convs);
      } catch (err) {
        console.error('Failed to refresh conversations:', err);
      }
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
      } else if (suggestion.actionType === 'show_chart') {
        try {
          const payload = JSON.parse(suggestion.payload);
          useUIStore.getState().showChartFromChat(payload.securityName, payload.timeRange, payload.indicators);
          useUIStore.getState().setCurrentView('charts');
          result = `Technische Analyse für "${payload.securityName}" geöffnet`;
        } catch {
          result = 'Fehler beim Öffnen der technischen Analyse';
        }
      } else if (suggestion.actionType === 'screener_run') {
        try {
          const payload = JSON.parse(suggestion.payload);
          const filters = (payload.filters || []).map((f: { indicator: string; condition: string; value: number; value2?: number }, i: number) => ({
            ...f,
            id: `chat-${i}-${Date.now()}`,
            enabled: true,
          }));
          useUIStore.getState().applyScreenerFromChat(
            filters,
            payload.mode || 'market',
            payload.indexId
          );
          useUIStore.getState().setCurrentView('screener');
          result = `Screener geöffnet mit ${filters.length} Filter(n)`;
        } catch {
          result = 'Fehler beim Öffnen des Screeners';
        }
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
        prev.map((s) => ((s.id != null && suggestion.id != null ? s.id === suggestion.id : s.payload === suggestion.payload) ? { ...s, status: 'confirmed' as const } : s))
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
      prev.map((s) => ((s.id != null && suggestion.id != null ? s.id === suggestion.id : s.payload === suggestion.payload) ? { ...s, status: 'declined' as const } : s))
    );
  };

  // ============================================================================
  // Query Approval Handlers (Security Feature)
  // ============================================================================

  /**
   * Execute a pending query once without session approval
   * SECURITY: Query is executed but not approved for future use
   */
  const handleApproveQueryOnce = async (query: PendingQuery & { messageId: number; id?: number }) => {
    if (!currentConversationId) return;

    setExecutingQuery(query.payload);
    try {
      // Execute the query without approving the type
      const results = await invoke<string[]>('execute_pending_query', {
        queryType: query.queryType,
        payload: query.payload,
      });

      // Remove this query from pending
      setPendingQueries((prev) => prev.filter((q) => q.payload !== query.payload));
      if (query.id) {
        await invoke('update_suggestion_status', { id: query.id, status: 'confirmed' });
      }

      // Show query results as a new message
      if (results.length > 0) {
        const resultContent = results.join('\n\n');
        const msgId = await invoke<number>('save_chat_message', {
          role: 'assistant',
          content: resultContent,
          conversationId: currentConversationId,
        });

        setMessages((prev) => [
          ...prev,
          {
            id: String(msgId),
            role: 'assistant',
            content: resultContent,
            timestamp: new Date(),
          },
        ]);
      }
    } catch (err) {
      console.error('Failed to execute query:', err);
      setError(typeof err === 'string' ? err : String(err));
    } finally {
      setExecutingQuery(null);
    }
  };

  /**
   * Approve query type for the entire session and execute
   * SECURITY: All queries of this type will be auto-approved until app restart
   * NOTE: This uses execute_sql_query with approve_pattern=true to register the SQL pattern
   * for automatic approval of similar queries in the future.
   */
  const handleApproveQuerySession = async (query: PendingQuery & { messageId: number; id?: number }) => {
    if (!currentConversationId) return;

    setExecutingQuery(query.payload);
    try {
      // Execute all pending queries of this type with pattern approval
      const sametype = pendingQueries.filter((q) => q.queryType === query.queryType);
      const allResults: string[] = [];

      for (const q of sametype) {
        // Parse the SQL from the payload and execute with pattern approval
        // This registers the SQL pattern so similar queries are auto-approved
        const sqlQuery = JSON.parse(q.payload) as { sql: string; description: string };
        const result = await invoke<string>('execute_sql_query', {
          sql: sqlQuery.sql,
          approvePattern: true, // This is the key: approve the pattern for future queries
        });
        allResults.push(result);
        if (q.id) {
          await invoke('update_suggestion_status', { id: q.id, status: 'confirmed' });
        }
      }

      // Remove all queries of this type from pending
      setPendingQueries((prev) => prev.filter((q) => q.queryType !== query.queryType));

      // Show query results as a new message
      if (allResults.length > 0) {
        const resultContent = allResults.join('\n\n');
        const msgId = await invoke<number>('save_chat_message', {
          role: 'assistant',
          content: resultContent,
          conversationId: currentConversationId,
        });

        setMessages((prev) => [
          ...prev,
          {
            id: String(msgId),
            role: 'assistant',
            content: resultContent,
            timestamp: new Date(),
          },
        ]);
      }
    } catch (err) {
      console.error('Failed to approve and execute query:', err);
      setError(typeof err === 'string' ? err : String(err));
    } finally {
      setExecutingQuery(null);
    }
  };

  /**
   * Decline a pending query
   * SECURITY: Query is removed without exposing any data
   */
  const handleDeclineQuery = async (query: PendingQuery & { messageId: number; id?: number }) => {
    setPendingQueries((prev) => prev.filter((q) => q.payload !== query.payload));
    if (query.id) {
      try {
        await invoke('update_suggestion_status', { id: query.id, status: 'declined' });
      } catch (err) {
        console.error('Failed to update pending query status:', err);
      }
    }
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
          shares: t.shares ?? null,
          gross_amount: t.grossAmount ?? null,
          gross_currency: t.grossCurrency || null,
          amount: t.amount ?? null,
          currency: t.currency,
          fees: t.fees ?? null,
          fees_foreign: t.feesForeign ?? null,
          fees_foreign_currency: t.feesForeignCurrency || null,
          exchange_rate: t.exchangeRate ?? null,
          taxes: t.taxes ?? null,
          note: t.note || null,
        })),
        portfolioId: portfolioId,
        deliveryMode: deliveryMode,
      });

      // 1. Show success message ONLY if transactions were imported
      if (result.importedCount > 0) {
        const successContent = `✓ ${result.importedCount} Transaktion${result.importedCount === 1 ? '' : 'en'} erfolgreich importiert`;
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

      // 2. Show duplicates as separate chat message with special styling
      if (result.duplicates.length > 0) {
        const duplicateContent = `🔄 ${result.duplicates.length === 1 ? 'Duplikat' : 'Duplikate'} übersprungen:\n${result.duplicates.map(d => `• ${d}`).join('\n')}`;
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
            isDuplicate: true, // Mark as duplicate for special styling (amber/orange border)
          },
        ]);
      }

      // 3. Show errors as separate chat message with special styling
      if (result.errors.length > 0) {
        const errorContent = `⚠️ ${result.errors.length === 1 ? 'Fehler' : 'Fehler'} beim Import:\n${result.errors.map(e => `• ${e}`).join('\n')}`;
        const errMsgId = await invoke<number>('save_chat_message', {
          role: 'assistant',
          content: errorContent,
          conversationId: currentConversationId,
        });

        setMessages((prev) => [
          ...prev,
          {
            id: String(errMsgId),
            role: 'assistant',
            content: errorContent,
            timestamp: new Date(),
            isError: true, // Mark as error for special styling (red border)
          },
        ]);
      }

      // 4. If nothing happened at all, show info message
      if (result.importedCount === 0 && result.duplicates.length === 0 && result.errors.length === 0) {
        const infoContent = 'ℹ️ Keine Transaktionen zum Importieren gefunden.';
        const infoMsgId = await invoke<number>('save_chat_message', {
          role: 'assistant',
          content: infoContent,
          conversationId: currentConversationId,
        });

        setMessages((prev) => [
          ...prev,
          {
            id: String(infoMsgId),
            role: 'assistant',
            content: infoContent,
            timestamp: new Date(),
          },
        ]);
      }

      // Update suggestion status
      if (suggestion.id) {
        await invoke('update_suggestion_status', { id: suggestion.id, status: 'confirmed' });
      }
      setSuggestions((prev) =>
        prev.map((s) => ((s.id != null && suggestion.id != null ? s.id === suggestion.id : s.payload === suggestion.payload) ? { ...s, status: 'confirmed' as const } : s))
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
      prev.map((s) => ((s.id != null && suggestion.id != null ? s.id === suggestion.id : s.payload === suggestion.payload) ? { ...s, status: 'declined' as const } : s))
    );
  };

  if (!isOpen) return null;

  return (
    <>
      {/* Panel — flex flow or fixed overlay depending on mode */}
      <div
        ref={panelRef}
        style={isFullscreen ? {} : {
          width: isCollapsed ? COLLAPSED_WIDTH : panelWidth,
          ...(overlay ? {} : { flexBasis: isCollapsed ? COLLAPSED_WIDTH : panelWidth }),
        }}
        className={cn(
          'relative transition-[width,flex-basis] duration-200',
          'bg-background shadow-xl',
          'flex flex-col',
          isFullscreen
            ? 'fixed inset-0 z-50 border-0'
            : cn(
                'h-full border-l border-border',
                overlay ? 'fixed right-0 top-0 z-50' : 'shrink-0',
              ),
          isResizing && 'select-none !transition-none'
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

        {/* Resize Handle — only when expanded and not fullscreen */}
        {!isCollapsed && !isFullscreen && (
          <div
            onMouseDown={handleMouseDown}
            onDoubleClick={toggleCollapsed}
            className={cn(
              'absolute -left-2 top-0 bottom-0 w-4 cursor-ew-resize z-10',
              'hover:bg-primary/20 active:bg-primary/30 transition-colors',
              'group flex items-center justify-center',
              isResizing && 'bg-primary/30'
            )}
          >
            <GripVertical className="h-6 w-3 text-muted-foreground/50 group-hover:text-primary/70" />
          </div>
        )}

        {/* Collapsed sidebar strip */}
        {isCollapsed && !isFullscreen ? (
          <div className="flex flex-col items-center h-full py-3 gap-3">
            <button
              onClick={toggleCollapsed}
              className="p-2 rounded hover:bg-muted transition-colors"
              title="Chat aufklappen (⌘.)"
            >
              <PanelRightOpen className="h-5 w-5 text-muted-foreground" />
            </button>
            <button
              onClick={toggleCollapsed}
              className="p-2 rounded hover:bg-muted transition-colors"
              title="Chat aufklappen"
            >
              <MessageSquare className="h-5 w-5 text-muted-foreground" />
            </button>
            {messages.length > 0 && (
              <span className="text-[10px] text-muted-foreground/60">{messages.length}</span>
            )}
          </div>
        ) : (
        <>
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border">
          <div className="flex items-center gap-2 min-w-0 overflow-hidden">
            {/* New Conversation Button */}
            <button
              onClick={handleNewConversation}
              className="p-1.5 rounded hover:bg-muted transition-colors shrink-0"
              title="Neuer Chat"
            >
              <Plus className="h-4 w-4" />
            </button>

            {/* Conversation Dropdown */}
            <div className="min-w-0 flex-1">
              <DropdownMenu
                trigger={
                  <span className="truncate block max-w-[140px]">
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
              onClick={() => { if (!isFullscreen) setIsCollapsed(false); setIsFullscreen(!isFullscreen); }}
              className="p-2 rounded hover:bg-muted transition-colors"
              title={isFullscreen ? 'Vollbild beenden' : 'Vollbild'}
            >
              {isFullscreen ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
            </button>
            {!isFullscreen && (
              <button
                onClick={toggleCollapsed}
                className="p-2 rounded hover:bg-muted transition-colors"
                title="Einklappen (⌘.)"
              >
                <PanelRightClose className="h-4 w-4" />
              </button>
            )}
            <button
              onClick={() => { setIsFullscreen(false); onClose(); }}
              className="p-2 rounded hover:bg-muted transition-colors"
              title="Chat schließen"
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
                    className="block w-full text-left px-3 py-2 text-sm rounded-lg bg-muted/50 hover:bg-muted transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {question}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            messages.map((message) => {
              // Get suggestions for this message (if it's an assistant message)
              const messageSuggestions = message.role === 'assistant'
                ? suggestions.filter((s) => s.messageId === Number(message.id))
                : [];

              // Get pending queries for this message (security feature)
              const messagePendingQueries = message.role === 'assistant'
                ? pendingQueries.filter((q) => q.messageId === Number(message.id))
                : [];

              return (
                <div key={message.id} className="space-y-3">
                  <ChatMessage message={message} onDelete={deleteMessage} />

                  {/* Render suggestions inline after their associated message */}
                  {messageSuggestions.map((suggestion, idx) => {
                    // Transaction create/transfer suggestions
                    if (suggestion.actionType === 'transaction_create' || suggestion.actionType === 'portfolio_transfer') {
                      return (
                        <div key={`txn-${message.id}-${idx}`} className={cn(suggestion.status !== 'pending' && 'opacity-60')}>
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
                      );
                    }

                    // Delete suggestions
                    if (suggestion.actionType === 'transaction_delete') {
                      return (
                        <div key={`delete-${message.id}-${idx}`} className={cn('p-3 rounded-lg border border-red-500/30 bg-red-500/5', suggestion.status !== 'pending' && 'opacity-60')}>
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
                                    className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-red-600 text-white hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
                                    className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
                      );
                    }

                    // Extracted transactions from images
                    if (suggestion.actionType === 'extracted_transactions') {
                      let payload: ExtractedTransactionsPayload | null = null;
                      try {
                        payload = JSON.parse(suggestion.payload);
                      } catch {
                        return null;
                      }
                      if (!payload) return null;

                      return (
                        <div
                          key={`extracted-${message.id}-${idx}`}
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
                    }

                    // Show chart suggestions
                    if (suggestion.actionType === 'show_chart') {
                      return (
                        <div key={`chart-${message.id}-${idx}`} className={cn('p-3 rounded-lg border border-purple-500/30 bg-purple-500/5', suggestion.status !== 'pending' && 'opacity-60')}>
                          <div className="flex items-start gap-2">
                            <CandlestickChart className="h-4 w-4 text-purple-500 mt-0.5 shrink-0" />
                            <div className="flex-1 min-w-0">
                              <p className="text-sm font-medium text-purple-600">Technische Analyse</p>
                              <p className="text-sm text-muted-foreground">{suggestion.description}</p>
                              {suggestion.status === 'pending' ? (
                                <div className="flex gap-2 mt-2">
                                  <button
                                    onClick={() => executeSuggestion(suggestion)}
                                    disabled={executingSuggestion !== null}
                                    className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-purple-600 text-white hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                                  >
                                    {executingSuggestion === suggestion.payload ? (
                                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                                    ) : (
                                      <CandlestickChart className="h-3.5 w-3.5" />
                                    )}
                                    Chart öffnen
                                  </button>
                                  <button
                                    onClick={() => declineSuggestion(suggestion)}
                                    disabled={executingSuggestion !== null}
                                    className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                                  >
                                    <XCircle className="h-3.5 w-3.5" />
                                    Abbrechen
                                  </button>
                                </div>
                              ) : (
                                <div className="mt-2 text-xs text-muted-foreground flex items-center gap-1.5">
                                  {suggestion.status === 'confirmed' ? (
                                    <><CheckCircle className="h-3.5 w-3.5 text-green-600" /> Chart geöffnet</>
                                  ) : (
                                    <><XCircle className="h-3.5 w-3.5 text-muted-foreground" /> Abgebrochen</>
                                  )}
                                </div>
                              )}
                            </div>
                          </div>
                        </div>
                      );
                    }

                    // Screener run suggestions
                    if (suggestion.actionType === 'screener_run') {
                      let screenerPayload: { filters?: Array<{ indicator: string; condition: string; value: number; value2?: number }>; mode?: string; indexId?: string } | null = null;
                      try {
                        screenerPayload = JSON.parse(suggestion.payload);
                      } catch {
                        screenerPayload = null;
                      }
                      return (
                        <div key={`screener-${message.id}-${idx}`} className={cn('p-3 rounded-lg border border-blue-500/30 bg-blue-500/5', suggestion.status !== 'pending' && 'opacity-60')}>
                          <div className="flex items-start gap-2">
                            <SlidersHorizontal className="h-4 w-4 text-blue-500 mt-0.5 shrink-0" />
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <p className="text-sm font-medium text-blue-600">Screener-Scan</p>
                                {screenerPayload?.indexId && (
                                  <span className="text-xs px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-500 font-medium uppercase">{screenerPayload.indexId.replace(/\d+$/, ' $&').trim()}</span>
                                )}
                                {screenerPayload?.mode === 'local' && (
                                  <span className="text-xs px-1.5 py-0.5 rounded bg-green-500/10 text-green-500 font-medium">Eigene Aktien</span>
                                )}
                              </div>
                              <p className="text-sm text-muted-foreground">{suggestion.description}</p>
                              {screenerPayload?.filters && screenerPayload.filters.length > 0 && (
                                <ul className="mt-1.5 space-y-0.5">
                                  {screenerPayload.filters.map((f, fi) => (
                                    <li key={fi} className="text-xs text-muted-foreground flex items-center gap-1">
                                      <span className="inline-block w-1.5 h-1.5 rounded-full bg-blue-400 shrink-0" />
                                      {f.indicator} {f.condition} {f.value}{f.value2 !== undefined ? ` – ${f.value2}` : ''}
                                    </li>
                                  ))}
                                </ul>
                              )}
                              {suggestion.status === 'pending' ? (
                                <div className="flex gap-2 mt-2">
                                  <button
                                    onClick={() => executeSuggestion(suggestion)}
                                    disabled={executingSuggestion !== null}
                                    className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-blue-600 text-white hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                                  >
                                    {executingSuggestion === suggestion.payload ? (
                                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                                    ) : (
                                      <SlidersHorizontal className="h-3.5 w-3.5" />
                                    )}
                                    Im Screener öffnen
                                  </button>
                                  <button
                                    onClick={() => declineSuggestion(suggestion)}
                                    disabled={executingSuggestion !== null}
                                    className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                                  >
                                    <XCircle className="h-3.5 w-3.5" />
                                    Abbrechen
                                  </button>
                                </div>
                              ) : (
                                <div className="mt-2 text-xs text-muted-foreground flex items-center gap-1.5">
                                  {suggestion.status === 'confirmed' ? (
                                    <><CheckCircle className="h-3.5 w-3.5 text-green-600" /> Im Screener geöffnet</>
                                  ) : (
                                    <><XCircle className="h-3.5 w-3.5 text-muted-foreground" /> Abgebrochen</>
                                  )}
                                </div>
                              )}
                            </div>
                          </div>
                        </div>
                      );
                    }

                    // Other suggestions (watchlist, etc.)
                    return (
                      <div key={`other-${message.id}-${idx}`} className={cn('p-3 rounded-lg border border-amber-500/30 bg-amber-500/5', suggestion.status !== 'pending' && 'opacity-60')}>
                        <div className="flex items-start gap-2">
                          <AlertTriangle className="h-4 w-4 text-amber-500 mt-0.5 shrink-0" />
                          <div className="flex-1 min-w-0">
                            <p className="text-sm">{suggestion.description}</p>
                            {suggestion.status === 'pending' ? (
                              <div className="flex gap-2 mt-2">
                                <button
                                  onClick={() => executeSuggestion(suggestion)}
                                  disabled={executingSuggestion !== null}
                                  className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
                                  className="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-md bg-muted hover:bg-muted/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
                    );
                  })}

                  {/* SECURITY: Render pending queries that need user approval */}
                  {messagePendingQueries.map((query, idx) => (
                    <QueryApprovalCard
                      key={`query-${message.id}-${idx}`}
                      query={query}
                      onApproveOnce={() => handleApproveQueryOnce(query)}
                      onApproveSession={() => handleApproveQuerySession(query)}
                      onDecline={() => handleDeclineQuery(query)}
                      isExecuting={executingQuery === query.payload}
                      disabled={executingQuery !== null}
                      sqlApprovalMode={sqlApprovalMode}
                    />
                  ))}
                </div>
              );
            })
          )}

          {isLoading && (
            <div className="flex items-center gap-2 p-3 rounded-lg bg-muted/50">
              <Loader2 className="h-4 w-4 animate-spin text-primary" />
              <span className="text-sm text-muted-foreground">Denke nach...</span>
            </div>
          )}

          {error && (
            <div className="p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-sm text-destructive">
              <p className="mb-2">{error}</p>
              {lastFailedInput && (
                <button
                  onClick={() => {
                    setError(null);
                    sendMessage(lastFailedInput);
                  }}
                  className="text-xs px-2 py-1 rounded bg-destructive/20 hover:bg-destructive/30 transition-colors"
                >
                  Erneut versuchen
                </button>
              )}
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
        <div className="p-4 border-t border-border overflow-visible">
          {!hasApiKey() ? (
            <div className="text-center text-sm text-muted-foreground p-2">
              Bitte konfiguriere deinen {aiProvider.toUpperCase()} API-Key in den Einstellungen.
            </div>
          ) : (
            <>
            {lastResponseCost && (
              <div className="text-[10px] text-muted-foreground/50 text-right px-1 font-mono">
                {lastResponseCost}
              </div>
            )}
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

              {/* Microphone button for speech-to-text - only shown for OpenAI provider */}
              {isOpenAiProvider && !isRecording && (
                <button
                  type="button"
                  onClick={toggleRecording}
                  disabled={isLoading || isTranscribing || !hasSpeechToText}
                  className={cn(
                    'p-2 rounded-lg transition-colors shrink-0',
                    isTranscribing
                      ? 'bg-primary/20 text-primary'
                      : hasSpeechToText
                      ? 'text-muted-foreground hover:bg-muted hover:text-foreground'
                      : 'text-muted-foreground/50 cursor-not-allowed'
                  )}
                  title={
                    isTranscribing
                      ? 'Transkribiere...'
                      : 'Spracheingabe (Whisper)'
                  }
                >
                  {isTranscribing ? (
                    <Loader2 className="h-5 w-5 animate-spin" />
                  ) : (
                    <Mic className="h-5 w-5" />
                  )}
                </button>
              )}

              {/* Classic oscilloscope waveform during recording */}
              {isRecording && (
                <div className="flex-1 flex items-center gap-3 px-3 py-2 rounded-lg bg-muted/30 border border-border">
                  {/* Canvas waveform */}
                  <canvas
                    ref={canvasRef}
                    className="flex-1 h-10"
                    style={{ imageRendering: 'pixelated' }}
                  />

                  {/* Recording indicator */}
                  <div className="flex items-center gap-2 shrink-0">
                    <div className="w-2 h-2 rounded-full bg-red-500 animate-pulse" />
                    <span className="text-xs text-muted-foreground">Aufnahme...</span>
                  </div>

                  {/* Stop button */}
                  <button
                    type="button"
                    onClick={stopRecording}
                    className="p-2 rounded-lg hover:bg-muted transition-colors shrink-0"
                    title="Aufnahme stoppen"
                  >
                    <Square className="h-4 w-4 text-destructive" />
                  </button>
                </div>
              )}

              {!isRecording && (
                <>
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
                </>
              )}
            </div>
            </>
          )}

          {/* Model selector with Vision indicator */}
          <div className="mt-2 flex items-center justify-between overflow-visible">
            <AIModelSelector
              featureId="chatAssistant"
              value={{ provider: aiProvider, model: aiModel }}
              onChange={setTempSelection}
              compact
              disabled={isLoading}
              forceDirection="up"
            />
            <VisionIndicator model={aiModel} provider={aiProvider} className="ml-2" />
          </div>
        </div>
      </>
      )}
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
