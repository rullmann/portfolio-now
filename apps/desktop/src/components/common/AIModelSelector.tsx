/**
 * AIModelSelector - Reusable component for selecting AI provider and model.
 *
 * Displays a compact dropdown with available providers and models,
 * grouped by provider. Providers without an API key are shown disabled
 * with a link to settings.
 *
 * Features:
 * - Filters by available API keys
 * - Optional vision-only mode for features requiring image processing
 * - "Save as default" checkbox to persist selection
 */

import { useState, useRef, useEffect, useMemo } from 'react';
import { ChevronDown, Check, Settings, Sparkles } from 'lucide-react';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { useSettingsStore, AI_MODELS, WEB_SEARCH_AI_MODELS, type AiProvider, type AiFeatureId } from '../../store';
import { useUIStore } from '../../store';
import { AIProviderLogo, AI_PROVIDER_NAMES } from './AIProviderLogo';
import { cn } from '../../lib/utils';

interface AIModelSelectorProps {
  /** Feature ID for "Save as default" functionality */
  featureId: AiFeatureId;
  /** Only show vision-capable models */
  requiresVision?: boolean;
  /** Only show web-search-capable models (OpenAI, Perplexity) */
  requiresWebSearch?: boolean;
  /** Current selected provider and model */
  value: { provider: AiProvider; model: string };
  /** Callback when selection changes */
  onChange: (value: { provider: AiProvider; model: string }) => void;
  /** Compact mode (smaller) */
  compact?: boolean;
  /** Disable the selector */
  disabled?: boolean;
}

// Provider display order
const PROVIDER_ORDER: AiProvider[] = ['claude', 'openai', 'gemini', 'perplexity'];

// Vision-only models (for requiresVision filter)
// Note: All models in AI_MODELS are vision-capable, but some features may need this filter
const NON_VISION_MODELS = new Set<string>([
  // Add non-vision model IDs here if needed in the future
]);

export function AIModelSelector({
  featureId,
  requiresVision = false,
  requiresWebSearch = false,
  value,
  onChange,
  compact = false,
  disabled = false,
}: AIModelSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [saveAsDefault, setSaveAsDefault] = useState(false);
  const [openDirection, setOpenDirection] = useState<'up' | 'down'>('down');
  const dropdownRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const { keys } = useSecureApiKeys();
  const { setAiFeatureSetting } = useSettingsStore();
  const { setCurrentView, setScrollTarget } = useUIStore();

  // Map provider to API key presence
  const providerHasKey = useMemo(() => ({
    claude: !!keys.anthropicApiKey,
    openai: !!keys.openaiApiKey,
    gemini: !!keys.geminiApiKey,
    perplexity: !!keys.perplexityApiKey,
  }), [keys]);

  // Get available providers (with API key, filtered by web search if needed)
  const availableProviders = useMemo(() => {
    const withKey = PROVIDER_ORDER.filter(p => providerHasKey[p]);
    if (requiresWebSearch) {
      return withKey.filter(p => WEB_SEARCH_AI_MODELS[p] && WEB_SEARCH_AI_MODELS[p]!.length > 0);
    }
    return withKey;
  }, [providerHasKey, requiresWebSearch]);

  // Get models for each provider (filtered by vision/web-search if needed)
  const modelsByProvider = useMemo(() => {
    const result: Partial<Record<AiProvider, Array<{ id: string; name: string; description: string }>>> = {};

    for (const provider of PROVIDER_ORDER) {
      if (requiresWebSearch) {
        result[provider] = [...(WEB_SEARCH_AI_MODELS[provider] || [])];
      } else {
        const models = AI_MODELS[provider] || [];
        result[provider] = requiresVision
          ? models.filter(m => !NON_VISION_MODELS.has(m.id))
          : [...models];
      }
    }

    return result;
  }, [requiresVision, requiresWebSearch]);

  // Get current model info
  const currentModelInfo = useMemo(() => {
    const models = requiresWebSearch
      ? (WEB_SEARCH_AI_MODELS[value.provider] || [])
      : (AI_MODELS[value.provider] || []);
    return models.find(m => m.id === value.model) || { id: value.model, name: value.model, description: '' };
  }, [value, requiresWebSearch]);

  // Get short display name for the current model
  const shortModelName = useMemo(() => {
    // Extract short name: "Claude Sonnet 4.5" -> "Sonnet 4.5", "GPT-4o" -> "GPT-4o"
    const name = currentModelInfo.name;
    if (name.startsWith('Claude ')) return name.substring(7);
    if (name.startsWith('Gemini ')) return name.substring(7);
    return name;
  }, [currentModelInfo]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [isOpen]);

  // Handle selection
  const handleSelect = (provider: AiProvider, model: string) => {
    onChange({ provider, model });

    // Save as default if checkbox is checked
    if (saveAsDefault) {
      setAiFeatureSetting(featureId, { provider, model });
      setSaveAsDefault(false);
    }

    setIsOpen(false);
  };

  // Navigate to AI settings
  const navigateToSettings = () => {
    setScrollTarget('ai-analysis');
    setCurrentView('settings');
    setIsOpen(false);
  };

  // Check if current provider still has API key
  const currentProviderHasKey = providerHasKey[value.provider];

  // Calculate dropdown direction based on available space
  const calculateDirection = () => {
    if (!buttonRef.current) return 'down';

    const buttonRect = buttonRef.current.getBoundingClientRect();
    const dropdownHeight = 360; // Approximate max height (320px content + padding)
    const spaceBelow = window.innerHeight - buttonRect.bottom;
    const spaceAbove = buttonRect.top;

    // Open upward if not enough space below but enough above
    if (spaceBelow < dropdownHeight && spaceAbove > spaceBelow) {
      return 'up';
    }
    return 'down';
  };

  const handleToggle = () => {
    if (disabled) return;
    if (!isOpen) {
      setOpenDirection(calculateDirection());
    }
    setIsOpen(!isOpen);
  };

  return (
    <div ref={dropdownRef} className="relative inline-block">
      {/* Trigger Button */}
      <button
        ref={buttonRef}
        onClick={handleToggle}
        disabled={disabled}
        className={cn(
          'flex items-center gap-1.5 rounded border border-border bg-background transition-colors',
          compact ? 'px-2 py-1 text-xs' : 'px-2.5 py-1.5 text-sm',
          !disabled && 'hover:bg-muted cursor-pointer',
          disabled && 'opacity-50 cursor-not-allowed',
          isOpen && 'ring-2 ring-primary ring-offset-1'
        )}
      >
        {currentProviderHasKey ? (
          <>
            <AIProviderLogo provider={value.provider} size={compact ? 12 : 14} />
            <span className="font-medium">{shortModelName}</span>
          </>
        ) : (
          <>
            <Sparkles size={compact ? 12 : 14} className="text-muted-foreground" />
            <span className="text-muted-foreground">KI wählen</span>
          </>
        )}
        <ChevronDown size={compact ? 12 : 14} className={cn('text-muted-foreground transition-transform', isOpen && 'rotate-180')} />
      </button>

      {/* Dropdown - auto-positions based on available space */}
      {isOpen && (
        <div className={cn(
          "absolute z-50 min-w-[260px] rounded-lg border border-border bg-popover shadow-lg",
          openDirection === 'down' ? 'top-full mt-1' : 'bottom-full mb-1'
        )}>
          <div className="max-h-[320px] overflow-y-auto py-1">
            {PROVIDER_ORDER.filter(p => !requiresWebSearch || (WEB_SEARCH_AI_MODELS[p] && WEB_SEARCH_AI_MODELS[p]!.length > 0)).map(provider => {
              const hasKey = providerHasKey[provider];
              const models = modelsByProvider[provider] || [];

              return (
                <div key={provider}>
                  {/* Provider Header */}
                  <div className={cn(
                    'flex items-center gap-2 px-3 py-1.5 text-xs font-medium border-t border-border first:border-t-0',
                    !hasKey && 'text-muted-foreground'
                  )}>
                    <AIProviderLogo provider={provider} size={14} className={!hasKey ? 'opacity-50' : ''} />
                    <span>{AI_PROVIDER_NAMES[provider]}</span>
                    {!hasKey && (
                      <span className="text-muted-foreground/70">(Kein API-Key)</span>
                    )}
                  </div>

                  {hasKey ? (
                    // Model List
                    models.map(model => {
                      const isSelected = value.provider === provider && value.model === model.id;
                      return (
                        <button
                          key={model.id}
                          onClick={() => handleSelect(provider, model.id)}
                          className={cn(
                            'w-full flex items-center gap-2 px-3 py-1.5 text-sm text-left hover:bg-muted transition-colors',
                            isSelected && 'bg-primary/10'
                          )}
                        >
                          <span className="w-4 shrink-0">
                            {isSelected && <Check size={14} className="text-primary" />}
                          </span>
                          <div className="flex-1 min-w-0">
                            <div className="font-medium">{model.name}</div>
                            <div className="text-xs text-muted-foreground truncate">{model.description}</div>
                          </div>
                        </button>
                      );
                    })
                  ) : (
                    // Settings Link
                    <button
                      onClick={navigateToSettings}
                      className="w-full flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground hover:bg-muted transition-colors"
                    >
                      <span className="w-4" />
                      <Settings size={14} />
                      <span>In Einstellungen konfigurieren →</span>
                    </button>
                  )}
                </div>
              );
            })}
          </div>

          {/* Save as Default Checkbox */}
          {availableProviders.length > 0 && (
            <div className="border-t border-border px-3 py-2">
              <label className="flex items-center gap-2 text-xs cursor-pointer">
                <input
                  type="checkbox"
                  checked={saveAsDefault}
                  onChange={(e) => setSaveAsDefault(e.target.checked)}
                  className="rounded border-border"
                />
                <span className="text-muted-foreground">Als Standard speichern</span>
              </label>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
