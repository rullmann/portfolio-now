/**
 * Hook for secure API key management
 *
 * This hook provides a bridge between the UI and secure storage for API keys.
 * It handles loading keys from secure storage on mount and saving them when changed.
 *
 * SECURITY: API keys are stored in Tauri's secure store (app_data_dir/secure-keys.json)
 * instead of browser localStorage. This prevents access from arbitrary JavaScript code.
 */

import { useEffect, useState, useCallback } from 'react';
import {
  getAllApiKeys,
  storeApiKey,
  migrateFromLocalStorage,
  isSecureStorageAvailable,
  type ApiKeyType,
} from '../lib/secureStorage';
import { useSettingsStore } from '../store';

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
        console.warn('Secure storage not available, using localStorage fallback');
        setIsLoading(false);
        return;
      }

      // Try to migrate from localStorage first
      const localStorageKeys = {
        brandfetch: brandfetchApiKey,
        finnhub: finnhubApiKey,
        coingecko: coingeckoApiKey,
        alphaVantage: alphaVantageApiKey,
        twelveData: twelveDataApiKey,
        anthropic: anthropicApiKey,
        openai: openaiApiKey,
        gemini: geminiApiKey,
        perplexity: perplexityApiKey,
      };

      const migrated = await migrateFromLocalStorage(localStorageKeys);
      if (migrated) {
        console.log('Migrated API keys from localStorage to secure storage');
      }

      // Load keys from secure storage
      const secureKeys = await getAllApiKeys();

      // Update Zustand store with secure keys
      if (secureKeys.brandfetch) setBrandfetchApiKey(secureKeys.brandfetch);
      if (secureKeys.finnhub) setFinnhubApiKey(secureKeys.finnhub);
      if (secureKeys.coingecko) setCoingeckoApiKey(secureKeys.coingecko);
      if (secureKeys.alphaVantage) setAlphaVantageApiKey(secureKeys.alphaVantage);
      if (secureKeys.twelveData) setTwelveDataApiKey(secureKeys.twelveData);
      if (secureKeys.anthropic) setAnthropicApiKey(secureKeys.anthropic);
      if (secureKeys.openai) setOpenaiApiKey(secureKeys.openai);
      if (secureKeys.gemini) setGeminiApiKey(secureKeys.gemini);
      if (secureKeys.perplexity) setPerplexityApiKey(secureKeys.perplexity);
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

      // Store in secure storage
      if (secureAvailable) {
        try {
          await storeApiKey(keyType, value);
        } catch (error) {
          console.error(`Failed to store ${keyType} key securely:`, error);
          // Key is still in Zustand, so UI will work
        }
      }
    },
    [secureAvailable, setterMap]
  );

  return {
    keys,
    isLoading,
    isSecureStorageAvailable: secureAvailable,
    setApiKey,
    refreshKeys: loadKeys,
  };
}
