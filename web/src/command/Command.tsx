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
    <div className="mx-auto max-w-[1600px]">
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
