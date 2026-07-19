// Client-side CSV export (architect resolution 8) from the currently loaded,
// filtered, RLS-scoped view. It exports the FULL row fields (both bases,
// regardless of which columns are responsively hidden on screen) — but only
// the rows the caller's session actually returned, so a rep's file can never
// contain an account her screen doesn't show. Excel-safe: UTF-8 BOM + CRLF,
// every field quoted.

function quote(value: string | number | null | undefined): string {
  const s = value === null || value === undefined ? "" : String(value);
  return `"${s.replace(/"/g, '""')}"`;
}

/**
 * Build + download a CSV. `columns` are [header, accessor] pairs applied to
 * each row in `rows`; `filename` follows plenum-{tab}-{period}-{basis}[...].csv.
 */
export function downloadCsv<T>(
  filename: string,
  columns: ReadonlyArray<readonly [string, (row: T) => string | number | null]>,
  rows: readonly T[],
): void {
  const header = columns.map(([h]) => quote(h)).join(",");
  const body = rows
    .map((row) => columns.map(([, get]) => quote(get(row))).join(","))
    .join("\r\n");
  const csv = "﻿" + header + "\r\n" + body + (body ? "\r\n" : "");

  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/** filename per resolution 8: plenum-{tab}-{period}-{basis}[-{kind}][-{group}].csv */
export function csvFilename(
  tab: string,
  period: string,
  basis: string,
  kind?: string,
  group?: string,
): string {
  const parts = ["plenum", tab, period, basis];
  if (kind && kind !== "all") parts.push(kind);
  if (group) parts.push(group);
  return parts.join("-") + ".csv";
}
