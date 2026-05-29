import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test.describe('analytics', () => {
  test('home redirects to /login when unauthenticated', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
  });

  test('login page axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/login');
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(r.violations, JSON.stringify(r.violations, null, 2)).toEqual([]);
  });

  test('ingest endpoint accepts events', async ({ request }) => {
    const r = await request.post(
      `${process.env.VITE_BACKEND_URL ?? 'http://localhost:3026'}/v1/ingest`,
      { data: { kind: 'e2e_test', session_id: 's' } }
    );
    expect(r.status()).toBe(204);
  });
});
