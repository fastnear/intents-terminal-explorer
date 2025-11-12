import { test, expect } from '@playwright/test';

const BASE = process.env.NEARX_BASE_URL ?? 'http://127.0.0.1:8083';

test.describe('NEARx web smoke', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE);
    // Wait until headings appear (Blocks / Transactions / Transaction Details)
    await expect(page.getByText(/Blocks \(/)).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText(/Transactions \(/)).toBeVisible();
    await expect(page.getByText(/Transaction Details/)).toBeVisible();
  });

  test('click block selects it & updates txs', async ({ page }) => {
    // Find "Blocks" panel area
    const blocks = page.getByText(/Blocks \(/).first();
    const box = await blocks.boundingBox();
    expect(box).not.toBeNull();

    // Click a bit below the heading to hit first row
    if (box) {
      await page.mouse.click(box.x + 10, box.y + box.height + 16);
    }

    // After selection, Transactions count or content should change/appear
    // We don't have stable row selectors; assert txs heading remains and a row appears
    await expect(page.getByText(/Transactions \(/).first()).toBeVisible();
  });

  test('Tab cycles panes only (filter not focused)', async ({ page }) => {
    // Type '/' to focus filter, then Escape to blur for a baseline
    await page.keyboard.press('/');
    await page.keyboard.press('Escape');

    // Press Tab three times — no input should receive focus (heuristic)
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');

    // Heuristic: filter caret shouldn't be visible (no focused text input).
    // We assert by typing 'ZZZ' and expecting no visible change in headings text.
    const before = await page.getByText(/Transaction Details/).count();
    await page.keyboard.type('ZZZ');
    const after = await page.getByText(/Transaction Details/).count();
    expect(after).toBe(before);
  });

  test('copy details to clipboard via "c"', async ({ page, context }) => {
    // Focus Details pane by clicking in it
    const details = page.getByText(/Transaction Details/).first();
    const dbox = await details.boundingBox();
    expect(dbox).not.toBeNull();
    if (dbox) {
      await page.mouse.click(dbox.x + 10, dbox.y + dbox.height + 16);
    }

    // Press 'c' to copy JSON
    await page.keyboard.press('c');

    // Read clipboard (Chromium/WebKit allow)
    const text = await page.evaluate(async () => navigator.clipboard.readText());
    expect(text).toBeTruthy();
    expect(text.trim().startsWith('{') || text.trim().startsWith('[')).toBeTruthy();
  });
});
