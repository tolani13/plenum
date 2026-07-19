// The single QueryClient + the identity-safety wiring (architect resolution 12:
// a cross-user data flash is a TENANCY bug, not cosmetics).
//
//   · retry: false        — a metrics 4xx (e.g. 422 the UI shouldn't have sent)
//                            must surface once, never loop (constraint 3).
//   · a 401 anywhere       — the session is gone (e.g. the API restarted and its
//                            MemoryStore was wiped). We emit ONE event; the app
//                            root purges the cache and returns to /login. No
//                            stale tenant data can survive that.
//
// The other half of the purge (clear on EVERY login success, before the new
// identity's queries run) lives in the auth provider.

import { QueryCache, QueryClient } from "@tanstack/react-query";
import { ApiError } from "./api";

/** Fired when any query sees a 401. The app root listens and hard-resets auth. */
export const UNAUTHORIZED_EVENT = "plenum:unauthorized";

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error) => {
      if (error instanceof ApiError && error.status === 401) {
        window.dispatchEvent(new Event(UNAUTHORIZED_EVENT));
      }
    },
  }),
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
      staleTime: 30_000,
    },
  },
});
