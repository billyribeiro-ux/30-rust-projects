import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('add a book manually, see it in the library, open detail page', async ({ page }) => {
  await gotoHydrated(page, '/');

  const title = uniq('E2E Book');
  await page.getByLabel(/^Title/i).fill(title);
  await page.getByLabel(/^Author/i).fill('A. Tester');
  await page.getByRole('button', { name: /add manually/i }).click();
  await page.waitForLoadState('networkidle');

  const card = page.getByRole('article').filter({ hasText: title });
  await expect(card).toBeVisible();

  // Click into the detail page
  await card.getByRole('heading', { name: title }).click();
  await page.waitForLoadState('networkidle');

  await expect(page.getByRole('heading', { level: 1, name: title })).toBeVisible();
  await expect(page.getByText(/by A\. Tester/)).toBeVisible();
});

test('status tab change persists across reload', async ({ page, request }) => {
  // Seed a book via API for a stable starting point.
  const title = uniq('Tab Persist');
  const created = await request
    .post('http://localhost:3007/api/books', {
      data: { title, author: 'API', status: 'want_to_read' }
    })
    .then((r) => r.json());

  await gotoHydrated(page, `/books/${created.id}`);

  await page.getByRole('tab', { name: /^Reading$/ }).click();
  await page.waitForLoadState('networkidle');
  await expect(page.getByRole('tab', { name: /^Reading$/ })).toHaveAttribute(
    'aria-selected',
    'true'
  );

  // Reload and verify it stuck
  await gotoHydrated(page, `/books/${created.id}`);
  await expect(page.getByRole('tab', { name: /^Reading$/ })).toHaveAttribute(
    'aria-selected',
    'true'
  );
});

test('streamed stats render after the book list', async ({ page, request }) => {
  // Seed a couple books so stats are non-zero
  await request.post('http://localhost:3007/api/books', {
    data: { title: uniq('A'), status: 'reading' }
  });
  await request.post('http://localhost:3007/api/books', {
    data: { title: uniq('B'), status: 'finished' }
  });

  await gotoHydrated(page, '/');

  // The stats panel exists with role="region" via aria-label
  const stats = page.locator('[aria-label="Stats"]');
  await expect(stats).toBeVisible();
  await expect(stats).toContainText('Books');
  await expect(stats).toContainText('Reading');
  await expect(stats).toContainText('Finished');
});

test('unknown book id returns the custom 404 page', async ({ page }) => {
  const res = await page.goto('/books/does-not-exist');
  expect(res?.status()).toBe(404);
  await expect(page.getByRole('heading', { name: /not found/i })).toBeVisible();
});
