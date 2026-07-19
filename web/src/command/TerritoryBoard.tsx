// The Territory Board — PLENUM's signature. A fixed 4×2 cartogram for the
// full book (VP/admin), roughly geographic: northern tier over southern,
// west→east (architect resolution 9). Rep/RM scopes show only their tiles in
// a compact centered grid — a lone tile in an 8-slot map would read as broken.
// Both collapse 4→2→1 by available width, slot order preserved.

import { chain, descBasis, ascStr, ranked } from "../lib/rank";
import type { Basis } from "../lib/params";
import type { TerritoryRow } from "../lib/types";
import { Tile } from "./Tile";

// Row 1: Canada West, Canada East, Midwest, Northeast.
// Row 2: West, Mountain, South Central, Southeast.
const CARTOGRAM_SLOTS = [
  "CW-1",
  "CE-1",
  "MW-1",
  "NE-1",
  "W-1",
  "MT-1",
  "SC-1",
  "SE-1",
] as const;

function aggregateLeakagePct(rows: readonly TerritoryRow[]): number {
  const gross = rows.reduce((s, r) => s + r.gross_cents, 0);
  const net = rows.reduce((s, r) => s + r.net_cents, 0);
  return gross === 0 ? 0 : ((gross - net) / gross) * 100;
}

export function TerritoryBoard({
  rows,
  basis,
  onSelect,
}: {
  rows: TerritoryRow[];
  basis: Basis;
  onSelect: (code: string) => void;
}) {
  const agg = aggregateLeakagePct(rows);

  // Rank by chosen basis, tie-break on code — exactly the API's ORDER BY.
  const rankByCode = new Map<string, number>();
  for (const { rank, row } of ranked(
    rows,
    chain(descBasis<TerritoryRow>(basis), ascStr((r) => r.territory_code)),
  )) {
    rankByCode.set(row.territory_code, rank);
  }

  const full = rows.length === 8;

  if (full) {
    const byCode = new Map(rows.map((r) => [r.territory_code, r]));
    return (
      <div
        data-testid="territory-board"
        data-board-variant="cartogram"
        className="grid grid-cols-1 gap-3 min-[480px]:grid-cols-2 min-[900px]:grid-cols-4"
      >
        {CARTOGRAM_SLOTS.map((code) => {
          const row = byCode.get(code);
          if (!row) return null;
          return (
            <Tile
              key={code}
              rank={rankByCode.get(code) ?? 0}
              row={row}
              basis={basis}
              aggLeakagePct={agg}
              onClick={() => onSelect(code)}
            />
          );
        })}
      </div>
    );
  }

  // Compact scoped variant — ordered by rank, centered, collapses by width.
  const ordered = ranked(
    rows,
    chain(descBasis<TerritoryRow>(basis), ascStr((r) => r.territory_code)),
  );
  return (
    <div
      data-testid="territory-board"
      data-board-variant="compact"
      className="mx-auto grid max-w-3xl grid-cols-1 gap-3 min-[420px]:grid-cols-2 min-[680px]:grid-cols-3"
    >
      {ordered.map(({ rank, row }) => (
        <Tile
          key={row.territory_code}
          rank={rank}
          row={row}
          basis={basis}
          aggLeakagePct={agg}
          onClick={() => onSelect(row.territory_code)}
        />
      ))}
    </div>
  );
}
