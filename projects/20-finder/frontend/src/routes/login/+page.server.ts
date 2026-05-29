import { fail, redirect, type Cookies } from '@sveltejs/kit';
import type { Actions } from './$types';

const SESSION_COOKIE = 'app_session';
const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3019';

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '').trim().toLowerCase();
    const password = String(data.get('password') ?? '');
    if (!email || !password) {
      return fail(422, { email, error: 'Email and password are required.' });
    }

    let res: Response;
    try {
      res = await fetch(`${BACKEND_URL}/api/auth/login`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email, password })
      });
    } catch {
      return fail(500, { email, error: 'Could not reach the server.' });
    }
    if (!res.ok) {
      if (res.status === 401) return fail(401, { email, error: 'Invalid email or password.' });
      return fail(res.status, { email, error: `Login failed (${res.status}).` });
    }
    forwardSessionCookie(res, cookies);
    await res.text().catch(() => null);
    redirect(303, '/');
  }
};

function forwardSessionCookie(res: Response, cookies: Cookies) {
  const header = res.headers.get('set-cookie');
  if (!header) return;
  const match = header.match(/app_session=([^;]+)/);
  if (!match) return;
  cookies.set(SESSION_COOKIE, match[1]!, {
    path: '/',
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env.NODE_ENV === 'production',
    maxAge: 60 * 60 * 24 * 30
  });
}
