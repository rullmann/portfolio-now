/**
 * AI Feature Matrix - Configure AI provider and model per feature.
 *
 * Compact table-style layout for configuring each AI feature with its own
 * provider and model. Only providers with configured API keys are shown.
 * Shows output token limits and deprecation warnings per model.
 */

import { useState, useMemo, useEffect, useCallback } from 'react';
import {
  BarChart3,
  Lightbulb,
  MessageSquare,
  FileText,
  FileSpreadsheet,
  Eye,
  AlertCircle,
  AlertTriangle,
  Bot,
  Newspaper,
  Globe,
  RefreshCw,
} from 'lucide-react';
import {
  useSettingsStore,
  AI_FEATURES,
  DEFAULT_MODELS,
  fetchModelsForProvider,
  getCachedModels,
  clearModelCache,
  type AiFeatureId,
  type AiProvider,
  type AiModelInfo,
} from '../../store';
import { AIProviderLogo } from '../common/AIProviderLogo';

// Map feature icon names to components
const FEATURE_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  BarChart3,
  Lightbulb,
  MessageSquare,
  FileText,
  FileSpreadsheet,
  Bot,
  Newspaper,
};

// Provider display names
const PROVIDER_NAMES: Record<AiProvider, string> = {
  claude: 'Claude',
  openai: 'OpenAI',
  gemini: 'Gemini',
  perplexity: 'Perplexity',
  openrouter: 'OpenRouter',
};

/** Format token count: 16384 -> "16K", 65536 -> "64K" */
function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(tokens % 1_000_000 === 0 ? 0 : 1)}M`;
  if (tokens >= 1_000) return `${Math.round(tokens / 1_000)}K`;
  return `${tokens}`;
}

/** Check if model is near deprecation (within 30 days) */
function isNearDeprecation(deprecationDate?: string): boolean {
  if (!deprecationDate) return false;
  const date = new Date(deprecationDate);
  return date <= new Date(Date.now() + 30 * 24 * 60 * 60 * 1000);
}

interface AiFeatureMatrixProps {
  /** API keys per provider (from secure storage) */
  apiKeys: {
    anthropicApiKey: string;
    openaiApiKey: string;
    geminiApiKey: string;
    perplexityApiKey: string;
    openrouterApiKey: string;
  };
}

export function AiFeatureMatrix({ apiKeys }: AiFeatureMatrixProps) {
  const { aiFeatureSettings, setAiFeatureSetting } = useSettingsStore();
  const [dynamicModels, setDynamicModels] = useState<Partial<Record<AiProvider, AiModelInfo[]>>>({});
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Map provider to API key
  const providerKeys = useMemo((): Record<AiProvider, string> => ({
    claude: apiKeys.anthropicApiKey || '',
    openai: apiKeys.openaiApiKey || '',
    gemini: apiKeys.geminiApiKey || '',
    perplexity: apiKeys.perplexityApiKey || '',
    openrouter: apiKeys.openrouterApiKey || '',
  }), [apiKeys]);

  // Determine which providers have API keys configured
  const availableProviders = useMemo(() => {
    const providers: AiProvider[] = [];
    if (apiKeys.anthropicApiKey?.trim()) providers.push('claude');
    if (apiKeys.openaiApiKey?.trim()) providers.push('openai');
    if (apiKeys.geminiApiKey?.trim()) providers.push('gemini');
    if (apiKeys.perplexityApiKey?.trim()) providers.push('perplexity');
    if (apiKeys.openrouterApiKey?.trim()) providers.push('openrouter');
    return providers;
  }, [apiKeys]);

  const hasAnyProvider = availableProviders.length > 0;

  // Fetch models dynamically for all available providers on mount
  const loadAllModels = useCallback(async () => {
    await Promise.all(availableProviders.map(async (provider) => {
      const apiKey = providerKeys[provider];
      if (!apiKey) return;
      try {
        const models = await fetchModelsForProvider(provider, apiKey);
        setDynamicModels(prev => ({ ...prev, [provider]: models }));
      } catch {
        // Fallback handled by getModelsForProvider
      }
    }));
  }, [availableProviders, providerKeys]);

  useEffect(() => {
    loadAllModels();
  }, [loadAllModels]);

  // Refresh all models (clear cache first)
  const handleRefresh = async () => {
    setIsRefreshing(true);
    clearModelCache();
    await loadAllModels();
    setIsRefreshing(false);
  };

  // Get models for a specific provider — dynamic first, then static fallback
  const getModelsForProvider = useCallback((provider: AiProvider, requiresWebSearch?: boolean): AiModelInfo[] => {
    const models = dynamicModels[provider] || getCachedModels(provider);
    if (requiresWebSearch) return models.filter(m => m.supportsWebSearch);
    return models;
  }, [dynamicModels]);

  // Get available providers for web-search features
  const webSearchProviders = useMemo(() => {
    return availableProviders.filter(p => getModelsForProvider(p, true).length > 0);
  }, [availableProviders, getModelsForProvider]);

  // Auto-migrate features when providers become unavailable
  useEffect(() => {
    if (!hasAnyProvider) return;

    AI_FEATURES.forEach((feature) => {
      const config = aiFeatureSettings[feature.id];
      if (!config) return;

      const validProviders = feature.requiresWebSearch ? webSearchProviders : availableProviders;

      // Check if current provider is still available
      if (validProviders.length > 0 && !validProviders.includes(config.provider)) {
        // Auto-migrate to first available provider
        const newProvider = validProviders[0];
        const models = getModelsForProvider(newProvider, feature.requiresWebSearch);
        setAiFeatureSetting(feature.id, {
          provider: newProvider,
          model: models[0]?.id || DEFAULT_MODELS[newProvider],
        });
      }
    });
  }, [availableProviders, webSearchProviders, aiFeatureSettings, setAiFeatureSetting, hasAnyProvider]);

  // Handle provider change for a feature
  const handleProviderChange = (featureId: AiFeatureId, provider: AiProvider, requiresWebSearch?: boolean) => {
    const models = getModelsForProvider(provider, requiresWebSearch);
    const defaultModel = models[0]?.id || DEFAULT_MODELS[provider];
    setAiFeatureSetting(featureId, { provider, model: defaultModel });
  };

  // Handle model change for a feature
  const handleModelChange = (featureId: AiFeatureId, model: string) => {
    const currentConfig = aiFeatureSettings[featureId];
    setAiFeatureSetting(featureId, { ...currentConfig, model });
  };

  // Get validated config (fallback if provider unavailable)
  const getValidatedConfig = (featureId: AiFeatureId, requiresWebSearch?: boolean) => {
    const config = aiFeatureSettings[featureId];
    const validProviders = requiresWebSearch ? webSearchProviders : availableProviders;

    if (!validProviders.includes(config.provider) && validProviders.length > 0) {
      return {
        provider: validProviders[0],
        model: DEFAULT_MODELS[validProviders[0]],
        needsMigration: true,
      };
    }

    const models = getModelsForProvider(config.provider, requiresWebSearch);
    if (!models.some(m => m.id === config.model) && models.length > 0) {
      return {
        provider: config.provider,
        model: models[0].id,
        needsMigration: false,
      };
    }

    return { ...config, needsMigration: false };
  };

  // Get model info for currently selected model
  const getSelectedModelInfo = (provider: AiProvider, modelId: string, requiresWebSearch?: boolean): AiModelInfo | undefined => {
    const models = getModelsForProvider(provider, requiresWebSearch);
    return models.find(m => m.id === modelId);
  };

  if (!hasAnyProvider) {
    return (
      <div className="rounded-lg border border-dashed border-border bg-muted/20 p-4 text-center">
        <AlertCircle className="h-8 w-8 mx-auto mb-2 text-muted-foreground/50" />
        <p className="text-sm text-muted-foreground">
          Konfiguriere mindestens einen API-Key, um KI-Funktionen zu nutzen.
        </p>
      </div>
    );
  }

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {/* Header */}
      <div className="grid grid-cols-[1fr,140px,180px,70px] gap-2 px-3 py-2 bg-muted/50 text-xs font-medium text-muted-foreground border-b border-border">
        <div>Funktion</div>
        <div>Provider</div>
        <div>Modell</div>
        <div className="flex items-center gap-1">
          Limit
          <button
            onClick={handleRefresh}
            disabled={isRefreshing}
            className="p-0.5 rounded hover:bg-muted transition-colors"
            title="Modelle neu laden"
          >
            <RefreshCw size={10} className={isRefreshing ? 'animate-spin' : ''} />
          </button>
        </div>
      </div>

      {/* Rows */}
      <div className="divide-y divide-border">
        {AI_FEATURES.map((feature) => {
          const Icon = FEATURE_ICONS[feature.icon] || FileText;
          const config = getValidatedConfig(feature.id, feature.requiresWebSearch);
          const featureProviders = feature.requiresWebSearch ? webSearchProviders : availableProviders;
          const models = getModelsForProvider(config.provider, feature.requiresWebSearch);
          const selectedModel = getSelectedModelInfo(config.provider, config.model, feature.requiresWebSearch);
          const nearDepr = isNearDeprecation(selectedModel?.deprecationDate);

          return (
            <div
              key={feature.id}
              className={`grid grid-cols-[1fr,140px,180px,70px] gap-2 px-3 py-2.5 items-center hover:bg-muted/30 transition-colors ${
                nearDepr ? 'border-l-2 border-amber-400' : ''
              }`}
            >
              {/* Feature Info */}
              <div className="flex items-center gap-2 min-w-0">
                <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
                <span className="font-medium text-sm truncate">{feature.name}</span>
                {feature.requiresVision && (
                  <span title="Benötigt Vision-Modell">
                    <Eye className="h-3 w-3 text-blue-500 shrink-0" />
                  </span>
                )}
                {feature.requiresWebSearch && (
                  <span title="Benötigt Web-Suche">
                    <Globe className="h-3 w-3 text-green-500 shrink-0" />
                  </span>
                )}
              </div>

              {/* Provider Dropdown */}
              <div className="relative">
                <select
                  value={config.provider}
                  onChange={(e) => handleProviderChange(feature.id, e.target.value as AiProvider, feature.requiresWebSearch)}
                  className="w-full pl-7 pr-2 py-1.5 text-xs border border-border rounded bg-background appearance-none cursor-pointer hover:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30"
                >
                  {featureProviders.map((provider) => (
                    <option key={provider} value={provider}>
                      {PROVIDER_NAMES[provider]}
                    </option>
                  ))}
                </select>
                <div className="absolute left-1.5 top-1/2 -translate-y-1/2 pointer-events-none">
                  <AIProviderLogo provider={config.provider} size={14} />
                </div>
              </div>

              {/* Model Dropdown */}
              <div className="relative">
                <select
                  value={config.model}
                  onChange={(e) => handleModelChange(feature.id, e.target.value)}
                  className="w-full px-2 py-1.5 text-xs border border-border rounded bg-background appearance-none cursor-pointer hover:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30"
                >
                  {models.map((model) => (
                    <option key={model.id} value={model.id}>
                      {model.name}
                    </option>
                  ))}
                </select>
              </div>

              {/* Output Token Limit */}
              <div className="flex items-center gap-1">
                {selectedModel?.maxOutputTokens ? (
                  <span
                    className="text-[10px] font-mono font-medium text-emerald-600 dark:text-emerald-400"
                    title={`Max. ${selectedModel.maxOutputTokens.toLocaleString()} Output-Tokens`}
                  >
                    {formatTokens(selectedModel.maxOutputTokens)}
                  </span>
                ) : (
                  <span className="text-[10px] text-muted-foreground">—</span>
                )}
                {nearDepr && (
                  <AlertTriangle
                    size={12}
                    className="text-amber-500 shrink-0"
                    title={`Wird am ${new Date(selectedModel!.deprecationDate!).toLocaleDateString('de-DE')} abgeschaltet`}
                  />
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
