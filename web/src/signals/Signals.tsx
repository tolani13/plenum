// Signals queue (spec §8 screen 4, ruling R6) — the daily-driver. Four lanes
// in signal-type order, collapsing 4→2→1 by available width; every card
// carries its receipts (reasons ON the face — the AI-with-receipts contract);
// actions write back through the P4 signal surface. Draft-quote-from-signal
// is pure CLIENT-SIDE COMPOSITION of P3 machinery (R6): pick or create the
// account's opportunity, then open the quote builder pre-filled with the
// signal's cartridge (or conquest best-fit) and qty = cartridge_count.

import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "react-router";
import { FilePlus2, PhoneCall, UserRoundPlus, XCircle } from "lucide-react";
import { money, count } from "../lib/format";
import type { SignalRow, SignalType } from "../lib/types";
import { useMe } from "../auth/auth";
import {
  useActionSignal,
  useAssignees,
  useAssignSignal,
  useDismissSignal,
  useSignals,
  useSignalsSummary,
  type QueueFilter,
} from "../lib/signals";
import { useCreateActivity, useCreateOpportunity, useOpportunities, usePatchStage } from "../lib/crm";
import { useScreenReady } from "../lib/useScreenReady";
import { ErrorPanel, LoadingPanel } from "../components/states";
import { Segmented } from "../components/Segmented";

const LANES: { type: SignalType; label: string }[] = [
  { type: "reorder_due", label: "Reorder due" },
  { type: "defection_risk", label: "Defection risk" },
  { type: "conquest", label: "Conquest" },
  { type: "discount_anomaly", label: "Discount anomaly" },
];

const FILTERS: { value: QueueFilter; label: string }[] = [
  { value: "active", label: "Active" },
  { value: "actioned", label: "Actioned" },
  { value: "dismissed", label: "Dismissed" },
];

const EMPTY_LANE: Record<SignalType, string> = {
  reorder_due: "No reorder signals in scope — no units inside their cadence window.",
  defection_risk: "No defection alarms in scope — every cadence is holding.",
  conquest: "No conquest targets in scope — no competitor units on file.",
  discount_anomaly: "No discount anomalies in scope for the recent window.",
};

function scoreLabel(score: number): string {
  return score.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function StatusChip({ signal }: { signal: SignalRow }) {
  const tone =
    signal.status === "open"
      ? "border-data/50 text-data"
      : signal.status === "assigned"
        ? "border-warn/50 text-warn"
        : signal.status === "actioned"
          ? "border-ok/50 text-ok"
          : "border-seam text-text-dim";
  const label =
    signal.status === "assigned" && signal.assignee_name
      ? `assigned · ${signal.assignee_name}`
      : signal.status;
  return (
    <span
      className={`nameplate inline-block max-w-full truncate rounded-sm border px-1.5 py-0.5 text-2xs ${tone}`}
      title={label}
    >
      {label}
    </span>
  );
}

/** Assign: reps self-assign; wider roles pick from the scope-valid roster
 *  (lazily fetched — the R6 picker). */
function AssignControl({
  signal,
  onToast,
}: {
  signal: SignalRow;
  onToast: (msg: string) => void;
}) {
  const me = useMe();
  const assign = useAssignSignal();
  const [open, setOpen] = useState(false);
  const assignees = useAssignees(signal.account_id, open);

  const doAssign = (assigneeId: string, name: string) =>
    assign.mutate(
      { id: signal.id, assignee_id: assigneeId },
      {
        onSuccess: () => {
          setOpen(false);
          onToast(`Assigned to ${name}.`);
        },
        onError: (e) => onToast(e instanceof Error ? e.message : "Could not assign."),
      },
    );

  if (!me.data) return null;
  if (me.data.role === "rep") {
    return (
      <button
        onClick={() => doAssign(me.data!.id, "you")}
        disabled={assign.isPending}
        data-testid="signal-assign-self"
        className="inline-flex items-center gap-1 rounded border border-seam px-2 py-1 text-2xs text-text-dim transition-colors hover:bg-surface hover:text-text disabled:opacity-50"
      >
        <UserRoundPlus size={12} />
        <span className="nameplate">Assign to me</span>
      </button>
    );
  }
  if (!open) {
    return (
      <button
        onClick={() => setOpen(true)}
        data-testid="signal-assign-open"
        className="inline-flex items-center gap-1 rounded border border-seam px-2 py-1 text-2xs text-text-dim transition-colors hover:bg-surface hover:text-text"
      >
        <UserRoundPlus size={12} />
        <span className="nameplate">Assign…</span>
      </button>
    );
  }
  return (
    <select
      autoFocus
      data-testid="signal-assign-select"
      value=""
      onChange={(e) => {
        const pick = assignees.data?.items.find((u) => u.id === e.target.value);
        if (pick) doAssign(pick.id, pick.name);
      }}
      onBlur={() => setOpen(false)}
      className="nameplate rounded border border-seam bg-bg px-2 py-1 text-2xs text-text outline-none focus:border-seam-strong"
    >
      <option value="">
        {assignees.isLoading ? "Loading…" : "Assign to…"}
      </option>
      {(assignees.data?.items ?? []).map((u) => (
        <option key={u.id} value={u.id}>
          {u.name} ({u.role})
        </option>
      ))}
    </select>
  );
}

function DismissDialog({
  signal,
  onConfirm,
  onCancel,
  pending,
}: {
  signal: SignalRow;
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  pending: boolean;
}) {
  const [reason, setReason] = useState("");
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-bg/70 p-4">
      <div className="w-full max-w-md rounded-lg border border-seam bg-surface p-5">
        <h3 className="nameplate-strong text-base text-text">
          Dismiss this {signal.type.replace("_", " ")} signal
        </h3>
        <p className="mt-2 text-sm text-text-dim">
          {signal.account_name} · a reason is required — dismissals are
          captured data.
        </p>
        <input
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="Why is this not actionable?"
          className="mt-3 w-full rounded border border-seam bg-bg px-3 py-2 text-sm text-text outline-none focus:border-seam-strong"
          data-testid="dismiss-reason"
        />
        <div className="mt-4 flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="rounded border border-seam px-3 py-1.5 text-2xs text-text-dim hover:bg-surface-2"
          >
            <span className="nameplate">Cancel</span>
          </button>
          <button
            onClick={() => onConfirm(reason.trim())}
            disabled={pending || !reason.trim()}
            data-testid="dismiss-confirm"
            className="rounded bg-alarm/15 px-3 py-1.5 text-2xs text-alarm hover:bg-alarm/25 disabled:opacity-50"
          >
            <span className="nameplate">Dismiss</span>
          </button>
        </div>
      </div>
    </div>
  );
}

export function Signals() {
  const [filter, setFilter] = useState<QueueFilter>("active");
  const signals = useSignals(filter);
  const summary = useSignalsSummary();
  const opps = useOpportunities("all");
  const createOpp = useCreateOpportunity();
  const patchStage = usePatchStage();
  const createActivity = useCreateActivity();
  const actionSignal = useActionSignal();
  const dismissSignal = useDismissSignal();
  const navigate = useNavigate();

  const [toast, setToast] = useState<string | null>(null);
  const [dismissFor, setDismissFor] = useState<SignalRow | null>(null);
  const [busyCard, setBusyCard] = useState<string | null>(null);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 4000);
    return () => clearTimeout(t);
  }, [toast]);

  const settled =
    (signals.isSuccess || signals.isError) &&
    (summary.isSuccess || summary.isError);
  useScreenReady(settled);

  const byLane = useMemo(() => {
    const map = new Map<SignalType, SignalRow[]>();
    for (const lane of LANES) map.set(lane.type, []);
    for (const row of signals.data?.items ?? []) {
      map.get(row.type)?.push(row);
    }
    return map;
  }, [signals.data]);

  // ── Draft-quote-from-signal (R6): client composition of P3 machinery ──────
  const draftQuote = async (signal: SignalRow) => {
    if (!signal.cartridge_product_id) {
      setToast("This signal has no cartridge to quote.");
      return;
    }
    setBusyCard(signal.id);
    try {
      // The account's open opp: stage ∉ won/lost; several → highest amount,
      // then lowest id. (The Ridgeline case has exactly one — the win-back.)
      const open = (opps.data?.items ?? [])
        .filter(
          (o) =>
            o.account_id === signal.account_id &&
            o.stage !== "won" &&
            o.stage !== "lost",
        )
        .sort(
          (a, b) => b.amount_cents - a.amount_cents || a.id.localeCompare(b.id),
        );
      let oppId = open[0]?.id;
      if (!oppId) {
        // None exists: create one (filter-program, amount = the signal's
        // annual value, owner = caller via the API default), then move it to
        // qualified — the two P3 endpoints, composed.
        const created = await createOpp.mutateAsync({
          account_id: signal.account_id,
          kind: "filter-program",
          amount_cents: signal.annual_value_cents ?? 0,
        });
        await patchStage.mutateAsync({ oppId: created.id, stage: "qualified" });
        oppId = created.id;
      }
      const params = new URLSearchParams({
        opp: oppId,
        product: signal.cartridge_product_id,
        qty: String(signal.cartridge_count ?? 1),
        signal: signal.id,
      });
      navigate(`/quotes/new?${params.toString()}`);
    } catch (e) {
      setToast(e instanceof Error ? e.message : "Could not start the quote.");
    } finally {
      setBusyCard(null);
    }
  };

  // ── Log Call: the existing activities write + an action write-back ────────
  const logCall = (signal: SignalRow) => {
    setBusyCard(signal.id);
    createActivity.mutate(
      {
        account_id: signal.account_id,
        kind: "call",
        body: `Follow-up call on the ${signal.type.replace("_", " ")} signal (${
          signal.serial ?? signal.account_name
        }).`,
      },
      {
        onSuccess: () =>
          actionSignal.mutate(
            { id: signal.id, outcome: "call_logged" },
            {
              onSuccess: () => {
                setBusyCard(null);
                setToast(`Call logged on ${signal.account_name}.`);
              },
              onError: () => {
                setBusyCard(null);
                setToast("Call logged, but the signal did not update.");
              },
            },
          ),
        onError: (e) => {
          setBusyCard(null);
          setToast(e instanceof Error ? e.message : "Could not log the call.");
        },
      },
    );
  };

  const confirmDismiss = (signal: SignalRow, reason: string) =>
    dismissSignal.mutate(
      { id: signal.id, reason },
      {
        onSuccess: () => {
          setDismissFor(null);
          setToast(`Dismissed — ${signal.account_name}.`);
        },
        onError: (e) => {
          setDismissFor(null);
          setToast(e instanceof Error ? e.message : "Could not dismiss.");
        },
      },
    );

  if (signals.isLoading) return <LoadingPanel label="Loading signals" />;
  if (signals.isError)
    return (
      <div className="mx-auto max-w-[1600px]">
        <ErrorPanel onRetry={() => signals.refetch()} />
      </div>
    );

  const active = filter === "active";
  const total = signals.data?.total ?? 0;

  return (
    <div className="mx-auto max-w-[1800px] space-y-4">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="nameplate-strong text-xl text-text">Signals</h1>
          <div className="nameplate text-2xs text-text-dim">
            {count(total)} {filter} · derived from the installed base, never
            hand-entered
          </div>
        </div>
        <Segmented
          value={filter}
          onChange={setFilter}
          ariaLabel="Signal status filter"
          testid="signals-filter"
          options={FILTERS}
        />
      </header>

      <div className="grid grid-cols-1 gap-3 min-[760px]:grid-cols-2 min-[1280px]:grid-cols-4">
        {LANES.map((lane) => {
          const rows = byLane.get(lane.type) ?? [];
          return (
            <section
              key={lane.type}
              data-testid={`lane-${lane.type}`}
              className="flex min-w-0 flex-col rounded-lg border border-seam bg-surface/60"
            >
              <div className="flex items-center justify-between border-b border-seam px-3 py-2">
                <span className="nameplate text-2xs text-text-dim">
                  {lane.label}
                </span>
                <span className="tabular text-2xs text-text-dim">
                  {rows.length}
                </span>
              </div>
              <div className="flex flex-col gap-2 p-2">
                {rows.length === 0 ? (
                  <div className="px-2 py-6 text-center text-2xs text-text-dim">
                    {active
                      ? EMPTY_LANE[lane.type]
                      : `No ${filter} ${lane.label.toLowerCase()} signals.`}
                  </div>
                ) : (
                  rows.map((s) => (
                    <article
                      key={s.id}
                      data-testid="signal-card"
                      data-signal-type={s.type}
                      className="rounded-lg border border-seam bg-surface-2 p-3"
                    >
                      <div className="flex items-start justify-between gap-2">
                        <Link
                          to={`/accounts/${s.account_id}`}
                          className="min-w-0 truncate text-sm text-text hover:text-data"
                        >
                          {s.account_name}
                        </Link>
                        <span
                          className="tabular shrink-0 text-sm text-warn"
                          title="signal score"
                        >
                          {scoreLabel(s.score)}
                        </span>
                      </div>
                      <div className="mt-0.5 flex items-center gap-2 text-2xs text-text-dim">
                        <span className="nameplate rounded-sm border border-seam px-1 py-0.5">
                          {s.territory_code}
                        </span>
                        <span className="min-w-0 truncate">
                          {s.serial
                            ? `${s.serial} · ${s.site_label ?? ""}`
                            : (s.site_label ?? "")}
                        </span>
                      </div>

                      <ul className="mt-2 space-y-1 border-t border-seam/60 pt-2">
                        {s.reasons.map((r, i) => (
                          <li key={i} className="flex gap-2 text-2xs">
                            <span className="nameplate shrink-0 text-text-dim">
                              {r.label}
                            </span>
                            <span className="min-w-0 flex-1 text-right text-text">
                              {r.detail}
                            </span>
                          </li>
                        ))}
                      </ul>

                      <div className="mt-2 flex flex-wrap items-center gap-1.5 border-t border-seam/60 pt-2">
                        <StatusChip signal={s} />
                        {s.status === "actioned" && s.outcome && (
                          <span
                            className="min-w-0 truncate text-2xs text-text-dim"
                            title={s.outcome}
                          >
                            {s.outcome}
                          </span>
                        )}
                        {s.status === "dismissed" && s.dismissed_reason && (
                          <span
                            className="min-w-0 truncate text-2xs text-text-dim"
                            title={s.dismissed_reason}
                          >
                            {s.dismissed_reason}
                          </span>
                        )}
                      </div>

                      {(s.status === "open" || s.status === "assigned") && (
                        <div className="mt-2 flex flex-wrap items-center gap-1.5">
                          <AssignControl signal={s} onToast={setToast} />
                          {s.type !== "discount_anomaly" && (
                            <button
                              onClick={() => draftQuote(s)}
                              disabled={busyCard === s.id}
                              data-testid="signal-draft-quote"
                              className="inline-flex items-center gap-1 rounded border border-seam px-2 py-1 text-2xs text-data transition-colors hover:bg-surface disabled:opacity-50"
                            >
                              <FilePlus2 size={12} />
                              <span className="nameplate">Draft quote</span>
                            </button>
                          )}
                          <button
                            onClick={() => logCall(s)}
                            disabled={busyCard === s.id}
                            data-testid="signal-log-call"
                            className="inline-flex items-center gap-1 rounded border border-seam px-2 py-1 text-2xs text-text-dim transition-colors hover:bg-surface hover:text-text disabled:opacity-50"
                          >
                            <PhoneCall size={12} />
                            <span className="nameplate">Log call</span>
                          </button>
                          <button
                            onClick={() => setDismissFor(s)}
                            data-testid="signal-dismiss"
                            className="ml-auto inline-flex items-center gap-1 rounded border border-seam px-2 py-1 text-2xs text-text-dim transition-colors hover:text-alarm"
                          >
                            <XCircle size={12} />
                            <span className="nameplate">Dismiss</span>
                          </button>
                        </div>
                      )}
                      {s.annual_value_cents !== null &&
                        s.annual_value_cents > 0 && (
                          <div className="mt-1.5 text-right text-2xs text-text-dim">
                            annuity{" "}
                            <span className="tabular text-text">
                              {money(s.annual_value_cents)}
                            </span>
                            /yr
                          </div>
                        )}
                    </article>
                  ))
                )}
              </div>
            </section>
          );
        })}
      </div>

      {toast && (
        <div
          data-testid="toast"
          className="fixed bottom-4 left-1/2 z-50 -translate-x-1/2 rounded-lg border border-seam-strong bg-surface-2 px-4 py-2.5 text-sm text-text shadow-lg"
        >
          {toast}
        </div>
      )}

      {dismissFor && (
        <DismissDialog
          signal={dismissFor}
          pending={dismissSignal.isPending}
          onConfirm={(reason) => confirmDismiss(dismissFor, reason)}
          onCancel={() => setDismissFor(null)}
        />
      )}
    </div>
  );
}
