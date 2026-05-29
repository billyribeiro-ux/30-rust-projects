import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test.describe('interview', () => {
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

  test('SAML metadata endpoint serves valid XML', async ({ request }) => {
    const r = await request.get(
      `${process.env.VITE_BACKEND_URL ?? 'http://localhost:3025'}/saml/metadata`
    );
    expect(r.status()).toBe(200);
    const text = await r.text();
    expect(text).toContain('<md:EntityDescriptor');
  });
});
