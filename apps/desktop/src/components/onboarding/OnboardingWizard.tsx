/**
 * Multi-step onboarding wizard for first-time users.
 * Shown when appMode === null (first launch).
 * Replaces WelcomeModal and mode selection screen.
 */

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  User,
  Camera,
  Briefcase,
  CandlestickChart,
  Bot,
  Image as ImageIcon,
  ChevronLeft,
  ChevronRight,
  Check,
  Eye,
  EyeOff,
  ExternalLink,
  Rocket,
  X,
} from 'lucide-react';
import {
  useAppStore,
  useUIStore,
  useSettingsStore,
  getDefaultViewForMode,
  type AppMode,
  type AiProvider,
} from '../../store';
import { useSecureApiKeys } from '../../hooks/useSecureApiKeys';
import { AIProviderLogo } from '../common/AIProviderLogo';

const TOTAL_STEPS = 6;

const AI_PROVIDERS: { id: AiProvider; name: string; keyUrl: string }[] = [
  { id: 'claude', name: 'Claude', keyUrl: 'https://console.anthropic.com/' },
  { id: 'openai', name: 'OpenAI', keyUrl: 'https://platform.openai.com/api-keys' },
  { id: 'gemini', name: 'Gemini', keyUrl: 'https://aistudio.google.com/apikey' },
  { id: 'perplexity', name: 'Perplexity', keyUrl: 'https://www.perplexity.ai/settings/api' },
  { id: 'openrouter', name: 'OpenRouter', keyUrl: 'https://openrouter.ai/keys' },
];

const AI_KEY_TYPE_MAP: Record<AiProvider, string> = {
  claude: 'anthropic',
  openai: 'openai',
  gemini: 'gemini',
  perplexity: 'perplexity',
  openrouter: 'openrouter',
};

// ============================================================================
// Progress Bar
// ============================================================================

function ProgressBar({ current, total }: { current: number; total: number }) {
  return (
    <div className="flex items-center gap-1.5 mb-8">
      {Array.from({ length: total }).map((_, i) => (
        <div key={i} className="flex items-center gap-1.5 flex-1">
          <div
            className={`w-2.5 h-2.5 rounded-full shrink-0 transition-all duration-300 ${
              i < current
                ? 'bg-primary scale-100'
                : i === current
                  ? 'bg-primary scale-125'
                  : 'bg-muted'
            }`}
          />
          {i < total - 1 && (
            <div
              className={`flex-1 h-0.5 rounded transition-colors duration-300 ${
                i < current ? 'bg-primary' : 'bg-muted'
              }`}
            />
          )}
        </div>
      ))}
    </div>
  );
}

// ============================================================================
// Step Components
// ============================================================================

function NameStep({
  name,
  setName,
  onNext,
}: {
  name: string;
  setName: (v: string) => void;
  onNext: () => void;
}) {
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name.trim()) onNext();
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      <div className="text-center">
        <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
          <User size={32} className="text-primary" />
        </div>
        <h2 className="text-xl font-semibold">Wie darf ich dich nennen?</h2>
        <p className="text-sm text-muted-foreground mt-2">
          Wird in KI-Konversationen verwendet, um dich persönlich anzusprechen.
        </p>
      </div>

      <input
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value.slice(0, 50))}
        maxLength={50}
        placeholder="z.B. Max"
        autoFocus
        className="w-full px-4 py-3 border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary text-lg text-center"
      />

      <button
        type="submit"
        disabled={!name.trim()}
        className="w-full flex items-center justify-center gap-2 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      >
        Weiter
        <ChevronRight size={16} />
      </button>
    </form>
  );
}

function ProfilePictureStep({
  picture,
  setPicture,
  onNext,
  onBack,
}: {
  picture: string | null;
  setPicture: (v: string | null) => void;
  onNext: () => void;
  onBack: () => void;
}) {
  const [isUploading, setIsUploading] = useState(false);

  const handleUpload = async () => {
    setIsUploading(true);
    try {
      const { open: openDialog } = await import('@tauri-apps/plugin-dialog');
      const selectedPath = await openDialog({
        multiple: false,
        filters: [{ name: 'Bilder', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }],
        title: 'Profilbild auswählen',
      });
      if (!selectedPath) return;

      const result = await invoke<{ data: string; mimeType: string }>('read_image_as_base64', { path: selectedPath });
      if (result) {
        setPicture(`data:${result.mimeType};base64,${result.data}`);
      }
    } catch {
      // User cancelled or error
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="text-center">
        <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
          <ImageIcon size={32} className="text-primary" />
        </div>
        <h2 className="text-xl font-semibold">Profilbild hinzufügen</h2>
        <p className="text-sm text-muted-foreground mt-2">
          Optional — wird in der App und im Chat angezeigt.
        </p>
      </div>

      <div className="flex flex-col items-center gap-4">
        <div className="relative group">
          {picture ? (
            <div className="relative">
              <img
                src={picture}
                alt="Profilbild"
                className="w-28 h-28 rounded-full object-cover border-2 border-border"
              />
              <button
                onClick={() => setPicture(null)}
                className="absolute -top-1 -right-1 w-7 h-7 bg-destructive text-destructive-foreground rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
              >
                <X size={14} />
              </button>
            </div>
          ) : (
            <div className="w-28 h-28 rounded-full bg-muted flex items-center justify-center border-2 border-dashed border-border">
              <User size={48} className="text-muted-foreground" />
            </div>
          )}
        </div>
        <button
          onClick={handleUpload}
          disabled={isUploading}
          className="flex items-center gap-2 text-sm text-primary hover:text-primary/80 transition-colors disabled:opacity-50"
        >
          <Camera size={16} />
          {isUploading ? 'Hochladen...' : picture ? 'Ändern' : 'Bild auswählen'}
        </button>
      </div>

      <div className="flex gap-3">
        <button
          onClick={onBack}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
        >
          <ChevronLeft size={16} />
          Zurück
        </button>
        <button
          onClick={onNext}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          {picture ? 'Weiter' : 'Überspringen'}
          <ChevronRight size={16} />
        </button>
      </div>
    </div>
  );
}

function AppModeStep({
  mode,
  setMode,
  onNext,
  onBack,
}: {
  mode: AppMode;
  setMode: (v: AppMode) => void;
  onNext: () => void;
  onBack: () => void;
}) {
  return (
    <div className="space-y-6">
      <div className="text-center">
        <h2 className="text-xl font-semibold">Wie möchtest du die App nutzen?</h2>
        <p className="text-sm text-muted-foreground mt-2">
          Du kannst den Modus jederzeit in den Einstellungen ändern.
        </p>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <button
          onClick={() => setMode('portfolio')}
          className={`flex flex-col items-center gap-3 p-5 rounded-xl border-2 transition-all ${
            mode === 'portfolio'
              ? 'border-primary bg-primary/5'
              : 'border-border hover:border-primary/50'
          }`}
        >
          <Briefcase className={`w-8 h-8 ${mode === 'portfolio' ? 'text-primary' : 'text-muted-foreground'}`} />
          <div className="text-center">
            <div className="font-semibold text-sm mb-1">Portfolio verwalten</div>
            <div className="text-xs text-muted-foreground">
              Portfolios, Konten, Buchungen, Dividenden & technische Analyse
            </div>
          </div>
        </button>
        <button
          onClick={() => setMode('analysis')}
          className={`flex flex-col items-center gap-3 p-5 rounded-xl border-2 transition-all ${
            mode === 'analysis'
              ? 'border-primary bg-primary/5'
              : 'border-border hover:border-primary/50'
          }`}
        >
          <CandlestickChart className={`w-8 h-8 ${mode === 'analysis' ? 'text-primary' : 'text-muted-foreground'}`} />
          <div className="text-center">
            <div className="font-semibold text-sm mb-1">Märkte analysieren</div>
            <div className="text-xs text-muted-foreground">
              Watchlist, Charts, Screener — kein Portfolio nötig
            </div>
          </div>
        </button>
      </div>

      <div className="flex gap-3">
        <button
          onClick={onBack}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
        >
          <ChevronLeft size={16} />
          Zurück
        </button>
        <button
          onClick={onNext}
          disabled={!mode}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Weiter
          <ChevronRight size={16} />
        </button>
      </div>
    </div>
  );
}

function AiSetupStep({
  selectedProvider,
  setSelectedProvider,
  apiKey,
  setApiKey,
  onNext,
  onBack,
}: {
  selectedProvider: AiProvider | null;
  setSelectedProvider: (v: AiProvider | null) => void;
  apiKey: string;
  setApiKey: (v: string) => void;
  onNext: () => void;
  onBack: () => void;
}) {
  const [showKey, setShowKey] = useState(false);

  return (
    <div className="space-y-6">
      <div className="text-center">
        <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
          <Bot size={32} className="text-primary" />
        </div>
        <h2 className="text-xl font-semibold">KI-Unterstützung einrichten</h2>
        <p className="text-sm text-muted-foreground mt-2">
          Optional — für Chart-Analyse, Chat-Assistent und Portfolio Insights.
        </p>
      </div>

      <div className="grid grid-cols-5 gap-2">
        {AI_PROVIDERS.map((p) => (
          <button
            key={p.id}
            onClick={() => {
              if (selectedProvider === p.id) {
                setSelectedProvider(null);
                setApiKey('');
              } else {
                setSelectedProvider(p.id);
                setApiKey('');
              }
            }}
            className={`flex flex-col items-center gap-1.5 p-3 rounded-lg border-2 transition-all ${
              selectedProvider === p.id
                ? 'border-primary bg-primary/5'
                : 'border-border hover:border-primary/50'
            }`}
          >
            <AIProviderLogo provider={p.id} size={24} />
            <span className="text-xs font-medium">{p.name}</span>
          </button>
        ))}
      </div>

      {selectedProvider && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium">API Key</label>
            <a
              href={AI_PROVIDERS.find((p) => p.id === selectedProvider)?.keyUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-primary hover:underline flex items-center gap-1"
              onClick={(e) => {
                e.preventDefault();
                import('@tauri-apps/plugin-shell').then(({ open }) =>
                  open(AI_PROVIDERS.find((p) => p.id === selectedProvider)?.keyUrl || '')
                );
              }}
            >
              Key erhalten
              <ExternalLink size={10} />
            </a>
          </div>
          <div className="relative">
            <input
              type={showKey ? 'text' : 'password'}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={`${AI_PROVIDERS.find((p) => p.id === selectedProvider)?.name} API Key`}
              className="w-full px-3 py-2 pr-10 border border-border rounded-lg bg-background font-mono text-sm focus:outline-none focus:ring-2 focus:ring-primary"
            />
            <button
              type="button"
              onClick={() => setShowKey(!showKey)}
              className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
          </div>
        </div>
      )}

      <div className="flex gap-3">
        <button
          onClick={onBack}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
        >
          <ChevronLeft size={16} />
          Zurück
        </button>
        <button
          onClick={onNext}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          {selectedProvider && apiKey ? 'Weiter' : 'Überspringen'}
          <ChevronRight size={16} />
        </button>
      </div>
    </div>
  );
}

function BrandfetchStep({
  apiKey,
  setApiKey,
  onNext,
  onBack,
}: {
  apiKey: string;
  setApiKey: (v: string) => void;
  onNext: () => void;
  onBack: () => void;
}) {
  const [showKey, setShowKey] = useState(false);

  return (
    <div className="space-y-6">
      <div className="text-center">
        <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
          <ImageIcon size={32} className="text-primary" />
        </div>
        <h2 className="text-xl font-semibold">Logos für Wertpapiere</h2>
        <p className="text-sm text-muted-foreground mt-2">
          Optional — Mit einer Brandfetch Client ID werden Logos für Aktien und ETF-Anbieter geladen.
        </p>
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">Brandfetch Client ID</label>
          <button
            type="button"
            onClick={() => {
              import('@tauri-apps/plugin-shell').then(({ open }) =>
                open('https://docs.brandfetch.com/logo-api/overview')
              );
            }}
            className="text-xs text-primary hover:underline flex items-center gap-1"
          >
            Client ID erhalten
            <ExternalLink size={10} />
          </button>
        </div>
        <div className="relative">
          <input
            type={showKey ? 'text' : 'password'}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="Deine Brandfetch Client ID"
            className="w-full px-3 py-2 pr-10 border border-border rounded-lg bg-background font-mono text-sm focus:outline-none focus:ring-2 focus:ring-primary"
          />
          <button
            type="button"
            onClick={() => setShowKey(!showKey)}
            className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
          >
            {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
          </button>
        </div>
      </div>

      <div className="flex gap-3">
        <button
          onClick={onBack}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
        >
          <ChevronLeft size={16} />
          Zurück
        </button>
        <button
          onClick={onNext}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
        >
          {apiKey ? 'Weiter' : 'Überspringen'}
          <ChevronRight size={16} />
        </button>
      </div>
    </div>
  );
}

function DoneStep({
  name,
  picture,
  mode,
  aiProvider,
  brandfetchKey,
  onBack,
  onFinish,
  isFinishing,
}: {
  name: string;
  picture: string | null;
  mode: AppMode;
  aiProvider: AiProvider | null;
  brandfetchKey: string;
  onBack: () => void;
  onFinish: () => void;
  isFinishing: boolean;
}) {
  return (
    <div className="space-y-6">
      <div className="text-center">
        <div className="w-16 h-16 bg-primary/10 rounded-full flex items-center justify-center mx-auto mb-4">
          <Rocket size={32} className="text-primary" />
        </div>
        <h2 className="text-xl font-semibold">Alles bereit!</h2>
        <p className="text-sm text-muted-foreground mt-2">
          Hier ist eine Zusammenfassung deiner Einstellungen.
        </p>
      </div>

      <div className="space-y-3">
        {/* Name + Picture */}
        <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50">
          {picture ? (
            <img src={picture} alt="" className="w-10 h-10 rounded-full object-cover" />
          ) : (
            <div className="w-10 h-10 rounded-full bg-muted flex items-center justify-center">
              <User size={20} className="text-muted-foreground" />
            </div>
          )}
          <div>
            <div className="font-medium text-sm">{name}</div>
            <div className="text-xs text-muted-foreground">Dein Name</div>
          </div>
          <Check size={16} className="ml-auto text-green-500" />
        </div>

        {/* Mode */}
        <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50">
          {mode === 'portfolio' ? (
            <Briefcase size={20} className="text-primary" />
          ) : (
            <CandlestickChart size={20} className="text-primary" />
          )}
          <div>
            <div className="font-medium text-sm">
              {mode === 'portfolio' ? 'Portfolio verwalten' : 'Märkte analysieren'}
            </div>
            <div className="text-xs text-muted-foreground">App-Modus</div>
          </div>
          <Check size={16} className="ml-auto text-green-500" />
        </div>

        {/* AI */}
        <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50">
          <Bot size={20} className={aiProvider ? 'text-primary' : 'text-muted-foreground'} />
          <div>
            <div className="font-medium text-sm">
              {aiProvider
                ? AI_PROVIDERS.find((p) => p.id === aiProvider)?.name
                : 'Nicht konfiguriert'}
            </div>
            <div className="text-xs text-muted-foreground">KI-Provider</div>
          </div>
          {aiProvider ? (
            <Check size={16} className="ml-auto text-green-500" />
          ) : (
            <span className="ml-auto text-xs text-muted-foreground">Optional</span>
          )}
        </div>

        {/* Brandfetch */}
        <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50">
          <ImageIcon size={20} className={brandfetchKey ? 'text-primary' : 'text-muted-foreground'} />
          <div>
            <div className="font-medium text-sm">
              {brandfetchKey ? 'Konfiguriert' : 'Nicht konfiguriert'}
            </div>
            <div className="text-xs text-muted-foreground">Wertpapier-Logos</div>
          </div>
          {brandfetchKey ? (
            <Check size={16} className="ml-auto text-green-500" />
          ) : (
            <span className="ml-auto text-xs text-muted-foreground">Optional</span>
          )}
        </div>
      </div>

      <div className="flex gap-3">
        <button
          onClick={onBack}
          className="px-4 py-2.5 border border-border rounded-lg hover:bg-muted transition-colors"
        >
          <ChevronLeft size={16} />
        </button>
        <button
          onClick={onFinish}
          disabled={isFinishing}
          className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50"
        >
          <Rocket size={16} />
          {isFinishing ? 'Wird eingerichtet...' : "Los geht's!"}
        </button>
      </div>
    </div>
  );
}

// ============================================================================
// Main Wizard
// ============================================================================

export function OnboardingWizard() {
  const [step, setStep] = useState(0);
  const [isFinishing, setIsFinishing] = useState(false);

  // Wizard state (committed only at the end)
  const [name, setName] = useState('');
  const [picture, setPicture] = useState<string | null>(null);
  const [appMode, setAppMode] = useState<AppMode>(null);
  const [aiProvider, setAiProvider] = useState<AiProvider | null>(null);
  const [aiApiKey, setAiApiKey] = useState('');
  const [brandfetchKey, setBrandfetchKey] = useState('');

  // Store hooks
  const setAppModeStore = useAppStore((s) => s.setAppMode);
  const setCurrentView = useUIStore((s) => s.setCurrentView);
  const { setUserName, setProfilePicture, setAiEnabled, setAiProvider: setAiProviderStore } = useSettingsStore();
  const { setApiKey: setSecureApiKey } = useSecureApiKeys();

  const next = () => setStep((s) => Math.min(s + 1, TOTAL_STEPS - 1));
  const back = () => setStep((s) => Math.max(s - 1, 0));

  const handleFinish = async () => {
    setIsFinishing(true);
    try {
      // 1. Name
      setUserName(name.trim());

      // 2. Profile picture
      if (picture) {
        try {
          await invoke('set_user_profile_picture', { pictureBase64: picture });
          setProfilePicture(picture);
        } catch {
          // Non-critical
        }
      }

      // 3. AI provider + key (non-critical — don't block mode setup)
      if (aiProvider && aiApiKey.trim()) {
        try {
          const keyType = AI_KEY_TYPE_MAP[aiProvider];
          await setSecureApiKey(keyType as Parameters<typeof setSecureApiKey>[0], aiApiKey.trim());
          setAiEnabled(true);
          setAiProviderStore(aiProvider);
        } catch (err) {
          console.warn('Failed to save AI key:', err);
        }
      }

      // 4. Brandfetch key (non-critical)
      if (brandfetchKey.trim()) {
        try {
          await setSecureApiKey('brandfetch', brandfetchKey.trim());
        } catch (err) {
          console.warn('Failed to save Brandfetch key:', err);
        }
      }

      // 5. App mode (LAST — this dismisses the wizard)
      setAppModeStore(appMode!);
      setCurrentView(getDefaultViewForMode(appMode!));

      // 6. Mark welcome as seen
      localStorage.setItem('portfolio-welcome-seen', 'true');
    } catch (err) {
      console.error('Onboarding error:', err);
      // Fallback: at least set the mode so user can get into the app
      const fallbackMode = appMode || 'portfolio';
      setAppModeStore(fallbackMode);
      setCurrentView(getDefaultViewForMode(fallbackMode));
    } finally {
      setIsFinishing(false);
    }
  };

  return (
    <div className="h-screen bg-background flex items-center justify-center">
      <div className="w-full max-w-md px-6">
        <ProgressBar current={step} total={TOTAL_STEPS} />

        <div key={step} className="animate-fade-up">
          {step === 0 && (
            <NameStep name={name} setName={setName} onNext={next} />
          )}
          {step === 1 && (
            <ProfilePictureStep
              picture={picture}
              setPicture={setPicture}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 2 && (
            <AppModeStep
              mode={appMode}
              setMode={setAppMode}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 3 && (
            <AiSetupStep
              selectedProvider={aiProvider}
              setSelectedProvider={setAiProvider}
              apiKey={aiApiKey}
              setApiKey={setAiApiKey}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 4 && (
            <BrandfetchStep
              apiKey={brandfetchKey}
              setApiKey={setBrandfetchKey}
              onNext={next}
              onBack={back}
            />
          )}
          {step === 5 && (
            <DoneStep
              name={name}
              picture={picture}
              mode={appMode}
              aiProvider={aiProvider}
              brandfetchKey={brandfetchKey}
              onBack={back}
              onFinish={handleFinish}
              isFinishing={isFinishing}
            />
          )}
        </div>
      </div>
    </div>
  );
}
