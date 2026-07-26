// Quiet loading / error / empty states. No spinners on the instruments — a
// dim pulse while loading, one alarm line + Retry on error, a plain line when
// a scope legitimately has no rows (empty ≠ error, ever).
//
// D-2 (2026-07-25): the alarm line must be TRUE. It used to default to
// "Couldn’t reach the API." for every caller that passed no message — so a
// typed 500 from a reachable, answering API was reported as a connectivity
// problem, and that message sent diagnosis at the network. Now:
//   · pass `error` and the panel names what actually failed, including the
//     server's typed code and message (see describeError);
//   · pass `message` to override with screen-specific copy (unchanged);
//   · pass neither and the fallback is cause-agnostic — it no longer claims
//     to know something it does not.
// Only a genuine transport failure (NetworkError) says the API was
// unreachable, because only then is that true.

import { RotateCw } from "lucide-react";
import { describeError } from "../lib/api";

export function LoadingPanel({ label = "Loading" }: { label?: string }) {
  return (
    <div className="pulse rounded-lg border border-seam bg-surface p-6">
      <span className="nameplate text-2xs text-text-dim">{label}…</span>
    </div>
  );
}

export function ErrorPanel({
  onRetry,
  error,
  message,
  retryLabel = "Retry",
}: {
  onRetry: () => void;
  error?: unknown;
  message?: string;
  /** D-3: the error boundaries say "Try again" — plainer than "Retry" for a
   *  failure the user did not cause and cannot read a status code about. */
  retryLabel?: string;
}) {
  const line =
    message ??
    (error === undefined || error === null
      ? "Couldn’t load this panel."
      : describeError(error));
  return (
    <div className="rounded-lg border border-alarm/40 bg-surface p-6">
      <div className="text-xs text-alarm" data-testid="error-message">
        {line}
      </div>
      <button
        onClick={onRetry}
        className="mt-3 inline-flex items-center gap-1.5 rounded border border-seam px-2.5 py-1.5 text-2xs text-text-dim transition-colors hover:bg-surface-2 hover:text-text"
      >
        <RotateCw size={13} strokeWidth={2} />
        <span className="nameplate">{retryLabel}</span>
      </button>
    </div>
  );
}

export function EmptyPanel({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-seam bg-surface p-6 text-xs text-text-dim">
      {message}
    </div>
  );
}
