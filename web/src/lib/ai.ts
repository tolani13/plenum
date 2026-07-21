// P4 AI hooks. /api/ai/status gates every AI affordance (no probing errors —
// R8); ask and the discount recommendation are on-demand mutations, never
// fired per keystroke.

import { useMutation, useQuery } from "@tanstack/react-query";
import { apiGet, apiPost } from "./api";
import type { AiStatus, AskResult, DiscountRec } from "./types";

export function useAiStatus() {
  return useQuery({
    queryKey: ["ai", "status"],
    queryFn: () => apiGet<AiStatus>(`/api/ai/status`),
    staleTime: 5 * 60_000, // flags change only with an API restart
  });
}

export function useAsk() {
  return useMutation({
    mutationFn: (question: string) =>
      apiPost<AskResult>(`/api/ai/ask`, { question }),
  });
}

export interface DiscountRecInput {
  product_id: string;
  account_id: string;
  qty: number;
  discount_pct: number;
}

export function useDiscountRec() {
  return useMutation({
    mutationFn: (input: DiscountRecInput) =>
      apiPost<DiscountRec>(`/api/ai/discount-recommendation`, input),
  });
}
