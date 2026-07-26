// D-2 (2026-07-25): the screen must say something TRUE about what failed.
//
// The defect this locks down: TabPanel rendered <ErrorPanel onRetry=… /> with
// no message, so ErrorPanel's hardcoded default — "Couldn’t reach the API." —
// was shown for EVERY failure. On the live site the Items query was heavy
// enough that concurrent requests killed the free-tier Postgres backends and
// the API returned a typed 500; the screen blamed the network, and sent
// diagnosis in the wrong direction for a day.
//
// Three cases, one law: only a genuine transport failure may claim the API
// could not be reached.
//   1 the API answers with a typed error  -> name its code and message
//   2 the request never gets an answer    -> say the API was unreachable
//   3 the API answers slowly but succeeds -> never an error state at all
//
// Requires the API on 127.0.0.1:5777 (this dev server only proxies to it).

import { test, expect, type Page } from "@playwright/test";

const PASSWORD = "demo-plenum-2026";
const VP = "valerie.price@plenum.demo";

/** The sentence that used to be shown for everything. */
const CONNECTIVITY_CLAIM = "reach the API";

/** The alarm line, located the same way before and after the fix, so the
 *  red state fails on the MESSAGE and not on a missing test hook. */
function alarmLine(page: Page) {
  return page.locator('[data-testid="error-message"], .text-alarm').first();
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

test("D-2: a typed API error is reported as an API error, not a network one", async ({
  page,
}) => {
  await loginAs(page, VP);

  // The exact failure the live site produced: the API answered, with its own
  // typed envelope, after the query blew up the database.
  await page.route("**/api/metrics/items*", (route) =>
    route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({
        error: { code: "internal", message: "internal error" },
      }),
    }),
  );

  await page.goto("/leaderboards?tab=items");
  const alarm = alarmLine(page);
  await expect(alarm).toBeVisible({ timeout: 20_000 });

  const text = (await alarm.textContent()) ?? "";
  // It names what the server actually said …
  expect(text).toContain("internal error");
  expect(text).toContain("internal");
  expect(text).toContain("500");
  // … and it does NOT claim a connectivity problem, because there wasn't one.
  expect(text).not.toContain(CONNECTIVITY_CLAIM);
});

test("D-2: a genuine transport failure DOES say the API was unreachable", async ({
  page,
}) => {
  await loginAs(page, VP);

  // Nothing answers — the only case in which the connectivity sentence is true.
  await page.route("**/api/metrics/items*", (route) =>
    route.abort("connectionrefused"),
  );

  await page.goto("/leaderboards?tab=items");
  const alarm = alarmLine(page);
  await expect(alarm).toBeVisible({ timeout: 20_000 });
  expect((await alarm.textContent()) ?? "").toContain(CONNECTIVITY_CLAIM);
});

test("D-2: a slow but successful response is never an error state", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await loginAs(page, VP);

  // 20 s of latency on a 200 — slower than any budget, still not a failure.
  await page.route("**/api/metrics/items*", async (route) => {
    await new Promise((r) => setTimeout(r, 20_000));
    await route.continue();
  });

  await page.goto("/leaderboards?tab=items");
  await page.waitForSelector('body[data-screen-ready="true"]', {
    timeout: 60_000,
  });
  await expect(page.locator('[data-testid="error-message"], .text-alarm')).toHaveCount(0);
  expect(await page.locator("tbody tr").count()).toBeGreaterThan(0);
});
