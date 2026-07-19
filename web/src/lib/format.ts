// Money + number formatting. Money arrives as BIGINT cents (integer) from the
// API and is displayed EXACT to the penny everywhere a full figure shows;
// tiles may abbreviate ($2.47M) with the exact figure carried in `title`.
// 2467089087 cents fits Number safely (< 2^53), so plain division is exact.

const USD = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const INT = new Intl.NumberFormat("en-US");

/** cents → "$24,670,890.87" (exact, 2 decimals). */
export function money(cents: number): string {
  return USD.format(cents / 100);
}

/** cents → "$2.47M" / "$34.5K" / "$980" — abbreviated for dense tiles. */
export function moneyAbbrev(cents: number): string {
  const d = cents / 100;
  const abs = Math.abs(d);
  const sign = d < 0 ? "-" : "";
  if (abs >= 1e9) return `${sign}$${(abs / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `${sign}$${(abs / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${sign}$${(abs / 1e3).toFixed(1)}K`;
  return `${sign}$${abs.toFixed(0)}`;
}

/** A ratio/percent number → "14.3%"; null/NaN → the mist em-dash. */
export function percent(pct: number | null | undefined, digits = 1): string {
  if (pct === null || pct === undefined || Number.isNaN(pct)) return "—";
  return `${pct.toFixed(digits)}%`;
}

/** Integer with thousands separators. */
export function count(n: number): string {
  return INT.format(n);
}
