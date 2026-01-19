/**
 * AI Feature Matrix - Configure AI provider and model per feature.
 *
 * Compact table-style layout for configuring each AI feature with its own
 * provider and model. Only providers with configured API keys are shown.
 */

import { useMemo, useEffect } from 'react';
import {
  BarChart3,
  Lightbulb,
  MessageSquare,
  FileText,
  FileSpreadsheet,
  Eye,
  AlertCircle,
} from 'lucide-react';
import {
  useSettingsStore,
  AI_FEATURES,
  AI_MODELS,
  DEFAULT_MODELS,
  type AiFeatureId,
  type AiProvider,
} from '../../store';
import { AIProviderLogo } from '../common/AIProviderLogo';

// Map feature icon names to components
const FEATURE_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  BarChart3,
  Lightbulb,
  MessageSquare,
  FileText,
  FileSpreadsheet,
};

// Provider display names
const PROVIDER_NAMES: Record<AiProvider, string> = {
  claude: 'Claude',
  openai: 'OpenAI',
  gemini: 'Gemini',
  perplexity: 'Perplexity',
};

interface AiFeatureMatrixProps {
  /** API keys per provider (from secure storage) */
  apiKeys: {
    anthropicApiKey: string;
    openaiApiKey: string;
    geminiApiKey: string;
    perplexityApiKey: string;
  };
}

export function AiFeatureMatrix({ apiKeys }: AiFeatureMatrixProps) {
  const { aiFeatureSettings, setAiFeatureSetting } = useSettingsStore();

  // Determine which providers have API keys configured
  const availableProviders = useMemo(() => {
    const providers: AiProvider[] = [];
    if (apiKeys.anthropicApiKey?.trim()) providers.push('claude');
    if (apiKeys.openaiApiKey?.trim()) providers.push('openai');
    if (apiKeys.geminiApiKey?.trim()) providers.push('gemini');
    if (apiKeys.perplexityApiKey?.trim()) providers.push('perplexity');
    return providers;
  }, [apiKeys]);

  const hasAnyProvider = availableProviders.length > 0;

  // Get models for a specific provider
  const getModelsForProvider = (provider: AiProvider) => {
    const models = AI_MODELS[provider] || [];
    return [...models];
  };

  // Auto-migrate features when providers become unavailable
  useEffect(() => {
    if (!hasAnyProvider) return;

    AI_FEATURES.forEach((feature) => {
      const config = aiFeatureSettings[feature.id];
      if (!config) return;

      // Check if current provider is still available
      if (!availableProviders.includes(config.provider)) {
        // Auto-migrate to first available provider
        const newProvider = availableProviders[0];
        const models = getModelsForProvider(newProvider);
        setAiFeatureSetting(feature.id, {
          provider: newProvider,
          model: models[0]?.id || DEFAULT_MODELS[newProvider],
        });
      }
    });
  }, [availableProviders, aiFeatureSettings, setAiFeatureSetting, hasAnyProvider]);

  // Handle provider change for a feature
  const handleProviderChange = (featureId: AiFeatureId, provider: AiProvider) => {
    const models = getModelsForProvider(provider);
    const defaultModel = models[0]?.id || DEFAULT_MODELS[provider];
    setAiFeatureSetting(featureId, { provider, model: defaultModel });
  };

  // Handle model change for a feature
  const handleModelChange = (featureId: AiFeatureId, model: string) => {
    const currentConfig = aiFeatureSettings[featureId];
    setAiFeatureSetting(featureId, { ...currentConfig, model });
  };

  // Get validated config (fallback if provider unavailable)
  const getValidatedConfig = (featureId: AiFeatureId) => {
    const config = aiFeatureSettings[featureId];

    if (!availableProviders.includes(config.provider) && availableProviders.length > 0) {
      return {
        provider: availableProviders[0],
        model: DEFAULT_MODELS[availableProviders[0]],
        needsMigration: true,
      };
    }

    const models = getModelsForProvider(config.provider);
    if (!models.some(m => m.id === config.model) && models.length > 0) {
      return {
        provider: config.provider,
        model: models[0].id,
        needsMigration: false,
      };
    }

    return { ...config, needsMigration: false };
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
      <div className="grid grid-cols-[1fr,140px,180px] gap-2 px-3 py-2 bg-muted/50 text-xs font-medium text-muted-foreground border-b border-border">
        <div>Funktion</div>
        <div>Provider</div>
        <div>Modell</div>
      </div>

      {/* Rows */}
      <div className="divide-y divide-border">
        {AI_FEATURES.map((feature) => {
          const Icon = FEATURE_ICONS[feature.icon] || FileText;
          const config = getValidatedConfig(feature.id);
          const models = getModelsForProvider(config.provider);

          return (
            <div
              key={feature.id}
              className="grid grid-cols-[1fr,140px,180px] gap-2 px-3 py-2.5 items-center hover:bg-muted/30 transition-colors"
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
              </div>

              {/* Provider Dropdown */}
              <div className="relative">
                <select
                  value={config.provider}
                  onChange={(e) => handleProviderChange(feature.id, e.target.value as AiProvider)}
                  className="w-full pl-7 pr-2 py-1.5 text-xs border border-border rounded bg-background appearance-none cursor-pointer hover:border-primary/50 focus:outline-none focus:ring-1 focus:ring-primary/30"
                >
                  {availableProviders.map((provider) => (
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
            </div>
          );
        })}
      </div>
    </div>
  );
}
