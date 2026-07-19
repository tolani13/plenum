// Controls state lives in the URL (architect resolution 6): refresh-safe,
// deep-linkable for the demo script. This is the ONE module that parses and
// validates ?basis=&period=&kind=&tab=&group= — every screen reads through it,
// so an out-of-grammar value can never reach the API as a surprise 422.
//
// The period grammar mirrors crates/domain/src/period.rs exactly:
//   YYYY (1970–2100) | YYYY-Qn (n=1..4) | cumulative | ttm.

export type Basis = "gross" | "net";
export type Kind = "all" | "capital" | "consumable";
export type Tab = "reps" | "items" | "customers";
export type Group = "product" | "family";

/** Years the seeded order book spans (2023 → 2026). Drives the period rail. */
export const PERIOD_YEARS = [2023, 2024, 2025, 2026] as const;
export const QUARTERS = [1, 2, 3, 4] as const;

/** Command is fixed to the current calendar year, YTD (resolution 6; no period
 *  control on Command per spec §8). The browser clock reads 2026 for the demo. */
export const CURRENT_YEAR = new Date().getFullYear();
export const COMMAND_PERIOD = String(CURRENT_YEAR);

const YEAR_MIN = 1970;
const YEAR_MAX = 2100;

export function isValidPeriod(raw: string): boolean {
  const s = raw.trim().toLowerCase();
  if (s === "cumulative" || s === "ttm") return true;
  const q = s.match(/^(\d{4})-q([1-4])$/);
  if (q) {
    const y = Number(q[1]);
    return y >= YEAR_MIN && y <= YEAR_MAX;
  }
  if (/^\d{4}$/.test(s)) {
    const y = Number(s);
    return y >= YEAR_MIN && y <= YEAR_MAX;
  }
  return false;
}

export function parsePeriod(raw: string | null, fallback: string): string {
  if (raw && isValidPeriod(raw)) return raw.trim().toLowerCase();
  return fallback;
}

export function parseBasis(raw: string | null, fallback: Basis = "net"): Basis {
  return raw === "gross" || raw === "net" ? raw : fallback;
}

export function parseKind(raw: string | null, fallback: Kind = "all"): Kind {
  return raw === "all" || raw === "capital" || raw === "consumable"
    ? raw
    : fallback;
}

export function parseTab(raw: string | null, fallback: Tab = "reps"): Tab {
  return raw === "reps" || raw === "items" || raw === "customers"
    ? raw
    : fallback;
}

export function parseGroup(raw: string | null, fallback: Group = "product"): Group {
  return raw === "product" || raw === "family" ? raw : fallback;
}

/** Human label for a period token, for scrubber state and headings. */
export function periodLabel(period: string): string {
  if (period === "cumulative") return "Cumulative";
  if (period === "ttm") return "TTM";
  const q = period.match(/^(\d{4})-q([1-4])$/);
  if (q) return `${q[1]} Q${q[2]}`;
  return period;
}
