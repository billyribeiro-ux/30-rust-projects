/**
 * End-to-end suite for the jobs dashboard.
 *
 * Backend at $VITE_BACKEND_URL must be running.
 */

import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = process.env.VITE_BACKEND_URL ?? 'http://localhost:3021';
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
  const password = 'correct horse battery staple';
  const res = await req.post(`${BACKEND}/api/auth/register`, {
    data: { email, password, name: 'E2E' }
  });
  expect(res.status()).toBe(201);
  const setCookie = res.headers()['set-cookie'] ?? '';
  const match = setCookie.match(new RegExp(`${COOKIE}=([^;]+)`));
  if (!match) throw new Error('no session cookie');
  return { email, password, cookie: match[1]! };
}

test.describe('jobs dashboard', () => {
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

  test('signed-in user sees the dashboard and can enqueue a job', async ({ page, request }) => {
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
    // Don't use networkidle here — the SSE connection keeps the network active.
    await page.goto('/jobs');
    await expect(page.getByRole('heading', { name: 'Queues' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Enqueue' })).toBeVisible();

    await page.getByLabel(/^kind/i).selectOption('send_email');
    await Promise.all([
      page.waitForResponse((r) => r.url().includes('?/enqueue') && r.status() < 400),
      page.getByRole('button', { name: /enqueue/i }).click()
    ]);

    // The "default" queue should be present in the Queues table.
    await expect(page.getByRole('cell', { name: 'default' }).first()).toBeVisible({
      timeout: 5_000
    });
  });
});
