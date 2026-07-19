// Quiet loading / error / empty states. No spinners on the instruments — a
// dim pulse while loading, one alarm line + Retry on error, a plain line when
// a scope legitimately has no rows (empty ≠ error, ever).

import { RotateCw } from "lucide-react";

export function LoadingPanel({ label = "Loading" }: { label?: string }) {
  return (
    <div className="pulse rounded-lg border border-seam bg-surface p-6">
      <span className="nameplate text-2xs text-text-dim">{label}…</span>
    </div>
  );
}

export function ErrorPanel({
  onRetry,
  message = "Couldn’t reach the API.",
}: {
  onRetry: () => void;
  message?: string;
}) {
  return (
    <div className="rounded-lg border border-alarm/40 bg-surface p-6">
      <div className="text-xs text-alarm">{message}</div>
      <button
        onClick={onRetry}
        className="mt-3 inline-flex items-center gap-1.5 rounded border border-seam px-2.5 py-1.5 text-2xs text-text-dim transition-colors hover:bg-surface-2 hover:text-text"
      >
        <RotateCw size={13} strokeWidth={2} />
        <span className="nameplate">Retry</span>
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
