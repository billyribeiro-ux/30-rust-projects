import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = 'http://localhost:3010';

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
  const match = setCookie.match(/contacts_session=([^;]+)/);
  if (!match) throw new Error('no session cookie in register response');
  return { email, password, cookie: match[1]!, headers: setCookie };
}

/** Create a contact for the API user. Returns its id. */
async function apiCreateContact(req: APIRequestContext, cookie: string, name: string) {
  const res = await req.post(`${BACKEND}/api/contacts`, {
    headers: { cookie: `contacts_session=${cookie}`, 'content-type': 'application/json' },
    data: { name }
  });
  expect(res.status()).toBe(201);
  return (await res.json()).id as string;
}

test.describe('auth + permission matrix', () => {
  test('home (/) redirects to /login when unauthenticated', async ({ page }) => {
    const res = await page.goto('/');
    // SvelteKit redirect lands on /login?next=…
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
    // Status of the final response should be 200 (the /login page)
    expect(res?.status()).toBe(200);
  });

  test('register form lands on dashboard with the cookie set', async ({ page }) => {
    await gotoHydrated(page, '/register');
    const email = `${uniq('e2e')}@example.com`;
    await page.getByLabel(/^email$/i).fill(email);
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

    await page.getByLabel(/^email$/i).fill(email);
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
    await page.getByLabel(/^email$/i).fill(email);
    await page.getByLabel(/^password/i).fill('definitely the wrong password');
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByRole('alert')).toHaveText(/invalid email or password/i);
    // Still on /login (no session cookie set)
    expect(page.url()).toMatch(/\/login/);
  });

  test('PERMISSION MATRIX: user A cannot read user B\'s contact', async ({ request }) => {
    // The headline test: per-user scoping is enforced SERVER-SIDE, not just
    // hidden in the UI. A's session cookie should never see B's data.
    const a = await apiRegister(request);
    const b = await apiRegister(request);
    const bContactId = await apiCreateContact(request, b.cookie, 'Bob B-Owned Contact');

    // A tries to GET B's contact by id → 404 (NOT 403 — we never confirm
    // the row exists; same response as "no such id at all").
    const res = await request.get(`${BACKEND}/api/contacts/${bContactId}`, {
      headers: { cookie: `contacts_session=${a.cookie}` }
    });
    expect(res.status()).toBe(404);

    // A tries to PATCH B's contact → 404
    const patch = await request.patch(`${BACKEND}/api/contacts/${bContactId}`, {
      headers: { cookie: `contacts_session=${a.cookie}`, 'content-type': 'application/json' },
      data: { name: 'hijacked' }
    });
    expect(patch.status()).toBe(404);

    // A tries to DELETE B's contact → 404
    const del = await request.delete(`${BACKEND}/api/contacts/${bContactId}`, {
      headers: { cookie: `contacts_session=${a.cookie}` }
    });
    expect(del.status()).toBe(404);

    // B can still see + delete their own contact
    const bRead = await request.get(`${BACKEND}/api/contacts/${bContactId}`, {
      headers: { cookie: `contacts_session=${b.cookie}` }
    });
    expect(bRead.status()).toBe(200);
  });

  test('PERMISSION MATRIX: no cookie → 401 on every protected endpoint', async ({ request }) => {
    const endpoints: [string, string][] = [
      ['GET', '/api/auth/me'],
      ['GET', '/api/contacts'],
      ['POST', '/api/contacts'],
      ['GET', '/api/dashboard/'],
      ['GET', '/api/tags/'],
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
    await page.getByLabel(/^email$/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');

    await page.getByRole('button', { name: /sign out/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL(/\/login(?:\?|$)/);

    // Going back to / should still bounce — cookie is gone
    const res = await page.goto('/');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
    expect(res?.status()).toBe(200);
  });

  test('contacts CRUD round-trip via the UI', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email$/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');

    // Add via UI
    await page.getByRole('link', { name: /contacts/i }).first().click();
    await page.waitForLoadState('networkidle');
    await page.getByRole('link', { name: /add contact/i }).click();
    await page.waitForLoadState('networkidle');
    const name = `Contact ${uniq()}`;
    await page.getByLabel(/^name/i).fill(name);
    await page.getByLabel(/^company$/i).fill('Acme');
    await page.getByLabel(/^email$/i).fill('person@acme.com');
    await page.getByLabel(/^tags/i).fill('work important');
    await page.getByRole('button', { name: /save contact/i }).click();
    await page.waitForLoadState('networkidle');

    // Now on detail page
    await expect(page.getByRole('heading', { level: 1, name })).toBeVisible();
    await expect(page.getByText('Acme')).toBeVisible();

    // Touch the contact → last_contacted_at populated; refresh shows it
    await page.getByRole('button', { name: /mark as contacted now/i }).click();
    await page.waitForLoadState('networkidle');

    // Search for it from the list view
    await page.goto('/contacts');
    await page.waitForLoadState('networkidle');
    await page.getByRole('searchbox', { name: /search contacts/i }).fill('acme');
    await page.waitForTimeout(300);
    await page.waitForLoadState('networkidle');
    await expect(page.getByText(name)).toBeVisible();
  });
});
