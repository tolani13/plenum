// Quote builder (spec §8 screen 7) — product picker (list price from the server
// catalog), qty, discount 0–100 two-decimals, per-line and total gross/net/
// leakage, and a LIVE policy verdict that updates as lines change (R10). The
// client computes net only for display; the server derives every stored price
// and recomputes the verdict on submit (money law + R3).
//
// P4 touches (rulings R6 + R10):
//   · draft-quote-from-signal prefill — ?product=&qty=&signal= seed the first
//     line (the signal's cartridge or conquest best-fit, qty = cartridge
//     count); on successful creation the signal is actioned with outcome
//     quote_drafted:<quote_id> (best-effort — a failed write-back never
//     blocks the quote);
//   · the COMPS panel — on-demand comparables (median/IQR + sample) for one
//     line via POST /api/ai/discount-recommendation, shown only when
//     /api/ai/status says the recommender is on; narrative degrades to the
//     raw table without a key, never an error.

import { useMemo, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router";
import { ArrowLeft, Plus, Scale, Trash2 } from "lucide-react";
import { money, percent } from "../lib/format";
import { useCreateQuote, useOpportunities, useProducts, useDiscountPolicy } from "../lib/crm";
import type { QuoteLineInput } from "../lib/crm";
import { useSubmitQuote } from "../lib/crm";
import { useActionSignal } from "../lib/signals";
import { useAiStatus, useDiscountRec } from "../lib/ai";
import type { DiscountRec } from "../lib/types";
import { useScreenReady } from "../lib/useScreenReady";
import { ErrorPanel, LoadingPanel } from "../components/states";
import { verdictLabel, verdictTier } from "./verdict";

interface DraftLine {
  product_id: string;
  qty: number;
  discount_pct: number;
}

/** net = round(list * (100 - disc) / 100) — mirrors the SQL CHECK for display. */
function lineNet(list: number, disc: number): number {
  return Math.round((list * (100 - disc)) / 100);
}

function CompsPanel({ rec }: { rec: DiscountRec }) {
  const c = rec.comparables;
  return (
    <div className="rounded-lg border border-seam bg-surface p-4" data-testid="comps-panel">
      <div className="nameplate text-2xs text-data">Comps</div>
      <div className="mt-1 text-2xs text-text-dim">
        {c.family} · {c.industry} · {c.band_label}
      </div>
      {c.count === 0 ? (
        <div className="mt-2 text-xs text-text-dim">
          No comparable lines in your scope for this cohort.
        </div>
      ) : (
        <>
          <div className="mt-2 flex items-baseline justify-between text-sm">
            <span className="text-text-dim">
              median{" "}
              <span className="tabular text-text">{percent(c.median_pct, 1)}</span>
            </span>
            <span className="text-text-dim">
              IQR{" "}
              <span className="tabular text-text">
                {percent(c.p25, 1)}–{percent(c.p75, 1)}
              </span>
            </span>
            <span className="tabular text-2xs text-text-dim">{c.count} lines</span>
          </div>
          <div className="scroll-x mt-2">
            <table className="w-full text-2xs">
              <thead>
                <tr className="border-b border-seam">
                  <th className="nameplate px-1.5 py-1 text-left text-text-dim">Account</th>
                  <th className="nameplate px-1.5 py-1 text-right text-text-dim">Gross</th>
                  <th className="nameplate px-1.5 py-1 text-right text-text-dim">Disc.</th>
                </tr>
              </thead>
              <tbody>
                {c.sample.map((s, i) => (
                  <tr key={i} className="border-b border-seam/40 last:border-0">
                    <td
                      className="max-w-[9rem] truncate px-1.5 py-1 text-text"
                      title={`${s.account_name} · ${s.product_sku} × ${s.qty} · ${s.ordered_on}`}
                    >
                      {s.account_name}
                    </td>
                    <td className="tabular px-1.5 py-1 text-right text-text">
                      {money(s.gross_cents)}
                    </td>
                    <td className="tabular px-1.5 py-1 text-right text-text">
                      {percent(s.discount_pct, 1)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
      {rec.narrative ? (
        <div className="mt-2 border-t border-seam pt-2 text-xs text-text" data-testid="comps-narrative">
          {rec.narrative}
        </div>
      ) : (
        <div className="mt-2 border-t border-seam pt-2 text-2xs text-text-dim" data-testid="comps-degraded">
          Comparables only — the AI narrative is off (no key).
        </div>
      )}
    </div>
  );
}

export function QuoteBuilder() {
  const [params] = useSearchParams();
  const oppId = params.get("opp") ?? "";
  const prefillProduct = params.get("product");
  const prefillQty = Math.max(1, Number(params.get("qty")) || 1);
  const signalId = params.get("signal");
  const navigate = useNavigate();

  const opps = useOpportunities("all");
  const products = useProducts();
  const policy = useDiscountPolicy();
  const aiStatus = useAiStatus();
  const createQuote = useCreateQuote();
  const submitQuote = useSubmitQuote();
  const actionSignal = useActionSignal();
  const discountRec = useDiscountRec();

  const [lines, setLines] = useState<DraftLine[]>([
    prefillProduct
      ? { product_id: prefillProduct, qty: prefillQty, discount_pct: 0 }
      : { product_id: "", qty: 1, discount_pct: 0 },
  ]);
  const [error, setError] = useState<string | null>(null);
  const [compsFor, setCompsFor] = useState<number | null>(null);

  const ready =
    (opps.isSuccess || opps.isError) &&
    (products.isSuccess || products.isError) &&
    (policy.isSuccess || policy.isError);
  useScreenReady(ready);

  const opp = opps.data?.items.find((o) => o.id === oppId);
  const catalog = useMemo(
    () => (products.data?.items ?? []).filter((p) => p.list_price_cents > 0),
    [products.data],
  );
  const priceOf = (id: string) =>
    catalog.find((p) => p.id === id)?.list_price_cents ?? 0;

  const selfMax = policy.data?.self_max_pct ?? 10;
  const managerMax = policy.data?.manager_max_pct ?? 25;

  const totals = useMemo(() => {
    let gross = 0;
    let net = 0;
    let worst = 0;
    for (const l of lines) {
      const list = priceOf(l.product_id);
      if (!l.product_id) continue;
      gross += list * l.qty;
      net += lineNet(list, l.discount_pct) * l.qty;
      worst = Math.max(worst, l.discount_pct);
    }
    return { gross, net, leakage: gross - net, worst };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lines, catalog]);

  const tier = verdictTier(totals.worst, {
    self_max_pct: selfMax,
    manager_max_pct: managerMax,
  });
  const tone =
    tier === "self" ? "text-ok" : tier === "manager" ? "text-warn" : "text-alarm";

  if (opps.isLoading || products.isLoading) return <LoadingPanel label="Loading builder" />;
  if (!opp)
    return (
      <div className="mx-auto max-w-[1000px]">
        <ErrorPanel
          onRetry={() => opps.refetch()}
          message="That opportunity isn’t available (out of scope, or already closed)."
        />
      </div>
    );

  const setLine = (i: number, patch: Partial<DraftLine>) =>
    setLines((ls) => ls.map((l, j) => (j === i ? { ...l, ...patch } : l)));
  const addLine = () =>
    setLines((ls) => [...ls, { product_id: "", qty: 1, discount_pct: 0 }]);
  const removeLine = (i: number) =>
    setLines((ls) => (ls.length > 1 ? ls.filter((_, j) => j !== i) : ls));

  const validLines: QuoteLineInput[] = lines
    .filter((l) => l.product_id && l.qty > 0)
    .map((l) => ({
      product_id: l.product_id,
      qty: l.qty,
      discount_pct: l.discount_pct,
    }));

  const fetchComps = (i: number) => {
    const l = lines[i];
    if (!l?.product_id) return;
    setCompsFor(i);
    discountRec.mutate({
      product_id: l.product_id,
      account_id: opp.account_id,
      qty: l.qty,
      discount_pct: l.discount_pct,
    });
  };

  const persist = async (thenSubmit: boolean) => {
    setError(null);
    if (validLines.length === 0) {
      setError("Add at least one line with a product and quantity.");
      return;
    }
    if (lines.some((l) => l.product_id && (l.discount_pct < 0 || l.discount_pct > 100))) {
      setError("Discount must be between 0 and 100.");
      return;
    }
    try {
      const quote = await createQuote.mutateAsync({
        opportunity_id: oppId,
        lines: validLines,
      });
      // R6: the source signal shows ACTIONED once the draft exists. A failed
      // write-back is reported but never blocks the drafted quote.
      if (signalId) {
        try {
          await actionSignal.mutateAsync({
            id: signalId,
            outcome: `quote_drafted:${quote.id}`,
          });
        } catch {
          setError("Quote drafted, but the signal write-back failed.");
        }
      }
      if (thenSubmit) await submitQuote.mutateAsync(quote.id);
      navigate(`/quotes/${quote.id}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not save the quote.");
    }
  };

  const busy = createQuote.isPending || submitQuote.isPending;
  const compsOn = aiStatus.data?.discount ?? false;

  return (
    <div className="mx-auto max-w-[1100px] space-y-4">
      <div>
        <Link
          to={signalId ? "/signals" : "/pipeline"}
          className="mb-2 inline-flex items-center gap-1 text-2xs text-text-dim hover:text-text"
        >
          <ArrowLeft size={12} />{" "}
          <span className="nameplate">{signalId ? "Signals" : "Pipeline"}</span>
        </Link>
        <h1 className="nameplate-strong text-xl text-text">
          New quote · {opp.account_name}
        </h1>
        <div className="nameplate text-2xs text-text-dim">
          {opp.kind} · {opp.territory_code}
          {signalId ? " · drafted from a signal" : ""}
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 min-[820px]:grid-cols-[2fr_1fr]">
        <div className="space-y-2">
          {lines.map((l, i) => {
            const list = priceOf(l.product_id);
            const net = l.product_id ? lineNet(list, l.discount_pct) : 0;
            return (
              <div
                key={i}
                className="rounded-lg border border-seam bg-surface p-3"
                data-testid="builder-line"
              >
                <div className="flex flex-wrap items-end gap-2">
                  <label className="min-w-[10rem] flex-1">
                    <span className="nameplate mb-1 block text-2xs text-text-dim">
                      Product
                    </span>
                    <select
                      value={l.product_id}
                      onChange={(e) => setLine(i, { product_id: e.target.value })}
                      data-testid="line-product"
                      className="w-full rounded border border-seam bg-bg px-2 py-1.5 text-sm text-text outline-none focus:border-seam-strong"
                    >
                      <option value="">Select…</option>
                      {catalog.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.sku} — {p.name} ({money(p.list_price_cents)})
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="w-16">
                    <span className="nameplate mb-1 block text-2xs text-text-dim">
                      Qty
                    </span>
                    <input
                      type="number"
                      min={1}
                      value={l.qty}
                      onChange={(e) =>
                        setLine(i, { qty: Math.max(1, Number(e.target.value) || 1) })
                      }
                      data-testid="line-qty"
                      className="tabular w-full rounded border border-seam bg-bg px-2 py-1.5 text-sm text-text outline-none focus:border-seam-strong"
                    />
                  </label>
                  <label className="w-24">
                    <span className="nameplate mb-1 block text-2xs text-text-dim">
                      Disc. %
                    </span>
                    <input
                      type="number"
                      min={0}
                      max={100}
                      step={0.01}
                      value={l.discount_pct}
                      onChange={(e) =>
                        setLine(i, { discount_pct: Number(e.target.value) || 0 })
                      }
                      data-testid="line-discount"
                      className="tabular w-full rounded border border-seam bg-bg px-2 py-1.5 text-sm text-text outline-none focus:border-seam-strong"
                    />
                  </label>
                  {compsOn && l.product_id && (
                    <button
                      onClick={() => fetchComps(i)}
                      disabled={discountRec.isPending && compsFor === i}
                      data-testid="line-comps"
                      title="Comparable deals for this line"
                      className="inline-flex items-center gap-1 rounded border border-seam px-2 py-1.5 text-2xs text-data transition-colors hover:bg-surface-2 disabled:opacity-50"
                    >
                      <Scale size={13} />
                      <span className="nameplate">
                        {discountRec.isPending && compsFor === i ? "…" : "Comps"}
                      </span>
                    </button>
                  )}
                  <button
                    onClick={() => removeLine(i)}
                    className="rounded border border-seam p-1.5 text-text-dim hover:text-alarm"
                    title="Remove line"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
                {l.product_id && (
                  <div className="mt-2 flex justify-between text-2xs text-text-dim">
                    <span>
                      net <span className="tabular text-text">{money(net)}</span> /unit
                    </span>
                    <span>
                      line net{" "}
                      <span className="tabular text-text">{money(net * l.qty)}</span>
                    </span>
                  </div>
                )}
              </div>
            );
          })}
          <button
            onClick={addLine}
            className="flex items-center gap-1.5 rounded border border-seam px-3 py-1.5 text-2xs text-text-dim hover:bg-surface-2 hover:text-text"
            data-testid="add-line"
          >
            <Plus size={13} /> <span className="nameplate">Add line</span>
          </button>
        </div>

        <div className="space-y-3">
          <div className="rounded-lg border border-seam bg-surface p-4" data-testid="verdict">
            <div className="nameplate text-2xs text-text-dim">Live verdict</div>
            <div className={`mt-1 text-lg ${tone}`}>
              {verdictLabel(tier, {
                self_max_pct: selfMax,
                manager_max_pct: managerMax,
              })}
            </div>
            <div className="mt-1 text-2xs text-text-dim">
              worst line{" "}
              <span className="tabular text-text">{percent(totals.worst, 2)}</span>
            </div>
            <div className="mt-3 space-y-1 border-t border-seam pt-3 text-sm">
              <div className="flex justify-between">
                <span className="text-text-dim">Gross</span>
                <span className="tabular text-text">{money(totals.gross)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-dim">Net</span>
                <span className="tabular text-text">{money(totals.net)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-text-dim">Leakage</span>
                <span className="tabular text-warn">{money(totals.leakage)}</span>
              </div>
            </div>
          </div>

          {compsOn && compsFor !== null && discountRec.data && (
            <CompsPanel rec={discountRec.data} />
          )}
          {compsOn && compsFor !== null && discountRec.isError && (
            <div className="rounded border border-seam bg-surface px-3 py-2 text-2xs text-text-dim">
              Comps are unavailable right now.
            </div>
          )}

          {error && (
            <div className="rounded border border-alarm/40 bg-surface px-3 py-2 text-xs text-alarm">
              {error}
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={() => persist(false)}
              disabled={busy}
              data-testid="save-draft"
              className="flex-1 rounded border border-seam py-2 text-sm text-text-dim hover:bg-surface-2 hover:text-text disabled:opacity-50"
            >
              <span className="nameplate">Save draft</span>
            </button>
            <button
              onClick={() => persist(true)}
              disabled={busy}
              data-testid="submit-for-approval"
              className="flex-1 rounded bg-data/15 py-2 text-sm text-data hover:bg-data/25 disabled:opacity-50"
            >
              <span className="nameplate">
                {busy ? "Saving…" : "Submit for approval"}
              </span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
