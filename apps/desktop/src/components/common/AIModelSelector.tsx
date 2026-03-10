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
 * - Token limit, pricing, and deprecation badges per model
 */

import { useState, useRef, useEffect, useMemo, useCallback } from 'react';
import { ChevronDown, Check, Settings, Sparkles, Loader2, Eye, Globe, Zap, Star, AlertTriangle, Coins } from 'lucide-react';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { useSettingsStore, AI_MODELS, DEFAULT_MODELS, fetchModelsForProvider, type AiProvider, type AiFeatureId, type AiModelInfo } from '../../store';
import { useUIStore } from '../../store';
import { getLatestExchangeRate } from '../../lib/api';
import { AIProviderLogo, AI_PROVIDER_NAMES } from './AIProviderLogo';
import { cn } from '../../lib/utils';

// Model quality tiers — derived from model ID patterns
type ModelTier = 'top' | 'fast' | 'standard';

function getModelTier(model: AiModelInfo): ModelTier {
  const id = model.id.toLowerCase();
  // Top tier: opus, pro, o3, gpt-5 (non-mini)
  if (id.includes('opus') || id.includes('-pro') || id === 'o3' || (id.startsWith('gpt-5') && !id.includes('mini')))
    return 'top';
  // Fast tier: haiku, mini, flash, lite
  if (id.includes('haiku') || id.includes('mini') || id.includes('flash') || id.includes('lite'))
    return 'fast';
  return 'standard';
}

/** Format token count to short display: 16384 -> "16K", 128000 -> "128K", 1000000 -> "1M" */
function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(tokens % 1_000_000 === 0 ? 0 : 1)}M`;
  if (tokens >= 1_000) return `${Math.round(tokens / 1_000)}K`;
  return `${tokens}`;
}

/** Format pricing to compact display: 3.0 -> "€3", 0.25 -> "€0.25" */
function formatPrice(price: number, symbol: string = '$'): string {
  if (price >= 1) return `${symbol}${price % 1 === 0 ? price.toFixed(0) : price.toFixed(1)}`;
  if (price >= 0.01) return `${symbol}${price.toFixed(2)}`;
  return `<${symbol}0.01`;
}

/** Currency symbols for common base currencies */
const CURRENCY_SYMBOLS: Record<string, string> = {
  EUR: '€', USD: '$', GBP: '£', CHF: 'CHF ', JPY: '¥',
};

/** Check if a model is near deprecation (within 30 days) */
function isNearDeprecation(deprecationDate?: string): boolean {
  if (!deprecationDate) return false;
  const date = new Date(deprecationDate);
  const thirtyDaysFromNow = new Date(Date.now() + 30 * 24 * 60 * 60 * 1000);
  return date <= thirtyDaysFromNow;
}

/** Check if a model is already deprecated */
function isDeprecated(deprecationDate?: string): boolean {
  if (!deprecationDate) return false;
  return new Date(deprecationDate) <= new Date();
}

// Feature-specific model recommendations
const FEATURE_RECOMMENDATIONS: Record<AiFeatureId, { prefer: ModelTier; reason: string }> = {
  chartAnalysis: { prefer: 'standard', reason: 'Gute Balance aus Qualität & Geschwindigkeit' },
  portfolioInsights: { prefer: 'top', reason: 'Tiefgründige Analyse braucht starkes Modell' },
  chatAssistant: { prefer: 'fast', reason: 'Schnelle Antworten für flüssigen Chat' },
  pdfOcr: { prefer: 'fast', reason: 'Schnelle Texterkennung reicht' },
  csvImport: { prefer: 'fast', reason: 'Strukturerkennung braucht kein Top-Modell' },
  quoteAssistant: { prefer: 'fast', reason: 'Einfache Kursquellen-Suche' },
  newsResearch: { prefer: 'standard', reason: 'Web-Suche + Zusammenfassung' },
};

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
  /** Force dropdown direction (overrides auto-detection) */
  forceDirection?: 'up' | 'down';
}

// Provider display order
const PROVIDER_ORDER: AiProvider[] = ['claude', 'openai', 'gemini', 'perplexity', 'openrouter'];

export function AIModelSelector({
  featureId,
  requiresVision = false,
  requiresWebSearch = false,
  value,
  onChange,
  compact = false,
  disabled = false,
  forceDirection,
}: AIModelSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [saveAsDefault, setSaveAsDefault] = useState(false);
  const [openDirection, setOpenDirection] = useState<'up' | 'down'>('down');
  const dropdownRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);

  const { keys } = useSecureApiKeys();
  const { setAiFeatureSetting, baseCurrency } = useSettingsStore();
  const { setCurrentView, setScrollTarget } = useUIStore();

  // Dynamic model state
  const [dynamicModels, setDynamicModels] = useState<Partial<Record<AiProvider, AiModelInfo[]>>>({});
  const [loadingProviders, setLoadingProviders] = useState<Set<AiProvider>>(new Set());

  // Exchange rate for price conversion (USD -> baseCurrency)
  const [fxRate, setFxRate] = useState<number | null>(null);
  const currencySymbol = CURRENCY_SYMBOLS[baseCurrency] || baseCurrency + ' ';

  // Map provider to API key
  const providerKeys = useMemo(() => ({
    claude: keys.anthropicApiKey,
    openai: keys.openaiApiKey,
    gemini: keys.geminiApiKey,
    perplexity: keys.perplexityApiKey,
    openrouter: keys.openrouterApiKey,
  }), [keys]);

  // Map provider to API key presence
  const providerHasKey = useMemo(() => ({
    claude: !!providerKeys.claude,
    openai: !!providerKeys.openai,
    gemini: !!providerKeys.gemini,
    perplexity: !!providerKeys.perplexity,
    openrouter: !!providerKeys.openrouter,
  }), [providerKeys]);

  // Fetch models for a provider dynamically
  const loadModelsForProvider = useCallback(async (provider: AiProvider) => {
    const apiKey = providerKeys[provider];
    if (!apiKey) return;

    setLoadingProviders(prev => new Set(prev).add(provider));
    try {
      const models = await fetchModelsForProvider(provider, apiKey);
      setDynamicModels(prev => ({ ...prev, [provider]: models }));
    } finally {
      setLoadingProviders(prev => {
        const next = new Set(prev);
        next.delete(provider);
        return next;
      });
    }
  }, [providerKeys]);

  // Load models for all providers when dropdown opens
  useEffect(() => {
    if (!isOpen) return;
    PROVIDER_ORDER.forEach(provider => {
      if (providerHasKey[provider]) {
        loadModelsForProvider(provider);
      }
    });
  }, [isOpen, providerHasKey, loadModelsForProvider]);

  // Fetch exchange rate when dropdown opens (USD -> baseCurrency)
  useEffect(() => {
    if (!isOpen || baseCurrency === 'USD') {
      if (baseCurrency === 'USD') setFxRate(1);
      return;
    }
    if (fxRate !== null) return; // already fetched
    getLatestExchangeRate('USD', baseCurrency)
      .then(r => setFxRate(r.rate))
      .catch(() => setFxRate(null));
  }, [isOpen, baseCurrency, fxRate]);

  // Get models for a provider — dynamic if available, else static fallback
  const getModels = useCallback((provider: AiProvider): AiModelInfo[] => {
    const models = dynamicModels[provider] || AI_MODELS[provider] || [];
    if (requiresWebSearch) return models.filter(m => m.supportsWebSearch);
    if (requiresVision) return models.filter(m => m.supportsVision !== false);
    return models;
  }, [dynamicModels, requiresVision, requiresWebSearch]);

  // Get available providers (with API key, filtered by web search if needed)
  const availableProviders = useMemo(() => {
    const withKey = PROVIDER_ORDER.filter(p => providerHasKey[p]);
    if (requiresWebSearch) {
      return withKey.filter(p => getModels(p).length > 0);
    }
    return withKey;
  }, [providerHasKey, requiresWebSearch, getModels]);

  // Get models for each provider
  const modelsByProvider = useMemo(() => {
    const result: Partial<Record<AiProvider, AiModelInfo[]>> = {};
    for (const provider of PROVIDER_ORDER) {
      result[provider] = getModels(provider);
    }
    return result;
  }, [getModels]);

  // Get current model info
  const currentModelInfo = useMemo(() => {
    const models = getModels(value.provider);
    return models.find(m => m.id === value.model) || { id: value.model, name: value.model, description: '' };
  }, [value, getModels]);

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
      setOpenDirection(forceDirection ?? calculateDirection());
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
            {isNearDeprecation(currentModelInfo.deprecationDate) && (
              <AlertTriangle size={compact ? 10 : 12} className="text-amber-500" />
            )}
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
          "absolute z-50 min-w-[320px] rounded-lg border border-border bg-popover shadow-lg",
          openDirection === 'down' ? 'top-full mt-1' : 'bottom-full mb-1'
        )}>
          <div className="max-h-[420px] overflow-y-auto py-1">
            {PROVIDER_ORDER.filter(p => !requiresWebSearch || getModels(p).length > 0).map(provider => {
              const hasKey = providerHasKey[provider];
              const models = modelsByProvider[provider] || [];
              const isLoading = loadingProviders.has(provider);
              const rec = FEATURE_RECOMMENDATIONS[featureId];

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
                    {isLoading && (
                      <Loader2 size={12} className="animate-spin text-muted-foreground ml-auto" />
                    )}
                  </div>

                  {hasKey ? (
                    // Model List
                    models.map(model => {
                      const isSelected = value.provider === provider && value.model === model.id;
                      const tier = getModelTier(model);
                      const isRecommended = rec && tier === rec.prefer && model.id === DEFAULT_MODELS[provider];
                      const deprecated = isDeprecated(model.deprecationDate);
                      const nearDeprecation = isNearDeprecation(model.deprecationDate);

                      return (
                        <button
                          key={model.id}
                          onClick={() => !deprecated && handleSelect(provider, model.id)}
                          disabled={deprecated}
                          className={cn(
                            'w-full flex items-center gap-2 px-3 py-2 text-sm text-left transition-colors',
                            !deprecated && 'hover:bg-muted',
                            isSelected && 'bg-primary/10',
                            isRecommended && !isSelected && 'bg-amber-500/5',
                            deprecated && 'opacity-40 cursor-not-allowed',
                            nearDeprecation && !deprecated && 'border-l-2 border-amber-400',
                          )}
                        >
                          <span className="w-4 shrink-0">
                            {isSelected && <Check size={14} className="text-primary" />}
                          </span>
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-1.5">
                              <span className={cn('font-medium', deprecated && 'line-through')}>{model.name}</span>
                              {isRecommended && (
                                <span className="inline-flex items-center gap-0.5 px-1.5 py-0.5 text-[10px] font-medium rounded-full bg-amber-500/15 text-amber-600 dark:text-amber-400" title={rec.reason}>
                                  <Star size={8} className="fill-current" />
                                  Empfohlen
                                </span>
                              )}
                              {nearDeprecation && !deprecated && (
                                <span
                                  className="inline-flex items-center gap-0.5 px-1 py-0.5 text-[9px] font-medium rounded bg-amber-500/15 text-amber-600 dark:text-amber-400"
                                  title={`Wird am ${new Date(model.deprecationDate!).toLocaleDateString('de-DE')} abgeschaltet`}
                                >
                                  <AlertTriangle size={7} />
                                  EOL
                                </span>
                              )}
                            </div>
                            <div className="flex items-center gap-1 mt-0.5">
                              <span className="text-xs text-muted-foreground truncate max-w-[120px]">{model.description}</span>
                              {/* Capability & metadata badges */}
                              <span className="flex items-center gap-1 shrink-0 ml-auto">
                                {tier === 'top' && (
                                  <span className="inline-flex items-center px-1 py-0.5 text-[9px] font-medium rounded bg-purple-500/15 text-purple-600 dark:text-purple-400" title="Höchste Qualität">
                                    PRO
                                  </span>
                                )}
                                {tier === 'fast' && (
                                  <span className="inline-flex items-center gap-0.5 px-1 py-0.5 text-[9px] font-medium rounded bg-blue-500/15 text-blue-600 dark:text-blue-400" title="Schnell & günstig">
                                    <Zap size={7} className="fill-current" />
                                    FAST
                                  </span>
                                )}
                                {model.supportsVision && (
                                  <span className="inline-flex items-center px-1 py-0.5 text-[9px] font-medium rounded bg-green-500/15 text-green-600 dark:text-green-400" title="Kann Bilder analysieren">
                                    <Eye size={7} />
                                  </span>
                                )}
                                {model.supportsWebSearch && (
                                  <span className="inline-flex items-center px-1 py-0.5 text-[9px] font-medium rounded bg-sky-500/15 text-sky-600 dark:text-sky-400" title="Web-Suche">
                                    <Globe size={7} />
                                  </span>
                                )}
                                {model.maxOutputTokens && (
                                  <span
                                    className="inline-flex items-center px-1 py-0.5 text-[9px] font-medium rounded bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 font-mono"
                                    title={`Max. ${model.maxOutputTokens.toLocaleString()} Output-Tokens`}
                                  >
                                    {formatTokens(model.maxOutputTokens)}
                                  </span>
                                )}
                                {model.pricingInput != null && model.pricingOutput != null && (() => {
                                  const rate = fxRate ?? 1;
                                  const sym = fxRate != null ? currencySymbol : '$';
                                  const inp = model.pricingInput! * rate;
                                  const out = model.pricingOutput! * rate;
                                  return (
                                    <span
                                      className="inline-flex items-center gap-0.5 px-1 py-0.5 text-[9px] font-medium rounded bg-slate-500/10 text-slate-500 dark:text-slate-400 font-mono"
                                      title={`${formatPrice(inp, sym)} Input / ${formatPrice(out, sym)} Output pro 1M Tokens`}
                                    >
                                      <Coins size={7} />
                                      {formatPrice(inp, sym)}/{formatPrice(out, sym)}
                                    </span>
                                  );
                                })()}
                              </span>
                            </div>
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
