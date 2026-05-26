import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = 'http://localhost:3012';
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

async function apiCreateRoom(req: APIRequestContext, cookie: string, slug: string) {
  const res = await req.post(`${BACKEND}/api/rooms`, {
    headers: { cookie: `${COOKIE}=${cookie}`, 'content-type': 'application/json' },
    data: { slug, name: `Room ${slug}` }
  });
  expect(res.status()).toBe(201);
  return (await res.json()).slug as string;
}

test.describe('auth + permission matrix', () => {
  test('home (/) redirects to /login when unauthenticated', async ({ page }) => {
    const res = await page.goto('/');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
    expect(res?.status()).toBe(200);
  });

  test('register form lands on rooms with the cookie set', async ({ page }) => {
    await gotoHydrated(page, '/register');
    const email = `${uniq('e2e')}@example.com`;
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill('correct horse battery staple');
    await page.getByRole('button', { name: /create account/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');
    await expect(page.getByRole('heading', { name: /^rooms$/i })).toBeVisible();
  });

  test('login + rooms-list a11y (axe-core)', async ({ page, request }) => {
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

  test('PERMISSION MATRIX: non-member cannot read a room', async ({ request }) => {
    const a = await apiRegister(request);
    const b = await apiRegister(request);
    const slug = `bowned-${uniq('r')}`.toLowerCase().replace(/_/g, '-');
    await apiCreateRoom(request, b.cookie, slug);

    // A is not a member → 404
    const res = await request.get(`${BACKEND}/api/rooms/${slug}`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(res.status()).toBe(404);

    // A also can't read B's scrollback
    const msgs = await request.get(`${BACKEND}/api/rooms/${slug}/messages`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(msgs.status()).toBe(404);

    // B can read their own room
    const ok = await request.get(`${BACKEND}/api/rooms/${slug}`, {
      headers: { cookie: `${COOKIE}=${b.cookie}` }
    });
    expect(ok.status()).toBe(200);
  });

  test('PERMISSION MATRIX: no cookie → 401 on every protected endpoint', async ({ request }) => {
    const endpoints: [string, string][] = [
      ['GET', '/api/auth/me'],
      ['GET', '/api/rooms'],
      ['POST', '/api/rooms'],
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

  test('create a room via UI and land in the chat view', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');

    const slug = `e2e-${Date.now().toString(36)}`;
    // The create form is in the "Create a room" section.
    const createForm = page.locator('section[aria-label="Create a room"]');
    await createForm.getByLabel(/^slug/i).fill(slug);
    await createForm.getByLabel(/^name/i).fill('e2e test room');
    await createForm.getByRole('button', { name: /create room/i }).click();
    await page.waitForLoadState('networkidle');

    await expect(page).toHaveURL(`/rooms/${slug}`);
    await expect(page.getByRole('heading', { level: 1 })).toContainText(slug);
  });

  test('WebSocket round-trip: send a message and see it echo', async ({ page, request, browser }) => {
    const { email, password } = await apiRegister(request);
    const slug = `wsroom-${Date.now().toString(36)}`;
    // Pre-create the room via API so we go straight to the chat view.
    const ctx = await browser.newContext();
    await page.context().storageState();
    // Simpler: log in via UI, create via UI.
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');

    const createForm = page.locator('section[aria-label="Create a room"]');
    await createForm.getByLabel(/^slug/i).fill(slug);
    await createForm.getByLabel(/^name/i).fill('ws round trip');
    await createForm.getByRole('button', { name: /create room/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL(`/rooms/${slug}`);

    // Wait for the WS to connect (status pill flips to "Live").
    await expect(page.getByText(/^live$/i)).toBeVisible({ timeout: 10_000 });

    // Send a message.
    const composer = page.getByPlaceholder(`Message #${slug}…`);
    await composer.fill('hello over the wire');
    await page.getByRole('button', { name: /^send$/i }).click();

    // Server echoes it back via the broadcast channel.
    await expect(page.getByText('hello over the wire')).toBeVisible({ timeout: 5_000 });
    await ctx.close();
  });
});
