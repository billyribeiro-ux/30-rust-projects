import { test, expect, type Page, type APIRequestContext } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

const BACKEND = process.env.VITE_BACKEND_URL ?? 'http://localhost:3014';
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
  return { email, password, cookie: match[1]! };
}

test.describe('vault: chunked upload + dedup + auth', () => {
  test('home redirects to /vault then /login when unauthenticated', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');
    expect(page.url()).toMatch(/\/login(?:\?|$)/);
  });

  test('signup → vault page is reachable and axe-clean', async ({ page }) => {
    await gotoHydrated(page, '/signup');
    const signupAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(signupAxe.violations, JSON.stringify(signupAxe.violations, null, 2)).toEqual([]);

    const email = `${uniq('e2e')}@example.com`;
    await page.getByLabel(/^email/i).fill(email);
    await page.getByLabel(/^password/i).fill('correct horse battery staple');
    await page.getByRole('button', { name: /create account/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/vault');

    const vaultAxe = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(vaultAxe.violations, JSON.stringify(vaultAxe.violations, null, 2)).toEqual([]);
  });

  test('upload chunked file via API and see it in the list', async ({ page, request }) => {
    const a = await apiRegister(request);

    // Create an upload session.
    const filename = `${uniq('hello')}.txt`;
    const payload = 'hello, vault! '.repeat(50); // ~700 bytes
    const buf = Buffer.from(payload);
    const create = await request.post(`${BACKEND}/api/uploads`, {
      headers: { cookie: `${COOKIE}=${a.cookie}`, 'content-type': 'application/json' },
      data: { filename, size: buf.length, folder_id: null, content_type: 'text/plain' }
    });
    expect(create.status()).toBe(201);
    const { upload_id } = await create.json();

    // Split into two chunks and PATCH each.
    const half = Math.floor(buf.length / 2);
    const c1 = buf.subarray(0, half);
    const c2 = buf.subarray(half);

    const p1 = await request.patch(`${BACKEND}/api/uploads/${upload_id}`, {
      headers: {
        cookie: `${COOKIE}=${a.cookie}`,
        'content-type': 'application/offset+octet-stream',
        'upload-offset': '0'
      },
      data: c1
    });
    expect(p1.status()).toBe(204);
    expect(p1.headers()['upload-offset']).toBe(String(c1.length));

    const p2 = await request.patch(`${BACKEND}/api/uploads/${upload_id}`, {
      headers: {
        cookie: `${COOKIE}=${a.cookie}`,
        'content-type': 'application/offset+octet-stream',
        'upload-offset': String(c1.length)
      },
      data: c2
    });
    expect(p2.status()).toBe(201);

    // It should be listed.
    const list = await request.get(`${BACKEND}/api/files`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(list.status()).toBe(200);
    const files = await list.json();
    const match = files.find((f: { name: string }) => f.name === filename);
    expect(match).toBeTruthy();
    expect(match.size).toBe(buf.length);

    // Download and verify bytes.
    const dl = await request.get(`${BACKEND}/api/files/${match.id}/download`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    expect(dl.status()).toBe(200);
    const got = await dl.body();
    expect(got.equals(buf)).toBe(true);

    // Sign in as the user via the UI; the file shows up in the list.
    await gotoHydrated(page, '/login');
    await page.getByLabel(/^email/i).fill(a.email);
    await page.getByLabel(/^password/i).fill(a.password);
    await page.getByRole('button', { name: /sign in/i }).click();
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveURL('/vault');
    await expect(page.getByRole('link', { name: filename })).toBeVisible();
  });

  test('dedup: same bytes uploaded twice share a single blob row', async ({ request }) => {
    const a = await apiRegister(request);
    const data = Buffer.from('the same bytes ' + Math.random().toString(36));

    async function upload(filename: string): Promise<string> {
      const c = await request.post(`${BACKEND}/api/uploads`, {
        headers: { cookie: `${COOKIE}=${a.cookie}`, 'content-type': 'application/json' },
        data: { filename, size: data.length, folder_id: null, content_type: 'text/plain' }
      });
      expect(c.status()).toBe(201);
      const { upload_id } = await c.json();
      const p = await request.patch(`${BACKEND}/api/uploads/${upload_id}`, {
        headers: {
          cookie: `${COOKIE}=${a.cookie}`,
          'content-type': 'application/offset+octet-stream',
          'upload-offset': '0'
        },
        data
      });
      expect(p.status()).toBe(201);
      return upload_id;
    }
    await upload(`a-${uniq()}.txt`);
    await upload(`b-${uniq()}.txt`);

    const list = await request.get(`${BACKEND}/api/files`, {
      headers: { cookie: `${COOKIE}=${a.cookie}` }
    });
    const files = (await list.json()) as Array<{ sha256: string }>;
    const recents = files.slice(-2);
    expect(recents.length).toBe(2);
    expect(recents[0]!.sha256).toBe(recents[1]!.sha256);
  });

  test('unauthenticated upload requests are 401', async ({ request }) => {
    const res = await request.post(`${BACKEND}/api/uploads`, {
      data: { filename: 'x.txt', size: 10, folder_id: null, content_type: 'text/plain' }
    });
    expect(res.status()).toBe(401);
  });
});
