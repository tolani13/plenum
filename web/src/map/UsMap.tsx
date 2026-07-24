// The map renderer (R3): the CC0 US-states geometry + two schematic Canada
// blocks, projected in the Territory Board's visual language — graphite
// ocean, token-colored graticule, panel-toned landmass, seam strokes,
// territory fills from the eight P5 tokens. Pure presentation: which
// territory a state belongs to, which territories are in the caller's
// scope, and what a hover should SAY all arrive as props. No fetches here.
//
// Scope honesty is structural: dollar strings are only ever placed inside
// elements whose data-territory is an IN-SCOPE territory (the tooltip line
// for foreign states carries no money at all) — the tripwire's map-scope
// assertion greps the DOM for exactly that.

import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { DC_CALLOUT, INSET_SEPARATORS, US_STATES } from "./usStates";

/** Territory fill tokens (tokens.css — the only palette source). */
const TERRITORY_FILL: Record<string, string> = {
  "NE-1": "var(--color-terr-ne1)",
  "SE-1": "var(--color-terr-se1)",
  "MW-1": "var(--color-terr-mw1)",
  "SC-1": "var(--color-terr-sc1)",
  "MT-1": "var(--color-terr-mt1)",
  "W-1": "var(--color-terr-w1)",
  "CE-1": "var(--color-terr-ce1)",
  "CW-1": "var(--color-terr-cw1)",
};

/** T1 planning palette: token name → tokens.css entry. LITERAL var() names
 *  on purpose — Tailwind v4 prunes @theme variables that never appear
 *  verbatim in scanned source, so a constructed `var(--color-${token})`
 *  would leave every planning token undefined at runtime. */
const PLANNING_FILL: Record<string, string> = {
  "terr-plan-1": "var(--color-terr-plan-1)",
  "terr-plan-2": "var(--color-terr-plan-2)",
  "terr-plan-3": "var(--color-terr-plan-3)",
  "terr-plan-4": "var(--color-terr-plan-4)",
  "terr-plan-5": "var(--color-terr-plan-5)",
  "terr-plan-6": "var(--color-terr-plan-6)",
  "terr-plan-7": "var(--color-terr-plan-7)",
  "terr-plan-8": "var(--color-terr-plan-8)",
};

export function planningFill(token: string): string {
  return PLANNING_FILL[token] ?? "var(--color-land-dim)";
}

const PLANNING_POOL = Object.keys(PLANNING_FILL);

/** T1 (D5) — THE one fill resolution, built from the territory config the
 *  caller's payload carries (states-feed roster or the admin list). Rules,
 *  in order: an API color_token wins when present; the canonical P5 mapping
 *  otherwise; a runtime territory with no token gets a deterministic pool
 *  chip (its position in the sorted list of such codes → pool index).
 *  Legend, editor, panel, tooltip, and map ALL render through the resolver
 *  this returns — one function, no drift. */
export function makeTerritoryFill(
  territories: readonly { territory_code: string; color_token: string | null }[],
): (code: string) => string {
  const tokened = new Map<string, string>();
  const untokenedRuntime: string[] = [];
  for (const t of territories) {
    if (t.color_token) {
      tokened.set(t.territory_code, t.color_token);
    } else if (!(t.territory_code in TERRITORY_FILL)) {
      untokenedRuntime.push(t.territory_code);
    }
  }
  untokenedRuntime.sort();
  return (code: string): string => {
    const token = tokened.get(code);
    if (token) return planningFill(token);
    if (TERRITORY_FILL[code]) return TERRITORY_FILL[code];
    const i = untokenedRuntime.indexOf(code);
    if (i >= 0) return planningFill(PLANNING_POOL[i % PLANNING_POOL.length]);
    return "var(--color-land-dim)";
  };
}

/** The source asset spans 0…593; the Canada band lives ABOVE it in negative
 *  y, so the blocks can never collide with the northern-tier states. */
const MAP_VIEWBOX = "0 -88 959 681";

/** The two schematic Canada blocks (province detail out of scope — R3),
 *  drawn as chamfered instrument plates in the same coordinate space. */
const CANADA_BLOCKS: {
  code: string; // territory_states state_code ("CA-W" / "CA-E")
  label: string;
  d: string;
  labelX: number;
  labelY: number;
}[] = [
  {
    code: "CA-W",
    label: "CANADA WEST",
    d: "M 71,-78 h 350 l 12,12 v 34 l -12,12 H 83 l -12,-12 v -34 z",
    labelX: 246,
    labelY: -44,
  },
  {
    code: "CA-E",
    label: "CANADA EAST",
    d: "M 493,-78 h 350 l 12,12 v 34 l -12,12 h -350 l -12,-12 v -34 z",
    labelX: 668,
    labelY: -44,
  },
];

/** Small manual nudges where a bbox centre lands badly (lakes, panhandles). */
const LABEL_NUDGE: Record<string, { dx: number; dy: number }> = {
  MI: { dx: 12, dy: 16 },
  FL: { dx: 14, dy: 0 },
  LA: { dx: -8, dy: 0 },
  MN: { dx: -4, dy: 6 },
  ID: { dx: 0, dy: 14 },
  CA: { dx: -6, dy: 0 },
  AK: { dx: -10, dy: -6 },
  HI: { dx: 8, dy: 6 },
  VA: { dx: 4, dy: 4 },
  KY: { dx: 4, dy: 2 },
  WV: { dx: -2, dy: 4 },
  NY: { dx: 4, dy: 4 },
};

/** States too small to hold their own label — they get an east leader-line
 *  column instead (dropped below 768px; the tooltip carries their names). */
const LEADER_COLUMN_X = 926;

interface Box {
  cx: number;
  cy: number;
  w: number;
  h: number;
}

export interface MapHover {
  code: string;
  name: string;
  territory: string;
  inScope: boolean;
  x: number;
  y: number;
}

export function UsMap({
  territoryOf,
  scopeCodes,
  selected,
  onSelect,
  onHover,
  canadaSubtitle,
  fill,
  editing = false,
  onPaint,
  onStateDragStart,
}: {
  /** state code → territory code (territory_states, via the roster). */
  territoryOf: (state: string) => string | undefined;
  /** The caller's in-scope territory codes (identity, not payload). */
  scopeCodes: ReadonlySet<string>;
  selected: string | null;
  onSelect: (territoryCode: string) => void;
  /** Hover reporting for the money tooltip (null = left the map). */
  onHover: (hover: MapHover | null) => void;
  /** In-scope Canada block money line, keyed by territory code. */
  canadaSubtitle: (territoryCode: string) => string | null;
  /** THE fill resolution (makeTerritoryFill) — one function, no drift (D5). */
  fill: (code: string) => string;
  /** T1 edit mode (vp/admin only — the parent gates it): clicking a US
   *  state PAINTS it into the editor's selected territory instead of
   *  opening the panel; mousedown starts a possible drag. Canada blocks
   *  are not clickable in edit mode (v1 lock). */
  editing?: boolean;
  onPaint?: (stateCode: string) => void;
  onStateDragStart?: (stateCode: string, e: React.MouseEvent) => void;
}) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [boxes, setBoxes] = useState<Record<string, Box>>({});

  // Label anchors from path geometry — viewBox coordinates are constant, so
  // one post-mount measure serves every rendered size.
  useLayoutEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const next: Record<string, Box> = {};
    svg.querySelectorAll<SVGPathElement>("path[data-state]").forEach((p) => {
      const code = p.dataset.state!;
      const b = p.getBBox();
      next[code] = {
        cx: b.x + b.width / 2,
        cy: b.y + b.height / 2,
        w: b.width,
        h: b.height,
      };
    });
    setBoxes(next);
  }, []);

  // Leader-line column for the tiny Atlantic states: measured, sorted by
  // latitude, distributed with a minimum gap — deterministic, no overlap.
  const leaders = useMemo(() => {
    const tiny = US_STATES.filter((s) => {
      const b = boxes[s.code];
      return b && (b.w < 26 || b.h < 16);
    }).sort((a, b) => boxes[a.code].cy - boxes[b.code].cy);
    const placed: { code: string; from: Box; y: number }[] = [];
    let lastY = -Infinity;
    for (const s of tiny) {
      const b = boxes[s.code];
      const y = Math.max(b.cy, lastY + 14);
      placed.push({ code: s.code, from: b, y });
      lastY = y;
    }
    return placed;
  }, [boxes]);
  const leaderSet = useMemo(
    () => new Set(leaders.map((l) => l.code)),
    [leaders],
  );

  const report = (
    e: React.MouseEvent,
    code: string,
    name: string,
    territory: string,
  ) => {
    onHover({
      code,
      name,
      territory,
      inScope: scopeCodes.has(territory),
      x: e.clientX,
      y: e.clientY,
    });
  };

  const stateNodes = US_STATES.map((s) => {
    const territory = territoryOf(s.code) ?? "";
    // T1: edit mode paints CONFIG — every territory's fill shows (the editor
    // is vp/admin-only, and a just-created territory is not yet in the
    // session's cached scope list; money stays scope-gated regardless).
    const inScope = editing || scopeCodes.has(territory);
    const isSelected = selected !== null && territory === selected;
    return (
      <path
        key={s.code}
        d={s.d}
        data-state={s.code}
        data-territory={territory}
        className="cursor-pointer transition-[filter,fill-opacity] duration-150"
        style={{
          fill: inScope ? fill(territory) : "var(--color-land-dim)",
          stroke: isSelected
            ? "var(--color-seam-strong)"
            : "var(--color-seam)",
          strokeWidth: isSelected ? 1.4 : 0.75,
          filter: isSelected ? "brightness(1.22)" : undefined,
        }}
        onMouseEnter={(e) => report(e, s.code, s.name, territory)}
        onMouseMove={(e) => report(e, s.code, s.name, territory)}
        onMouseLeave={() => onHover(null)}
        onMouseDown={
          editing ? (e) => onStateDragStart?.(s.code, e) : undefined
        }
        onClick={
          editing
            ? () => onPaint?.(s.code)
            : () => territory && onSelect(territory)
        }
      />
    );
  });

  return (
    <svg
      ref={svgRef}
      viewBox={MAP_VIEWBOX}
      className="block h-auto w-full"
      role="img"
      aria-label="Territory map — US states and Canada blocks colored by territory"
      data-testid="territory-map-svg"
    >
      <defs>
        {/* The graticule: a quiet instrument grid behind the landmass. */}
        <pattern
          id="plenum-graticule"
          width="26"
          height="26"
          patternUnits="userSpaceOnUse"
        >
          <path
            d="M 26 0 L 0 0 0 26"
            fill="none"
            stroke="var(--color-graticule)"
            strokeWidth="0.6"
          />
        </pattern>
        {/* Selection glow — flow blue, restrained (data emphasis only). */}
        <filter id="plenum-territory-glow" x="-8%" y="-8%" width="116%" height="116%">
          <feDropShadow
            dx="0"
            dy="0"
            stdDeviation="3.2"
            floodColor="var(--color-data)"
            floodOpacity="0.35"
          />
        </filter>
      </defs>

      {/* Ocean + graticule ground (covers the Canada band too). */}
      <rect x="0" y="-88" width="959" height="681" rx="4" fill="var(--color-ocean)" />
      <rect x="0" y="-88" width="959" height="681" rx="4" fill="url(#plenum-graticule)" />

      {/* Canada blocks — same seam-and-plate language, above the US. */}
      {CANADA_BLOCKS.map((b) => {
        const territory = territoryOf(b.code) ?? "";
        const inScope = editing || scopeCodes.has(territory);
        const isSelected = selected !== null && territory === selected;
        const sub = scopeCodes.has(territory)
          ? canadaSubtitle(territory)
          : null;
        return (
          <g
            key={b.code}
            data-state={b.code}
            data-territory={territory}
            className={`transition-[filter] duration-150 ${
              editing ? "cursor-not-allowed" : "cursor-pointer"
            }`}
            style={{ filter: isSelected ? "brightness(1.22)" : undefined }}
            onMouseEnter={(e) =>
              report(e, b.code, `${b.label} — schematic block`, territory)
            }
            onMouseMove={(e) =>
              report(e, b.code, `${b.label} — schematic block`, territory)
            }
            onMouseLeave={() => onHover(null)}
            // v1 lock: Canada blocks are inert in edit mode (the tooltip
            // carries the "lands in v2" note — TerritoryMap renders it).
            onClick={editing ? undefined : () => territory && onSelect(territory)}
          >
            <path
              d={b.d}
              style={{
                fill: inScope ? fill(territory) : "var(--color-land-dim)",
                stroke: isSelected
                  ? "var(--color-seam-strong)"
                  : "var(--color-seam)",
                strokeWidth: isSelected ? 1.4 : 0.75,
              }}
            />
            <text
              x={b.labelX}
              y={b.labelY - (sub ? 7 : 0)}
              textAnchor="middle"
              className="nameplate pointer-events-none select-none"
              style={{
                fill: "var(--color-text-dim)",
                fontSize: 13,
                letterSpacing: "0.14em",
              }}
            >
              {b.label} · {territory}
            </text>
            {sub && (
              <text
                x={b.labelX}
                y={b.labelY + 12}
                textAnchor="middle"
                className="tabular pointer-events-none select-none"
                style={{ fill: "var(--color-text)", fontSize: 12 }}
              >
                {sub}
              </text>
            )}
          </g>
        );
      })}

      {/* Selected territory glow layer: re-draw its states underneath with
          the flow-blue drop shadow (restraint: one filter, low opacity). */}
      {selected && (
        <g filter="url(#plenum-territory-glow)" className="pointer-events-none">
          {US_STATES.filter((s) => territoryOf(s.code) === selected).map(
            (s) => (
              <path key={s.code} d={s.d} fill="var(--color-ocean)" />
            ),
          )}
          {CANADA_BLOCKS.filter((b) => territoryOf(b.code) === selected).map(
            (b) => (
              <path key={b.code} d={b.d} fill="var(--color-ocean)" />
            ),
          )}
        </g>
      )}

      {/* The states. */}
      <g style={{ strokeLinejoin: "round" }}>{stateNodes}</g>

      {/* AK/HI inset separators, from the source asset. */}
      {INSET_SEPARATORS.map((s, i) => (
        <path
          key={i}
          d={s.d}
          fill="none"
          stroke="var(--color-seam)"
          strokeWidth="1.2"
          className="pointer-events-none"
        />
      ))}

      {/* DC callout — the source file's clickable circle for the smallest
          jurisdiction. */}
      {DC_CALLOUT && (
        <circle
          cx={DC_CALLOUT.cx}
          cy={DC_CALLOUT.cy}
          r={DC_CALLOUT.r}
          data-state="DC"
          data-territory={territoryOf("DC") ?? ""}
          className="cursor-pointer"
          style={{
            fill: editing || scopeCodes.has(territoryOf("DC") ?? "")
              ? fill(territoryOf("DC") ?? "")
              : "var(--color-land-dim)",
            stroke: "var(--color-seam)",
            strokeWidth: 0.75,
          }}
          onMouseEnter={(e) =>
            report(e, "DC", "District of Columbia", territoryOf("DC") ?? "")
          }
          onMouseLeave={() => onHover(null)}
          onMouseDown={editing ? (e) => onStateDragStart?.("DC", e) : undefined}
          onClick={
            editing
              ? () => onPaint?.("DC")
              : () => {
                  const t = territoryOf("DC");
                  if (t) onSelect(t);
                }
          }
        />
      )}

      {/* Abbreviation labels: in-state where the shape can hold them… */}
      <g className="pointer-events-none select-none">
        {US_STATES.filter(
          (s) => boxes[s.code] && !leaderSet.has(s.code),
        ).map((s) => {
          const b = boxes[s.code];
          const nudge = LABEL_NUDGE[s.code] ?? { dx: 0, dy: 0 };
          return (
            <text
              key={s.code}
              x={b.cx + nudge.dx}
              y={b.cy + nudge.dy + 3.5}
              textAnchor="middle"
              className="nameplate"
              style={{
                fill: "var(--color-text-dim)",
                fontSize: 10,
                letterSpacing: "0.08em",
              }}
            >
              {s.code}
            </text>
          );
        })}
      </g>

      {/* …and an east leader column for the tiny Atlantic states (dropped
          below 768 — the tooltip still carries them; spec §8 doctrine). */}
      <g className="pointer-events-none hidden select-none min-[768px]:block">
        {leaders.map((l) => (
          <g key={l.code}>
            <path
              d={`M ${l.from.cx + 3},${l.from.cy} L ${LEADER_COLUMN_X - 6},${l.y}`}
              fill="none"
              stroke="var(--color-seam)"
              strokeWidth="0.6"
            />
            <text
              x={LEADER_COLUMN_X}
              y={l.y + 3}
              textAnchor="start"
              className="nameplate"
              style={{
                fill: "var(--color-text-dim)",
                fontSize: 9,
                letterSpacing: "0.08em",
              }}
            >
              {l.code}
            </text>
          </g>
        ))}
      </g>
    </svg>
  );
}
