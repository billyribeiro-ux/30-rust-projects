import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

// Each parallel test gets a unique habit name so the shared DB doesn't
// collide across viewports / specs running concurrently.
function uniqueName(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('create habit, toggle today, see current streak = 1, delete', async ({ page }) => {
  await gotoHydrated(page, '/');

  const habitName = uniqueName('Read');
  await page.getByLabel(/new habit name/i).fill(habitName);
  await page.getByRole('button', { name: /add habit/i }).click();
  await page.waitForLoadState('networkidle');

  const row = page.getByRole('article').filter({ hasText: habitName });
  await expect(row).toBeVisible();
  await expect(row.getByRole('heading', { name: habitName })).toBeVisible();

  // Toggle today (the last grid cell — tabindex=0 by default)
  await page.waitForLoadState('networkidle');
  const todayCell = row.locator('button[aria-pressed]').last();
  await todayCell.click();
  await page.waitForLoadState('networkidle');

  // Current streak should now be 1 — the first <dd> inside the stats list
  await expect(row.locator('.stats dd').first()).toHaveText('1');

  // Delete
  page.once('dialog', (d) => d.accept());
  await row.getByRole('button', { name: new RegExp(`delete habit "${habitName}"`, 'i') }).click();
  await page.waitForLoadState('networkidle');
  await expect(page.getByRole('article').filter({ hasText: habitName })).toHaveCount(0);
});

test('validation: empty habit name disables Add', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: /add habit/i })).toBeDisabled();
});

test('keyboard: arrow keys move focus across calendar grid', async ({ page }) => {
  await gotoHydrated(page, '/');

  const habitName = uniqueName('Stretch');
  await page.getByLabel(/new habit name/i).fill(habitName);
  await page.getByRole('button', { name: /add habit/i }).click();
  await page.waitForLoadState('networkidle');

  const row = page.getByRole('article').filter({ hasText: habitName });
  await expect(row).toBeVisible();
  const cells = row.locator('button[aria-pressed]');
  await expect(cells).toHaveCount(49);
  const count = 49;

  const last = cells.nth(count - 1);
  await last.focus();
  await expect(last).toBeFocused();

  await page.keyboard.press('ArrowUp');
  await expect(cells.nth(count - 2)).toBeFocused();

  await page.keyboard.press('ArrowLeft');
  await expect(cells.nth(count - 2 - 7)).toBeFocused();

  await page.keyboard.press('Home');
  await expect(cells.first()).toBeFocused();

  await page.keyboard.press('End');
  await expect(cells.last()).toBeFocused();

  page.once('dialog', (d) => d.accept());
  await row.getByRole('button', { name: new RegExp(`delete habit "${habitName}"`, 'i') }).click();
});

test('respects prefers-reduced-motion (no cell transition)', async ({ browser }) => {
  const context = await browser.newContext({ reducedMotion: 'reduce' });
  const page = await context.newPage();
  await gotoHydrated(page, '/');

  const habitName = uniqueName('Meditate');
  await page.getByLabel(/new habit name/i).fill(habitName);
  await page.getByRole('button', { name: /add habit/i }).click();
  await page.waitForLoadState('networkidle');

  const row = page.getByRole('article').filter({ hasText: habitName });
  const cell = row.locator('button[aria-pressed]').last();
  const transition = await cell.evaluate((el) => getComputedStyle(el).transitionDuration);
  expect(transition).toBe('0s');

  page.once('dialog', (d) => d.accept());
  await row.getByRole('button', { name: new RegExp(`delete habit "${habitName}"`, 'i') }).click();
  await context.close();
});
