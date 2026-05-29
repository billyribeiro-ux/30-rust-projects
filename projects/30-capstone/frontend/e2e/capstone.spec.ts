import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test.describe('capstone', () => {
  test('marketing homepage is axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(r.violations, JSON.stringify(r.violations, null, 2)).toEqual([]);
  });

  test('pricing page renders', async ({ page }) => {
    await gotoHydrated(page, '/pricing');
    await expect(page.getByRole('heading', { name: /pricing/i })).toBeVisible();
  });

  test('/p redirects to /login when unauthenticated', async ({ page }) => {
    await page.goto('/p');
    await page.waitForLoadState('networkidle');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
  });
});
