// Metrics query hooks. Every request fetches the whole scoped result
// (limit=200) and sends basis=net FIXED — the payload is dual-basis, so the
// displayed basis is a client-side choice and the query key omits it. That is
// what makes the GROSS/NET toggle a pure re-render (no refetch). period / kind
// / group DO change which rows exist, so they ARE in the key and refetch.

import { useQuery } from "@tanstack/react-query";
import { apiGet } from "./api";
import { FETCH_LIMIT as LIMIT, q } from "./fetchAll";
import type {
  CoverageRow,
  CustomerRow,
  DataQualityBody,
  DefectionRow,
  ItemRow,
  LeaderboardRow,
  LeakagePage,
  Page,
  StatesPage,
  TerritoryRow,
} from "./types";
import type { Group, Kind } from "./params";

export function useTerritories(period: string) {
  return useQuery({
    queryKey: ["metrics", "territories", period],
    queryFn: () =>
      apiGet<Page<TerritoryRow>>(
        `/api/metrics/territories?period=${q(period)}&basis=net&limit=${LIMIT}`,
      ),
  });
}

// Coverage takes basis only (rejects period + kind with 422 by design).
export function useCoverage() {
  return useQuery({
    queryKey: ["metrics", "coverage"],
    queryFn: () =>
      apiGet<Page<CoverageRow>>(`/api/metrics/coverage?basis=net&limit=${LIMIT}`),
  });
}

// Defection takes limit/offset only (rejects period/basis/kind with 422).
export function useDefection() {
  return useQuery({
    queryKey: ["metrics", "defection"],
    queryFn: () =>
      apiGet<Page<DefectionRow>>(`/api/metrics/defection?limit=${LIMIT}`),
  });
}

export function useLeaderboard(period: string, kind: Kind) {
  return useQuery({
    queryKey: ["metrics", "leaderboard", period, kind],
    queryFn: () =>
      apiGet<Page<LeaderboardRow>>(
        `/api/metrics/leaderboard?period=${q(period)}&basis=net&kind=${kind}&limit=${LIMIT}`,
      ),
  });
}

export function useItems(period: string, kind: Kind, group: Group) {
  return useQuery({
    queryKey: ["metrics", "items", period, kind, group],
    queryFn: () =>
      apiGet<Page<ItemRow>>(
        `/api/metrics/items?period=${q(period)}&basis=net&kind=${kind}&group=${group}&limit=${LIMIT}`,
      ),
  });
}

export function useCustomers(period: string, kind: Kind) {
  return useQuery({
    queryKey: ["metrics", "customers", period, kind],
    queryFn: () =>
      apiGet<Page<CustomerRow>>(
        `/api/metrics/customers?period=${q(period)}&basis=net&kind=${kind}&limit=${LIMIT}`,
      ),
  });
}

// ── P5 additions ────────────────────────────────────────────────────────────

/** The Leakage screen's one fetch: distribution + heat follow period/kind;
 *  the outlier zone rides `outliers=policy` — the discount_anomaly
 *  generator's exact math, so feed rows and signal chips agree 1:1 (R1). */
export function useLeakage(period: string, kind: Kind) {
  return useQuery({
    queryKey: ["metrics", "leakage", period, kind],
    queryFn: () =>
      apiGet<LeakagePage>(
        `/api/metrics/leakage?period=${q(period)}&kind=${kind}&outliers=policy&limit=${LIMIT}`,
      ),
  });
}

/** Per-state money + the territory roster/coloring config (R3). Dual-basis
 *  payload — basis stays a client display choice, exactly like every other
 *  metrics hook. */
export function useStates(period: string) {
  return useQuery({
    queryKey: ["metrics", "states", period],
    queryFn: () =>
      apiGet<StatesPage>(
        `/api/metrics/states?period=${q(period)}&basis=net&limit=${LIMIT}`,
      ),
  });
}

/** The R2 finders — read-only, RLS-scoped mess census. */
export function useDataQuality() {
  return useQuery({
    queryKey: ["data-quality"],
    queryFn: () => apiGet<DataQualityBody>(`/api/data-quality`),
  });
}
