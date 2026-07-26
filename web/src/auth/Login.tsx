// Login screen: centered graphite panel, PLENUM nameplate, email + password.
// One alarm line on failure — the server returns identical bodies for unknown
// email vs wrong password (anti-enumeration), and we do not undo that: every
// failure reads "Invalid credentials." No demo-credentials hint here; the
// README owns credentials (rejected alternative).

import { useState } from "react";
import { LogIn } from "lucide-react";
import { ApiError } from "../lib/api";
import { useScreenReady } from "../lib/useScreenReady";
import { useLogin } from "./auth";

export function Login() {
  useScreenReady(true, "login"); // the form is the screen; ready as soon as it paints
  const login = useLogin();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  const failed = login.isError;
  const message =
    login.error instanceof ApiError && login.error.status !== 401
      ? "Something went wrong. Try again."
      : "Invalid credentials.";

  return (
    <div className="flex min-h-screen items-center justify-center bg-bg px-4">
      <div className="w-full max-w-sm">
        <div className="mb-6 text-center">
          <div className="nameplate-strong text-xl text-text">PLENUM</div>
          <div className="nameplate mt-1 text-2xs text-text-dim">
            Installed-base revenue control
          </div>
        </div>

        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!login.isPending) login.mutate({ email, password });
          }}
          className="rounded-lg border border-seam bg-surface p-6"
          noValidate
        >
          <label className="nameplate mb-1 block text-2xs text-text-dim">
            Email
          </label>
          <input
            type="email"
            autoComplete="username"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="mb-4 w-full rounded border border-seam bg-bg px-3 py-2 text-sm text-text outline-none focus:border-seam-strong"
            data-testid="login-email"
          />

          <label className="nameplate mb-1 block text-2xs text-text-dim">
            Password
          </label>
          <input
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="mb-5 w-full rounded border border-seam bg-bg px-3 py-2 text-sm text-text outline-none focus:border-seam-strong"
            data-testid="login-password"
          />

          <button
            type="submit"
            disabled={login.isPending}
            className="flex w-full items-center justify-center gap-2 rounded bg-data/15 py-2 text-sm font-medium text-data transition-colors hover:bg-data/25 disabled:opacity-50"
            data-testid="login-submit"
          >
            <LogIn size={15} strokeWidth={2} />
            {login.isPending ? "Signing in…" : "Sign in"}
          </button>

          <div
            className="mt-3 h-4 text-center text-xs text-alarm"
            role="alert"
            data-testid="login-error"
          >
            {failed ? message : ""}
          </div>
        </form>
      </div>
    </div>
  );
}
