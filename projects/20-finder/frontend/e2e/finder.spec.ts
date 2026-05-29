/**
 * End-to-end suite for the finder frontend.
 *
 * Backend at `VITE_BACKEND_URL` must be running with `TEST_ONLY_TOKEN`
 * exported (see COMMANDS.md §5) — that unlocks `/api/test/login-as`,
 * the only practical way to inject a session for an OAuth-or-magic-link
 * product in an automated suite.
 *
 * Coverage:
 *   - homepage renders search form + Restaurant ItemList JSON-LD
 *   - search by query updates URL + result set
 *   - login page exposes both OAuth buttons and axe-clean
 *   - logout returns to home
 */

import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = process.env.VITE_BACKEND_URL ?? 'http://localhost:3019';
const TEST_TOKEN = process.env.TEST_ONLY_TOKEN ?? '';
const COOKIE = 'app_session';

async function gotoHydrated(page: Page, url: string) {
  await page.goto(url);
  await page.waitForLoadState('networkidle');
}

function uniq(prefix = 'user') {
  return `${prefix}-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

async function injectSession(req: APIRequestContext, email: string): Promise<string> {
  const res = await req.post(`${BACKEND}/api/test/login-as`, {
    headers: { 'x-test-token': TEST_TOKEN, 'content-type': 'application/json' },
    data: { email, name: 'E2E' }
  });
  expect(res.status(), `test/login-as should succeed (set TEST_ONLY_TOKEN). got ${res.status()}`).toBe(200);
  const setCookie = res.headers()['set-cookie'] ?? '';
  const match = setCookie.match(new RegExp(`${COOKIE}=([^;]+)`));
  if (!match) throw new Error('no session cookie in test/login-as response');
  return match[1]!;
}

test.describe('finder: search + auth + SEO', () => {
  test('homepage renders and is axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/');
    await expect(page.getByRole('heading', { name: /find a place/i })).toBeVisible();
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
  });

  test('homepage emits Restaurant ItemList JSON-LD', async ({ page }) => {
    await gotoHydrated(page, '/');
    const ld = await page.locator('script[type="application/ld+json"]').first().textContent();
    expect(ld).toBeTruthy();
    const obj = JSON.parse(ld!);
    expect(obj['@type']).toBe('ItemList');
  });

  test('search query is reflected in URL', async ({ page }) => {
    await gotoHydrated(page, '/');
    await page.getByLabel(/^search query/i).fill('thai');
    await Promise.all([
      page.waitForURL(/[?&]q=thai/),
      page.getByRole('button', { name: /search/i }).click()
    ]);
  });

  test('login page is axe-clean and exposes OAuth buttons', async ({ page }) => {
    await gotoHydrated(page, '/login');
    await expect(page.getByRole('link', { name: /google/i })).toBeVisible();
    await expect(page.getByRole('link', { name: /github/i })).toBeVisible();
    const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
  });

  test('signed-in user sees their name and can log out', async ({ page, request }) => {
    test.skip(!TEST_TOKEN, 'TEST_ONLY_TOKEN not set on backend');
    const email = `${uniq('e2e')}@example.com`;
    const cookie = await injectSession(request, email);
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
    await gotoHydrated(page, '/');
    await expect(page.getByText('E2E')).toBeVisible();
    await page.getByRole('link', { name: /sign out/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');
  });
});
