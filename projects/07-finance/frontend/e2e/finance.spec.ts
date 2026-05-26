import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix: string): string {
  return `${prefix} ${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

test('home page is accessible (axe-core)', async ({ page }) => {
  await gotoHydrated(page, '/');
  const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
});

test('seed accounts + transaction, see correct balances', async ({ page, request }) => {
  // Create asset + expense accounts via API
  const checkingName = uniq('Checking');
  const groceriesName = uniq('Groceries');

  const checking = await request
    .post('http://localhost:3006/api/accounts', {
      data: { name: checkingName, kind: 'asset', color: '#4F46E5' }
    })
    .then((r) => r.json());
  const groceries = await request
    .post('http://localhost:3006/api/accounts', {
      data: { name: groceriesName, kind: 'expense', color: '#DC2626' }
    })
    .then((r) => r.json());

  // Spent $42.50 on groceries — Checking -4250, Groceries +4250
  const txRes = await request.post('http://localhost:3006/api/transactions', {
    data: {
      description: 'shopping',
      postings: [
        { account_id: checking.id, amount_minor: -4250 },
        { account_id: groceries.id, amount_minor: 4250 }
      ]
    }
  });
  expect(txRes.status()).toBe(201);

  await gotoHydrated(page, '/');

  // Account list shows our two accounts (.first() because the names also
  // appear in the From/To selects and the asset-target select for imports).
  await expect(page.getByText(checkingName).first()).toBeVisible();
  await expect(page.getByText(groceriesName).first()).toBeVisible();

  // The transaction is in history
  await expect(page.locator('li').filter({ hasText: 'shopping' }).first()).toBeVisible();
});

test('unbalanced transaction is rejected by the backend', async ({ request }) => {
  // Try to POST a transaction whose postings don't sum to zero — must 422.
  const a = await request
    .post('http://localhost:3006/api/accounts', {
      data: { name: uniq('A'), kind: 'asset', color: '#0284C7' }
    })
    .then((r) => r.json());
  const b = await request
    .post('http://localhost:3006/api/accounts', {
      data: { name: uniq('B'), kind: 'expense', color: '#7C3AED' }
    })
    .then((r) => r.json());

  const res = await request.post('http://localhost:3006/api/transactions', {
    data: {
      description: 'bad',
      postings: [
        { account_id: a.id, amount_minor: 100 },
        { account_id: b.id, amount_minor: 50 }
      ]
    }
  });
  expect(res.status()).toBe(422);
  const body = await res.json();
  expect(body.error.fields[0].field).toBe('postings');
});

test('submit button disabled until from/to/amount are valid', async ({ page, request }) => {
  await request.post('http://localhost:3006/api/accounts', {
    data: { name: uniq('AcctA'), kind: 'asset', color: '#059669' }
  });
  await request.post('http://localhost:3006/api/accounts', {
    data: { name: uniq('AcctB'), kind: 'expense', color: '#DB2777' }
  });

  await gotoHydrated(page, '/');
  await expect(page.getByRole('button', { name: /record transaction/i })).toBeDisabled();
});

test('top expenses chart renders a canvas', async ({ page }) => {
  await gotoHydrated(page, '/');
  await expect(page.getByRole('img', { name: /top expense balances/i })).toBeVisible();
});
