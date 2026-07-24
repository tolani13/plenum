// T1 — territory editing (planning view) hooks. The editor's disclosed read
// (GET /api/territories, vp/admin) + the four writes. Every mutation
// invalidates the states feed (the map's grouping/coloring/planning sums
// regroup from the server's live territory_states — the frontend never
// recomputes geography client-side, constraint 9) and the admin list.
// The Territory Board feed (["metrics","territories"]) is deliberately NOT
// invalidated: the planning-view law says map edits move nothing official,
// and the API guarantees it — there is nothing to refetch.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiDelete, apiGet, apiPost, apiPut, apiPatch } from "./api";
import type { AdminTerritory, AssignedState, Page } from "./types";

/** The eight planning chips (tokens.css --color-terr-plan-*; the API
 *  validates color_token against the same names — one list, mirrored). */
export const PLANNING_PALETTE = [
  "terr-plan-1",
  "terr-plan-2",
  "terr-plan-3",
  "terr-plan-4",
  "terr-plan-5",
  "terr-plan-6",
  "terr-plan-7",
  "terr-plan-8",
] as const;

export function useTerritoriesAdmin(enabled: boolean) {
  return useQuery({
    queryKey: ["territories-admin"],
    queryFn: () => apiGet<Page<AdminTerritory>>("/api/territories"),
    enabled,
  });
}

function useGeoInvalidate() {
  const qc = useQueryClient();
  return () => {
    qc.invalidateQueries({ queryKey: ["metrics", "states"] });
    qc.invalidateQueries({ queryKey: ["territories-admin"] });
  };
}

export function useAssignState() {
  const invalidate = useGeoInvalidate();
  return useMutation({
    mutationFn: (input: { state_code: string; territory_code: string }) =>
      apiPut<AssignedState>(`/api/territory-states/${input.state_code}`, {
        territory_code: input.territory_code,
      }),
    onSuccess: invalidate,
  });
}

export interface CreateTerritoryInput {
  code: string;
  name: string;
  region: string;
  color_token?: string;
}

export function useCreateTerritory() {
  const invalidate = useGeoInvalidate();
  return useMutation({
    mutationFn: (input: CreateTerritoryInput) =>
      apiPost<AdminTerritory>("/api/territories", input),
    onSuccess: invalidate,
  });
}

export function usePatchTerritory() {
  const invalidate = useGeoInvalidate();
  return useMutation({
    mutationFn: (input: { code: string; name?: string; color_token?: string }) =>
      apiPatch<AdminTerritory>(`/api/territories/${input.code}`, {
        name: input.name,
        color_token: input.color_token,
      }),
    onSuccess: invalidate,
  });
}

export function useDeleteTerritory() {
  const invalidate = useGeoInvalidate();
  return useMutation({
    mutationFn: (code: string) =>
      apiDelete<{ id: string; code: string; deleted: boolean }>(
        `/api/territories/${code}`,
      ),
    onSuccess: invalidate,
  });
}
