import { useEffect } from "react";

// The tripwire (and any harness) waits for body[data-screen-ready="true"]
// before measuring — it means the active screen's queries have settled
// (success OR error), so what's on screen is final, not mid-fetch. Each screen
// calls this with the settled-ness of its own queries.
//
// D-4 (2026-07-26): it now also records WHICH screen is mounted, in
// body[data-screen]. That marker exists because of how D-4 was missed: the
// nine D-3 specs all loaded a route fresh and asserted on the URL, and the URL
// was right the whole time — /data-quality in the address bar with the map
// still on screen. A test that only reads the router cannot see a screen that
// failed to swap. This value is written by the SCREEN ITSELF, so it reports
// what is actually mounted rather than what the router intended.
//
// `screen` is required, not optional, so a new screen cannot quietly ship
// without a marker and inherit the previous screen's name.
export function useScreenReady(ready: boolean, screen: string): void {
  useEffect(() => {
    document.body.dataset.screen = screen;
    document.body.dataset.screenReady = ready ? "true" : "false";
    return () => {
      document.body.dataset.screenReady = "false";
    };
  }, [ready, screen]);
}
