/**
 * End-to-end suite for the newsletter frontend.
 *
 * Backend at $VITE_BACKEND_URL must be running (no live Stripe — the
 * subscribe action will fail with a clean error if STRIPE_SECRET_KEY
 * is a stub, which is fine for the "form is wired up" check).
 *
 * Coverage:
 *   - homepage renders + axe-clean
 *   - subscribe with free plan succeeds (no Stripe call)
 *   - 404 for unknown slug
 */

import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq() {
  return `${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

test.describe('newsletter', () => {
  test('homepage renders and is axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/');
    await expect(page.getByRole('heading', { name: /the newsletter/i })).toBeVisible();
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
  });

  test('homepage links to RSS feed', async ({ page }) => {
    await page.goto('/');
    const rss = await page.locator('link[rel="alternate"][type="application/rss+xml"]').first().getAttribute('href');
    expect(rss).toContain('/feed.xml');
  });

  test('subscribe with free plan returns a success message', async ({ page }) => {
    await gotoHydrated(page, '/');
    await page.getByLabel(/^email/i).fill(`free-${uniq()}@example.com`);
    await page.getByRole('radio', { name: /free/i }).check();
    await page.getByRole('button', { name: /subscribe/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByText(/check your inbox|subscribed/i)).toBeVisible();
  });

  test('unknown post slug is 404', async ({ page }) => {
    const res = await page.goto('/p/this-slug-does-not-exist');
    expect(res?.status()).toBe(404);
  });
});
