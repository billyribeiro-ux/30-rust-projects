import { test, expect, type Page } from '@playwright/test';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

test('dashboard renders the four stat tiles', async ({ page }) => {
  await gotoHydrated(page, '/');
  await expect(page.getByTestId('stat-last7')).toBeVisible();
  await expect(page.getByTestId('stat-volume')).toBeVisible();
  await expect(page.getByTestId('stat-prs')).toBeVisible();
  await expect(page.getByTestId('stat-avg')).toBeVisible();
});

test('dashboard exposes a CSV export link to /api/export.csv', async ({ page }) => {
  await gotoHydrated(page, '/');
  const link = page.getByTestId('export-link');
  await expect(link).toBeVisible();
  await expect(link).toHaveAttribute('href', /\/api\/export\.csv$/);
});
