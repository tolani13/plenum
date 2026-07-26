// D-3 (2026-07-26): one lazy route, with a door on it — and a retry that
// actually retries.
//
// This replaces App.tsx's module-level `lazy()` constants plus the bare
// <Suspense> wrapper. Two things it does that the old shape could not:
//
//  1. It puts an ErrorBoundary BETWEEN the route and the shell, so a chunk
//     that never downloads costs the user one panel, not the whole app.
//
//  2. It can genuinely re-fetch. TWO caches sit in the way, and both were
//     measured before this was written, not assumed:
//       · React.lazy memoizes the promise it created — once rejected, the
//         same lazy object rethrows the same error on every later render, so
//         a re-mount alone changes nothing. Hence a fresh lazy() per attempt.
//       · The BROWSER's module map stores the failure against the module URL
//         and never retries it. Proven in Chrome against this app: after an
//         aborted load of /src/leakage/Leakage.tsx, a second import() of the
//         identical specifier issued NO network request at all and rejected
//         instantly, while import() of the same file with a query string
//         appended fetched and resolved. A keyed re-mount therefore cannot
//         work either — the URL has to change.
//     So a retry re-imports the failed module under a one-time URL. The
//     browser names that URL in the error it throws ("Failed to fetch
//     dynamically imported module: <url>"), which is where it comes from; it
//     is accepted only if it is same-origin.
//
//  Where the URL is NOT recoverable (Safari's message omits it), the retry
//  falls back to the plain import. That is honest rather than clever: if the
//  module map has the failure cached the panel simply comes back, and the
//  user has lost nothing — no reload, no sign-out.
//
// THE LIMIT OF THE IN-PAGE RETRY, measured against the production build, not
// reasoned about: when the whole network is down, the screen's chunk is not
// the only casualty — its shared dependency chunk dies with it
// (Leakage-*.js AND BarChart-*.js, recharts). Re-importing the screen under a
// busted URL re-fetches the screen, but its own static import of
// ./BarChart-*.js resolves to the UNbusted URL, whose failure the module map
// still remembers, so the retry fails a second time. Nothing inside the
// document can clear that — the module map dies only with the document.
// So the panel escalates: first press re-imports (recovers the common case,
// no reload); if that fails too, the second press reloads the document, which
// is the only thing that can work. That reload was verified NOT to cost the
// user anything: session and URL both survive it (the MemoryStore is
// server-side — a reload restarts the page, not the API).
//
// ── D-4 (2026-07-26): the regression D-3 introduced, and why it happened ───
//
// D-3 replaced four distinct lazy() components with FOUR USES OF ONE
// COMPONENT TYPE. React reconciles by type and position, so navigating from
// one lazy route to another updated the existing LazyRoute in place instead
// of mounting a new one — and that turned out to be unsurvivable, for a
// reason worth writing down because it is not obvious:
//
//   A component that suspends never commits that render. React retries it
//   from the LAST COMMITTED state. The old code built its lazy() inside a
//   useMemo during render, so every retry recomputed the memo (deps still
//   differed from the committed ones), produced a BRAND-NEW lazy object,
//   called import() again, and suspended again — forever.
//
// Measured on the real app, one lazy→lazy click: 8 572 renders, 8 572 lazy()
// creations and 4 286 loader invocations in four seconds, still climbing at
// 22 506 / 11 253 six seconds later. `memo` equalled `render` exactly — the
// memo never once hit its cache, which is the signature of a render that
// never commits. Meanwhile React Router runs navigations in a transition, so
// <Suspense> kept showing the PREVIOUS screen throughout. The URL was right,
// no error was thrown, and the user simply saw the wrong screen while the tab
// spun a hot loop.
//
// Two independent fixes, because the bug had two independent causes:
//
//   1. IDENTITY. `LazyRoute` keys its inner component on the pathname, so
//      React sees four distinct instances rather than one shared one. The key
//      is applied HERE, not by the caller, so a new lazy route cannot forget
//      it. Pathname cannot collide: React Router matches exactly one route
//      element for a given pathname, so two different screens can never be at
//      the same pathname at the same time. It also stops per-route retry
//      state (`attempt`/`bust`) leaking from one screen onto the next, which
//      the shared instance did silently.
//   2. STABILITY. The lazy() is no longer built during render. `lazyFor`
//      returns the SAME component for the same (loader, bust, attempt), so a
//      render that never commits can no longer spawn a new one. Even if some
//      future change re-introduces a non-committing render, the loop cannot
//      come back — the second call returns the first call's component, whose
//      promise is already in flight.
//
// The loaders must be module-scope constants — `lazyFor` keys off their
// identity.

import {
  lazy,
  Suspense,
  useCallback,
  useState,
  type ComponentType,
  type LazyExoticComponent,
} from "react";
import { useLocation } from "react-router";
import { ErrorBoundary, describeRenderError, isChunkLoadError } from "./ErrorBoundary";
import { LoadingPanel } from "./states";

/** Loads a screen. `bust` — when given — is a same-origin URL to import in
 *  place of the static specifier, so the browser treats it as a new module. */
export type ScreenLoader = (
  bust?: string,
) => Promise<{ default: ComponentType }>;

/** Builds a loader for one named export of one lazily-imported module.
 *  The static `import()` stays analyzable, which is what keeps Vite's route
 *  chunking (and the 500 kB main-bundle law) intact. */
export function screenLoader(
  importer: () => Promise<Record<string, unknown>>,
  exportName: string,
): ScreenLoader {
  return (bust) =>
    (bust ? import(/* @vite-ignore */ bust) : importer()).then((m) => ({
      default: (m as Record<string, ComponentType>)[exportName],
    }));
}

/** Stable lazy components, one per (loader, bust, attempt). Built OUTSIDE
 *  render on purpose — see the D-4 note above: a suspending render never
 *  commits, so anything built during render is built again on every retry. */
const lazyCache = new Map<
  ScreenLoader,
  Map<string, LazyExoticComponent<ComponentType>>
>();

function lazyFor(
  load: ScreenLoader,
  bust: string | null,
  attempt: number,
): LazyExoticComponent<ComponentType> {
  let perLoader = lazyCache.get(load);
  if (!perLoader) {
    perLoader = new Map();
    lazyCache.set(load, perLoader);
  }
  // `attempt` is part of the identity so the D-3 retry still gets a genuinely
  // new component even when there is no URL to bust (the Safari fallback).
  const key = `${attempt}#${bust ?? ""}`;
  let component = perLoader.get(key);
  if (!component) {
    component = lazy(() => load(bust ?? undefined));
    perLoader.set(key, component);
  }
  return component;
}

/** The module URL the browser named in a failed dynamic import, if it named
 *  one, and only when it is ours. */
function failedModuleUrl(error: unknown): string | null {
  if (!(error instanceof Error)) return null;
  const found = /(https?:\/\/[^\s"'()]+)/.exec(error.message);
  if (!found) return null;
  try {
    const url = new URL(found[1]);
    return url.origin === window.location.origin ? url.href : null;
  } catch {
    return null;
  }
}

/** The same module, under a URL the module map has no verdict on. */
function retryUrl(url: string, attempt: number): string {
  const u = new URL(url);
  u.searchParams.set("d3-retry", String(attempt));
  return u.href;
}

export function LazyRoute({
  load,
  name,
}: {
  load: ScreenLoader;
  name: string;
}) {
  // D-4: the identity, applied here so no route can be added without it.
  const { pathname } = useLocation();
  return <LazyScreen key={pathname} load={load} name={name} />;
}

function LazyScreen({ load, name }: { load: ScreenLoader; name: string }) {
  const [{ attempt, bust }, setRetry] = useState<{
    attempt: number;
    bust: string | null;
  }>({ attempt: 0, bust: null });

  const Screen = lazyFor(load, bust, attempt);

  const retry = useCallback((error: unknown) => {
    setRetry((prev) => {
      const next = prev.attempt + 1;
      const url = failedModuleUrl(error);
      return { attempt: next, bust: url ? retryUrl(url, next) : null };
    });
  }, []);

  // Second rung: one in-page re-import has already been spent and failed, so
  // the only thing left that can work is a new document. Never automatic —
  // the user presses it, and lands back on this same screen, still signed in.
  const spent = attempt > 0;
  const reload = useCallback(() => window.location.reload(), []);
  const describeSpent = useCallback(
    (error: unknown) =>
      isChunkLoadError(error)
        ? "This screen’s code still isn’t downloading. Reloading PLENUM will clear it — you’ll stay signed in and come back to this screen."
        : describeRenderError(error),
    [],
  );

  return (
    <ErrorBoundary
      key={attempt}
      region={name}
      onRetry={spent ? reload : retry}
      retryLabel={spent ? "Reload PLENUM" : "Try again"}
      describe={spent ? describeSpent : undefined}
    >
      <Suspense fallback={<LoadingPanel label="Loading" />}>
        <Screen />
      </Suspense>
    </ErrorBoundary>
  );
}
