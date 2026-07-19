// Client-side ranking. Every P2 dataset is fetched whole (limit=200 ≥ total),
// so we rank in the browser by the exact comparator the API's ORDER BY uses.
// Two payoffs: the GROSS/NET toggle re-ranks with ZERO refetch (no flash —
// gate P2-1), and the rank column always matches what the server would return.

import type { Basis } from "./params";

export interface Ranked<T> {
  rank: number;
  row: T;
}

export type Comparator<T> = (a: T, b: T) => number;

interface Dual {
  gross_cents: number;
  net_cents: number;
}

/** Chosen basis, descending — the primary sort key on every ranked surface. */
export function descBasis<T extends Dual>(basis: Basis): Comparator<T> {
  const field = basis === "gross" ? "gross_cents" : "net_cents";
  return (a, b) => b[field] - a[field];
}

export function ascStr<T>(get: (row: T) => string): Comparator<T> {
  return (a, b) => get(a).localeCompare(get(b));
}

/** Ascending number, nulls last (e.g. leakage_pct tie-break on the rep board). */
export function ascNumNullsLast<T>(get: (row: T) => number | null): Comparator<T> {
  return (a, b) => {
    const x = get(a);
    const y = get(b);
    if (x === null) return y === null ? 0 : 1;
    if (y === null) return -1;
    return x - y;
  };
}

/** Apply comparators in order; first non-zero decides. */
export function chain<T>(...cmps: Comparator<T>[]): Comparator<T> {
  return (a, b) => {
    for (const cmp of cmps) {
      const r = cmp(a, b);
      if (r !== 0) return r;
    }
    return 0;
  };
}

/** Sort a copy and attach 1-based ranks. */
export function ranked<T>(rows: readonly T[], compare: Comparator<T>): Ranked<T>[] {
  return [...rows]
    .sort(compare)
    .map((row, i) => ({ rank: i + 1, row }));
}
