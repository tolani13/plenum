// D-3 (2026-07-26): the blank-screen blast door.
//
// React has exactly one mechanism for a render-time failure — a class
// component with getDerivedStateFromError/componentDidCatch. There is no hook
// form and no way to do it from outside React, so this is the lowest layer
// available on the client. Without one, an error thrown during render
// unmounts the WHOLE root: the document goes empty, nav and all, and the only
// way back is a manual page reload — which a user staring at a black viewport
// has no reason to expect, and which loses whatever they were looking at.
//
// <Suspense> is NOT this. Suspense owns the pending state of a lazy import();
// a REJECTED import() is rethrown during render and passes straight through
// it. That is why the four lazy routes could black out the app.
//
// Reset semantics, two independent doors:
//   · resetKey — change it and a caught error clears itself (the screen
//     boundary passes the pathname, so navigating away always recovers);
//   · onRetry  — the panel's button clears the error and tells the owner to
//     rebuild whatever failed (LazyRoute rebuilds the lazy component, because
//     React.lazy memoizes its rejection and would otherwise replay it).

import { Component, type ErrorInfo, type ReactNode } from "react";
import { ApiError, NetworkError, describeError } from "../lib/api";
import { ErrorPanel } from "./states";

/** A dynamic import() that never arrived — the browser's own wording varies. */
export function isChunkLoadError(error: unknown): boolean {
  if (!(error instanceof Error)) return false;
  const m = error.message;
  return (
    m.includes("Failed to fetch dynamically imported module") ||
    m.includes("error loading dynamically imported module") ||
    m.includes("Importing a module script failed") ||
    m.includes("Failed to load module script") ||
    m.includes("Unable to preload CSS")
  );
}

/** One true sentence about a RENDER failure.
 *
 *  D-2's law applies here too: a message must not assert a cause it cannot
 *  know. `describeError` speaks about REQUESTS, so it is the right voice only
 *  when the thing that reached the boundary really was an API failure. A
 *  chunk that did not download is a script-loading failure — it must never
 *  borrow the API's wording, and it must never claim the API was unreachable
 *  (the API may be perfectly healthy; the browser simply never got the code).
 */
export function describeRenderError(error: unknown): string {
  if (isChunkLoadError(error)) {
    return "This screen’s code didn’t finish downloading, so it couldn’t be shown. Nothing else in PLENUM was affected.";
  }
  if (error instanceof ApiError || error instanceof NetworkError) {
    return describeError(error);
  }
  if (error instanceof Error && error.message) {
    return `This screen failed to render: ${error.message}`;
  }
  return "This screen failed to render, and gave no reason.";
}

/** The same sentence for the ROOT boundary, which has no screen to speak of —
 *  when it catches, what failed is the application, and saying "this screen"
 *  there would be the D-2 mistake in a new place. */
export function describeRootError(error: unknown): string {
  if (isChunkLoadError(error) || error instanceof ApiError || error instanceof NetworkError) {
    return describeRenderError(error);
  }
  if (error instanceof Error && error.message) {
    return `PLENUM failed to render: ${error.message}`;
  }
  return "PLENUM failed to render, and gave no reason.";
}

interface Props {
  children: ReactNode;
  /** Names the guarded region in the console line and in the DOM. */
  region: string;
  /** Any change to this value clears a caught error (e.g. the pathname). */
  resetKey?: unknown;
  /** Called after the panel's button clears the error, to rebuild the child.
   *  Receives the caught error — LazyRoute needs the failed module's URL. */
  onRetry?: (error: unknown) => void;
  /** Plain-language button label. Never a stack trace, never jargon. */
  retryLabel?: string;
  /** Overrides the panel's sentence — LazyRoute says something different once
   *  a retry has already been spent. Defaults to describeRenderError. */
  describe?: (error: unknown) => string;
  /** Wrapper classes — the root boundary paints its own page, having no shell. */
  className?: string;
}

interface State {
  hasError: boolean;
  error: unknown;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: undefined };

  static getDerivedStateFromError(error: unknown): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: unknown, info: ErrorInfo): void {
    // Once, with the component stack — the panel stays plain-language, the
    // console keeps everything a developer needs.
    console.error(
      `[PLENUM] render failure caught by the ${this.props.region} boundary`,
      error,
      info.componentStack,
    );
  }

  componentDidUpdate(prev: Props): void {
    if (this.state.hasError && prev.resetKey !== this.props.resetKey) {
      this.setState({ hasError: false, error: undefined });
    }
  }

  private handleRetry = (): void => {
    const { error } = this.state;
    this.setState({ hasError: false, error: undefined });
    this.props.onRetry?.(error);
  };

  render(): ReactNode {
    if (!this.state.hasError) return this.props.children;
    return (
      <div
        data-testid="error-boundary"
        data-region={this.props.region}
        className={this.props.className}
      >
        <ErrorPanel
          onRetry={this.handleRetry}
          message={(this.props.describe ?? describeRenderError)(
            this.state.error,
          )}
          retryLabel={this.props.retryLabel ?? "Try again"}
        />
      </div>
    );
  }
}
