// + New account (P5, R9a) — the small owed UI over P3's route-only
// POST /api/accounts. The house dialog pattern (Pipeline's Won/Lost
// dialogs): graphite scrim, seam panel, typed 422s rendered inline — the
// server stays the validator (leave the name blank and the API's friendly
// "name is required" comes back as the message; the client never pre-empts
// the law). Territory options come from the config roster filtered to the
// CALLER'S scope — the server re-checks regardless (403 out of scope).

import { useState } from "react";
import { useNavigate } from "react-router";
import { useMe } from "../auth/auth";
import { ApiError } from "../lib/api";
import { useAccountsList, useCreateAccount } from "../lib/crm";
import { useStates } from "../lib/queries";
import { COMMAND_PERIOD } from "../lib/params";

const STATUSES = ["customer", "prospect", "at_risk", "dormant"] as const;

export function NewAccountDialog({ onClose }: { onClose: () => void }) {
  const me = useMe();
  const roster = useStates(COMMAND_PERIOD);
  const accounts = useAccountsList();
  const create = useCreateAccount();
  const navigate = useNavigate();

  const [name, setName] = useState("");
  const [industry, setIndustry] = useState("");
  const [territoryId, setTerritoryId] = useState("");
  const [status, setStatus] = useState<string>("prospect");
  const [parentId, setParentId] = useState("");
  const [error, setError] = useState<string | null>(null);

  const myCodes = new Set(me.data?.territories ?? []);
  const options = (roster.data?.territories ?? []).filter((t) =>
    myCodes.has(t.territory_code),
  );
  const effectiveTerritory =
    territoryId || (options.length === 1 ? options[0].territory_id : "");

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (create.isPending) return;
    setError(null);
    if (!effectiveTerritory) {
      setError("Pick a territory.");
      return;
    }
    create.mutate(
      {
        // Deliberately NOT trimmed/pre-validated: the server owns validation
        // and its typed 422 renders below (R9a).
        name,
        industry,
        territory_id: effectiveTerritory,
        status,
        parent_account_id: parentId || null,
      },
      {
        onSuccess: (created) => {
          onClose();
          navigate(`/accounts/${created.id}`);
        },
        onError: (err) => {
          setError(
            err instanceof ApiError
              ? err.message
              : "Could not create the account.",
          );
        },
      },
    );
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-bg/70 p-4">
      <form
        onSubmit={submit}
        className="w-full max-w-md rounded-lg border border-seam bg-surface p-5"
        data-testid="new-account-dialog"
      >
        <h3 className="nameplate-strong text-base text-text">New account</h3>
        <p className="mt-1 text-2xs text-text-dim">
          Created in your territory scope — Postgres checks it again on write.
        </p>

        <label className="nameplate mt-4 mb-1 block text-2xs text-text-dim">
          Name
        </label>
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Account name"
          autoFocus
          data-testid="new-account-name"
          className="w-full rounded border border-seam bg-bg px-3 py-2 text-sm text-text outline-none focus:border-seam-strong"
        />

        <label className="nameplate mt-3 mb-1 block text-2xs text-text-dim">
          Industry
        </label>
        <input
          value={industry}
          onChange={(e) => setIndustry(e.target.value)}
          placeholder="e.g. metals, pharma, grain"
          data-testid="new-account-industry"
          className="w-full rounded border border-seam bg-bg px-3 py-2 text-sm text-text outline-none focus:border-seam-strong"
        />

        <div className="mt-3 grid grid-cols-1 gap-3 min-[420px]:grid-cols-2">
          <label className="block">
            <span className="nameplate mb-1 block text-2xs text-text-dim">
              Territory
            </span>
            <select
              value={effectiveTerritory}
              onChange={(e) => setTerritoryId(e.target.value)}
              data-testid="new-account-territory"
              className="w-full rounded border border-seam bg-bg px-2 py-2 text-sm text-text outline-none focus:border-seam-strong"
            >
              {options.length !== 1 && <option value="">Select…</option>}
              {options.map((t) => (
                <option key={t.territory_id} value={t.territory_id}>
                  {t.territory_code} — {t.territory_name}
                </option>
              ))}
            </select>
          </label>
          <label className="block">
            <span className="nameplate mb-1 block text-2xs text-text-dim">
              Status
            </span>
            <select
              value={status}
              onChange={(e) => setStatus(e.target.value)}
              data-testid="new-account-status"
              className="w-full rounded border border-seam bg-bg px-2 py-2 text-sm text-text outline-none focus:border-seam-strong"
            >
              {STATUSES.map((s) => (
                <option key={s} value={s}>
                  {s}
                </option>
              ))}
            </select>
          </label>
        </div>

        <label className="nameplate mt-3 mb-1 block text-2xs text-text-dim">
          Parent account (optional)
        </label>
        <select
          value={parentId}
          onChange={(e) => setParentId(e.target.value)}
          data-testid="new-account-parent"
          className="w-full rounded border border-seam bg-bg px-2 py-2 text-sm text-text outline-none focus:border-seam-strong"
        >
          <option value="">None</option>
          {(accounts.data?.items ?? []).map((a) => (
            <option key={a.id} value={a.id}>
              {a.name} ({a.territory_code})
            </option>
          ))}
        </select>

        {error && (
          <div
            className="mt-3 rounded border border-alarm/40 bg-surface px-3 py-2 text-xs text-alarm"
            role="alert"
            data-testid="new-account-error"
          >
            {error}
          </div>
        )}

        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            className="rounded border border-seam px-3 py-1.5 text-2xs text-text-dim hover:bg-surface-2"
          >
            <span className="nameplate">Cancel</span>
          </button>
          <button
            type="submit"
            disabled={create.isPending}
            data-testid="new-account-submit"
            className="rounded bg-data/15 px-3 py-1.5 text-2xs text-data hover:bg-data/25 disabled:opacity-50"
          >
            <span className="nameplate">
              {create.isPending ? "Creating…" : "Create account"}
            </span>
          </button>
        </div>
      </form>
    </div>
  );
}
