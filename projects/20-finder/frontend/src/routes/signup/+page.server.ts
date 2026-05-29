import { fail, redirect, type Cookies } from '@sveltejs/kit';
import type { Actions } from './$types';

const SESSION_COOKIE = 'app_session';
const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3019';

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '').trim().toLowerCase();
    const password = String(data.get('password') ?? '');
    const name = String(data.get('name') ?? '').trim();
    if (!email || !password || !name)
      return fail(422, { email, name, error: 'All fields are required.' });
    if (password.length < 12)
      return fail(422, { email, name, error: 'Password must be at least 12 characters.' });

    let res: Response;
    try {
      res = await fetch(`${BACKEND_URL}/api/auth/register`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email, password, name })
      });
    } catch {
      return fail(500, { email, name, error: 'Could not reach the server.' });
    }
    if (!res.ok) {
      const body = (await res.json().catch(() => null)) as
        | { error?: { message?: string } }
        | null;
      return fail(res.status, {
        email,
        name,
        error: body?.error?.message ?? `Signup failed (${res.status}).`
      });
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
