import { test, expect } from '@playwright/test';

test('full task lifecycle: add, toggle done, delete', async ({ page }) => {
  await page.goto('/');

  await expect(page.getByRole('heading', { name: 'Today' })).toBeVisible();

  const input = page.getByLabel('New task');
  await input.fill('write tests');
  await page.getByRole('button', { name: /add/i }).click();

  const item = page.getByRole('listitem').filter({ hasText: 'write tests' });
  await expect(item).toBeVisible();

  await expect(page.locator('p.meta')).toContainText('1 of 1 remaining');

  const checkbox = item.getByRole('button', { name: /mark "write tests" as done/i });
  await checkbox.click();

  await expect(page.locator('p.meta')).toContainText('0 of 1 remaining');

  const removeBtn = item.getByRole('button', { name: /delete "write tests"/i });
  await removeBtn.click();

  await expect(item).toHaveCount(0);
  await expect(page.getByText('No tasks yet')).toBeVisible();
});

test('validation: empty title is blocked client-side', async ({ page }) => {
  await page.goto('/');
  const addBtn = page.getByRole('button', { name: /add/i });
  await expect(addBtn).toBeDisabled();
});
