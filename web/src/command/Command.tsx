// Command — the signature screen. Territory Board + four KPIs + the GROSS/NET
// toggle. Period is fixed to current-year YTD (spec §8; Command has no period
// control). The toggle only changes a URL display choice: the payload is
// dual-basis and the query key omits basis, so flipping re-renders in place —
// every dollar figure + KPI flips, no refetch, no flash (gate P2-1).
//
// P4 rewire (R7): the 4th KPI reads OPEN SIGNALS from /api/signals/summary
// (the defection-risk stand-in retires); each Territory Board tile gains its
// open-signal count, matched by territory code (the summary carries both id
// and code; the metrics payload exposes code). Signal counts are
// basis-invariant by definition — the flip moves money, never the radar.

import { useState } from "react";
import { useSearchParams } from "react-router";
import { COMMAND_PERIOD, parseBasis, periodLabel } from "../lib/params";
import { useTerritories, useCoverage } from "../lib/queries";
import { useSignalsSummary } from "../lib/signals";
import { useScreenReady } from "../lib/useScreenReady";
import { BasisToggle } from "../components/BasisToggle";
import { ErrorPanel, LoadingPanel } from "../components/states";
import { KpiRow } from "./KpiRow";
import { TerritoryBoard } from "./TerritoryBoard";
import { DrillDrawer } from "./DrillDrawer";

export function Command() {
  const [params, setParams] = useSearchParams();
  const basis = parseBasis(params.get("basis"));
  const [selected, setSelected] = useState<string | null>(null);

  const territories = useTerritories(COMMAND_PERIOD);
  const coverage = useCoverage();
  const summary = useSignalsSummary();

  const settled = [territories, coverage, summary].every(
    (query) => query.isSuccess || query.isError,
  );
  useScreenReady(settled);

  const setBasis = (next: string) => {
    const p = new URLSearchParams(params);
    p.set("basis", next);
    setParams(p, { replace: true });
  };

  const rows = territories.data?.items ?? [];
  const signalCounts = new Map<string, number>(
    (summary.data?.territories ?? []).map((t) => [t.territory_code, t.open_count]),
  );

  return (
    // R8 (P5): at generous viewport heights the KPI row + Territory Board
    // DISTRIBUTE the column instead of leaving a dead void under the board —
    // flex stretch only (no fixed pixel heights), and only when the height
    // is actually there (≥900px), so laptop/tablet landscape is untouched.
    <div className="mx-auto max-w-[1600px] [@media(min-height:900px)]:flex [@media(min-height:900px)]:min-h-[calc(100dvh-2.5rem)] [@media(min-height:900px)]:flex-col">
      <header className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="nameplate-strong text-xl text-text">Command</h1>
          <div className="nameplate text-2xs text-text-dim">
            {periodLabel(COMMAND_PERIOD)} · year to date
          </div>
        </div>
        <BasisToggle value={basis} onChange={setBasis} />
      </header>

      <div className="mb-4">
        {territories.isSuccess ? (
          <KpiRow
            territories={rows}
            basis={basis}
            coverageRows={coverage.data?.items}
            summary={summary.data}
          />
        ) : territories.isError ? (
          <ErrorPanel onRetry={() => territories.refetch()} />
        ) : (
          <LoadingPanel label="Loading KPIs" />
        )}
      </div>

      <div className="[@media(min-height:900px)]:min-h-0 [@media(min-height:900px)]:flex-1">
        {territories.isSuccess ? (
          rows.length === 0 ? (
            <div className="rounded-lg border border-seam bg-surface p-6 text-xs text-text-dim">
              No territory activity in scope for this year.
            </div>
          ) : (
            <TerritoryBoard
              rows={rows}
              basis={basis}
              signalCounts={signalCounts}
              onSelect={setSelected}
            />
          )
        ) : territories.isError ? (
          <ErrorPanel
            onRetry={() => territories.refetch()}
            message="Couldn’t load the Territory Board."
          />
        ) : (
          <LoadingPanel label="Loading Territory Board" />
        )}
      </div>

      {selected && (
        <DrillDrawer
          code={selected}
          basis={basis}
          territories={rows}
          coverageRows={coverage.data?.items}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}
