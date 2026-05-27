import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

const SESSION_COOKIE = 'app_session';
const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3013';

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '').trim().toLowerCase();
    const password = String(data.get('password') ?? '');
    const name = String(data.get('name') ?? '').trim();

    if (!email) return fail(422, { email, name, error: 'Email is required.' });
    if (password.length < 12) {
      return fail(422, { email, name, error: 'Password must be at least 12 characters.' });
    }

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

    if (res.status === 409) {
      return fail(409, { email, name, error: 'An account with that email already exists.' });
    }
    if (!res.ok) {
      const body = await res.json().catch(() => null);
      const msg = body?.error?.message ?? `Registration failed (${res.status}).`;
      return fail(res.status, { email, name, error: msg });
    }

    // Forward the session cookie so the user is auto-logged-in.
    const header = res.headers.get('set-cookie');
    const m = header?.match(/app_session=([^;]+)/);
    if (m) {
      cookies.set(SESSION_COOKIE, m[1]!, {
        path: '/',
        httpOnly: true,
        sameSite: 'lax',
        secure: process.env.NODE_ENV === 'production',
        maxAge: 60 * 60 * 24 * 30
      });
    }
    await res.text().catch(() => null);
    redirect(303, '/');
  }
};
