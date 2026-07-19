// Quotes (spec §8 screen 7) — two tabs: My quotes (creator view) and Approvals
// (the pending items the caller's ROLE tier can decide; the server already
// filters this — a rep's Approvals tab is legitimately empty). Rows link to the
// quote detail. The builder and the approval actions live in QuoteDetail /
// QuoteBuilder.

import { useSearchParams, useNavigate } from "react-router";
import { money, percent } from "../lib/format";
import { useQuotesList } from "../lib/crm";
import { useScreenReady } from "../lib/useScreenReady";
import { Segmented } from "../components/Segmented";
import { EmptyPanel, ErrorPanel, LoadingPanel } from "../components/states";
import { QuoteStatusChip } from "./badges";

type View = "mine" | "approvals";

export function Quotes() {
  const [params, setParams] = useSearchParams();
  const view: View = params.get("tab") === "approvals" ? "approvals" : "mine";
  const navigate = useNavigate();
  const query = useQuotesList(view);
  useScreenReady(query.isSuccess || query.isError);

  const setView = (v: View) => {
    const p = new URLSearchParams(params);
    p.set("tab", v);
    setParams(p, { replace: true });
  };

  const rows = query.data?.items ?? [];

  return (
    <div className="mx-auto max-w-[1400px] space-y-4">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="nameplate-strong text-xl text-text">Quotes</h1>
          <div className="nameplate text-2xs text-text-dim">
            Discount governance · every decision audit-trailed
          </div>
        </div>
        <Segmented<View>
          ariaLabel="Quote view"
          testid="quotes-tabs"
          value={view}
          onChange={setView}
          options={[
            { value: "mine", label: "My quotes" },
            { value: "approvals", label: "Approvals" },
          ]}
        />
      </header>

      {query.isLoading && <LoadingPanel label="Loading quotes" />}
      {query.isError && <ErrorPanel onRetry={() => query.refetch()} />}
      {query.isSuccess &&
        (rows.length === 0 ? (
          <EmptyPanel
            message={
              view === "approvals"
                ? "Nothing awaiting your approval."
                : "You haven’t created any quotes yet. Draft one from a Pipeline card."
            }
          />
        ) : (
          <div className="scroll-x rounded-lg border border-seam bg-surface">
            <table className="w-full text-sm">
              <thead className="border-b border-seam">
                <tr>
                  {["Account", "Status", "Worst disc.", "Gross", "Net", "By"].map(
                    (h, i) => (
                      <th
                        key={h}
                        className={`nameplate px-3 py-2 text-2xs text-text-dim ${
                          i >= 2 && i <= 4 ? "text-right" : "text-left"
                        } ${i === 5 ? "hidden min-[720px]:table-cell" : ""}`}
                      >
                        {h}
                      </th>
                    ),
                  )}
                </tr>
              </thead>
              <tbody>
                {rows.map((q) => (
                  <tr
                    key={q.id}
                    onClick={() => navigate(`/quotes/${q.id}`)}
                    data-testid="quote-row"
                    className="cursor-pointer border-b border-seam/40 last:border-0 hover:bg-surface-2/50"
                  >
                    <td className="px-3 py-2 text-text">{q.account_name}</td>
                    <td className="px-3 py-2">
                      <QuoteStatusChip status={q.status} />
                    </td>
                    <td className="tabular px-3 py-2 text-right text-text">
                      {percent(q.worst_discount_pct)}
                    </td>
                    <td className="tabular px-3 py-2 text-right text-text">
                      {money(q.gross_cents)}
                    </td>
                    <td className="tabular px-3 py-2 text-right text-text">
                      {money(q.net_cents)}
                    </td>
                    <td className="hidden px-3 py-2 text-text-dim min-[720px]:table-cell">
                      {q.created_by_name}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))}
    </div>
  );
}
