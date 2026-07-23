// Responsive tripwire + rep-scope assertions — gate P2-2's automated half,
// extended in P3 (Pipeline, Quotes, Account 360, quote detail), P4 (Signals,
// Ask), and P5 (Territory Map, Leakage, Data Quality).
//
// Layout: every screen at every supported width must render no wider than its
// viewport (spec §8: document.documentElement.scrollWidth <= window.innerWidth
// — a page-level horizontal scrollbar is a build failure).
//   14 screens × 5 widths = 70 layout observables.
//   (12 static: login, command, map, leaderboards ×3, leakage, pipeline,
//    quotes, signals, ask, data-quality; 2 dynamic: /accounts/:id and
//    /quotes/:id, ids resolved in-test.)
//
// Scope: FIVE assertions on the surface —
//   · a rep (SE-1) sees EXACTLY ONE territory tile on Command, and it is SE-1;
//   · a rep's Pipeline shows exactly the opportunities the API returns for them,
//     and every one is SE-1;
//   · P4: a rep's Signals queue renders exactly the API's active rows for
//     them (every lane expanded first — R6 pagination), all SE-1;
//   · P5 leakage-scope: the rep's heat table contains her rows and ONLY her
//     rows — no foreign rep name anywhere in the grid;
//   · P5 map-scope: the rep's map DOM contains NO dollar value inside any
//     element attributed to a foreign territory (fills are config; money is
//     scoped).
//
// Requires the API on 127.0.0.1:5777 (this dev server only proxies to it).

import { test, expect, type BrowserContext, type Page } from "@playwright/test";

const PASSWORD = "demo-plenum-2026";

const VIEWPORTS = [
  { name: "375", width: 375, height: 812 },
  { name: "768x1024", width: 768, height: 1024 },
  { name: "1024x768", width: 1024, height: 768 },
  { name: "1440x900", width: 1440, height: 900 },
  { name: "2560x1440", width: 2560, height: 1440 },
] as const;

const STATIC_SCREENS = [
  { name: "login", path: "/login", auth: false },
  { name: "command", path: "/command", auth: true },
  { name: "map", path: "/map", auth: true },
  { name: "leaderboards-reps", path: "/leaderboards?tab=reps", auth: true },
  { name: "leaderboards-items", path: "/leaderboards?tab=items", auth: true },
  { name: "leaderboards-customers", path: "/leaderboards?tab=customers", auth: true },
  { name: "leakage", path: "/leakage", auth: true },
  { name: "pipeline", path: "/pipeline", auth: true },
  { name: "quotes", path: "/quotes", auth: true },
  { name: "signals", path: "/signals", auth: true },
  { name: "ask", path: "/ask", auth: true },
  { name: "data-quality", path: "/data-quality", auth: true },
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

async function overflowPx(page: Page): Promise<number> {
  return page.evaluate(
    () => document.documentElement.scrollWidth - window.innerWidth,
  );
}

test("tripwire: responsive layout (70) + rep scope (command + pipeline + signals + leakage + map)", async ({
  browser,
}) => {
  const authed: BrowserContext = await browser.newContext();
  const anon: BrowserContext = await browser.newContext();

  // Establish the VP session once, and resolve real ids for the dynamic screens
  // through the API (rep-scope style — the VP sees everything, so any id works).
  const vpPage = await authed.newPage();
  await vpPage.setViewportSize({ width: 1440, height: 900 });
  await loginAs(vpPage, "valerie.price@plenum.demo");
  const ids = await vpPage.evaluate(async () => {
    const accounts = await (await fetch("/api/accounts?limit=1")).json();
    const quotes = await (
      await fetch("/api/quotes?view=approvals&limit=1")
    ).json();
    return {
      accountId: accounts.items[0]?.id ?? null,
      quoteId: quotes.items[0]?.id ?? null,
    };
  });
  await vpPage.close();

  const screens = [
    ...STATIC_SCREENS,
    ...(ids.accountId
      ? [{ name: "account-360", path: `/accounts/${ids.accountId}`, auth: true }]
      : []),
    ...(ids.quoteId
      ? [{ name: "quote-detail", path: `/quotes/${ids.quoteId}`, auth: true }]
      : []),
  ];
  const expected = screens.length * VIEWPORTS.length;

  const failures: string[] = [];
  let pass = 0;

  for (const screen of screens) {
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

  // ── rep-scope, on two surfaces ──────────────────────────────────────────────
  const repPage = await anon.newPage();
  await repPage.setViewportSize({ width: 1440, height: 900 });
  await loginAs(repPage, "serena.estes@plenum.demo");

  // 1 · Command — exactly one tile, SE-1.
  await repPage.goto("/command");
  await waitReady(repPage);
  const tiles = repPage.getByTestId("territory-tile");
  const tileCount = await tiles.count();
  const firstCode = tileCount
    ? await tiles.first().getAttribute("data-tile-code")
    : null;
  const commandScopeOk = tileCount === 1 && firstCode === "SE-1";
  console.log(
    commandScopeOk
      ? `  PASS  command-scope  (1 tile, ${firstCode})`
      : `  FAIL  command-scope  (${tileCount} tiles, first=${firstCode})`,
  );

  // 2 · Pipeline — the cards on screen ARE exactly the rep's RLS-scoped opps,
  // and every one is SE-1.
  const apiOpps = await repPage.evaluate(
    async () =>
      (await (await fetch("/api/opportunities?limit=200")).json()).items as {
        territory_code: string;
      }[],
  );
  const allSe1 = apiOpps.every((o) => o.territory_code === "SE-1");
  await repPage.goto("/pipeline");
  await waitReady(repPage);
  const cardCount = await repPage.getByTestId("pipeline-card").count();
  const pipelineScopeOk = allSe1 && cardCount === apiOpps.length;
  console.log(
    pipelineScopeOk
      ? `  PASS  pipeline-scope  (${cardCount} cards, all SE-1)`
      : `  FAIL  pipeline-scope  (${cardCount} cards vs ${apiOpps.length} api, allSe1=${allSe1})`,
  );

  // 3 · P4 Signals — the queue renders exactly the rep's RLS-scoped active
  // rows, and every one is SE-1 (no foreign territory code on any card).
  // P5 (R6): lanes paginate at 25, so expand every lane first — which also
  // exercises the Show-more control itself.
  const apiSignals = await repPage.evaluate(
    async () =>
      (await (await fetch("/api/signals?status=active&limit=200")).json())
        .items as { territory_code: string }[],
  );
  const signalsAllSe1 = apiSignals.every((s) => s.territory_code === "SE-1");
  await repPage.goto("/signals");
  await waitReady(repPage);
  for (;;) {
    const more = repPage.getByTestId("lane-more");
    if ((await more.count()) === 0) break;
    await more.first().click();
  }
  const signalCardCount = await repPage.getByTestId("signal-card").count();
  const signalsScopeOk = signalsAllSe1 && signalCardCount === apiSignals.length;
  console.log(
    signalsScopeOk
      ? `  PASS  signals-scope  (${signalCardCount} cards, all SE-1)`
      : `  FAIL  signals-scope  (${signalCardCount} cards vs ${apiSignals.length} api, allSe1=${signalsAllSe1})`,
  );

  // 4 · P5 Leakage — the rep's heat table holds her rows and ONLY her rows.
  const apiHeatReps = await repPage.evaluate(async () => {
    const page = await (
      await fetch("/api/metrics/leakage?period=cumulative&limit=200")
    ).json();
    return [
      ...new Set((page.heat as { rep_name: string }[]).map((c) => c.rep_name)),
    ].sort();
  });
  await repPage.goto("/leakage");
  await waitReady(repPage);
  const domHeatReps = [
    ...new Set(
      await repPage
        .getByTestId("heat-rep")
        .evaluateAll((els) => els.map((e) => (e as HTMLElement).dataset.rep)),
    ),
  ].sort();
  const leakageScopeOk =
    domHeatReps.length > 0 &&
    JSON.stringify(domHeatReps) === JSON.stringify(apiHeatReps) &&
    domHeatReps.every((r) => r === "Serena Estes");
  console.log(
    leakageScopeOk
      ? `  PASS  leakage-scope  (heat reps = [${domHeatReps.join(", ")}])`
      : `  FAIL  leakage-scope  (dom=[${domHeatReps.join(", ")}] api=[${apiHeatReps.join(", ")}])`,
  );

  // 5 · P5 Territory Map — every element attributed to a FOREIGN territory
  // must carry no dollar value; the map itself must be fully drawn (shape is
  // config, money is scoped).
  const myCodes = await repPage.evaluate(
    async () =>
      (await (await fetch("/api/auth/me")).json()).territories as string[],
  );
  await repPage.goto("/map");
  await waitReady(repPage);
  const mapAudit = await repPage.evaluate((scope: string[]) => {
    const own = new Set(scope);
    const nodes = Array.from(
      document.querySelectorAll<HTMLElement>("[data-territory]"),
    );
    const foreignWithDollars = nodes
      .filter((n) => {
        const t = n.dataset.territory ?? "";
        return t !== "" && !own.has(t);
      })
      .filter((n) => (n.textContent ?? "").includes("$"))
      .map((n) => `${n.tagName}[${n.dataset.territory}]`);
    const statesDrawn = document.querySelectorAll("[data-state]").length;
    return { foreignWithDollars, statesDrawn };
  }, myCodes);
  const mapScopeOk =
    mapAudit.foreignWithDollars.length === 0 && mapAudit.statesDrawn >= 53;
  console.log(
    mapScopeOk
      ? `  PASS  map-scope  (${mapAudit.statesDrawn} shapes drawn, 0 foreign $)`
      : `  FAIL  map-scope  (foreign $ in: ${mapAudit.foreignWithDollars.join(", ")}; shapes=${mapAudit.statesDrawn})`,
  );
  await repPage.close();

  console.log(
    `\nTRIPWIRE ${pass}/${expected} layout ${failures.length === 0 ? "PASS" : "FAIL"} · ` +
      `command-scope ${commandScopeOk ? "PASS" : "FAIL"} · ` +
      `pipeline-scope ${pipelineScopeOk ? "PASS" : "FAIL"} · ` +
      `signals-scope ${signalsScopeOk ? "PASS" : "FAIL"} · ` +
      `leakage-scope ${leakageScopeOk ? "PASS" : "FAIL"} · ` +
      `map-scope ${mapScopeOk ? "PASS" : "FAIL"}` +
      (failures.length ? `\n  failures: ${failures.join(", ")}` : ""),
  );

  await authed.close();
  await anon.close();

  expect(failures, `layout overflow: ${failures.join("; ")}`).toHaveLength(0);
  expect(commandScopeOk, "rep must see exactly one tile, SE-1").toBe(true);
  expect(pipelineScopeOk, "rep pipeline must show only their SE-1 opps").toBe(
    true,
  );
  expect(signalsScopeOk, "rep signals queue must show only SE-1 cards").toBe(
    true,
  );
  expect(leakageScopeOk, "rep heat table must hold only her own rows").toBe(
    true,
  );
  expect(
    mapScopeOk,
    "rep map must carry no foreign-territory dollar values",
  ).toBe(true);
});
