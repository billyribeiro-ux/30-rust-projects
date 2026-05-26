import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = 'http://localhost:3011';
const COOKIE = 'app_session';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix = 'user') {
  return `${prefix}-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

/** Register a user directly via the API and return { email, password, cookie }. */
async function apiRegister(req: APIRequestContext) {
  const email = `${uniq()}@example.com`;
  const password = 'correct horse battery staple';
  const res = await req.post(`${BACKEND}/api/auth/register`, {
    data: { email, password, name: 'Test User' }
  });
  expect(res.status()).toBe(201);
  const setCookie = res.headers()['set-cookie'] ?? '';
  const match = setCookie.match(new RegExp(`${COOKIE}=([^;]+)`));
  if (!match) throw new Error('no session cookie in register response');
  return { email, password, cookie: match[1]!, headers: setCookie };
}

/** Create an application for the API user. Returns its id. */
async function apiCreateApplication(req: APIRequestContext, cookie: string, company: string) {
  const res = await req.post(`${BACKEND}/api/applications`, {
    headers: { cookie: `${COOKIE}=${cookie}`, 'content-type': 'application/json' },
    data: { company, role: 'Senior Engineer' }
  });
  expect(res.status()).toBe(201);
  return (await res.json()).id as string;
}

test.describe('auth + permission matrix', () => {
  test('home (/) redirects to /login when unauthenticated', async ({ page }) => {
    const res = await page.goto('/');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
    expect(res?.status()).toBe(200);
  });

  test('register form lands on dashboard with the cookie set', async ({ page }) => {
    await gotoHydrated(page, '/register');
    const email = `${uniq('e2e')}@example.com`;
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill('correct horse battery staple');
    await page.getByRole('button', { name: /create account/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');
    await expect(page.getByRole('heading', { name: /dashboard/i })).toBeVisible();
  });

  test('login + dashboard a11y (axe-core)', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);

    await gotoHydrated(page, '/login');
    const loginAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(loginAxe.violations, JSON.stringify(loginAxe.violations, null, 2)).toEqual([]);

    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');

    const dashAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(dashAxe.violations, JSON.stringify(dashAxe.violations, null, 2)).toEqual([]);
  });

  test('wrong password returns generic 401 error', async ({ page, request }) => {
    const { email } = await apiRegister(request);
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill('definitely the wrong password');
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('alert')).toHaveText(/invalid email or password/i);
    expect(page.url()).toMatch(/\/login/);
  });

  test("PERMISSION MATRIX: user A cannot read user B's application", async ({ request }) => {
    // Per-user scoping is enforced SERVER-SIDE, not just hidden in the UI.
    // A's cookie should never see B's data. Returning 404 (not 403) avoids
    // leaking whether the id exists at all.
    const a = await apiRegister(request);
    const b = await apiRegister(request);
    const bAppId = await apiCreateApplication(request, b.cookie, 'B-Owned Corp');

    const res = await request.get(`${BACKEND}/api/applications/${bAppId}`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(res.status()).toBe(404);

    const patch = await request.patch(`${BACKEND}/api/applications/${bAppId}`, {
      headers: { cookie: `${COOKIE}=${a.cookie}`, 'content-type': 'application/json' },
      data: { company: 'hijacked' }
    });
    expect(patch.status()).toBe(404);

    const del = await request.delete(`${BACKEND}/api/applications/${bAppId}`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(del.status()).toBe(404);

    // B can still see their own row
    const bRead = await request.get(`${BACKEND}/api/applications/${bAppId}`, {
      headers: { cookie: `${COOKIE}=${b.cookie}` }
    });
    expect(bRead.status()).toBe(200);
  });

  test('PERMISSION MATRIX: no cookie → 401 on every protected endpoint', async ({ request }) => {
    const endpoints: [string, string][] = [
      ['GET', '/api/auth/me'],
      ['GET', '/api/applications'],
      ['POST', '/api/applications'],
      ['GET', '/api/dashboard'],
      ['POST', '/api/auth/logout']
    ];
    for (const [method, path] of endpoints) {
      const res = await request.fetch(`${BACKEND}${path}`, {
        method,
        data: method === 'POST' ? {} : undefined
      });
      expect(res.status(), `${method} ${path} should require auth`).toBe(401);
    }
  });

  test('logout clears the cookie and bounces back to /login', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');

    await page.getByRole('button', { name: /sign out/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL(/\/login(?:\?|$)/);

    const res = await page.goto('/');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
    expect(res?.status()).toBe(200);
  });

  test('applications CRUD round-trip via the UI', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');

    // Navigate to applications list (specifically the nav link, not the brand)
    await page.locator('nav a[href="/applications"]').first().click();
    await page.waitForLoadState('networkidle');

    // Add a new application
    await page.getByRole('link', { name: /add application/i }).click();
    await page.waitForLoadState('networkidle');
    const company = `Acme ${uniq()}`;
    await page.getByLabel(/^company/i).fill(company);
    await page.getByLabel(/^role/i).fill('Staff Engineer');
    await page.getByLabel(/^location/i).fill('Remote');
    await page.getByRole('button', { name: /save/i }).click();
    await page.waitForLoadState('networkidle');

    // Detail page
    await expect(page.getByRole('heading', { level: 1 })).toContainText(company);
    await expect(page.getByText('Staff Engineer').first()).toBeVisible();

    // Back to list — the new application is visible
    await page.goto('/applications');
    await page.waitForLoadState('networkidle');
    await expect(page.getByText(company)).toBeVisible();
  });

  test('ICS calendar export is text/calendar and contains VCALENDAR', async ({ request }) => {
    const { cookie } = await apiRegister(request);
    const res = await request.get(`${BACKEND}/api/export/next-steps.ics`, {
      headers: { cookie: `${COOKIE}=${cookie}` }
    });
    expect(res.status()).toBe(200);
    expect(res.headers()['content-type']).toMatch(/text\/calendar/);
    const body = await res.text();
    expect(body).toContain('BEGIN:VCALENDAR');
    expect(body).toContain('END:VCALENDAR');
  });
});
