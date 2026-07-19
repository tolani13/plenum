// A segmented control — the house control-room switch. One active segment
// (surface step + air text), the rest dim. Used for basis, kind, group, tabs.

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  title?: string;
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  ariaLabel,
  testid,
}: {
  options: ReadonlyArray<SegmentOption<T>>;
  value: T;
  onChange: (value: T) => void;
  ariaLabel: string;
  testid?: string;
}) {
  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      data-testid={testid}
      className="inline-flex overflow-hidden rounded border border-seam bg-surface"
    >
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <button
            key={opt.value}
            role="tab"
            aria-selected={active}
            title={opt.title}
            onClick={() => onChange(opt.value)}
            data-testid={testid ? `${testid}-${opt.value}` : undefined}
            className={[
              "nameplate px-3 py-1.5 text-2xs transition-colors",
              active
                ? "bg-surface-2 text-text"
                : "text-text-dim hover:text-text",
            ].join(" ")}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
