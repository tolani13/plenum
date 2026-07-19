import { useEffect } from "react";

// The tripwire (and any harness) waits for body[data-screen-ready="true"]
// before measuring — it means the active screen's queries have settled
// (success OR error), so what's on screen is final, not mid-fetch. Each screen
// calls this with the settled-ness of its own queries.
export function useScreenReady(ready: boolean): void {
  useEffect(() => {
    document.body.dataset.screenReady = ready ? "true" : "false";
    return () => {
      document.body.dataset.screenReady = "false";
    };
  }, [ready]);
}
