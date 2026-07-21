// A Territory Board tile — a live instrument, uniform with its neighbours
// (§8's discipline). Nameplate (code + name), rank badge (by current basis),
// revenue (basis, abbreviated with the exact figure in `title`), a NET-based
// quota-attainment bar (attainment is net/prorated-quota BY DEFINITION — it
// does not move with the basis toggle), the leakage LED, and — P4 (R7) — the
// tile's open-signal count (spec §8's fourth tile instrument; an instrument
// reads 0, it never goes blank).

import { moneyAbbrev, money, percent } from "../lib/format";
import type { Basis } from "../lib/params";
import type { TerritoryRow } from "../lib/types";
import { Led, leakageBand } from "./Led";

function AttainmentBar({ pct }: { pct: number | null }) {
  if (pct === null) {
    return (
      <div className="mt-3 nameplate text-2xs text-text-dim">quota n/a</div>
    );
  }
  const filled = Math.max(0, Math.min(pct, 100));
  const over = pct > 100;
  return (
    <div className="mt-3">
      <div className="flex items-baseline justify-between">
        <span className="nameplate text-2xs text-text-dim">Quota</span>
        <span className="tabular text-2xs text-text-dim">
          {percent(pct, 0)}
        </span>
      </div>
      <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-bg">
        <div
          className={`h-full rounded-full ${over ? "bg-ok" : "bg-data"}`}
          style={{ width: `${filled}%` }}
        />
      </div>
    </div>
  );
}

export function Tile({
  rank,
  row,
  basis,
  aggLeakagePct,
  openSignals,
  onClick,
}: {
  rank: number;
  row: TerritoryRow;
  basis: Basis;
  aggLeakagePct: number;
  openSignals: number;
  onClick: () => void;
}) {
  const revenue = basis === "gross" ? row.gross_cents : row.net_cents;
  const band = leakageBand(row.leakage_pct, aggLeakagePct);

  return (
    <button
      onClick={onClick}
      data-testid="territory-tile"
      data-tile-code={row.territory_code}
      className="flex flex-col rounded-lg border border-seam bg-surface p-3 text-left transition-colors hover:border-seam-strong focus:border-seam-strong focus:outline-none"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="nameplate-strong text-sm text-text">
            {row.territory_code}
          </div>
          <div className="truncate text-2xs text-text-dim">
            {row.territory_name}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Led
            band={band}
            title={`leakage ${percent(row.leakage_pct)} (book ${percent(aggLeakagePct)})`}
          />
          <span className="nameplate text-2xs text-text-dim">#{rank}</span>
        </div>
      </div>

      <div
        className="tabular mt-3 text-lg text-text"
        title={money(revenue)}
      >
        {moneyAbbrev(revenue)}
      </div>
      <div className="nameplate text-2xs text-text-dim">{basis} revenue</div>

      <AttainmentBar pct={row.quota_attainment_pct} />

      <div className="mt-2 flex items-baseline justify-between border-t border-seam/60 pt-2">
        <span className="nameplate text-2xs text-text-dim">Signals</span>
        <span
          className={`tabular text-2xs ${openSignals > 0 ? "text-warn" : "text-text-dim"}`}
          data-testid="tile-signals"
        >
          {openSignals}
        </span>
      </div>
    </button>
  );
}
