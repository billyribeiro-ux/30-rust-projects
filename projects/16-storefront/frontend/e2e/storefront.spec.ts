import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test.describe('storefront', () => {
  test('home page renders the storefront title', async ({ page }) => {
    await gotoHydrated(page, '/');
    await expect(page.getByRole('heading', { name: /Digital Storefront/i })).toBeVisible();
    // The email field is the gate for checkout — must be reachable.
    await expect(page.getByLabel(/^Email/)).toBeVisible();
  });

  test('cancel page is reachable from URL', async ({ page }) => {
    await gotoHydrated(page, '/cancel');
    await expect(page.getByRole('heading', { name: /cancelled/i })).toBeVisible();
  });

  test('success page surfaces session_id from query param', async ({ page }) => {
    await gotoHydrated(page, '/success?session_id=cs_test_abc');
    await expect(page.getByText('cs_test_abc')).toBeVisible();
  });

  test('admin login page is reachable', async ({ page }) => {
    await gotoHydrated(page, '/admin/login');
    await expect(page.getByRole('heading', { name: /Admin sign in/i })).toBeVisible();
  });

  test('home page passes axe-core (wcag2a + wcag2aa)', async ({ page }) => {
    await gotoHydrated(page, '/');
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
  });

  test('cancel page passes axe-core', async ({ page }) => {
    await gotoHydrated(page, '/cancel');
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
  });

  test('admin login passes axe-core', async ({ page }) => {
    await gotoHydrated(page, '/admin/login');
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
  });
});
