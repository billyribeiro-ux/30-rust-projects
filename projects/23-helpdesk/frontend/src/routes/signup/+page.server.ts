import { fail, redirect, type Cookies } from '@sveltejs/kit';
import type { Actions } from './$types';

const SESSION_COOKIE = 'app_session';
const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3022';

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '').trim().toLowerCase();
    const password = String(data.get('password') ?? '');
    const name = String(data.get('name') ?? '').trim();
    if (!email || !password || !name)
      return fail(422, { email, name, error: 'All fields required.' });
    if (password.length < 12)
      return fail(422, { email, name, error: 'Password must be ≥ 12 characters.' });
    let res: Response;
    try {
      res = await fetch(`${BACKEND_URL}/api/auth/register`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email, password, name })
      });
    } catch {
      return fail(500, { email, name, error: 'Could not reach server.' });
    }
    if (!res.ok) {
      const body = (await res.json().catch(() => null)) as { error?: { message?: string } } | null;
      return fail(res.status, {
        email,
        name,
        error: body?.error?.message ?? `Signup failed (${res.status}).`
      });
    }
    forward(res, cookies);
    await res.text().catch(() => null);
    redirect(303, '/tenants');
  }
};

function forward(res: Response, cookies: Cookies) {
  const h = res.headers.get('set-cookie');
  if (!h) return;
  const m = h.match(/app_session=([^;]+)/);
  if (!m) return;
  cookies.set(SESSION_COOKIE, m[1]!, {
    path: '/',
    httpOnly: true,
    sameSite: 'lax',
    secure: process.env.NODE_ENV === 'production',
    maxAge: 60 * 60 * 24 * 30
  });
}
