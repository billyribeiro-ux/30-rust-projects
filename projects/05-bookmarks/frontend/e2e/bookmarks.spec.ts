import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniqueTitle(): string {
  return `E2E ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('add a bookmark, see it in the grid, delete it', async ({ page }) => {
  await gotoHydrated(page, '/');

  await page.getByRole('button', { name: /add bookmark/i }).first().click();
  const panel = page.getByRole('region', { name: /add bookmark/i });
  await expect(panel).toBeVisible();

  const title = uniqueTitle();
  await panel.getByLabel(/^URL/).fill('https://example.com');
  await panel.getByLabel(/^Title/).fill(title);
  await panel.getByLabel(/^Description/).fill('Demo bookmark');
  await panel.getByLabel(/^Tags/).fill('demo rust');
  await panel.getByRole('button', { name: /save bookmark/i }).click();
  await page.waitForLoadState('networkidle');

  const card = page.getByRole('article').filter({ hasText: title });
  await expect(card).toBeVisible();
  await expect(card).toContainText('Demo bookmark');
  await expect(card.getByRole('button', { name: /filter by tag rust/i })).toBeVisible();

  page.once('dialog', (d) => d.accept());
  await card.getByRole('button', { name: new RegExp(`delete bookmark "${title}"`, 'i') }).click();
  await page.waitForLoadState('networkidle');
  await expect(page.getByRole('article').filter({ hasText: title })).toHaveCount(0);
});

test('search updates the URL after debounce', async ({ page, request }) => {
  const title = uniqueTitle();
  await request.post('http://localhost:3004/api/bookmarks', {
    data: {
      url: 'https://search.example.com',
      title,
      description: 'searchable description',
      tags: ['search']
    }
  });

  await gotoHydrated(page, '/');
  await page.getByRole('searchbox', { name: /search bookmarks/i }).fill(title);

  // Debounce is 200ms; give it a bit more.
  await page.waitForTimeout(350);
  await page.waitForLoadState('networkidle');

  await expect(page).toHaveURL(new RegExp(`q=${encodeURIComponent(title).replace(/%20/g, '\\+')}`));
  await expect(page.getByRole('article').filter({ hasText: title })).toBeVisible();
});

test('clicking a sidebar tag filters and adds ?tag= to URL', async ({ page, request }) => {
  const title = uniqueTitle();
  const tagName = `e2etag${Date.now()}`;
  await request.post('http://localhost:3004/api/bookmarks', {
    data: {
      url: 'https://tag.example.com',
      title,
      description: '',
      tags: [tagName]
    }
  });

  await gotoHydrated(page, '/');

  // Click the tag in the sidebar.
  await page.getByRole('button', { name: new RegExp(`^${tagName} `) }).first().click();
  await page.waitForLoadState('networkidle');

  await expect(page).toHaveURL(new RegExp(`tag=${tagName}`));
  // The filtered grid should show the bookmark we just created.
  await expect(page.getByRole('article').filter({ hasText: title })).toBeVisible();
});

test('clickOutside closes the Add panel', async ({ page }) => {
  await gotoHydrated(page, '/');
  await page.getByRole('button', { name: /add bookmark/i }).first().click();
  await expect(page.getByRole('region', { name: /add bookmark/i })).toBeVisible();

  // Click on the page brand area — outside the panel.
  await page.getByRole('heading', { level: 1, name: /bookmarks/i }).click();
  await expect(page.getByRole('region', { name: /add bookmark/i })).not.toBeVisible();
});
