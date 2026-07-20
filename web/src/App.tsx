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
          <Route path="/leaderboards" element={<Leaderboards />} />
          <Route path="/pipeline" element={<Pipeline />} />
          <Route path="/quotes" element={<Quotes />} />
          <Route path="/quotes/new" element={<QuoteBuilder />} />
          <Route path="/quotes/:id" element={<QuoteDetail />} />
          <Route path="/accounts/:id" element={<Account360 />} />
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/command" replace />} />
    </Routes>
  );
}
