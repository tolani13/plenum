// Data Quality (P5, R2) — the panel that FINDS the seeded mess instead of
// hiding it. Read-only: deterministic SQL finders on the server, each
// finding linking to the offending record. Mess is information — a system
// that expects dirty data and surfaces it calmly is the credibility play
// (spec §9 beat 5). RLS-scoped like everything else: a rep sees only her
// own scope's mess, so the seeded trio is only guaranteed complete at VP
// view — the scope chip says so out loud.

import { Link } from "react-router";
import { money } from "../lib/format";
import { useDataQuality } from "../lib/queries";
import { useMe } from "../auth/auth";
import { useScreenReady } from "../lib/useScreenReady";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/states";

function FindingCard({
  title,
  countLabel,
  emptyNote,
  children,
  testid,
}: {
  title: string;
  countLabel: string;
  emptyNote: string;
  children: React.ReactNode | null;
  testid: string;
}) {
  return (
    <section
      className="rounded-lg border border-seam bg-surface"
      data-testid={testid}
    >
      <div className="flex items-baseline justify-between gap-2 border-b border-seam px-4 py-2.5">
        <span className="nameplate text-2xs text-text-dim">{title}</span>
        <span className="tabular text-2xs text-text" data-testid={`${testid}-count`}>
          {countLabel}
        </span>
      </div>
      <div className="p-3">
        {children ?? <div className="px-1 py-2 text-2xs text-text-dim">{emptyNote}</div>}
      </div>
    </section>
  );
}

function TerritoryChip({ code }: { code: string }) {
  return (
    <span className="nameplate shrink-0 rounded-sm border border-seam px-1 py-0.5 text-2xs text-text-dim">
      {code}
    </span>
  );
}

export function DataQuality() {
  const me = useMe();
  const query = useDataQuality();
  useScreenReady(query.isSuccess || query.isError, "data-quality");

  if (query.isLoading) return <LoadingPanel label="Scanning the book" />;
  if (query.isError)
    return (
      <div className="mx-auto max-w-[1200px]">
        <ErrorPanel onRetry={() => query.refetch()} />
      </div>
    );

  const dq = query.data!;
  const total =
    dq.duplicate_names.length +
    dq.null_cadence_units.length +
    dq.full_discount_lines.length +
    dq.zero_site_accounts.length;
  const fullBook = me.data?.role === "vp" || me.data?.role === "admin";

  return (
    <div className="mx-auto max-w-[1200px] space-y-4">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="nameplate-strong text-xl text-text">Data Quality</h1>
          <div className="nameplate text-2xs text-text-dim">
            mess is information — a CRM that expects dirty data and says so
          </div>
        </div>
        <span
          className="nameplate rounded-sm border border-seam px-1.5 py-0.5 text-2xs text-text-dim"
          data-testid="dq-scope-chip"
        >
          {fullBook
            ? `full book · ${me.data?.role}`
            : `your scope only (${me.data?.territories.join("+") ?? "—"}) — the full census lives at VP view`}
        </span>
      </header>

      {total === 0 ? (
        <EmptyPanel message="Clean book — no duplicate-ish names, no unknown cadences, no fully-discounted lines, no site-less accounts in your scope. (The seeded mess lives in other territories; the VP sees all of it.)" />
      ) : (
        <div className="grid grid-cols-1 gap-4 min-[900px]:grid-cols-2">
          <FindingCard
            title="Duplicate-ish account names"
            countLabel={`${dq.duplicate_names.length} pair${dq.duplicate_names.length === 1 ? "" : "s"}`}
            emptyNote="No near-duplicates in scope."
            testid="dq-duplicates"
          >
            {dq.duplicate_names.length === 0 ? null : (
              <ul className="space-y-2">
                {dq.duplicate_names.map((d) => (
                  <li
                    key={`${d.a_id}-${d.b_id}`}
                    data-testid="dq-duplicate-row"
                    className="rounded border border-seam bg-surface-2 p-2.5"
                  >
                    <div className="flex flex-wrap items-center gap-2">
                      <Link
                        to={`/accounts/${d.a_id}`}
                        className="text-sm text-text hover:text-data"
                      >
                        {d.a_name}
                      </Link>
                      <TerritoryChip code={d.a_territory_code} />
                      <span className="nameplate text-2xs text-text-dim">
                        vs
                      </span>
                      <Link
                        to={`/accounts/${d.b_id}`}
                        className="text-sm text-text hover:text-data"
                      >
                        {d.b_name}
                      </Link>
                      <TerritoryChip code={d.b_territory_code} />
                    </div>
                    <div className="mt-1 text-2xs text-text-dim">
                      normalize to the same key “{d.name_key}” — likely one
                      customer entered twice
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </FindingCard>

          <FindingCard
            title="Installed units with unknown cadence"
            countLabel={`${dq.null_cadence_units.length} unit${dq.null_cadence_units.length === 1 ? "" : "s"}`}
            emptyNote="Every cartridge-bearing unit in scope has a stated change-out cadence."
            testid="dq-null-cadence"
          >
            {dq.null_cadence_units.length === 0 ? null : (
              <ul className="space-y-2">
                {dq.null_cadence_units.map((u) => (
                  <li
                    key={u.unit_id}
                    data-testid="dq-cadence-row"
                    className="rounded border border-seam bg-surface-2 p-2.5"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <Link
                        to={`/accounts/${u.account_id}`}
                        className="text-sm text-text hover:text-data"
                      >
                        {u.account_name}
                      </Link>
                      <TerritoryChip code={u.territory_code} />
                    </div>
                    <div className="mt-1 text-2xs text-text-dim">
                      <span className="tabular">{u.serial}</span>
                      {u.cartridge_sku ? ` · ${u.cartridge_sku}` : ""} · no
                      expected_changeout_months — cadence math (reorder radar,
                      defection alarm, coverage) cannot run; the 360 shows its
                      CADENCE UNKNOWN chip
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </FindingCard>

          <FindingCard
            title="Order lines at 100% discount"
            countLabel={`${dq.full_discount_lines.length} line${dq.full_discount_lines.length === 1 ? "" : "s"}`}
            emptyNote="No fully-comped lines in scope."
            testid="dq-full-discount"
          >
            {dq.full_discount_lines.length === 0 ? null : (
              <ul className="space-y-2">
                {dq.full_discount_lines.map((l) => (
                  <li
                    key={l.order_line_id}
                    data-testid="dq-comped-row"
                    className="rounded border border-seam bg-surface-2 p-2.5"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <Link
                        to={`/accounts/${l.account_id}`}
                        className="text-sm text-text hover:text-data"
                      >
                        {l.account_name}
                      </Link>
                      <TerritoryChip code={l.territory_code} />
                    </div>
                    <div className="mt-1 text-2xs text-text-dim">
                      <span className="tabular">{l.ordered_on}</span> ·{" "}
                      {l.product_sku} × {l.qty} · list{" "}
                      <span className="tabular">
                        {money(l.list_unit_cents * l.qty)}
                      </span>{" "}
                      → net <span className="tabular text-warn">$0.00</span> —
                      revenue given away, or a comp that should be a service
                      record
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </FindingCard>

          <FindingCard
            title="Accounts with zero sites"
            countLabel={`${dq.zero_site_accounts.length}`}
            emptyNote="None — every account in scope carries at least one site."
            testid="dq-zero-sites"
          >
            {dq.zero_site_accounts.length === 0 ? null : (
              <ul className="space-y-2">
                {dq.zero_site_accounts.map((a) => (
                  <li
                    key={a.account_id}
                    className="flex items-center justify-between gap-2 rounded border border-seam bg-surface-2 p-2.5"
                  >
                    <Link
                      to={`/accounts/${a.account_id}`}
                      className="text-sm text-text hover:text-data"
                    >
                      {a.account_name}
                    </Link>
                    <TerritoryChip code={a.territory_code} />
                  </li>
                ))}
              </ul>
            )}
          </FindingCard>
        </div>
      )}
    </div>
  );
}
