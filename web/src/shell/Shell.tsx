// App shell: a left rail on wide screens, a wrapping top bar on narrow ones
// (width behaviour, not device sniffing). Nav carries only BUILT screens —
// no dead links. The user chip proves identity at a glance (name · role ·
// scope codes); logout purges the cache on the way out.
//
// P4: Cmd-K / Ctrl-K is the global Ask PLENUM affordance (R9) — it jumps to
// /ask and focuses the question input (already there → just re-focuses).
// P5: three new entries (Territory Map after Command — R3; Leakage between
// Leaderboards and Pipeline — R1; Data Quality last — R2) and the
// + New account affordance (R9a) for every role.

import { useEffect, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router";
import {
  BarChart3,
  Columns3,
  FileText,
  LayoutDashboard,
  ListChecks,
  LogOut,
  Map as MapIcon,
  MessageSquareText,
  Plus,
  Radar,
  TrendingDown,
} from "lucide-react";
import { useMe, useLogout } from "../auth/auth";
import { ASK_FOCUS_EVENT } from "../lib/events";
import { ErrorBoundary } from "../components/ErrorBoundary";
import { RenderErrorProbe } from "../components/RenderErrorProbe";
import { NewAccountDialog } from "./NewAccountDialog";

function scopeLabel(territories: string[]): string {
  if (territories.length === 8) return "ALL";
  if (territories.length === 0) return "—";
  return territories.join("+");
}

function NavItem({
  to,
  icon,
  label,
}: {
  to: string;
  icon: React.ReactNode;
  label: string;
}) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        [
          "flex items-center gap-2 rounded px-3 py-2 text-sm transition-colors",
          isActive
            ? "bg-surface-2 text-text"
            : "text-text-dim hover:bg-surface-2 hover:text-text",
        ].join(" ")
      }
    >
      {icon}
      <span className="nameplate text-2xs">{label}</span>
    </NavLink>
  );
}

export function Shell() {
  const me = useMe();
  const logout = useLogout();
  const navigate = useNavigate();
  const location = useLocation();
  const [newAccountOpen, setNewAccountOpen] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        if (location.pathname === "/ask") {
          window.dispatchEvent(new Event(ASK_FOCUS_EVENT));
        } else {
          navigate("/ask");
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, location.pathname]);

  return (
    <div className="flex min-h-screen flex-col bg-bg text-text md:flex-row">
      <aside className="flex flex-row flex-wrap items-center gap-x-4 gap-y-2 border-b border-seam bg-surface px-4 py-3 md:sticky md:top-0 md:h-screen md:w-52 md:shrink-0 md:flex-col md:flex-nowrap md:items-stretch md:gap-y-1 md:border-b-0 md:border-r">
        <div className="nameplate-strong text-base text-text md:mb-4">
          PLENUM
        </div>

        <nav className="flex w-full flex-row flex-wrap gap-1 md:mt-2 md:w-auto md:flex-col md:flex-nowrap">
          <NavItem
            to="/command"
            icon={<LayoutDashboard size={15} strokeWidth={2} />}
            label="Command"
          />
          <NavItem
            to="/map"
            icon={<MapIcon size={15} strokeWidth={2} />}
            label="Territory Map"
          />
          <NavItem
            to="/leaderboards"
            icon={<BarChart3 size={15} strokeWidth={2} />}
            label="Leaderboards"
          />
          <NavItem
            to="/leakage"
            icon={<TrendingDown size={15} strokeWidth={2} />}
            label="Leakage"
          />
          <NavItem
            to="/pipeline"
            icon={<Columns3 size={15} strokeWidth={2} />}
            label="Pipeline"
          />
          <NavItem
            to="/quotes"
            icon={<FileText size={15} strokeWidth={2} />}
            label="Quotes"
          />
          <NavItem
            to="/signals"
            icon={<Radar size={15} strokeWidth={2} />}
            label="Signals"
          />
          <NavItem
            to="/ask"
            icon={<MessageSquareText size={15} strokeWidth={2} />}
            label="Ask"
          />
          <NavItem
            to="/data-quality"
            icon={<ListChecks size={15} strokeWidth={2} />}
            label="Data Quality"
          />
        </nav>

        <div className="ml-auto flex items-center gap-3 md:ml-0 md:mt-auto md:flex-col md:items-stretch md:gap-2">
          <button
            onClick={() => setNewAccountOpen(true)}
            className="flex items-center gap-1.5 rounded border border-seam px-2 py-1.5 text-2xs text-data transition-colors hover:bg-surface-2 md:justify-center"
            data-testid="new-account"
            title="Create an account in your territory scope"
          >
            <Plus size={14} strokeWidth={2} />
            <span className="nameplate">New account</span>
          </button>
          {me.data && (
            <div
              className="min-w-0 text-right md:text-left"
              data-testid="user-chip"
            >
              <div className="truncate text-xs text-text">{me.data.name}</div>
              <div className="nameplate text-2xs text-text-dim">
                {me.data.role} · {scopeLabel(me.data.territories)}
              </div>
            </div>
          )}
          <button
            onClick={() => logout.mutate()}
            className="flex items-center gap-1.5 rounded px-2 py-1.5 text-2xs text-text-dim transition-colors hover:bg-surface-2 hover:text-text md:justify-center md:border md:border-seam"
            data-testid="logout"
            title="Sign out"
          >
            <LogOut size={14} strokeWidth={2} />
            <span className="nameplate">Logout</span>
          </button>
        </div>
      </aside>

      {/* D-3: every routed screen — the nine eager ones as much as the four
          lazy ones — renders inside this boundary, so a render error costs
          the content area and nothing else. resetKey = the pathname, so
          navigating away always clears a caught error. The lazy routes carry
          their own inner boundary (LazyRoute) with the chunk retry. */}
      <main className="min-w-0 flex-1 px-4 py-5 sm:px-6 lg:px-8">
        <ErrorBoundary region="screen" resetKey={location.pathname}>
          <RenderErrorProbe />
          <Outlet />
        </ErrorBoundary>
      </main>

      {newAccountOpen && (
        <NewAccountDialog onClose={() => setNewAccountOpen(false)} />
      )}
    </div>
  );
}
