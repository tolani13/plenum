// Cross-screen window events. P5: ASK_FOCUS_EVENT moved here from Ask.tsx —
// the Shell listens for Cmd-K everywhere, but Ask is now a lazy route (R7),
// and importing the constant from Ask.tsx would drag the whole recharts
// chunk back into the main bundle.

/** Re-focus event the Shell fires on Cmd-K/Ctrl-K when already on /ask. */
export const ASK_FOCUS_EVENT = "plenum:ask-focus";

// D-3 (2026-07-26): the acceptance hook for the blank-screen blast door. One
// line in the console —
//   window.dispatchEvent(new Event('plenum-test-render-error'))
// — makes a component throw DURING RENDER, which is the only way to exercise
// an error boundary from outside React. Deliberately un-namespaced: D. types
// it by hand. It arms nothing until fired, and the boundary it proves is the
// same one that catches real failures.
/** Forces an uncaught render error under the SHELL — proves the screen
 *  boundary: the panel appears where the screen was, the nav stays put. */
export const RENDER_ERROR_EVENT = "plenum-test-render-error";

/** Forces one ABOVE the shell — proves the root boundary, the backstop that
 *  catches what no screen boundary can see (the Shell itself, RequireAuth,
 *  Login, the router). The panel replaces the page; the document is never
 *  empty, which is the whole law. */
export const ROOT_RENDER_ERROR_EVENT = "plenum-test-root-error";
