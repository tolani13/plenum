// Account 360 (spec §8 screen 5) — the installed-base timeline as the hero,
// plus header KPIs (cumulative gross/net/leakage), recent orders, opportunities,
// the activity log (writable), contacts, and the designed signals-empty state
// (P4). Reached by clicking a customer/card/inbox row — not a rail entry (R7).

import { useState } from "react";
import { Link, useParams } from "react-router";
import { ArrowLeft } from "lucide-react";
import { money, percent } from "../lib/format";
import type { ActivityKind } from "../lib/types";
import { useAccountDetail, useCreateActivity } from "../lib/crm";
import { useScreenReady } from "../lib/useScreenReady";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/states";
import { StageChip } from "./badges";
import { Timeline } from "./Timeline";

function Kpi({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="rounded-lg border border-seam bg-surface p-4">
      <div className="nameplate text-2xs text-text-dim">{label}</div>
      <div className="tabular mt-1 truncate text-xl text-text" title={value}>
        {value}
      </div>
      <div className="mt-0.5 h-4 text-2xs text-text-dim">{sub ?? ""}</div>
    </div>
  );
}

function Panel({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-lg border border-seam bg-surface">
      <h2 className="nameplate border-b border-seam px-4 py-2.5 text-2xs text-text-dim">
        {title}
      </h2>
      <div className="p-4">{children}</div>
    </section>
  );
}

const KINDS: ActivityKind[] = ["call", "visit", "email", "note"];

function ActivityLog({ accountId }: { accountId: string }) {
  const account = useAccountDetail(accountId);
  const create = useCreateActivity();
  const [kind, setKind] = useState<ActivityKind>("call");
  const [body, setBody] = useState("");

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const text = body.trim();
    if (!text || create.isPending) return;
    create.mutate(
      { account_id: accountId, kind, body: text },
      { onSuccess: () => setBody("") },
    );
  };

  const items = account.data?.activities.items ?? [];
  return (
    <div className="space-y-3">
      <form onSubmit={submit} className="flex flex-wrap items-center gap-2">
        <select
          value={kind}
          onChange={(e) => setKind(e.target.value as ActivityKind)}
          className="nameplate rounded border border-seam bg-bg px-2 py-1.5 text-2xs text-text outline-none focus:border-seam-strong"
          data-testid="activity-kind"
        >
          {KINDS.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
        <input
          value={body}
          onChange={(e) => setBody(e.target.value)}
          placeholder="Log a call, visit, email, or note…"
          className="min-w-0 flex-1 rounded border border-seam bg-bg px-3 py-1.5 text-sm text-text outline-none focus:border-seam-strong"
          data-testid="activity-body"
        />
        <button
          type="submit"
          disabled={create.isPending || !body.trim()}
          data-testid="activity-log"
          className="rounded border border-seam px-3 py-1.5 text-2xs text-data transition-colors hover:bg-surface-2 disabled:opacity-40"
        >
          <span className="nameplate">Log</span>
        </button>
      </form>

      {items.length === 0 ? (
        <div className="text-xs text-text-dim">No activity logged yet.</div>
      ) : (
        <ul className="space-y-2">
          {items.map((a) => (
            <li key={a.id} className="border-b border-seam/40 pb-2 last:border-0">
              <div className="flex items-center justify-between gap-2">
                <span className="nameplate text-2xs text-text-dim">{a.kind}</span>
                <span className="tabular text-2xs text-text-dim">
                  {a.occurred_at.slice(0, 16).replace("T", " ")}
                </span>
              </div>
              <div className="text-sm text-text">{a.body}</div>
              <div className="text-2xs text-text-dim">{a.rep_name}</div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function Account360() {
  const { id } = useParams();
  const account = useAccountDetail(id);
  useScreenReady(account.isSuccess || account.isError);

  if (account.isLoading) return <LoadingPanel label="Loading account" />;
  if (account.isError || !account.data)
    return (
      <div className="mx-auto max-w-[1200px]">
        <ErrorPanel
          onRetry={() => account.refetch()}
          message="Couldn’t load this account (or it isn’t in your territory)."
        />
      </div>
    );

  const a = account.data;
  return (
    <div className="mx-auto max-w-[1400px] space-y-4">
      <div>
        <Link
          to="/leaderboards?tab=customers"
          className="mb-2 inline-flex items-center gap-1 text-2xs text-text-dim hover:text-text"
        >
          <ArrowLeft size={12} /> <span className="nameplate">Back</span>
        </Link>
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <h1 className="nameplate-strong text-xl text-text">{a.name}</h1>
          <div className="flex items-center gap-2">
            <span className="nameplate text-2xs text-text-dim">{a.industry}</span>
            <span className="nameplate text-2xs text-text-dim">·</span>
            <span className="nameplate text-2xs text-text-dim">{a.status}</span>
            <span className="nameplate rounded-sm border border-seam px-1.5 py-0.5 text-2xs text-text-dim">
              {a.territory_code}
            </span>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 min-[560px]:grid-cols-3">
        <Kpi label="Cumulative gross" value={money(a.cumulative.gross_cents)} />
        <Kpi label="Cumulative net" value={money(a.cumulative.net_cents)} />
        <Kpi
          label="Leakage"
          value={percent(a.cumulative.leakage_pct)}
          sub={money(a.cumulative.leakage_cents)}
        />
      </div>

      <Timeline units={a.units} />

      <div className="grid grid-cols-1 gap-4 min-[900px]:grid-cols-2">
        <Panel title="Recent orders">
          {a.recent_orders.length === 0 ? (
            <div className="text-xs text-text-dim">No orders on file.</div>
          ) : (
            <div className="scroll-x">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-seam">
                    <th className="nameplate px-2 py-1.5 text-left text-2xs text-text-dim">
                      Date
                    </th>
                    <th className="nameplate px-2 py-1.5 text-right text-2xs text-text-dim">
                      Gross
                    </th>
                    <th className="nameplate px-2 py-1.5 text-right text-2xs text-text-dim">
                      Net
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {a.recent_orders.map((o) => (
                    <tr key={o.id} className="border-b border-seam/40 last:border-0">
                      <td className="tabular px-2 py-1.5 text-text">{o.ordered_on}</td>
                      <td className="tabular px-2 py-1.5 text-right text-text">
                        {money(o.gross_cents)}
                      </td>
                      <td className="tabular px-2 py-1.5 text-right text-text">
                        {money(o.net_cents)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Panel>

        <Panel title="Opportunities">
          {a.opportunities.length === 0 ? (
            <div className="text-xs text-text-dim">No open opportunities.</div>
          ) : (
            <ul className="space-y-2">
              {a.opportunities.map((o) => (
                <li
                  key={o.id}
                  className="flex items-center justify-between gap-2 border-b border-seam/40 pb-2 last:border-0"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <StageChip stage={o.stage} />
                      <span className="text-sm text-text">{o.kind}</span>
                    </div>
                    <div className="text-2xs text-text-dim">{o.owner_name}</div>
                  </div>
                  <div className="tabular text-sm text-text">
                    {money(o.amount_cents)}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Panel>

        <Panel title="Activity log">
          <ActivityLog accountId={a.id} />
        </Panel>

        <Panel title="Contacts">
          {a.contacts.length === 0 ? (
            <div className="text-xs text-text-dim">No contacts on file.</div>
          ) : (
            <ul className="space-y-2">
              {a.contacts.map((c) => (
                <li key={c.id} className="border-b border-seam/40 pb-2 last:border-0">
                  <div className="text-sm text-text">{c.name}</div>
                  <div className="text-2xs text-text-dim">
                    {[c.title, c.email].filter(Boolean).join(" · ") || "—"}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>

      <Panel title="Signals">
        <EmptyPanel message="No signals yet — the signal engine (reorder / defection / conquest) lands in P4." />
      </Panel>
    </div>
  );
}
