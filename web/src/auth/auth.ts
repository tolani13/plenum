// Auth actions + identity query. The cache-purge half of the tenancy safety
// lives here (architect resolution 12): clear() on EVERY login success (before
// the new identity's queries run) and on logout. The other half (a 401 mid-
// session) is in lib/queryClient.ts.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router";
import { apiGet, apiPost } from "../lib/api";
import type { Me } from "../lib/types";

/** The logged-in identity (id, name, email, role, scoped territory codes). */
export function useMe() {
  return useQuery<Me>({
    queryKey: ["me"],
    queryFn: () => apiGet<Me>("/api/auth/me"),
  });
}

interface Credentials {
  email: string;
  password: string;
}

export function useLogin() {
  const qc = useQueryClient();
  const navigate = useNavigate();
  return useMutation({
    mutationFn: (creds: Credentials) =>
      apiPost<{ id: string }>("/api/auth/login", creds),
    onSuccess: () => {
      // Purge any prior identity's cache BEFORE the new identity loads. The
      // guard then refetches `me` fresh (with territories) — no stale tenant
      // data can flash on Command's first paint.
      qc.clear();
      navigate("/command", { replace: true });
    },
  });
}

export function useLogout() {
  const qc = useQueryClient();
  const navigate = useNavigate();
  const done = () => {
    qc.clear();
    navigate("/login", { replace: true });
  };
  return useMutation({
    mutationFn: () => apiPost<null>("/api/auth/logout"),
    onSuccess: done,
    onError: done, // drop local state and return to login regardless
  });
}
