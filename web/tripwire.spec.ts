// Responsive tripwire + rep-scope assertion — gate P2-2's automated half.
//
// Layout: every screen at every supported width must render no wider than its
// viewport (spec §8: document.documentElement.scrollWidth <= window.innerWidth
// — a page-level horizontal scrollbar is a build failure). 5 widths × 5 screens
// = 25 observables.
//
// Scope: a rep (SE-1) at 1440 sees EXACTLY ONE territory tile, and it is SE-1
// — the RLS boundary, asserted on the surface.
//
// Requires the API on 127.0.0.1:5777 (this dev server only proxies to it). If
// login never reaches Command, the message says exactly that.

import { test, expect, type BrowserContext, type Page } from "@playwright/test";

const PASSWORD = "demo-plenum-2026";

const VIEWPORTS = [
  { name: "375", width: 375, height: 812 },
  { name: "768x1024", width: 768, height: 1024 },
  { name: "1024x768", width: 1024, height: 768 },
  { name: "1440x900", width: 1440, height: 900 },
  { name: "2560x1440", width: 2560, height: 1440 },
] as const;

const SCREENS = [
  { name: "login", path: "/login", auth: false },
  { name: "command", path: "/command", auth: true },
  { name: "leaderboards-reps", path: "/leaderboards?tab=reps", auth: true },
  { name: "leaderboards-items", path: "/leaderboards?tab=items", auth: true },
  { name: "leaderboards-customers", path: "/leaderboards?tab=customers", auth: true },
] as const;

async function loginAs(page: Page, email: string): Promise<void> {
  await page.goto("/login");
  await page.getByTestId("login-email").fill(email);
  await page.getByTestId("login-password").fill(PASSWORD);
  await page.getByTestId("login-submit").click();
  try {
    await page.waitForURL("**/command", { timeout: 15_000 });
    await page.waitForSelector('body[data-screen-ready="true"]', {
      timeout: 15_000,
    });
  } catch (e) {
    throw new Error(
      `Login as ${email} never reached Command. The API on 127.0.0.1:5777 is ` +
        `probably not running — start it with: cargo run --bin api  (${String(e)})`,
    );
  }
}

async function waitReady(page: Page): Promise<void> {
  await page.waitForSelector('body[data-screen-ready="true"]', {
    timeout: 20_000,
  });
}

/** scrollWidth <= innerWidth, per the spec formula. Returns the overflow px. */
async function overflowPx(page: Page): Promise<number> {
  return page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
}

test("tripwire: responsive layout (25) + rep scope", async ({ browser }) => {
  const authed: BrowserContext = await browser.newContext();
  const anon: BrowserContext = await browser.newContext();

  // Establish the VP session once, in the authed context.
  const vpPage = await authed.newPage();
  await vpPage.setViewportSize({ width: 1440, height: 900 });
  await loginAs(vpPage, "valerie.price@plenum.demo");
  await vpPage.close();

  const failures: string[] = [];
  let pass = 0;

  for (const screen of SCREENS) {
    const ctx = screen.auth ? authed : anon;
    for (const vp of VIEWPORTS) {
      const page = await ctx.newPage();
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await page.goto(screen.path);
      await waitReady(page);

      const overflow = await overflowPx(page);
      const tag = `${screen.name}@${vp.name}`;
      if (overflow <= 0) {
        pass++;
        console.log(`  PASS  ${tag}  (scrollWidth−innerWidth = ${overflow})`);
      } else {
        failures.push(`${tag} overflow=${overflow}px`);
        console.log(`  FAIL  ${tag}  overflow=${overflow}px`);
      }
      await page.close();
    }
  }

  // Rep-scope assertion — fresh session, no shared state with the VP context.
  const repPage = await anon.newPage();
  await repPage.setViewportSize({ width: 1440, height: 900 });
  await loginAs(repPage, "serena.estes@plenum.demo");
  await repPage.goto("/command");
  await waitReady(repPage);
  const tiles = repPage.getByTestId("territory-tile");
  const tileCount = await tiles.count();
  const firstCode = tileCount
    ? await tiles.first().getAttribute("data-tile-code")
    : null;
  const scopeOk = tileCount === 1 && firstCode === "SE-1";
  console.log(
    scopeOk
      ? `  PASS  rep-scope  (1 tile, ${firstCode})`
      : `  FAIL  rep-scope  (${tileCount} tiles, first=${firstCode})`,
  );
  await repPage.close();

  console.log(
    `\nTRIPWIRE ${pass}/25 layout ${failures.length === 0 ? "PASS" : "FAIL"} · ` +
      `rep-scope ${scopeOk ? "PASS" : "FAIL"}` +
      (failures.length ? `\n  failures: ${failures.join(", ")}` : ""),
  );

  await authed.close();
  await anon.close();

  expect(failures, `layout overflow: ${failures.join("; ")}`).toHaveLength(0);
  expect(scopeOk, "rep must see exactly one tile, SE-1").toBe(true);
});
