import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = process.env.VITE_BACKEND_URL ?? 'http://localhost:3022';
const COOKIE = 'app_session';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq() {
  return `${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

async function apiRegister(req: APIRequestContext) {
  const email = `${uniq()}@example.com`;
  const r = await req.post(`${BACKEND}/api/auth/register`, {
    data: { email, password: 'correct horse battery', name: 'E2E' }
  });
  expect(r.status()).toBe(201);
  const setCookie = r.headers()['set-cookie'] ?? '';
  const m = setCookie.match(new RegExp(`${COOKIE}=([^;]+)`));
  if (!m) throw new Error('no cookie');
  return m[1]!;
}

test.describe('helpdesk', () => {
  test('home redirects to /login when unauthenticated', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
  });

  test('login page is axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/login');
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(r.violations, JSON.stringify(r.violations, null, 2)).toEqual([]);
  });

  test('signed-in user creates a tenant and opens a ticket', async ({ page, request }) => {
    const cookie = await apiRegister(request);
    await page.context().addCookies([
      {
        name: COOKIE,
        value: cookie,
        domain: 'localhost',
        path: '/',
        httpOnly: true,
        secure: false,
        sameSite: 'Lax'
      }
    ]);

    await gotoHydrated(page, '/tenants');
    await expect(page.getByRole('heading', { name: 'Tenants' })).toBeVisible();

    // Create tenant.
    const slug = `e2e${uniq().replace(/-/g, '').slice(0, 30)}`;
    await page.getByLabel(/^slug/i).fill(slug);
    await page.getByLabel(/^name/i).fill('E2E Tenant');
    await Promise.all([
      page.waitForResponse((r) => r.url().includes('?/create') && r.status() < 400),
      page.getByRole('button', { name: /create tenant/i }).click()
    ]);

    // Navigate to it.
    await page.getByText('E2E Tenant').click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('heading', { name: 'Tickets' })).toBeVisible();

    // Open a ticket.
    await page.getByLabel(/^subject/i).fill('Help me');
    await page.getByLabel(/^body/i).fill('Stuck on step 3.');
    await Promise.all([
      page.waitForResponse((r) => r.url().includes('?/create') && r.status() < 400),
      page.getByRole('button', { name: /create ticket/i }).click()
    ]);
    await expect(page.getByText('Help me').first()).toBeVisible({ timeout: 5_000 });
  });
});
