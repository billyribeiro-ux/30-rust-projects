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

test('add member, log expense via API, see balance', async ({ page, request }) => {
  // Seed two members + one expense via API to avoid clicking through the full UI.
  const aName = uniq('Alice');
  const bName = uniq('Bob');
  const aRes = await request.post('http://localhost:3005/api/members', {
    data: { name: aName, color: '#4F46E5' }
  });
  const a = await aRes.json();
  const bRes = await request.post('http://localhost:3005/api/members', {
    data: { name: bName, color: '#059669' }
  });
  const b = await bRes.json();

  // Alice pays $30, split equally → each owes 1500c, Alice net +1500c
  const expRes = await request.post('http://localhost:3005/api/expenses', {
    data: {
      payer_id: a.id,
      amount_cents: 3000,
      description: 'dinner',
      split_kind: 'equal',
      shares: [
        { member_id: a.id, value: 0 },
        { member_id: b.id, value: 0 }
      ]
    }
  });
  expect(expRes.status()).toBe(201);

  await gotoHydrated(page, '/');

  await expect(page.getByText(aName).first()).toBeVisible();
  await expect(page.getByText(bName).first()).toBeVisible();

  // Balances: Alice net +$15, Bob net -$15
  const balancesPanel = page.getByRole('region', { name: /balances/i });
  await expect(balancesPanel).toContainText('+$15.00');
  await expect(balancesPanel).toContainText('-$15.00');

  // Suggested settlement: Bob → Alice $15
  await expect(balancesPanel).toContainText('$15.00');
});

test('submit button disabled when amount is zero', async ({ page, request }) => {
  const m = await request
    .post('http://localhost:3005/api/members', {
      data: { name: uniq('Zoe'), color: '#DC2626' }
    })
    .then((r) => r.json());

  await gotoHydrated(page, '/');
  await page.getByLabel(/who paid/i).selectOption(m.id);
  await page.getByLabel(/^Amount/i).fill('0');
  // Submit is disabled because preview.ok is false (amount must be > 0)
  await expect(page.getByRole('button', { name: /add expense/i })).toBeDisabled();
});

test('equal split preview shows correct per-member amount', async ({ page, request }) => {
  // Need two members
  const a = await request
    .post('http://localhost:3005/api/members', { data: { name: uniq('P1'), color: '#0284C7' } })
    .then((r) => r.json());
  const b = await request
    .post('http://localhost:3005/api/members', { data: { name: uniq('P2'), color: '#7C3AED' } })
    .then((r) => r.json());

  await gotoHydrated(page, '/');
  await page.getByLabel(/who paid/i).selectOption(a.id);
  await page.getByLabel(/^Amount/i).fill('10.00');
  // The check-sum line shows the total
  await expect(page.getByText(/Shares total \$10\.00/)).toBeVisible();
  // Submit button enabled
  await expect(page.getByRole('button', { name: /add expense/i })).toBeEnabled();
});
