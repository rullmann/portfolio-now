/**
 * Settings view for application configuration.
 */

import { useState, useEffect, useCallback } from 'react';
import { Eye, EyeOff, ExternalLink, Trash2, RefreshCw, Sparkles, Loader2, CheckCircle2, User, AlertTriangle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore, useUIStore, toast, AI_MODELS, getVisionModels } from '../../store';
import type { VisionModel } from '../../store';
import { open } from '@tauri-apps/plugin-shell';
import { clearLogoCache, rebuildFifoLots } from '../../lib/api';
import { AIProviderLogo, AI_PROVIDER_NAMES } from '../../components/common/AIProviderLogo';

// Use VisionModel from store for model type

export function SettingsView() {
  const {
    userName,
    setUserName,
    syncOnlyHeldSecurities,
    setSyncOnlyHeldSecurities,
    deliveryMode,
    setDeliveryMode,
    language,
    setLanguage,
    theme,
    setTheme,
    baseCurrency,
    setBaseCurrency,
    brandfetchApiKey,
    setBrandfetchApiKey,
    finnhubApiKey,
    setFinnhubApiKey,
    coingeckoApiKey,
    setCoingeckoApiKey,
    alphaVantageApiKey,
    setAlphaVantageApiKey,
    twelveDataApiKey,
    setTwelveDataApiKey,
    aiProvider,
    setAiProvider,
    aiModel,
    setAiModel,
    anthropicApiKey,
    setAnthropicApiKey,
    openaiApiKey,
    setOpenaiApiKey,
    geminiApiKey,
    setGeminiApiKey,
    perplexityApiKey,
    setPerplexityApiKey,
  } = useSettingsStore();

  const [showBrandfetchKey, setShowBrandfetchKey] = useState(false);
  const [showFinnhubKey, setShowFinnhubKey] = useState(false);
  const [showCoingeckoKey, setShowCoingeckoKey] = useState(false);
  const [showAlphaVantageKey, setShowAlphaVantageKey] = useState(false);
  const [showTwelveDataKey, setShowTwelveDataKey] = useState(false);
  const [showAiKey, setShowAiKey] = useState(false);
  const [dynamicModels, setDynamicModels] = useState<VisionModel[] | null>(null);
  const [isLoadingModels, setIsLoadingModels] = useState(false);
  const [cacheResult, setCacheResult] = useState<string | null>(null);
  const [isClearing, setIsClearing] = useState(false);
  const [isRebuilding, setIsRebuilding] = useState(false);
  const [rebuildResult, setRebuildResult] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState('');

  const { scrollTarget, setScrollTarget } = useUIStore();

  // Scroll to target section if set
  useEffect(() => {
    if (scrollTarget) {
      const element = document.getElementById(scrollTarget);
      if (element) {
        // Small delay to ensure the page is rendered
        setTimeout(() => {
          element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }, 100);
      }
      setScrollTarget(null);
    }
  }, [scrollTarget, setScrollTarget]);

  const handleRebuildFifo = async () => {
    setIsRebuilding(true);
    setRebuildResult(null);
    try {
      const result = await rebuildFifoLots();
      setRebuildResult(`${result.securitiesProcessed} Securities verarbeitet, ${result.lotsCreated} aktive Lots`);
      toast.success('FIFO-Daten erfolgreich neu berechnet');
    } catch (err) {
      const errorMsg = `Fehler: ${err instanceof Error ? err.message : String(err)}`;
      setRebuildResult(errorMsg);
      toast.error(errorMsg);
    } finally {
      setIsRebuilding(false);
    }
  };

  const handleClearCache = async () => {
    setIsClearing(true);
    setCacheResult(null);

    try {
      const count = await clearLogoCache();
      if (count > 0) {
        setCacheResult(`${count} alte Cache-Dateien gelöscht`);
      } else {
        setCacheResult('Kein Cache vorhanden');
      }
    } catch (err) {
      setCacheResult(`Fehler: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsClearing(false);
    }
  };

  const handleDeleteAllData = async () => {
    if (deleteConfirm !== 'LÖSCHEN') {
      toast.error('Bitte gib "LÖSCHEN" ein, um zu bestätigen');
      return;
    }

    setIsDeleting(true);
    try {
      await invoke('delete_all_data');
      toast.success('Alle Daten wurden gelöscht. Die App wird neu geladen...');
      setDeleteConfirm('');
      // Reload the page to reset all state
      setTimeout(() => {
        window.location.reload();
      }, 1500);
    } catch (err) {
      toast.error(`Fehler beim Löschen: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsDeleting(false);
    }
  };

  // Get current API key for selected provider
  const currentApiKey = aiProvider === 'claude' ? anthropicApiKey
    : aiProvider === 'openai' ? openaiApiKey
    : aiProvider === 'gemini' ? geminiApiKey
    : perplexityApiKey;

  // Fetch available vision models from backend registry
  const fetchAiModels = useCallback(async () => {
    setIsLoadingModels(true);
    try {
      const models = await getVisionModels(aiProvider);
      setDynamicModels(models);

      // Check if current model is in the list
      const currentModelExists = models.some(m => m.id === aiModel);

      if (!currentModelExists && models.length > 0) {
        // Model is deprecated/invalid - migrate to first available model
        const recommendedModel = models[0];
        setAiModel(recommendedModel.id);
        toast.warning(
          `Modell "${aiModel}" nicht mehr verfügbar. Automatisch auf "${recommendedModel.name}" gewechselt.`,
        );
      }
    } catch (err) {
      console.error('Failed to fetch vision models:', err);
      toast.error(`Modelle laden fehlgeschlagen: ${err}`);
      setDynamicModels(null);
    } finally {
      setIsLoadingModels(false);
    }
  }, [aiProvider, aiModel, setAiModel]);

  // Load models when provider changes
  useEffect(() => {
    setDynamicModels(null);
    fetchAiModels();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [aiProvider]);

  // Auto-load models on mount (from backend registry, no API key needed)
  useEffect(() => {
    if (!dynamicModels && !isLoadingModels) {
      fetchAiModels();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Only on mount

  // Check if current model might be outdated (not in static list)
  const modelMightBeOutdated = !AI_MODELS[aiProvider].some(m => m.id === aiModel) && !dynamicModels;

  return (
    <div className="space-y-6">
      {/* Profile Settings */}
      <div className="bg-card rounded-lg border border-border p-6">
        <div className="flex items-center gap-2 mb-4">
          <User size={20} className="text-primary" />
          <h2 className="text-lg font-semibold">Profil</h2>
        </div>
        <div className="space-y-4">
          <div>
            <label className="text-sm font-medium">Dein Name</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              Wird in KI-Konversationen verwendet, um dich persönlich anzusprechen.
            </p>
            <input
              type="text"
              value={userName}
              onChange={(e) => setUserName(e.target.value)}
              placeholder="z.B. Max"
              className="w-full max-w-xs rounded-md border border-input bg-background px-3 py-2"
            />
          </div>
        </div>
      </div>

      {/* Display Settings */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Anzeige</h2>
        <div className="space-y-4">
          <div>
            <label className="text-sm font-medium">Sprache</label>
            <select
              value={language}
              onChange={(e) => setLanguage(e.target.value as 'de' | 'en')}
              className="mt-1 block w-full max-w-xs rounded-md border border-input bg-background px-3 py-2"
            >
              <option value="de">Deutsch</option>
              <option value="en">English</option>
            </select>
          </div>
          <div>
            <label className="text-sm font-medium">Design</label>
            <select
              value={theme}
              onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')}
              className="mt-1 block w-full max-w-xs rounded-md border border-input bg-background px-3 py-2"
            >
              <option value="light">Hell</option>
              <option value="dark">Dunkel</option>
              <option value="system">System</option>
            </select>
          </div>
          <div>
            <label className="text-sm font-medium">Basiswährung</label>
            <select
              value={baseCurrency}
              onChange={(e) => setBaseCurrency(e.target.value)}
              className="mt-1 block w-full max-w-xs rounded-md border border-input bg-background px-3 py-2"
            >
              <option value="EUR">EUR - Euro</option>
              <option value="USD">USD - US Dollar</option>
              <option value="CHF">CHF - Schweizer Franken</option>
              <option value="GBP">GBP - Britisches Pfund</option>
            </select>
          </div>
        </div>
      </div>

      {/* Transaction Settings */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Buchungen</h2>
        <div className="space-y-4">
          <div className="flex items-start justify-between gap-4">
            <div className="flex-1">
              <label className="text-sm font-medium">Einlieferungsmodus</label>
              <p className="text-sm text-muted-foreground mt-0.5">
                Wenn aktiviert, werden neue Käufe standardmäßig als <strong>Einlieferung</strong> erfasst (ohne Kontobuchung).
                Dividenden werden mit einer automatischen Ausbuchung vom Referenzkonto verknüpft, sodass der Kontostand unverändert bleibt.
              </p>
              <p className="text-xs text-muted-foreground mt-2">
                Nützlich wenn Sie Ihr Portfolio nur zur Bestandsverfolgung nutzen und die Geldbewegungen bei Ihrer Bank verwalten.
              </p>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={deliveryMode}
              onClick={() => setDeliveryMode(!deliveryMode)}
              className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 ${
                deliveryMode ? 'bg-primary' : 'bg-muted'
              }`}
            >
              <span
                className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                  deliveryMode ? 'translate-x-5' : 'translate-x-0'
                }`}
              />
            </button>
          </div>
        </div>
      </div>

      {/* Quote Sync Settings */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Kursabruf</h2>
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <label className="text-sm font-medium">Nur Wertpapiere im Bestand</label>
              <p className="text-sm text-muted-foreground mt-0.5">
                Kurse nur für Wertpapiere abrufen, die aktuell im Depot gehalten werden
              </p>
            </div>
            <button
              type="button"
              role="switch"
              aria-checked={syncOnlyHeldSecurities}
              onClick={() => setSyncOnlyHeldSecurities(!syncOnlyHeldSecurities)}
              className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 ${
                syncOnlyHeldSecurities ? 'bg-primary' : 'bg-muted'
              }`}
            >
              <span
                className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                  syncOnlyHeldSecurities ? 'translate-x-5' : 'translate-x-0'
                }`}
              />
            </button>
          </div>

          {/* Finnhub API Key */}
          <div className="pt-4 border-t border-border">
            <label className="text-sm font-medium">Finnhub API Key</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              Ermöglicht Kurse von Finnhub abzurufen (US-Aktien, Echtzeit-Daten).{' '}
              <button
                type="button"
                onClick={() => open('https://finnhub.io/register')}
                className="text-primary hover:underline inline-flex items-center gap-1"
              >
                Kostenlosen API Key erhalten
                <ExternalLink size={12} />
              </button>
            </p>
            <div className="relative max-w-md">
              <input
                type={showFinnhubKey ? 'text' : 'password'}
                value={finnhubApiKey}
                onChange={(e) => setFinnhubApiKey(e.target.value)}
                placeholder="Ihr Finnhub API Key"
                className="w-full rounded-md border border-input bg-background px-3 py-2 pr-10 font-mono text-sm"
              />
              <button
                type="button"
                onClick={() => setShowFinnhubKey(!showFinnhubKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
              >
                {showFinnhubKey ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </div>
            {finnhubApiKey && (
              <p className="text-xs text-green-600 mt-1">
                API Key gespeichert. Finnhub ist jetzt als Kursquelle verfügbar.
              </p>
            )}
          </div>

          {/* CoinGecko API Key */}
          <div className="pt-4 border-t border-border">
            <label className="text-sm font-medium">CoinGecko API Key</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              Für Kryptowährungen (BTC, ETH, etc.). Funktioniert auch ohne Key (limitiert).{' '}
              <button
                type="button"
                onClick={() => open('https://www.coingecko.com/en/api/pricing')}
                className="text-primary hover:underline inline-flex items-center gap-1"
              >
                Demo API Key erhalten (kostenlos)
                <ExternalLink size={12} />
              </button>
            </p>
            <div className="relative max-w-md">
              <input
                type={showCoingeckoKey ? 'text' : 'password'}
                value={coingeckoApiKey}
                onChange={(e) => setCoingeckoApiKey(e.target.value)}
                placeholder="CG-... (optional)"
                className="w-full rounded-md border border-input bg-background px-3 py-2 pr-10 font-mono text-sm"
              />
              <button
                type="button"
                onClick={() => setShowCoingeckoKey(!showCoingeckoKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
              >
                {showCoingeckoKey ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </div>
            {coingeckoApiKey ? (
              <p className="text-xs text-green-600 mt-1">
                API Key gespeichert. Höhere Rate-Limits für Krypto-Kurse aktiv.
              </p>
            ) : (
              <p className="text-xs text-muted-foreground mt-1">
                Ohne Key: max. 10-30 Anfragen/Minute. Mit Demo-Key: 30 Anfragen/Minute.
              </p>
            )}
          </div>

          {/* Alpha Vantage API Key */}
          <div className="pt-4 border-t border-border">
            <label className="text-sm font-medium">Alpha Vantage API Key</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              Weltweite Aktien und ETFs. Kostenloser Tier: 25 Anfragen/Tag.{' '}
              <button
                type="button"
                onClick={() => open('https://www.alphavantage.co/support/#api-key')}
                className="text-primary hover:underline inline-flex items-center gap-1"
              >
                Kostenlosen API Key erhalten
                <ExternalLink size={12} />
              </button>
            </p>
            <div className="relative max-w-md">
              <input
                type={showAlphaVantageKey ? 'text' : 'password'}
                value={alphaVantageApiKey}
                onChange={(e) => setAlphaVantageApiKey(e.target.value)}
                placeholder="Ihr Alpha Vantage API Key"
                className="w-full rounded-md border border-input bg-background px-3 py-2 pr-10 font-mono text-sm"
              />
              <button
                type="button"
                onClick={() => setShowAlphaVantageKey(!showAlphaVantageKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
              >
                {showAlphaVantageKey ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </div>
            {alphaVantageApiKey && (
              <p className="text-xs text-green-600 mt-1">
                API Key gespeichert. Alpha Vantage ist jetzt als Kursquelle verfügbar.
              </p>
            )}
          </div>

          {/* Twelve Data API Key */}
          <div className="pt-4 border-t border-border">
            <label className="text-sm font-medium">Twelve Data API Key</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              Schweizer Aktien (SIX), europäische Märkte. 800 Credits/Tag kostenlos.{' '}
              <button
                type="button"
                onClick={() => open('https://twelvedata.com/pricing')}
                className="text-primary hover:underline inline-flex items-center gap-1"
              >
                Kostenlosen API Key erhalten
                <ExternalLink size={12} />
              </button>
            </p>
            <div className="relative max-w-md">
              <input
                type={showTwelveDataKey ? 'text' : 'password'}
                value={twelveDataApiKey}
                onChange={(e) => setTwelveDataApiKey(e.target.value)}
                placeholder="Ihr Twelve Data API Key"
                className="w-full rounded-md border border-input bg-background px-3 py-2 pr-10 font-mono text-sm"
              />
              <button
                type="button"
                onClick={() => setShowTwelveDataKey(!showTwelveDataKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
              >
                {showTwelveDataKey ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </div>
            {twelveDataApiKey ? (
              <p className="text-xs text-green-600 mt-1">
                API Key gespeichert. Ideal für Schweizer Aktien (z.B. NESN, NOVN, ROG).
              </p>
            ) : (
              <p className="text-xs text-muted-foreground mt-1">
                Symbol-Format: NESN.SW wird automatisch zu NESN:SIX konvertiert.
              </p>
            )}
          </div>
        </div>
      </div>

      {/* AI Analysis Settings */}
      <div id="ai-analysis" className="bg-card rounded-lg border border-border p-6 scroll-mt-4">
        <div className="flex items-center gap-2 mb-4">
          <Sparkles size={20} className="text-primary" />
          <h2 className="text-lg font-semibold">KI-Analyse</h2>
        </div>
        <p className="text-sm text-muted-foreground mb-4">
          Nutze KI zur technischen Analyse deiner Charts. Die KI analysiert das Chartbild und gibt eine strukturierte Einschätzung.
        </p>

        {/* Active Provider Display */}
        {currentApiKey && (
          <div className="mb-6 p-4 rounded-lg bg-muted/50 border border-border">
            <div className="flex items-center gap-3">
              <div className="flex items-center justify-center w-12 h-12 rounded-lg bg-background border border-border shadow-sm">
                <AIProviderLogo provider={aiProvider} size={28} />
              </div>
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-lg">{AI_PROVIDER_NAMES[aiProvider]}</span>
                  <CheckCircle2 size={16} className="text-green-600" />
                </div>
                <div className="text-sm text-muted-foreground">
                  {(dynamicModels || AI_MODELS[aiProvider]).find(m => m.id === aiModel)?.name || aiModel}
                </div>
              </div>
              <div className="text-right text-xs text-muted-foreground">
                <div>Aktiver Provider</div>
                <div className="font-mono">{aiModel.split('-').slice(-1)[0]}</div>
              </div>
            </div>
          </div>
        )}
        <div className="space-y-4">
          {/* Provider Selection with Logos */}
          <div>
            <label className="text-sm font-medium mb-2 block">KI-Anbieter</label>
            <div className="flex flex-wrap gap-2">
              {(['claude', 'openai', 'gemini', 'perplexity'] as const).map((provider) => {
                const isActive = aiProvider === provider;
                const hasKey = provider === 'claude' ? !!anthropicApiKey
                  : provider === 'openai' ? !!openaiApiKey
                  : provider === 'gemini' ? !!geminiApiKey
                  : !!perplexityApiKey;
                return (
                  <button
                    key={provider}
                    type="button"
                    onClick={() => {
                      setAiProvider(provider);
                      setShowAiKey(false);
                    }}
                    className={`flex items-center gap-2 px-4 py-2.5 rounded-lg border-2 transition-all ${
                      isActive
                        ? 'border-primary bg-primary/5 shadow-sm'
                        : 'border-border hover:border-muted-foreground/30 hover:bg-muted/50'
                    }`}
                  >
                    <AIProviderLogo provider={provider} size={20} />
                    <span className={`font-medium ${isActive ? 'text-foreground' : 'text-muted-foreground'}`}>
                      {AI_PROVIDER_NAMES[provider]}
                    </span>
                    {hasKey && (
                      <CheckCircle2 size={14} className="text-green-600 ml-1" />
                    )}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Model Selection */}
          <div className="max-w-lg">
            <label className="text-sm font-medium flex items-center gap-2">
              Modell
              {dynamicModels && (
                <span className="text-xs text-green-600 dark:text-green-400">
                  (Live von API)
                </span>
              )}
              {modelMightBeOutdated && (
                <span className="text-xs text-amber-600 dark:text-amber-400" title="Modell nicht in Standard-Liste - prüfen empfohlen">
                  (Prüfen empfohlen)
                </span>
              )}
            </label>
            <div className="flex gap-2 mt-1">
              <select
                value={aiModel}
                onChange={(e) => setAiModel(e.target.value)}
                className="flex-1 rounded-md border border-input bg-background px-3 py-2"
              >
                {(dynamicModels || AI_MODELS[aiProvider]).map((model) => (
                  <option key={model.id} value={model.id}>
                    {model.name} ({model.description})
                  </option>
                ))}
              </select>
              <button
                type="button"
                onClick={fetchAiModels}
                disabled={isLoadingModels || !currentApiKey}
                title={currentApiKey ? 'Modelle von API laden' : 'API Key erforderlich'}
                className="px-3 py-2 rounded-md border border-input bg-background hover:bg-muted disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {isLoadingModels ? (
                  <Loader2 size={16} className="animate-spin" />
                ) : (
                  <RefreshCw size={16} />
                )}
              </button>
            </div>
            {!dynamicModels && currentApiKey && (
              <p className="text-xs text-muted-foreground mt-1">
                Klicke ↻ um aktuelle Modelle zu laden
              </p>
            )}
          </div>

          {/* Dynamic API Key Field */}
          <div className="pt-4 border-t border-border">
            <label className="text-sm font-medium">
              {aiProvider === 'claude' && 'Anthropic API Key'}
              {aiProvider === 'openai' && 'OpenAI API Key'}
              {aiProvider === 'gemini' && 'Google Gemini API Key'}
            </label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              {aiProvider === 'claude' && (
                <>
                  Für Claude. Sehr gute Chart-Analyse und strukturierte Antworten.{' '}
                  <button
                    type="button"
                    onClick={() => open('https://console.anthropic.com/')}
                    className="text-primary hover:underline inline-flex items-center gap-1"
                  >
                    API Key erhalten
                    <ExternalLink size={12} />
                  </button>
                </>
              )}
              {aiProvider === 'openai' && (
                <>
                  Für GPT-4 Vision. Gute visuelle Analyse-Fähigkeiten.{' '}
                  <button
                    type="button"
                    onClick={() => open('https://platform.openai.com/api-keys')}
                    className="text-primary hover:underline inline-flex items-center gap-1"
                  >
                    API Key erhalten
                    <ExternalLink size={12} />
                  </button>
                </>
              )}
              {aiProvider === 'gemini' && (
                <>
                  Für Gemini. Kostenloser Tier verfügbar.{' '}
                  <button
                    type="button"
                    onClick={() => open('https://aistudio.google.com/app/apikey')}
                    className="text-primary hover:underline inline-flex items-center gap-1"
                  >
                    API Key erhalten
                    <ExternalLink size={12} />
                  </button>
                </>
              )}
            </p>
            <div className="relative max-w-md">
              <input
                type={showAiKey ? 'text' : 'password'}
                value={
                  aiProvider === 'claude'
                    ? anthropicApiKey
                    : aiProvider === 'openai'
                      ? openaiApiKey
                      : aiProvider === 'gemini'
                        ? geminiApiKey
                        : perplexityApiKey
                }
                onChange={(e) => {
                  if (aiProvider === 'claude') setAnthropicApiKey(e.target.value);
                  else if (aiProvider === 'openai') setOpenaiApiKey(e.target.value);
                  else if (aiProvider === 'gemini') setGeminiApiKey(e.target.value);
                  else setPerplexityApiKey(e.target.value);
                }}
                placeholder={
                  aiProvider === 'claude'
                    ? 'sk-ant-...'
                    : aiProvider === 'openai'
                      ? 'sk-...'
                      : aiProvider === 'gemini'
                        ? 'AI...'
                        : 'pplx-...'
                }
                className="w-full rounded-md border border-input bg-background px-3 py-2 pr-10 font-mono text-sm"
              />
              <button
                type="button"
                onClick={() => setShowAiKey(!showAiKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
              >
                {showAiKey ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </div>
            {((aiProvider === 'claude' && anthropicApiKey) ||
              (aiProvider === 'openai' && openaiApiKey) ||
              (aiProvider === 'gemini' && geminiApiKey) ||
              (aiProvider === 'perplexity' && perplexityApiKey)) && (
              <p className="text-xs text-green-600 mt-1">
                API Key gespeichert.{' '}
                {aiProvider === 'claude' && 'Claude'}
                {aiProvider === 'openai' && 'GPT-4'}
                {aiProvider === 'gemini' && 'Gemini'}
                {aiProvider === 'perplexity' && 'Perplexity'} ist als KI-Anbieter verfügbar.
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Logo Settings */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Logos</h2>
        <div className="space-y-4">
          {/* Client ID Input */}
          <div>
            <label className="text-sm font-medium">Brandfetch Client ID</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              Ermöglicht das Laden von Unternehmens- und ETF-Anbieter-Logos.{' '}
              <button
                type="button"
                onClick={() => open('https://docs.brandfetch.com/logo-api/overview')}
                className="text-primary hover:underline inline-flex items-center gap-1"
              >
                Client ID erhalten
                <ExternalLink size={12} />
              </button>
            </p>
            <div className="relative max-w-md">
              <input
                type={showBrandfetchKey ? 'text' : 'password'}
                value={brandfetchApiKey}
                onChange={(e) => setBrandfetchApiKey(e.target.value)}
                placeholder="Ihre Brandfetch Client ID"
                className="w-full rounded-md border border-input bg-background px-3 py-2 pr-10 font-mono text-sm"
              />
              <button
                type="button"
                onClick={() => setShowBrandfetchKey(!showBrandfetchKey)}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground"
              >
                {showBrandfetchKey ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
            </div>
            {brandfetchApiKey && (
              <p className="text-xs text-green-600 mt-1">
                Client ID gespeichert. Logos werden für ETFs und bekannte Aktien geladen.
              </p>
            )}
          </div>

          {/* Cache Cleanup */}
          <div className="pt-2 border-t border-border">
            <label className="text-sm font-medium">Cache leeren</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-3">
              Löscht alle gecachten Logos. Sie werden bei Bedarf automatisch neu geladen.
            </p>
            <button
              type="button"
              onClick={handleClearCache}
              disabled={isClearing}
              className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50"
            >
              <Trash2 size={16} />
              {isClearing ? 'Lösche...' : 'Cache leeren'}
            </button>
            {cacheResult && (
              <p
                className={`text-xs mt-2 ${
                  cacheResult.startsWith('Fehler') ? 'text-destructive' : 'text-muted-foreground'
                }`}
              >
                {cacheResult}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Data Maintenance */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Datenpflege</h2>
        <div className="space-y-4">
          {/* FIFO Rebuild */}
          <div>
            <label className="text-sm font-medium">FIFO Cost Basis</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-3">
              Berechnet alle FIFO-Lots und Einstandskosten neu. Nützlich wenn Einstand oder Gewinn/Verlust nicht korrekt angezeigt werden.
            </p>
            <button
              type="button"
              onClick={handleRebuildFifo}
              disabled={isRebuilding}
              className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-md hover:bg-muted transition-colors disabled:opacity-50"
            >
              <RefreshCw size={16} className={isRebuilding ? 'animate-spin' : ''} />
              {isRebuilding ? 'Berechne neu...' : 'FIFO neu berechnen'}
            </button>
            {rebuildResult && (
              <p
                className={`text-xs mt-2 ${
                  rebuildResult.startsWith('Fehler') ? 'text-destructive' : 'text-green-600'
                }`}
              >
                {rebuildResult}
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Danger Zone */}
      <div className="bg-card rounded-lg border border-destructive/50 p-6">
        <div className="flex items-center gap-2 mb-4">
          <AlertTriangle size={20} className="text-destructive" />
          <h2 className="text-lg font-semibold text-destructive">Gefahrenbereich</h2>
        </div>
        <div className="space-y-4">
          <div>
            <label className="text-sm font-medium">Alle Daten löschen</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-3">
              Löscht alle Daten aus der Datenbank: Wertpapiere, Konten, Depots, Buchungen, Kurse und Einstellungen.
              <strong className="text-destructive"> Diese Aktion kann nicht rückgängig gemacht werden!</strong>
            </p>
            <div className="flex items-end gap-3">
              <div className="flex-1 max-w-xs">
                <label className="text-xs text-muted-foreground mb-1 block">
                  Gib "LÖSCHEN" ein, um zu bestätigen:
                </label>
                <input
                  type="text"
                  value={deleteConfirm}
                  onChange={(e) => setDeleteConfirm(e.target.value)}
                  placeholder="LÖSCHEN"
                  className="w-full rounded-md border border-destructive/50 bg-background px-3 py-2 text-sm"
                />
              </div>
              <button
                type="button"
                onClick={handleDeleteAllData}
                disabled={isDeleting || deleteConfirm !== 'LÖSCHEN'}
                className="flex items-center gap-2 px-4 py-2 text-sm bg-destructive text-destructive-foreground rounded-md hover:bg-destructive/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Trash2 size={16} />
                {isDeleting ? 'Lösche...' : 'Alle Daten löschen'}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
