/**
 * Settings view for application configuration.
 *
 * SECURITY: API keys are stored in Tauri's secure store (not localStorage).
 * The useSecureApiKeys hook handles loading and saving keys securely.
 */

import { useState, useEffect, useCallback } from 'react';
import { Eye, EyeOff, ExternalLink, Trash2, RefreshCw, Sparkles, User, AlertTriangle, Shield, CheckCircle2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore, useUIStore, toast } from '../../store';
import { open } from '@tauri-apps/plugin-shell';
import { clearLogoCache, rebuildFifoLots } from '../../lib/api';
import { AIProviderLogo } from '../../components/common/AIProviderLogo';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import type { ApiKeyType } from '../../lib/secureStorage';
import { AttributeTypeManager } from '../../components/attributes';
import { AiFeatureMatrix } from '../../components/settings';

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
    aiEnabled,
    setAiEnabled,
    anthropicApiKey,
    setAnthropicApiKey,
    openaiApiKey,
    setOpenaiApiKey,
    geminiApiKey,
    setGeminiApiKey,
    perplexityApiKey,
    setPerplexityApiKey,
    divvyDiaryApiKey,
    setDivvyDiaryApiKey,
  } = useSettingsStore();

  // SECURITY: Use secure storage for API keys
  const {
    keys: secureKeys,
    isSecureStorageAvailable: secureStorageAvailable,
    isUsingInsecureFallback,
    setApiKey,
  } = useSecureApiKeys();

  // Use secure keys if available, fall back to store
  const effectiveBrandfetchApiKey = secureStorageAvailable ? secureKeys.brandfetchApiKey : brandfetchApiKey;
  const effectiveFinnhubApiKey = secureStorageAvailable ? secureKeys.finnhubApiKey : finnhubApiKey;
  const effectiveCoingeckoApiKey = secureStorageAvailable ? secureKeys.coingeckoApiKey : coingeckoApiKey;
  const effectiveAlphaVantageApiKey = secureStorageAvailable ? secureKeys.alphaVantageApiKey : alphaVantageApiKey;
  const effectiveTwelveDataApiKey = secureStorageAvailable ? secureKeys.twelveDataApiKey : twelveDataApiKey;
  const effectiveAnthropicApiKey = secureStorageAvailable ? secureKeys.anthropicApiKey : anthropicApiKey;
  const effectiveOpenaiApiKey = secureStorageAvailable ? secureKeys.openaiApiKey : openaiApiKey;
  const effectiveGeminiApiKey = secureStorageAvailable ? secureKeys.geminiApiKey : geminiApiKey;
  const effectivePerplexityApiKey = secureStorageAvailable ? secureKeys.perplexityApiKey : perplexityApiKey;

  // Secure key setters that store in both secure storage and Zustand
  const handleSetApiKey = useCallback(async (keyType: ApiKeyType, value: string) => {
    await setApiKey(keyType, value);
    // Also update Zustand for immediate UI access
    switch (keyType) {
      case 'brandfetch': setBrandfetchApiKey(value); break;
      case 'finnhub': setFinnhubApiKey(value); break;
      case 'coingecko': setCoingeckoApiKey(value); break;
      case 'alphaVantage': setAlphaVantageApiKey(value); break;
      case 'twelveData': setTwelveDataApiKey(value); break;
      case 'anthropic': setAnthropicApiKey(value); break;
      case 'openai': setOpenaiApiKey(value); break;
      case 'gemini': setGeminiApiKey(value); break;
      case 'perplexity': setPerplexityApiKey(value); break;
    }
  }, [setApiKey, setBrandfetchApiKey, setFinnhubApiKey, setCoingeckoApiKey, setAlphaVantageApiKey, setTwelveDataApiKey, setAnthropicApiKey, setOpenaiApiKey, setGeminiApiKey, setPerplexityApiKey]);

  const [showBrandfetchKey, setShowBrandfetchKey] = useState(false);
  const [showFinnhubKey, setShowFinnhubKey] = useState(false);
  const [showCoingeckoKey, setShowCoingeckoKey] = useState(false);
  const [showAlphaVantageKey, setShowAlphaVantageKey] = useState(false);
  const [showTwelveDataKey, setShowTwelveDataKey] = useState(false);
  const [showAiKeys, setShowAiKeys] = useState<Record<string, boolean>>({});
  const [cacheResult, setCacheResult] = useState<string | null>(null);
  const [isClearing, setIsClearing] = useState(false);
  const [isRebuilding, setIsRebuilding] = useState(false);
  const [rebuildResult, setRebuildResult] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState('');
  const [attributesExpanded, setAttributesExpanded] = useState(false);

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

  // Toggle visibility of AI provider API key
  const toggleShowAiKey = (provider: string) => {
    setShowAiKeys(prev => ({ ...prev, [provider]: !prev[provider] }));
  };

  // Check if any AI provider has an API key
  const hasAnyAiKey = !!(effectiveAnthropicApiKey || effectiveOpenaiApiKey || effectiveGeminiApiKey || effectivePerplexityApiKey);

  return (
    <div className="space-y-6">
      {/* SECURITY WARNING: localStorage fallback active */}
      {isUsingInsecureFallback && (
        <div className="bg-amber-50 dark:bg-amber-950/50 border border-amber-200 dark:border-amber-800 rounded-lg p-4">
          <div className="flex items-start gap-3">
            <AlertTriangle size={20} className="text-amber-600 dark:text-amber-400 shrink-0 mt-0.5" />
            <div>
              <h3 className="font-semibold text-amber-800 dark:text-amber-200">
                Sichere Speicherung nicht verfügbar
              </h3>
              <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                API-Schlüssel werden im Browser-Speicher (localStorage) abgelegt. Dies ist weniger sicher als die normale Tauri-Speicherung.
                Bei Sicherheitsbedenken können Sie die Schlüssel nur für die aktuelle Sitzung eingeben und danach wieder löschen.
              </p>
              <p className="text-xs text-amber-600 dark:text-amber-400 mt-2">
                Mögliche Ursachen: Ausführung im Browser statt als Desktop-App, fehlende Dateiberechtigungen.
              </p>
            </div>
          </div>
        </div>
      )}

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
                value={effectiveFinnhubApiKey}
                onChange={(e) => handleSetApiKey('finnhub', e.target.value)}
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
            {effectiveFinnhubApiKey && (
              <p className="text-xs text-green-600 mt-1 flex items-center gap-1">
                {secureStorageAvailable && <Shield size={12} />}
                API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert. Finnhub ist jetzt als Kursquelle verfügbar.
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
                value={effectiveCoingeckoApiKey}
                onChange={(e) => handleSetApiKey('coingecko', e.target.value)}
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
            {effectiveCoingeckoApiKey ? (
              <p className="text-xs text-green-600 mt-1 flex items-center gap-1">
                {secureStorageAvailable && <Shield size={12} />}
                API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert. Höhere Rate-Limits für Krypto-Kurse aktiv.
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
                value={effectiveAlphaVantageApiKey}
                onChange={(e) => handleSetApiKey('alphaVantage', e.target.value)}
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
            {effectiveAlphaVantageApiKey && (
              <p className="text-xs text-green-600 mt-1 flex items-center gap-1">
                {secureStorageAvailable && <Shield size={12} />}
                API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert. Alpha Vantage ist jetzt als Kursquelle verfügbar.
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
                value={effectiveTwelveDataApiKey}
                onChange={(e) => handleSetApiKey('twelveData', e.target.value)}
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
            {effectiveTwelveDataApiKey ? (
              <p className="text-xs text-green-600 mt-1 flex items-center gap-1">
                {secureStorageAvailable && <Shield size={12} />}
                API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert. Ideal für Schweizer Aktien (z.B. NESN, NOVN, ROG).
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
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Sparkles size={20} className="text-primary" />
            <h2 className="text-lg font-semibold">KI-Analyse</h2>
          </div>
          {/* Global AI Enable Toggle */}
          <button
            type="button"
            role="switch"
            aria-checked={aiEnabled}
            onClick={() => setAiEnabled(!aiEnabled)}
            className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 ${
              aiEnabled ? 'bg-primary' : 'bg-muted'
            }`}
            title={aiEnabled ? 'KI-Features deaktivieren' : 'KI-Features aktivieren'}
          >
            <span
              className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                aiEnabled ? 'translate-x-5' : 'translate-x-0'
              }`}
            />
          </button>
        </div>

        {!aiEnabled ? (
          <div className="rounded-lg border border-dashed border-amber-500/30 bg-amber-500/5 p-4 text-center">
            <p className="text-sm text-amber-600 dark:text-amber-400">
              KI-Features sind deaktiviert. Aktiviere den Schalter oben, um Chat, Chart-Analyse und mehr zu nutzen.
            </p>
          </div>
        ) : (
          <div className="space-y-5">
            {/* API Keys - Compact 2-column Grid */}
            <div>
              <div className="flex items-center justify-between mb-3">
                <h3 className="text-sm font-medium">API-Keys</h3>
                {hasAnyAiKey && secureStorageAvailable && (
                  <span className="text-xs text-green-600 flex items-center gap-1">
                    <Shield size={10} />
                    Sicher gespeichert
                  </span>
                )}
              </div>
              <div className="grid grid-cols-2 gap-3">
                {/* Claude */}
                <div className="flex items-center gap-2 p-2 rounded-lg border border-border bg-background">
                  <AIProviderLogo provider="claude" size={16} />
                  <div className="flex-1 min-w-0">
                    <input
                      type={showAiKeys.claude ? 'text' : 'password'}
                      value={effectiveAnthropicApiKey}
                      onChange={(e) => handleSetApiKey('anthropic', e.target.value)}
                      placeholder="Claude API Key"
                      className="w-full bg-transparent text-xs font-mono focus:outline-none"
                    />
                  </div>
                  {effectiveAnthropicApiKey ? (
                    <CheckCircle2 size={14} className="text-green-600 shrink-0" />
                  ) : (
                    <button
                      type="button"
                      onClick={() => open('https://console.anthropic.com/')}
                      className="text-xs text-primary hover:underline shrink-0"
                    >
                      Holen
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => toggleShowAiKey('claude')}
                    className="p-0.5 text-muted-foreground hover:text-foreground shrink-0"
                  >
                    {showAiKeys.claude ? <EyeOff size={12} /> : <Eye size={12} />}
                  </button>
                </div>

                {/* OpenAI */}
                <div className="flex items-center gap-2 p-2 rounded-lg border border-border bg-background">
                  <AIProviderLogo provider="openai" size={16} />
                  <div className="flex-1 min-w-0">
                    <input
                      type={showAiKeys.openai ? 'text' : 'password'}
                      value={effectiveOpenaiApiKey}
                      onChange={(e) => handleSetApiKey('openai', e.target.value)}
                      placeholder="OpenAI API Key"
                      className="w-full bg-transparent text-xs font-mono focus:outline-none"
                    />
                  </div>
                  {effectiveOpenaiApiKey ? (
                    <CheckCircle2 size={14} className="text-green-600 shrink-0" />
                  ) : (
                    <button
                      type="button"
                      onClick={() => open('https://platform.openai.com/api-keys')}
                      className="text-xs text-primary hover:underline shrink-0"
                    >
                      Holen
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => toggleShowAiKey('openai')}
                    className="p-0.5 text-muted-foreground hover:text-foreground shrink-0"
                  >
                    {showAiKeys.openai ? <EyeOff size={12} /> : <Eye size={12} />}
                  </button>
                </div>

                {/* Gemini */}
                <div className="flex items-center gap-2 p-2 rounded-lg border border-border bg-background">
                  <AIProviderLogo provider="gemini" size={16} />
                  <div className="flex-1 min-w-0">
                    <input
                      type={showAiKeys.gemini ? 'text' : 'password'}
                      value={effectiveGeminiApiKey}
                      onChange={(e) => handleSetApiKey('gemini', e.target.value)}
                      placeholder="Gemini API Key (Free)"
                      className="w-full bg-transparent text-xs font-mono focus:outline-none"
                    />
                  </div>
                  {effectiveGeminiApiKey ? (
                    <CheckCircle2 size={14} className="text-green-600 shrink-0" />
                  ) : (
                    <button
                      type="button"
                      onClick={() => open('https://aistudio.google.com/app/apikey')}
                      className="text-xs text-primary hover:underline shrink-0"
                    >
                      Holen
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => toggleShowAiKey('gemini')}
                    className="p-0.5 text-muted-foreground hover:text-foreground shrink-0"
                  >
                    {showAiKeys.gemini ? <EyeOff size={12} /> : <Eye size={12} />}
                  </button>
                </div>

                {/* Perplexity */}
                <div className="flex items-center gap-2 p-2 rounded-lg border border-border bg-background">
                  <AIProviderLogo provider="perplexity" size={16} />
                  <div className="flex-1 min-w-0">
                    <input
                      type={showAiKeys.perplexity ? 'text' : 'password'}
                      value={effectivePerplexityApiKey}
                      onChange={(e) => handleSetApiKey('perplexity', e.target.value)}
                      placeholder="Perplexity API Key"
                      className="w-full bg-transparent text-xs font-mono focus:outline-none"
                    />
                  </div>
                  {effectivePerplexityApiKey ? (
                    <CheckCircle2 size={14} className="text-green-600 shrink-0" />
                  ) : (
                    <button
                      type="button"
                      onClick={() => open('https://www.perplexity.ai/settings/api')}
                      className="text-xs text-primary hover:underline shrink-0"
                    >
                      Holen
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => toggleShowAiKey('perplexity')}
                    className="p-0.5 text-muted-foreground hover:text-foreground shrink-0"
                  >
                    {showAiKeys.perplexity ? <EyeOff size={12} /> : <Eye size={12} />}
                  </button>
                </div>
              </div>
            </div>

            {/* Feature Matrix */}
            <div>
              <h3 className="text-sm font-medium mb-3">Funktionen konfigurieren</h3>
              <AiFeatureMatrix
                apiKeys={{
                  anthropicApiKey: effectiveAnthropicApiKey,
                  openaiApiKey: effectiveOpenaiApiKey,
                  geminiApiKey: effectiveGeminiApiKey,
                  perplexityApiKey: effectivePerplexityApiKey,
                }}
              />
            </div>
          </div>
        )}
      </div>

      {/* External Services */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Externe Dienste</h2>
        <div className="space-y-4">
          {/* DivvyDiary API Key */}
          <div>
            <div className="flex items-center gap-2 mb-1">
              <label className="text-sm font-medium">DivvyDiary API-Key</label>
            </div>
            <p className="text-sm text-muted-foreground mt-0.5 mb-2">
              Ermöglicht den Export Ihres Portfolios zu DivvyDiary (Dividenden-Kalender).{' '}
              <button
                type="button"
                onClick={() => open('https://divvydiary.com/settings')}
                className="text-primary hover:underline inline-flex items-center gap-1"
              >
                API-Key in DivvyDiary erhalten
                <ExternalLink size={12} />
              </button>
            </p>
            <div className="relative max-w-md">
              <input
                type="password"
                value={divvyDiaryApiKey}
                onChange={(e) => setDivvyDiaryApiKey(e.target.value)}
                placeholder="Ihr DivvyDiary API-Key"
                className="w-full rounded-md border border-input bg-background px-3 py-2 pr-10 font-mono text-sm"
              />
            </div>
            {divvyDiaryApiKey && (
              <p className="text-xs text-green-600 mt-1 flex items-center gap-1">
                <Shield size={12} />
                API-Key gespeichert. Sie können jetzt Portfolios zu DivvyDiary exportieren (Header &rarr; Exportieren).
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
                value={effectiveBrandfetchApiKey}
                onChange={(e) => handleSetApiKey('brandfetch', e.target.value)}
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
            {effectiveBrandfetchApiKey && (
              <p className="text-xs text-green-600 mt-1 flex items-center gap-1">
                {secureStorageAvailable && <Shield size={12} />}
                Client ID {secureStorageAvailable ? 'sicher ' : ''}gespeichert. Logos werden für ETFs und bekannte Aktien geladen.
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

      {/* Custom Attributes */}
      <div className="bg-card rounded-lg border border-border p-6">
        <h2 className="text-lg font-semibold mb-4">Erweiterte Daten</h2>
        <AttributeTypeManager
          expanded={attributesExpanded}
          onToggleExpand={() => setAttributesExpanded(!attributesExpanded)}
        />
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
