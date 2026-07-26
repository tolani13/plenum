// D-3 (2026-07-26): the acceptance hook for the blank-screen blast door.
//
// An error boundary can only be proven by a THROW DURING RENDER — a thrown
// error in an event handler or a rejected promise never reaches
// componentDidCatch. There is no way to force that from the console without a
// component that co-operates, so this one does: it listens for
// RENDER_ERROR_EVENT and, once armed, throws on its next render.
//
// It renders nothing, holds no state until fired, and lives inside the same
// boundary that catches real screen failures — so what D. sees when he fires
// it is exactly what a real uncaught render error will look like.
//
// Two are mounted, one per boundary, so each layer can be proven on its own:
//   · inside the Shell  -> RENDER_ERROR_EVENT      -> screen boundary (nav survives)
//   · at the app root   -> ROOT_RENDER_ERROR_EVENT -> root boundary   (backstop)

import { useEffect, useState } from "react";
import { RENDER_ERROR_EVENT } from "../lib/events";

export function RenderErrorProbe({
  event = RENDER_ERROR_EVENT,
}: {
  event?: string;
}) {
  const [armed, setArmed] = useState(false);

  useEffect(() => {
    const arm = () => setArmed(true);
    window.addEventListener(event, arm);
    return () => window.removeEventListener(event, arm);
  }, [event]);

  if (armed) {
    throw new Error(`forced render error (${event})`);
  }
  return null;
}
