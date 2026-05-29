/**
 * End-to-end suite for the kanban frontend.
 *
 * The Playwright config boots `pnpm preview` on port 4191 — meaning these
 * tests drive the SSR'd SvelteKit Node server, which talks to the Rust
 * backend at `VITE_BACKEND_URL` (defaults to http://localhost:3018).
 *
 * The Rust backend must already be running (see COMMANDS.md). The suite
 * is idempotent: each test creates a unique user + board so it stays
 * green when re-run against the same database.
 *
 * Coverage:
 *  - sign-up flow lands you on /boards (axe-clean)
 *  - board create + open round trips
 *  - drag attribute is exposed only to editors+
 *  - logout returns you to /login
 */

import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = process.env.VITE_BACKEND_URL ?? 'http://localhost:3018';
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
  const name = 'Test User';
  const res = await req.post(`${BACKEND}/api/auth/register`, {
    data: { email, password, name }
  });
  expect(res.status()).toBe(201);
  const setCookie = res.headers()['set-cookie'] ?? '';
  const match = setCookie.match(new RegExp(`${COOKIE}=([^;]+)`));
  if (!match) throw new Error('no session cookie in register response');
  return { email, password, name, cookie: match[1]! };
}

test.describe('kanban: auth + boards + drag-drop UI', () => {
  test('home redirects to /boards then /login when unauthenticated', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
  });

  test('login page is axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/login');
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
  });

  test('signup → /boards is reachable and axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/signup');
    const signupAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(signupAxe.violations, JSON.stringify(signupAxe.violations, null, 2)).toEqual([]);

    const email = `${uniq('e2e')}@example.com`;
    await page.getByLabel(/^name/i).fill('E2E User');
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill('correct horse battery staple');
    await page.getByRole('button', { name: /create account/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/boards');

    const boardsAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(boardsAxe.violations, JSON.stringify(boardsAxe.violations, null, 2)).toEqual([]);
  });

  test('create board and open it; cards container has draggable for editor', async ({
    page,
    request
  }) => {
    const a = await apiRegister(request);

    // Server-side: create a board + a list + a card via the API.
    const slug = uniq('e2e').toLowerCase();
    const boardRes = await request.post(`${BACKEND}/api/boards`, {
      headers: { cookie: `${COOKIE}=${a.cookie}`, 'content-type': 'application/json' },
      data: { name: 'E2E Board', slug }
    });
    expect(boardRes.status()).toBe(201);
    const board = await boardRes.json();

    const listRes = await request.post(`${BACKEND}/api/boards/${board.id}/lists`, {
      headers: { cookie: `${COOKIE}=${a.cookie}`, 'content-type': 'application/json' },
      data: { name: 'Todo' }
    });
    expect(listRes.status()).toBe(201);
    const list = await listRes.json();

    const cardRes = await request.post(`${BACKEND}/api/lists/${list.id}/cards`, {
      headers: { cookie: `${COOKIE}=${a.cookie}`, 'content-type': 'application/json' },
      data: { title: 'Buy groceries' }
    });
    expect(cardRes.status()).toBe(201);

    // Forward the session cookie into the browser so SSR sees us as logged in.
    await page.context().addCookies([
      {
        name: COOKIE,
        value: a.cookie,
        domain: 'localhost',
        path: '/',
        httpOnly: true,
        secure: false,
        sameSite: 'Lax'
      }
    ]);

    await gotoHydrated(page, `/boards/${slug}`);
    await expect(page.getByRole('heading', { name: 'E2E Board' })).toBeVisible();
    await expect(page.getByText('Buy groceries')).toBeVisible();

    // Editor role means the card is draggable.
    const card = page.locator('[data-card-id]').first();
    await expect(card).toHaveAttribute('draggable', 'true');
  });

  test('logout returns to /login', async ({ page, request }) => {
    const a = await apiRegister(request);
    await page.context().addCookies([
      {
        name: COOKIE,
        value: a.cookie,
        domain: 'localhost',
        path: '/',
        httpOnly: true,
        secure: false,
        sameSite: 'Lax'
      }
    ]);
    await gotoHydrated(page, '/boards');
    await page.getByRole('button', { name: /sign out/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/login');
  });
});
