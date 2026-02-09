/**
 * Twitter/X OAuth Authentication Hook
 *
 * Manages the OAuth 2.0 PKCE flow for connecting to X/Twitter.
 * Handles token storage, auto-refresh, and connection state.
 */

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';
import { getApiKey } from '../lib/secureStorage';
import {
  storeTwitterTokens,
  getTwitterTokens,
  clearTwitterTokens,
  type TwitterTokens,
} from '../lib/twitterStorage';

export interface UseTwitterAuthReturn {
  isConnected: boolean;
  isConnecting: boolean;
  username: string | null;
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  getAccessToken: () => Promise<string | null>;
}

export function useTwitterAuth(): UseTwitterAuthReturn {
  const [tokens, setTokens] = useState<TwitterTokens | null>(null);
  const [isConnecting, setIsConnecting] = useState(false);

  // Load stored tokens on mount
  useEffect(() => {
    getTwitterTokens().then(setTokens);
  }, []);

  const isConnected = tokens !== null;
  const username = tokens?.username ?? null;

  const connect = useCallback(async () => {
    setIsConnecting(true);
    try {
      // 1. Get client ID from secure storage
      const clientId = await getApiKey('twitterClientId');
      if (!clientId) {
        throw new Error('Twitter Client ID nicht konfiguriert. Bitte in den Einstellungen eingeben.');
      }

      // 2. Start OAuth flow (generates PKCE, returns auth URL)
      const { auth_url } = await invoke<{ auth_url: string; port: number }>(
        'twitter_start_auth',
        { clientId }
      );

      // 3. Open browser for authorization
      await open(auth_url);

      // 4. Wait for callback (blocks until redirect or 120s timeout)
      const { code } = await invoke<{ code: string; state: string }>(
        'twitter_await_callback'
      );

      // 5. Exchange code for tokens
      const tokenResponse = await invoke<{
        access_token: string;
        refresh_token: string | null;
        expires_in: number;
      }>('twitter_exchange_token', { clientId, code });

      // 6. Get user info
      const user = await invoke<{ id: string; name: string; username: string }>(
        'twitter_get_user_info',
        { accessToken: tokenResponse.access_token }
      );

      // 7. Store tokens
      const newTokens: TwitterTokens = {
        accessToken: tokenResponse.access_token,
        refreshToken: tokenResponse.refresh_token ?? '',
        expiresAt: Date.now() + (tokenResponse.expires_in * 1000),
        username: user.username,
        userId: user.id,
      };

      await storeTwitterTokens(newTokens);
      setTokens(newTokens);
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const disconnect = useCallback(async () => {
    await clearTwitterTokens();
    setTokens(null);
  }, []);

  const getAccessToken = useCallback(async (): Promise<string | null> => {
    const currentTokens = await getTwitterTokens();
    if (!currentTokens) return null;

    // Check if token is still valid (with 5 min buffer)
    if (currentTokens.expiresAt > Date.now() + 5 * 60 * 1000) {
      return currentTokens.accessToken;
    }

    // Token expired or about to expire, refresh it
    if (!currentTokens.refreshToken) return null;

    try {
      const clientId = await getApiKey('twitterClientId');
      if (!clientId) return null;

      const tokenResponse = await invoke<{
        access_token: string;
        refresh_token: string | null;
        expires_in: number;
      }>('twitter_refresh_token', {
        clientId,
        refreshToken: currentTokens.refreshToken,
      });

      const refreshedTokens: TwitterTokens = {
        ...currentTokens,
        accessToken: tokenResponse.access_token,
        refreshToken: tokenResponse.refresh_token ?? currentTokens.refreshToken,
        expiresAt: Date.now() + (tokenResponse.expires_in * 1000),
      };

      await storeTwitterTokens(refreshedTokens);
      setTokens(refreshedTokens);

      return refreshedTokens.accessToken;
    } catch (error) {
      console.error('Failed to refresh Twitter token:', error);
      // Token refresh failed, clear tokens
      await clearTwitterTokens();
      setTokens(null);
      return null;
    }
  }, []);

  return {
    isConnected,
    isConnecting,
    username,
    connect,
    disconnect,
    getAccessToken,
  };
}
