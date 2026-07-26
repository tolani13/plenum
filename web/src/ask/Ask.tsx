// Ask PLENUM (spec §8 screen 8, ruling R9). A natural-language question goes
// to the backend, which returns table + the VALIDATED SQL it actually ran
// (receipts, always). The saved-question library renders ALWAYS — with the
// AI off (no key / flag / 503) the input yields to a quiet note and the
// library still answers the standing questions with client-side links. A 503
// can never become an error screen here by construction.

import { useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router";
import {
  Bar,
  BarChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { ApiError } from "../lib/api";
import { useAiStatus, useAsk } from "../lib/ai";
// P5 (R7): ASK_FOCUS_EVENT lives in lib/events now — this screen is a lazy
// route, and the Shell must not import anything from its chunk.
import { ASK_FOCUS_EVENT } from "../lib/events";
import type { AskResult } from "../lib/types";
import { useScreenReady } from "../lib/useScreenReady";

/** The 7 metric screens as curated questions (spec §6.5) — every link lands
 *  on a live surface; nothing here needs the model. */
const LIBRARY: { q: string; to: string; note: string }[] = [
  {
    q: "How is every territory doing this year?",
    to: "/command",
    note: "Command · Territory Board, YTD",
  },
  {
    q: "Who leads the rep leaderboard in 2026, net basis?",
    to: "/leaderboards?tab=reps&period=2026&basis=net",
    note: "Leaderboards · Reps",
  },
  {
    q: "Which product families sell most in 2026?",
    to: "/leaderboards?tab=items&period=2026&basis=net&group=family",
    note: "Leaderboards · Items by family",
  },
  {
    q: "Top customers, cumulative, both bases",
    to: "/leaderboards?tab=customers&period=cumulative&basis=net",
    note: "Leaderboards · Customers (toggle the basis)",
  },
  {
    q: "How much are we leaking to discounts this year?",
    to: "/command",
    note: "Command · Leakage % KPI",
  },
  {
    q: "How covered is this quarter's aftermarket?",
    to: "/command",
    note: "Command · Coverage % KPI",
  },
  {
    q: "Which installed units are going quiet?",
    to: "/signals",
    note: "Signals · Defection lane",
  },
];

/** Chart-worthiness (R9): one leading label column + ≥1 numeric column and
 *  ≤50 rows. Money columns (*_cents) chart in dollars. */
function chartData(result: AskResult) {
  if (result.rows.length === 0 || result.rows.length > 50) return null;
  if (result.columns.length < 2) return null;
  const first = result.rows[0];
  if (typeof first[0] !== "string") return null;
  const numericIdx = result.columns.findIndex(
    (_, i) => i > 0 && typeof first[i] === "number",
  );
  if (numericIdx === -1) return null;
  const numericCol = result.columns[numericIdx];
  const isCents = numericCol.endsWith("_cents");
  const data = result.rows.map((r) => ({
    label: String(r[0]),
    value:
      typeof r[numericIdx] === "number"
        ? isCents
          ? (r[numericIdx] as number) / 100
          : (r[numericIdx] as number)
        : 0,
  }));
  return {
    data,
    valueLabel: isCents ? `${numericCol} (in $)` : numericCol,
  };
}

function cellText(v: string | number | boolean | null): string {
  if (v === null) return "—";
  if (typeof v === "number") return v.toLocaleString("en-US");
  return String(v);
}

export function Ask() {
  const status = useAiStatus();
  const ask = useAsk();
  const [question, setQuestion] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [aiDown, setAiDown] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useScreenReady(status.isSuccess || status.isError, "ask");

  useEffect(() => {
    const focus = () => inputRef.current?.focus();
    window.addEventListener(ASK_FOCUS_EVENT, focus);
    return () => window.removeEventListener(ASK_FOCUS_EVENT, focus);
  }, []);

  const askEnabled = (status.data?.ask ?? false) && !aiDown;
  const result = ask.data;
  const chart = useMemo(() => (result ? chartData(result) : null), [result]);

  const run = (e: React.FormEvent) => {
    e.preventDefault();
    const q = question.trim();
    if (!q || ask.isPending) return;
    setError(null);
    ask.mutate(q, {
      onError: (err) => {
        // A 503 = AI off/unreachable — fold into the designed off state
        // (library still answers), NEVER an error screen (R9).
        if (err instanceof ApiError && err.status === 503) {
          setAiDown(true);
        } else {
          setError(err instanceof Error ? err.message : "The question failed.");
        }
      },
    });
  };

  return (
    <div className="mx-auto max-w-[1200px] space-y-4">
      <header>
        <h1 className="nameplate-strong text-xl text-text">Ask PLENUM</h1>
        <div className="nameplate text-2xs text-text-dim">
          natural language → SQL over the whitelisted semantic layer · your
          territory scope applies
        </div>
      </header>

      {askEnabled ? (
        <form onSubmit={run} className="flex flex-wrap gap-2">
          <input
            ref={inputRef}
            autoFocus
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            placeholder="e.g. top 10 customers by net revenue in 2025"
            data-testid="ask-input"
            className="min-w-0 flex-1 rounded border border-seam bg-surface px-3 py-2 text-sm text-text outline-none focus:border-seam-strong"
          />
          <button
            type="submit"
            disabled={ask.isPending || !question.trim()}
            data-testid="ask-run"
            className="rounded bg-data/15 px-4 py-2 text-sm text-data hover:bg-data/25 disabled:opacity-50"
          >
            <span className="nameplate">{ask.isPending ? "Asking…" : "Run"}</span>
          </button>
        </form>
      ) : (
        <div
          className="rounded-lg border border-seam bg-surface p-4 text-xs text-text-dim"
          data-testid="ask-off-note"
        >
          AI is off — the library below answers the standing questions.
        </div>
      )}

      {error && (
        <div className="rounded border border-alarm/40 bg-surface px-3 py-2 text-xs text-alarm">
          {error}
        </div>
      )}

      {askEnabled && result && (
        <div className="space-y-3" data-testid="ask-result">
          {chart && (
            <div className="rounded-lg border border-seam bg-surface p-4">
              <div className="nameplate mb-2 text-2xs text-text-dim">
                {chart.valueLabel}
              </div>
              <div className="h-64 w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={chart.data} margin={{ left: 8, right: 8 }}>
                    <XAxis
                      dataKey="label"
                      tick={{ fill: "var(--color-text-dim)", fontSize: 10 }}
                      stroke="var(--color-seam)"
                      interval={0}
                      angle={-20}
                      textAnchor="end"
                      height={56}
                    />
                    <YAxis
                      tick={{ fill: "var(--color-text-dim)", fontSize: 10 }}
                      stroke="var(--color-seam)"
                      width={90}
                      tickFormatter={(v: number) => v.toLocaleString("en-US")}
                    />
                    <Tooltip
                      cursor={{ fill: "var(--color-surface-2)" }}
                      contentStyle={{
                        background: "var(--color-surface)",
                        border: "1px solid var(--color-seam)",
                        borderRadius: 4,
                        color: "var(--color-text)",
                        fontSize: 12,
                      }}
                      formatter={(v) => [Number(v).toLocaleString("en-US")]}
                    />
                    <Bar dataKey="value" fill="var(--color-data)" />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </div>
          )}

          <div className="rounded-lg border border-seam bg-surface">
            <div className="flex items-baseline justify-between border-b border-seam px-4 py-2.5">
              <span className="nameplate text-2xs text-text-dim">Result</span>
              <span className="tabular text-2xs text-text-dim">
                {result.row_count} row{result.row_count === 1 ? "" : "s"}
                {result.truncated ? " · truncated at 500" : ""}
              </span>
            </div>
            {result.rows.length === 0 ? (
              <div className="p-4 text-xs text-text-dim">
                No rows in your scope for that question.
              </div>
            ) : (
              <div className="scroll-x p-2">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-seam">
                      {result.columns.map((c) => (
                        <th
                          key={c}
                          className="nameplate whitespace-nowrap px-2 py-1.5 text-left text-2xs text-text-dim"
                        >
                          {c}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {result.rows.map((row, i) => (
                      <tr key={i} className="border-b border-seam/40 last:border-0">
                        {row.map((v, j) => (
                          <td
                            key={j}
                            className={`whitespace-nowrap px-2 py-1.5 ${
                              typeof v === "number"
                                ? "tabular text-right text-text"
                                : "text-text"
                            }`}
                          >
                            {cellText(v)}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>

          <div className="rounded-lg border border-seam bg-surface">
            <div className="border-b border-seam px-4 py-2.5">
              <span className="nameplate text-2xs text-data">
                SQL receipts — what actually ran
              </span>
            </div>
            <div className="scroll-x p-4">
              <pre
                className="whitespace-pre-wrap break-words font-mono text-xs text-text"
                data-testid="ask-sql"
              >
                {result.sql}
              </pre>
            </div>
          </div>
        </div>
      )}

      <section className="rounded-lg border border-seam bg-surface">
        <div className="border-b border-seam px-4 py-2.5">
          <span className="nameplate text-2xs text-text-dim">
            Saved questions — the standing library
          </span>
        </div>
        <ul className="divide-y divide-seam/40">
          {LIBRARY.map((item) => (
            <li key={item.q}>
              <Link
                to={item.to}
                data-testid="library-link"
                className="flex flex-wrap items-baseline justify-between gap-2 px-4 py-2.5 transition-colors hover:bg-surface-2"
              >
                <span className="text-sm text-text">{item.q}</span>
                <span className="nameplate text-2xs text-text-dim">
                  {item.note}
                </span>
              </Link>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
