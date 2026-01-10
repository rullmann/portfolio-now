/**
 * Error handling utilities with German error messages and retry logic.
 */

// ============================================================================
// Error Types
// ============================================================================

export type ErrorCode =
  | 'NETWORK_ERROR'
  | 'TIMEOUT_ERROR'
  | 'SERVER_ERROR'
  | 'VALIDATION_ERROR'
  | 'NOT_FOUND'
  | 'UNAUTHORIZED'
  | 'RATE_LIMITED'
  | 'UNKNOWN_ERROR';

export interface AppError {
  code: ErrorCode;
  message: string;
  originalError?: unknown;
  retryable: boolean;
}

// ============================================================================
// German Error Messages
// ============================================================================

const errorMessages: Record<ErrorCode, string> = {
  NETWORK_ERROR: 'Netzwerkfehler. Bitte prüfen Sie Ihre Internetverbindung.',
  TIMEOUT_ERROR: 'Die Anfrage hat zu lange gedauert. Bitte versuchen Sie es erneut.',
  SERVER_ERROR: 'Serverfehler. Bitte versuchen Sie es später erneut.',
  VALIDATION_ERROR: 'Ungültige Eingabe. Bitte prüfen Sie Ihre Daten.',
  NOT_FOUND: 'Die angeforderte Ressource wurde nicht gefunden.',
  UNAUTHORIZED: 'Keine Berechtigung für diese Aktion.',
  RATE_LIMITED: 'Zu viele Anfragen. Bitte warten Sie einen Moment.',
  UNKNOWN_ERROR: 'Ein unbekannter Fehler ist aufgetreten.',
};

// ============================================================================
// Error Classification
// ============================================================================

/**
 * Classify an error and return a structured AppError
 */
export function classifyError(error: unknown): AppError {
  // Handle string errors
  if (typeof error === 'string') {
    return classifyErrorMessage(error);
  }

  // Handle Error objects
  if (error instanceof Error) {
    return classifyErrorMessage(error.message, error);
  }

  // Handle unknown errors
  return {
    code: 'UNKNOWN_ERROR',
    message: errorMessages.UNKNOWN_ERROR,
    originalError: error,
    retryable: false,
  };
}

function classifyErrorMessage(message: string, originalError?: unknown): AppError {
  const lowerMessage = message.toLowerCase();

  // Network errors
  if (
    lowerMessage.includes('network') ||
    lowerMessage.includes('fetch') ||
    lowerMessage.includes('connection') ||
    lowerMessage.includes('offline') ||
    lowerMessage.includes('netzwerk')
  ) {
    return {
      code: 'NETWORK_ERROR',
      message: errorMessages.NETWORK_ERROR,
      originalError,
      retryable: true,
    };
  }

  // Timeout errors
  if (lowerMessage.includes('timeout') || lowerMessage.includes('timed out')) {
    return {
      code: 'TIMEOUT_ERROR',
      message: errorMessages.TIMEOUT_ERROR,
      originalError,
      retryable: true,
    };
  }

  // Rate limiting
  if (
    lowerMessage.includes('rate limit') ||
    lowerMessage.includes('too many requests') ||
    lowerMessage.includes('429')
  ) {
    return {
      code: 'RATE_LIMITED',
      message: errorMessages.RATE_LIMITED,
      originalError,
      retryable: true,
    };
  }

  // Not found
  if (lowerMessage.includes('not found') || lowerMessage.includes('404')) {
    return {
      code: 'NOT_FOUND',
      message: errorMessages.NOT_FOUND,
      originalError,
      retryable: false,
    };
  }

  // Validation errors
  if (
    lowerMessage.includes('invalid') ||
    lowerMessage.includes('validation') ||
    lowerMessage.includes('ungültig')
  ) {
    return {
      code: 'VALIDATION_ERROR',
      message: errorMessages.VALIDATION_ERROR,
      originalError,
      retryable: false,
    };
  }

  // Server errors
  if (
    lowerMessage.includes('server') ||
    lowerMessage.includes('500') ||
    lowerMessage.includes('502') ||
    lowerMessage.includes('503')
  ) {
    return {
      code: 'SERVER_ERROR',
      message: errorMessages.SERVER_ERROR,
      originalError,
      retryable: true,
    };
  }

  // Default: return original message but mark as unknown
  return {
    code: 'UNKNOWN_ERROR',
    message: message || errorMessages.UNKNOWN_ERROR,
    originalError,
    retryable: false,
  };
}

// ============================================================================
// Retry Logic
// ============================================================================

export interface RetryOptions {
  maxRetries?: number;
  delayMs?: number;
  backoffMultiplier?: number;
  onRetry?: (attempt: number, error: AppError) => void;
}

const defaultRetryOptions: Required<Omit<RetryOptions, 'onRetry'>> = {
  maxRetries: 3,
  delayMs: 1000,
  backoffMultiplier: 2,
};

/**
 * Execute a function with automatic retry for retryable errors
 */
export async function withRetry<T>(
  fn: () => Promise<T>,
  options: RetryOptions = {}
): Promise<T> {
  const { maxRetries, delayMs, backoffMultiplier } = {
    ...defaultRetryOptions,
    ...options,
  };

  let lastError: AppError | null = null;
  let currentDelay = delayMs;

  for (let attempt = 1; attempt <= maxRetries + 1; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = classifyError(error);

      // If error is not retryable or we've exhausted retries, throw
      if (!lastError.retryable || attempt > maxRetries) {
        throw lastError;
      }

      // Notify about retry
      options.onRetry?.(attempt, lastError);

      // Wait before retrying
      await sleep(currentDelay);
      currentDelay *= backoffMultiplier;
    }
  }

  // Should never reach here, but TypeScript needs this
  throw lastError;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ============================================================================
// Error Formatting
// ============================================================================

/**
 * Get a user-friendly German error message
 */
export function getErrorMessage(error: unknown): string {
  const appError = classifyError(error);
  return appError.message;
}

/**
 * Format error for logging (includes technical details)
 */
export function formatErrorForLog(error: unknown): string {
  const appError = classifyError(error);
  const originalMessage =
    appError.originalError instanceof Error
      ? appError.originalError.message
      : String(appError.originalError || '');

  return `[${appError.code}] ${appError.message}${
    originalMessage ? ` (Original: ${originalMessage})` : ''
  }`;
}

// ============================================================================
// Global Error Handler
// ============================================================================

type ErrorHandler = (error: AppError) => void;

let globalErrorHandler: ErrorHandler | null = null;

/**
 * Set the global error handler
 */
export function setGlobalErrorHandler(handler: ErrorHandler): void {
  globalErrorHandler = handler;
}

/**
 * Handle an error globally (if handler is set)
 */
export function handleError(error: unknown): AppError {
  const appError = classifyError(error);

  if (globalErrorHandler) {
    globalErrorHandler(appError);
  }

  // Log to console for debugging
  console.error(formatErrorForLog(error));

  return appError;
}

// ============================================================================
// API Error Wrapper
// ============================================================================

/**
 * Wrap an API call with error handling
 */
export async function wrapApiCall<T>(
  fn: () => Promise<T>,
  options: {
    retry?: boolean | RetryOptions;
    silent?: boolean;
  } = {}
): Promise<T> {
  try {
    if (options.retry) {
      const retryOptions =
        typeof options.retry === 'object' ? options.retry : undefined;
      return await withRetry(fn, retryOptions);
    }
    return await fn();
  } catch (error) {
    const appError = error instanceof Object && 'code' in error
      ? (error as AppError)
      : classifyError(error);

    if (!options.silent && globalErrorHandler) {
      globalErrorHandler(appError);
    }

    throw appError;
  }
}
