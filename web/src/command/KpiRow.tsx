// The four top-line KPIs. Which figures flip with the basis toggle and which
// hold is fixed by metric definition (architect resolution 2):
//   1. NET|GROSS YTD   — value + label FLIP with basis.
//   2. LEAKAGE %       — Σleakage/Σgross; ONE formula, basis-invariant.
//   3. COVERAGE %      — units-due-weighted count ratio, invariant; but its
//                        projected-$ sub-line is money and FLIPS with basis.
//   4. OPEN SIGNALS    — P4 (R7): the live active-signal count from
//                        /api/signals/summary; basis-invariant by definition
//                        (counts, not dollars). Sub-line = the by-type digest.

import { money, percent, count } from "../lib/format";
import type { Basis } from "../lib/params";
import type { CoverageRow, SignalsSummary, TerritoryRow } from "../lib/types";

function Kpi({
  label,
  value,
  sub,
  testid,
  tone = "text-text",
}: {
  label: string;
  value: string;
  sub?: string;
  testid: string;
  tone?: string;
}) {
  return (
    <div className="rounded-lg border border-seam bg-surface p-4" data-testid={testid}>
      <div className="nameplate text-2xs text-text-dim">{label}</div>
      <div
        className={`tabular mt-1 truncate text-xl ${tone}`}
        title={value}
        data-testid={`${testid}-value`}
      >
        {value}
      </div>
      <div className="mt-1 h-4 truncate text-2xs text-text-dim">{sub ?? ""}</div>
    </div>
  );
}

export function KpiRow({
  territories,
  basis,
  coverageRows,
  summary,
}: {
  territories: TerritoryRow[];
  basis: Basis;
  coverageRows: CoverageRow[] | undefined;
  summary: SignalsSummary | undefined;
}) {
  // KPI 1 + 2 — from the territory rows.
  const grossSum = territories.reduce((s, r) => s + r.gross_cents, 0);
  const netSum = territories.reduce((s, r) => s + r.net_cents, 0);
  const ytd = basis === "gross" ? grossSum : netSum;
  const leakagePct = grossSum === 0 ? null : ((grossSum - netSum) / grossSum) * 100;

  // KPI 3 — units-due-weighted coverage + projected consumable revenue (basis).
  let coverageValue = "—";
  let coverageSub: string | undefined;
  if (coverageRows) {
    const due = coverageRows.reduce((s, r) => s + r.units_due, 0);
    const covered = coverageRows.reduce(
      (s, r) => s + (r.pct_covered === null ? 0 : (r.units_due * r.pct_covered) / 100),
      0,
    );
    const projected = coverageRows.reduce(
      (s, r) =>
        s +
        (basis === "gross"
          ? r.projected_consumable_gross_cents
          : r.projected_consumable_net_cents),
      0,
    );
    coverageValue = due === 0 ? "—" : percent((covered / due) * 100);
    coverageSub = `${count(due)} due · ${money(projected)} proj.`;
  }

  // KPI 4 — open signals (open ∪ assigned) with the by-type digest.
  let signalsValue = "—";
  let signalsSub: string | undefined;
  if (summary) {
    signalsValue = count(summary.total);
    const b = summary.by_type;
    signalsSub = `${b.reorder_due} reorder · ${b.defection_risk} defection · ${b.conquest} conquest · ${b.discount_anomaly} anomaly`;
  }

  return (
    <div className="grid grid-cols-1 gap-3 min-[560px]:grid-cols-2 min-[1120px]:grid-cols-4">
      <Kpi
        testid="kpi-ytd"
        label={`${basis === "gross" ? "Gross" : "Net"} YTD`}
        value={money(ytd)}
      />
      <Kpi
        testid="kpi-leakage"
        label="Leakage %"
        value={percent(leakagePct)}
        sub="of gross"
      />
      <Kpi
        testid="kpi-coverage"
        label="Coverage %"
        value={coverageValue}
        sub={coverageSub}
      />
      <Kpi
        testid="kpi-signals"
        label="Open Signals"
        value={signalsValue}
        sub={signalsSub}
        tone="text-warn"
      />
    </div>
  );
}
