// The installed-base timeline — Account 360's hero (spec §8). Bespoke DOM, no
// chart library (recharts stays idle per the design doctrine). Each unit is a
// life-axis: commissioned → now, with a change-out DUE marker derived from
// last_filter_order_on + expected_changeout_months. Overdue units carry the
// alarm token; NULL-cadence units carry a "cadence unknown" chip (the seeded
// data-quality beat, RENDERED — never an invented date, never a crash);
// competitor-source units are visually distinct (conquest fuel, P4 acts on it).

import type { UnitTimeline } from "../lib/types";

function parseDate(s: string): Date {
  return new Date(s + "T00:00:00Z");
}
function addMonths(d: Date, m: number): Date {
  const x = new Date(d);
  x.setUTCMonth(x.getUTCMonth() + m);
  return x;
}
function clampPct(d: Date, start: number, end: number): number {
  const span = end - start;
  if (span <= 0) return 0;
  return Math.max(0, Math.min(100, ((d.getTime() - start) / span) * 100));
}
function yearOf(t: number): number {
  return new Date(t).getUTCFullYear();
}

interface UnitState {
  unit: UnitTimeline;
  commissioned: Date;
  due: Date | null;
  overdue: boolean;
  isCompetitor: boolean;
  cadenceUnknown: boolean;
  noCartridge: boolean;
}

function unitState(u: UnitTimeline, now: Date): UnitState {
  const commissioned = parseDate(u.commissioned_on);
  const isCompetitor = u.source !== "ours";
  const noCartridge = u.cartridge_name === null;
  const cadenceUnknown =
    u.expected_changeout_months === null && !noCartridge;

  let due: Date | null = null;
  let overdue = false;
  if (u.expected_changeout_months !== null && !noCartridge) {
    const base = u.last_filter_order_on
      ? parseDate(u.last_filter_order_on)
      : commissioned;
    due = addMonths(base, u.expected_changeout_months);
    overdue = due.getTime() < now.getTime();
  }
  return {
    unit: u,
    commissioned,
    due,
    overdue,
    isCompetitor,
    cadenceUnknown,
    noCartridge,
  };
}

function Chip({
  label,
  tone,
  title,
}: {
  label: string;
  tone: "dim" | "warn" | "alarm" | "data";
  title?: string;
}) {
  const cls = {
    dim: "border-seam text-text-dim",
    warn: "border-warn/50 text-warn",
    alarm: "border-alarm/50 text-alarm",
    data: "border-data/50 text-data",
  }[tone];
  return (
    <span
      title={title}
      className={`nameplate rounded-sm border px-1.5 py-0.5 text-[0.6rem] ${cls}`}
    >
      {label}
    </span>
  );
}

export function Timeline({ units }: { units: UnitTimeline[] }) {
  if (units.length === 0) {
    return (
      <div className="rounded-lg border border-seam bg-surface p-6 text-xs text-text-dim">
        No installed units on file for this account.
      </div>
    );
  }

  const now = new Date();
  const states = units.map((u) => unitState(u, now));

  // Axis domain: earliest commission → now (plus a small headroom).
  const starts = states.map((s) => s.commissioned.getTime());
  const axisStart = Math.min(...starts);
  const axisEnd = now.getTime();

  // Year gridlines across the span.
  const years: number[] = [];
  for (let y = yearOf(axisStart); y <= yearOf(axisEnd); y++) years.push(y);

  return (
    <div className="rounded-lg border border-seam bg-surface p-4">
      <div className="mb-3 flex items-center justify-between">
        <span className="nameplate text-2xs text-text-dim">
          Installed-base timeline · commissioned → change-out due
        </span>
        <div className="hidden items-center gap-3 sm:flex">
          <span className="flex items-center gap-1 text-[0.6rem] text-text-dim">
            <span className="inline-block h-2 w-2 rounded-full bg-data" /> due
          </span>
          <span className="flex items-center gap-1 text-[0.6rem] text-text-dim">
            <span className="inline-block h-2 w-2 rounded-full bg-alarm" /> overdue
          </span>
        </div>
      </div>

      <div className="space-y-2.5">
        {states.map((s) => {
          const commPct = clampPct(s.commissioned, axisStart, axisEnd);
          const duePct = s.due ? clampPct(s.due, axisStart, axisEnd) : null;
          return (
            <div
              key={s.unit.id}
              className="grid grid-cols-1 gap-2 sm:grid-cols-[minmax(9rem,14rem)_1fr] sm:items-center"
            >
              {/* label */}
              <div className="min-w-0">
                <div className="flex items-center gap-1.5">
                  <span className="nameplate truncate text-xs text-text">
                    {s.unit.family}
                  </span>
                  <span className="tabular truncate text-2xs text-text-dim">
                    {s.unit.serial}
                  </span>
                </div>
                <div className="mt-0.5 flex flex-wrap items-center gap-1">
                  {s.isCompetitor ? (
                    <Chip
                      label={s.unit.source}
                      tone="warn"
                      title="competitor unit — conquest target"
                    />
                  ) : (
                    <span className="truncate text-2xs text-text-dim">
                      {s.unit.cartridge_name ?? "—"}
                    </span>
                  )}
                  {s.cadenceUnknown && (
                    <Chip
                      label="cadence unknown"
                      tone="dim"
                      title="expected_changeout_months not on file — cadence math cannot run"
                    />
                  )}
                  {s.overdue && <Chip label="overdue" tone="alarm" />}
                </div>
              </div>

              {/* track */}
              <div className="relative h-6">
                <div className="absolute inset-x-0 top-1/2 h-px -translate-y-1/2 bg-seam" />
                {/* life bar: commissioned → now */}
                <div
                  className={`absolute top-1/2 h-1.5 -translate-y-1/2 rounded-full ${
                    s.isCompetitor ? "bg-warn/40" : "bg-seam-strong"
                  }`}
                  style={{ left: `${commPct}%`, right: "0%" }}
                  title={`commissioned ${s.unit.commissioned_on}`}
                />
                {/* due marker */}
                {duePct !== null && (
                  <div
                    className={`absolute top-1/2 h-3.5 w-1 -translate-x-1/2 -translate-y-1/2 rounded-sm ${
                      s.overdue ? "bg-alarm" : "bg-data"
                    }`}
                    style={{ left: `${duePct}%` }}
                    title={
                      (s.overdue ? "overdue since " : "change-out due ") +
                      (s.due ? s.due.toISOString().slice(0, 10) : "")
                    }
                  />
                )}
                {/* now marker */}
                <div
                  className="absolute top-1/2 h-4 w-px -translate-y-1/2 bg-text-dim/60"
                  style={{ left: "100%" }}
                />
              </div>
            </div>
          );
        })}
      </div>

      {/* year axis */}
      <div className="relative mt-2 hidden h-4 sm:block">
        {years.map((y) => {
          const p = clampPct(
            new Date(Date.UTC(y, 0, 1)),
            axisStart,
            axisEnd,
          );
          return (
            <span
              key={y}
              className="tabular absolute -translate-x-1/2 text-[0.6rem] text-text-dim"
              style={{ left: `${p}%` }}
            >
              {y}
            </span>
          );
        })}
      </div>
    </div>
  );
}
