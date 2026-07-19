// Route guard. Everything except /login sits behind this. It gates on
// GET /api/auth/me: while it resolves, a quiet graphite splash (never a flash
// of another identity's data); on 401/error, straight to /login.

import { Navigate, Outlet } from "react-router";
import { useMe } from "./auth";

export function RequireAuth() {
  const me = useMe();

  if (me.isLoading) {
    return (
      <div
        data-testid="boot-splash"
        className="flex min-h-screen items-center justify-center bg-bg"
      >
        <span className="nameplate text-2xs text-text-dim">PLENUM</span>
      </div>
    );
  }

  if (me.isError || !me.data) {
    return <Navigate to="/login" replace />;
  }

  return <Outlet />;
}
