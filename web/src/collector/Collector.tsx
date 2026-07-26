// B-1 (2026-07-26): the collector demo, ported into PLENUM as the tenth
// screen. This was the demo's App.tsx — its app ROOT — and it is now a
// screen: no providers, no router, no page chrome of its own beyond the
// tab switcher, because PLENUM's Shell already owns all of that.
//
// What it is: a simulated cartridge dust collector with two views — the 3D
// unit (Showcase) and the telemetry dashboard (Dashboard). The simulator is
// self-contained and deterministic (seed 20260717, the same seed epoch as
// PLENUM's own fixture data) and makes NO network calls. Pushing its
// telemetry into PLENUM's reorder branch is B-2, deliberately a separate
// unit because it touches an admin-gated write.
//
// De-branded on the way in: the vendor name, the product line and the
// standalone demo's disclaimer line are gone. The domain vocabulary — cartridge,
// pulse-jet, dP, in. w.g., plenum, hopper, NFPA — stays, because those are
// industry terms and the audience is a domain audience.

import { useState } from "react";
import { Activity, Box } from "lucide-react";
import { useScreenReady } from "../lib/useScreenReady";
import Showcase from "./Showcase";
import Dashboard from "./Dashboard";
import { useSim } from "./simulator";

type View = "unit" | "intelligence";

export function Collector() {
  const [view, setView] = useState<View>("unit");
  const { live, history } = useSim();

  // The simulator builds its 14-day history synchronously before first paint,
  // so the screen is genuinely settled as soon as a sample exists — there is
  // no request in flight to wait for.
  useScreenReady(history.length > 0 && live.dp > 0, "collector");

  const tab = (v: View, icon: React.ReactNode, label: string) => (
    <button
      onClick={() => setView(v)}
      aria-current={view === v}
      data-testid={`collector-tab-${v}`}
      className={`flex items-center gap-2 rounded px-3 py-1.5 text-sm transition-colors ${
        view === v ? "bg-surface-2 text-text" : "text-text-dim hover:text-text"
      }`}
    >
      {icon}
      <span className="hidden sm:inline">{label}</span>
    </button>
  );

  return (
    <div className="mx-auto flex w-full min-w-0 max-w-[1500px] flex-col gap-4">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded border border-ok/40">
            <span className="h-3 w-3 rounded-full border-2 border-ok" />
          </span>
          <div className="min-w-0">
            <h1 className="nameplate-strong truncate text-xl leading-none text-text">
              Collector telemetry
            </h1>
            <div className="truncate text-2xs text-text-faint">
              Connected-equipment concept · simulated unit
            </div>
          </div>
        </div>

        <nav className="flex items-center gap-1 rounded border border-seam bg-surface p-1">
          {tab("unit", <Box className="h-4 w-4" />, "Unit")}
          {tab("intelligence", <Activity className="h-4 w-4" />, "Intelligence")}
        </nav>

        <div className="hidden items-center gap-2 text-xs text-text-dim md:flex">
          <span className="led" />
          <span className="tabular">
            {live.cfm.toLocaleString()} CFM · streaming
          </span>
        </div>
      </header>

      <div className="min-w-0">
        {view === "unit" ? <Showcase /> : <Dashboard />}
      </div>
    </div>
  );
}
