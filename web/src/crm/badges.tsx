// Shared nameplate badges for stages and quote statuses. Tokens only — the
// tone maps to §8 state semantics (green = settled/won, amber = pending/action,
// alarm = lost/rejected, data = in-flight, dim = neutral).

import type { OppStage, QuoteStatus } from "../lib/types";

type Tone = "dim" | "ok" | "warn" | "alarm" | "data";

const TONE_CLASS: Record<Tone, string> = {
  dim: "border-seam text-text-dim",
  ok: "border-ok/50 text-ok",
  warn: "border-warn/50 text-warn",
  alarm: "border-alarm/50 text-alarm",
  data: "border-data/50 text-data",
};

function Badge({ label, tone }: { label: string; tone: Tone }) {
  return (
    <span
      className={`nameplate inline-block rounded-sm border px-1.5 py-0.5 text-2xs ${TONE_CLASS[tone]}`}
    >
      {label}
    </span>
  );
}

const STAGE_TONE: Record<OppStage, Tone> = {
  lead: "dim",
  qualified: "data",
  quoted: "data",
  negotiation: "warn",
  won: "ok",
  lost: "alarm",
};

export function StageChip({ stage }: { stage: OppStage }) {
  return <Badge label={stage} tone={STAGE_TONE[stage]} />;
}

const QUOTE_TONE: Record<QuoteStatus, Tone> = {
  draft: "dim",
  pending_approval: "warn",
  approved: "ok",
  sent: "data",
  accepted: "ok",
  rejected: "alarm",
};

const QUOTE_LABEL: Record<QuoteStatus, string> = {
  draft: "Draft",
  pending_approval: "Pending approval",
  approved: "Approved",
  sent: "Sent",
  accepted: "Accepted",
  rejected: "Rejected",
};

export function QuoteStatusChip({ status }: { status: QuoteStatus }) {
  return <Badge label={QUOTE_LABEL[status]} tone={QUOTE_TONE[status]} />;
}
