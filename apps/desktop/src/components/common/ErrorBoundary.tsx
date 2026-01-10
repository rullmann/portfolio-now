import { Component, type ErrorInfo, type ReactNode } from 'react';
import { AlertTriangle, RefreshCw } from 'lucide-react';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
  resetKeys?: unknown[];
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
}

/**
 * Error Boundary component to catch React errors.
 * Prevents app crashes from propagating and shows a user-friendly error message.
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error, errorInfo: null };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught error:', error, errorInfo);
    this.setState({ errorInfo });
    this.props.onError?.(error, errorInfo);
  }

  componentDidUpdate(prevProps: Props) {
    // Reset error state when resetKeys change
    if (this.state.hasError && this.props.resetKeys) {
      const hasChanged = this.props.resetKeys.some(
        (key, index) => key !== prevProps.resetKeys?.[index]
      );
      if (hasChanged) {
        this.setState({ hasError: false, error: null, errorInfo: null });
      }
    }
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null, errorInfo: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="flex flex-col items-center justify-center p-8 text-center">
          <AlertTriangle className="w-12 h-12 text-destructive mb-4" />
          <h3 className="text-lg font-semibold mb-2">Ein Fehler ist aufgetreten</h3>
          <p className="text-sm text-muted-foreground mb-4 max-w-md">
            {this.state.error?.message || 'Unbekannter Fehler'}
          </p>
          {process.env.NODE_ENV === 'development' && this.state.errorInfo && (
            <details className="mb-4 text-left max-w-full overflow-auto">
              <summary className="cursor-pointer text-sm text-muted-foreground hover:text-foreground">
                Technische Details
              </summary>
              <pre className="mt-2 p-3 bg-muted rounded-md text-xs overflow-auto max-h-48">
                {this.state.error?.stack}
                {'\n\nComponent Stack:\n'}
                {this.state.errorInfo.componentStack}
              </pre>
            </details>
          )}
          <button
            onClick={this.handleRetry}
            className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
          >
            <RefreshCw size={16} />
            Erneut versuchen
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
