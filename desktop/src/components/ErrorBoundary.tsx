import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";

export class ErrorBoundary extends Component<
  { children: ReactNode; label: string },
  { error?: Error }
> {
  state: { error?: Error } = {};

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`Failed to render ${this.props.label}`, error, info);
  }

  componentDidUpdate(previous: Readonly<{ children: ReactNode; label: string }>) {
    if (previous.label !== this.props.label && this.state.error) {
      this.setState({ error: undefined });
    }
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="component-error" role="alert">
        <AlertTriangle size={24} />
        <strong>{this.props.label} could not be displayed</strong>
        <span>{this.state.error.message}</span>
        <button type="button" onClick={() => this.setState({ error: undefined })}>
          <RefreshCw size={13} />
          Try again
        </button>
      </div>
    );
  }
}
