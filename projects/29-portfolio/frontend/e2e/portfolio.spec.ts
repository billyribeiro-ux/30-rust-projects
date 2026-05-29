import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test.describe('portfolio', () => {
  test('homepage renders and is axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/');
    await expect(page.getByRole('heading', { name: /selected work/i })).toBeVisible();
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(r.violations, JSON.stringify(r.violations, null, 2)).toEqual([]);
  });

  test('unknown work slug returns 404', async ({ page }) => {
    const res = await page.goto('/work/this-doesnt-exist');
    expect(res?.status()).toBe(404);
  });
});
