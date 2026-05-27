import { test, expect, type APIRequestContext, type Browser } from '@playwright/test';

const BACKEND = 'http://localhost:3016';
const COOKIE = 'app_session';

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
  return { email, password, cookie: match[1]! };
}

async function apiCreatePoll(req: APIRequestContext, cookie: string, slug: string) {
  const res = await req.post(`${BACKEND}/api/polls`, {
    headers: { cookie: `${COOKIE}=${cookie}`, 'content-type': 'application/json' },
    data: {
      slug,
      question: 'What should we ship?',
      options: ['Onboarding', 'Search', 'Reports']
    }
  });
  expect(res.status(), `create poll ${slug}: ${await res.text()}`).toBe(201);
  return res.json();
}

test.describe('polls — auth + live results matrix', () => {
  test('public poll page renders without auth', async ({ page, request }) => {
    const owner = await apiRegister(request);
    const poll = await apiCreatePoll(request, owner.cookie, uniq('pub'));
    await page.goto(`/p/${poll.slug}`);
    await expect(page.locator('h1')).toHaveText(poll.question);
    // Make sure we're not redirected to /login
    expect(page.url()).toContain(`/p/${poll.slug}`);
  });

  test('admin endpoints require auth', async ({ request }) => {
    const endpoints: [string, string][] = [
      ['GET', '/api/polls'],
      ['POST', '/api/polls'],
      ['POST', `/api/polls/${uniq('x')}/close`],
      ['GET', '/api/auth/me']
    ];
    for (const [method, path] of endpoints) {
      const res = await request.fetch(`${BACKEND}${path}`, {
        method,
        data: method === 'POST' ? {} : undefined
      });
      expect(res.status(), `${method} ${path} should require auth`).toBe(401);
    }
  });

  test('public stream + vote: second browser sees the count update', async ({
    browser,
    request
  }) => {
    const owner = await apiRegister(request);
    const poll = await apiCreatePoll(request, owner.cookie, uniq('live'));
    const optionA = poll.options[0].id as string;

    // Open two independent browser contexts (two different "audience members").
    const ctxA = await browser.newContext();
    const ctxB = await browser.newContext();
    const pageA = await ctxA.newPage();
    const pageB = await ctxB.newPage();

    await pageA.goto(`/p/${poll.slug}`);
    await pageB.goto(`/p/${poll.slug}`);
    await expect(pageA.locator('h1')).toBeVisible();
    await expect(pageB.locator('h1')).toBeVisible();

    // Subscribe pageB's EventSource on the client. Wait a beat for the
    // connection to establish before we vote from another origin.
    await pageB.waitForTimeout(500);

    // Pages B votes via UI; A only watches.
    await pageB.locator('button.opt').first().click();
    // The "Thanks" status appears once the vote is recorded on B.
    await expect(pageB.getByRole('status')).toContainText(/thanks/i, { timeout: 5000 });

    // Now Page A should have received an SSE counts event and its first
    // option should display the updated count somehow — but voter view only
    // shows counts after `myVote`. Instead, verify via the public REST GET
    // that the count incremented, plus open the admin stage page (also SSE)
    // and confirm it shows the right total.
    const adminPage = await browser.newContext({
      extraHTTPHeaders: { cookie: `${COOKIE}=${owner.cookie}` }
    }).then((c) => c.newPage());
    await adminPage.goto(`/polls/${poll.slug}`);
    await expect(adminPage.locator('.total')).toContainText(/1 total vote/, { timeout: 5000 });

    // Sanity: REST count for option A is 1.
    const fresh = await request.get(`${BACKEND}/api/polls/${poll.slug}`);
    expect(fresh.status()).toBe(200);
    const body = await fresh.json();
    expect(body.total_votes).toBe(1);
    const opt = body.options.find((o: { id: string }) => o.id === optionA);
    expect(opt.count).toBe(1);

    await ctxA.close();
    await ctxB.close();
  });

  test('double-voting is rejected (409)', async ({ request }) => {
    const owner = await apiRegister(request);
    const poll = await apiCreatePoll(request, owner.cookie, uniq('dbl'));
    const optionId = poll.options[0].id;

    // First vote — anonymous voter context (no cookie).
    const r1 = await request.post(`${BACKEND}/api/polls/${poll.slug}/vote`, {
      headers: {
        'x-forwarded-for': '8.8.8.8',
        'content-type': 'application/json'
      },
      data: { option_id: optionId }
    });
    expect(r1.status()).toBe(201);
    const voter = (r1.headers()['set-cookie'] ?? '').match(/voter=([^;]+)/);
    expect(voter, 'voter cookie should be set on first vote').not.toBeNull();
    const voterCookie = voter![1];

    // Second vote with the same voter cookie + IP — different option.
    const r2 = await request.post(`${BACKEND}/api/polls/${poll.slug}/vote`, {
      headers: {
        'x-forwarded-for': '8.8.8.8',
        cookie: `voter=${voterCookie}`,
        'content-type': 'application/json'
      },
      data: { option_id: poll.options[1].id }
    });
    expect(r2.status()).toBe(409);
  });

  test('QR endpoint returns SVG for an existing poll', async ({ request }) => {
    const owner = await apiRegister(request);
    const poll = await apiCreatePoll(request, owner.cookie, uniq('qr'));
    const res = await request.get(`${BACKEND}/api/polls/${poll.slug}/qr`);
    expect(res.status()).toBe(200);
    expect(res.headers()['content-type']).toMatch(/image\/svg/);
    const body = await res.text();
    expect(body).toMatch(/<svg/);
  });

  test('close endpoint flips is_open and is owner-only', async ({ request }) => {
    const owner = await apiRegister(request);
    const stranger = await apiRegister(request);
    const poll = await apiCreatePoll(request, owner.cookie, uniq('close'));

    // Stranger cannot close.
    const r0 = await request.post(`${BACKEND}/api/polls/${poll.slug}/close`, {
      headers: { cookie: `${COOKIE}=${stranger.cookie}` }
    });
    expect(r0.status()).toBe(404);

    // Owner closes.
    const r1 = await request.post(`${BACKEND}/api/polls/${poll.slug}/close`, {
      headers: { cookie: `${COOKIE}=${owner.cookie}` }
    });
    expect(r1.status()).toBe(204);

    // New vote attempt → 409.
    const r2 = await request.post(`${BACKEND}/api/polls/${poll.slug}/vote`, {
      headers: {
        'x-forwarded-for': '4.4.4.4',
        'content-type': 'application/json'
      },
      data: { option_id: poll.options[0].id }
    });
    expect(r2.status()).toBe(409);
  });

  test('home (/) redirects to /login when unauthenticated', async ({ page }) => {
    await page.goto('/');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
  });

  test('logged-in user sees their poll on the dashboard', async ({
    browser,
    request
  }) => {
    const owner = await apiRegister(request);
    const poll = await apiCreatePoll(request, owner.cookie, uniq('dash'));
    const ctx = await browser.newContext({
      extraHTTPHeaders: { cookie: `${COOKIE}=${owner.cookie}` }
    });
    const page = await ctx.newPage();
    await page.goto('/');
    await expect(page.locator('h1')).toHaveText(/my polls/i);
    await expect(page.locator('.card')).toContainText(poll.question);
    await ctx.close();
  });
});
