/**
 * AI Feature Migration Modal
 *
 * Shown when an API key is removed and AI features that were using that provider
 * need to be migrated to another available provider.
 */

import { useState } from 'react';
import { ArrowRight, AlertCircle } from 'lucide-react';
import {
  useSettingsStore,
  AI_FEATURES,
  AI_MODELS,
  DEFAULT_MODELS,
  type AiProvider,
} from '../../store';
import { AIProviderLogo } from '../common/AIProviderLogo';
import { useEscapeKey } from '../../lib/hooks';

// Provider display names
const PROVIDER_NAMES: Record<AiProvider, string> = {
  claude: 'Claude',
  openai: 'OpenAI',
  gemini: 'Gemini',
  perplexity: 'Perplexity',
};

export function AiMigrationModal() {
  const {
    pendingFeatureMigration,
    clearPendingFeatureMigration,
    setAiFeatureSetting,
  } = useSettingsStore();

  const [selectedProvider, setSelectedProvider] = useState<AiProvider | null>(
    pendingFeatureMigration?.availableProviders[0] || null
  );

  useEscapeKey(!!pendingFeatureMigration, clearPendingFeatureMigration);

  if (!pendingFeatureMigration) return null;

  const { features, fromProvider, availableProviders } = pendingFeatureMigration;

  // Get feature names for display
  const featureNames = features
    .map((id) => AI_FEATURES.find((f) => f.id === id)?.name || id)
    .join(', ');

  const handleMigrate = () => {
    if (!selectedProvider) return;

    // Get the default model for the selected provider
    const models = AI_MODELS[selectedProvider] || [];
    const defaultModel = models[0]?.id || DEFAULT_MODELS[selectedProvider];

    // Migrate all affected features to the selected provider
    features.forEach((featureId) => {
      setAiFeatureSetting(featureId, {
        provider: selectedProvider,
        model: defaultModel,
      });
    });

    clearPendingFeatureMigration();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={clearPendingFeatureMigration}
      />

      {/* Modal */}
      <div className="relative bg-card border border-border rounded-lg shadow-lg w-full max-w-md mx-4 overflow-hidden">
        {/* Header */}
        <div className="bg-amber-500/10 p-6 text-center">
          <div className="w-16 h-16 bg-amber-500/20 rounded-full flex items-center justify-center mx-auto mb-4">
            <AlertCircle size={32} className="text-amber-500" />
          </div>
          <h2 className="text-xl font-semibold">KI-Provider nicht verfügbar</h2>
          <p className="text-muted-foreground mt-2 text-sm">
            Der API-Key für {PROVIDER_NAMES[fromProvider]} wurde entfernt.
          </p>
        </div>

        {/* Content */}
        <div className="p-6 space-y-4">
          <div className="bg-muted/50 rounded-lg p-4">
            <p className="text-sm">
              <span className="font-medium">Betroffene Funktionen:</span>
              <br />
              <span className="text-muted-foreground">{featureNames}</span>
            </p>
          </div>

          <div>
            <label className="block text-sm font-medium mb-3">
              Zu welchem Provider migrieren?
            </label>
            <div className="space-y-2">
              {availableProviders.map((provider) => (
                <button
                  key={provider}
                  type="button"
                  onClick={() => setSelectedProvider(provider)}
                  className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                    selectedProvider === provider
                      ? 'border-primary bg-primary/5'
                      : 'border-border hover:border-primary/50 hover:bg-muted/30'
                  }`}
                >
                  <AIProviderLogo provider={provider} size={24} />
                  <span className="font-medium">{PROVIDER_NAMES[provider]}</span>
                  {selectedProvider === provider && (
                    <span className="ml-auto text-primary text-sm">Ausgewählt</span>
                  )}
                </button>
              ))}
            </div>
          </div>

          <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground pt-2">
            <AIProviderLogo provider={fromProvider} size={16} />
            <span>{PROVIDER_NAMES[fromProvider]}</span>
            <ArrowRight size={14} />
            {selectedProvider && (
              <>
                <AIProviderLogo provider={selectedProvider} size={16} />
                <span>{PROVIDER_NAMES[selectedProvider]}</span>
              </>
            )}
          </div>

          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={clearPendingFeatureMigration}
              className="flex-1 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
            >
              Später
            </button>
            <button
              type="button"
              onClick={handleMigrate}
              disabled={!selectedProvider}
              className="flex-1 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Migrieren
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
