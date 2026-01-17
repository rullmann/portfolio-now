/**
 * Hook for secure API key management
 *
 * This hook provides a bridge between the UI and secure storage for API keys.
 * It handles loading keys from secure storage on mount and saving them when changed.
 *
 * SECURITY: API keys are stored in Tauri's secure store (app_data_dir/secure-keys.json)
 * instead of browser localStorage. This prevents access from arbitrary JavaScript code.
 */

import { useEffect, useState, useCallback, useRef } from 'react';
import {
  getAllApiKeys,
  storeApiKey,
  migrateFromLocalStorage,
  isSecureStorageAvailable,
  type ApiKeyType,
} from '../lib/secureStorage';
import { useSettingsStore } from '../store';

// Key used by Zustand persist middleware for settings
const ZUSTAND_STORAGE_KEY = 'portfolio-settings';

/**
 * Read API keys directly from localStorage (legacy Zustand persist storage).
 * This reads the raw JSON to get keys that were stored before the migration
 * to secure storage, avoiding any race conditions with Zustand hydration.
 */
function readLegacyKeysFromLocalStorage(): Partial<Record<ApiKeyType, string>> {
  try {
    const raw = localStorage.getItem(ZUSTAND_STORAGE_KEY);
    if (!raw) return {};

    const parsed = JSON.parse(raw);
    const state = parsed?.state || parsed;

    return {
      brandfetch: state.brandfetchApiKey || '',
      finnhub: state.finnhubApiKey || '',
      coingecko: state.coingeckoApiKey || '',
      alphaVantage: state.alphaVantageApiKey || '',
      twelveData: state.twelveDataApiKey || '',
      anthropic: state.anthropicApiKey || '',
      openai: state.openaiApiKey || '',
      gemini: state.geminiApiKey || '',
      perplexity: state.perplexityApiKey || '',
    };
  } catch {
    return {};
  }
}

/**
 * Clear API keys from localStorage after successful migration.
 * This removes only the API key fields, preserving other settings.
 */
function clearLegacyKeysFromLocalStorage(): void {
  try {
    const raw = localStorage.getItem(ZUSTAND_STORAGE_KEY);
    if (!raw) return;

    const parsed = JSON.parse(raw);
    const state = parsed?.state || parsed;

    // Remove API key fields
    delete state.brandfetchApiKey;
    delete state.finnhubApiKey;
    delete state.coingeckoApiKey;
    delete state.alphaVantageApiKey;
    delete state.twelveDataApiKey;
    delete state.anthropicApiKey;
    delete state.openaiApiKey;
    delete state.geminiApiKey;
    delete state.perplexityApiKey;

    // Write back
    if (parsed?.state) {
      parsed.state = state;
      localStorage.setItem(ZUSTAND_STORAGE_KEY, JSON.stringify(parsed));
    } else {
      localStorage.setItem(ZUSTAND_STORAGE_KEY, JSON.stringify(state));
    }
  } catch {
    // Ignore errors during cleanup
  }
}

interface SecureApiKeys {
  // Quote providers
  brandfetchApiKey: string;
  finnhubApiKey: string;
  coingeckoApiKey: string;
  alphaVantageApiKey: string;
  twelveDataApiKey: string;
  // AI providers
  anthropicApiKey: string;
  openaiApiKey: string;
  geminiApiKey: string;
  perplexityApiKey: string;
}

interface UseSecureApiKeysReturn {
  keys: SecureApiKeys;
  isLoading: boolean;
  isSecureStorageAvailable: boolean;
  /** True when secure storage is unavailable and localStorage fallback is active */
  isUsingInsecureFallback: boolean;
  setApiKey: (keyType: ApiKeyType, value: string) => Promise<void>;
  refreshKeys: () => Promise<void>;
}

/**
 * Hook to manage API keys with secure storage
 *
 * On first load, this hook will:
 * 1. Check if secure storage is available
 * 2. Migrate existing keys from localStorage if needed
 * 3. Load keys from secure storage
 *
 * When setting a key, it will:
 * 1. Store in secure storage
 * 2. Update the Zustand store for immediate UI access
 */
export function useSecureApiKeys(): UseSecureApiKeysReturn {
  const [isLoading, setIsLoading] = useState(true);
  const [secureAvailable, setSecureAvailable] = useState(false);
  const [fallbackMode, setFallbackMode] = useState(false);
  const migrationAttempted = useRef(false);

  // Get current keys from Zustand store
  const brandfetchApiKey = useSettingsStore((s) => s.brandfetchApiKey);
  const finnhubApiKey = useSettingsStore((s) => s.finnhubApiKey);
  const coingeckoApiKey = useSettingsStore((s) => s.coingeckoApiKey);
  const alphaVantageApiKey = useSettingsStore((s) => s.alphaVantageApiKey);
  const twelveDataApiKey = useSettingsStore((s) => s.twelveDataApiKey);
  const anthropicApiKey = useSettingsStore((s) => s.anthropicApiKey);
  const openaiApiKey = useSettingsStore((s) => s.openaiApiKey);
  const geminiApiKey = useSettingsStore((s) => s.geminiApiKey);
  const perplexityApiKey = useSettingsStore((s) => s.perplexityApiKey);

  // Get setters from Zustand store
  const setBrandfetchApiKey = useSettingsStore((s) => s.setBrandfetchApiKey);
  const setFinnhubApiKey = useSettingsStore((s) => s.setFinnhubApiKey);
  const setCoingeckoApiKey = useSettingsStore((s) => s.setCoingeckoApiKey);
  const setAlphaVantageApiKey = useSettingsStore((s) => s.setAlphaVantageApiKey);
  const setTwelveDataApiKey = useSettingsStore((s) => s.setTwelveDataApiKey);
  const setAnthropicApiKey = useSettingsStore((s) => s.setAnthropicApiKey);
  const setOpenaiApiKey = useSettingsStore((s) => s.setOpenaiApiKey);
  const setGeminiApiKey = useSettingsStore((s) => s.setGeminiApiKey);
  const setPerplexityApiKey = useSettingsStore((s) => s.setPerplexityApiKey);

  const keys: SecureApiKeys = {
    brandfetchApiKey,
    finnhubApiKey,
    coingeckoApiKey,
    alphaVantageApiKey,
    twelveDataApiKey,
    anthropicApiKey,
    openaiApiKey,
    geminiApiKey,
    perplexityApiKey,
  };

  // Map key types to Zustand setters
  const setterMap: Record<ApiKeyType, (key: string) => void> = {
    brandfetch: setBrandfetchApiKey,
    finnhub: setFinnhubApiKey,
    coingecko: setCoingeckoApiKey,
    alphaVantage: setAlphaVantageApiKey,
    twelveData: setTwelveDataApiKey,
    anthropic: setAnthropicApiKey,
    openai: setOpenaiApiKey,
    gemini: setGeminiApiKey,
    perplexity: setPerplexityApiKey,
  };

  // Load keys from secure storage and sync with Zustand
  const loadKeys = useCallback(async () => {
    try {
      const available = await isSecureStorageAvailable();
      setSecureAvailable(available);

      if (!available) {
        // Secure storage not available - use localStorage fallback
        console.warn('Secure storage not available, using localStorage fallback mode');
        setFallbackMode(true);

        // In fallback mode, read keys from localStorage and update Zustand
        // SECURITY FIX: Always set all keys (including empty) to clear stale values
        const legacyKeys = readLegacyKeysFromLocalStorage();
        setBrandfetchApiKey(legacyKeys.brandfetch || '');
        setFinnhubApiKey(legacyKeys.finnhub || '');
        setCoingeckoApiKey(legacyKeys.coingecko || '');
        setAlphaVantageApiKey(legacyKeys.alphaVantage || '');
        setTwelveDataApiKey(legacyKeys.twelveData || '');
        setAnthropicApiKey(legacyKeys.anthropic || '');
        setOpenaiApiKey(legacyKeys.openai || '');
        setGeminiApiKey(legacyKeys.gemini || '');
        setPerplexityApiKey(legacyKeys.perplexity || '');

        setIsLoading(false);
        return;
      }

      // Secure storage is available - migrate from localStorage if not done yet
      // Read directly from localStorage to avoid Zustand hydration race condition
      if (!migrationAttempted.current) {
        migrationAttempted.current = true;
        const legacyKeys = readLegacyKeysFromLocalStorage();

        // Only migrate if there are actually keys to migrate
        const hasLegacyKeys = Object.values(legacyKeys).some(k => k && k.length > 0);
        if (hasLegacyKeys) {
          const migrated = await migrateFromLocalStorage(legacyKeys, clearLegacyKeysFromLocalStorage);
          if (migrated) {
            console.log('Migrated API keys from localStorage to secure storage');
          }
        }
      }

      // Load keys from secure storage
      const secureKeys = await getAllApiKeys();

      // Update Zustand store with secure keys
      // SECURITY FIX: Always set all keys (including empty) to clear stale values
      // This ensures deleted keys in secure storage are reflected in the UI
      setBrandfetchApiKey(secureKeys.brandfetch);
      setFinnhubApiKey(secureKeys.finnhub);
      setCoingeckoApiKey(secureKeys.coingecko);
      setAlphaVantageApiKey(secureKeys.alphaVantage);
      setTwelveDataApiKey(secureKeys.twelveData);
      setAnthropicApiKey(secureKeys.anthropic);
      setOpenaiApiKey(secureKeys.openai);
      setGeminiApiKey(secureKeys.gemini);
      setPerplexityApiKey(secureKeys.perplexity);
    } catch (error) {
      console.error('Failed to load API keys from secure storage:', error);
    } finally {
      setIsLoading(false);
    }
  }, []); // Empty deps - only run once on mount

  // Load keys on mount
  useEffect(() => {
    loadKeys();
  }, [loadKeys]);

  // Set a single API key (stores in secure storage and updates Zustand)
  const setApiKey = useCallback(
    async (keyType: ApiKeyType, value: string) => {
      // Update Zustand immediately for UI responsiveness
      const setter = setterMap[keyType];
      if (setter) {
        setter(value);
      }

      // Store in appropriate storage
      if (secureAvailable) {
        try {
          await storeApiKey(keyType, value);
        } catch (error) {
          console.error(`Failed to store ${keyType} key securely:`, error);
          // Key is still in Zustand, so UI will work
        }
      } else if (fallbackMode) {
        // Fallback: persist to localStorage via Zustand's storage key
        try {
          const raw = localStorage.getItem(ZUSTAND_STORAGE_KEY);
          const parsed = raw ? JSON.parse(raw) : { state: {} };
          const state = parsed?.state || parsed;

          // Map ApiKeyType to Zustand field name
          const fieldMap: Record<ApiKeyType, string> = {
            brandfetch: 'brandfetchApiKey',
            finnhub: 'finnhubApiKey',
            coingecko: 'coingeckoApiKey',
            alphaVantage: 'alphaVantageApiKey',
            twelveData: 'twelveDataApiKey',
            anthropic: 'anthropicApiKey',
            openai: 'openaiApiKey',
            gemini: 'geminiApiKey',
            perplexity: 'perplexityApiKey',
          };

          state[fieldMap[keyType]] = value;

          if (parsed?.state) {
            parsed.state = state;
            localStorage.setItem(ZUSTAND_STORAGE_KEY, JSON.stringify(parsed));
          } else {
            localStorage.setItem(ZUSTAND_STORAGE_KEY, JSON.stringify({ state }));
          }
        } catch (error) {
          console.error(`Failed to store ${keyType} key in localStorage fallback:`, error);
        }
      }
    },
    [secureAvailable, fallbackMode, setterMap]
  );

  return {
    keys,
    isLoading,
    isSecureStorageAvailable: secureAvailable,
    isUsingInsecureFallback: fallbackMode,
    setApiKey,
    refreshKeys: loadKeys,
  };
}
