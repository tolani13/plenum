// Command — the signature screen. Territory Board + four KPIs + the GROSS/NET
// toggle. Period is fixed to current-year YTD (spec §8; Command has no period
// control). The toggle only changes a URL display choice: the payload is
// dual-basis and the query key omits basis, so flipping re-renders in place —
// every dollar figure + KPI flips, no refetch, no flash (gate P2-1).
//
// Architect ruling 2026-07-19: at 2026 the frozen book ranks territories the
// same on both bases, so the tile ORDER holds honestly (the rank-movement
// observable lives on the Customers leaderboard). Nothing here fakes motion.

import { useState } from "react";
import { useSearchParams } from "react-router";
import { COMMAND_PERIOD, parseBasis, periodLabel } from "../lib/params";
import { useTerritories, useCoverage, useDefection } from "../lib/queries";
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
  const defection = useDefection();

  const settled = [territories, coverage, defection].every(
    (query) => query.isSuccess || query.isError,
  );
  useScreenReady(settled);

  const setBasis = (next: string) => {
    const p = new URLSearchParams(params);
    p.set("basis", next);
    setParams(p, { replace: true });
  };

  const rows = territories.data?.items ?? [];

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
            defectionRows={defection.data?.items}
            defectionTotal={defection.data?.total}
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
          <TerritoryBoard rows={rows} basis={basis} onSelect={setSelected} />
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
          defectionRows={defection.data?.items}
          defectionTotal={defection.data?.total}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}
