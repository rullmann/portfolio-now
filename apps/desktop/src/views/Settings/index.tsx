/**
 * Settings view with sidebar navigation.
 *
 * SECURITY: API keys are stored in Tauri's secure store (not localStorage).
 * The useSecureApiKeys hook handles loading and saving keys securely.
 */

import { useState, useEffect, useCallback } from 'react';
import {
  Eye,
  EyeOff,
  ExternalLink,
  Trash2,
  RefreshCw,
  Sparkles,
  User,
  AlertTriangle,
  Shield,
  CheckCircle2,
  Search,
  Receipt,
  TrendingUp,
  Link,
  Database,
  Camera,
  X,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useSettingsStore, useUIStore, toast, type ChartTimeRange } from '../../store';
import { open } from '@tauri-apps/plugin-shell';
import { clearLogoCache, rebuildFifoLots, validateAllSecurities, getValidationStatus, getUserProfilePicture, setUserProfilePicture } from '../../lib/api';
import { AIProviderLogo } from '../../components/common/AIProviderLogo';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import type { ApiKeyType } from '../../lib/secureStorage';
import { AttributeTypeManager } from '../../components/attributes';
import { AiFeatureMatrix, UserTemplatesSettings } from '../../components/settings';
import type { ValidationStatusSummary, ValidationResponse } from '../../lib/types';
import { cn } from '../../lib/utils';
import { MessageSquarePlus } from 'lucide-react';

// Settings sections configuration
const SETTINGS_SECTIONS = [
  { id: 'general', name: 'Allgemein', icon: User },
  { id: 'transactions', name: 'Buchungen', icon: Receipt },
  { id: 'quotes', name: 'Kurse', icon: TrendingUp },
  { id: 'ai', name: 'KI-Analyse', icon: Sparkles },
  { id: 'queries', name: 'Eigene Abfragen', icon: MessageSquarePlus },
  { id: 'services', name: 'Dienste', icon: Link },
  { id: 'data', name: 'Daten', icon: Database },
  { id: 'danger', name: 'Gefahrenzone', icon: AlertTriangle },
] as const;

type SettingsSection = typeof SETTINGS_SECTIONS[number]['id'];

export function SettingsView() {
  const [activeSection, setActiveSection] = useState<SettingsSection>('general');

  const {
    userName,
    setUserName,
    profilePicture,
    setProfilePicture,
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
    symbolValidation,
    setSymbolValidationSettings,
    aiFeatureSettings,
    chatContextSize,
    setChatContextSize,
    defaultChartTimeRange,
    setDefaultChartTimeRange,
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
  const [isValidating, setIsValidating] = useState(false);
  const [validationStatus, setValidationStatus] = useState<ValidationStatusSummary | null>(null);
  const [validationResult, setValidationResult] = useState<ValidationResponse | null>(null);
  const [forceValidation, setForceValidation] = useState(false);
  const [isUploadingPicture, setIsUploadingPicture] = useState(false);

  const { scrollTarget, setScrollTarget } = useUIStore();

  // Load profile picture from database on mount
  useEffect(() => {
    const loadProfilePicture = async () => {
      try {
        const picture = await getUserProfilePicture();
        setProfilePicture(picture);
      } catch (err) {
        console.error('Failed to load profile picture:', err);
      }
    };
    loadProfilePicture();
  }, [setProfilePicture]);

  // Handle profile picture upload
  const handleUploadProfilePicture = async () => {
    setIsUploadingPicture(true);
    try {
      // First open a file dialog to select an image
      const { open: openDialog } = await import('@tauri-apps/plugin-dialog');
      const selectedPath = await openDialog({
        multiple: false,
        filters: [{ name: 'Bilder', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
        title: 'Profilbild auswählen',
      });

      if (!selectedPath) {
        // User cancelled
        return;
      }

      // Read the image as base64
      const result = await invoke<{ data: string; mimeType: string; filename: string }>('read_image_as_base64', { path: selectedPath });
      if (result) {
        const pictureData = `data:${result.mimeType};base64,${result.data}`;
        await setUserProfilePicture(pictureData);
        setProfilePicture(pictureData);
        toast.success('Profilbild erfolgreich hochgeladen');
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (!errorMsg.includes('cancelled') && !errorMsg.includes('abgebrochen')) {
        toast.error(`Fehler beim Hochladen: ${errorMsg}`);
      }
    } finally {
      setIsUploadingPicture(false);
    }
  };

  // Handle profile picture removal
  const handleRemoveProfilePicture = async () => {
    try {
      await setUserProfilePicture(null);
      setProfilePicture(null);
      toast.success('Profilbild entfernt');
    } catch (err) {
      toast.error(`Fehler beim Entfernen: ${err instanceof Error ? err.message : String(err)}`);
    }
  };

  // Map scrollTarget to section
  useEffect(() => {
    if (scrollTarget) {
      const sectionMap: Record<string, SettingsSection> = {
        'ai-analysis': 'ai',
        'profile': 'general',
        'quotes': 'quotes',
        'transactions': 'transactions',
        'services': 'services',
        'data': 'data',
        'danger': 'danger',
      };
      const section = sectionMap[scrollTarget];
      if (section) {
        setActiveSection(section);
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

  // Load validation status on mount
  useEffect(() => {
    loadValidationStatus();
  }, [symbolValidation.validateOnlyHeld]);

  const loadValidationStatus = async () => {
    try {
      const status = await getValidationStatus(symbolValidation.validateOnlyHeld);
      setValidationStatus(status);
    } catch {
      // Status loading is optional
    }
  };

  const handleRunValidation = async () => {
    setIsValidating(true);
    setValidationResult(null);
    try {
      // Collect API keys for validation
      const validationApiKeys = {
        coingeckoApiKey: effectiveCoingeckoApiKey || undefined,
        finnhubApiKey: effectiveFinnhubApiKey || undefined,
        alphaVantageApiKey: effectiveAlphaVantageApiKey || undefined,
        twelveDataApiKey: effectiveTwelveDataApiKey || undefined,
      };

      // Get AI config from feature settings (use portfolioInsights provider for validation)
      const aiConfig = symbolValidation.enableAiFallback ? {
        enabled: true,
        provider: aiFeatureSettings.portfolioInsights.provider,
        model: aiFeatureSettings.portfolioInsights.model,
        apiKey: (() => {
          switch (aiFeatureSettings.portfolioInsights.provider) {
            case 'claude': return effectiveAnthropicApiKey;
            case 'openai': return effectiveOpenaiApiKey;
            case 'gemini': return effectiveGeminiApiKey;
            case 'perplexity': return effectivePerplexityApiKey;
            default: return '';
          }
        })(),
      } : undefined;

      const result = await validateAllSecurities({
        onlyHeld: symbolValidation.validateOnlyHeld,
        force: forceValidation,
        apiKeys: validationApiKeys,
        aiConfig: aiConfig,
      });

      setValidationResult(result);
      setSymbolValidationSettings({ lastAutoValidation: new Date().toISOString() });
      await loadValidationStatus();

      const summary = result.summary;
      if (summary && (summary.validated > 0 || summary.aiSuggested > 0)) {
        toast.success(`Validierung abgeschlossen: ${summary.validated} validiert, ${summary.aiSuggested} KI-Vorschläge`);
      } else if (summary && summary.failed > 0) {
        toast.warning(`${summary.failed} Wertpapiere konnten nicht validiert werden`);
      } else {
        toast.info('Alle Kursquellen sind bereits korrekt konfiguriert');
      }
    } catch (err) {
      toast.error(`Validierungsfehler: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setIsValidating(false);
    }
  };

  // Toggle visibility of AI provider API key
  const toggleShowAiKey = (provider: string) => {
    setShowAiKeys(prev => ({ ...prev, [provider]: !prev[provider] }));
  };

  // Check if any AI provider has an API key
  const hasAnyAiKey = !!(effectiveAnthropicApiKey || effectiveOpenaiApiKey || effectiveGeminiApiKey || effectivePerplexityApiKey);

  // Handle chat context size change with validation
  const handleChatContextSizeChange = (value: string) => {
    const num = parseInt(value, 10);
    if (!isNaN(num)) {
      const clamped = Math.min(500, Math.max(5, num));
      setChatContextSize(clamped);
    }
  };

  return (
    <div className="flex h-full">
      {/* Sidebar */}
      <aside className="w-56 shrink-0 border-r border-border bg-muted/30">
        <div className="p-4">
          <h1 className="text-lg font-semibold mb-4">Einstellungen</h1>
          <nav className="space-y-1">
            {SETTINGS_SECTIONS.map(section => (
              <button
                key={section.id}
                onClick={() => setActiveSection(section.id)}
                className={cn(
                  "w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors",
                  activeSection === section.id
                    ? "bg-primary text-primary-foreground"
                    : "hover:bg-muted text-muted-foreground hover:text-foreground",
                  section.id === 'danger' && activeSection !== section.id && "text-destructive hover:text-destructive"
                )}
              >
                <section.icon size={18} />
                {section.name}
              </button>
            ))}
          </nav>
        </div>
      </aside>

      {/* Content */}
      <main className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl">
          {/* SECURITY WARNING: localStorage fallback active */}
          {isUsingInsecureFallback && (
            <div className="bg-amber-50 dark:bg-amber-950/50 border border-amber-200 dark:border-amber-800 rounded-lg p-4 mb-6">
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

          {/* General Section */}
          {activeSection === 'general' && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-1">Allgemein</h2>
                <p className="text-sm text-muted-foreground">Profil und Anzeigeeinstellungen</p>
              </div>

              {/* Profile */}
              <div className="bg-card rounded-lg border border-border p-6">
                <div className="flex items-center gap-2 mb-4">
                  <User size={20} className="text-primary" />
                  <h3 className="text-lg font-semibold">Profil</h3>
                </div>

                <div className="flex items-start gap-6">
                  {/* Profile Picture */}
                  <div className="flex flex-col items-center gap-2">
                    <div className="relative group">
                      {profilePicture ? (
                        <div className="relative">
                          <img
                            src={profilePicture}
                            alt="Profilbild"
                            className="w-20 h-20 rounded-full object-cover border-2 border-border"
                          />
                          <button
                            onClick={handleRemoveProfilePicture}
                            className="absolute -top-1 -right-1 w-6 h-6 bg-destructive text-destructive-foreground rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
                            title="Profilbild entfernen"
                          >
                            <X size={14} />
                          </button>
                        </div>
                      ) : (
                        <div className="w-20 h-20 rounded-full bg-muted flex items-center justify-center border-2 border-dashed border-border">
                          <User size={32} className="text-muted-foreground" />
                        </div>
                      )}
                    </div>
                    <button
                      onClick={handleUploadProfilePicture}
                      disabled={isUploadingPicture}
                      className="flex items-center gap-1.5 text-xs text-primary hover:text-primary/80 transition-colors disabled:opacity-50"
                    >
                      <Camera size={14} />
                      {isUploadingPicture ? 'Hochladen...' : profilePicture ? 'Ändern' : 'Bild hochladen'}
                    </button>
                  </div>

                  {/* Name Input */}
                  <div className="flex-1">
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

              {/* Display */}
              <div className="bg-card rounded-lg border border-border p-6">
                <h3 className="text-lg font-semibold mb-4">Anzeige</h3>
                <div className="grid grid-cols-3 gap-4">
                  <div>
                    <label className="text-sm font-medium">Sprache</label>
                    <select
                      value={language}
                      onChange={(e) => setLanguage(e.target.value as 'de' | 'en')}
                      className="mt-1 block w-full rounded-md border border-input bg-background px-3 py-2"
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
                      className="mt-1 block w-full rounded-md border border-input bg-background px-3 py-2"
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
                      className="mt-1 block w-full rounded-md border border-input bg-background px-3 py-2"
                    >
                      <option value="EUR">EUR</option>
                      <option value="USD">USD</option>
                      <option value="CHF">CHF</option>
                      <option value="GBP">GBP</option>
                    </select>
                  </div>
                  <div>
                    <label className="text-sm font-medium">Chart-Zeitraum</label>
                    <select
                      value={defaultChartTimeRange}
                      onChange={(e) => setDefaultChartTimeRange(e.target.value as ChartTimeRange)}
                      className="mt-1 block w-full rounded-md border border-input bg-background px-3 py-2"
                    >
                      <option value="1W">1 Woche</option>
                      <option value="1M">1 Monat</option>
                      <option value="3M">3 Monate</option>
                      <option value="6M">6 Monate</option>
                      <option value="YTD">Jahr bis heute</option>
                      <option value="1Y">1 Jahr</option>
                      <option value="3Y">3 Jahre</option>
                      <option value="5Y">5 Jahre</option>
                      <option value="MAX">Maximum</option>
                    </select>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Transactions Section */}
          {activeSection === 'transactions' && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-1">Buchungen</h2>
                <p className="text-sm text-muted-foreground">Einstellungen für Transaktionen</p>
              </div>

              <div className="bg-card rounded-lg border border-border p-6">
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
                    className={cn(
                      "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2",
                      deliveryMode ? 'bg-primary' : 'bg-muted'
                    )}
                  >
                    <span
                      className={cn(
                        "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out",
                        deliveryMode ? 'translate-x-5' : 'translate-x-0'
                      )}
                    />
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Quotes Section */}
          {activeSection === 'quotes' && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-1">Kurse</h2>
                <p className="text-sm text-muted-foreground">API-Keys für Kursanbieter</p>
              </div>

              {/* Quote Sync Toggle */}
              <div className="bg-card rounded-lg border border-border p-6">
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
                    className={cn(
                      "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2",
                      syncOnlyHeldSecurities ? 'bg-primary' : 'bg-muted'
                    )}
                  >
                    <span
                      className={cn(
                        "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out",
                        syncOnlyHeldSecurities ? 'translate-x-5' : 'translate-x-0'
                      )}
                    />
                  </button>
                </div>
              </div>

              {/* API Keys */}
              <div className="bg-card rounded-lg border border-border p-6 space-y-6">
                <h3 className="text-lg font-semibold">API-Keys</h3>

                {/* Finnhub */}
                <div>
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
                      API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert.
                    </p>
                  )}
                </div>

                {/* CoinGecko */}
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
                      API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert.
                    </p>
                  ) : (
                    <p className="text-xs text-muted-foreground mt-1">
                      Ohne Key: max. 10-30 Anfragen/Minute. Mit Demo-Key: 30 Anfragen/Minute.
                    </p>
                  )}
                </div>

                {/* Alpha Vantage */}
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
                      API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert.
                    </p>
                  )}
                </div>

                {/* Twelve Data */}
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
                      API Key {secureStorageAvailable ? 'sicher ' : ''}gespeichert.
                    </p>
                  ) : (
                    <p className="text-xs text-muted-foreground mt-1">
                      Symbol-Format: NESN.SW wird automatisch zu NESN:SIX konvertiert.
                    </p>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* AI Section */}
          {activeSection === 'ai' && (
            <div className="space-y-6">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-xl font-semibold mb-1">KI-Analyse</h2>
                  <p className="text-sm text-muted-foreground">KI-Features und API-Keys</p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={aiEnabled}
                  onClick={() => setAiEnabled(!aiEnabled)}
                  className={cn(
                    "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2",
                    aiEnabled ? 'bg-primary' : 'bg-muted'
                  )}
                  title={aiEnabled ? 'KI-Features deaktivieren' : 'KI-Features aktivieren'}
                >
                  <span
                    className={cn(
                      "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out",
                      aiEnabled ? 'translate-x-5' : 'translate-x-0'
                    )}
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
                <>
                  {/* API Keys */}
                  <div className="bg-card rounded-lg border border-border p-6">
                    <div className="flex items-center justify-between mb-3">
                      <h3 className="text-lg font-semibold">API-Keys</h3>
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
                  <div className="bg-card rounded-lg border border-border p-6">
                    <h3 className="text-lg font-semibold mb-3">Funktionen konfigurieren</h3>
                    <AiFeatureMatrix
                      apiKeys={{
                        anthropicApiKey: effectiveAnthropicApiKey,
                        openaiApiKey: effectiveOpenaiApiKey,
                        geminiApiKey: effectiveGeminiApiKey,
                        perplexityApiKey: effectivePerplexityApiKey,
                      }}
                    />
                  </div>

                  {/* Chat Context Window */}
                  <div className="bg-card rounded-lg border border-border p-6">
                    <h3 className="text-lg font-semibold mb-2">Chat-Kontextfenster</h3>
                    <p className="text-sm text-muted-foreground mb-3">
                      Anzahl der Nachrichten, die an die KI gesendet werden.
                    </p>
                    <div className="flex items-center gap-3">
                      <input
                        type="number"
                        min={5}
                        max={500}
                        value={chatContextSize}
                        onChange={(e) => handleChatContextSizeChange(e.target.value)}
                        onBlur={(e) => {
                          const val = Math.min(500, Math.max(5, Number(e.target.value) || 20));
                          setChatContextSize(val);
                        }}
                        className="w-24 rounded-md border border-input bg-background px-3 py-2 text-center"
                      />
                      <span className="text-sm text-muted-foreground">Nachrichten (5-500)</span>
                    </div>
                    <p className="text-xs text-muted-foreground mt-2">
                      Der Chat-Verlauf wird vollständig gespeichert, aber nur die letzten {chatContextSize} Nachrichten werden an die KI gesendet.
                    </p>
                  </div>
                </>
              )}
            </div>
          )}

          {/* User Queries Section */}
          {activeSection === 'queries' && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-1">Eigene Abfragen</h2>
                <p className="text-sm text-muted-foreground">
                  Definiere benutzerdefinierte SQL-Abfragen für den ChatBot
                </p>
              </div>
              <UserTemplatesSettings />
            </div>
          )}

          {/* Services Section */}
          {activeSection === 'services' && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-1">Dienste</h2>
                <p className="text-sm text-muted-foreground">Externe Dienste und Logos</p>
              </div>

              {/* DivvyDiary */}
              <div className="bg-card rounded-lg border border-border p-6">
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
                    API-Key gespeichert. Sie können jetzt Portfolios zu DivvyDiary exportieren.
                  </p>
                )}
              </div>

              {/* Brandfetch / Logos */}
              <div className="bg-card rounded-lg border border-border p-6 space-y-4">
                <h3 className="text-lg font-semibold">Logos</h3>
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
                      Client ID {secureStorageAvailable ? 'sicher ' : ''}gespeichert.
                    </p>
                  )}
                </div>

                <div className="pt-4 border-t border-border">
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
          )}

          {/* Data Section */}
          {activeSection === 'data' && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-1">Daten</h2>
                <p className="text-sm text-muted-foreground">Symbol-Validierung und Datenpflege</p>
              </div>

              {/* Symbol Validation */}
              <div className="bg-card rounded-lg border border-border p-6">
                <div className="flex items-center gap-2 mb-4">
                  <Search size={20} className="text-primary" />
                  <h3 className="text-lg font-semibold">Symbol-Validierung</h3>
                </div>
                <p className="text-sm text-muted-foreground mb-4">
                  Automatische Überprüfung und Korrektur von Kursquellen-Konfigurationen.
                </p>
                <div className="space-y-4">
                  {/* Auto-Validate Interval */}
                  <div>
                    <label className="text-sm font-medium">Auto-Validierung</label>
                    <p className="text-sm text-muted-foreground mt-0.5 mb-2">
                      Intervall für automatische Validierung beim App-Start
                    </p>
                    <select
                      value={symbolValidation.autoValidateIntervalDays}
                      onChange={(e) => setSymbolValidationSettings({ autoValidateIntervalDays: Number(e.target.value) as 0 | 7 | 14 | 30 })}
                      className="mt-1 block w-full max-w-xs rounded-md border border-input bg-background px-3 py-2"
                    >
                      <option value={0}>Deaktiviert</option>
                      <option value={7}>Wöchentlich</option>
                      <option value={14}>Alle 2 Wochen</option>
                      <option value={30}>Monatlich</option>
                    </select>
                    {symbolValidation.lastAutoValidation && (
                      <p className="text-xs text-muted-foreground mt-1">
                        Letzte Validierung: {new Date(symbolValidation.lastAutoValidation).toLocaleDateString('de-DE', {
                          day: '2-digit',
                          month: '2-digit',
                          year: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit'
                        })}
                      </p>
                    )}
                  </div>

                  {/* Only Held Toggle */}
                  <div className="flex items-center justify-between pt-4 border-t border-border">
                    <div>
                      <label className="text-sm font-medium">Nur Wertpapiere im Bestand</label>
                      <p className="text-sm text-muted-foreground mt-0.5">
                        Validierung nur für Wertpapiere mit aktuellem Bestand
                      </p>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={symbolValidation.validateOnlyHeld}
                      onClick={() => setSymbolValidationSettings({ validateOnlyHeld: !symbolValidation.validateOnlyHeld })}
                      className={cn(
                        "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2",
                        symbolValidation.validateOnlyHeld ? 'bg-primary' : 'bg-muted'
                      )}
                    >
                      <span
                        className={cn(
                          "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out",
                          symbolValidation.validateOnlyHeld ? 'translate-x-5' : 'translate-x-0'
                        )}
                      />
                    </button>
                  </div>

                  {/* AI Fallback Toggle */}
                  <div className="flex items-center justify-between pt-4 border-t border-border">
                    <div>
                      <label className="text-sm font-medium">KI-Fallback aktivieren</label>
                      <p className="text-sm text-muted-foreground mt-0.5">
                        Bei nicht eindeutigen Ergebnissen KI zur Symbol-Ermittlung nutzen
                      </p>
                      {symbolValidation.enableAiFallback && !hasAnyAiKey && (
                        <p className="text-xs text-amber-600 mt-1">
                          Kein KI-API-Key konfiguriert.
                        </p>
                      )}
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={symbolValidation.enableAiFallback}
                      onClick={() => setSymbolValidationSettings({ enableAiFallback: !symbolValidation.enableAiFallback })}
                      className={cn(
                        "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2",
                        symbolValidation.enableAiFallback ? 'bg-primary' : 'bg-muted'
                      )}
                    >
                      <span
                        className={cn(
                          "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out",
                          symbolValidation.enableAiFallback ? 'translate-x-5' : 'translate-x-0'
                        )}
                      />
                    </button>
                  </div>

                  {/* Force Re-validation Toggle */}
                  <div className="flex items-center justify-between pt-4 border-t border-border">
                    <div>
                      <label className="text-sm font-medium">Neu validieren erzwingen</label>
                      <p className="text-sm text-muted-foreground mt-0.5">
                        Ignoriert Cache und prüft alle Konfigurationen erneut
                      </p>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={forceValidation}
                      onClick={() => setForceValidation(!forceValidation)}
                      className={cn(
                        "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2",
                        forceValidation ? 'bg-primary' : 'bg-muted'
                      )}
                    >
                      <span
                        className={cn(
                          "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out",
                          forceValidation ? 'translate-x-5' : 'translate-x-0'
                        )}
                      />
                    </button>
                  </div>

                  {/* Status & Run Button */}
                  <div className="pt-4 border-t border-border">
                    {validationStatus && (
                      <div className="mb-4 p-3 rounded-lg bg-muted/50">
                        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
                          <div>
                            <span className="text-muted-foreground">Gesamt:</span>
                            <span className="ml-1 font-medium">{validationStatus.totalSecurities}</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground">Validiert:</span>
                            <span className="ml-1 font-medium text-green-600">{validationStatus.validatedCount}</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground">KI-Vorschläge:</span>
                            <span className="ml-1 font-medium text-amber-600">{validationStatus.aiSuggestedCount}</span>
                          </div>
                          <div>
                            <span className="text-muted-foreground">Ausstehend:</span>
                            <span className="ml-1 font-medium text-blue-600">{validationStatus.pendingCount}</span>
                          </div>
                        </div>
                      </div>
                    )}
                    <button
                      type="button"
                      onClick={handleRunValidation}
                      disabled={isValidating}
                      className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
                    >
                      <Search size={16} className={isValidating ? 'animate-pulse' : ''} />
                      {isValidating ? 'Validiere...' : 'Jetzt validieren'}
                    </button>
                    {validationResult?.summary && (
                      <div className="mt-3 text-sm">
                        <p className="text-green-600">
                          {validationResult.summary.validated} validiert
                        </p>
                        {validationResult.summary.aiSuggested > 0 && (
                          <p className="text-amber-600">
                            {validationResult.summary.aiSuggested} KI-Vorschläge
                          </p>
                        )}
                        {validationResult.summary.failed > 0 && (
                          <p className="text-red-600">
                            {validationResult.summary.failed} fehlgeschlagen
                          </p>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              </div>

              {/* FIFO Rebuild */}
              <div className="bg-card rounded-lg border border-border p-6">
                <h3 className="text-lg font-semibold mb-4">FIFO Cost Basis</h3>
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

              {/* Custom Attributes */}
              <div className="bg-card rounded-lg border border-border p-6">
                <h3 className="text-lg font-semibold mb-4">Erweiterte Daten</h3>
                <AttributeTypeManager
                  expanded={attributesExpanded}
                  onToggleExpand={() => setAttributesExpanded(!attributesExpanded)}
                />
              </div>
            </div>
          )}

          {/* Danger Section */}
          {activeSection === 'danger' && (
            <div className="space-y-6">
              <div>
                <h2 className="text-xl font-semibold mb-1 text-destructive">Gefahrenzone</h2>
                <p className="text-sm text-muted-foreground">Irreversible Aktionen</p>
              </div>

              <div className="bg-card rounded-lg border border-destructive/50 p-6">
                <div className="flex items-center gap-2 mb-4">
                  <AlertTriangle size={20} className="text-destructive" />
                  <h3 className="text-lg font-semibold text-destructive">Alle Daten löschen</h3>
                </div>
                <p className="text-sm text-muted-foreground mb-3">
                  Löscht alle Daten aus der Datenbank: Wertpapiere, Konten, Depots, Buchungen, Kurse, Einstellungen und Chat-Verläufe.
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
          )}
        </div>
      </main>
    </div>
  );
}
