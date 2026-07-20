// P3 CRM data hooks — reads (TanStack Query) and the first WRITE surface in the
// app (TanStack mutations + targeted invalidation). A booked order must make
// the leaderboard/customer/360 numbers move without a manual reload, so
// patchStage invalidates the metrics + opportunities + account caches on
// success (gate P3-2's "immediately reflects" is real, not a reload artifact).

import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { apiGet, apiPatch, apiPost } from "./api";
import type {
  AccountDetail,
  AccountItem,
  AuditRow,
  DiscountPolicy,
  OppRow,
  OppStage,
  Page,
  ProductItem,
  QuoteDetail,
  QuoteListRow,
  StageResult,
} from "./types";

const q = encodeURIComponent;
const LIMIT = 200;

// ── reads ───────────────────────────────────────────────────────────────────

export function useOpportunities(stage?: OppStage | "all") {
  const filter = stage && stage !== "all" ? `&stage=${stage}` : "";
  return useQuery({
    queryKey: ["opportunities", stage ?? "all"],
    queryFn: () =>
      apiGet<Page<OppRow>>(`/api/opportunities?limit=${LIMIT}${filter}`),
  });
}

export function useAccountsList() {
  return useQuery({
    queryKey: ["accounts", "list"],
    queryFn: () => apiGet<Page<AccountItem>>(`/api/accounts?limit=${LIMIT}`),
  });
}

export function useAccountDetail(id: string | undefined) {
  return useQuery({
    queryKey: ["account", id],
    queryFn: () => apiGet<AccountDetail>(`/api/accounts/${q(id!)}`),
    enabled: !!id,
  });
}

export function useProducts() {
  return useQuery({
    queryKey: ["products"],
    queryFn: () => apiGet<Page<ProductItem>>(`/api/products?limit=${LIMIT}`),
    staleTime: 5 * 60_000, // the catalog is stable
  });
}

export function useDiscountPolicy() {
  return useQuery({
    queryKey: ["policy", "discount"],
    queryFn: () => apiGet<DiscountPolicy>(`/api/policy/discount`),
    staleTime: 5 * 60_000,
  });
}

export function useQuotesList(view: "mine" | "approvals") {
  return useQuery({
    queryKey: ["quotes", view],
    queryFn: () =>
      apiGet<Page<QuoteListRow>>(`/api/quotes?view=${view}&limit=${LIMIT}`),
  });
}

export function useQuoteDetail(id: string | undefined) {
  return useQuery({
    queryKey: ["quote", id],
    queryFn: () => apiGet<QuoteDetail>(`/api/quotes/${q(id!)}`),
    enabled: !!id,
  });
}

export function useQuoteAudit(id: string | undefined) {
  return useQuery({
    queryKey: ["quote-audit", id],
    queryFn: () =>
      apiGet<{ items: AuditRow[]; total: number }>(
        `/api/quotes/${q(id!)}/audit`,
      ),
    enabled: !!id,
  });
}

// ── writes ──────────────────────────────────────────────────────────────────

/** Invalidate every cache a write can move (metrics, pipeline, accounts). */
function useCrmInvalidate() {
  const qc = useQueryClient();
  return (accountId?: string) => {
    qc.invalidateQueries({ queryKey: ["opportunities"] });
    qc.invalidateQueries({ queryKey: ["quotes"] });
    qc.invalidateQueries({ queryKey: ["metrics"] });
    qc.invalidateQueries({ queryKey: ["accounts"] });
    if (accountId) qc.invalidateQueries({ queryKey: ["account", accountId] });
  };
}

export interface CreateOppInput {
  account_id: string;
  kind: string;
  amount_cents: number;
  expected_close?: string;
}

export function useCreateOpportunity() {
  const invalidate = useCrmInvalidate();
  return useMutation({
    mutationFn: (input: CreateOppInput) =>
      apiPost<OppRow>("/api/opportunities", input),
    onSuccess: (opp) => invalidate(opp.account_id),
  });
}

export function usePatchStage() {
  const invalidate = useCrmInvalidate();
  return useMutation({
    mutationFn: (input: { oppId: string; stage: OppStage; lost_reason?: string }) =>
      apiPatch<StageResult>(`/api/opportunities/${input.oppId}/stage`, {
        stage: input.stage,
        lost_reason: input.lost_reason,
      }),
    onSuccess: (result) => invalidate(result.opportunity.account_id),
  });
}

export interface QuoteLineInput {
  product_id: string;
  qty: number;
  discount_pct: number;
}

export function useCreateQuote() {
  const invalidate = useCrmInvalidate();
  return useMutation({
    mutationFn: (input: { opportunity_id: string; lines: QuoteLineInput[] }) =>
      apiPost<QuoteDetail>("/api/quotes", input),
    onSuccess: (quote) => invalidate(quote.account_id),
  });
}

export function useSubmitQuote() {
  const invalidate = useCrmInvalidate();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiPost<QuoteDetail>(`/api/quotes/${id}/submit`),
    onSuccess: (quote) => {
      invalidate(quote.account_id);
      qc.invalidateQueries({ queryKey: ["quote", quote.id] });
      qc.invalidateQueries({ queryKey: ["quote-audit", quote.id] });
    },
  });
}

export function useApproveQuote() {
  const invalidate = useCrmInvalidate();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiPost<QuoteDetail>(`/api/quotes/${id}/approve`),
    onSuccess: (quote) => {
      invalidate(quote.account_id);
      qc.invalidateQueries({ queryKey: ["quote", quote.id] });
      qc.invalidateQueries({ queryKey: ["quote-audit", quote.id] });
    },
  });
}

export function useRejectQuote() {
  const invalidate = useCrmInvalidate();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: { id: string; reason: string }) =>
      apiPost<QuoteDetail>(`/api/quotes/${input.id}/reject`, {
        reason: input.reason,
      }),
    onSuccess: (quote) => {
      invalidate(quote.account_id);
      qc.invalidateQueries({ queryKey: ["quote", quote.id] });
      qc.invalidateQueries({ queryKey: ["quote-audit", quote.id] });
    },
  });
}

export interface CreateActivityInput {
  account_id: string;
  kind: string;
  body: string;
}

export function useCreateActivity() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateActivityInput) =>
      apiPost("/api/activities", input),
    onSuccess: (_data, input) => {
      qc.invalidateQueries({ queryKey: ["account", input.account_id] });
    },
  });
}
