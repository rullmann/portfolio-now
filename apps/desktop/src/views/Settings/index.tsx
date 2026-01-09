/**
 * Settings view for application configuration.
 */

import { useState } from 'react';
import { Eye, EyeOff, ExternalLink, Trash2 } from 'lucide-react';
import { useSettingsStore } from '../../store';
import { open } from '@tauri-apps/plugin-shell';
import { clearLogoCache } from '../../lib/api';

export function SettingsView() {
  const {
    syncOnlyHeldSecurities,
    setSyncOnlyHeldSecurities,
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
  } = useSettingsStore();

  const [showBrandfetchKey, setShowBrandfetchKey] = useState(false);
  const [showFinnhubKey, setShowFinnhubKey] = useState(false);
  const [showCoingeckoKey, setShowCoingeckoKey] = useState(false);
  const [showAlphaVantageKey, setShowAlphaVantageKey] = useState(false);
  const [showTwelveDataKey, setShowTwelveDataKey] = useState(false);
  const [cacheResult, setCacheResult] = useState<string | null>(null);
  const [isClearing, setIsClearing] = useState(false);

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

  return (
    <div className="space-y-6">
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
            <label className="text-sm font-medium">Cache</label>
            <p className="text-sm text-muted-foreground mt-0.5 mb-3">
              Alte Cache-Dateien von früheren Versionen bereinigen.
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
    </div>
  );
}
