// The leakage LED. Bands are RELATIVE to the visible book (architect
// resolution 10): green = leaking no faster than the aggregate; amber = up to
// +3 points worse; red = more than +3 points worse. Null leakage → off (mist).
// One sentence for the room: "red means leaking margin faster than the book."

export type LedBand = "ok" | "warn" | "alarm" | "off";

/** pct = this row's leakage_pct; agg = Σleakage / Σgross of the visible rows. */
export function leakageBand(pct: number | null, agg: number): LedBand {
  if (pct === null) return "off";
  if (pct <= agg) return "ok";
  if (pct <= agg + 3) return "warn";
  return "alarm";
}

const CLASS: Record<LedBand, string> = {
  ok: "bg-ok",
  warn: "bg-warn",
  alarm: "bg-alarm",
  off: "bg-text-dim/40",
};

export function Led({ band, title }: { band: LedBand; title?: string }) {
  return (
    <span
      title={title}
      data-testid="leakage-led"
      data-band={band}
      className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${CLASS[band]}`}
    />
  );
}
