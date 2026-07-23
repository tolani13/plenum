// P4 signal hooks — the queue reads and the write-back mutations. Writes
// invalidate the signals caches (list + summary share the "signals" key root)
// and the account caches (the 360 signals panel), so a card that changes
// state leaves every surface at once.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost } from "./api";
import { FETCH_LIMIT as LIMIT, q } from "./fetchAll";
import type { AssigneeRow, Page, SignalRow, SignalsSummary } from "./types";

/** P5 (R4): the queue's status filter gains the machine-retired shelf. */
export type QueueFilter = "active" | "actioned" | "dismissed" | "expired";

export function useSignals(status: QueueFilter) {
  return useQuery({
    queryKey: ["signals", "list", status],
    queryFn: () =>
      apiGet<Page<SignalRow>>(`/api/signals?status=${q(status)}&limit=${LIMIT}`),
  });
}

export function useSignalsSummary() {
  return useQuery({
    queryKey: ["signals", "summary"],
    queryFn: () => apiGet<SignalsSummary>(`/api/signals/summary`),
  });
}

/** The R6 picker feed — fetched lazily (enabled) when a picker opens. */
export function useAssignees(accountId: string, enabled: boolean) {
  return useQuery({
    queryKey: ["signals", "assignees", accountId],
    queryFn: () =>
      apiGet<{ items: AssigneeRow[] }>(
        `/api/signals/assignees?account_id=${q(accountId)}`,
      ),
    enabled,
    staleTime: 5 * 60_000, // team rosters are stable
  });
}

function useSignalInvalidate() {
  const qc = useQueryClient();
  return (accountId?: string) => {
    qc.invalidateQueries({ queryKey: ["signals"] });
    qc.invalidateQueries({ queryKey: ["accounts"] });
    if (accountId) qc.invalidateQueries({ queryKey: ["account", accountId] });
  };
}

export function useAssignSignal() {
  const invalidate = useSignalInvalidate();
  return useMutation({
    mutationFn: (input: { id: string; assignee_id: string }) =>
      apiPost<SignalRow>(`/api/signals/${input.id}/assign`, {
        assignee_id: input.assignee_id,
      }),
    onSuccess: (row) => invalidate(row.account_id),
  });
}

export function useActionSignal() {
  const invalidate = useSignalInvalidate();
  return useMutation({
    mutationFn: (input: { id: string; outcome: string }) =>
      apiPost<SignalRow>(`/api/signals/${input.id}/action`, {
        outcome: input.outcome,
      }),
    onSuccess: (row) => invalidate(row.account_id),
  });
}

export function useDismissSignal() {
  const invalidate = useSignalInvalidate();
  return useMutation({
    mutationFn: (input: { id: string; reason: string }) =>
      apiPost<SignalRow>(`/api/signals/${input.id}/dismiss`, {
        reason: input.reason,
      }),
    onSuccess: (row) => invalidate(row.account_id),
  });
}
