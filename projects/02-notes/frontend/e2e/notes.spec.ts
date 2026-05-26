import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// SvelteKit attaches event handlers during hydration. Auto-waiting on visible
// elements does not wait for hydration, so a click on a reactive button can be
// dropped if it fires before the handler is installed. `networkidle` is a
// reliable signal that the page bundle has loaded and hydration has run.
async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('about page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/about');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('full lifecycle: create, view, delete', async ({ page }) => {
  await gotoHydrated(page, '/notes/new');
  await page.getByLabel(/title/i).fill('Notes E2E sample');
  await page
    .getByLabel(/body/i)
    .fill('# A heading\n\nThis is a **bold** sample.\n\n- one\n- two');
  await page.getByRole('button', { name: /save note/i }).click();

  await expect(page).toHaveURL(/\/notes\/notes-e2e-sample(?:-\d+)?$/);
  await expect(page.getByRole('heading', { level: 1, name: 'Notes E2E sample' })).toBeVisible();
  await expect(page.getByRole('heading', { level: 1, name: 'A heading' })).toBeVisible();
  await expect(page.locator('strong', { hasText: 'bold' })).toBeVisible();

  const noteAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(noteAxe.violations, JSON.stringify(noteAxe.violations, null, 2)).toEqual([]);

  await page.waitForLoadState('networkidle');
  page.once('dialog', (d) => d.accept());
  await page.getByRole('button', { name: /delete/i }).click();
  await expect(page).toHaveURL('/');
});

test('preview tab renders markdown without saving', async ({ page }) => {
  await gotoHydrated(page, '/notes/new');
  await page.getByLabel(/title/i).fill('Preview demo');
  await page.getByLabel(/body/i).fill('## Subheading\n\nparagraph');
  await page.getByRole('tab', { name: /preview/i }).click();
  await expect(page.getByRole('heading', { level: 2, name: 'Subheading' })).toBeVisible();
});

test('validation: empty title blocks save', async ({ page }) => {
  await page.goto('/notes/new');
  const save = page.getByRole('button', { name: /save note/i });
  await expect(save).toBeDisabled();
});

test('unknown slug shows 404 page', async ({ page }) => {
  const res = await page.goto('/notes/this-slug-does-not-exist-12345');
  expect(res?.status()).toBe(404);
  await expect(page.getByRole('heading', { name: /not found/i })).toBeVisible();
});
