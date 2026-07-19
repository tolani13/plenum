// App shell: a left rail on wide screens, a wrapping top bar on narrow ones
// (width behaviour, not device sniffing). Nav is Command + Leaderboards ONLY —
// no dead links to unbuilt screens. The user chip proves identity at a glance
// (name · role · scope codes); logout purges the cache on the way out.

import { NavLink, Outlet } from "react-router";
import { BarChart3, Columns3, FileText, LayoutDashboard, LogOut } from "lucide-react";
import { useMe, useLogout } from "../auth/auth";

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

  return (
    <div className="flex min-h-screen flex-col bg-bg text-text md:flex-row">
      <aside className="flex flex-row flex-wrap items-center gap-x-4 gap-y-2 border-b border-seam bg-surface px-4 py-3 md:sticky md:top-0 md:h-screen md:w-52 md:shrink-0 md:flex-col md:flex-nowrap md:items-stretch md:gap-y-1 md:border-b-0 md:border-r">
        <div className="nameplate-strong text-base text-text md:mb-4">
          PLENUM
        </div>

        <nav className="flex flex-row gap-1 md:mt-2 md:flex-col">
          <NavItem
            to="/command"
            icon={<LayoutDashboard size={15} strokeWidth={2} />}
            label="Command"
          />
          <NavItem
            to="/leaderboards"
            icon={<BarChart3 size={15} strokeWidth={2} />}
            label="Leaderboards"
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
        </nav>

        <div className="ml-auto flex items-center gap-3 md:ml-0 md:mt-auto md:flex-col md:items-stretch md:gap-2">
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

      <main className="min-w-0 flex-1 px-4 py-5 sm:px-6 lg:px-8">
        <Outlet />
      </main>
    </div>
  );
}
