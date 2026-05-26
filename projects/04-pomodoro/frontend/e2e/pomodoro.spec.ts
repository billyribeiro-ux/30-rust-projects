import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('initial state: 25:00 work timer, paused', async ({ page }) => {
  await gotoHydrated(page, '/');
  await expect(page.getByTestId('clock')).toHaveText('25:00');
  await expect(page.getByRole('tab', { name: /focus/i })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('button', { name: /start timer/i })).toBeVisible();
});

test('Space toggles start/pause; the label flips', async ({ page }) => {
  await gotoHydrated(page, '/');

  // Don't focus the label input — pressing Space inside an input would type.
  await page.locator('main').click({ position: { x: 5, y: 5 } });
  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: /pause timer/i })).toBeVisible();

  await page.keyboard.press('Space');
  await expect(page.getByRole('button', { name: /start timer/i })).toBeVisible();
});

test('mode keys 1/2/3 switch the active tab and reset the clock', async ({ page }) => {
  await gotoHydrated(page, '/');
  await page.locator('main').click({ position: { x: 5, y: 5 } });

  await page.keyboard.press('2');
  await expect(page.getByRole('tab', { name: /short break/i })).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await expect(page.getByTestId('clock')).toHaveText('05:00');

  await page.keyboard.press('3');
  await expect(page.getByRole('tab', { name: /long break/i })).toHaveAttribute(
    'aria-selected',
    'true'
  );
  await expect(page.getByTestId('clock')).toHaveText('15:00');

  await page.keyboard.press('1');
  await expect(page.getByRole('tab', { name: /focus/i })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByTestId('clock')).toHaveText('25:00');
});

test('recorded session shows in history; delete removes it', async ({ page, request }) => {
  // Seed a session directly via the API (the live timer would take 25 min).
  const startedAt = new Date(Date.now() - 25 * 60 * 1000).toISOString();
  const endedAt = new Date().toISOString();
  const label = `E2E ${Date.now()}`;
  const res = await request.post('http://localhost:3003/api/sessions', {
    data: {
      kind: 'work',
      label,
      planned_seconds: 1500,
      actual_seconds: 1500,
      started_at: startedAt,
      ended_at: endedAt
    }
  });
  expect(res.status()).toBe(201);

  await gotoHydrated(page, '/');

  const row = page.locator('li').filter({ hasText: label });
  await expect(row).toBeVisible();
  await expect(row).toContainText('Focus');
  await expect(row).toContainText('25 min');

  await row.getByRole('button', { name: /delete focus session/i }).click();
  await page.waitForLoadState('networkidle');
  await expect(page.locator('li').filter({ hasText: label })).toHaveCount(0);
});

test('respects prefers-reduced-motion (ring transition disabled)', async ({ browser }) => {
  const context = await browser.newContext({ reducedMotion: 'reduce' });
  const page = await context.newPage();
  await gotoHydrated(page, '/');

  const ring = page.locator('.dial .ring').first();
  const duration = await ring.evaluate((el) => getComputedStyle(el).transitionDuration);
  expect(duration).toBe('0s');

  await context.close();
});
