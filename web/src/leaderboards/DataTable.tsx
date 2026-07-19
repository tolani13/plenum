// Instrument-grade table: TanStack Table for interactive column sorting, with
// element-contained horizontal scroll as the LAST-resort overflow response
// (the columns priority-hide by width first). The rank column always shows the
// server's rank (resolution 7) — user sorting reorders the display but never
// re-defines the ranking. A footer totals row is the anchor surface.

import { useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { ColumnDef, RowData, SortingState } from "@tanstack/react-table";
import { ChevronDown, ChevronUp } from "lucide-react";

declare module "@tanstack/react-table" {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  interface ColumnMeta<TData extends RowData, TValue> {
    className?: string;
    align?: "left" | "right";
  }
}

export function DataTable<T>({
  columns,
  data,
  footer,
  initialSort,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  columns: ColumnDef<T, any>[];
  data: T[];
  footer?: React.ReactNode[];
  initialSort?: SortingState;
}) {
  const [sorting, setSorting] = useState<SortingState>(initialSort ?? []);
  const table = useReactTable({
    data,
    columns,
    state: { sorting },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  const cellAlign = (align?: "left" | "right") =>
    align === "right" ? "text-right tabular" : "text-left";

  return (
    <div className="scroll-x rounded-lg border border-seam bg-surface">
      <table className="w-full text-sm">
        <thead className="border-b border-seam">
          {table.getHeaderGroups().map((hg) => (
            <tr key={hg.id}>
              {hg.headers.map((h) => {
                const meta = h.column.columnDef.meta;
                const sortable = h.column.getCanSort();
                const sorted = h.column.getIsSorted();
                return (
                  <th
                    key={h.id}
                    className={`nameplate px-3 py-2 text-2xs text-text-dim ${cellAlign(
                      meta?.align,
                    )} ${meta?.className ?? ""} ${
                      sortable ? "cursor-pointer select-none" : ""
                    }`}
                    onClick={h.column.getToggleSortingHandler()}
                  >
                    <span
                      className={`inline-flex items-center gap-1 ${
                        meta?.align === "right" ? "flex-row-reverse" : ""
                      }`}
                    >
                      {flexRender(h.column.columnDef.header, h.getContext())}
                      {sorted === "asc" && <ChevronUp size={11} />}
                      {sorted === "desc" && <ChevronDown size={11} />}
                    </span>
                  </th>
                );
              })}
            </tr>
          ))}
        </thead>

        <tbody>
          {table.getRowModel().rows.map((r) => (
            <tr
              key={r.id}
              className="border-b border-seam/40 last:border-0 hover:bg-surface-2/50"
            >
              {r.getVisibleCells().map((c) => {
                const meta = c.column.columnDef.meta;
                return (
                  <td
                    key={c.id}
                    className={`px-3 py-2 text-text ${cellAlign(meta?.align)} ${
                      meta?.className ?? ""
                    }`}
                  >
                    {flexRender(c.column.columnDef.cell, c.getContext())}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>

        {footer && (
          <tfoot className="border-t border-seam bg-surface-2/40">
            <tr data-testid="table-footer">
              {table.getVisibleFlatColumns().map((col, i) => {
                const meta = col.columnDef.meta;
                return (
                  <td
                    key={col.id}
                    className={`nameplate px-3 py-2 text-2xs text-text ${cellAlign(
                      meta?.align,
                    )} ${meta?.className ?? ""}`}
                  >
                    {footer[i]}
                  </td>
                );
              })}
            </tr>
          </tfoot>
        )}
      </table>
    </div>
  );
}
