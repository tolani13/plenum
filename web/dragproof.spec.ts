// T1 FIX — trusted-event drag proof. Synthetic-event proof was ruled
// INADMISSIBLE for the drag path after acceptance check 4 failed under
// D.'s real mouse (native text selection captured the gesture; the drag
// never armed). This spec drives a REAL mouse through Playwright's
// trusted CDP input: mouse.down() on Alabama's shape, incremental
// mouse.move() to the NE-1 editor row, mouse.up() — asserting the state
// actually re-homed, the ghost chip appeared mid-drag, and NO text got
// selected. It then restores AL to SE-1 via trusted CLICK-to-paint, which
// simultaneously proves the fix left click-to-paint intact (the
// byte-identical constraint) and leaves geography canonical.
//
// Requires the API on 127.0.0.1:5777, same as the tripwire.

import { test, expect } from "@playwright/test";

const PASSWORD = "demo-plenum-2026";

test("T1 drag: trusted mouse drag of AL onto the NE-1 editor row repaints it, ghost shown, zero text selection; trusted click-to-paint restores it", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto("/login");
  await page.getByTestId("login-email").fill("valerie.price@plenum.demo");
  await page.getByTestId("login-password").fill(PASSWORD);
  await page.getByTestId("login-submit").click();
  await page.waitForURL("**/command", { timeout: 15_000 });

  await page.goto("/map?edit=1");
  await page.waitForSelector('body[data-screen-ready="true"]', {
    timeout: 20_000,
  });
  await expect(page.getByTestId("map-editor")).toBeVisible();

  const al = page.locator('path[data-state="AL"]');
  await expect(al).toHaveAttribute("data-territory", "SE-1");

  // ── the REAL drag: AL → the NE-1 editor row ──────────────────────────────
  const alBox = await al.boundingBox();
  const row = page.locator(
    '[data-testid="editor-territory-row"][data-drop-territory="NE-1"]',
  );
  const rowBox = await row.boundingBox();
  if (!alBox || !rowBox) throw new Error("shapes not measurable");

  const sx = alBox.x + alBox.width / 2;
  const sy = alBox.y + alBox.height / 2;
  const tx = rowBox.x + rowBox.width / 2;
  const ty = rowBox.y + rowBox.height / 2;

  await page.mouse.move(sx, sy);
  await page.mouse.down();
  const steps = 18;
  for (let i = 1; i <= steps; i++) {
    await page.mouse.move(sx + ((tx - sx) * i) / steps, sy + ((ty - sy) * i) / steps);
  }
  // Mid-drag, cursor over the row: the ghost chip must exist.
  const ghostCount = await page.getByTestId("map-drag-ghost").count();
  await page.mouse.up();

  await expect(al).toHaveAttribute("data-territory", "NE-1", {
    timeout: 8_000,
  });
  expect(ghostCount, "ghost chip rides the cursor during a real drag").toBeGreaterThan(0);
  const collapsedAfterDrag = await page.evaluate(
    () => window.getSelection()?.isCollapsed ?? true,
  );
  expect(collapsedAfterDrag, "a real drag must not select text").toBe(true);

  // ── trusted CLICK-to-paint restores canon (and proves paint intact) ──────
  await page
    .locator('[data-testid="editor-select-territory"][data-territory="SE-1"]')
    .click();
  await page.mouse.click(sx, sy); // a real click on the AL shape
  await expect(al).toHaveAttribute("data-territory", "SE-1", {
    timeout: 8_000,
  });
  const collapsedAfterClick = await page.evaluate(
    () => window.getSelection()?.isCollapsed ?? true,
  );
  expect(collapsedAfterClick).toBe(true);
});
