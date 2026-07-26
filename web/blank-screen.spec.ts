// D-3 (2026-07-26): a blank screen must be unreachable.
//
// The defect this locks down: the four lazy routes (Ask, Territory Map,
// Leakage, Data Quality) sat under a <Suspense> and nothing else. Suspense
// handles the PENDING state of a dynamic import(); it has no error path. When
// the chunk download fails, React.lazy rethrows the rejection during render,
// no boundary catches it, and React unmounts the entire root — nav, header,
// user badge and all. D. reproduced it live on 2026-07-26 with Wi-Fi off:
// black viewport at /leakage, console reading
//   TypeError: Failed to fetch dynamically imported module:
//   https://plenum.onrender.com/assets/Leakage-Bnf7PrT6.js
// and only a manual page reload recovered.
//
// Four laws, one spec:
//   1 a lazy chunk that fails to download -> panel INSIDE the surviving shell
//   2 any uncaught render error           -> panel, never an empty document
//   3 retry after connectivity returns    -> the screen loads, no page reload
//   4 healthy network                     -> all four routes load as before
//
// D-4 (2026-07-26) adds the fifth, and it is here because of how the first
// four failed to catch a regression D-3 itself introduced:
//   5 clicking any destination shows THAT destination — from anywhere, in any
//     order, lazy route to lazy route included.
// Every one of the nine D-3 specs passed while the app was visibly broken,
// because each loads a route FRESH and then asserts on the URL. The URL was
// correct the entire time: /data-quality in the address bar with the territory
// map still on screen. So the D-4 specs (a) navigate, rather than loading
// fresh, and (b) assert on body[data-screen], which the mounted SCREEN writes
// about itself — never on the URL, which is the router's opinion, not the
// DOM's.
//
// Requires the API on 127.0.0.1:5777 (this dev server only proxies to it).

import { test, expect, type Page } from "@playwright/test";
import {
  RENDER_ERROR_EVENT,
  ROOT_RENDER_ERROR_EVENT,
} from "./src/lib/events";

const PASSWORD = "demo-plenum-2026";
const VP = "valerie.price@plenum.demo";

/** The lazy routes, by the module basename their chunk carries in dev AND
 *  in a production build (`/src/leakage/Leakage.tsx` · `assets/Leakage-*.js`). */
const LAZY = [
  { path: "/leakage", module: "Leakage", nav: "Leakage", screen: "leakage" },
  { path: "/map", module: "TerritoryMap", nav: "Territory Map", screen: "map" },
  { path: "/ask", module: "Ask", nav: "Ask", screen: "ask" },
  {
    path: "/data-quality",
    module: "DataQuality",
    nav: "Data Quality",
    screen: "data-quality",
  },
] as const;

/** Eager screens, for the lazy→eager→lazy legs and the full-nav walk. */
const EAGER = [
  { nav: "Command", screen: "command" },
  { nav: "Leaderboards", screen: "leaderboards" },
  { nav: "Pipeline", screen: "pipeline" },
  { nav: "Quotes", screen: "quotes" },
  { nav: "Signals", screen: "signals" },
] as const;

/** The nine nav destinations in the order they appear in the rail. */
const NAV_ORDER = [
  { nav: "Command", screen: "command" },
  { nav: "Territory Map", screen: "map" },
  { nav: "Leaderboards", screen: "leaderboards" },
  { nav: "Leakage", screen: "leakage" },
  { nav: "Pipeline", screen: "pipeline" },
  { nav: "Quotes", screen: "quotes" },
  { nav: "Signals", screen: "signals" },
  { nav: "Ask", screen: "ask" },
  { nav: "Data Quality", screen: "data-quality" },
] as const;

/** Click a nav link and wait for whatever screen ends up mounted to settle.
 *  Deliberately does NOT wait for the destination — waiting for the thing
 *  under test would hide the defect behind a timeout. */
async function clickNav(page: Page, nav: string): Promise<void> {
  await page.getByRole("link", { name: nav, exact: true }).click();
  await page.waitForTimeout(1_200);
  await page
    .waitForSelector('body[data-screen-ready="true"]', { timeout: 20_000 })
    .catch(() => {});
}

/** What is ACTUALLY mounted, straight from the screen's own marker. */
async function mountedScreen(page: Page): Promise<string> {
  return (await page.evaluate(() => document.body.dataset.screen)) ?? "(none)";
}

/** Matches the chunk for one lazy screen, dev-served or built. */
function chunkOf(module: string) {
  const re = new RegExp(`/${module}(\\.tsx|-[A-Za-z0-9_-]+\\.js)(\\?|$)`);
  return (url: URL) => re.test(url.pathname + url.search);
}

/** The shell's own furniture — what must SURVIVE a screen failure. */
async function shellIsAlive(page: Page): Promise<void> {
  await expect(page.getByTestId("user-chip")).toBeVisible();
  await expect(page.getByRole("link", { name: "Leakage" })).toBeVisible();
  await expect(page.getByText("PLENUM", { exact: true }).first()).toBeVisible();
}

async function loginAs(page: Page, email: string): Promise<void> {
  await page.goto("/login");
  await page.getByTestId("login-email").fill(email);
  await page.getByTestId("login-password").fill(PASSWORD);
  await page.getByTestId("login-submit").click();
  try {
    await page.waitForURL("**/command", { timeout: 15_000 });
  } catch (e) {
    throw new Error(
      `Login as ${email} never reached Command. The API on 127.0.0.1:5777 is ` +
        `probably not running — start it with: cargo run --bin api  (${String(e)})`,
    );
  }
}

for (const route of LAZY) {
  test(`D-3: ${route.nav} — a chunk that never downloads renders a panel, not a blank document`, async ({
    page,
  }) => {
    await loginAs(page, VP);
    await page.route(chunkOf(route.module), (r) => r.abort("failed"));

    await page.getByRole("link", { name: route.nav, exact: true }).click();

    // The panel is there, named and retryable …
    const panel = page.getByTestId("error-boundary");
    await expect(panel).toBeVisible({ timeout: 20_000 });
    await expect(panel.getByRole("button", { name: "Try again" })).toBeVisible();

    // … it says the code did not download, and does NOT blame the API …
    const line = (await panel.getByTestId("error-message").textContent()) ?? "";
    expect(line).toContain("didn’t finish downloading");
    expect(line).not.toContain("reach the API");

    // … and the shell is still standing, which is the whole point.
    await shellIsAlive(page);
    expect(await page.locator("body *").count()).toBeGreaterThan(20);
  });
}

test("D-3: an uncaught render error renders a panel inside the shell, never an empty document", async ({
  page,
}) => {
  await loginAs(page, VP);
  await page.waitForSelector('body[data-screen-ready="true"]', {
    timeout: 30_000,
  });

  // Acceptance check 5's one-liner, exactly as D. will type it.
  await page.evaluate(
    (name) => window.dispatchEvent(new Event(name)),
    RENDER_ERROR_EVENT,
  );

  const panel = page.getByTestId("error-boundary");
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel.getByRole("button", { name: "Try again" })).toBeVisible();
  await shellIsAlive(page);
});

test("D-3: an error ABOVE the shell hits the root boundary — a panel, not an empty document", async ({
  page,
}) => {
  await loginAs(page, VP);
  await page.waitForSelector('body[data-screen-ready="true"]', {
    timeout: 30_000,
  });

  // Nothing renders outside this one. The shell goes — it is what failed —
  // but the document must not.
  await page.evaluate(
    (name) => window.dispatchEvent(new Event(name)),
    ROOT_RENDER_ERROR_EVENT,
  );

  const panel = page.getByTestId("error-boundary");
  await expect(panel).toBeVisible({ timeout: 10_000 });
  await expect(panel).toHaveAttribute("data-region", "app root");
  expect(
    (await panel.getByTestId("error-message").textContent()) ?? "",
  ).toContain("PLENUM failed to render");
  expect((await page.locator("body").innerText()).trim().length).toBeGreaterThan(
    20,
  );
});

test("D-3: retry after the network returns loads the screen, with no page reload", async ({
  page,
}) => {
  await loginAs(page, VP);

  // Offline for the first attempt only — exactly D.'s "Wi-Fi off, then on".
  let attempts = 0;
  await page.route(chunkOf("Leakage"), (r) => {
    attempts += 1;
    return attempts === 1 ? r.abort("failed") : r.continue();
  });

  await page.getByRole("link", { name: "Leakage", exact: true }).click();
  await expect(page.getByTestId("error-boundary")).toBeVisible({
    timeout: 20_000,
  });

  // A marker that only survives if the document is never reloaded — a reload
  // would sign the user out (MemoryStore sessions), which is worse than the
  // error it recovers from.
  await page.evaluate(() => {
    (window as unknown as { __plenumNoReload?: boolean }).__plenumNoReload = true;
  });

  await page.getByRole("button", { name: "Try again" }).click();

  await page.waitForSelector('body[data-screen-ready="true"]', {
    timeout: 30_000,
  });
  await expect(page.getByTestId("error-boundary")).toHaveCount(0);
  expect(page.url()).toContain("/leakage");
  expect(
    await page.evaluate(
      () =>
        (window as unknown as { __plenumNoReload?: boolean }).__plenumNoReload ===
        true,
    ),
  ).toBe(true);
  expect(attempts).toBeGreaterThan(1);
});

test("D-3: when the in-page retry cannot work, the panel escalates instead of lying", async ({
  page,
}) => {
  await loginAs(page, VP);

  // Nothing about Leakage ever downloads — the case where the screen chunk's
  // shared dependency is poisoned too and no in-document re-import can win.
  await page.route(chunkOf("Leakage"), (r) => r.abort("failed"));
  await page.getByRole("link", { name: "Leakage", exact: true }).click();

  const panel = page.getByTestId("error-boundary");
  await expect(panel).toBeVisible({ timeout: 20_000 });
  await panel.getByRole("button", { name: "Try again" }).click();

  // Second failure: the button becomes the one thing that CAN work, and the
  // sentence says what it will do rather than asking for the same press again.
  await expect(
    panel.getByRole("button", { name: "Reload PLENUM" }),
  ).toBeVisible({ timeout: 20_000 });
  const line = (await panel.getByTestId("error-message").textContent()) ?? "";
  expect(line).toContain("stay signed in");
  await shellIsAlive(page);
});

test("D-3: on a healthy network all four lazy routes still load", async ({
  page,
}) => {
  await loginAs(page, VP);
  for (const route of LAZY) {
    await page.getByRole("link", { name: route.nav, exact: true }).click();
    await page.waitForURL(`**${route.path}`, { timeout: 20_000 });
    await page.waitForSelector('body[data-screen-ready="true"]', {
      timeout: 30_000,
    });
    await expect(page.getByTestId("error-boundary")).toHaveCount(0);
  }
});

// ── D-4: route identity ────────────────────────────────────────────────────
// Every ORDERED pair of the four lazy routes — 12 transitions, each one a
// navigation from an already-mounted lazy screen, which is the case none of
// the D-3 specs exercised.

test("D-4: every ordered pair of lazy routes lands on the destination screen", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await loginAs(page, VP);

  const results: string[] = [];
  const wrong: string[] = [];

  for (const from of LAZY) {
    for (const to of LAZY) {
      if (from.screen === to.screen) continue;

      // Reach `from` from a known-good starting point, so each pair is
      // measured independently and one failure cannot mask the next.
      await clickNav(page, "Command");
      await clickNav(page, from.nav);
      const start = await mountedScreen(page);

      await clickNav(page, to.nav);
      const landed = await mountedScreen(page);
      const url = new URL(page.url()).pathname;

      const ok = landed === to.screen && url === to.path;
      results.push(
        `${ok ? "PASS" : "FAIL"}  ${from.screen} -> ${to.screen}  ::  ` +
          `url ${url}  ::  mounted ${landed}` +
          (ok ? "" : `   <-- still showing ${start}`),
      );
      if (!ok) wrong.push(`${from.screen} -> ${to.screen} showed ${landed}`);
    }
  }

  console.log("\nD-4 lazy -> lazy transitions (12):\n" + results.join("\n"));
  expect(wrong, `wrong screen on ${wrong.length} of 12 transitions`).toEqual([]);
});

test("D-4: a lazy route reached via an eager route in between still lands right", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await loginAs(page, VP);

  const results: string[] = [];
  const wrong: string[] = [];

  for (const lazy of LAZY) {
    for (const eager of EAGER.slice(0, 2)) {
      // lazy -> eager -> lazy, both directions through the eager screen.
      await clickNav(page, lazy.nav);
      await clickNav(page, eager.nav);
      const middle = await mountedScreen(page);
      await clickNav(page, lazy.nav);
      const back = await mountedScreen(page);

      const ok = middle === eager.screen && back === lazy.screen;
      results.push(
        `${ok ? "PASS" : "FAIL"}  ${lazy.screen} -> ${eager.screen} -> ` +
          `${lazy.screen}  ::  got ${middle} then ${back}`,
      );
      if (!ok) wrong.push(`${lazy.screen}/${eager.screen}: ${middle},${back}`);
    }
  }

  console.log("\nD-4 lazy -> eager -> lazy:\n" + results.join("\n"));
  expect(wrong).toEqual([]);
});

test("D-4: one route's spent retry state does not follow you to the next route", async ({
  page,
}) => {
  await loginAs(page, VP);

  // Burn Leakage's retry all the way to the second rung, so its LazyRoute
  // state is as dirty as it can get.
  await page.route(chunkOf("Leakage"), (r) => r.abort("failed"));
  await clickNav(page, "Leakage");
  const panel = page.getByTestId("error-boundary");
  await expect(panel).toBeVisible({ timeout: 20_000 });
  await panel.getByRole("button", { name: "Try again" }).click();
  await expect(
    panel.getByRole("button", { name: "Reload PLENUM" }),
  ).toBeVisible({ timeout: 20_000 });

  // Now leave. The shared instance used to carry `attempt`/`bust` across —
  // meaning the Map would have inherited Leakage's spent retry and a bust URL
  // pointing at Leakage's chunk.
  await clickNav(page, "Territory Map");
  expect(await mountedScreen(page)).toBe("map");
  await expect(page.getByTestId("error-boundary")).toHaveCount(0);

  // And coming back gives Leakage a FRESH retry budget. Its chunk is still
  // unreachable — and stays unreachable for the life of this document even if
  // the network returns, because the module map remembers the failed URL
  // (D-3's measured law) — so a panel is correct here. What proves no state
  // leaked is that the button is "Try again" again, not the spent
  // "Reload PLENUM" the shared instance would have carried over.
  await clickNav(page, "Leakage");
  const back = page.getByTestId("error-boundary");
  await expect(back).toBeVisible({ timeout: 20_000 });
  await expect(back.getByRole("button", { name: "Try again" })).toBeVisible();
});

test("D-4: all nine nav destinations, top to bottom and back up", async ({
  page,
}) => {
  test.setTimeout(180_000);
  await loginAs(page, VP);

  const walk = [...NAV_ORDER, ...[...NAV_ORDER].reverse()];
  const results: string[] = [];
  const wrong: string[] = [];

  for (const stop of walk) {
    await clickNav(page, stop.nav);
    const landed = await mountedScreen(page);
    const ok = landed === stop.screen;
    results.push(`${ok ? "PASS" : "FAIL"}  clicked ${stop.nav} :: ${landed}`);
    if (!ok) wrong.push(`${stop.nav} showed ${landed}`);
  }

  console.log("\nD-4 full nav walk (9 down, 9 up):\n" + results.join("\n"));
  expect(wrong).toEqual([]);
});
