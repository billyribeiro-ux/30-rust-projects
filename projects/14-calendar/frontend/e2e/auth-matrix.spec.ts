import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = 'http://localhost:3013';
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

async function apiCreateCalendar(req: APIRequestContext, cookie: string, name: string) {
  const res = await req.post(`${BACKEND}/api/calendars`, {
    headers: { cookie: `${COOKIE}=${cookie}`, 'content-type': 'application/json' },
    data: { name, default_tz: 'UTC' }
  });
  expect(res.status()).toBe(201);
  return (await res.json()).id as string;
}

async function apiCreateRecurringEvent(
  req: APIRequestContext,
  cookie: string,
  calendarId: string,
  rrule: string
) {
  const res = await req.post(`${BACKEND}/api/events`, {
    headers: { cookie: `${COOKIE}=${cookie}`, 'content-type': 'application/json' },
    data: {
      calendar_id: calendarId,
      title: 'Weekly standup',
      start_at: '2026-05-25T16:00:00Z',
      end_at: '2026-05-25T16:30:00Z',
      tz: 'UTC',
      rrule
    }
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

  test('register form lands on calendar with the cookie set', async ({ page }) => {
    await gotoHydrated(page, '/register');
    const email = `${uniq('e2e')}@example.com`;
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill('correct horse battery staple');
    await page.getByRole('button', { name: /create account/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');
    await expect(page.locator('h1')).toBeVisible();
  });

  test('login + month-view a11y (axe-core)', async ({ page, request }) => {
    const { email, password } = await apiRegister(request);

    await gotoHydrated(page, '/login');
    const loginAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(loginAxe.violations, JSON.stringify(loginAxe.violations, null, 2)).toEqual([]);

    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill(password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/');

    const monthAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(monthAxe.violations, JSON.stringify(monthAxe.violations, null, 2)).toEqual([]);
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

  test("PERMISSION MATRIX: user A cannot read user B's calendar", async ({ request }) => {
    const a = await apiRegister(request);
    const b = await apiRegister(request);
    const bCal = await apiCreateCalendar(request, b.cookie, 'B-Owned');

    const res = await request.get(`${BACKEND}/api/calendars/${bCal}`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(res.status()).toBe(404);

    const events = await request.get(
      `${BACKEND}/api/events?from=2026-01-01T00:00:00Z&to=2026-01-08T00:00:00Z&calendar_id=${bCal}`,
      { headers: { cookie: `${COOKIE}=${a.cookie}` } }
    );
    expect(events.status()).toBe(200);
    expect(await events.json()).toEqual([]);

    const bRead = await request.get(`${BACKEND}/api/calendars/${bCal}`, {
      headers: { cookie: `${COOKIE}=${b.cookie}` }
    });
    expect(bRead.status()).toBe(200);
  });

  test('PERMISSION MATRIX: no cookie → 401 on every protected endpoint', async ({ request }) => {
    const endpoints: [string, string][] = [
      ['GET', '/api/auth/me'],
      ['GET', '/api/calendars'],
      ['POST', '/api/calendars'],
      ['GET', '/api/events?from=2026-01-01T00:00:00Z&to=2026-01-08T00:00:00Z'],
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

  test('RRULE expansion: 4 occurrences in a 4-week window', async ({ request }) => {
    const a = await apiRegister(request);
    const cal = await apiCreateCalendar(request, a.cookie, 'Work');
    await apiCreateRecurringEvent(
      request,
      a.cookie,
      cal,
      'RRULE:FREQ=WEEKLY;BYDAY=MO;COUNT=8'
    );

    const res = await request.get(
      `${BACKEND}/api/events?from=2026-05-24T00:00:00Z&to=2026-06-22T00:00:00Z`,
      { headers: { cookie: `${COOKIE}=${a.cookie}` } }
    );
    expect(res.status()).toBe(200);
    const events = await res.json();
    expect(events.length).toBe(4);
    for (const e of events) {
      expect(e.is_recurring).toBe(true);
      expect(e.title).toBe('Weekly standup');
    }
  });

  test('Range query rejects ranges greater than 90 days', async ({ request }) => {
    const a = await apiRegister(request);
    const res = await request.get(
      `${BACKEND}/api/events?from=2026-01-01T00:00:00Z&to=2026-06-01T00:00:00Z`,
      { headers: { cookie: `${COOKIE}=${a.cookie}` } }
    );
    expect(res.status()).toBe(422); // AppError::Validation maps to 422 Unprocessable Entity
  });

  test('ICS export returns text/calendar with VCALENDAR', async ({ request }) => {
    const a = await apiRegister(request);
    const cal = await apiCreateCalendar(request, a.cookie, 'Personal');
    const res = await request.get(`${BACKEND}/api/export/calendar/${cal}`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(res.status()).toBe(200);
    expect(res.headers()['content-type']).toMatch(/text\/calendar/);
    const body = await res.text();
    expect(body).toContain('BEGIN:VCALENDAR');
    expect(body).toContain('END:VCALENDAR');
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
