// Column definitions, footer totals, and CSV field maps for the three
// leaderboard tabs. Columns priority-hide by width (resolution 11): rank +
// name + the chosen-basis revenue + leakage% survive the narrowest screens;
// the other basis, the capital/consumable split, and tab extras drop first.
// The on-screen split columns show the chosen basis; the CSV always carries
// every field, both bases (resolution 8).

import type { ColumnDef } from "@tanstack/react-table";
import { money, percent, count } from "../lib/format";
import type { Basis, Group } from "../lib/params";
import type { CustomerRow, ItemRow, LeaderboardRow } from "../lib/types";

export type RepX = LeaderboardRow & { _rank: number };
export type ItemX = ItemRow & { _rank: number };
export type CustomerX = CustomerRow & { _rank: number };

const usd = (cents: number): string => (cents / 100).toFixed(2);

function rankCell(v: number) {
  return <span className="nameplate text-text-dim">#{v}</span>;
}

// A revenue column that only survives narrow widths when it is the chosen basis.
function basisHide(isChosen: boolean): string {
  return isChosen ? "" : "hidden min-[880px]:table-cell";
}

/* ------------------------------------------------------------------ reps -- */

export function repColumns(basis: Basis): ColumnDef<RepX, unknown>[] {
  return [
    {
      accessorKey: "_rank",
      header: "#",
      cell: (c) => rankCell(c.getValue<number>()),
    },
    {
      accessorKey: "rep_name",
      header: "Rep",
      cell: (c) => <span className="text-text">{c.getValue<string>()}</span>,
    },
    {
      accessorKey: "gross_cents",
      header: "Gross",
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: basisHide(basis === "gross") },
    },
    {
      accessorKey: "net_cents",
      header: "Net",
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: basisHide(basis === "net") },
    },
    {
      accessorKey: "leakage_pct",
      header: "Leakage %",
      cell: (c) => percent(c.getValue<number | null>()),
      meta: { align: "right" },
    },
    {
      id: "capital",
      header: "Capital",
      accessorFn: (r) =>
        basis === "gross" ? r.capital_gross_cents : r.capital_net_cents,
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: "hidden min-[1120px]:table-cell" },
    },
    {
      id: "consumable",
      header: "Consumable",
      accessorFn: (r) =>
        basis === "gross" ? r.consumable_gross_cents : r.consumable_net_cents,
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: "hidden min-[1120px]:table-cell" },
    },
    {
      accessorKey: "top_account_name",
      header: "Top account",
      cell: (c) => (
        <span className="text-text-dim">{c.getValue<string | null>() ?? "—"}</span>
      ),
      meta: { className: "hidden min-[700px]:table-cell" },
    },
  ];
}

export function repFooter(rows: RepX[], basis: Basis): React.ReactNode[] {
  const g = rows.reduce((s, r) => s + r.gross_cents, 0);
  const n = rows.reduce((s, r) => s + r.net_cents, 0);
  const cap = rows.reduce(
    (s, r) => s + (basis === "gross" ? r.capital_gross_cents : r.capital_net_cents),
    0,
  );
  const cons = rows.reduce(
    (s, r) =>
      s + (basis === "gross" ? r.consumable_gross_cents : r.consumable_net_cents),
    0,
  );
  return [
    "",
    "Total",
    money(g),
    money(n),
    percent(g === 0 ? null : ((g - n) / g) * 100),
    money(cap),
    money(cons),
    "",
  ];
}

export const repCsv: ReadonlyArray<readonly [string, (r: RepX) => string | number]> = [
  ["Rank", (r) => r._rank],
  ["Rep", (r) => r.rep_name],
  ["Gross", (r) => usd(r.gross_cents)],
  ["Net", (r) => usd(r.net_cents)],
  ["Leakage %", (r) => (r.leakage_pct ?? "")],
  ["Capital (gross)", (r) => usd(r.capital_gross_cents)],
  ["Capital (net)", (r) => usd(r.capital_net_cents)],
  ["Consumable (gross)", (r) => usd(r.consumable_gross_cents)],
  ["Consumable (net)", (r) => usd(r.consumable_net_cents)],
  ["Top account", (r) => r.top_account_name ?? ""],
];

/* ------------------------------------------------------------- customers -- */

export function customerColumns(basis: Basis): ColumnDef<CustomerX, unknown>[] {
  return [
    {
      accessorKey: "_rank",
      header: "#",
      cell: (c) => rankCell(c.getValue<number>()),
    },
    {
      accessorKey: "account_name",
      header: "Account",
      cell: (c) => <span className="text-text">{c.getValue<string>()}</span>,
    },
    {
      accessorKey: "gross_cents",
      header: "Gross",
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: basisHide(basis === "gross") },
    },
    {
      accessorKey: "net_cents",
      header: "Net",
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: basisHide(basis === "net") },
    },
    {
      accessorKey: "leakage_pct",
      header: "Leakage %",
      cell: (c) => percent(c.getValue<number | null>()),
      meta: { align: "right" },
    },
    {
      id: "capital",
      header: "Capital",
      accessorFn: (r) =>
        basis === "gross" ? r.capital_gross_cents : r.capital_net_cents,
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: "hidden min-[900px]:table-cell" },
    },
    {
      id: "consumable",
      header: "Consumable",
      accessorFn: (r) =>
        basis === "gross" ? r.consumable_gross_cents : r.consumable_net_cents,
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: "hidden min-[900px]:table-cell" },
    },
  ];
}

export function customerFooter(rows: CustomerX[], basis: Basis): React.ReactNode[] {
  const g = rows.reduce((s, r) => s + r.gross_cents, 0);
  const n = rows.reduce((s, r) => s + r.net_cents, 0);
  const cap = rows.reduce(
    (s, r) => s + (basis === "gross" ? r.capital_gross_cents : r.capital_net_cents),
    0,
  );
  const cons = rows.reduce(
    (s, r) =>
      s + (basis === "gross" ? r.consumable_gross_cents : r.consumable_net_cents),
    0,
  );
  return [
    "",
    "Total",
    money(g),
    money(n),
    percent(g === 0 ? null : ((g - n) / g) * 100),
    money(cap),
    money(cons),
  ];
}

export const customerCsv: ReadonlyArray<
  readonly [string, (r: CustomerX) => string | number]
> = [
  ["Rank", (r) => r._rank],
  ["Account", (r) => r.account_name],
  ["Gross", (r) => usd(r.gross_cents)],
  ["Net", (r) => usd(r.net_cents)],
  ["Leakage %", (r) => (r.leakage_pct ?? "")],
  ["Capital (gross)", (r) => usd(r.capital_gross_cents)],
  ["Capital (net)", (r) => usd(r.capital_net_cents)],
  ["Consumable (gross)", (r) => usd(r.consumable_gross_cents)],
  ["Consumable (net)", (r) => usd(r.consumable_net_cents)],
];

/* ----------------------------------------------------------------- items -- */

export function itemColumns(basis: Basis, group: Group): ColumnDef<ItemX, unknown>[] {
  const cols: ColumnDef<ItemX, unknown>[] = [
    {
      accessorKey: "_rank",
      header: "#",
      cell: (c) => rankCell(c.getValue<number>()),
    },
    {
      accessorKey: "product_name",
      header: group === "family" ? "Family" : "Product",
      cell: (c) => {
        const row = c.row.original;
        return (
          <div className="min-w-0">
            <div className="truncate text-text">{row.product_name}</div>
            {group === "product" && row.product_sku && (
              <div className="nameplate text-2xs text-text-dim">
                {row.product_sku}
              </div>
            )}
          </div>
        );
      },
    },
  ];

  if (group === "product") {
    cols.push({
      accessorKey: "family",
      header: "Family",
      cell: (c) => (
        <span className="text-text-dim">{c.getValue<string>()}</span>
      ),
      meta: { className: "hidden min-[820px]:table-cell" },
    });
  }

  cols.push(
    {
      accessorKey: "kind",
      header: "Kind",
      cell: (c) => (
        <span className="nameplate text-2xs text-text-dim">
          {c.getValue<string>()}
        </span>
      ),
      meta: { className: "hidden min-[620px]:table-cell" },
    },
    {
      accessorKey: "units",
      header: "Units",
      cell: (c) => count(c.getValue<number>()),
      meta: { align: "right" },
    },
    {
      accessorKey: "gross_cents",
      header: "Gross",
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: basisHide(basis === "gross") },
    },
    {
      accessorKey: "net_cents",
      header: "Net",
      cell: (c) => money(c.getValue<number>()),
      meta: { align: "right", className: basisHide(basis === "net") },
    },
    {
      accessorKey: "attach_rate_pct",
      header: "Attach %",
      cell: (c) => percent(c.getValue<number | null>()),
      meta: { align: "right", className: "hidden min-[720px]:table-cell" },
    },
  );

  return cols;
}

export function itemFooter(rows: ItemX[], group: Group): React.ReactNode[] {
  const units = rows.reduce((s, r) => s + r.units, 0);
  const g = rows.reduce((s, r) => s + r.gross_cents, 0);
  const n = rows.reduce((s, r) => s + r.net_cents, 0);
  const head: React.ReactNode[] = group === "product" ? ["", "Total", ""] : ["", "Total"];
  return [...head, "", count(units), money(g), money(n), ""];
}

export const itemCsv: ReadonlyArray<readonly [string, (r: ItemX) => string | number]> =
  [
    ["Rank", (r) => r._rank],
    ["SKU", (r) => r.product_sku ?? ""],
    ["Name", (r) => r.product_name],
    ["Family", (r) => r.family],
    ["Kind", (r) => r.kind],
    ["Units", (r) => r.units],
    ["Gross", (r) => usd(r.gross_cents)],
    ["Net", (r) => usd(r.net_cents)],
    ["Attach %", (r) => (r.attach_rate_pct ?? "")],
  ];
