// Territory drill — a right-side drawer composed from data already fetched
// (architect resolution 3): no metrics endpoint takes a territory filter. It
// shows the tile's full instrument row, its coverage row, and its at-risk
// units filtered client-side from the defection page. Esc or backdrop closes.
//
// P4 (R7): Command itself no longer calls useDefection (its 4th KPI reads
// the signals summary) — so the drawer owns that fetch now, LAZILY: the
// defection feed loads only when someone actually drills a tile. The
// /metrics/defection endpoint is untouched.

import { useEffect } from "react";
import { X } from "lucide-react";
import { money, percent, count } from "../lib/format";
import { useDefection } from "../lib/queries";
import type { Basis } from "../lib/params";
import type { CoverageRow, TerritoryRow } from "../lib/types";

const DEFECTION_FETCH_CAP = 200;

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
      <span className={`tabular text-sm ${strong ? "text-text" : "text-text-dim"}`}>
        {value}
      </span>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mt-5">
      <div className="nameplate mb-1 text-2xs text-data">{title}</div>
      {children}
    </div>
  );
}

export function DrillDrawer({
  code,
  basis,
  territories,
  coverageRows,
  onClose,
}: {
  code: string;
  basis: Basis;
  territories: TerritoryRow[];
  coverageRows: CoverageRow[] | undefined;
  onClose: () => void;
}) {
  const defection = useDefection();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const t = territories.find((r) => r.territory_code === code);
  const cov = coverageRows?.find((r) => r.territory_code === code);
  const atRisk = (defection.data?.items ?? [])
    .filter((r) => r.territory_code === code)
    .sort((a, b) => b.score - a.score);
  const capped =
    defection.data !== undefined && defection.data.total > DEFECTION_FETCH_CAP;

  return (
    <div className="fixed inset-0 z-50 flex" data-testid="drill-drawer" role="dialog" aria-modal="true">
      <div
        className="flex-1 bg-black/50"
        onClick={onClose}
        data-testid="drill-backdrop"
        aria-hidden="true"
      />
      <div className="scroll-y w-full max-w-md overflow-y-auto border-l border-seam bg-surface p-5">
        <div className="flex items-start justify-between">
          <div>
            <div className="nameplate-strong text-lg text-text">{code}</div>
            <div className="text-2xs text-text-dim">
              {t?.territory_name ?? ""}
            </div>
          </div>
          <button
            onClick={onClose}
            className="rounded p-1 text-text-dim transition-colors hover:bg-surface-2 hover:text-text"
            aria-label="Close"
            data-testid="drill-close"
          >
            <X size={18} strokeWidth={2} />
          </button>
        </div>

        {t && (
          <Section title="Revenue">
            <Field
              label="Gross"
              value={money(t.gross_cents)}
              strong={basis === "gross"}
            />
            <Field
              label="Net"
              value={money(t.net_cents)}
              strong={basis === "net"}
            />
            <Field label="Leakage" value={money(t.leakage_cents)} />
            <Field label="Leakage %" value={percent(t.leakage_pct)} />
            <Field label="Orders" value={count(t.order_count)} />
            <Field label="Active accounts" value={count(t.active_accounts)} />
            <Field
              label="Quota attainment"
              value={percent(t.quota_attainment_pct)}
            />
          </Section>
        )}

        <Section title="Aftermarket coverage — this quarter">
          {cov ? (
            <>
              <Field label="Units due" value={count(cov.units_due)} />
              <Field label="% covered" value={percent(cov.pct_covered)} />
              <Field
                label="Projected (gross)"
                value={money(cov.projected_consumable_gross_cents)}
                strong={basis === "gross"}
              />
              <Field
                label="Projected (net)"
                value={money(cov.projected_consumable_net_cents)}
                strong={basis === "net"}
              />
            </>
          ) : (
            <div className="text-2xs text-text-dim">
              No units due for change-out this quarter.
            </div>
          )}
        </Section>

        <Section title={`At-risk units${capped ? " (top 200 by score)" : ""}`}>
          {defection.isLoading ? (
            <div className="pulse text-2xs text-text-dim">Loading…</div>
          ) : atRisk.length === 0 ? (
            <div className="text-2xs text-text-dim">
              No units past their reorder cadence in scope.
            </div>
          ) : (
            <ul className="space-y-1.5">
              {atRisk.map((u) => (
                <li
                  key={u.serial}
                  className="rounded border border-seam bg-bg px-2.5 py-2"
                  data-testid="atrisk-unit"
                >
                  <div className="flex items-baseline justify-between gap-2">
                    <span className="truncate text-xs text-text">
                      {u.account_name}
                    </span>
                    <span className="tabular shrink-0 text-2xs text-warn">
                      {money(u.annual_consumable_value_cents)}/yr
                    </span>
                  </div>
                  <div className="mt-0.5 flex items-center justify-between text-2xs text-text-dim">
                    <span className="truncate">
                      {u.serial} · {u.site}
                    </span>
                    <span className="tabular shrink-0">
                      {count(u.days_silent)}d silent
                    </span>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Section>
      </div>
    </div>
  );
}
