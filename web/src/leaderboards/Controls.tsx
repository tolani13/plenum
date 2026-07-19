// Leaderboards controls: the tab rail (reps | items | customers), the period
// scrubber (year rail 2023–2026 · Q1–Q4 · CUMULATIVE · TTM, exclusive), the
// shared GROSS/NET toggle, the kind filter, and the items-only group toggle.
// Every change is a URL patch (resolution 6) — refresh-safe and deep-linkable.

import { Segmented } from "../components/Segmented";
import { BasisToggle } from "../components/BasisToggle";
import {
  CURRENT_YEAR,
  PERIOD_YEARS,
  QUARTERS,
  type Basis,
  type Group,
  type Kind,
  type Tab,
} from "../lib/params";

const TABS: ReadonlyArray<{ value: Tab; label: string }> = [
  { value: "reps", label: "Reps" },
  { value: "items", label: "Items" },
  { value: "customers", label: "Customers" },
];

function ScrubBtn({
  active,
  onClick,
  children,
  testid,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
  testid?: string;
}) {
  return (
    <button
      onClick={onClick}
      data-testid={testid}
      aria-pressed={active}
      className={`nameplate px-2.5 py-1.5 text-2xs transition-colors ${
        active ? "bg-surface-2 text-text" : "text-text-dim hover:text-text"
      }`}
    >
      {children}
    </button>
  );
}

function Group_({ children }: { children: React.ReactNode }) {
  return (
    <div className="inline-flex overflow-hidden rounded border border-seam bg-surface">
      {children}
    </div>
  );
}

export interface ControlsState {
  tab: Tab;
  period: string;
  basis: Basis;
  kind: Kind;
  group: Group;
}

export function Controls({
  state,
  onChange,
}: {
  state: ControlsState;
  onChange: (patch: Partial<ControlsState>) => void;
}) {
  const { tab, period, basis, kind, group } = state;

  const q = period.match(/^(\d{4})-q([1-4])$/);
  const activeYear = q
    ? Number(q[1])
    : /^\d{4}$/.test(period)
      ? Number(period)
      : CURRENT_YEAR;
  const activeQuarter = q ? Number(q[2]) : null;
  const yearActive = /^\d{4}$/.test(period);

  return (
    <div className="space-y-3">
      {/* tab rail */}
      <div
        role="tablist"
        aria-label="Leaderboard"
        className="flex gap-1 border-b border-seam"
        data-testid="tab-rail"
      >
        {TABS.map((t) => {
          const active = t.value === tab;
          return (
            <button
              key={t.value}
              role="tab"
              aria-selected={active}
              onClick={() => onChange({ tab: t.value })}
              data-testid={`tab-${t.value}`}
              className={`nameplate -mb-px border-b-2 px-3 py-2 text-2xs transition-colors ${
                active
                  ? "border-data text-text"
                  : "border-transparent text-text-dim hover:text-text"
              }`}
            >
              {t.label}
            </button>
          );
        })}
      </div>

      {/* control row */}
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <div className="flex flex-wrap items-center gap-1.5" data-testid="period-scrubber">
          <Group_>
            {PERIOD_YEARS.map((y) => (
              <ScrubBtn
                key={y}
                active={yearActive && Number(period) === y}
                onClick={() => onChange({ period: String(y) })}
                testid={`period-year-${y}`}
              >
                {y}
              </ScrubBtn>
            ))}
          </Group_>
          <Group_>
            {QUARTERS.map((n) => (
              <ScrubBtn
                key={n}
                active={activeQuarter === n}
                onClick={() => onChange({ period: `${activeYear}-q${n}` })}
                testid={`period-q${n}`}
              >
                Q{n}
              </ScrubBtn>
            ))}
          </Group_>
          <Group_>
            <ScrubBtn
              active={period === "cumulative"}
              onClick={() => onChange({ period: "cumulative" })}
              testid="period-cumulative"
            >
              Cum
            </ScrubBtn>
            <ScrubBtn
              active={period === "ttm"}
              onClick={() => onChange({ period: "ttm" })}
              testid="period-ttm"
            >
              TTM
            </ScrubBtn>
          </Group_>
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          <BasisToggle value={basis} onChange={(b) => onChange({ basis: b })} />
          <Segmented
            ariaLabel="Kind"
            testid="kind-filter"
            value={kind}
            onChange={(k) => onChange({ kind: k })}
            options={[
              { value: "all", label: "All" },
              { value: "capital", label: "Capital" },
              { value: "consumable", label: "Consumable" },
            ]}
          />
          {tab === "items" && (
            <Segmented
              ariaLabel="Group"
              testid="group-toggle"
              value={group}
              onChange={(g) => onChange({ group: g })}
              options={[
                { value: "product", label: "Product" },
                { value: "family", label: "Family" },
              ]}
            />
          )}
        </div>
      </div>
    </div>
  );
}
