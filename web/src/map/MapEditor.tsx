// T1 (D4) — the map editor panel: the side panel's Edit-mode face. Rendered
// ONLY for vp/admin with edit mode on (the parent gates it; a rep's DOM
// never contains any of these testids — tripwire-asserted). All validation
// verdicts come from the server and render VERBATIM (the R9a discipline:
// the client never pre-empts the law) — delete refusals especially, since
// their reason string names WHICH emptiness check failed.

import { useMemo, useState } from "react";
import { ApiError } from "../lib/api";
import { money } from "../lib/format";
import {
  PLANNING_PALETTE,
  useCreateTerritory,
  useDeleteTerritory,
  usePatchTerritory,
} from "../lib/geo";
import type { AdminTerritory, TerritoryRoster } from "../lib/types";
import { planningFill } from "./UsMap";

function ChipPicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (token: string) => void;
}) {
  return (
    <div className="flex flex-wrap gap-1.5">
      {PLANNING_PALETTE.map((token) => (
        <button
          key={token}
          type="button"
          title={token}
          onClick={() => onChange(token)}
          data-testid="editor-color-chip"
          data-token={token}
          className={`h-5 w-5 rounded-sm border ${
            value === token
              ? "border-seam-strong ring-1 ring-seam-strong"
              : "border-seam"
          }`}
          style={{ background: planningFill(token) }}
        />
      ))}
    </div>
  );
}

const inputCls =
  "w-full rounded border border-seam bg-bg px-2 py-1.5 text-sm text-text outline-none focus:border-seam-strong";

export function MapEditor({
  territories,
  roster,
  planningSums,
  basis,
  fill,
  paintTarget,
  onSelectPaintTarget,
}: {
  /** GET /api/territories — the editor's own read (full list incl. empty). */
  territories: AdminTerritory[];
  /** The states-feed roster (mapped state codes per territory). */
  roster: TerritoryRoster[];
  /** Client-side per-territory sum of the caller's state rows (D4ii). */
  planningSums: ReadonlyMap<string, number>;
  basis: string;
  fill: (code: string) => string;
  paintTarget: string | null;
  onSelectPaintTarget: (code: string | null) => void;
}) {
  const create = useCreateTerritory();
  const patch = usePatchTerritory();
  const remove = useDeleteTerritory();

  const [error, setError] = useState<string | null>(null);
  const [editingCode, setEditingCode] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [editToken, setEditToken] = useState("");

  const [showNew, setShowNew] = useState(false);
  const [newCode, setNewCode] = useState("");
  const [newName, setNewName] = useState("");
  const [newRegion, setNewRegion] = useState("");
  const [newToken, setNewToken] = useState<string>(PLANNING_PALETTE[0]);

  const stateCounts = useMemo(() => {
    const m = new Map<string, number>();
    for (const t of roster) m.set(t.territory_code, t.state_codes.length);
    return m;
  }, [roster]);

  /** Region dropdown options — the existing territories' region values. */
  const regions = useMemo(
    () => [...new Set(territories.map((t) => t.region))].sort(),
    [territories],
  );

  const surface = (err: unknown, fallback: string) =>
    setError(err instanceof ApiError ? err.message : fallback);

  const startEdit = (t: AdminTerritory) => {
    setError(null);
    setEditingCode(t.code);
    setEditName(t.name);
    setEditToken(t.color_token ?? "");
  };

  const saveEdit = (t: AdminTerritory) => {
    setError(null);
    patch.mutate(
      {
        code: t.code,
        name: editName !== t.name ? editName : undefined,
        color_token:
          editToken && editToken !== t.color_token ? editToken : undefined,
      },
      {
        onSuccess: () => setEditingCode(null),
        onError: (err) => surface(err, "Could not update the territory."),
      },
    );
  };

  const submitNew = (e: React.FormEvent) => {
    e.preventDefault();
    if (create.isPending) return;
    setError(null);
    create.mutate(
      {
        // Deliberately not pre-validated: the server owns the law and its
        // typed 422 renders below verbatim.
        code: newCode,
        name: newName,
        region: newRegion,
        color_token: newToken || undefined,
      },
      {
        onSuccess: (t) => {
          setShowNew(false);
          setNewCode("");
          setNewName("");
          onSelectPaintTarget(t.code);
        },
        onError: (err) => surface(err, "Could not create the territory."),
      },
    );
  };

  return (
    <div data-testid="map-editor">
      <div className="nameplate-strong text-base text-text">Map editor</div>
      <p className="mt-1 text-2xs text-text-dim">
        Select a territory, then click states on the map to paint them — or
        drag a state onto a row below.
      </p>

      <div className="mt-3 space-y-1">
        {territories.map((t) => {
          const isTarget = paintTarget === t.code;
          const sum = planningSums.get(t.code) ?? 0;
          const states = stateCounts.get(t.code) ?? 0;
          return (
            <div
              key={t.code}
              data-testid="editor-territory-row"
              data-drop-territory={t.code}
              className={`rounded border px-2 py-1.5 transition-colors ${
                isTarget
                  ? "border-seam-strong bg-surface-2"
                  : "border-seam/60 hover:border-seam"
              }`}
            >
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => onSelectPaintTarget(isTarget ? null : t.code)}
                  data-testid="editor-select-territory"
                  data-territory={t.code}
                  className="flex min-w-0 flex-1 items-center gap-2 text-left"
                  title={`${t.name} — select to paint`}
                >
                  <span
                    className="inline-block h-3 w-3 shrink-0 rounded-sm border border-seam"
                    style={{ background: fill(t.code) }}
                  />
                  <span className="nameplate text-2xs text-text">{t.code}</span>
                  <span className="truncate text-2xs text-text-dim">
                    {t.name}
                  </span>
                </button>
                <span
                  className="tabular shrink-0 text-2xs text-text"
                  data-testid="editor-planning-sum"
                  data-territory={t.code}
                >
                  {money(sum)}
                </span>
              </div>
              <div className="mt-0.5 flex items-center gap-2 pl-5">
                <span className="text-2xs text-text-dim">
                  {states} state{states === 1 ? "" : "s"} ·{" "}
                  {t.region.replace("_", " ")} · {basis}
                </span>
                <span className="ml-auto flex gap-2">
                  <button
                    type="button"
                    className="text-2xs text-text-dim hover:text-text"
                    onClick={() =>
                      editingCode === t.code
                        ? setEditingCode(null)
                        : startEdit(t)
                    }
                    data-testid="editor-rename"
                    data-territory={t.code}
                  >
                    edit
                  </button>
                  <button
                    type="button"
                    className="text-2xs text-text-dim hover:text-alarm"
                    onClick={() => {
                      setError(null);
                      remove.mutate(t.code, {
                        onError: (err) =>
                          surface(err, "Could not delete the territory."),
                      });
                    }}
                    data-testid="editor-delete"
                    data-territory={t.code}
                  >
                    delete
                  </button>
                </span>
              </div>

              {editingCode === t.code && (
                <div className="mt-2 space-y-2 border-t border-seam/60 pt-2">
                  <input
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    data-testid="editor-rename-input"
                    className={inputCls}
                  />
                  <ChipPicker value={editToken} onChange={setEditToken} />
                  <button
                    type="button"
                    onClick={() => saveEdit(t)}
                    disabled={patch.isPending}
                    data-testid="editor-rename-save"
                    className="rounded border border-seam-strong px-2 py-1 text-2xs text-text hover:bg-surface-2"
                  >
                    Save
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>

      {error && (
        <div
          data-testid="editor-error"
          className="mt-3 rounded border border-alarm/50 bg-alarm/10 px-2 py-1.5 text-2xs text-text"
        >
          {error}
        </div>
      )}

      <div className="mt-3 border-t border-seam/60 pt-3">
        {!showNew ? (
          <button
            type="button"
            onClick={() => {
              setShowNew(true);
              setError(null);
              setNewRegion(regions[0] ?? "");
            }}
            data-testid="editor-new-territory"
            className="rounded border border-seam px-2.5 py-1.5 text-2xs text-text hover:border-seam-strong"
          >
            + New territory
          </button>
        ) : (
          <form onSubmit={submitNew} className="space-y-2">
            <div className="nameplate text-2xs text-text-dim">
              New territory
            </div>
            <div className="grid grid-cols-[92px_minmax(0,1fr)] gap-2">
              <input
                value={newCode}
                onChange={(e) => setNewCode(e.target.value.toUpperCase())}
                placeholder="Code"
                data-testid="editor-new-code"
                className={inputCls}
              />
              <input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder="Name"
                data-testid="editor-new-name"
                className={inputCls}
              />
            </div>
            <select
              value={newRegion}
              onChange={(e) => setNewRegion(e.target.value)}
              data-testid="editor-new-region"
              className={inputCls}
            >
              {regions.map((r) => (
                <option key={r} value={r}>
                  {r.replace("_", " ")}
                </option>
              ))}
            </select>
            <ChipPicker value={newToken} onChange={setNewToken} />
            <div className="flex gap-2">
              <button
                type="submit"
                disabled={create.isPending}
                data-testid="editor-new-create"
                className="rounded border border-seam-strong px-2.5 py-1.5 text-2xs text-text hover:bg-surface-2"
              >
                Create
              </button>
              <button
                type="button"
                onClick={() => setShowNew(false)}
                className="rounded border border-seam px-2.5 py-1.5 text-2xs text-text-dim hover:text-text"
              >
                Cancel
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
