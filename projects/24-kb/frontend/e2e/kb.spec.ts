import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test.describe('kb', () => {
  test('homepage renders and is axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/');
    await expect(page.getByRole('heading', { name: /knowledge base/i }).first()).toBeVisible();
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(r.violations, JSON.stringify(r.violations, null, 2)).toEqual([]);
  });

  test('search form bounces query into URL', async ({ page }) => {
    await gotoHydrated(page, '/');
    await page.getByLabel(/search query/i).fill('postgres');
    await Promise.all([
      page.waitForURL(/[?&]q=postgres/),
      page.getByRole('button', { name: /search/i }).click()
    ]);
  });

  test('unknown slug returns 404', async ({ page }) => {
    const res = await page.goto('/a/this-doesnt-exist');
    expect(res?.status()).toBe(404);
  });

  test('sitemap.xml is well-formed', async ({ request }) => {
    const r = await request.get(
      `${process.env.VITE_BACKEND_URL ?? 'http://localhost:3023'}/sitemap.xml`
    );
    expect(r.status()).toBe(200);
    const body = await r.text();
    expect(body).toContain('<urlset');
  });
});
