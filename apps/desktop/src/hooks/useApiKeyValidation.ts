import { useState, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type ValidationState = 'idle' | 'validating' | 'valid' | 'invalid';

export interface ValidationResult {
  state: ValidationState;
  error?: string;
}

interface ApiKeyValidationResponse {
  valid: boolean;
  error: string | null;
}

export function useApiKeyValidation(debounceMs = 1000) {
  const [validationStates, setValidationStates] = useState<Record<string, ValidationResult>>({});
  const timersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  const requestIdRef = useRef<Record<string, number>>({});

  const validateKey = useCallback((provider: string, key: string) => {
    // Clear any pending timer for this provider
    if (timersRef.current[provider]) {
      clearTimeout(timersRef.current[provider]);
    }

    // Empty key -> idle immediately
    if (!key.trim()) {
      setValidationStates(prev => {
        const next = { ...prev };
        delete next[provider];
        return next;
      });
      return;
    }

    // Set validating state after a short delay
    timersRef.current[provider] = setTimeout(async () => {
      // Track request to handle stale responses
      const requestId = (requestIdRef.current[provider] || 0) + 1;
      requestIdRef.current[provider] = requestId;

      setValidationStates(prev => ({
        ...prev,
        [provider]: { state: 'validating' },
      }));

      try {
        const result = await invoke<ApiKeyValidationResponse>('validate_api_key', {
          provider,
          apiKey: key,
        });

        // Stale check: only update if this is still the latest request
        if (requestIdRef.current[provider] !== requestId) return;

        setValidationStates(prev => ({
          ...prev,
          [provider]: result.valid
            ? { state: 'valid' }
            : { state: 'invalid', error: result.error || 'Ungültiger API Key' },
        }));
      } catch (err) {
        if (requestIdRef.current[provider] !== requestId) return;

        const errorMsg = err instanceof Error ? err.message : String(err);
        // Rate limit errors -> show as info, not as invalid key
        if (errorMsg.includes('Rate limit')) {
          setValidationStates(prev => {
            const next = { ...prev };
            delete next[provider];
            return next;
          });
        } else {
          setValidationStates(prev => ({
            ...prev,
            [provider]: { state: 'invalid', error: errorMsg },
          }));
        }
      }
    }, debounceMs);
  }, [debounceMs]);

  return { validationStates, validateKey };
}
