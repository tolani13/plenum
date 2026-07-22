// Cross-screen window events. P5: ASK_FOCUS_EVENT moved here from Ask.tsx —
// the Shell listens for Cmd-K everywhere, but Ask is now a lazy route (R7),
// and importing the constant from Ask.tsx would drag the whole recharts
// chunk back into the main bundle.

/** Re-focus event the Shell fires on Cmd-K/Ctrl-K when already on /ask. */
export const ASK_FOCUS_EVENT = "plenum:ask-focus";
