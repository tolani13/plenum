// Territory Map (P5, R3) — the Territory Board projected onto the continent.
// Period is fixed to the current YTD exactly like Command (the acceptance
// checks compare this screen's totals against Command tiles and
// Leaderboards at the same period+basis); the GROSS/NET toggle flips every
// dollar on the screen at once, zero refetch (dual-basis payloads, the P2
// law). Scope model: the map's SHAPE is config for everyone (all 51 + the
// two Canada blocks always render); its MONEY is scoped — out-of-scope
// territories are dimmed silhouettes with no dollar values anywhere in the
// DOM, hover included (the tripwire's map-scope assertion).

import { useMemo, useState } from "react";
import { useSearchParams } from "react-router";
import { COMMAND_PERIOD, parseBasis } from "../lib/params";
import { useStates, useTerritories } from "../lib/queries";
import { money, moneyAbbrev, percent, count } from "../lib/format";
import { useMe } from "../auth/auth";
import { useScreenReady } from "../lib/useScreenReady";
import { BasisToggle } from "../components/BasisToggle";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/states";
import type { StateRow, TerritoryRoster } from "../lib/types";
import { territoryFill, UsMap, type MapHover } from "./UsMap";

function Field({
  label,
  value,
  strong,
}: {
  label: string;
  value: string;
  strong?: boolean;
}) {
  return (
    <div className="flex items-baseline justify-between gap-3 border-b border-seam/60 py-1.5">
      <span className="nameplate text-2xs text-text-dim">{label}</span>
      <span
        className={`tabular text-sm ${strong ? "text-text" : "text-text-dim"}`}
      >
        {value}
      </span>
    </div>
  );
}

export function TerritoryMap() {
  const [params, setParams] = useSearchParams();
  const basis = parseBasis(params.get("basis"));
  const [selected, setSelected] = useState<string | null>(null);
  const [hover, setHover] = useState<MapHover | null>(null);

  const me = useMe();
  const states = useStates(COMMAND_PERIOD);
  const territories = useTerritories(COMMAND_PERIOD);

  const settled =
    (states.isSuccess || states.isError) &&
    (territories.isSuccess || territories.isError);
  useScreenReady(settled);

  const setBasis = (next: string) => {
    const p = new URLSearchParams(params);
    p.set("basis", next);
    setParams(p, { replace: true });
  };

  const scopeCodes = useMemo(
    () => new Set(me.data?.territories ?? []),
    [me.data],
  );
  const roster: TerritoryRoster[] = useMemo(
    () => states.data?.territories ?? [],
    [states.data],
  );
  const territoryOfState = useMemo(() => {
    const m = new Map<string, string>();
    for (const t of roster) {
      for (const s of t.state_codes) m.set(s, t.territory_code);
    }
    return (state: string) => m.get(state);
  }, [roster]);
  const stateRows = useMemo(() => {
    const m = new Map<string, StateRow>();
    for (const r of states.data?.items ?? []) m.set(r.state_code, r);
    return m;
  }, [states.data]);

  const basisCents = (r: { gross_cents: number; net_cents: number }) =>
    basis === "gross" ? r.gross_cents : r.net_cents;

  // In-scope Canada block money line: the block's provinces summed from the
  // scoped state rows (the block IS the territory on this map).
  const canadaSubtitle = (territoryCode: string): string | null => {
    if (!scopeCodes.has(territoryCode)) return null;
    let sum = 0;
    let seen = false;
    for (const r of stateRows.values()) {
      if (r.territory_code === territoryCode) {
        sum += basisCents(r);
        seen = true;
      }
    }
    return seen ? `${moneyAbbrev(sum)} ${basis}` : null;
  };

  if (states.isLoading || territories.isLoading) {
    return <LoadingPanel label="Loading territory map" />;
  }
  if (states.isError) {
    return (
      <div className="mx-auto max-w-[1600px]">
        <ErrorPanel
          onRetry={() => states.refetch()}
          message="Couldn’t load the map feed."
        />
      </div>
    );
  }

  const selectedRoster = roster.find((t) => t.territory_code === selected);
  const selectedMetrics = territories.data?.items.find(
    (t) => t.territory_code === selected,
  );
  const selectedInScope = selected !== null && scopeCodes.has(selected);
  const hoverRow = hover ? stateRows.get(hover.code) : undefined;

  return (
    <div className="mx-auto max-w-[1700px] space-y-4">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="nameplate-strong text-xl text-text">Territory Map</h1>
          <div className="nameplate text-2xs text-text-dim">
            {COMMAND_PERIOD} · year to date · the board, projected on the
            continent
          </div>
        </div>
        <BasisToggle value={basis} onChange={setBasis} />
      </header>

      <div className="grid grid-cols-1 gap-4 min-[980px]:grid-cols-[minmax(0,1fr)_minmax(280px,340px)]">
        {/* ── the map panel ── */}
        <section className="min-w-0 rounded-lg border border-seam bg-surface p-3">
          <UsMap
            territoryOf={territoryOfState}
            scopeCodes={scopeCodes}
            selected={selected}
            onSelect={(code) =>
              setSelected((cur) => (cur === code ? null : code))
            }
            onHover={setHover}
            canadaSubtitle={canadaSubtitle}
          />

          {/* legend — config for everyone; scope shown honestly */}
          <div
            className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t border-seam/60 pt-2.5"
            data-testid="map-legend"
          >
            {roster.map((t) => {
              const inScope = scopeCodes.has(t.territory_code);
              return (
                <button
                  key={t.territory_code}
                  onClick={() =>
                    setSelected((cur) =>
                      cur === t.territory_code ? null : t.territory_code,
                    )
                  }
                  className={`flex items-center gap-1.5 text-2xs transition-colors ${
                    selected === t.territory_code
                      ? "text-text"
                      : "text-text-dim hover:text-text"
                  }`}
                  title={t.territory_name}
                >
                  <span
                    className="inline-block h-2.5 w-2.5 rounded-sm border border-seam"
                    style={{
                      background: inScope
                        ? territoryFill(t.territory_code)
                        : "var(--color-land-dim)",
                    }}
                  />
                  <span className="nameplate">{t.territory_code}</span>
                </button>
              );
            })}
            {me.data && scopeCodes.size < roster.length && (
              <span className="nameplate ml-auto rounded-sm border border-seam px-1.5 py-0.5 text-2xs text-text-dim">
                scope {me.data.territories.join("+")} · foreign territories:
                shape only
              </span>
            )}
          </div>
        </section>

        {/* ── the territory panel ── */}
        <aside
          className="min-w-0 rounded-lg border border-seam bg-surface p-4"
          data-testid="map-panel"
          data-territory={selected ?? ""}
        >
          {!selectedRoster ? (
            <EmptyPanel message="Click a state or a Canada block — the territory panel opens here with its board figures and staffing." />
          ) : (
            <div>
              <div className="flex items-start justify-between gap-2">
                <div>
                  <div className="nameplate-strong text-lg text-text">
                    {selectedRoster.territory_code}
                  </div>
                  <div className="text-2xs text-text-dim">
                    {selectedRoster.territory_name} ·{" "}
                    {selectedRoster.region.replace("_", " ")}
                  </div>
                </div>
                <span
                  className="mt-1 inline-block h-3 w-3 rounded-sm border border-seam"
                  style={{
                    background: selectedInScope
                      ? territoryFill(selectedRoster.territory_code)
                      : "var(--color-land-dim)",
                  }}
                />
              </div>

              <div className="mt-4">
                <div className="nameplate mb-1 text-2xs text-data">
                  Territory manager
                </div>
                <div className="text-sm text-text">
                  {selectedRoster.tm_names.join(" · ") || "—"}
                </div>
                <div className="nameplate mb-1 mt-3 text-2xs text-data">
                  Regional manager
                </div>
                <div className="text-sm text-text">
                  {selectedRoster.rm_names.join(" · ") || "—"}
                </div>
              </div>

              <div className="mt-4">
                <div className="nameplate mb-1 text-2xs text-data">
                  {COMMAND_PERIOD} YTD
                </div>
                {selectedInScope && selectedMetrics ? (
                  <>
                    <Field
                      label="Gross"
                      value={money(selectedMetrics.gross_cents)}
                      strong={basis === "gross"}
                    />
                    <Field
                      label="Net"
                      value={money(selectedMetrics.net_cents)}
                      strong={basis === "net"}
                    />
                    <Field
                      label="Leakage"
                      value={money(selectedMetrics.leakage_cents)}
                    />
                    <Field
                      label="Leakage %"
                      value={percent(selectedMetrics.leakage_pct)}
                    />
                    <Field
                      label="Quota attainment"
                      value={percent(selectedMetrics.quota_attainment_pct)}
                    />
                    <Field
                      label="Orders"
                      value={count(selectedMetrics.order_count)}
                    />
                  </>
                ) : selectedInScope ? (
                  <div className="text-xs text-text-dim">
                    No {COMMAND_PERIOD} activity in this territory yet.
                  </div>
                ) : (
                  <div className="text-xs text-text-dim">
                    Out of your scope — geography and staffing are config; the
                    ledger is not. No dollar values are available here.
                  </div>
                )}
              </div>

              <div className="mt-4 border-t border-seam/60 pt-3">
                <div className="nameplate text-2xs text-text-dim">
                  Mapped states
                </div>
                <div className="mt-1 flex flex-wrap gap-1">
                  {selectedRoster.state_codes.map((s) => (
                    <span
                      key={s}
                      className="nameplate rounded-sm border border-seam px-1 py-0.5 text-2xs text-text-dim"
                    >
                      {s}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          )}
        </aside>
      </div>

      {/* ── hover tooltip — money ONLY for in-scope states ── */}
      {hover && (
        <div
          data-testid="map-tooltip"
          data-territory={hover.territory}
          className="pointer-events-none fixed z-50 rounded border border-seam-strong bg-surface-2 px-2.5 py-1.5 text-2xs shadow-lg"
          style={{
            left: Math.min(hover.x + 14, window.innerWidth - 200),
            top: hover.y + 14,
          }}
        >
          <div className="text-text">{hover.name}</div>
          <div className="nameplate mt-0.5 text-text-dim">
            {hover.territory || "unassigned"}
          </div>
          {hover.inScope && hoverRow ? (
            <div className="tabular mt-1 text-text">
              {money(basisCents(hoverRow))}{" "}
              <span className="nameplate text-text-dim">{basis}</span>
              <span className="text-text-dim">
                {" "}
                · {count(hoverRow.order_count)} orders
              </span>
            </div>
          ) : hover.inScope ? (
            <div className="mt-1 text-text-dim">no YTD activity</div>
          ) : (
            <div className="nameplate mt-1 text-text-dim">
              out of scope — no ledger access
            </div>
          )}
        </div>
      )}
    </div>
  );
}
