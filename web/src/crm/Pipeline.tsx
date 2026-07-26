// Pipeline (spec §8 screen 6) — six fixed lanes of §8 cards with stage
// write-back. Native HTML5 drag-and-drop PLUS a per-card "Move to…" select
// (the non-drag fallback for touch/keyboard — no drag library, ruling
// constraint 2). Dropping on Won opens a confirm dialog naming the approved
// quote that books; a successful booking toasts the order. Territory-scoped by
// RLS: a rep sees only their own cards.

import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router";
import { GripVertical, FilePlus2 } from "lucide-react";
import { money } from "../lib/format";
import type { OppRow, OppStage } from "../lib/types";
import { useOpportunities, usePatchStage } from "../lib/crm";
import { useScreenReady } from "../lib/useScreenReady";
import { ErrorPanel, LoadingPanel } from "../components/states";
import { StageChip } from "./badges";

const LANES: { stage: OppStage; label: string }[] = [
  { stage: "lead", label: "Lead" },
  { stage: "qualified", label: "Qualified" },
  { stage: "quoted", label: "Quoted" },
  { stage: "negotiation", label: "Negotiation" },
  { stage: "won", label: "Won" },
  { stage: "lost", label: "Lost" },
];

const MOVE_TARGETS: OppStage[] = [
  "lead",
  "qualified",
  "quoted",
  "negotiation",
  "won",
  "lost",
];

function Card({
  opp,
  onDragStart,
  onDragEnd,
  onMove,
}: {
  opp: OppRow;
  onDragStart: () => void;
  onDragEnd: () => void;
  onMove: (target: OppStage) => void;
}) {
  const terminal = opp.stage === "won" || opp.stage === "lost";
  return (
    <div
      draggable={!terminal}
      onDragStart={(e) => {
        e.dataTransfer.setData("text/plain", opp.id);
        e.dataTransfer.effectAllowed = "move";
        onDragStart();
      }}
      onDragEnd={onDragEnd}
      data-testid="pipeline-card"
      data-account={opp.account_name}
      className="rounded-lg border border-seam bg-surface-2 p-3 transition-colors hover:border-seam-strong"
    >
      <div className="flex items-start justify-between gap-2">
        <Link
          to={`/accounts/${opp.account_id}`}
          className="min-w-0 truncate text-sm text-text hover:text-data"
        >
          {opp.account_name}
        </Link>
        {!terminal && (
          <GripVertical
            size={13}
            className="mt-0.5 shrink-0 cursor-grab text-text-dim"
          />
        )}
      </div>
      <div className="mt-1 flex items-center gap-2">
        <span className="nameplate text-2xs text-text-dim">{opp.kind}</span>
        <span className="tabular ml-auto text-sm text-text">
          {money(opp.amount_cents)}
        </span>
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-1.5">
        <span className="nameplate rounded-sm border border-seam px-1.5 py-0.5 text-2xs text-text-dim">
          {opp.owner_name}
        </span>
        {opp.has_approved_quote && (
          <span
            className="nameplate rounded-sm border border-ok/50 px-1.5 py-0.5 text-2xs text-ok"
            title={`approved quote ready to book${
              opp.approved_quote_net_cents !== null
                ? ` (${money(opp.approved_quote_net_cents)} net)`
                : ""
            }`}
            data-testid="armed-badge"
          >
            ✓ quote
          </span>
        )}
      </div>

      {!terminal && (
        <div className="mt-2 flex items-center gap-2">
          <Link
            to={`/quotes/new?opp=${opp.id}`}
            className="inline-flex items-center gap-1 rounded border border-seam px-2 py-1 text-2xs text-data transition-colors hover:bg-surface"
            data-testid="draft-quote"
          >
            <FilePlus2 size={12} />
            <span className="nameplate">Draft quote</span>
          </Link>
          <select
            aria-label="Move to stage"
            data-testid="move-to"
            value=""
            onChange={(e) => {
              if (e.target.value) onMove(e.target.value as OppStage);
              e.currentTarget.value = "";
            }}
            className="nameplate ml-auto rounded border border-seam bg-bg px-2 py-1 text-2xs text-text-dim outline-none focus:border-seam-strong"
          >
            <option value="">Move to…</option>
            {MOVE_TARGETS.filter((s) => s !== opp.stage).map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </div>
      )}
    </div>
  );
}

function WonDialog({
  opp,
  onConfirm,
  onCancel,
  pending,
}: {
  opp: OppRow;
  onConfirm: () => void;
  onCancel: () => void;
  pending: boolean;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-bg/70 p-4">
      <div className="w-full max-w-md rounded-lg border border-seam bg-surface p-5">
        <h3 className="nameplate-strong text-base text-text">Book the order</h3>
        <p className="mt-2 text-sm text-text-dim">
          Winning <span className="text-text">{opp.account_name}</span> books a
          real order from its approved quote
          {opp.approved_quote_net_cents !== null && (
            <>
              {" "}
              (<span className="tabular text-text">
                {money(opp.approved_quote_net_cents)}
              </span>{" "}
              net)
            </>
          )}
          . The quote becomes <span className="text-text">accepted</span> and the
          leaderboard moves immediately.
        </p>
        <div className="mt-4 flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="rounded border border-seam px-3 py-1.5 text-2xs text-text-dim hover:bg-surface-2"
          >
            <span className="nameplate">Cancel</span>
          </button>
          <button
            onClick={onConfirm}
            disabled={pending}
            data-testid="confirm-won"
            className="rounded bg-ok/15 px-3 py-1.5 text-2xs text-ok hover:bg-ok/25 disabled:opacity-50"
          >
            <span className="nameplate">{pending ? "Booking…" : "Book order"}</span>
          </button>
        </div>
      </div>
    </div>
  );
}

function LostDialog({
  opp,
  onConfirm,
  onCancel,
  pending,
}: {
  opp: OppRow;
  onConfirm: (reason: string) => void;
  onCancel: () => void;
  pending: boolean;
}) {
  const [reason, setReason] = useState("");
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-bg/70 p-4">
      <div className="w-full max-w-md rounded-lg border border-seam bg-surface p-5">
        <h3 className="nameplate-strong text-base text-text">
          Mark {opp.account_name} lost
        </h3>
        <p className="mt-2 text-sm text-text-dim">A reason is required.</p>
        <input
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          placeholder="Why was this lost?"
          className="mt-3 w-full rounded border border-seam bg-bg px-3 py-2 text-sm text-text outline-none focus:border-seam-strong"
          data-testid="lost-reason"
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
            data-testid="confirm-lost"
            className="rounded bg-alarm/15 px-3 py-1.5 text-2xs text-alarm hover:bg-alarm/25 disabled:opacity-50"
          >
            <span className="nameplate">Mark lost</span>
          </button>
        </div>
      </div>
    </div>
  );
}

export function Pipeline() {
  const opps = useOpportunities("all");
  const patch = usePatchStage();
  const navigate = useNavigate();
  useScreenReady(opps.isSuccess || opps.isError, "pipeline");

  const [dragging, setDragging] = useState<OppRow | null>(null);
  const [overLane, setOverLane] = useState<OppStage | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [wonFor, setWonFor] = useState<OppRow | null>(null);
  const [lostFor, setLostFor] = useState<OppRow | null>(null);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 4000);
    return () => clearTimeout(t);
  }, [toast]);

  const move = (opp: OppRow, target: OppStage) => {
    if (target === opp.stage) return;
    if (target === "won") {
      if (!opp.has_approved_quote) {
        setToast("This deal needs an approved quote before it can be won.");
        return;
      }
      setWonFor(opp);
      return;
    }
    if (target === "lost") {
      setLostFor(opp);
      return;
    }
    patch.mutate(
      { oppId: opp.id, stage: target },
      {
        onSuccess: () => setToast(`Moved ${opp.account_name} → ${target}.`),
        onError: () => setToast("Could not move that deal."),
      },
    );
  };

  const confirmWon = (opp: OppRow) =>
    patch.mutate(
      { oppId: opp.id, stage: "won" },
      {
        onSuccess: (res) => {
          setWonFor(null);
          setToast(
            `Order booked — ${money(res.booked_order?.net_cents ?? 0)} net · ${opp.account_name}`,
          );
        },
        onError: () => {
          setWonFor(null);
          setToast("Could not book the order.");
        },
      },
    );

  const confirmLost = (opp: OppRow, reason: string) =>
    patch.mutate(
      { oppId: opp.id, stage: "lost", lost_reason: reason },
      {
        onSuccess: () => {
          setLostFor(null);
          setToast(`Marked ${opp.account_name} lost.`);
        },
        onError: () => setToast("Could not update that deal."),
      },
    );

  if (opps.isLoading) return <LoadingPanel label="Loading pipeline" />;
  if (opps.isError || !opps.data)
    return (
      <div className="mx-auto max-w-[1600px]">
        <ErrorPanel onRetry={() => opps.refetch()} />
      </div>
    );

  const rows = opps.data.items;

  return (
    <div className="mx-auto max-w-[1800px] space-y-4">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="nameplate-strong text-xl text-text">Pipeline</h1>
          <div className="nameplate text-2xs text-text-dim">
            Drag a card, or use “Move to…” · Won books an order from its approved quote
          </div>
        </div>
        <button
          onClick={() => navigate(0)}
          className="hidden rounded border border-seam px-2.5 py-1.5 text-2xs text-text-dim hover:bg-surface-2 sm:block"
        >
          <span className="nameplate">Refresh</span>
        </button>
      </header>

      <div className="scroll-x pb-2">
        <div className="flex min-w-max gap-3">
          {LANES.map((lane) => {
            const laneOpps = rows.filter((o) => o.stage === lane.stage);
            return (
              <div
                key={lane.stage}
                data-testid="pipeline-lane"
                data-stage={lane.stage}
                onDragOver={(e) => {
                  e.preventDefault();
                  setOverLane(lane.stage);
                }}
                onDragLeave={() => setOverLane(null)}
                onDrop={(e) => {
                  e.preventDefault();
                  setOverLane(null);
                  if (dragging) move(dragging, lane.stage);
                  setDragging(null);
                }}
                className={`flex w-[15rem] shrink-0 flex-col rounded-lg border bg-surface/60 ${
                  overLane === lane.stage ? "border-seam-strong" : "border-seam"
                }`}
              >
                <div className="flex items-center justify-between border-b border-seam px-3 py-2">
                  <div className="flex items-center gap-2">
                    <StageChip stage={lane.stage} />
                  </div>
                  <span className="tabular text-2xs text-text-dim">
                    {laneOpps.length}
                  </span>
                </div>
                <div className="flex flex-col gap-2 p-2">
                  {laneOpps.length === 0 ? (
                    <div className="px-1 py-4 text-center text-2xs text-text-dim">
                      —
                    </div>
                  ) : (
                    laneOpps.map((opp) => (
                      <Card
                        key={opp.id}
                        opp={opp}
                        onDragStart={() => setDragging(opp)}
                        onDragEnd={() => setDragging(null)}
                        onMove={(target) => move(opp, target)}
                      />
                    ))
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {toast && (
        <div
          data-testid="toast"
          className="fixed bottom-4 left-1/2 z-50 -translate-x-1/2 rounded-lg border border-seam-strong bg-surface-2 px-4 py-2.5 text-sm text-text shadow-lg"
        >
          {toast}
        </div>
      )}

      {wonFor && (
        <WonDialog
          opp={wonFor}
          pending={patch.isPending}
          onConfirm={() => confirmWon(wonFor)}
          onCancel={() => setWonFor(null)}
        />
      )}
      {lostFor && (
        <LostDialog
          opp={lostFor}
          pending={patch.isPending}
          onConfirm={(reason) => confirmLost(lostFor, reason)}
          onCancel={() => setLostFor(null)}
        />
      )}
    </div>
  );
}
