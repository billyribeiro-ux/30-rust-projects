import { test, expect, type Page } from '@playwright/test';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

test('add an exercise via the form', async ({ page }) => {
  await gotoHydrated(page, '/exercises');
  const name = uniq('E2E Bench');
  await page.getByLabel(/^Name/i).fill(name);
  await page.getByRole('button', { name: /^Add$/ }).click();
  await page.waitForLoadState('networkidle');
  await expect(page.getByText(name)).toBeVisible();
});

test('FTS5 search filters via /api/exercises?q=', async ({ page, request }) => {
  // Seed via API so the search has a deterministic target word in the catalog.
  const name = uniq('Overhead Press');
  await request.post('http://localhost:3008/api/exercises', {
    data: { name, muscle_group: 'shoulders' }
  });

  // Hit the API directly and assert the FTS match works.
  const res = await request.get(
    'http://localhost:3008/api/exercises?q=' + encodeURIComponent('overhead')
  );
  expect(res.ok()).toBe(true);
  const list = (await res.json()) as { name: string }[];
  expect(list.some((e) => e.name === name)).toBe(true);
});
