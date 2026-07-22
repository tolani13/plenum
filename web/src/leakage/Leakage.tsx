// Leakage (spec §8 screen 3, ruling R1) — the app's second signature
// element. Three zones, all RLS-scoped:
//   1 · discount distribution — recharts bar over the period/kind slice
//       (the Ask chart's tokens-only pattern);
//   2 · outlier feed — `outliers=policy`: THE SAME math and config the
//       discount_anomaly generator runs, so every row lines up 1:1 with a
//       signal and carries its chip; rows land on the account 360;
//   3 · rep × family heat table — CSS grid, cells banded by leakage_pct
//       via the P5 heat tokens (LED-band logic extended; the alarm hue is
//       reserved for the worst band). Row order is leakage% DESC, so the
//       leakage rep reads worst at the top at VP view — the demo beat.
// Leakage takes NO basis parameter by metric definition (its whole point is
// gross vs net at once); the endpoint 422s one, and this screen never sends
// one.

import { useMemo, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import {
  Bar,
  BarChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { money, percent } from "../lib/format";
import { parseKind, parsePeriod, PERIOD_YEARS, QUARTERS } from "../lib/params";
import type { Kind } from "../lib/params";
import { useLeakage } from "../lib/queries";
import type { HeatCell } from "../lib/types";
import { useScreenReady } from "../lib/useScreenReady";
import { Segmented } from "../components/Segmented";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/states";

/** Heat band relative to the visible table's aggregate leakage — the P2 LED
 *  logic, widened to five steps. Band 4 is the only alarm-hued band. */
function heatBand(pct: number | null, agg: number): 0 | 1 | 2 | 3 | 4 {
  if (pct === null) return 0;
  if (pct <= agg - 3) return 0;
  if (pct <= agg) return 1;
  if (pct <= agg + 3) return 2;
  if (pct <= agg + 6) return 3;
  return 4;
}

const HEAT_BG: Record<number, string> = {
  0: "var(--color-heat-0)",
  1: "var(--color-heat-1)",
  2: "var(--color-heat-2)",
  3: "var(--color-heat-3)",
  4: "var(--color-heat-4)",
};

function ScrubBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      className={`nameplate px-2.5 py-1.5 text-2xs transition-colors ${
        active ? "bg-surface-2 text-text" : "text-text-dim hover:text-text"
      }`}
    >
      {children}
    </button>
  );
}

function ScrubGroup({ children }: { children: React.ReactNode }) {
  return (
    <div className="inline-flex overflow-hidden rounded border border-seam bg-surface">
      {children}
    </div>
  );
}

interface HeatMatrix {
  families: string[];
  rows: {
    rep: string;
    cells: Map<string, HeatCell>;
    gross: number;
    net: number;
    pct: number | null;
  }[];
  colTotals: Map<string, { gross: number; net: number }>;
  aggPct: number;
  totalGross: number;
  totalNet: number;
}

function buildMatrix(cells: HeatCell[]): HeatMatrix {
  const families = new Map<string, number>();
  const reps = new Map<string, { cells: Map<string, HeatCell>; gross: number; net: number }>();
  const colTotals = new Map<string, { gross: number; net: number }>();
  let totalGross = 0;
  let totalNet = 0;
  for (const c of cells) {
    families.set(c.family, (families.get(c.family) ?? 0) + c.gross_cents);
    const r = reps.get(c.rep_name) ?? { cells: new Map(), gross: 0, net: 0 };
    r.cells.set(c.family, c);
    r.gross += c.gross_cents;
    r.net += c.net_cents;
    reps.set(c.rep_name, r);
    const col = colTotals.get(c.family) ?? { gross: 0, net: 0 };
    col.gross += c.gross_cents;
    col.net += c.net_cents;
    colTotals.set(c.family, col);
    totalGross += c.gross_cents;
    totalNet += c.net_cents;
  }
  // Priority order: families by gross DESC (left-most = biggest business).
  const familyOrder = [...families.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([f]) => f);
  const rows = [...reps.entries()]
    .map(([rep, r]) => ({
      rep,
      cells: r.cells,
      gross: r.gross,
      net: r.net,
      pct: r.gross === 0 ? null : ((r.gross - r.net) / r.gross) * 100,
    }))
    .sort((a, b) => (b.pct ?? -1) - (a.pct ?? -1) || a.rep.localeCompare(b.rep));
  return {
    families: familyOrder,
    rows,
    colTotals,
    aggPct: totalGross === 0 ? 0 : ((totalGross - totalNet) / totalGross) * 100,
    totalGross,
    totalNet,
  };
}

export function Leakage() {
  const [params, setParams] = useSearchParams();
  const period = parsePeriod(params.get("period"), "cumulative");
  const kind = parseKind(params.get("kind"));
  const query = useLeakage(period, kind);
  const navigate = useNavigate();
  const [heatHover, setHeatHover] = useState<string | null>(null);

  useScreenReady(query.isSuccess || query.isError);

  const patch = (p: Record<string, string>) => {
    const sp = new URLSearchParams(params);
    for (const [k, v] of Object.entries(p)) sp.set(k, v);
    setParams(sp, { replace: true });
  };

  const matrix = useMemo(
    () => buildMatrix(query.data?.heat ?? []),
    [query.data],
  );

  const q = period.match(/^(\d{4})-q([1-4])$/);
  const activeYear = q
    ? Number(q[1])
    : /^\d{4}$/.test(period)
      ? Number(period)
      : PERIOD_YEARS[PERIOD_YEARS.length - 1];
  const activeQuarter = q ? Number(q[2]) : null;

  if (query.isLoading) return <LoadingPanel label="Loading leakage" />;
  if (query.isError)
    return (
      <div className="mx-auto max-w-[1600px]">
        <ErrorPanel onRetry={() => query.refetch()} />
      </div>
    );

  const data = query.data!;
  const dist = data.discount_distribution.map((b) => ({
    bucket: `${b.bucket}%`,
    lines: b.line_count,
  }));
  const distEmpty = data.discount_distribution.every((b) => b.line_count === 0);

  return (
    <div className="mx-auto max-w-[1700px] space-y-4">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="nameplate-strong text-xl text-text">Leakage</h1>
          <div className="nameplate text-2xs text-text-dim">
            gross − net, made visible · no basis toggle here — this screen IS
            both bases at once
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <ScrubGroup>
            {PERIOD_YEARS.map((y) => (
              <ScrubBtn
                key={y}
                active={/^\d{4}$/.test(period) && Number(period) === y}
                onClick={() => patch({ period: String(y) })}
              >
                {y}
              </ScrubBtn>
            ))}
          </ScrubGroup>
          <ScrubGroup>
            {QUARTERS.map((n) => (
              <ScrubBtn
                key={n}
                active={activeQuarter === n}
                onClick={() => patch({ period: `${activeYear}-q${n}` })}
              >
                Q{n}
              </ScrubBtn>
            ))}
          </ScrubGroup>
          <ScrubGroup>
            <ScrubBtn
              active={period === "cumulative"}
              onClick={() => patch({ period: "cumulative" })}
            >
              Cum
            </ScrubBtn>
            <ScrubBtn
              active={period === "ttm"}
              onClick={() => patch({ period: "ttm" })}
            >
              TTM
            </ScrubBtn>
          </ScrubGroup>
          <Segmented<Kind>
            ariaLabel="Kind"
            testid="leakage-kind"
            value={kind}
            onChange={(k) => patch({ kind: k })}
            options={[
              { value: "all", label: "All" },
              { value: "capital", label: "Capital" },
              { value: "consumable", label: "Consumable" },
            ]}
          />
        </div>
      </header>

      <div className="grid grid-cols-1 gap-4 min-[1100px]:grid-cols-[minmax(0,3fr)_minmax(0,4fr)]">
        {/* ── 1 · discount distribution ── */}
        <section
          className="rounded-lg border border-seam bg-surface p-4"
          data-testid="leakage-distribution"
        >
          <div className="nameplate mb-2 text-2xs text-text-dim">
            Discount distribution · line count by discount band
          </div>
          {distEmpty ? (
            <EmptyPanel message="No order lines in scope for this period." />
          ) : (
            <div className="h-56 w-full">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={dist} margin={{ left: 8, right: 8 }}>
                  <XAxis
                    dataKey="bucket"
                    tick={{ fill: "var(--color-text-dim)", fontSize: 10 }}
                    stroke="var(--color-seam)"
                    interval={0}
                  />
                  <YAxis
                    tick={{ fill: "var(--color-text-dim)", fontSize: 10 }}
                    stroke="var(--color-seam)"
                    width={56}
                    tickFormatter={(v: number) => v.toLocaleString("en-US")}
                  />
                  <Tooltip
                    cursor={{ fill: "var(--color-surface-2)" }}
                    contentStyle={{
                      background: "var(--color-surface)",
                      border: "1px solid var(--color-seam)",
                      borderRadius: 4,
                      color: "var(--color-text)",
                      fontSize: 12,
                    }}
                    formatter={(v) => [
                      `${Number(v).toLocaleString("en-US")} lines`,
                    ]}
                  />
                  <Bar dataKey="lines" fill="var(--color-data)" />
                </BarChart>
              </ResponsiveContainer>
            </div>
          )}
        </section>

        {/* ── 2 · outlier feed ── */}
        <section className="flex min-w-0 flex-col rounded-lg border border-seam bg-surface">
          <div className="flex flex-wrap items-baseline justify-between gap-2 border-b border-seam px-4 py-2.5">
            <span className="nameplate text-2xs text-text-dim">
              Outlier feed · the discount-anomaly window, live
            </span>
            <span className="text-2xs text-text-dim">
              same math + config as the signal generator — rows and chips
              agree
            </span>
          </div>
          {data.outliers.length === 0 ? (
            <div className="p-4">
              <EmptyPanel message="No lines beat the policy threshold in the trailing window — the book is holding its prices." />
            </div>
          ) : (
            <div className="scroll-x max-h-[19rem] overflow-y-auto">
              <table className="w-full text-sm">
                <thead className="sticky top-0 bg-surface">
                  <tr className="border-b border-seam">
                    {["Date", "Account", "Rep", "Line", "Disc.", "Gross", ""].map(
                      (h, i) => (
                        <th
                          key={i}
                          className={`nameplate whitespace-nowrap px-3 py-2 text-2xs text-text-dim ${
                            i === 4 || i === 5 ? "text-right" : "text-left"
                          }`}
                        >
                          {h}
                        </th>
                      ),
                    )}
                  </tr>
                </thead>
                <tbody>
                  {data.outliers.map((o) => (
                    <tr
                      key={o.order_line_id}
                      data-testid="outlier-row"
                      onClick={() => navigate(`/accounts/${o.account_id}`)}
                      className="cursor-pointer border-b border-seam/40 last:border-0 hover:bg-surface-2/50"
                      title={`family median ${percent(o.family_median_pct)} · threshold ${percent(o.threshold_pct)}`}
                    >
                      <td className="tabular whitespace-nowrap px-3 py-1.5 text-text-dim">
                        {o.ordered_on}
                      </td>
                      <td className="max-w-[11rem] truncate px-3 py-1.5 text-text">
                        {o.account_name}
                      </td>
                      <td className="max-w-[8rem] truncate px-3 py-1.5 text-text-dim">
                        {o.rep_name}
                      </td>
                      <td className="whitespace-nowrap px-3 py-1.5">
                        <span className="nameplate text-2xs text-text-dim">
                          {o.product_sku} × {o.qty}
                        </span>
                      </td>
                      <td className="tabular px-3 py-1.5 text-right text-warn">
                        {percent(o.discount_pct)}
                      </td>
                      <td className="tabular px-3 py-1.5 text-right text-text">
                        {money(o.list_unit_cents * o.qty)}
                      </td>
                      <td className="px-3 py-1.5 text-right">
                        {o.signal_id && (
                          <Link
                            to="/signals"
                            onClick={(e) => e.stopPropagation()}
                            data-testid="outlier-signal-chip"
                            className="nameplate inline-block rounded-sm border border-warn/50 px-1.5 py-0.5 text-2xs text-warn hover:bg-surface-2"
                            title={`discount_anomaly signal · ${o.signal_status ?? ""}`}
                          >
                            signal
                          </Link>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </div>

      {/* ── 3 · rep × family heat table — the signature ── */}
      <section className="rounded-lg border border-seam bg-surface">
        <div className="flex flex-wrap items-baseline justify-between gap-2 border-b border-seam px-4 py-2.5">
          <span className="nameplate text-2xs text-text-dim">
            Rep × family heat · leakage % per cell · worst rows first
          </span>
          <span className="tabular text-2xs text-text-dim">
            book {percent(matrix.aggPct)} · leakage{" "}
            {money(matrix.totalGross - matrix.totalNet)}
          </span>
        </div>
        {matrix.rows.length === 0 ? (
          <div className="p-4">
            <EmptyPanel message="No order lines in scope for this period." />
          </div>
        ) : (
          <div className="scroll-x p-3" data-testid="heat-table">
            <div
              className="grid gap-px"
              style={{
                gridTemplateColumns: `minmax(8.5rem, 12rem) repeat(${matrix.families.length}, minmax(4.6rem, 1fr)) minmax(6.5rem, 8rem)`,
                minWidth: `${8.5 + matrix.families.length * 4.6 + 6.5}rem`,
              }}
            >
              {/* header row */}
              <div className="nameplate px-2 py-1.5 text-2xs text-text-dim">
                Rep × family
              </div>
              {matrix.families.map((f) => (
                <div
                  key={f}
                  className="nameplate truncate px-2 py-1.5 text-right text-2xs text-text-dim"
                  title={f}
                >
                  {f.replace("Filters-", "F·")}
                </div>
              ))}
              <div className="nameplate border-l border-seam-strong px-2 py-1.5 text-right text-2xs text-text">
                Row total
              </div>

              {/* rep rows */}
              {matrix.rows.map((r) => (
                <HeatRowCells
                  key={r.rep}
                  row={r}
                  families={matrix.families}
                  aggPct={matrix.aggPct}
                  hovered={heatHover}
                  onHover={setHeatHover}
                />
              ))}

              {/* column totals rail */}
              <div className="nameplate border-t border-seam-strong px-2 py-1.5 text-2xs text-text">
                Family total
              </div>
              {matrix.families.map((f) => {
                const col = matrix.colTotals.get(f);
                const pct =
                  col && col.gross > 0
                    ? ((col.gross - col.net) / col.gross) * 100
                    : null;
                return (
                  <div
                    key={f}
                    className="tabular border-t border-seam-strong px-2 py-1.5 text-right text-2xs text-text-dim"
                    title={
                      col
                        ? `gross ${money(col.gross)} · net ${money(col.net)} · leakage ${money(col.gross - col.net)}`
                        : undefined
                    }
                  >
                    {percent(pct)}
                  </div>
                );
              })}
              <div
                className="tabular border-l border-t border-seam-strong px-2 py-1.5 text-right text-2xs text-text"
                title={`gross ${money(matrix.totalGross)} · net ${money(matrix.totalNet)}`}
              >
                {percent(matrix.aggPct)}
              </div>
            </div>
            <div className="mt-2 flex flex-wrap items-center gap-3 px-1 text-2xs text-text-dim">
              <span className="nameplate">bands</span>
              {[0, 1, 2, 3, 4].map((b) => (
                <span key={b} className="flex items-center gap-1">
                  <span
                    className="inline-block h-2.5 w-2.5 rounded-sm border border-seam"
                    style={{ background: HEAT_BG[b] }}
                  />
                  {b === 0
                    ? "≤ book −3"
                    : b === 1
                      ? "at book"
                      : b === 2
                        ? "+3"
                        : b === 3
                          ? "+6"
                          : "worst"}
                </span>
              ))}
              <span className="ml-auto">
                hover a cell for exact gross / net / leakage
              </span>
            </div>
          </div>
        )}
      </section>

    </div>
  );
}

function HeatRowCells({
  row,
  families,
  aggPct,
  hovered,
  onHover,
}: {
  row: HeatMatrix["rows"][number];
  families: string[];
  aggPct: number;
  hovered: string | null;
  onHover: (key: string | null) => void;
}) {
  return (
    <>
      <div
        className="truncate px-2 py-1.5 text-sm text-text"
        data-testid="heat-rep"
        data-rep={row.rep}
        title={`gross ${money(row.gross)} · net ${money(row.net)} · leakage ${money(row.gross - row.net)}`}
      >
        {row.rep}
      </div>
      {families.map((f) => {
        const cell = row.cells.get(f);
        const pct =
          cell && cell.gross_cents > 0
            ? ((cell.gross_cents - cell.net_cents) / cell.gross_cents) * 100
            : null;
        const band = cell ? heatBand(pct, aggPct) : 0;
        const key = `${row.rep}·${f}`;
        return (
          <div
            key={f}
            data-testid="heat-cell"
            data-rep={row.rep}
            className={`tabular px-2 py-1.5 text-right text-2xs transition-[filter] duration-150 ${
              pct === null ? "text-text-dim/50" : "text-text"
            }`}
            style={{
              background: cell ? HEAT_BG[band] : "var(--color-heat-0)",
              filter: hovered === key ? "brightness(1.3)" : undefined,
            }}
            onMouseEnter={() => onHover(key)}
            onMouseLeave={() => onHover(null)}
            title={
              cell
                ? `${row.rep} · ${f}\ngross ${money(cell.gross_cents)} · net ${money(cell.net_cents)} · leakage ${money(cell.gross_cents - cell.net_cents)}`
                : `${row.rep} · ${f} — no lines`
            }
          >
            {pct === null ? "—" : percent(pct)}
          </div>
        );
      })}
      <div
        className="tabular border-l border-seam-strong px-2 py-1.5 text-right text-2xs text-text"
        title={`gross ${money(row.gross)} · net ${money(row.net)} · leakage ${money(row.gross - row.net)}`}
      >
        {percent(row.pct)}
        <span className="ml-1.5 text-text-dim">
          {money(row.gross - row.net)}
        </span>
      </div>
    </>
  );
}
