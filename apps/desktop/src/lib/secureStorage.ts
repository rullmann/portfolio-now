/**
 * Secure Storage for sensitive data like API keys
 *
 * Uses Tauri's plugin-store which stores data in the app data directory.
 * This is more secure than browser localStorage because:
 * - Data is stored outside the WebView's accessible storage
 * - Not accessible by arbitrary JavaScript code injected into the page
 * - Isolated to the Tauri application context
 *
 * SECURITY NOTES:
 * - Keys are stored in app_data_dir/secure-keys.json (isolated from WebView)
 * - The store is only accessible from the Tauri app, not arbitrary JS
 * - Keys are never logged or exposed in error messages
 *
 * FUTURE: When tauri-plugin-keyring becomes stable for Tauri v2,
 * this can be upgraded to use OS-level encrypted storage (macOS Keychain,
 * Windows Credential Manager, Linux Secret Service).
 */

import { load, type Store } from '@tauri-apps/plugin-store';

// Store instance for API keys (cached after first load)
let secureStore: Store | null = null;

// Store filename for secure credentials
const SECURE_STORE_FILE = 'secure-keys.json';

/**
 * Get or create the secure store instance
 */
async function getStore(): Promise<Store> {
  if (!secureStore) {
    secureStore = await load(SECURE_STORE_FILE);
  }
  return secureStore;
}

/**
 * Available API key types
 */
export type ApiKeyType =
  | 'brandfetch'
  | 'finnhub'
  | 'coingecko'
  | 'alphaVantage'
  | 'twelveData'
  | 'anthropic'
  | 'openai'
  | 'gemini'
  | 'perplexity';

/**
 * Store an API key securely
 *
 * @param keyType - Type of API key
 * @param value - The API key value (will be stored securely)
 */
export async function storeApiKey(keyType: ApiKeyType, value: string): Promise<void> {
  try {
    const store = await getStore();
    await store.set(`apiKey.${keyType}`, value);
    await store.save();
  } catch (error) {
    // Don't expose the key value in error messages
    console.error(`Failed to store API key for ${keyType}`);
    throw new Error(`Failed to store API key: ${keyType}`);
  }
}

/**
 * Retrieve an API key from secure storage
 *
 * @param keyType - Type of API key to retrieve
 * @returns The API key value or empty string if not found
 */
export async function getApiKey(keyType: ApiKeyType): Promise<string> {
  try {
    const store = await getStore();
    const value = await store.get<string>(`apiKey.${keyType}`);
    return value ?? '';
  } catch (error) {
    console.error(`Failed to retrieve API key for ${keyType}`);
    return '';
  }
}

/**
 * Delete an API key from secure storage
 *
 * @param keyType - Type of API key to delete
 */
export async function deleteApiKey(keyType: ApiKeyType): Promise<void> {
  try {
    const store = await getStore();
    await store.delete(`apiKey.${keyType}`);
    await store.save();
  } catch (error) {
    console.error(`Failed to delete API key for ${keyType}`);
  }
}

/**
 * Get all stored API keys
 *
 * @returns Object with all API key types and their values
 */
export async function getAllApiKeys(): Promise<Record<ApiKeyType, string>> {
  const keys: ApiKeyType[] = [
    'brandfetch',
    'finnhub',
    'coingecko',
    'alphaVantage',
    'twelveData',
    'anthropic',
    'openai',
    'gemini',
    'perplexity',
  ];

  const result: Record<ApiKeyType, string> = {
    brandfetch: '',
    finnhub: '',
    coingecko: '',
    alphaVantage: '',
    twelveData: '',
    anthropic: '',
    openai: '',
    gemini: '',
    perplexity: '',
  };

  for (const key of keys) {
    result[key] = await getApiKey(key);
  }

  return result;
}

/**
 * Store multiple API keys at once
 *
 * @param keys - Object with API key types and values
 */
export async function storeAllApiKeys(keys: Partial<Record<ApiKeyType, string>>): Promise<void> {
  const store = await getStore();

  for (const [keyType, value] of Object.entries(keys)) {
    if (value !== undefined) {
      await store.set(`apiKey.${keyType}`, value);
    }
  }

  await store.save();
}

/**
 * Migrate API keys from localStorage to secure storage
 *
 * This function should be called once during app initialization
 * to migrate existing keys from the old localStorage-based storage.
 * After successful migration, the keys are removed from localStorage.
 *
 * @param localStorageKeys - Object with API keys from localStorage
 * @param clearFromLocalStorage - Callback to clear keys from localStorage/Zustand
 * @returns true if migration was performed, false if already migrated
 */
export async function migrateFromLocalStorage(
  localStorageKeys: Partial<Record<ApiKeyType, string>>,
  clearFromLocalStorage?: () => void
): Promise<boolean> {
  try {
    const store = await getStore();

    // Check if migration already done
    const migrationDone = await store.get<boolean>('migration.fromLocalStorage');
    if (migrationDone) {
      return false;
    }

    // Store all keys that have values
    let keysMigrated = 0;
    for (const [keyType, value] of Object.entries(localStorageKeys)) {
      if (value) {
        await store.set(`apiKey.${keyType}`, value);
        keysMigrated++;
      }
    }

    // Mark migration as done
    await store.set('migration.fromLocalStorage', true);
    await store.save();

    // Clear keys from localStorage after successful migration
    if (keysMigrated > 0 && clearFromLocalStorage) {
      clearFromLocalStorage();
      console.log(`Migrated ${keysMigrated} API keys and cleared from localStorage`);
    } else if (keysMigrated > 0) {
      console.log(`Migrated ${keysMigrated} API keys to secure storage`);
    }

    return true;
  } catch (error) {
    console.error('Failed to migrate API keys:', error);
    return false;
  }
}

/**
 * Check if secure storage is available
 *
 * @returns true if secure storage can be used
 */
export async function isSecureStorageAvailable(): Promise<boolean> {
  try {
    const store = await getStore();
    await store.keys();
    return true;
  } catch {
    return false;
  }
}

/**
 * Check if using OS keyring (encrypted) or store (isolated but not encrypted)
 *
 * Currently always returns false as keyring is not yet available for Tauri v2.
 * This function is provided for future compatibility.
 *
 * @returns true if OS keyring is being used (currently always false)
 */
export async function isUsingKeyring(): Promise<boolean> {
  // Keyring not available yet - return false
  // When keyring becomes available, this will check if keyring is working
  return false;
}
