# PLENUM — Handoff Log

One entry per build unit. Newest first.

---

## 2026-08-07 — CAD-01: parametric collector geometry pipeline
Branch: feat/cad-geometry-01
Added cad/ module: build123d parametric source for two collector geometries
(downflow/horizontal cartridges, crossflow/vertical cartridges) from a shared
parameter dataclass. Exports STEP (B-rep, CAD interchange) and GLB (web mesh)
for each. No app integration — artifacts committed as files only.
Rationale: geometry becomes a single dimensioned source of truth feeding both
the browser demo and desktop CAD use. Replaces hand-authored Three.js primitives
as the authority for collector dimensions.
Deferred: pleat geometry (face-count cost, no consumer yet); Three.js wiring
and chunk-splitting (unit CAD-02); airflow field (unlabeled illustrative only
until a validated solver run exists).
Merge: f5595d2 2026-08-07 20:45:07 -0400

- **Unit:** new local authoring toolchain plus committed artifacts. Tier 2
  (logic/feature). **No security surface:** this unit adds no endpoint, no
  auth path, no tenancy or RLS interaction, no user input, no external data
  fetch, no query, no secret, and no money. It touches no Rust crate and no
  file under `web/`. Stating that explicitly rather than skipping the
  section.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Branch point:** main `cfd1cd9`.

- **PYTHON-STAYS-OUT-OF-THE-SHIP-PATH LAW (constraint 10).** `Dockerfile`,
  `render.yaml` and `scripts/check.sh` are untouched — proven by an empty
  `git diff --stat main -- Dockerfile render.yaml scripts/check.sh`. The
  Render image gains no Python layer; the CI gauntlet gains no Python step.
  The CAD toolchain is a LOCAL authoring tool whose outputs are committed
  artifacts, following the precedent set by
  `web/src/map/blank-us-map-states-only.svg`. `cad/.venv/` is git-ignored;
  `cad/out/` is committed.

- **Shipped.** `cad/params.py` (frozen dataclass, every dimension + validation
  + derived geometry + `expected_bounds()`), `cad/common.py` (housing shell,
  hopper loft, vertical/horizontal tube sheets, rectangular inlet duct, round
  outlet duct, cartridge grid), `cad/downflow.py` (Geometry A),
  `cad/crossflow.py` (Geometry B), `cad/build.py` (CLI → four artifacts),
  `cad/test_geometry.py` (20 tests), `cad/README.md`, `cad/requirements.txt`,
  `.gitignore` additions, and the four artifacts in `cad/out/`.
- **PARAMETER LAW (constraint 9):** no dimension appears inside a geometry
  function. `common.py`, `downflow.py` and `crossflow.py` contain only
  indices, halves, the 90° rotations that lay a cylinder on its side, and one
  named `cut_epsilon` (itself a parameter) for boolean hygiene. Every solid
  dimension arrives on the `CollectorParams` instance.
- **The two geometries are defined by inlet elevation relative to the media,
  and that is ASSERTED, not assumed.** `test_downflow_inlet_is_above_all_media`
  measures the built inlet duct's bore against the topmost cartridge's upper
  surface; `test_crossflow_inlet_is_below_all_media` does the mirror. Both
  read bounding boxes off labelled leaves of the assembly compound rather
  than re-deriving the arithmetic under test.
- **Units and origin:** millimetres, `Unit.MM` written into both formats.
  z = 0 is the hopper discharge face, so overall height is exactly
  `housing_height + hopper_height` (3300 mm at the shipped parameters) —
  D.'s acceptance check 2, also asserted as a test.
- **Cutaway:** the `-Y` housing wall is omitted (`housing_cutaway`, a
  parameter) so the cartridge bank is visible in a mesh viewer — without it
  D.'s acceptance check 4 could not be performed. The roof and hopper rim
  still span the full depth, so the assembly's extents are unchanged.
- **Deps added (local venv only, never the repo or the image):** build123d
  0.11.1 (Apache-2.0), cadquery-ocp-novtk / cadquery-ocp-proxy 7.9.3.1.1
  (Apache-2.0 metadata; the bound OpenCASCADE 7.9.3 kernel is LGPL-2.1 with
  the Open CASCADE Exception upstream — the wheels ship no license file, so
  that is recorded from upstream, not verified from disk), pytest 9.1.1
  (MIT). Table in `cad/README.md` and in `cad/build.py`'s docstring.
- **Checks status (internal, output pasted in the session report):**
  20/20 pytest pass · `python cad/build.py` writes four non-zero files ·
  measured bbox of BOTH assemblies = X -1150..1150, Y -750..750, Z 0..3300,
  identical to `expected_bounds()` to within 0.1 mm · cartridge count 8/8 ·
  halving `cartridge_count` changes count and volume in both geometries ·
  `git diff --stat main -- Dockerfile render.yaml scripts/check.sh` EMPTY.
- **Out-of-scope observations (reported, not fixed):** README phase-state
  discrepancy (see below); the Three.js collector's dimensions do not
  correspond to `cad/params.py` and reconciling them is CAD-02.
- **Report, do not fix — README phase state.** `README.md` states "Phase
  state: P0–P4 merged; P5 (polish + Territory Map + Leakage + Data Quality +
  signal auto-expiry) built on `p5-polish-map`." Actual `git log` on
  `main` at branch point is `cfd1cd9`, with P5, the Render deploy, T1,
  D-1/D-2, D-3, D-4 and B-1 all merged — later than the README claims. The
  HANDOFF-LOG and CLAUDE.md are current; only the README lags. Not edited in
  this unit, per instruction.
- **Commits:** `23cb74b` (the module, artifacts and docs) and `f533f2d` (pin
  the STEP header timestamp so a rebuild is byte-identical — export_step
  otherwise stamps wall-clock time and every build rewrote both .step files
  with no geometric change, making a dirty cad/out/ meaningless as a staleness
  signal) on `feat/cad-geometry-01`.
- **Merge record:** `f5595d2` — feat/cad-geometry-01 merged to main (--no-ff),
  2026-08-07 20:45:07 -0400, on D.'s literal "merge" (merge = approval per
  this unit's pre-authorized PHASE 2). `origin/main` was confirmed unchanged
  at `cfd1cd9` immediately before the merge; pushed cfd1cd9..f5595d2. The
  live Render service is unaffected: `autoDeploy: false`, and the image build
  reads no file this unit touched.

## 2026-07-26 · B-1 — The collector demo, ported into PLENUM at /collector

- **Unit:** new product surface, first of two bridge units. Branch
  `collector-port` from main `a6e5aaa`. Tier 2: new client surface plus a
  dependency change (§2.9 supply chain, §2.10 client). No endpoint, scoped
  read, write, input, money, secret, external data or query — **the collector
  makes no network call at all in this unit.** B-2 (the telemetry push into
  the reorder branch) is NOT in this unit and will be Tier 3, because it
  touches an admin-gated write.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)

- **What was ported.** Ten files from `C:\AI_Projects\Claude\camfil-apc-demo`
  (left untouched — it remains D.'s standalone copy and the fallback) into
  `web/src/collector/`, flat, mirroring `map/` and `leakage/`. The demo's
  `App.tsx` became the SCREEN (`Collector.tsx`) — a tab switcher, not an app
  root; its `main.tsx`, `index.html`, `index.css`, `vite.config.ts`,
  `tsconfig.json`, `package-lock.json` and `node_modules` were not ported.
  A new `materials.ts` bridges tokens.css to three.js.
- **De-branded.** `grep -rn -i "camfil\|gold series\|interview concept"` over
  `web/src`, `web/index.html` and `web/package.json` returns **nothing**. The
  vendor name, the product line, the "interview concept" disclaimer and the
  old page title are all gone; the tab title was already `PLENUM`. Industry
  vocabulary stays on purpose — cartridge dust collector, pulse-jet, dP,
  in. w.g., plenum, hopper, rotary discharge, NFPA, GSX-class.

- **Palette: option (a), mapped onto existing tokens — and the reason is
  evidence, not preference.** The two palettes turned out to be the SAME
  design language: `graphite #0b0f14`, `panel #121a23`, `seam #23303d`,
  `air #e8edf2`, `mist #8ca0b3`, `brand #19b36b`, `amber #f5b81c`,
  `alarm #e5484d` and `flow #3e9bff` are **byte-identical** to PLENUM's
  `--color-bg / surface / seam / text / text-dim / ok / warn / alarm / data`.
  Only two things were genuinely new: `mist-2 #5f7284` (a third text step),
  added as `--color-text-faint`; and the 3D unit's material values. A second
  palette file would have duplicated nine identical hexes. Charts now pass
  `var(--color-…)` strings exactly as `Ask` and `Leakage` already do — proven
  in the rendered SVG, whose stroke attributes read
  `["var(--color-data)", "var(--color-warn)"]`.
- **The Tailwind v4 trap, found by looking at the screen rather than at the
  tests.** Every automated check passed while **the 3D unit rendered as a
  flat white blank**. Cause: Tailwind v4 emits only those `@theme` variables
  that some utility class actually references. Nothing writes
  `bg-coll-steel`, so all thirteen material tokens were tree-shaken out of
  the stylesheet; `getPropertyValue` returned `""` and every three.js
  material fell back to default white. Measured directly:

  ```
  --color-bg          in built CSS: 1   browser: "#0b0f14"
  --color-text-faint  in built CSS: 1   browser: "#5f7284"
  --color-coll-steel  in built CSS: 0   browser: ""
  --color-coll-grid   in built CSS: 0   browser: ""
  ```

  Fix: the material tokens live in a plain `:root` block in tokens.css, not
  in `@theme` — they are consumed by JS, never by a utility, so they never
  belonged in `@theme`. tokens.css remains the single source and
  `web/src/collector/` contains **zero** hex literals (grep pasted).

- **Dependencies — four, all MIT, all version-verified against the registry
  (not from memory):** `three@0.185.1`, `@react-three/fiber@9.6.1`,
  `@react-three/drei@10.7.7`, `@types/three@0.185.1`. Peer ranges checked:
  fiber wants `react >=19 <19.3` and PLENUM has 19.2.7 installed; drei wants
  `three >=0.159` and `@react-three/fiber ^9.0.0`. `recharts` and
  `lucide-react` were already present and are **reused, not duplicated** —
  all ten icons the collector imports were confirmed to exist in PLENUM's
  lucide-react 1.25.0 (the demo was on 0.525.0, a different major). Lockfile
  committed. `npm audit` reports **one high finding, `react-router`, which
  predates this unit and comes from none of the four** — see the BLAST DOOR
  block.
- **Bundle — the whole reason it is lazy:**

  | chunk | before (a6e5aaa) | after |
  |---|---|---|
  | **index (main)** | **427.81 kB** | **428.39 kB** |
  | BarChart (recharts, shared) | 352.15 kB | 354.73 kB |
  | TerritoryMap | 65.76 kB | 65.76 kB |
  | Leakage | 10.79 kB | 10.79 kB |
  | Ask | 6.56 kB | 6.56 kB |
  | DataQuality | 5.99 kB | 5.99 kB |
  | **Collector (new)** | — | **982.08 kB** (gzip 266.53) |

  The main chunk grew **0.58 kB** and stays under the 500 kB law. three.js and
  drei are entirely inside the lazy Collector chunk. recharts is SHARED with
  Leakage and Ask rather than duplicated (BarChart +2.58 kB, not +350). Vite
  does warn about the 982 kB chunk; that is the lazy route, not main, and it
  is a deliberate one-time cost on first click — recorded as an accepted
  weakness rather than silenced.

- **Evidence.** The demo's own 8 acceptance checks re-run at `/collector`
  inside PLENUM, logged in as a **rep**: canvas 1166×646 with live dP moving
  4.13″→4.22″ in 6 s on a `rgb(11,15,20)` page; exploded toggle flipping
  "Exploded view"→"Assemble unit"; 4 hotspot markers, pulse card opening and
  closing; dP climbing 4.30→4.42→4.56→4.58 then resetting to 3.90 with the
  newest Event feed line reading **"Pulse cycle fired · dP reset to floor"**
  (matching D.'s standalone 4.60→3.93); the 6.0″ service-limit label with
  "Aug 16 (21 days out)"; and **0 px horizontal overflow at all seven of the
  demo's own widths on both tabs, plus iPad in both orientations**. The orbit
  DRAG and the pulse-flash COLOUR judgement are left to D. — a synthetic drag
  is inadmissible under the T1 precedent, and a glow is a thing to be seen.
  Suite: web **21/21** (tripwire now **80/80** layout — `/collector` added
  five width assertions — plus 7 scope), API clippy clean and `cargo test`
  **65 passed / 0 failed**. `/collector` is behind the login wall and reachable
  by all four roles, both specs pasted.

- **What D. must still do.** The eight B-1 acceptance checks under his own
  hands, then trigger the Render deploy (autoDeploy is off by design).

- **Merged to main `5c3592b` at 2026-07-26T21:52:07Z** on D.'s "merge" order,
  `--no-ff` from `collector-port` (`bba80c3`). Pushed to origin. The live site
  will not carry the collector until the Render deploy is triggered. **B-2 —
  the telemetry push and the reorder-card beat — is the next unit.**

---

## 2026-07-26 · D-4 — Lazy route to lazy route showed the previous screen

- **Unit:** regression fix. Branch `fix-lazy-nav` from main `1e26947`. Tier 2:
  client rendering only — no endpoint, scope predicate, write, query, money,
  secret or identity touched. **This is a regression CC introduced in D-3
  (`5aff1a4`)**, found by D. in his own browser after the merge and after the
  live deploy. Live was serving it.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)

- **The defect, as D. measured it.** Click a nav link, wait 2.2 s, read the
  URL and the content together. `eager→eager` fine, `eager→lazy` fine,
  `lazy→eager` fine, **`lazy→lazy` shows the previous screen** — URL correct,
  no error, no boundary, no console exception, element count identical at
  every step. Four of the nine nav destinations are lazy (`/map`, `/leakage`,
  `/ask`, `/data-quality`), including two the demo path clicks straight
  through.

- **Root cause, one sentence:** D-3 replaced four distinct `lazy()` components
  with four uses of ONE component type, so React updated the existing
  `LazyRoute` in place instead of mounting a new one — and because that
  component built its `lazy()` inside a `useMemo` **during render**, and a
  suspending render never commits, every retry recomputed the memo, produced a
  brand-new `lazy()`, re-imported and suspended again, forever.

- **The architect's hypothesis was right about identity and wrong about the
  consequence, and the measurement is what separated them.** The prompt
  offered four candidates: (a) memo not recomputing, (b) recomputing but the
  child not re-rendering, (c) Suspense holding the previous child, (d) router
  element identity. Instrumenting `LazyRoute` with render / `lazy()`-creation
  / loader counters:

  | navigation | renders | `lazy()` created | loader invoked | mounted |
  |---|---|---|---|---|
  | eager → lazy (works) | 2 | 2 | 1 | `map` ✓ |
  | lazy → lazy (broken) | 8 572 | 8 572 | 4 286 | `map` ✗ |
  | same, 6 s later | 22 506 | 22 506 | 11 253 | `map` ✗ |

  So (a) is false — the memo recomputed 8 572 times; `memo` equalled `render`
  **exactly**, meaning it never once hit its cache, which is the signature of a
  render that never commits. (c) is the visible symptom but not the cause:
  React Router runs navigations in a transition, so `<Suspense>` kept the
  previous screen on screen while the child suspended — but the child suspended
  *again on every retry*, so "previous screen" became permanent. It was not a
  settled wrong state: **an unbounded loop at ~2 800 renders/sec and ~1 400
  module resolutions/sec, indefinitely** — a wrong screen *and* a hot CPU on
  the demo machine.

- **The fix, two independent changes for two independent causes:**
  - **Identity.** `LazyRoute` now keys its inner `LazyScreen` on the pathname.
    The key is applied INSIDE `LazyRoute`, not by the caller, so a new lazy
    route cannot forget it. Pathname cannot collide: React Router matches
    exactly one route element for a given pathname, so two different screens
    can never be at the same pathname at the same time. This also stops
    per-route retry state (`attempt`/`bust`) leaking onto the next screen —
    a latent bug the shared instance was hiding, now covered by its own spec.
  - **Stability.** `lazy()` is no longer built during render. `lazyFor` is a
    module-level cache returning the SAME component for the same
    (loader, bust, attempt), so a render that never commits can no longer
    spawn a new one. The loop is structurally unreachable, not merely
    unreached: even if a future change re-introduces a non-committing render,
    the second call returns the first call's component.
  - Post-fix, same probe, same click: **2 renders, 1 `lazy()`, 1 loader
    invocation** — identical to the healthy path — and flat at 6 s.

- **The coverage failure, which mattered as much as the code.** All nine D-3
  specs passed while the app was visibly broken. Every one of them loaded a
  route FRESH and asserted on the URL — and the URL was correct the entire
  time. A test that reads the router cannot see a screen that failed to swap.
  Fixed at the root: `useScreenReady(ready, screen)` now takes a **required**
  screen name and writes `body[data-screen]`, so the mounted screen reports
  itself and a new screen cannot ship without a marker. The D-4 specs navigate
  rather than load fresh, and assert on that marker, never on the URL.
- **Evidence.** Red: 12 of 12 lazy→lazy transitions showed the previous
  screen; `lazy→eager→lazy` passed; the nine-item walk failed only on
  `Ask → Data Quality`, the one adjacent lazy pair — the pattern visible in
  the output. Green: all 12 pairs, all 8 eager-in-between legs, the 18-stop
  nav walk, and the state-leak spec. Full web suite **18/18** twice in a row
  (tripwire 75 layout + 7 scope, dragproof, honest-errors 3/3, blank-screen
  13). API untouched and re-proven: clippy clean, `cargo test` 65/0. Build:
  main bundle **427.81 kB**, under the 500 kB law. **Also verified against the
  BUILT bundle served by the API's `WEB_DIST` tier** — production React does
  not double-render under StrictMode, so dev alone would not have been proof:
  12/12 pairs and the 18-stop walk all pass there too.

- **What D. must still do.** Run the six D-4 acceptance checks (1–5 in dev,
  6 live), then trigger the Render deploy — autoDeploy is off by design.
  Cowork can fire the deploy through the Render MCP.

- **Out of scope, reported not fixed:** `/quotes/:id` and `/accounts/:id`
  render one component type across different params, which is the same
  *class* of shared identity — but they are eager, so no suspension and no
  loop is possible, and neither is reachable from the nav. Not exercised by
  this unit; flagged for whoever touches those screens next.

- **Merged to main `00c74c8` at 2026-07-26T19:33:56Z** on D.'s "merge" order,
  `--no-ff` from `fix-lazy-nav` (`732a4db`). Pushed to origin. **The live site
  is still serving the D-4 defect until the Render deploy is triggered** —
  autoDeploy is off by design.

---

## 2026-07-26 · D-3 — A blank screen is now unreachable

- **Unit:** defect fix, not a phase. Branch `fix-blank-screen` from main
  `98a744f`. Tier 2: client rendering only — no endpoint, no scoped read, no
  write, no query, no money, no secret, no dependency, no identity. Nothing
  about authorization or scope is in play.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)

- **The defect, as D. found it.** Not from a test and not from a report —
  **D. found it running acceptance check 7 on the live site on 2026-07-26**.
  Wi-Fi off, click LEAKAGE on `plenum.onrender.com`, and **the entire
  document went empty**: no nav, no PLENUM header, no user badge, black
  viewport, URL still `/leakage`. **The reproduction was captured in his own
  browser**, console reading:

  ```
  [EXCEPTION] (https://plenum.onrender.com/assets/index-uPPySGuq.js:8:64916)
  TypeError: Failed to fetch dynamically imported module:
  https://plenum.onrender.com/assets/Leakage-Bnf7PrT6.js
  ```

  Recovery required a manual page reload. Screenshot confirmed by D.

- **Root cause, one sentence:** `React.lazy` rethrows a rejected dynamic
  `import()` during render and `<Suspense>` has no error path, so with no
  error boundary anywhere in `web/src` the failure reached the root and React
  unmounted the whole tree — which is a blank document, not a broken screen.

- **Reproduced before it was fixed, in dev, measured both ways.** Same page,
  before and after the click: **275 elements → 2**, `document.body.innerHTML`
  **87 bytes**, body text empty, nav gone, same console exception as D.'s live
  capture. The same collapse — 275 → 2 — happens for *any* uncaught render
  error, which is why this was never only a Leakage-offline bug. Both red
  states are the commit `D-3 RED` on this branch: the spec and the probe
  landed first, with no boundaries, and 6 of its 7 specs failed.

- **The fix, at each layer.** One reusable class component
  (`components/ErrorBoundary.tsx` — `getDerivedStateFromError` /
  `componentDidCatch` are the only render-time mechanism React offers, so this
  is the lowest available layer on the client), mounted three times:
  - **root** (`main.tsx`) — above the router and the providers; catches what
    no screen boundary can see (the Shell itself, RequireAuth, Login).
  - **screen** (`Shell.tsx`, around `<Outlet/>`) — every routed screen, the
    nine eager ones as much as the four lazy ones, so the nav survives.
    `resetKey` = the pathname, so navigating away always clears the panel.
  - **per lazy route** (`components/LazyRoute.tsx`) — owns the Suspense
    fallback, the boundary and the retry for each of the four lazy screens.
    `App.tsx` no longer holds a bare `lazy()`/`<Suspense>` anywhere.
- **Message honesty (the D-2 law, applied to a new class of failure).** A
  chunk that did not download is NOT an API failure, so it must not borrow the
  API's wording and must never claim the API was unreachable — the API may be
  perfectly healthy. `describeRenderError` reuses `describeError` from
  `lib/api.ts` verbatim for `ApiError`/`NetworkError`, and adds distinct
  branches for a chunk-load failure and a plain render error.
  `describeRootError` is separate for the same reason: at the root there is no
  screen, so saying "this screen" would be the D-2 mistake in a new place.
  Buttons stay plain — "Try again", never a stack trace. **No D-2 string
  changed**; `honest-errors.spec.ts` still passes 3/3.

- **The retry took two measurements to get right, and the first design was
  wrong.** A fresh `lazy()` per attempt is necessary but NOT sufficient:
  - `React.lazy` memoizes its promise — a rejected lazy object rethrows the
    same error forever, so a keyed re-mount alone changes nothing.
  - **The browser's module map caches the failure against the URL.** Proven
    directly, bypassing React: after an aborted load of
    `/src/leakage/Leakage.tsx`, a second `import()` of the identical specifier
    **issued no network request at all** and rejected instantly, while the
    same file with a query string appended fetched and resolved. So a keyed
    re-mount cannot work either — **the URL has to change**.

    ```
    1 plain, offline      : REJECT: Failed to fetch dynamically imported module: .../Leakage.tsx
    2 plain, back online  : REJECT: Failed to fetch dynamically imported module: .../Leakage.tsx
    3 ?d3=1, back online  : OK: Leakage
    requests the page actually made:
       ABORT http://127.0.0.1:5177/src/leakage/Leakage.tsx
       PASS  http://127.0.0.1:5177/src/leakage/Leakage.tsx?d3-retry=1
    ```
  So the retry re-imports the failed module under a one-time
  `?d3-retry=N` URL, taken from the URL the browser names in its own error and
  accepted only if it is same-origin. Vite's static, analyzable `import()`
  stays the normal path, so route chunking is untouched.
- **The limit of an in-page retry, measured against the PRODUCTION build, not
  reasoned about.** With the whole network down the screen's chunk is not the
  only casualty — its shared dependency chunk dies with it
  (`Leakage-*.js` **and** `BarChart-*.js`, recharts). Re-importing the screen
  under a busted URL re-fetches the screen, but its own static import of
  `./BarChart-*.js` resolves to the UNbusted URL, whose failure the module map
  still remembers. Nothing inside the document can clear that. **So the panel
  escalates rather than lying:** first press re-imports (recovers the common
  case, no reload); if that fails too, the button becomes an explicit
  **"Reload PLENUM"** — never automatic, always the user's press. That reload
  was verified to cost nothing: session and URL both survive it in dev and in
  the production shape (the MemoryStore is server-side — a reload restarts the
  page, not the API; only an API restart or `demo-reset.ps1` signs anyone out).
- **Test hooks, permanent and documented**, because a boundary can only be
  proven by a throw during render: `plenum-test-render-error` (screen
  boundary — panel appears inside the shell) and `plenum-test-root-error`
  (root boundary — panel replaces the page, document never empty).

- **Evidence.**
  - `blank-screen.spec.ts` — 9 specs, red then green: four lazy routes each
    losing their chunk, an uncaught render error under the shell, one above
    it, the retry, the escalation, and all four routes on a healthy network.
  - Full web suite **14/14** (tripwire 75 layout + 7 scope, dragproof,
    honest-errors 3/3, blank-screen 9). API: `cargo clippy --all-targets -D
    warnings` clean, `cargo test` **65 passed / 0 failed**.
  - Production-shape walk (built chunks served by the API's `WEB_DIST` tier,
    the Render shape): offline → panel with the shell alive (94 elements, nav
    present) → "Try again" → honest escalation → "Reload PLENUM" → Leakage
    renders (2147 elements), still signed in, still on `/leakage`.
  - Build: main bundle **427.42 kB**, under the 500 kB law with 72 kB of
    headroom; the four route chunks are unchanged (5.98 / 6.55 / 10.78 /
    65.75 kB) and no size warning is emitted.

- **What D. must still do.** Run the six D-3 acceptance checks under his own
  hands — checks 1–5 in dev on 127.0.0.1:5177 (DevTools → Network → Offline on
  LEAKAGE and on TERRITORY MAP, the retry, the three healthy routes, and the
  console one-liner), then trigger the Render deploy deliberately (autoDeploy
  is off by design) and run check 6 on the live site. **Note for check 2:** in
  dev only the screen's own module dies, so one press of "Try again" loads
  Leakage — measured, no reload, no re-login. On a real whole-network outage
  against the built bundle the first press may not be enough and the button
  becomes "Reload PLENUM"; that is the designed second rung, not a failure.

- **Out of scope, reported not fixed:** the ten `ErrorPanel` call sites that
  still pass no `error` object (known and accepted since D-2) are untouched.

- **Merged to main `b74672b` at 2026-07-26T16:30:10Z** on D.'s "merge" order,
  `--no-ff` from `fix-blank-screen` (`7c1bbc2` red state → `5aff1a4` fix).
  Pushed to origin. The live service tracks main, but **autoDeploy is off by
  design** — D. triggers the Render deploy deliberately, then runs acceptance
  check 6 on the live site.

---

## 2026-07-25 · D-1/D-2 — Items leaderboard performance + honest failure

- **Unit:** fix, not a phase. Branch `fix-items-perf` from main `81eba71`.
  Tier 2 (no auth, no tenancy rule, no money path changed) with a mandatory
  scope proof, because `/api/metrics/items` is a scoped read.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **The two defects, as D. found them by clicking around after the live
  walk:** D-1 `/api/metrics/items` took 34–39 s on the live site; D-2 the
  Items tab then showed "Couldn't reach the API." even though the API had
  answered 200.

- **EXPLAIN ANALYZE first, before any fix — and it re-aimed the
  hypothesis.** The architect's prime suspect was the attach-rate
  `LEFT JOIN LATERAL`. Profiling all four shapes on the frozen seed says:
  **partly confirmed, and for the wrong reason.**
  - The LATERAL *is* real per-row work: one `v_unit_facts` scan per
    consumable row (16 scans, 841 unit-evaluations) plus a correlated
    `EXISTS` per unit — 213 917 / 686 551 / 92 382 / 213 425 shared-buffer
    hits for the four shapes. Real, but locally only 43–222 ms of it.
  - **What actually dominated the clock was JIT.** The LATERAL pushed the
    planner's cost estimate to 1 316 336 / 9 160 850 / 12 051 377 / 689 707
    — every one past `jit_optimize_above_cost` AND `jit_inline_above_cost`
    (500 000 each) — so **every single request** LLVM-compiled 464–563
    functions with full inlining and optimization before executing a row.
    Measured JIT share: 1519/1794, 1059/1289, 1402/1448, 1065/1134 ms —
    **83–97 % of the wall clock**. Re-running the identical queries with
    `SET LOCAL jit = off` collapsed them to 76 / 222 / 43 / 65 ms, which is
    the isolation proof.
  - **The consumable contradiction is resolved:** at the same period,
    `kind=all` and `kind=consumable` do essentially identical attach work
    (213 917 vs 213 425 buffers), so attach-rate volume cannot explain an
    8–10× gap. What differs between them is plan size and therefore JIT tier
    — shape 4's estimate (689 707) sits nearest the 500 000 threshold and
    compiles the fewest functions (464 vs 551). The free instance's exact
    tier could not be confirmed (its Postgres is IP-allowlisted and the
    Render MCP path cannot reach it); the fix removes the question by
    dropping every estimate below `jit_above_cost` entirely.
- **Root cause D-1, one sentence:** the attach-rate LATERAL inflated the
  items query's cost estimate past Postgres's JIT inlining/optimization
  thresholds, so every request paid a full LLVM compilation of ~500
  functions before it did any work — while also doing genuine per-row unit
  scans.
- **Root cause D-2, one sentence:** `TabPanel` rendered `<ErrorPanel>` with
  no message, so `ErrorPanel`'s hardcoded default — "Couldn't reach the
  API." — was shown for *every* failure, including the typed 500 the API
  really returned; the error never reached the panel that had to describe
  it.
- **D-2's trigger, found in the live logs, not guessed.** A slow-but-200
  response never produces an error state — proven three times (local 45 s
  delay, live clean load 38 s, live click-around 79.5 s: pulse, then rows,
  never an alarm). Three *concurrent* items requests on the live site,
  however, all returned `500 {"error":{"code":"internal","message":"internal
  error"}}` after 50 s, and the Render app log for that instant reads
  `database error … peer closed connection without sending TLS
  close_notify` + `Connection reset by peer (os error 104)` — the Postgres
  backends were killed. The same signature appears at 2026-07-25T02:06:41Z,
  the night D. was clicking around. So the API was right to return 500; the
  client was wrong to call it a network problem.

- **Fix, at each layer:**
  - **SQL (Phase B preference 1 — make the per-row work unnecessary).** All
    four items queries rewritten from `LEFT JOIN LATERAL … EXISTS` to a
    set-based CTE chain: `page` (ORDER BY + LIMIT first), `fits`
    (`unnest(filter_fits)`), `served` (one `DISTINCT` scan of
    `v_order_facts`), `att` (one `v_unit_facts` scan, grouped). No index was
    needed and no migration was written — the additive-index option
    (preference 2) was not required once the per-row work was gone.
  - **Client (Phase C — cause, not the string).** `lib/api.ts` gains a typed
    `NetworkError` (fetch itself rejecting, or a body that cannot be read)
    so a transport failure is distinguishable from an API answer, plus
    `describeError()`. `ErrorPanel` takes the `error` and names the server's
    typed code, message and status; `message` still overrides; the
    no-argument fallback is now cause-agnostic ("Couldn't load this panel.")
    instead of asserting connectivity it cannot know about. Leaderboards
    passes `query.error` on all three tabs.
- **Before → after, wall clock, same data, same box:**

  | shape | cost before | cost after | before | after |
  |---|---|---|---|---|
  | 2026 · product · all | 1 316 336 | 20 691 | 1794.3 ms | 45.4 ms |
  | cumulative · product · all | 9 160 850 | 20 638 | 1289.3 ms | 59.1 ms |
  | 2026 · family · all | 12 051 377 | 20 707 | 1447.7 ms | 37.6 ms |
  | 2026 · product · consumable | 689 707 | 20 073 | 1133.6 ms | 38.7 ms |

  Every post-fix estimate is below `jit_above_cost` (100 000), so **no plan
  contains a JIT section at all** — the tax is structurally gone, not tuned
  away. Buffers fell 213 917→118 809, 686 551→118 486, 213 425→118 317
  (family 92 382→94 823, flat). End-to-end through the router: 1.1–1.5 s →
  14–69 ms across 30 shapes.
- **Numeric identity:** 53 response bodies — VP and rep, gross and net, 13
  period/kind/group combinations, a mid-set `offset` page, and the 422
  cases — captured against the pre-fix binary and again against the fixed
  one. `diff -r` is **empty**. Not one number, percentage, row order or
  total moved.
- **Band-Aid Test:** passed all six; the fix is at the layer where the cost
  is created (the query shape) and where the lie is told (the panel that
  renders the message), not at the symptom. Full answers in the unit report.
- **Blast door:** VERDICT PASS. §2.2 scope proof (rep sees 19 rows /
  $485,364.67 vs the VP's 29 / $3,916,141.44, byte-identical to pre-fix, a
  strict subset; unknown `app.user_id` → 0 rows and 0 denominator, fail
  closed), §2.8 availability (every changed query measured; nothing over
  1 s), §2.10 client (no authorization decision moved client-side, nothing
  cached). Identity / money / secrets / provenance: not required — no
  surface.
- **Regression anchors:** re-run after the fix, all sixteen exact — orders
  17353 · order_lines 25497 · opportunities 16 / checksum 3367519569 ·
  quotes 1 / quote_lines 3 · mv 120/195/1699/614 · audit_log 157 ·
  territories 8 · territory_states 66 · users 17 · accounts 48 · sites 60 ·
  contacts 107 · products 42 · installed_units 232. No seed change.
- **Suites:** `cargo test` 65/65 · `cargo clippy --all-targets -D warnings`
  clean · `SQLX_OFFLINE=true cargo check` clean (4 `.sqlx` entries out, 4
  in) · web build main chunk 424.46 kB (< 500 kB law) · tripwire 75/75
  layout + 7 scope + dragproof + the 3 new honest-error specs, 5 specs
  passed.
- **Not live yet:** autoDeploy is OFF by design. This fix reaches
  https://plenum.onrender.com only on D.'s explicit redeploy, and a
  redeploy signs everyone out (MemoryStore sessions).
- **Accepted + merged:** D.'s literal "merge" reply 2026-07-25 (merge =
  approval per the unit's pre-authorized PHASE 2). Merge commit `d1aad0f`
  (--no-ff) on main, 2026-07-25 20:11:53 −04:00, pushed to origin
  (`81eba71..d1aad0f`); staleness check passed — origin/main was still
  `81eba71` at merge time. Branch `fix-items-perf` (`d70b9ef`) kept. The
  eight acceptance checks stay OWED to D.'s own hands and cannot run until
  the service is redeployed.

---

## 2026-07-25 · T1 DEPLOY + LIVE ACCEPTANCE WALK (no code change)

- **Unit:** operational, not a build. T1 (main `81eba71`) taken live and
  walked under D.'s observation. No source file changed.
- **Deploy:** manual deploy `dep-d9i0nljtqb8s73ad2ikg` triggered 2026-07-25
  01:05 UTC, live 01:08 UTC — under 3 minutes. autoDeploy remains OFF by
  design (render.yaml pins it, deploy-era ruling D9: a deploy restarts the
  in-memory session store and would sign everyone out mid-demo).
- **Migration 0014 applied in prod, PROVEN:** the prod reset printed
  `audit_log 157` — the specific evidence the 0014 audit triggers are live
  on the managed database. Every other regression anchor matched exactly.
- **Live acceptance walk — 12 checks, ZERO failures**, driven in D.'s browser
  under his observation:
  1 VP login · 2 Territory Map renders (51 states + DC + Canada blocks) ·
  3 Edit-map toggle present · 4 planning sums tie to Command to the cent ·
  5 persisted write (AL → NE-1) survived a full reload ·
  6 PLANNING-VIEW LAW held live (Command byte-identical while the map
  regrouped) · 7 rep scope isolation (serena sees $485,364.67, not the VP's
  $3,916,141.44) · 8 rep forcing `?edit=1` got ZERO edit affordances ·
  9 direct write API as rep → `403 forbidden`, typed · 10 AI-off degrades
  gracefully · 11 restore to canonical exact · 12 all screens render without
  error.
- **Verdict:** the gate on sharing the live link is CLOSED.
- **Defects found immediately afterwards by D. clicking around** (two AI
  audits and the walk itself missed both): D-1 `/api/metrics/items` 34–39 s;
  D-2 the client claims "Couldn't reach the API" on a successful 200.
  Both are the subject of the next unit.

---

## 2026-07-23 · T1 — Territory Map Editing (planning view)

- **Unit:** T1 (first post-ladder unit — NOT a phase): /map gains a
  VP/admin-only Edit mode — reassign US states + DC by click-to-paint or
  drag, create/rename/recolor/delete territories, every mutation audited,
  planning-view only (official rollups and rep scope untouched by design
  and by test) — branch `t1-territory-editing` from main `5367a5f`.
  Tier 3, one-and-done. Live site NOT touched (autoDeploy off; a Render
  deploy is D.'s separate call).
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Architect rulings recorded (T1-D1…D10, dispositions inline):**
  - **T1-D1 — Migration 0014 (additive) — SHIPPED as ruled.**
    territory_states gains `id uuid NOT NULL DEFAULT gen_random_uuid()`
    + UNIQUE, solely so `audit_row_change()` (which stamps entity_id from
    NEW.id/OLD.id) can audit it — state_code stays PK and join key, the
    0013 truncate-proof rationale untouched. The 0006 trigger is attached
    to BOTH territory_states and territories. territories gains nullable
    `color_token`. GRANT INSERT/UPDATE/DELETE on both tables to
    plenum_app — the DISCLOSED posture change: geography stops being
    app-read-only; defense for these RLS-less config tables = handler
    role gate (vp|admin, the generate-signals precedent) + app-immutable
    audit + typed errors, stated in the migration comment 0013-style.
    PRE finding, disclosed: territories already carried app DML via
    0007's blanket GRANT ON ALL TABLES (it existed at 0007 time; only
    territory_states, born in 0013, was SELECT-only) — the explicit grant
    is kept so T1's intended posture is visible in one place.
    SEED CONSEQUENCE, disclosed in the migration comment: with the
    triggers attached, the seed's territory INSERTs (8) and the D6
    geography restore (66 DELETE + 66 INSERT) are audited — audit_log at
    the seed's printed summary is now 157 (was 17), deterministic, NULL
    actor on superuser dev seeds (P0 semantics). No test pinned the 17.
  - **T1-D2 — Write surface — SHIPPED as ruled** in new
    crates/api/src/routes/territories.rs: PUT /api/territory-states/:code
    (404 unknown state · 422 Canada block AND province, message says
    "lands in v2" · 422 unknown territory · only-when-changed UPDATE so a
    repaint writes zero audit noise, the P4 no-clobber discipline);
    POST /api/territories (201; 422 dup code / bad code format (2–8
    uppercase-alnum-dash) / unknown region / color_token outside the
    8-name planning palette; quota_year_cents pinned 0 — creation never
    sets quota); PATCH :code (rename/recolor only, 404 unknown, 422
    empty patch); DELETE :code (422 with a reason NAMING every failing
    emptiness check — mapped states / rep assignments / accounts /
    orders / opportunities — with live counts); GET /api/territories
    (vp|admin disclosed read; full list incl. empty territories +
    color_token, house envelope, well under the 200 law). All vp|admin →
    403, unauth → 401, both proven for rep AND manager on every endpoint.
  - **T1-D3 — Authz plumbing — SHIPPED as ruled:** SessionUser extractor
    + common::require_role (the generate-signals shape); every write runs
    inside rls_tx so the app.user_id GUC pins the audit actor and
    reassign + audit commit atomically.
  - **T1-D4 — Edit mode UI — SHIPPED as ruled:** "Edit map" toggle
    rendered only for vp/admin (useMe), edit state in the URL (?edit=1 —
    the house controls-in-URL law; a rep with ?edit=1 forced gets the
    byte-identical P5 screen). Persistent planning-view banner verbatim.
    Side panel becomes MapEditor: territory list with color chips,
    per-territory planning sum (client-side sum of the caller's state
    rows — no new endpoint), New-territory form (code/name/region
    dropdown from existing region values/palette chips), inline rename +
    recolor, delete with the server's refusal reason rendered VERBATIM.
    Click-to-paint primary; drag a state onto a territory row OR legend
    chip secondary (custom pointer-drag with ghost — both paths call the
    same PUT). Canada blocks inert in edit mode, tooltip carries
    "Canada editing lands in v2". Non-edit mode byte-identical to P5.
  - **T1-D5 — Planning palette — SHIPPED as ruled:** 8 new
    --color-terr-plan-* tokens (one notch more chromatic than the
    canonical fills — planning reads as provisional; alarm/amber/flow
    reserved as ever). ONE fill resolution (makeTerritoryFill in
    UsMap.tsx): API color_token wins → canonical P5 mapping → sorted-code
    pool index for tokenless runtime territories; map, legend, panel,
    tooltip, and editor all render through it. BUILD FINDING, disclosed:
    Tailwind v4 prunes @theme variables that never appear literally in
    scanned source, so the token→var() map is written with literal names
    (a constructed `var(--color-${token})` left every planning token
    undefined — caught in the live browser walk, fixed, re-proven).
    Disclosed payload addition (P3 precedent): the /api/metrics/states
    roster row gains `color_token` (NULL on canonical territories → rep
    rendering unchanged) so every role renders runtime territories
    without the vp-only read.
  - **T1-D6 — Seed canonical-geography restore — SHIPPED as ruled:** the
    seed's final geography step DELETEs territory_states and re-INSERTs
    the canonical 66 rows, single-sourced by extracting the INSERT from
    the 0013 migration text via include_str! (never hand-copied), with a
    hard 66-row check. NEW LAW recorded: geography survives ordinary
    use; demo reset restores the canonical map (runtime territories die
    with the territories truncate; their dangling state rows die here).
    Plain row DML as the connection's user — no superuser assumption, so
    the managed-prod TLS reset path behaves identically. Proven: edited
    world (AL→NE-1, NM→ZZ-1 runtime territory) → seed → 66 rows, exact
    Census distribution, ZZ-1 gone, zero color_tokens, anchors intact.
  - **T1-D7 — metrics/states follows config LIVE — SHIPPED, one
    disclosed query change:** the items' territory_code now joins
    territory_states on the site's state (was: the order's territory_id
    → territories.code) and total counts DISTINCT states — byte-identical
    at canon (PRE-2's 100% alignment; p5's Σstate==Σterritory and
    grouping tests still green), and a reassigned state regroups
    immediately with no server-side geography caching. Zero-state /
    zero-TM/RM territories verified (a just-created territory appears in
    the roster with empty arrays; the editor lists it from the disclosed
    read before it has states).
  - **T1-D8 — Tests + tripwire — SHIPPED:** crates/api/tests/t1_http.rs
    (3 tests): the rep+manager 403 matrix over every endpoint + 401s;
    the PUT validation matrix; audit actor = acting user, repaint = zero
    audit delta; THE PLANNING-VIEW LAW AS A TEST — Board feed
    byte-identical across a reassignment while metrics/states regroups
    with exact mirror sums (period=2023, a frozen year concurrent crm
    bookings can't touch); create/patch/delete matrix incl. the
    per-reason delete guard and the audited INSERT/UPDATE/DELETE trio.
    Cross-process discipline, disclosed: cargo runs test binaries in
    parallel, so T1's mutating tests and p5's roster-pinning states test
    serialize on one Postgres advisory xact-lock (p5 gained the 6-line
    acquisition — test maintenance, the deploy-unit precedent; app code
    untouched). Canonical-restore-after-reseed is proven in the
    verification gauntlet (a reseed inside a #[test] would nuke the
    parallel suite's world). Tripwire: layout grows to 75 (adds
    /map?edit=1 at 5 widths, VP session); scope grows to 7 (rep AND
    manager: zero edit-affordance testids even with ?edit=1 forced; VP:
    toggle/editor/banner present). 75/75 + 7/7 PASS.
  - **T1-D9 — Forward-prep doc — COMMITTED:**
    docs/territory-realignment-prep.md (commit-realignment semantics,
    sub-state splits, non-state geographies, Canada v2). Whitespace
    note: the unit prompt's doc blocks arrived with line breaks
    stripped in transport; structure restored, content verbatim.
  - **T1-D10 — Rejections honored, none re-decided:** no scenario/draft
    tables · no free hex picker (tokens.css remains the only palette
    source; the API validates against the same 8 names) · code is the
    edit key, never territory id · Canada locked · managers excluded
    (and tripwire-asserted + 403-tested).
- **UI fixes found by the in-session browser walk (disclosed):** (i) edit
  mode renders every territory's CONFIG fill regardless of the session's
  cached scope list (a just-created territory isn't in the login-time
  me.territories; money stays scope-gated); (ii) the paint handler's
  isPending guard dropped fast successive clicks — removed (the server's
  only-when-changed law makes repeats no-ops); (iii) the Tailwind v4
  literal-token finding under D5.
- **Out-of-scope observations (reported, not fixed):** (a) the local dev
  .env now carries a REAL ANTHROPIC_API_KEY (P4-era; still untracked/
  gitignored, nothing in the repo) — it was incidentally exposed into
  this session's transcript during a PRE check, so ROTATE the key at the
  console; (b) a 0-quota runtime territory renders "—" attainment via the
  existing percent(null) path — no display breakage, no guard needed;
  (c) MS has zero 2026-YTD dollars, so a GC-1 built from LA+MS shows
  LA's sum alone — correct arithmetic, worth knowing before the
  acceptance walk's check 5; (d) painting a US state INTO CE-1/CW-1 is
  not blocked (the v1 lock is on Canada STATES, not targets) — harmless
  in a planning view, flagged for the realignment unit.
- **Verification (outputs in the session report):** PRE checks (schema
  shapes, SELECT-only grant, role-gate path, territoryFill shape, no
  /api/territories collision) · seed run with restore (66 rows, audit
  157, anchors exact) · cargo sqlx prepare (committed) · check.sh ALL
  CHECKS PASSED — 63 tests (12 domain + 7 validator + 22 prior HTTP + 8
  signals + 7 ai + 4 p5 + 3 t1) · adversarial curl matrix (rep+manager
  403 ×5 each; 404/422s verbatim; audit_log UPDATE/DELETE as plenum_app
  → permission denied) · planning-law live proof (Board feed
  byte-identical across GA→MT-1; GA row regrouped with identical
  dollars; audit actor "Valerie Price | UPDATE | SE-1 -> MT-1") ·
  browser-driven walk (toggle/banner/editor, paint GA, GC-1
  create→paint LA+MS→SE-1 delete refusal verbatim→restore→GC-1 delete;
  drag AL→NE-1 row with ghost; rep + ?edit=1 = zero affordances) · seed
  restore proof · npm run build (main 423.70 kB < 500 kB) · tripwire
  75/75 layout + 7 scope PASS.
- **Anchors:** frozen set unchanged and re-proven post-0014 (orders
  17353 / checksum 11556020473 · mv 120/195/1699/614 · ledger CUM NET
  2467089087). NEW deterministic anchor: territory_states 66 at seed;
  audit_log at seed print = 157 (supersedes 17 — the D1 disclosure).
  CLOCK-DRIFTING: signals 40/12/28/168 = 248 on 2026-07-23.
- **New dependencies: NONE.** Zero crates, zero npm packages.
- **FIX (same session, pre-merge) — acceptance check 4 FAILED under D.'s
  real mouse; drag rebuilt on pointer events, trusted-event red→green
  proven.** Defect: a real mouse-down on a state shape started NATIVE
  TEXT SELECTION (no preventDefault anywhere on the gesture), and
  dragging toward the editor highlighted the panel text — the drag path's
  in-session proof had used synthetic events, which trigger no native
  default behavior, so it passed while a trusted mouse failed (that
  proof class is now recorded INADMISSIBLE for gesture defects). Fix in
  TerritoryMap/UsMap: pointerdown (was mousedown) with e.preventDefault()
  — selection never starts; `click` still fires per the Pointer Events
  spec, so click-to-paint is byte-identical — + setPointerCapture on the
  originating shape, body user-select:none while a drag is armed
  (restored on pointerup/pointercancel), touch-action:none on shapes in
  edit mode, and the drop test hit-tests document.elementFromPoint
  (with capture, event.target stays the shape). New standing spec
  web/dragproof.spec.ts drives a REAL Playwright mouse (mouse.down →
  incremental moves → mouse.up): RED against the pre-fix code
  (isCollapsed === false — text selected, D.'s exact observation),
  GREEN after (AL re-homed, ghost present mid-drag, zero selection,
  trusted click-to-paint restore). playwright.config testMatch widened
  to include it — it runs with every tripwire. Post-fix: full suite 63
  green · tripwire 75/75 + 7 scope · dragproof green · geography
  canonical (66, AL→SE-1). D.'s own re-drag re-owed before "merge".
- **Phase gate: T1 ACCEPTED** — D.'s literal "merge" reply, 2026-07-23
  (merge = approval per this unit's pre-authorized PHASE 2). Attribution
  for the record: checks 1–3, 5–7, 8a passed under D.'s own hand
  pre-merge; check 4 failed, was fixed same-session (the pointer-event
  rebuild above) and re-proven with trusted Playwright input — D.'s own
  re-drag of check 4 plus check 8's reset half stand OWED as his
  observation pass; any failure reopens the gate.
- **Merge record:** `9e2e7db` — t1-territory-editing merged to main
  (--no-ff), 2026-07-23 22:30:16 -0400, on D.'s "merge". Staleness check
  passed (local main AND origin/main both still `5367a5f`). Pushed to
  origin/main (github.com/tolani13/plenum — main is public; the live
  Render service tracks main but autoDeploy is OFF, so the live site is
  UNCHANGED until D. orders a manual deploy). Branch t1-territory-editing
  kept, per precedent.

## 2026-07-22 · Deploy: PLENUM on Render (all-Render, one origin)

- **Unit:** production deploy (one Render web service serving API + SPA,
  one managed Render Postgres, seeded deterministic world, AI OFF, RLS over
  the wire) — branch `deploy-render` from main `1a37189`. Tier 3. FIRST
  unit with a remote: origin = github.com/tolani13/plenum (private).
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Architect rulings recorded (D1–D10, dispositions inline):**
  - **D1 — Multi-stage Dockerfile** (repo root, prod-only): node:20-slim
    builds the SPA from the committed lockfile (npm ci);
    rust:1.95-bookworm (the repo's pinned toolchain) builds `api` + `seed`
    release binaries with SQLX_OFFLINE=true; debian:bookworm-slim runtime
    (+ca-certificates, non-root user `plenum`) carries the two binaries,
    migrations/ (transparency — both binaries embed them), web/dist, and
    the entrypoint. .dockerignore keeps the context lean and secret-free.
    No .env, no key, no password in any layer.
  - **D2 — Render port binding.** docker/entrypoint.sh exports
    BIND_ADDR=0.0.0.0:${PORT:-10000} and execs /app/api (PID 1). Graceful
    shutdown CONFIRMED as-is for dev (ctrl-c); honest note: the handler
    listens for SIGINT only, so Render's SIGTERM stop is abrupt —
    consequence-free here (sessions are in-memory and lost on restart by
    design; requests are short) and adding a SIGTERM arm was outside the
    three sanctioned code touches. Recorded, not built.
  - **D3 — SPA from axum** (sanctioned touch a). tower-http ServeDir
    mounted as the router's fallback_service, gated on
    WEB_DIST/index.html existing (default web/dist: present in the
    container, absent from dev/test working directories — dev and the
    test-suite JSON-404 behavior stay byte-identical). Unknown /api/*
    paths keep the typed JSON 404 contract via an /api catch-all that
    outranks the static tier. Deep links serve index.html WITH 200
    (ServeDir::fallback — its not_found_service variant stamps 404; found
    by the container smoke test and fixed). Known blanket-SPA tradeoff,
    recorded: a missing /assets/* path also serves index with 200 (stale
    post-redeploy chunks recover on reload).
  - **D4 — Migrations on boot** (sanctioned touch b), env-gated
    MIGRATE_ON_BOOT=true (constraint 4's env-gate law; dev default false =
    byte-identical). Two prod realities folded into the same touch,
    disclosed: (i) migrations 0007+ GRANT to `plenum_app`, which managed
    Postgres doesn't have — the gated path ensures the role exists first
    (CREATE ROLE plenum_app NOLOGIN if absent; nothing connects as it in
    prod, it only has to be grantable); (ii) the P0 fail-fast exits on an
    empty users table, which would crash-loop the service between first
    deploy and the seed job — under the gate it WARNS and serves instead
    (health checks + login screen live; data arrives with the job).
    Rehearsed against a scratch DB from the image: migrate → EMPTY warn →
    health 200 → seed job → ledger anchor exact.
  - **D5 — Seed as an explicit one-off, never in the deploy lifecycle.**
    The image ships /app/seed. The ruling's first-preference form —
    `render jobs create <SERVICE_ID> --start-command "/app/seed"` — was
    attempted and REFUSED by the platform on the free plan (400: "free
    tier plans are not supported for jobs"); it remains the documented
    path if the service ever moves to a paid instance. The executed
    fallback (anticipated in the ruling's "whichever is available"): run
    the seed binary locally against the database's EXTERNAL connection
    string over TLS (?sslmode=require) — which surfaced and settled two
    managed-database realities, both disclosed:
      (i) external access needs an Access Control entry — added
          74.124.184.78/32 ("D. dev machine (seed/reset) 2026-07-23"),
          the tightest useful rule; resets from another network mean
          adding that network's IP in the dashboard first;
      (ii) the managed user is the table OWNER but NOT superuser, and
          0005 FORCEs RLS onto owners — the first remote seed died on
          accounts' WITH CHECK. Fix in the seed binary (disclosed,
          beyond the three API touches because D5's prod seed cannot
          function without it): single-connection pool + on NON-superuser
          connections only, pin the seeded ADMIN's identity
          (set_config session GUCs) before the load — admin scope is
          every territory, so every WITH CHECK passes exactly as RLS
          intends. Superuser dev runs skip the branch: proven
          byte-identical afterward (anchors exact, all 265 audit rows
          still NULL-actor). Managed seeds record the seeded admin as
          the audit actor — truthful and disclosed.
    Redeploys never touch data (proven: the reset is a separate command).
  - **D6 — /api/health** (sanctioned touch c): unauthenticated 200 "ok",
    no DB touch; the Render service health check points at it.
  - **D7 — render.yaml blueprint** at the repo root describes the whole
    topology (free Postgres 16 `plenum-db` + free Docker web service
    `plenum`, oregon, healthCheckPath, autoDeploy off, env wiring).
    The api reads APP_DATABASE_URL and the seed reads DATABASE_URL — both
    mapped from the same database in the blueprint, no code change.
    `render blueprints validate` passes modulo the repo-visibility item
    below.
  - **D8 — AI OFF in prod, provably.** No ANTHROPIC_API_KEY exists on the
    service; AI_ASK_ENABLED=false and AI_DISCOUNT_ENABLED=false pinned.
    Zero vendor spend possible from the live site.
  - **D9 — MemoryStore stays** (accepted): redeploy/idle spin-down signs
    everyone out; free-plan cold starts are tens of seconds. Documented in
    README "Deploy" known behaviors, plus the free-Postgres 30-day expiry.
  - **D10 — Privacy posture stated, not solved:** unguessable onrender.com
    URL + demo login + 100% synthetic data; hard gate is a later add.
    Documented in README verbatim ("who can see this").
  - **Test maintenance, disclosed:** the P5 policy-parity test assumed the
    signals census was generated the same UTC day as the request; the
    window slid at midnight mid-unit and it tripped. Fixed clock-honest
    (feed == in-window census; leftovers exactly the out-of-window rows
    pending expiry). App behavior untouched.
- **New dependency (the ONE permitted):** tower-http 0.6 (MIT),
  `fs` feature only — ServeDir/ServeFile for the static tier. Nothing else
  added (npm zero, crates zero beyond it).
- **Provisioning record (free plans only — nothing that bills):**
  - Auth path: the Render MCP token on this machine was unauthorized; the
    authenticated surface is the Render CLI v2.15.1 (workspace "Dilip's
    workspace", tea-d5ufur7fte5s73eaj0e0) and its stored API key against
    the public REST API (used for creation calls the CLI lacks; the key
    never left the machine, never printed, never committed).
  - CREATED: Postgres `plenum-db` — dpg-d9go6b3bc2fs738vcm00-a, plan
    free, region oregon, version 16, status available.
  - Web service `plenum`: creation was BLOCKED at first attempt — the repo
    was private and the Render workspace had no GitHub App grant for
    tolani13/plenum ("repository … invalid or unfetchable"), which no API
    can create. UNBLOCKED by D.'s call 2026-07-23: the repo was flipped
    PUBLIC (recorded as a deliberate posture change — the code was already
    audition material; secrets were never in it). The branch push needed
    for the build (deploy-render → origin, pre-merge) was done and
    disclosed — main untouched until PHASE 2.
  - CREATED: web service `plenum` — srv-d9goii4vikkc739qverg, plan free,
    region oregon, runtime docker (./Dockerfile), branch deploy-render,
    autoDeploy no, healthCheckPath /api/health, env keys wired:
    APP_DATABASE_URL + DATABASE_URL (internal connection string),
    COOKIE_SECURE=true, MIGRATE_ON_BOOT=true, AI_ASK_ENABLED=false,
    AI_DISCOUNT_ENABLED=false, RUST_LOG=info. LIVE URL:
    https://plenum.onrender.com
  - First deploy dep-d9goiicvikkc739qvflg (commit 6e8a6c3): LIVE in ~7
    minutes; Render's health check on /api/health passed (that is what
    "live" gates on). Pre-seed posture verified over HTTPS: health 200,
    SPA served, login on the empty world = clean 401, no crash-loop.
  - Access Control: ipAllowList entry 74.124.184.78/32 added (the
    seed/reset source machine); external connections are otherwise
    dropped at the proxy (the "0 bytes at EOF" signature).
  - Seed executed against the live database (twice: initial load + the
    reset rehearsal), redacted transcript in the session report:
    migrations no-op-verified, "non-superuser connection — RLS identity
    pinned to the seeded admin", ORDERS TOTAL 17353, opp checksum
    3367519569, mv 120/195/1699/614, signals 40/12/28/168 = 248
    (2026-07-23 clock).
  - LIVE-URL proofs (https://plenum.onrender.com): VP customers
    cumulative net 2467089087 cents ($24,670,890.87) · approvals inbox
    exactly 1 pending (13158000 / 9698040 @ 28%) · /api/ai/status
    {ask:false, discount:false} · serena territories = SE-1 only and her
    state rows = 4 SE-1 states summing to 278301715 (her frozen anchor) ·
    a VP-written activity persisted across a fresh session (Render
    Postgres, not memory) · post-reset re-proof: net 2467089087,
    approvals 1.
- **Verification (local, all output in the session report):** check.sh ALL
  CHECKS PASSED (60 tests incl. the clock-honest fix); docker build clean;
  container smoke — health 200, SPA 200 on /, /command, /map, typed JSON
  404 on unknown /api/*, VP login + 48 accounts through the container;
  first-boot rehearsal on a scratch DB (created + dropped by the
  rehearsal): migrations, EMPTY-world warn, health 200, then the seed FROM
  THE IMAGE loading the world to the exact ledger anchor (2467089087
  cents); dev parity — run-all.ps1 unchanged, both dev ports up, serena
  RLS spot-check SE-1-only; secret grep: sk-ant absent, only empty
  ANTHROPIC_API_KEY= placeholders, .env untracked.
- **README de-branding (D.'s order, pre-PHASE-2, 2026-07-23):** with the
  repo now public, README.md drops the company naming — the audition frame
  reads "an AI Sales & Solutions Architect role in the industrial
  dust-collection and air-filtration space", and every machine-local
  working-directory path became repo-relative (`plenum` / `plenum\web`).
  README only; local working docs unchanged. Committed on deploy-render.
- **Phase gate: DEPLOY ACCEPTED** — D.'s literal "merge" reply, 2026-07-23
  (merge = approval per this unit's pre-authorized PHASE 2 protocol).
  Attribution for the record: the live-URL checks 1–7 were proven
  programmatically in-session (anchors, RLS scope, AI-off, persistence,
  reset — outputs in the session report); D.'s own eyes on the live URL
  remain OWED, and the P5 14-check walk still stands owed and still gates
  sending the link onward.
- **Merge record:** `2e67b23` — deploy-render merged to main (--no-ff),
  2026-07-22 23:45:32 -0400 (machine clock; D.'s order dated 2026-07-23),
  on D.'s "merge". Staleness check passed (local main AND origin/main both
  still `1a37189`). Pushed: origin/main + origin/deploy-render (the
  sanctioned exception — D. wired the remote for deployment). Live service
  flipped from branch deploy-render to MAIN per the blueprint and
  redeployed. Branch deploy-render kept, per precedent.

## 2026-07-22 · P5 Polish + demo hardening + Territory Map (FINAL phase)

- **Unit:** P5 (Territory Map screen over a committed public-domain SVG,
  Leakage screen, Data Quality panel, signal auto-expiry, perf indexes +
  lane pagination + bundle split + states pass, + New account, param/LIMIT
  dedups, run-all/demo-reset scripts, PRODUCTION.md, README rewrite,
  tripwire 70+5) — branch `p5-polish-map` from main `8bfe7c7`. Tier 3,
  one-and-done. Repo LOCAL-ONLY.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Spec amendment recorded (D.'s order, 2026-07-22):** §12's "geographic
  map data" anti-goal is AMENDED — a static, public-domain,
  committed-to-repo map graphic is IN scope. Still banned: map tile
  services, geocoding APIs, runtime map fetches, geo libraries, any network
  dependency for the map. Honored: the asset is committed CC0 SVG, rendered
  as typed inline React paths, zero libraries, zero fetches.
- **Architect rulings recorded (R1–R13, dispositions inline):**
  - **R1 — Leakage screen (/leakage, rail between Leaderboards and
    Pipeline).** Shipped: distribution bar (recharts, tokens-only), outlier
    feed, rep × family heat table (CSS grid, five NEW heat tokens extending
    the P2 LED-band logic, alarm hue reserved for the worst band, row order
    leakage% DESC so Wes Turner reads worst at VP — test-pinned
    server-side). σ ALIGNMENT: /metrics/leakage's hardcoded `2` now reads
    signal_policy.discount_sigma — byte-identical at the 2.00 default,
    PROVEN by SHA-256 equality on three fixed requests projected onto the
    original 13-field contract (before == after, pasted in the report).
    WINDOW ALIGNMENT resolved via a DISCLOSED mode param (`outliers=policy`,
    default `period` = P1 behavior verbatim): the ruling's byte-identical
    requirement and its window-alignment clause could not BOTH hold on one
    path (the P1 feed is period-sliced stddev_samp; the generator is
    all-history stddev_pop over the trailing window), so the legacy path
    stays byte-identical and the SCREEN's outlier zone requests the policy
    path — the generator's math verbatim, rows matching the
    discount_anomaly signals 1:1 (172/172 on build day, chip join by
    order_line under the caller's RLS). Disclosed payload additions (P3
    precedent): outlier rows gain order_line_id / account_id /
    territory_code / signal_id / signal_status; LeakagePage gains `heat`
    cells (rep × family gross/net).
  - **R2 — Data Quality (/data-quality, last rail entry).** Shipped
    read-only: (a) duplicate-ish names via pure-SQL normalization (lower →
    strip punctuation → drop legal-suffix words → strip one trailing plural
    's' per word → join; no pg_trgm, no extension) — catches BOTH planted
    pairs incl. the plural "Keystone Coatings" / "Keystone Coating Co."
    case, never the legitimate parent/child; (b) cartridge-bearing units
    with NULL expected_changeout_months (the 360's CADENCE UNKNOWN chip
    predicate); (c) 100%-discount lines; (d) zero-site accounts (disclosed
    cheap addition, designed-empty on the seed). VP census exact
    (2 pairs / 2 units / 1 line / 0), serena's scope = the designed "clean
    book" empty state — both test-pinned. Scope chip states the
    only-complete-at-VP truth. New disclosed endpoint GET /api/data-quality
    (RLS-scoped naturally).
  - **R3 — Territory Map (/map, rail directly after Command).** Shipped.
    Asset: "Blank US Map (states only).svg" by Heitordp, Wikimedia Commons,
    **CC0 1.0 public domain** — committed byte-verbatim plus a provenance
    comment header at web/src/map/blank-us-map-states-only.svg; rendered
    via the derived typed module web/src/map/usStates.ts (51 paths with
    USPS codes + names, AK/HI inset separators, the DC callout circle).
    Migration 0013 seeds `territory_states` along US Census division lines
    (50 states + DC across the six US territories; CA-E/CA-W block codes +
    all provinces/territories for site-state dollar attribution), SELECT
    grant, NOT in the seed truncate list. DELIBERATE DEVIATION, disclosed:
    keyed by territory CODE with no FK — the seed wipe is `TRUNCATE
    territories … CASCADE`, and an FK would cascade-wipe this config on
    every reseed, defeating the same ruling's survives-reseed requirement;
    codes are unique and stable (the P4 tile-match precedent), so the
    code join is equivalent. PRE-2 VERDICT: **100.00% alignment in all 8
    territories** (the seed's city pools were territory-regional all
    along) → per-state dollars SHIPPED on hover, seed UNTOUCHED — the R3
    first-preference path never needed. Disclosed endpoint GET
    /api/metrics/states: per-state gross/net/leakage/order_count (metrics
    grammar, dual-basis, RLS-scoped — rep rows = own-territory states only,
    summing exactly to her territory total; adversarial-tested) + the
    config-level `territories` roster (TM = assigned reps, RM = their
    managers, mapped state codes — org chart + geography are config, the
    P4 assignees precedent; money is what scope guards). UI: ocean +
    token-colored graticule + eight NEW desaturated territory-fill tokens
    (alarm/amber never fills; flow blue reserved for selection glow),
    seam strokes, abbr labels (bbox-measured; tiny Atlantic states get an
    east leader-line column that drops below 768 — tooltip carries them),
    Canada as two chamfered schematic blocks above the continent (negative
    viewBox band, in-scope money line on the block), click → territory
    panel (nameplate, TERRITORY MANAGER, REGIONAL MANAGER via the manager
    chain, gross/net/leakage/leakage%/attainment/orders from the EXISTING
    territories metric — no new rollups), global basis toggle flips every
    dollar at once, rep view = own territory lit / foreign territories
    dimmed silhouettes with NO dollar values anywhere in the DOM (hover
    included; tripwire-asserted). Period fixed to Command's YTD so panel
    totals match Command tiles by construction (and Leaderboards at 2026).
    The drill-drawer jump link was OMITTED per the ruling's "if cheap"
    clause — the drawer is Command-internal state, not URL-addressable.
  - **R4 — Signal auto-expiry.** 0013: `signal_status` gains 'expired'
    (additive ALTER TYPE), signals gains nullable expired_at;
    generate_signals() re-created (DROP + CREATE — the return table gains
    the per-type `expired` count, and a return-shape change cannot ride
    CREATE OR REPLACE; grants re-issued) with a per-generator expiry step:
    open + machine-keyed + this type + dedupe_key NOT in the run's emitted
    key set → status='expired', expired_at=now(). assigned/actioned/
    dismissed NEVER touched. DISCLOSED completion: reopen-on-return — the
    upsert's conflict arm now also revives an 'expired' row whose key is
    re-emitted (status back to open, expired_at cleared, fresh
    score/reasons, counted in `updated`): expired is machine state, and
    without reopen a recovered-then-degraded unit could never alarm again
    (acceptance check 8 would be unrehearsable). Dismissed/actioned still
    never resurrect (P4 test untouched, green). Idempotency held: same-day
    double run = 0/0/0 per type with ZERO audit delta (proven twice —
    psql and the HTTP matrix). Expiries/reopens are real state changes:
    the 0006 trigger audits each (counts in the report). Write-backs on an
    expired card = typed 422 ("the generator reopens it if its predicate
    returns"). UI: Active definition unchanged (open ∪ assigned); the
    queue filter gains Expired; summary/tiles exclude expired by
    construction. Full adversarial matrix in tests/p5_http.rs.
  - **R5 — Perf indexes (follow the plans, not guesses).** PRE-4 EXPLAIN
    ANALYZE found the real culprit: v_unit_facts' last_paid lateral was
    seq-scanning `orders` per unit (no index leads with site_id), inflating
    the enriched-list plan cost to ~517k — past the JIT thresholds, so
    1,021 ms of the 1,187 ms was JIT compilation. ONE measured index —
    `idx_orders_site_ordered ON orders (site_id, ordered_on DESC, id
    DESC)` — plus a seed post-load ANALYZE (planner stats are stale after
    truncate-reload until autovacuum; disclosed load-path addition, touches
    no data, moves no anchor). AFTER: enriched active list 1,187.3 →
    **25.4 ms** (plan cost 18,985, JIT gone); generate_signals() 2,876 /
    2,649 → **72 ms**; v_unit_facts census 219 → 19 ms. The unit prompt's
    candidates were measured moot: signals(status,type) has existed since
    0003, and the signals table is ~250 rows.
  - **R6 — Lane pagination.** Each queue lane renders 25 cards + "Show 25
    more · N below" (client-side slice, no library; testids preserved;
    filter change resets). The tripwire expands every lane before its
    signals-scope count — exercising the control on every run.
  - **R7 — Bundle split.** React.lazy + Suspense (house LoadingPanel) for
    Ask + Territory Map + Leakage + Data Quality; ASK_FOCUS_EVENT moved to
    lib/events.ts so the Shell stops importing from Ask's chunk. Main
    chunk **773.63 kB → 423.29 kB** (no Vite warning); recharts isolated
    in a shared on-demand chunk (352.15 kB) reached only from Ask/Leakage;
    the map geometry rides its own 55 kB chunk.
  - **R8 — States pass.** Inventory table in the session report (14
    screens × loading/empty/error). The three new screens ship designed
    loading (house pulse), designed empty ("Click a state…", "clean book",
    "No lines beat the policy threshold…"), and typed-error ErrorPanel +
    Retry. Command vertical rhythm: height-gated flex distribution
    (≥900px viewports only; min-height via dvh calc + flex-1 + auto-rows-fr
    + an in-tile spacer — zero fixed pixel heights), untouched below.
  - **R9 — Small owed.** (a) "+ New account" in the shell for ALL roles →
    house-pattern modal; territory options = the config roster filtered to
    the CALLER'S scope; the server stays the validator (blank name →
    typed 422 "name is required" rendered inline — proven live); success
    navigates to the new /accounts/:id. (b) accounts.rs + metrics.rs
    dropped their P0/P1-era local parse_int/parse_page copies for
    routes/common.rs — identical grammar and 422 messages, full suite
    green. (c) web LIMIT/encode idiom unified in lib/fetchAll.ts
    (FETCH_LIMIT + q), consumed by queries/signals/crm hooks — tsc strict
    + tripwire green.
  - **R10 — PRODUCTION.md** committed verbatim at the repo root;
    placeholders filled: crates/seed/src/main.rs (importer seam),
    crates/api/src/routes/telemetry.rs (telemetry template).
  - **R11 — README rewrite** (audition frame · 3-command quickstart ·
    login table · one-line demo reset · §13 script verbatim with beat 2's
    customers-tab line and beat 6 in BOTH live and flags-off forms ·
    map-asset license note · known-behaviors appendix · troubleshooting).
    scripts/run-all.ps1 starts API + web in their own windows and — fresh
    clone only — materializes the dev .env first (values already committed
    in docker-compose.yml/initdb; COOKIE_SECURE=false so plain-HTTP
    localhost logins work; AI key left empty): without this the cookie
    posture is a hidden fourth command and gate P5-1 fails.
    scripts/demo-reset.ps1 = the reseed one-liner (re-login note printed).
  - **R12 — Tripwire 70 + 5.** 14 screens (adds /map, /leakage,
    /data-quality) × 5 widths, plus leakage-scope (rep heat table = her
    rows only, DOM set == API set) and map-scope (no '$' inside any
    foreign-territory DOM element; ≥53 shapes drawn) alongside the P2–P4
    command/pipeline/signals assertions.
  - **R13 — master-plan repairs** applied: both stale owed-walk passages
    replaced with the 8bfe7c7 acceptance-record wording (current bytes
    verified against git show 8bfe7c7 before editing — they matched the
    unit's quotes).
- **Shipped:** migrations/0013_territory_map_expiry.sql (territory_states
  seeded config · 'expired' + expired_at · idx_orders_site_ordered ·
  generate_signals v2 with per-type expired count + reopen);
  crates/api/src/routes/{states,data_quality}.rs (new) + metrics.rs (R1) +
  signals.rs (expired surface) + accounts.rs/common.rs (R9b) + mod.rs;
  crates/domain enums (Expired); crates/seed/src/main.rs (expired column +
  post-load ANALYZE); crates/api/tests/p5_http.rs (4 adversarial tests) +
  signals_http.rs extended to expired; .sqlx regenerated; web:
  map/{blank-us-map-states-only.svg,usStates.ts,UsMap.tsx,TerritoryMap.tsx},
  leakage/Leakage.tsx, dq/DataQuality.tsx, shell/NewAccountDialog.tsx,
  lib/{fetchAll,events}.ts + types/queries/signals/crm updates, Signals
  lanes + Expired filter, Shell nav + New account, App lazy routes,
  tokens.css (8 territory fills · ocean/graticule/land-dim · 5 heat bands),
  Command/TerritoryBoard/Tile rhythm, tripwire.spec.ts 70+5;
  scripts/{run-all,demo-reset}.ps1; PRODUCTION.md; README rewrite; this
  log; master-plan; CLAUDE.md.
- **Checks status (outputs in the session report):** PRE-1…PRE-6 PASS
  (two seed runs byte-identical on every frozen anchor; PRE-2 = 100.00%
  × 8; PRE-3 captures + PRE-4 baselines + PRE-5 chunk table + PRE-6
  env/no-key-material/login-table). scripts/check.sh ALL CHECKS PASSED —
  60 tests (12 domain + 7 validator + 22 prior HTTP + 8 signals_http +
  7 ai_http + 4 p5_http), fmt/clippy -D warnings/sqlx prepare --check
  clean. R1 equivalence: SHA-256(before) == SHA-256(after) on all three
  fixed requests. Tripwire **70/70 layout + 5 scope PASS**. npm run build
  clean, main 423.29 kB. Perf before/after pasted. Browser-driven internal
  walk: map GA-click → SE-1 panel (Serena Estes / Rachel Moore / YTD
  money), leakage 172 outliers with 172 chips + Wes-first heat, DQ trio
  exact, + New account blank-name 422 inline.
- **Anchors:** frozen set unchanged and re-proven twice post-0013 (orders
  17353/11556020473 · order_lines 25497/−166812187229 · opportunities
  16/3367519569 · quotes 1/lines 3 · mv 120/195/1699/614 · audit_log 17 at
  seed print · ledger/customers CUM NET 2467089087 = $24,670,890.87 ·
  serena 293778300/278301715 · Wes quote pending 13158000/9698040 @28%).
  PRE-2 verdict 100.00% → the state-dollar path shipped with ZERO seed
  change. CLOCK-DRIFTING (recompute, never pin): signal counts — build-day
  (2026-07-22) 39/12/28/172 = 251.
- **New dependencies: NONE.** Zero npm packages, zero crates. The one new
  asset: "Blank US Map (states only).svg" (Heitordp, Wikimedia Commons,
  CC0 1.0 Universal Public Domain Dedication) committed with a provenance
  header + license note in README; derived usStates.ts carries the same
  attribution.
- **Phase gate: P5 ACCEPTED** — D.'s literal "merge" reply, 2026-07-22
  (merge = approval per this unit's pre-authorized PHASE 2 protocol, the
  P1/P4 precedent). Attribution for the record: no per-check acceptance
  walk was run in-session before the order — the gate rests on the merge
  order plus the build evidence in this entry (check.sh 60/60, two
  byte-identical seed runs post-0013, the SHA-proven R1 equivalence,
  tripwire 70/70 + 5 scope, the fresh-clone 3-command login proof, and the
  browser-driven internal walk of the map panel / leakage chips / DQ trio /
  new-account 422). The 14-check walk in the session report remains OWED
  as D.'s own observation pass (the checks that write data plus the
  terminal checks: auto-expiry serials, tripwire, fresh clone, reseed, the
  real tablet) — run it before the demo rehearsal; any failure reopens the
  gate.
- **Commit:** built across `p5-polish-map` (`e8e88aa` schema+seed →
  `3766d7d` API → `5b64c02` tests → `164c6fe` web+tripwire → `ebdd622`
  scripts+docs; this acceptance/merge record added in the closeout commit
  on main).
- **Merge record:** `924da62` — p5-polish-map merged to main (--no-ff),
  2026-07-22 20:22:33 -0400, on D.'s "merge". Staleness check passed (main
  still `8bfe7c7` at merge time). Repo remains local-only; branch
  p5-polish-map kept, per precedent. FINAL phase: the P0→P5 ladder is
  complete.

## 2026-07-20 · P4 Signals + AI

- **Unit:** P4 (four deterministic signal generators + queue with write-backs,
  Command signal rewire, Ask PLENUM + discount recommender behind flags,
  telemetry ingest stub) — branch `p4-signals-ai` from main `964749f`. Tier 3,
  one-and-done. Repo LOCAL-ONLY.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Architect rulings recorded (R1–R14, verbatim intent; PR1–PR8 provenance:
  production-conversion seeds, D.'s directive 2026-07-20 — flip readiness
  without audition risk):**
  - **R1 (PR1) — Generators read the world, not the script.** All four derive
    ONLY from table data (cadence math over v_unit_facts, v_defection_risk
    verbatim, filter_fits cross-reference, order-line statistics). No seed
    constant or account-name special case anywhere in generator SQL/Rust — the
    Ridgeline card EMERGES. Proven by the fixture test: invented accounts/
    units/orders inside a rolled-back transaction produce all five expected
    cards (incl. the conquest ecm-fallback and telemetry branches) with the
    expected dedupe keys, reasons, and exact scores.
  - **R2 (PR2) — Generation is an idempotent, re-runnable job.**
    `generate_signals()` — plain invoker-rights plpgsql in 0012 (the
    refresh_rollups() shape MINUS SECURITY DEFINER; nothing needs definer
    rights), EXECUTE revoked from PUBLIC, granted to plenum_app, returning
    per-type (inserted, updated). Deterministic identity via the new
    `dedupe_key` + UNIQUE index: `reorder_due:<unit>:<due_date>` ·
    `reorder_due:<unit>:telemetry` · `defection_risk:<unit>:<due_date>` ·
    `conquest:<unit>` · `discount_anomaly:<order_line>`. Upsert = INSERT … ON
    CONFLICT DO UPDATE (score/reasons) WHERE status='open' AND something IS
    DISTINCT — reruns never duplicate, never touch assigned/actioned/
    dismissed, and a no-change rerun writes ZERO update rows (zero audit
    noise; proven: second same-day run all-zero, audit delta 0). Trigger:
    POST /api/admin/generate-signals (role=admin, the refresh pattern); the
    seed runs the same function post-refresh. Stale-predicate auto-expiry
    parked to P5 by ruling.
  - **R3 (PR3) — Thresholds are config rows.** `signal_policy` singleton in
    0012 (discount_policy pattern: boolean PK, CHECKs, seeded in-migration,
    NOT in the seed truncate list, SELECT grant): defection_multiplier 1.50 ·
    discount_sigma 2.00 · reorder_lookahead_days 30 · discount_window_days 90
    (PRE-5's first non-zero rung — 176 candidates at 90d, no laddering
    needed) · conquest_default_changeout_months 12 · telemetry_low_pct 20.00.
    0012 also CREATE OR REPLACEs v_defection_risk with the IDENTICAL column
    list, the literal 1.5 becoming the config multiplier — byte-identical
    output at the default (P1 metrics tests untouched and green). Aligning
    /metrics/leakage's 2σ feed to signal_policy stays PARKED to P5;
    metrics.rs is byte-identical this unit.
  - **R4 — The four generators (exact math, 30.44 days/month everywhere).**
    reorder_due: cadence window (due within lookahead AND under the
    defection boundary — the lanes partition cleanly) scored value-ranked ×
    overdue-boosted, PLUS the telemetry branch (filter_life_pct ≤ threshold,
    one live card per unit, ecm falling back to the config default);
    defection_risk: SELECT FROM v_defection_risk verbatim, the view's score;
    conquest: competitor units with no order history × best fitting
    consumable (highest list, tie-break sku ASC), fallback cadence marked in
    the receipts when it fired; discount_anomaly: per-family
    percentile_cont(0.5)/stddev_pop over ALL history, candidates in the
    trailing window above median + σ×spread, score = excess-leakage dollars
    on the line. reasons[] weights = the raw numeric term per label
    (days/cycles/months/dollars/pct — documented in the 0012 comment).
  - **R5 — Signal write surface.** GET /api/signals (status
    open|assigned|actioned|dismissed|active, active = open ∪ assigned
    default; type filter; envelope, limit max 200; score DESC id ASC;
    enriched via RLS-scoped joins — account/territory/site/serial/cartridge
    (conquest rows re-derive the SAME deterministic best-fit lateral the
    generator uses) + assignee + lifecycle timestamps + annual_value_cents
    for the R6 composition). POST :id/assign (assignee must carry the
    signal's territory in v_user_scope — 422; re-assign allowed; assigned_at
    = first assignment) · :id/action (outcome required) · :id/dismiss
    (reason required); actioned/dismissed TERMINAL (422 out); out-of-scope =
    404 via RLS; audit rides the 0006 trigger untouched. Disclosed
    beyond-the-list additions (P3 pattern): GET /api/signals/summary
    ({total, by_type, territories[]} over open ∪ assigned — Command's feed)
    and GET /api/signals/assignees?account_id= (the R6 picker's roster:
    users whose v_user_scope holds the account's territory; account
    RLS-gated 404 — no probing foreign teams; no other user directory
    exists).
  - **R6 — Signals queue UI** (screen 4): /signals + rail entry; four lanes
    in type order collapsing 4→2→1; Active|Actioned|Dismissed filter; cards
    carry account link, territory chip, site/serial, score, the reasons ON
    the card, status/assignee chip; inline Assign (self for reps, lazy
    scope-valid picker for RM/VP/admin), Draft Quote (not on anomaly), Log
    Call (POST /api/activities kind=call + action outcome call_logged),
    Dismiss (reason dialog, refuses empty). Draft-quote-from-signal is
    CLIENT-SIDE COMPOSITION of P3 machinery: pick the account's open opp
    (highest amount, then lowest id; Ridgeline has exactly one — the
    win-back), else POST /api/opportunities (filter-program, amount = the
    signal's annual value) + PATCH stage→qualified (the create endpoint
    seeds stage=lead and is prior-phase-frozen — the two-call composition
    honors the ruling without a backend change); builder opens pre-filled
    (cartridge/best-fit product, qty = cartridge_count); on creation the
    signal is actioned `quote_drafted:<quote_id>`; drafting still flips the
    opp to quoted via the P3 rule. Lane/card/kpi testids shipped.
  - **R7 — Command rewire.** KPI 4 = OPEN SIGNALS (summary.total; sub-line
    by-type digest; testid kpi-signals replaces kpi-defection); Territory
    Board tiles gain the open-signal count (matched by territory CODE — the
    P2 metrics payload exposes code, not id; the summary carries both;
    codes are unique so the match is equivalent). Command stops calling
    useDefection — the drawer now owns that fetch lazily (drill-only);
    /metrics/defection itself untouched. Gross/net flip unchanged; signal
    counts basis-invariant.
  - **R8 (PR4) — AI behind ONE seam.** crates/api/src/ai/: client.rs owns
    the ONLY Anthropic call (reqwest, api.anthropic.com/v1/messages,
    anthropic-version 2023-06-01, 15s connect+request); env at startup into
    AppState: ANTHROPIC_API_KEY (env-only secret; only its PRESENCE is
    logged), ANTHROPIC_MODEL default claude-sonnet-5, AI_ASK_ENABLED /
    AI_DISCOUNT_ENABLED default true. Effective ask = flag AND key; the
    discount flag alone gates its endpoint, the key only its narrative.
    error.rs gains AiUnavailable → 503 `ai_unavailable` in the house
    envelope; vendor failures surface as that, never a 500, never an error
    screen. GET /api/ai/status → {ask, discount} (authed) gates the UI.
  - **R9 (PR5) — Ask PLENUM with production controls.** POST /api/ai/ask:
    server-composed system prompt (0008/0010 whitelisted view schemas + §5
    dictionary digest + hard rules) → model SQL → sqlparser AST validation
    (exactly one statement; Query-only; SELECT INTO/locks refused; every
    relation ∈ {v_order_facts, v_territory_period, v_rep_period,
    v_product_period, v_customer_period, v_defection_risk} with
    query-defined CTEs allowed and their bodies walked; FROM-position table
    functions refused; a small function denylist — set_config, backend
    signals, file reads, the *_to_xml family — belt-and-braces under the
    grants) → execution ONLY inside the caller's READ-ONLY rls transaction
    (rls_readonly_tx — the read-only SET ordered before the GUC
    set_config, per the ruling) with SET LOCAL statement_timeout='5s',
    wrapped `SELECT row_to_json(plenum_ask) FROM ( sql ) plenum_ask LIMIT
    500` (truncated flag at 500; ordered columns from a server-side
    describe). Validation/timeout = typed 422; the CANONICAL validated SQL
    is always in the 200 (receipts). The validator is a pure function with
    its own adversarial matrix; the one runtime sqlx::query use is this
    execution path, documented in place. UI: /ask + nav + global
    Cmd-K/Ctrl-K focus (Shell); table (contained scroll) + recharts bar
    (tokens-only via CSS vars; one label col + ≥1 numeric + ≤50 rows;
    *_cents charted in dollars) + the SQL receipts block; the 7-question
    library ALWAYS renders (each a client-side link to a live screen); ask
    off (flag/key/503) = the quiet note + library, never an error screen.
    recharts owed decision RESOLVED: USED (first bundle entry; ~774 KB main
    chunk noted for P5 code-split).
  - **R10 — Discount recommender.** POST /api/ai/discount-recommendation
    (authed; 503 when the flag is off): comparables under the CALLER'S
    rls_tx — same family × account industry × same order-of-magnitude
    line-gross band (log10 bucket of cents, computed digit-exact, stated in
    the receipts as band_label) → {count, median/p25/p75, ≤10 sample lines};
    narrative from the R8 seam when a key is present; without one (or on
    vendor failure) narrative:null, degraded:true — the spec's exact
    degradation. A rep's comparables come from their own scope — disclosed
    behavior. UI: per-line COMPS button in the builder (on demand, never per
    keystroke), hidden entirely when status.discount is false.
  - **R11 (PR6) — Identity single-seam.** Honored: no P4 code reads
    session/auth internals outside SessionUser + the rls.rs helpers; the
    read-only variant lives IN rls.rs (the seam), not in ai/.
  - **R12 (PR7) — Seed framed as importer.** Comment-level seam markers in
    seed main.rs: world-generation vs DB-load boundary; the load path marked
    as the future ERP-extract-loader seam. No loader built.
  - **R13 (PR8) — Telemetry ingest stub: BUILT** (not cut). POST
    /api/telemetry/filter-life — role=admin (the integration-feed identity),
    422 outside 0–100 / non-numeric, 404 unknown serial, updates
    installed_units.filter_life_pct, echoes {unit_id, serial,
    filter_life_pct}. Written as the inbound-feed template. The R4 telemetry
    branch consumes it on the next generation.
  - **R14 — Account 360 signals fill.** accounts.rs replaces `signals: []`
    with the account's signals (active first, score DESC, cap 20) in the
    SAME enriched shape as the list (shared loader in signals.rs);
    Account360.tsx renders compact receipt cards linking to /signals; the
    designed empty state remains.
- **Consequential touchpoints beyond the prompt's named list (all
  rulings-driven, disclosed):** rls.rs (+rls_readonly_tx — R9's ordered
  read-only helper, kept in the identity seam per R11), QuoteBuilder.tsx
  (R6 prefill + R10 COMPS panel), Account360.tsx (R14 render),
  TerritoryBoard.tsx (prop pass-through for the R7 tile counts),
  DrillDrawer.tsx (owns useDefection lazily so Command stops calling it —
  R7 verbatim), the three prior test harnesses (AppState gained the ai
  field; they pin a hermetic keyless AiConfig so `cargo test` can NEVER
  reach the vendor regardless of .env).
- **Flagged discrepancy (report-don't-redecide):** acceptance check 7's
  "builder shows NO comps button" with only the key empty contradicts R8/R10
  (flag alone gates the endpoint/button; key gates the narrative) AND check
  10's "key removed → same panel, comparables only". Shipped the R8/R10
  behavior; check 7's comps clause is observable verbatim by also setting
  AI_DISCOUNT_ENABLED=false.
- **Shipped:** migrations/0012_signals_ai.sql (signals dedupe_key +
  order_line_id + unique index; signal_policy seeded config;
  v_defection_risk over the config row; generate_signals() + grants);
  crates/api/src/ai/{mod,client,validate}.rs; routes/{signals,telemetry}.rs;
  error.rs 503 variant; state.rs AiConfig; rls.rs read-only helper; main.rs
  env load; accounts.rs 360 fill; routes/mod.rs registrations; seed main.rs
  generation hook + R12 markers; workspace deps reqwest 0.12.28 (MIT/
  Apache-2.0, default-features off, json+rustls-tls) + sqlparser 0.62.0
  (Apache-2.0, visitor feature); tests signals_http.rs (8) + ai_http.rs (7)
  + 7 validator unit tests; web: signals/Signals.tsx, ask/Ask.tsx,
  lib/{signals,ai}.ts + types, Command/KpiRow/Tile/TerritoryBoard/
  DrillDrawer rewire, Shell nav + Cmd-K, App routes, QuoteBuilder prefill +
  COMPS, Account360 signals panel, tripwire 11×5 + signals-scope;
  .env.example AI keys; .sqlx regenerated; README P4 section; this log;
  master-plan; CLAUDE.md.
- **Checks status (outputs in the session report):** PRE-1…PRE-6 PASS
  (frozen anchors byte-identical; Ridgeline defection fuel with
  FLT-STATSAFE-GS3 ×32 and the $34,000 win-back opp; 28 conquest rows incl.
  Alpenglow's three; 38 reorder candidates; anomaly rung 90d=176; .env
  ignored, no key material in tree). scripts/check.sh ALL CHECKS PASSED
  (fmt · clippy -D warnings · sqlx prepare --check · 56 tests = 12 domain +
  7 validator + 22 prior HTTP + 8 signals_http + 7 ai_http). Two seed runs
  byte-identical incl. same-day signal counts. npm run build clean (tsc
  strict). Tripwire 55/55 layout + command-scope + pipeline-scope +
  signals-scope PASS. Browser-driven internal walk: Ridgeline card #1 in
  the defection lane with all four receipts; Draft Quote opened the builder
  on the win-back opp pre-filled FLT-STATSAFE-GS3 ×32; signal flipped
  actioned `quote_drafted:<quote_id>`; Command KPI OPEN SIGNALS + tile
  counts live; /ask off-state + 7-link library.
- **NEW ANCHORS:** frozen set unchanged (orders 17353/11556020473 ·
  order_lines 25497/−166812187229 · opportunities 16/3367519569 · mv
  120/195/1699/614 · audit_log 17 at seed). CLOCK-DRIFTING (recompute,
  never pin): per-type signal counts — build-day (UTC 2026-07-21) values
  38 reorder / 12 defection / 28 conquest / 173 anomaly = 251 TOTAL
  (the anomaly window slides daily; scores/days-silent drift daily; the
  dry-run earlier the same evening read 176 anomalies across the UTC date
  tick — the class in action). audit_log growth: +251 signal-INSERT rows
  after the first post-seed generation (audit = 17 at the seed's printed
  count, before the hook fires); test-suite runs add write-back/restore
  audit rows until the next reseed.
- **New dependencies:** reqwest 0.12.28 (MIT OR Apache-2.0; ~min-features)
  and sqlparser 0.62.0 (Apache-2.0; sqlparser-rs; +visitor) — both
  pre-authorized by the unit constraints; zero new npm packages (recharts
  ^3.9.2 already installed, now used).
- **P5-parked (appended):** leakage outlier feed reads signal_policy ·
  signal auto-expiry when predicates stop holding · recharts owed decision
  RESOLVED — used by the Ask chart (main bundle ~774 KB; consider
  code-splitting) · signals/ask lanes could virtualize the VP's ~170-card
  anomaly lane · main-chunk >500 KB Vite warning.
- **Phase gate: P4 ACCEPTED** — D.'s literal "merge" reply, 2026-07-21
  (merge = approval per this unit's pre-authorized PHASE 2 protocol, the P1
  precedent). Attribution for the record: no per-check acceptance walk was
  run in-session before the order — the gate rests on the merge order plus
  the build evidence in this entry (check.sh 56/56, two identical seed runs,
  tripwire 55 layout + 3 scope, and the browser-driven internal walk of gate
  P4-1 both halves and gate P4-2's flag-off half). The 12-check walk in the
  session report remains OWED as D.'s own observation pass (the checks that
  write data plus the terminal checks: tripwire, reseed, API restart) — run
  it before the demo rehearsal; any failure reopens the gate.
- **Commit:** built across `p4-signals-ai` (`d7d629f` schema+seed →
  `fafc1b5` API → `0fa7ab8` tests → `37f81e1` web → `8448264` tripwire →
  `59a0598` docs; this acceptance/merge record added in the closeout commit
  on main).
- **Merge record:** `56cdd9b` — p4-signals-ai merged to main (--no-ff),
  2026-07-21 22:21:18 -0400, on D.'s "merge". Staleness check passed (main
  still `964749f` at merge time). Repo remains local-only; branch
  p4-signals-ai kept, per precedent.
- **Acceptance record (appended 2026-07-21 — supersedes the owed-walk line
  above):** the 12-check walk WAS run before the merge order, in the Cowork
  architect session, 2026-07-21 evening (ET; UTC 07-21→22): 12/12 PASS. CC
  could not see that session, hence the honest owed-walk note, now
  discharged. Attribution per the amended browser-drive precedent — checks
  1–6, 8–11 Cowork-driven in D.'s browser under D.'s observation: P4-1 both
  halves (Ridgeline defection card emergent with receipts, 338 days silent,
  score 82,718.95 exact to formula; Draft Quote prefilled FLT-STATSAFE-GS3
  × 32 onto the seeded win-back opp, no duplicate opp, signal actioned
  quote_drafted:<quote id>); rep queue all-SE-1 + foreign write 404; write-
  backs (assign chip, log-call → 360 activity + actioned, dismiss reason-
  gated); double regenerate 0/0 all types, dismissed card never resurrected,
  totals stable; Command rewire (rep KPI == API, VP 248 = 36/11/28/173,
  tiles == summary, basis flip leaves counts still); P4-2 all three halves
  (key-off library + zero error states + degraded COMPS; live ask "top 10
  customers by net revenue in 2025" → 10 rows + chart + SQL, top row
  Vantage Metalworks $731,372.44 cent-exact vs Leaderboards; the rep's
  territory question returned ONE row, Southeast 1 — the generated SQL
  carried NO territory filter, scope came from the RLS session); COMPS
  narrative over a 9-line Ridgeline-history cohort (median 3.9%, IQR
  3.0–4.6%) concurrent with the VP-approval policy verdict; telemetry push
  8% on SN-GS3-00001 → regenerate → telemetry reorder card, score 71,516.16
  = 37,248 × 1.92 exact; day-boundary regenerate showed inserted 2 /
  updated 36+11+172, then same-day 0/0. Check 7's comps clause observed per
  the amended R10 reading (button shows keyless, comparables-only) — the
  unit prompt's no-button wording was Cowork's internal contradiction,
  resolved by CC per rulings, ratified at acceptance. Check 12 in D.'s own
  terminal: tripwire 55/55 + 3 scope PASS; reseed frozen anchors
  byte-identical (17353/11556020473, 25497/−166812187229, 16/3367519569,
  mv 120/195/1699/614, audit_log 17) with next-day signal counts
  39/12/28/172 = 251 (clock-drift class, benign); API restart → queue
  persisted (Ridgeline atop defection at 339 days, 82,963.68 exact).
  Incidental proof: a real vendor-side failure during the walk (new console
  org, zero credits) exercised the R8 posture live — typed 503, Ask folded
  to the library, no screen errored. P5 additions from the walk:
  generate_signals() ~2.1s and the enriched active-list ~1.05s tripped the
  1s slow-statement alert (index candidates); COMPS cohort honestly empty
  at tiny line-gross bands.

## 2026-07-19 · P3 CRM operational core

- **Unit:** P3 (Account 360 + installed-base timeline, Pipeline kanban with
  stage write-back + Won-books-order, Quote builder + approval state machine +
  audit UI, Activities) — branch `p3-crm-core` from main `a6805eb`. Tier 3,
  one-and-done. Repo LOCAL-ONLY.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Architect rulings recorded (R1–R10, verbatim intent):**
  - **R1 — Won books an order.** `PATCH /api/opportunities/:id/stage` to `won`
    requires ≥1 quote in status `approved`; else 422 (`"won requires an approved
    quote"`). Booking copies the most-recent approved quote's lines verbatim
    (list/net/discount triplet — passes the order_lines CHECK by construction)
    into a new order; `ordered_on = CURRENT_DATE`; account/territory from the
    opp; `rep_id = opportunities.owner_id`; `site_id` = the account's MIN(id)
    site. The consumed quote flips to `accepted`. `won`/`lost` terminal (any
    transition out = 422); `lost` requires `lost_reason`. Because the live
    quarter reads v_order_facts directly, a booked order moves the numbers
    immediately and `refresh_rollups()` must NOT change them (refresh
    invariance is itself a check).
  - **R2 — Seed gains a deterministic opportunity book (additive only).** ~14
    opps across territories/stages (lead→negotiation)/kinds PLUS story beat 6:
    the Ridgeline win-back opp (SE-1, owner serena, filter-program, qualified,
    ≈3_400_000 cents, no quote — D. drafts it live, gate P3-1). Separate RNG
    stream (StdRng seeded from SEED xor a NEW documented constant), appended
    AFTER all existing draws, territory always == the account's; NO change to
    accounts/orders/units/products/users. Frozen anchors identical.
  - **R3 — Thresholds become real seed-config.** New table `discount_policy`
    (self_max_pct 10.00, manager_max_pct 25.00), seeded, read per request.
    Submit computes the worst-line discount → verdict: ≤ self → auto-approved
    (status approved, approver=creator, `self_approved`); ≤ manager →
    pending_approval (regional_manager/vp/admin); > manager → pending_approval
    (vp/admin only). The approve/reject HANDLER enforces the role tier. Reject
    requires a reason.
  - **R4 — Audit trail app-immutable + scoped reads.** Migration 0011 `REVOKE
    UPDATE, DELETE ON audit_log FROM plenum_app` (INSERT + SELECT stay). Audit
    UI reads ONLY via `GET /api/quotes/:id/audit` — joined through the RLS'd
    quote (invisible quote → 404), actor names resolved. No generic /api/audit.
  - **R5 — Account 360 payload.** header + cumulative gross/net/leakage (from
    v_order_facts under RLS), sites, contacts, installed units (timeline),
    recent orders (capped), opportunities, activities (paginated), `signals:
    []` (P4 empty state). Invisible account → 404. NULL
    expected_changeout_months rendered as a "cadence unknown" chip — the mess
    is the feature, not a bug.
  - **R6 — POST /api/accounts ships route-only** (name/industry/territory_id/
    status/parent; scope enforced; 422 on garbage). No P3 screen; one curl.
  - **R7 — Navigation.** Rail gains Pipeline + Quotes. Account 360 lives at
    `/accounts/:id`, reached by clicking rows/cards (incl. Leaderboards
    customers rows). No dead links.
  - **R8 — List/pagination discipline unchanged.** Every new list: envelope
    `{items,limit,offset,total}`, limit max 200 (422 above), empty = 200,
    typed 401/403/404/422.
  - **R9 — Migration 0011 additive only.** quotes ADD
    discount_policy_result/submitted_at/decided_at/decision_reason; CREATE
    discount_policy; the audit REVOKE; grants (SELECT to plenum_app). Wes
    quote's discount_policy_result + submitted_at backfilled in seed.
  - **R10 — Client gets policy via GET /api/policy/discount** so the builder's
    live verdict is client-computed from server truth; submit recomputes
    server-side regardless (client verdict advisory, server verdict law).
- **Beyond the R-route-list (flagged, not hidden):** two supporting READS the
  §8 screen-7 builder + detail require — `GET /api/products` (global catalog
  for the picker; auth-guarded, non-RLS) and `GET /api/quotes/:id` (detail with
  lines + verdict; RLS via the quotes join). Both safe; reported openly.
- **Shipped:**
  - migrations/0011_crm_core.sql (quotes columns, discount_policy 10/25 seeded,
    audit REVOKE, grants).
  - crates/domain/src/discount.rs (DiscountPolicy / ApprovalTier /
    role_can_decide + 3 unit tests) — the R3 governance logic shared by seed
    and API.
  - crates/api/src/routes/: common.rs, accounts.rs (get_account 360 +
    create_account), opportunities.rs (list/create/patch_stage + R1 booking),
    quotes.rs (list/create/get/submit/approve/reject/audit), policy.rs,
    products.rs, activities.rs; mod.rs (routes + `patch`); api gains
    rust_decimal (workspace dep).
  - crates/seed/: story_beats.rs (opp book on isolated RNG stream + Wes verdict
    backfill), data.rs / insert.rs / main.rs (quote columns, opp checksum +
    per-stage output); seed gains serde_json (already a project dep via api).
  - crates/api/tests/crm_http.rs — 9 adversarial/integration tests.
  - web/: lib (apiPatch, CRM types, crm.ts hooks + mutations); crm/ (Timeline,
    Account360, Pipeline, Quotes, QuoteDetail, QuoteBuilder, badges, verdict);
    Shell nav (+Pipeline +Quotes, mobile-wrap); App routes; leaderboards
    customer-row → 360 link (metrics.rs untouched); tripwire.spec.ts extended.
  - .sqlx regenerated (88 files). Docs: this log, master-plan, CLAUDE.md.
- **Checks status (outputs in the session report):** scripts/check.sh ALL
  CHECKS PASSED (fmt · clippy -D warnings · sqlx prepare --check · 34 tests:
  12 domain unit + 13 prior HTTP untouched + 9 crm_http). Preconditions 1–6
  PASS (frozen anchors byte-identical across two seed runs; Ridgeline SE-1
  1 site/5 units; Wes 28% pending intact w/ vp_approval backfill; Harbor
  Steel/Gulf Coast NULL-vs-real cadence contrast in NE-1/SC-1; every opp has a
  site; opps 16 = lead 3/qualified 5/quoted 4/negotiation 4). Adversarial
  matrix green (401 every route; rep foreign 404s; rep-approve-own 403; RM
  >25% 403 / RM 10–25% 200; VP 28% 200 audit actor=VP; submit-non-draft /
  approve-draft / won-no-quote / lost-no-reason / out-of-won / limit=201 all
  422; forged prices ignored; Σ order == Σ quote gross+net; audit_log UPDATE/
  DELETE denied to plenum_app). Tripwire 45/45 layout + command-scope +
  pipeline-scope PASS. P3-1 + P3-2 round-trips proven over HTTP.
- **New anchors:** opportunities **16** (lead 3 / qualified 5 / quoted 4 /
  negotiation 4), opp checksum **3367519569**, quotes **1**; tripwire **45/45**
  layout + 2 scope. Frozen anchors unchanged: orders 17353/11556020473,
  order_lines 25497/-166812187229, mv 120/195/1699/614, customers CUM NET
  footer $24,670,890.87.
- **New dependencies:** none external — `rust_decimal` added to crates/api and
  `serde_json` to crates/seed are BOTH already project dependencies (workspace
  crates), no new crate enters the tree.
- **Phase gate: P3 ACCEPTED** — D.'s acceptance run, 2026-07-20, all 11
  checks PASS. Attribution: checks 1–6, 8, 9a driven by Cowork in D.'s
  browser under D.'s observation (browser-drive precedent amended by D. for
  P3 to include writes); checks 7, 9b (API restart survival), 10 (tripwire
  45/45 + 2 scope), 11 (reseed) run in D.'s own terminal; persistence and
  reseed re-verification Cowork-driven under observation. Observed: P3-1 —
  28% quote → pending_approval → VP approve, audit trail 3 rows with
  actors/timestamps on screen. P3-2 — booking $7,948.80 net, serena
  cumulative $2,783,017.15 → $2,790,965.95 exact, refresh-invariant.
- **Corrections for the record (D., 2026-07-20, at acceptance):**
  1. Serena's true cumulative anchor is **$2,937,783.00 gross /
     $2,783,017.15 net** — the P3 unit prompt's CURRENT STATE line
     ($12.9M/$10.8M) was Cowork's wrong reconstruction of digit-truncated
     skill text; CC's repro number was correct all along. (Grep confirms the
     wrong figure never entered any repo doc.)
  2. Post-booking `refresh_rollups()` transiently reports mv_product_period
     **1700**: the booked current-quarter order enters the matview but is
     read-filtered by the < current-quarter boundary — benign by design, no
     read surface changes; reseed restores 1699.
- **Commit:** built across `p3-crm-core` (`0a72011` schema+seed → `d3cdffa`
  API → `81c23fe` tests → `08be128` web → `05241e4` tripwire → `7ac1e08`
  docs; this acceptance/merge record added in the closeout commit on main).
- **Merge record:** `c8936ec` — p3-crm-core merged to main (--no-ff),
  git-stamped 2026-07-19 22:06:13 -0400 (machine clock), on D.'s "merge"
  with the 2026-07-20 acceptance record. Repo remains local-only; branch
  p3-crm-core kept, per precedent.

## 2026-07-19 · P2 Command + Leaderboards UI

- **Unit:** P2 (web/ scaffold, tokens, auth+shell, Command w/ Territory Board,
  Leaderboards w/ period/basis/kind/group controls, CSV export, Playwright
  responsive tripwire) — branch `p2-command-ui` from main 84c030d.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Architect resolutions recorded:** defection-risk KPI stands in for
  open-signals until P4 (signals table empty by design); basis toggle flips
  every dollar figure + board rank, leakage%/coverage%/defection count
  basis-invariant by metric definition, attainment always net; territory
  drill = client-composed drawer (no territory param exists server-side);
  Vite proxy serving model, API untouched on 127.0.0.1:5777; react-router 7
  added (spec stack named no router); URL-state controls; fetch-all ≤200
  tables; client CSV; 4×2 cartogram (CW CE MW NE / W MT SC SE) w/ compact
  scoped variant; relative leakage LED bands (aggregate, +3pts); full §8
  stack installed incl. recharts (idle until P3).
- **Gate amendment 2026-07-19 (architect ruling, in-session):** frozen seed
  yields no territory re-rank at 2026/cumulative and no rep-#1 flip
  (preconditions proven: territory gross/net order identical at 2026 AND
  cumulative, differs only 2023/2024; rep #1 = Wes Turner under both bases
  every period; leakage rep = Wes Turner, #1 gross with board-worst leakage
  14.31%). P2-1's re-rank observable RELOCATED to the customers tab (P1-1's
  proven surface — customers 2025 gross→net: Vantage Metalworks Coastal
  drops out of the net top-10, Blue Ridge Fabrication enters). Command
  toggle proves the every-dollar flip; leakage beat = worst-leakage-at-#1.
  No seed/SQL/Rust change; no synthetic motion. Evidence = precondition
  outputs at top of this unit's session report.
- **Port amendment (D.'s call 2026-07-19):** the web dev server's usual port
  5173 was held by another tenant of this machine (never-touch rule), so D.
  moved PLENUM's Vite dev server to **127.0.0.1:5177**. The API is unchanged
  on 5777; the web page proxies /api → 5777. Recorded as an amendment the
  way the 8080→5777 move was.
- **Dependency disclosure (constraint 2):** `@types/node` (dev-only,
  DefinitelyTyped, MIT) added beyond the Resolution-11 list — required for
  `process.env.VITE_API_TARGET` in vite.config.ts. No runtime dependency.
- **Shipped (all under web/, plus docs):**
  - Scaffold: package.json, tsconfig(.app/.node), vite.config.ts
    (host 127.0.0.1 port 5177 strictPort, proxy /api→5777), index.html,
    .gitignore; scripts dev | dev:lan | build (tsc -b && vite build) |
    tripwire.
  - src/styles/tokens.css — §8 palette in Tailwind v4 @theme, nameplate +
    tabular utilities, seam elevation, motion tokens.
  - src/lib/ — api.ts (fetch wrapper, typed ApiError), queryClient.ts (401
    → purge+redirect), format.ts (money/percent), types.ts (payload
    mirrors), params.ts (URL grammar), rank.ts (client re-rank), queries.ts
    (metrics hooks, basis-independent keys), csv.ts (BOM+CRLF export),
    useScreenReady.ts.
  - src/auth/ — auth.ts (useMe/useLogin/useLogout w/ clear() on login+logout),
    RequireAuth.tsx (guard), Login.tsx. src/App.tsx (routes + 401 listener),
    main.tsx. src/shell/Shell.tsx (rail + user chip + logout).
  - src/command/ — Command.tsx, KpiRow.tsx, TerritoryBoard.tsx, Tile.tsx,
    Led.tsx, DrillDrawer.tsx. src/components/ — Segmented, BasisToggle,
    states.
  - src/leaderboards/ — Leaderboards.tsx, Controls.tsx, DataTable.tsx
    (TanStack), columns.tsx (reps/items/customers + footers + CSV maps).
  - tripwire.spec.ts + playwright.config.ts.
- **Checks status (internal, output pasted in the session report):**
  zero-Rust-diff (git diff main -- crates/… migrations/… empty) · npm run
  build clean (tsc strict) · scripts/check.sh ALL CHECKS PASSED · tripwire
  25/25 layout + rep-scope PASS · anchor customers cumulative net footer
  $24,670,890.87 == API sum 2467089087 · adversarial: cross-login cache
  purge (VP 8 tiles → rep 1 SE-1 tile, no ghost), rep CSV scope (5 SE-1
  rows, 0 foreign), unauth deep-link → login, tripwire rep-scope · gate
  P2-1 (KPI flip + every-dollar flip, order holds at 2026 per ruling) +
  3b (customers re-rank on screen) + amended 5 (Wes #1 gross, worst
  leakage 14.3%) · error-state quiet ErrorPanel + Retry recovers ·
  regression anchors unchanged (17353/11556020473, 25497/-166812187229,
  120/195/1699/614).
- **Phase gate: P2 ACCEPTED** — D.'s acceptance run, 2026-07-19, all checks
  PASS: checks 1–6 and 3b under D.'s own hands; checks 7–8 driven by Cowork
  in D.'s browser under D.'s observation (D. opened and verified both export
  files); check 9 (tripwire) in D.'s terminal; check 10 passed in the amended
  desktop form (architect ruling 2026-07-19: window-resize sweep across the
  widths; the real-tablet portrait/landscape check is deferred to P5).
- **Commit:** `d0741e8` on `p2-command-ui` (this log line added in the
  immediate follow-up commit `a605957`).
- **Merge record:** `de0be08` — p2-command-ui merged to main (--no-ff),
  2026-07-19 17:11:50 -0400, on D.'s "merge". Repo remains local-only.

## 2026-07-18 · P1 Metrics core

- **Unit:** P1 Metrics core (v_order_facts + v_unit_facts, four mv_* rollups
  + scoped read views + refresh_rollups(), 7 metric endpoint groups,
  dual-basis, pagination) — branch `p1-metrics` from main a67bb39.
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Architect resolutions recorded:** metric 7 ships as GET /metrics/defection
  (spec §10 "all 7 groups" vs §7 six-route list); matview scoping = grant
  boundary (no plenum_app grant on raw mv_*) + scoped views carrying the 0005
  predicate verbatim; v_order_facts/v_unit_facts are security_invoker
  (plenum_admin is superuser — definer views would bypass RLS); refresh via
  SECURITY DEFINER refresh_rollups() gated by role=admin in the handler;
  cumulative/ttm read v_order_facts directly, quarters/years read rollups.
- **Shipped:**
  - migrations/0008_order_facts.sql — v_order_facts + v_unit_facts, both
    WITH (security_invoker = true); SELECT grants to plenum_app.
  - migrations/0009_rollups.sql — mv_territory_period / mv_rep_period /
    mv_product_period / mv_customer_period, keyed (entity, territory_id,
    quarter_start), WITH NO DATA, unique key indexes, deliberately NO
    plenum_app grants (the enforcement boundary).
  - migrations/0010_scoped_reads.sql — v_territory_period / v_rep_period /
    v_product_period / v_customer_period (definer views: 0005 v_user_scope
    predicate verbatim on BOTH branches + live-current-quarter UNION ALL,
    boundary pair on date_trunc('quarter', now())); v_defection_risk
    (security_invoker, P4 reuses it); refresh_rollups() SECURITY DEFINER
    (search_path pinned, EXECUTE revoked from PUBLIC, granted to
    plenum_app); SELECT grants on the scoped views.
  - crates/domain/src/period.rs — period/basis/kind grammar parser (pure
    logic, 5 unit tests); domain lib.rs/Cargo.toml wiring (chrono from
    workspace deps — no new dependency).
  - crates/api/src/routes/metrics.rs — all 7 metric endpoint groups; static
    sqlx queries only (bind-parameter CASE for basis/by, null-folded
    kind/date filters); rollup path for quarter/year + kind=all, live
    v_order_facts path for cumulative/ttm and kind-filtered slices.
  - crates/api/src/routes/admin.rs — POST /api/admin/refresh-rollups,
    role=admin gate before the definer call; 401/403 typed.
  - crates/api/src/routes/mod.rs — eight new route registrations.
  - crates/api/src/error.rs — Forbidden comment updated (P1 lands the first
    real 403; variant no longer dead code).
  - crates/api/tests/metrics_http.rs — 8 integration tests: rep scope on
    every endpoint, VP/rep cent-equality, gate P1-1, rollup-vs-live year
    sum, kind-slice zeroing, 401 everywhere, 14-case 422 matrix, refresh
    role gate + stability.
  - crates/seed/src/main.rs — ONLY seed change: post-load
    refresh_rollups() call + one console line per matview with row count.
  - README (P1 acceptance section), master-plan, CLAUDE.md, this log;
    .sqlx regenerated (30 new query files).
- **Checks status (internal, output pasted in the session report):**
  clippy -D warnings UNTRIMMED pasted (debt carried from P0 closeout,
  settled) · 22/22 tests (9 domain unit + 5 P0 HTTP + 8 P1 HTTP) · cargo
  sqlx prepare --check clean · seed determinism: two runs, ORDERS TOTAL
  17353 + checksums (orders 17353/11556020473, order_lines
  25497/-166812187229) + matview row counts (120/195/1699/614) identical ·
  adversarial matrix: rep GUC = SE-1-only on all 7 P1 views, no-GUC = 0
  rows everywhere, garbage GUC = 0 rows, mv_* SELECT as plenum_app =
  permission denied ×4, rep/VP SE-1 cent-equality · P1-1 PASS (ORDER
  DIFFERS True, ALL GROSS>=NET True; SAME TOP-10 SET False — stronger form,
  flagged for audit) · P1-2 PASS (2467089087 == 2467089087) ·
  rollup-vs-live equivalence: 0 mismatched rows on all four scoped views ·
  refresh: rep 403 / VP 403 / admin 200 + row counts, P1-2 unchanged after ·
  restart survival PASS (no re-seed).
- **Anchors for the record:** CUMULATIVE NET (all territories, VP view) =
  2467089087 cents; same number from raw order_lines as plenum_admin =
  2467089087 cents.
- **Owed settled:** untrimmed clippy output pasted in this unit's report
  (carried from P0 closeout).
- **Machine note (report, don't fix):** the bank demo binds 127.0.0.1:8080
  specifically, so PLENUM can bind 0.0.0.0:8080 at the same time and
  localhost:8080 traffic still reaches the BANK DEMO. "One API at a time"
  stands; D.'s acceptance run needs the bank demo stopped first. Internal
  verification ran on BIND_ADDR=127.0.0.1:18080 (env override only; no
  config change — the project stays on 8080).
- **Amendment (D.'s order, 2026-07-18, pre-acceptance):** API port moved
  8080 → **5777**, default bind 0.0.0.0 → **127.0.0.1** — executing the
  parked port-move decision (authorized once the bank demo proved real; the
  loopback-collision finding above was the trigger). PLENUM owns 5777, the
  bank demo keeps 8080, no contention; never-touch rule unchanged. Code
  delta: BIND_ADDR default in api/main.rs + .env.example only.
- **Phase gate: P1 ACCEPTED** — D.'s literal "merge" order, 2026-07-18
  (merge = approval per this unit's protocol), following D.'s acceptance
  run against 127.0.0.1:5777.
- **Commit:** `626d920` on `p1-metrics` (this log line added in the
  immediate follow-up commit); amendment `2b34203`.
- **Merge record:** `2f610ba` — p1-metrics merged to main (--no-ff),
  2026-07-18 19:48:45 -0400, on D.'s "merge". Repo remains local-only.

## 2026-07-17 · P0 Foundation

- **Unit:** P0 Foundation (repo scaffold, schema + RLS + audit triggers,
  deterministic seed, session auth, RLS session middleware, GET /api/accounts)
- **Architect:** Claude (Cowork) · **Builder:** CC (Claude Code)
- **Gate record:** §14 devil's-case gate waived by D. 2026-07-17; execute
  order = lock/go. Recorded here; not re-litigated.
- **Machine adaptations (D.'s calls, 2026-07-17, in-session):**
  - DB host port **5434** → container 5432 (native PostgreSQL services own
    5432/5433 on D.'s machine). All in-container psql commands unaffected.
  - Port 8080 freed by stopping `stack-ledger-api.exe` (Local-Secure-Ops
    bank demo) — D. authorized in-session; PLENUM API keeps 8080.
    **Correction, same day:** that demo is Codex's ACTIVE project, not
    leftover cruft. Standing rule from D.: PLENUM sessions leave the bank
    demo (and Grok Build's work) alone — never stop/modify other agents'
    processes or folders. 8080 is shared serially: run one API at a time;
    a "cannot bind 0.0.0.0:8080" from PLENUM means the bank demo is up,
    which is contention, not a P0 failure.
- **Shipped:**
  - Cargo workspace: `crates/domain` (enums, bp-based money math + property
    test vs the SQL CHECK), `crates/api` (axum 0.8, tower-sessions 0.14
    MemoryStore, argon2id, RLS-transaction helper, typed errors, /api/auth/*,
    /api/accounts), `crates/seed` (deterministic engine, seed 20260717).
  - `migrations/0001–0007`: 9 enums, 16 tables (BIGINT-cents price triplet
    CHECK on quote_lines/order_lines), spec + FK indexes, `v_user_scope`
    recursive scope view, RLS ENABLE+FORCE with fail-closed policies on
    accounts/orders/opportunities/quotes/signals/activities, audit trigger
    on quotes/signals/opportunities, grants for `plenum_app`.
  - Docker compose (postgres:16) + initdb script creating `plenum_app`
    (LOGIN, NOSUPERUSER, NOBYPASSRLS). API connects only as `plenum_app`.
  - Seed world: 8 territories, 17 users (12 reps/3 RMs/VP/admin), 48
    accounts, 60 sites, 107 contacts, 42 products, 232 installed units,
    17,353 orders / 25,497 order lines, all five §9 story beats. Determinism
    proven: two runs, identical count + sum(hashtext(id)).
  - Docs: README (P0 run + acceptance), root + docs copy of spec v01,
    repo CLAUDE.md, this log. `scripts/check.sh` gauntlet; `.sqlx/` committed.
- **Checks status (internal, all output pasted in the session report):**
  clippy -D warnings clean · 8/8 tests pass (money property test + 5 HTTP
  integration tests) · seed gate 17,353 > 15,000 · determinism checksum
  identical across runs · DB-level RLS matrix (rep 6 / RM 22 / VP 48 / no-GUC
  0 / random-uuid 0 / admin negative control) · HTTP matrix (rep SE-1-only,
  VP 8 codes, 401 no-cookie, identical 401s for bad creds, 422 limit=500) ·
  restart survival.
- **Phase gate: P0 ACCEPTED by D., 2026-07-17** ("I'm good with it then"),
  on the basis of the pasted evidence report plus a live browser
  demonstration (no-login 401 → rep sees SE-1 only → VP sees all 8
  territories, identity proven via /api/auth/me at each step). D. waived
  hands-on execution of the 7 checks — recorded as an evidence-based pass,
  not a hands-on pass. Merge not yet ordered; `p0-foundation` unmerged.
- **Out-of-scope observations:** logged in the session report only; nothing
  fixed beyond P0 scope.
- **Commit:** `64e4c13` on `p0-foundation` (this log line added in the
  immediate follow-up commit).
- **2026-07-17 closeout:** D. acceptance 7/7 PASS. Cowork audit PASS
  (evidence tier). master-plan.md added. Next: P1 unit from fresh Cowork
  session (skill plenum-01).
- **Bank-demo verification:** VERIFIED REAL: bank demo exists on this
  machine; b93e3d3 record stands. (Fresh disk check at closeout:
  stack-ledger-api.exe, Start-Bank-Demo.ps1, Check-Bank-Demo.ps1, and
  bank-demo-startup.log all present; this session also observed the
  process running from that exe before D.'s authorized stop.)
- **Owed carry-forward:** CC owes untrimmed clippy output in next unit's
  report.
- **Merge record:** `d4f512d` — p0-foundation merged to main (--no-ff),
  2026-07-17 22:47:10 -0400, on D.'s "merge". Repo remains local-only
  (no remote configured).
