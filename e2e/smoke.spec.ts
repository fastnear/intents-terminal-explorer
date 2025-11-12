import { test, expect } from '@playwright/test';

const TOP_RATIO = 0.52; // mirrors tokens.layout.top_ratio default

test('smoke: no WASM errors, keyboard + mouse basics, clipboard copy', async ({ page }) => {
  const errors: string[] = [];
  const fatals: string[] = [];

  page.on('console', (msg) => {
    const t = msg.type();
    const text = msg.text();
    if (t === 'error') errors.push(text);
    if (/RuntimeError|unreachable|panic|wasm/i.test(text)) fatals.push(text);
  });

  await page.goto('/');

  const canvas = page.locator('#nearx_canvas');
  await canvas.waitFor({ state: 'visible', timeout: 30_000 });

  await page.waitForTimeout(250);

  await test.step('keyboard: Tab cycle should not throw', async () => {
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    await page.keyboard.press('Shift+Tab');
    await page.waitForTimeout(50);
  });

  await test.step('mouse: click into Blocks/Tx/Details regions (canvas coordinates)', async () => {
    const box = await canvas.boundingBox();
    expect(box).toBeTruthy();
    if (!box) return;

    const xLeft = box.x + box.width * 0.25;
    const xRight = box.x + box.width * 0.75;
    const yTop = box.y + box.height * (TOP_RATIO * 0.40);
    const yBottom = box.y + box.height * (0.80);

    await page.mouse.click(xLeft, yTop);   // Blocks pane area
    await page.waitForTimeout(60);

    await page.mouse.click(xRight, yTop);  // Txs pane area
    await page.waitForTimeout(60);

    await page.mouse.click((box.x + box.width * 0.50), yBottom); // Details pane area
    await page.waitForTimeout(60);
  });

  await test.step('copy: press "c" to copy JSON (should not crash; clipboard readable)', async () => {
    await page.keyboard.press('c');
    const clip = await page.evaluate(async () => {
      try { return await navigator.clipboard.readText(); } catch { return '__READ_FAIL__'; }
    });
    expect(clip).toBeTruthy();
    expect(clip).not.toBe('__READ_FAIL__');
  });

  await test.step('no WASM panics printed', async () => {
    if (fatals.length) {
      console.error('FATAL console lines:', fatals);
    }
    expect(fatals.join('\n')).not.toMatch(/RuntimeError|unreachable|panic|wasm/i);
  });

  // Optional stricter mode when you know data is flowing:
  if (process.env.NEARX_E2E_REQUIRE_DATA === '1') {
    await test.step('optional: ensure clipboard JSON parses (data mode)', async () => {
      const clip = await page.evaluate(async () => {
        try { return await navigator.clipboard.readText(); } catch { return ''; }
      });
      expect(clip).toBeTruthy();
      let parsed = false;
      try {
        JSON.parse(clip);
        parsed = true;
      } catch {
        parsed = false;
      }
      expect(parsed).toBeTruthy();
    });
  }

  if (errors.length) {
    // Allow non-fatal noise, but print for visibility.
    console.warn('Non-fatal console errors observed:\n' + errors.join('\n'));
  }
});
