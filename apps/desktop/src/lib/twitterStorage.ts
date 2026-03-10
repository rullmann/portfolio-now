/**
 * Twitter Token Storage
 *
 * Stores OAuth tokens in the secure store (tauri-plugin-store).
 * Tokens are stored under the key 'twitter.tokens' in secure-keys.json.
 */

import { load, type Store } from '@tauri-apps/plugin-store';

const SECURE_STORE_FILE = 'secure-keys.json';
const TWITTER_TOKENS_KEY = 'twitter.tokens';

let secureStore: Store | null = null;

async function getStore(): Promise<Store> {
  if (!secureStore) {
    secureStore = await load(SECURE_STORE_FILE);
  }
  return secureStore;
}

export interface TwitterTokens {
  accessToken: string;
  refreshToken: string;
  expiresAt: number; // Unix timestamp (ms)
  username: string;
  userId: string;
}

export async function storeTwitterTokens(tokens: TwitterTokens): Promise<void> {
  try {
    const store = await getStore();
    await store.set(TWITTER_TOKENS_KEY, tokens);
    await store.save();
  } catch {
    console.error('Failed to store Twitter tokens');
    throw new Error('Failed to store Twitter tokens');
  }
}

export async function getTwitterTokens(): Promise<TwitterTokens | null> {
  try {
    const store = await getStore();
    const tokens = await store.get<TwitterTokens>(TWITTER_TOKENS_KEY);
    return tokens ?? null;
  } catch {
    console.error('Failed to retrieve Twitter tokens');
    return null;
  }
}

export async function clearTwitterTokens(): Promise<void> {
  try {
    const store = await getStore();
    await store.delete(TWITTER_TOKENS_KEY);
    await store.save();
  } catch {
    console.error('Failed to clear Twitter tokens');
  }
}
