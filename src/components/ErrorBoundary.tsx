import { Component, type ErrorInfo, type ReactNode } from "react";

type ErrorBoundaryProps = {
  children: ReactNode;
};

type ErrorBoundaryState = {
  hasError: boolean;
  errorMessage: string;
};

export default class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  public constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = {
      hasError: false,
      errorMessage: "",
    };
  }

  public static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return {
      hasError: true,
      errorMessage: error.message || "Unknown runtime error",
    };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("ErrorBoundary captured an exception:", error, errorInfo);
  }

  public render() {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-zinc-950 p-6">
        <div className="w-full max-w-xl rounded-2xl border border-rose-500/30 bg-zinc-900 p-6 text-center shadow-2xl">
          <h1 className="text-lg font-semibold text-rose-500">UI Runtime Error</h1>
          <p className="mt-2 text-sm text-zinc-400">
            An unexpected render error occurred. Check DevTools console for stack trace.
          </p>
          <pre className="mt-4 overflow-auto rounded-lg border border-zinc-800 bg-zinc-950 p-3 text-left text-xs text-rose-500">
            {this.state.errorMessage}
          </pre>
          <button
            className="mt-5 rounded-lg bg-rose-500 px-4 py-2 text-sm font-medium text-white transition hover:bg-rose-500"
            onClick={() => window.location.reload()}
          >
            Reload App
          </button>
        </div>
      </div>
    );
  }
}
