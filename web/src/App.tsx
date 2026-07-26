import { useEffect } from "react";
import {
  Navigate,
  Route,
  Routes,
  useLocation,
  useNavigate,
} from "react-router";
import { useQueryClient } from "@tanstack/react-query";
import { UNAUTHORIZED_EVENT } from "./lib/queryClient";
import { RequireAuth } from "./auth/RequireAuth";
import { Login } from "./auth/Login";
import { Shell } from "./shell/Shell";
import { Command } from "./command/Command";
import { Leaderboards } from "./leaderboards/Leaderboards";
import { Pipeline } from "./crm/Pipeline";
import { Quotes } from "./crm/Quotes";
import { QuoteDetail } from "./crm/QuoteDetail";
import { QuoteBuilder } from "./crm/QuoteBuilder";
import { Account360 } from "./crm/Account360";
import { Signals } from "./signals/Signals";
import { LazyRoute, screenLoader } from "./components/LazyRoute";

// P5 (R7): the heavy routes load on demand — Ask + Leakage carry recharts,
// the Map carries the state geometry — which puts the main chunk back under
// Vite's 500 KB line. The fallback is the house loading panel.
//
// D-3 (2026-07-26): the loaders are now module-scope constants handed to
// <LazyRoute>, which owns the Suspense fallback AND an error boundary. The
// old shape wrapped these in <Suspense> alone; a chunk that failed to
// download rethrew during render, no boundary caught it, and React unmounted
// the entire document. See components/ErrorBoundary.tsx.
const loadAsk = screenLoader(() => import("./ask/Ask"), "Ask");
const loadTerritoryMap = screenLoader(
  () => import("./map/TerritoryMap"),
  "TerritoryMap",
);
const loadLeakage = screenLoader(() => import("./leakage/Leakage"), "Leakage");
const loadDataQuality = screenLoader(
  () => import("./dq/DataQuality"),
  "DataQuality",
);

export function App() {
  const qc = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();

  // A 401 from ANY query (queryClient.ts dispatches it) means the session is
  // gone — purge the cache and return to login. Guarded so /login can't loop.
  useEffect(() => {
    const handler = () => {
      qc.clear();
      if (location.pathname !== "/login") {
        navigate("/login", { replace: true });
      }
    };
    window.addEventListener(UNAUTHORIZED_EVENT, handler);
    return () => window.removeEventListener(UNAUTHORIZED_EVENT, handler);
  }, [qc, navigate, location.pathname]);

  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route element={<RequireAuth />}>
        <Route element={<Shell />}>
          <Route index element={<Navigate to="/command" replace />} />
          <Route path="/command" element={<Command />} />
          <Route
            path="/map"
            element={<LazyRoute load={loadTerritoryMap} name="Territory Map" />}
          />
          <Route path="/leaderboards" element={<Leaderboards />} />
          <Route
            path="/leakage"
            element={<LazyRoute load={loadLeakage} name="Leakage" />}
          />
          <Route path="/pipeline" element={<Pipeline />} />
          <Route path="/quotes" element={<Quotes />} />
          <Route path="/quotes/new" element={<QuoteBuilder />} />
          <Route path="/quotes/:id" element={<QuoteDetail />} />
          <Route path="/accounts/:id" element={<Account360 />} />
          <Route path="/signals" element={<Signals />} />
          <Route path="/ask" element={<LazyRoute load={loadAsk} name="Ask" />} />
          <Route
            path="/data-quality"
            element={<LazyRoute load={loadDataQuality} name="Data Quality" />}
          />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/command" replace />} />
    </Routes>
  );
}
