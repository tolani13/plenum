// Quote detail (spec §8 screen 7) — lines, the live policy verdict, the
// role-gated submit/approve/reject actions (UI mirrors the server gate exactly,
// so a rep never sees an Approve control on their own quote — acceptance
// check 4), and the audit trail (R4: action, actor, timestamp, before→after).

import { useState } from "react";
import { Link, useParams } from "react-router";
import { ArrowLeft, Check, X, Send } from "lucide-react";
import { money, percent } from "../lib/format";
import { useMe } from "../auth/auth";
import {
  useApproveQuote,
  useDiscountPolicy,
  useQuoteAudit,
  useQuoteDetail,
  useRejectQuote,
  useSubmitQuote,
} from "../lib/crm";
import { useScreenReady } from "../lib/useScreenReady";
import { ErrorPanel, LoadingPanel } from "../components/states";
import { QuoteStatusChip } from "./badges";
import { roleCanDecide, verdictLabel, verdictTier } from "./verdict";

function VerdictPanel({
  worst,
  selfMax,
  managerMax,
}: {
  worst: number;
  selfMax: number;
  managerMax: number;
}) {
  const tier = verdictTier(worst, { self_max_pct: selfMax, manager_max_pct: managerMax });
  const tone =
    tier === "self" ? "text-ok" : tier === "manager" ? "text-warn" : "text-alarm";
  return (
    <div className="rounded-lg border border-seam bg-surface p-4" data-testid="verdict">
      <div className="nameplate text-2xs text-text-dim">Discount policy verdict</div>
      <div className={`mt-1 text-lg ${tone}`}>
        {verdictLabel(tier, { self_max_pct: selfMax, manager_max_pct: managerMax })}
      </div>
      <div className="mt-1 text-2xs text-text-dim">
        worst line <span className="tabular text-text">{percent(worst, 2)}</span> ·
        self ≤ {selfMax}% · manager ≤ {managerMax}% · VP &gt; {managerMax}%
      </div>
    </div>
  );
}

export function QuoteDetail() {
  const { id } = useParams();
  const quote = useQuoteDetail(id);
  const audit = useQuoteAudit(id);
  const policy = useDiscountPolicy();
  const me = useMe();
  const submit = useSubmitQuote();
  const approve = useApproveQuote();
  const reject = useRejectQuote();
  const [reason, setReason] = useState("");

  useScreenReady(
    (quote.isSuccess || quote.isError) && (policy.isSuccess || policy.isError),
    "quote-detail",
  );

  if (quote.isLoading) return <LoadingPanel label="Loading quote" />;
  if (quote.isError || !quote.data)
    return (
      <div className="mx-auto max-w-[1100px]">
        <ErrorPanel
          onRetry={() => quote.refetch()}
          message="Couldn’t load this quote (or it isn’t in your territory)."
        />
      </div>
    );

  const q = quote.data;
  const worst = q.worst_discount_pct ?? 0;
  const selfMax = policy.data?.self_max_pct ?? 10;
  const managerMax = policy.data?.manager_max_pct ?? 25;
  const tier = verdictTier(worst, { self_max_pct: selfMax, manager_max_pct: managerMax });

  const isCreator = me.data?.id === q.created_by;
  const canSubmit = q.status === "draft" && isCreator;
  const canDecide =
    q.status === "pending_approval" &&
    !!me.data &&
    roleCanDecide(me.data.role, tier);

  const busy = submit.isPending || approve.isPending || reject.isPending;

  return (
    <div className="mx-auto max-w-[1100px] space-y-4">
      <div>
        <Link
          to="/quotes"
          className="mb-2 inline-flex items-center gap-1 text-2xs text-text-dim hover:text-text"
        >
          <ArrowLeft size={12} /> <span className="nameplate">Quotes</span>
        </Link>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-3">
            <h1 className="nameplate-strong text-xl text-text">
              <Link to={`/accounts/${q.account_id}`} className="hover:text-data">
                {q.account_name}
              </Link>
            </h1>
            <QuoteStatusChip status={q.status} />
          </div>
          <div className="nameplate text-2xs text-text-dim">
            {q.territory_code} · by {q.created_by_name}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 min-[720px]:grid-cols-[2fr_1fr]">
        <div className="scroll-x rounded-lg border border-seam bg-surface">
          <table className="w-full text-sm">
            <thead className="border-b border-seam">
              <tr>
                {["Product", "Qty", "List", "Disc.", "Net", "Line net"].map((h, i) => (
                  <th
                    key={h}
                    className={`nameplate px-3 py-2 text-2xs text-text-dim ${
                      i === 0 ? "text-left" : "text-right"
                    }`}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {q.lines.map((l) => (
                <tr key={l.id} className="border-b border-seam/40 last:border-0">
                  <td className="px-3 py-2 text-text">
                    <div className="truncate">{l.product_name}</div>
                    <div className="nameplate text-2xs text-text-dim">
                      {l.product_sku}
                    </div>
                  </td>
                  <td className="tabular px-3 py-2 text-right text-text">{l.qty}</td>
                  <td className="tabular px-3 py-2 text-right text-text">
                    {money(l.list_unit_cents)}
                  </td>
                  <td className="tabular px-3 py-2 text-right text-text">
                    {percent(l.discount_pct, 2)}
                  </td>
                  <td className="tabular px-3 py-2 text-right text-text">
                    {money(l.net_unit_cents)}
                  </td>
                  <td className="tabular px-3 py-2 text-right text-text">
                    {money(l.line_net_cents)}
                  </td>
                </tr>
              ))}
            </tbody>
            <tfoot className="border-t border-seam bg-surface-2/40">
              <tr>
                <td className="nameplate px-3 py-2 text-2xs text-text" colSpan={4}>
                  Total · gross {money(q.gross_cents)} · leakage{" "}
                  {money(q.leakage_cents)}
                </td>
                <td className="nameplate px-3 py-2 text-right text-2xs text-text-dim">
                  Net
                </td>
                <td className="tabular px-3 py-2 text-right text-sm text-text">
                  {money(q.net_cents)}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>

        <div className="space-y-3">
          <VerdictPanel worst={worst} selfMax={selfMax} managerMax={managerMax} />

          {/* actions — mirror the server role gate exactly */}
          <div className="rounded-lg border border-seam bg-surface p-4">
            <div className="nameplate text-2xs text-text-dim">Actions</div>
            {canSubmit && (
              <button
                onClick={() => submit.mutate(q.id)}
                disabled={busy}
                data-testid="submit-quote"
                className="mt-2 flex w-full items-center justify-center gap-2 rounded bg-data/15 py-2 text-sm text-data hover:bg-data/25 disabled:opacity-50"
              >
                <Send size={14} /> Submit for approval
              </button>
            )}
            {canDecide && (
              <div className="mt-2 space-y-2">
                <button
                  onClick={() => approve.mutate(q.id)}
                  disabled={busy}
                  data-testid="approve-quote"
                  className="flex w-full items-center justify-center gap-2 rounded bg-ok/15 py-2 text-sm text-ok hover:bg-ok/25 disabled:opacity-50"
                >
                  <Check size={14} /> Approve
                </button>
                <input
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                  placeholder="Rejection reason…"
                  data-testid="reject-reason"
                  className="w-full rounded border border-seam bg-bg px-3 py-1.5 text-sm text-text outline-none focus:border-seam-strong"
                />
                <button
                  onClick={() => reject.mutate({ id: q.id, reason: reason.trim() })}
                  disabled={busy || !reason.trim()}
                  data-testid="reject-quote"
                  className="flex w-full items-center justify-center gap-2 rounded bg-alarm/15 py-2 text-sm text-alarm hover:bg-alarm/25 disabled:opacity-40"
                >
                  <X size={14} /> Reject
                </button>
              </div>
            )}
            {!canSubmit && !canDecide && (
              <div className="mt-2 text-xs text-text-dim">
                {q.status === "pending_approval"
                  ? "Awaiting an authorized approver."
                  : q.decision_reason
                    ? `Rejected: ${q.decision_reason}`
                    : "No actions available in this state."}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* audit trail (R4) */}
      <section
        className="rounded-lg border border-seam bg-surface"
        data-testid="audit-drawer"
      >
        <h2 className="nameplate border-b border-seam px-4 py-2.5 text-2xs text-text-dim">
          Audit trail
        </h2>
        <div className="p-4">
          {audit.isLoading ? (
            <div className="text-xs text-text-dim">Loading…</div>
          ) : (audit.data?.items.length ?? 0) === 0 ? (
            <div className="text-xs text-text-dim">No audit entries yet.</div>
          ) : (
            <ul className="space-y-2">
              {audit.data!.items.map((r) => (
                <li
                  key={r.id}
                  className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-seam/40 pb-2 text-sm last:border-0"
                >
                  <span className="nameplate text-2xs text-text-dim">{r.action}</span>
                  <span className="text-text">
                    {r.before_status ?? "—"} → {r.after_status ?? "—"}
                  </span>
                  <span className="text-text-dim">{r.actor_name ?? "system"}</span>
                  <span className="tabular ml-auto text-2xs text-text-dim">
                    {r.at.slice(0, 19).replace("T", " ")}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </section>
    </div>
  );
}
