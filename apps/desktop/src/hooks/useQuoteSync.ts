import { useCallback, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { syncAllPrices, type ApiKeys } from '../lib/api';
import { useQuoteSyncStore, type QuoteSyncProgressItem } from '../store/quoteSyncStore';

interface QuoteSyncProgressEvent {
  securityId: number;
  securityName: string;
  ticker: string;
  provider: string;
  status: 'started' | 'loading' | 'success' | 'error';
  price: number | null;
  currency: string | null;
  error: string | null;
  currentIndex: number;
  totalCount: number;
}

export function useQuoteSync() {
  const { isActive } = useQuoteSyncStore();
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const autoDismissRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const startTimeRef = useRef<number>(0);

  const startSync = useCallback(
    async (onlyHeld: boolean, apiKeys?: ApiKeys): Promise<void> => {
      if (isActive) return;

      startTimeRef.current = Date.now();
      const store = useQuoteSyncStore.getState();

      // Clean up previous listener if any
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      if (autoDismissRef.current) {
        clearTimeout(autoDismissRef.current);
        autoDismissRef.current = null;
      }

      // Set up event listener before starting sync
      unlistenRef.current = await listen<QuoteSyncProgressEvent>(
        'quote_sync_progress',
        (event) => {
          const data = event.payload;
          const syncStore = useQuoteSyncStore.getState();

          if (data.status === 'started') {
            syncStore.startSync(data.totalCount);
          } else {
            syncStore.updateProgress({
              securityId: data.securityId,
              securityName: data.securityName,
              ticker: data.ticker,
              provider: data.provider,
              status: data.status as QuoteSyncProgressItem['status'],
              price: data.price ?? undefined,
              currency: data.currency ?? undefined,
              error: data.error ?? undefined,
              currentIndex: data.currentIndex,
              totalCount: data.totalCount,
            });
          }
        }
      );

      try {
        // Optimistically show the overlay in case the "started" event hasn't arrived yet
        store.startSync(0);

        await syncAllPrices(onlyHeld, apiKeys);

        // Mark sync as complete
        const finalStore = useQuoteSyncStore.getState();
        finalStore.completeSync();

        // Ensure minimum display time of 1.5s
        const elapsed = Date.now() - startTimeRef.current;
        const minDisplayMs = 1500;
        const remainingMs = Math.max(0, minDisplayMs - elapsed);

        // Check if there were errors
        const hasErrors = Array.from(finalStore.items.values()).some(
          (item) => item.status === 'error'
        );

        // Auto-dismiss after 3s if no errors (+ remaining min display time)
        if (!hasErrors) {
          autoDismissRef.current = setTimeout(() => {
            useQuoteSyncStore.getState().dismiss();
          }, remainingMs + 3000);
        }
      } catch (err) {
        // On total failure, dismiss after min display
        const elapsed = Date.now() - startTimeRef.current;
        const delay = Math.max(0, 1500 - elapsed);
        autoDismissRef.current = setTimeout(() => {
          useQuoteSyncStore.getState().dismiss();
        }, delay);
        // Re-throw so callers can handle failures correctly
        throw err;
      } finally {
        // Clean up listener
        if (unlistenRef.current) {
          unlistenRef.current();
          unlistenRef.current = null;
        }
      }
    },
    [isActive]
  );

  return { startSync, isActive };
}
