// Metrics query hooks. Every request fetches the whole scoped result
// (limit=200) and sends basis=net FIXED — the payload is dual-basis, so the
// displayed basis is a client-side choice and the query key omits it. That is
// what makes the GROSS/NET toggle a pure re-render (no refetch). period / kind
// / group DO change which rows exist, so they ARE in the key and refetch.

import { useQuery } from "@tanstack/react-query";
import { apiGet } from "./api";
import type {
  CoverageRow,
  CustomerRow,
  DefectionRow,
  ItemRow,
  LeaderboardRow,
  Page,
  TerritoryRow,
} from "./types";
import type { Group, Kind } from "./params";

const LIMIT = 200;
const q = encodeURIComponent;

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
