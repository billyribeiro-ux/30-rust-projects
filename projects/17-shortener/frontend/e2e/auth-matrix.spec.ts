import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = 'http://localhost:3015';
const COOKIE = 'app_session';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix = 'user') {
  return `${prefix}-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

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

async function apiCreateLink(
  req: APIRequestContext,
  cookie: string,
  target_url: string,
  slug?: string
) {
  const res = await req.post(`${BACKEND}/api/links`, {
    headers: { cookie: `${COOKIE}=${cookie}`, 'content-type': 'application/json' },
    data: { target_url, slug: slug ?? null }
  });
  expect(res.status()).toBe(201);
  return (await res.json()) as { id: string; slug: string };
}

test.describe('auth + permission matrix', () => {
  test('home (/) redirects to /login when unauthenticated', async ({ page }) => {
    const res = await page.goto('/');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
    expect(res?.status()).toBe(200);
  });

  test('login + links page a11y (axe-core)', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);

    await gotoHydrated(page, '/login');
    const loginAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(loginAxe.violations, JSON.stringify(loginAxe.violations, null, 2)).toEqual([]);

    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');

    const homeAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(homeAxe.violations, JSON.stringify(homeAxe.violations, null, 2)).toEqual([]);
  });

  test('REDIRECT PATH: no auth, returns 307 to target_url', async ({ request }) => {
    const a = await apiRegister(request);
    const slug = `e2e-${Date.now().toString(36)}`;
    await apiCreateLink(request, a.cookie, 'https://example.com/landing', slug);

    // No cookie at all on the redirect — anyone with the slug can follow.
    const res = await request.get(`${BACKEND}/${slug}`, {
      maxRedirects: 0,
      headers: {
        'user-agent':
          'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15'
      }
    });
    expect(res.status()).toBe(307);
    expect(res.headers().location).toBe('https://example.com/landing');
  });

  test('BOT FILTER: curl UA gets the redirect but no click is recorded', async ({ request }) => {
    const a = await apiRegister(request);
    const slug = `bot-${Date.now().toString(36)}`;
    await apiCreateLink(request, a.cookie, 'https://example.com/landing', slug);

    // First: a real browser hit so stats has at least one non-bot row.
    const human = await request.get(`${BACKEND}/${slug}`, {
      maxRedirects: 0,
      headers: { 'user-agent': 'Mozilla/5.0 Chrome' }
    });
    expect(human.status()).toBe(307);

    // Then several bot hits.
    for (const ua of ['curl/8.4.0', 'Googlebot/2.1', 'Python-urllib/3.11', 'wget/1.21']) {
      const r = await request.get(`${BACKEND}/${slug}`, {
        maxRedirects: 0,
        headers: { 'user-agent': ua }
      });
      expect(r.status(), `bot UA ${ua} should still redirect`).toBe(307);
    }

    // Give tokio::spawn a beat to flush inserts.
    await new Promise((r) => setTimeout(r, 500));

    const stats = await request.get(`${BACKEND}/api/links/${slug}/stats`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(stats.status()).toBe(200);
    const body = await stats.json();
    // Bot rows are filtered out of the non-bot aggregates — only the human
    // hit shows up.
    expect(body.total_clicks).toBe(1);
  });

  test('STATS SCOPING: foreign-user slug returns 404, never 403', async ({ request }) => {
    const a = await apiRegister(request);
    const b = await apiRegister(request);
    const slug = `private-${Date.now().toString(36)}`;
    await apiCreateLink(request, b.cookie, 'https://b.example.com', slug);

    // User A can't see B's stats — and we return 404, not 403, so the
    // existence of the slug isn't leaked.
    const aStats = await request.get(`${BACKEND}/api/links/${slug}/stats`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(aStats.status()).toBe(404);

    // But B (the owner) can.
    const bStats = await request.get(`${BACKEND}/api/links/${slug}/stats`, {
      headers: { cookie: `${COOKIE}=${b.cookie}` }
    });
    expect(bStats.status()).toBe(200);
  });

  test('PERMISSION MATRIX: no cookie → 401 on every protected endpoint', async ({ request }) => {
    const endpoints: [string, string][] = [
      ['GET', '/api/auth/me'],
      ['GET', '/api/links'],
      ['POST', '/api/links'],
      ['POST', '/api/2fa/setup'],
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

  test('2FA SETUP returns a provisioning URI and a base64 PNG', async ({ request }) => {
    const a = await apiRegister(request);
    const res = await request.post(`${BACKEND}/api/2fa/setup`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.provisioning_uri).toMatch(/^otpauth:\/\/totp\//);
    // Base64 PNG: at minimum non-empty.
    expect(typeof body.qr_png_base64).toBe('string');
    expect(body.qr_png_base64.length).toBeGreaterThan(100);
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

  test('CREATE LINK via UI: form posts and link appears in list', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');

    const slug = `ui-${Date.now().toString(36)}`;
    await page.getByLabel(/target url/i).fill('https://example.com/');
    await page.getByLabel(/custom slug/i).fill(slug);
    await page.getByRole('button', { name: /create short link/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page.getByText(`/${slug}`)).toBeVisible();
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
  });
});
