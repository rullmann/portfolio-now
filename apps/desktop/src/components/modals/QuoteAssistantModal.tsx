/**
 * AI-powered Quote Assistant Modal.
 * Split-layout with problematic securities list and AI chat.
 */

import { useState, useEffect, useCallback } from 'react';
import { X, Bot, Settings } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useEscapeKey } from '../../lib/hooks/useEscapeKey';
import { useSettingsStore, AI_MODELS, type AiProvider } from '../../store';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { ProblematicSecurityList } from '../quote-assistant/ProblematicSecurityList';
import { QuoteChat } from '../quote-assistant/QuoteChat';
import type {
  ProblematicSecurity,
  ValidatedQuoteSuggestion,
  QuoteAssistantContext,
  QuoteAssistantRequest,
  QuoteAssistantResponse,
} from '../../lib/types';
import { toast } from '../../store';

interface QuoteAssistantModalProps {
  isOpen: boolean;
  onClose: () => void;
  onApplied?: () => void;
}

interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
  suggestion?: ValidatedQuoteSuggestion;
}

export function QuoteAssistantModal({
  isOpen,
  onClose,
  onApplied,
}: QuoteAssistantModalProps) {
  // State
  const [securities, setSecurities] = useState<ProblematicSecurity[]>([]);
  const [selectedSecurity, setSelectedSecurity] = useState<ProblematicSecurity | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoadingSecurities, setIsLoadingSecurities] = useState(true);
  const [isLoadingChat, setIsLoadingChat] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  // Settings
  const aiFeatureSettings = useSettingsStore((s) => s.aiFeatureSettings);
  const setAiFeatureSetting = useSettingsStore((s) => s.setAiFeatureSetting);
  const { keys } = useSecureApiKeys();

  // Get provider and model for quote assistant (defaults to perplexity for web search)
  const quoteAssistantConfig = aiFeatureSettings.quoteAssistant ?? {
    provider: 'perplexity' as AiProvider,
    model: 'sonar-pro',
  };
  const [provider, setProvider] = useState<AiProvider>(quoteAssistantConfig.provider);
  const [model, setModel] = useState(quoteAssistantConfig.model);

  // Get API key for selected provider
  const getApiKey = useCallback(() => {
    switch (provider) {
      case 'claude':
        return keys.anthropicApiKey || '';
      case 'openai':
        return keys.openaiApiKey || '';
      case 'gemini':
        return keys.geminiApiKey || '';
      case 'perplexity':
        return keys.perplexityApiKey || '';
      default:
        return '';
    }
  }, [provider, keys]);

  // Escape key handling
  useEscapeKey(isOpen, onClose);

  // Load problematic securities
  useEffect(() => {
    if (!isOpen) return;

    const loadSecurities = async () => {
      setIsLoadingSecurities(true);
      try {
        const result = await invoke<ProblematicSecurity[]>('get_quote_problem_securities', {
          staleDays: 7,
        });
        setSecurities(result);
      } catch (error) {
        console.error('Failed to load problematic securities:', error);
        toast.error('Fehler beim Laden der Wertpapiere');
      } finally {
        setIsLoadingSecurities(false);
      }
    };

    loadSecurities();
  }, [isOpen]);

  // Auto-start analysis when security is selected
  const handleSelectSecurity = useCallback(
    async (security: ProblematicSecurity) => {
      setSelectedSecurity(security);
      setMessages([
        {
          role: 'system',
          content: `${security.name} ausgewählt`,
        },
      ]);

      // Check for API key
      const apiKey = getApiKey();
      if (!apiKey) {
        setMessages((prev) => [
          ...prev,
          {
            role: 'assistant',
            content: `Bitte konfiguriere einen API-Key für ${provider} in den Einstellungen.`,
          },
        ]);
        return;
      }

      // Start analysis
      setIsLoadingChat(true);
      try {
        const context: QuoteAssistantContext = {
          securityId: security.id,
          securityName: security.name,
          isin: security.isin,
          ticker: security.ticker,
          currency: security.currency,
          currentFeed: security.feed,
          currentFeedUrl: security.feedUrl,
          problem: security.problemType,
          lastError: undefined,
          daysSinceQuote: undefined,
        };

        const request: QuoteAssistantRequest = {
          provider,
          model,
          apiKey,
          securityContext: context,
          history: [],
        };

        const response = await invoke<QuoteAssistantResponse>('chat_with_quote_assistant', {
          request,
        });

        setMessages((prev) => [
          ...prev,
          {
            role: 'assistant',
            content: response.message,
            suggestion: response.suggestion,
          },
        ]);
      } catch (error: any) {
        console.error('Chat error:', error);
        setMessages((prev) => [
          ...prev,
          {
            role: 'assistant',
            content: `Fehler: ${error.toString()}`,
          },
        ]);
      } finally {
        setIsLoadingChat(false);
      }
    },
    [provider, model, getApiKey]
  );

  // Send manual message
  const handleSendMessage = useCallback(
    async (userMessage: string) => {
      if (!selectedSecurity) return;

      const apiKey = getApiKey();
      if (!apiKey) {
        toast.error(`Kein API-Key für ${provider} konfiguriert`);
        return;
      }

      // Add user message
      setMessages((prev) => [...prev, { role: 'user', content: userMessage }]);
      setIsLoadingChat(true);

      try {
        const context: QuoteAssistantContext = {
          securityId: selectedSecurity.id,
          securityName: selectedSecurity.name,
          isin: selectedSecurity.isin,
          ticker: selectedSecurity.ticker,
          currency: selectedSecurity.currency,
          currentFeed: selectedSecurity.feed,
          currentFeedUrl: selectedSecurity.feedUrl,
          problem: selectedSecurity.problemType,
        };

        // Build history from messages
        const history = messages
          .filter((m) => m.role === 'user' || m.role === 'assistant')
          .map((m) => ({ role: m.role, content: m.content }));

        const request: QuoteAssistantRequest = {
          provider,
          model,
          apiKey,
          securityContext: context,
          userMessage,
          history,
        };

        const response = await invoke<QuoteAssistantResponse>('chat_with_quote_assistant', {
          request,
        });

        setMessages((prev) => [
          ...prev,
          {
            role: 'assistant',
            content: response.message,
            suggestion: response.suggestion,
          },
        ]);
      } catch (error: any) {
        console.error('Chat error:', error);
        setMessages((prev) => [
          ...prev,
          {
            role: 'assistant',
            content: `Fehler: ${error.toString()}`,
          },
        ]);
      } finally {
        setIsLoadingChat(false);
      }
    },
    [selectedSecurity, provider, model, getApiKey, messages]
  );

  // Apply suggestion
  const handleApplySuggestion = useCallback(
    async (suggestion: ValidatedQuoteSuggestion) => {
      if (!selectedSecurity || !suggestion.validated) return;

      setIsApplying(true);
      try {
        await invoke('apply_quote_assistant_suggestion', {
          securityId: selectedSecurity.id,
          provider: suggestion.suggestion.provider,
          ticker: suggestion.suggestion.ticker,
          feedUrl: suggestion.suggestion.feedUrl,
        });

        toast.success(`Kursquelle für ${selectedSecurity.name} aktualisiert`);

        // Remove from list
        setSecurities((prev) => prev.filter((s) => s.id !== selectedSecurity.id));
        setSelectedSecurity(null);
        setMessages([]);

        onApplied?.();
      } catch (error: any) {
        console.error('Apply error:', error);
        toast.error(`Fehler beim Anwenden: ${error.toString()}`);
      } finally {
        setIsApplying(false);
      }
    },
    [selectedSecurity, onApplied]
  );

  // Save settings
  const handleSaveSettings = () => {
    setAiFeatureSetting('quoteAssistant', { provider, model });
    setShowSettings(false);
    toast.success('Einstellungen gespeichert');
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg shadow-xl w-[900px] max-w-[95vw] h-[700px] max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <div className="flex items-center gap-2">
            <Bot className="h-5 w-5 text-primary" />
            <span className="font-semibold">Kursquellen-Assistent</span>
          </div>
          <div className="flex items-center gap-2">
            {/* Provider Selector */}
            <select
              value={provider}
              onChange={(e) => {
                const p = e.target.value as AiProvider;
                setProvider(p);
                // Set default model for provider
                const models = AI_MODELS[p];
                if (models && models.length > 0) {
                  setModel(models[0].id);
                }
              }}
              className="px-2 py-1 text-sm border rounded bg-background"
            >
              <option value="perplexity">Perplexity</option>
              <option value="openai">OpenAI</option>
              <option value="claude">Claude</option>
              <option value="gemini">Gemini</option>
            </select>
            <button
              onClick={() => setShowSettings(!showSettings)}
              className="p-1.5 hover:bg-muted rounded"
              title="Einstellungen"
            >
              <Settings className="h-4 w-4" />
            </button>
            <button
              onClick={onClose}
              className="p-1.5 hover:bg-muted rounded"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Settings Panel */}
        {showSettings && (
          <div className="px-4 py-3 border-b bg-muted/30">
            <div className="flex items-center gap-4">
              <div className="flex-1">
                <label className="text-sm font-medium">Modell</label>
                <select
                  value={model}
                  onChange={(e) => setModel(e.target.value)}
                  className="w-full mt-1 px-2 py-1 text-sm border rounded bg-background"
                >
                  {AI_MODELS[provider]?.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.name}
                    </option>
                  ))}
                </select>
              </div>
              <button
                onClick={handleSaveSettings}
                className="px-3 py-1.5 text-sm bg-primary text-primary-foreground rounded hover:bg-primary/90"
              >
                Speichern
              </button>
            </div>
          </div>
        )}

        {/* Content */}
        <div className="flex-1 flex overflow-hidden">
          {/* Left: Security List */}
          <div className="w-[300px] border-r p-4 overflow-y-auto">
            <ProblematicSecurityList
              securities={securities}
              selectedId={selectedSecurity?.id ?? null}
              onSelect={handleSelectSecurity}
              isLoading={isLoadingSecurities}
            />
          </div>

          {/* Right: Chat */}
          <div className="flex-1 flex flex-col">
            <QuoteChat
              security={selectedSecurity}
              messages={messages}
              onSendMessage={handleSendMessage}
              onApplySuggestion={handleApplySuggestion}
              isLoading={isLoadingChat}
              isApplying={isApplying}
            />
          </div>
        </div>

        {/* Footer */}
        <div className="px-4 py-3 border-t flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 border rounded-lg hover:bg-muted"
          >
            Schließen
          </button>
        </div>
      </div>
    </div>
  );
}
