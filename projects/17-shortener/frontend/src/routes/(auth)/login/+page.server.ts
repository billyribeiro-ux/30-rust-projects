import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

const SESSION_COOKIE = 'app_session';
const BACKEND_URL = process.env.VITE_BACKEND_URL ?? 'http://localhost:3015';

/**
 * Login is a two-stage flow when 2FA is on:
 *   stage 1: ?/default — email + password → either logged in OR a pending
 *            intermediate token (5-minute TTL).
 *   stage 2: ?/twoFactor — intermediate + 6-digit code → logged in.
 *
 * The intermediate token is round-tripped through a hidden form field so
 * the frontend doesn't need a separate session/cookie for it. It's already
 * single-use and short-lived in the backend.
 */
export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '')
      .trim()
      .toLowerCase();
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
      if (res.status === 401) {
        return fail(401, { email, error: 'Invalid email or password.' });
      }
      return fail(res.status, { email, error: `Login failed (${res.status}).` });
    }

    const body = (await res.json().catch(() => null)) as
      | { requires_2fa?: boolean; intermediate?: string }
      | null;

    // Stage-1 success on a 2FA-enabled account: backend skipped the session
    // cookie and returned a short-lived intermediate token instead. Render
    // the 2FA prompt; the form will re-POST as ?/twoFactor.
    if (body && body.requires_2fa && body.intermediate) {
      return { requires_2fa: true, intermediate: body.intermediate, email };
    }

    forwardSessionCookie(res, cookies);
    redirect(303, '/');
  },

  twoFactor: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const intermediate = String(data.get('intermediate') ?? '');
    const code = String(data.get('code') ?? '').trim();
    const email = String(data.get('email') ?? '');

    if (!intermediate || !code) {
      return fail(422, {
        requires_2fa: true,
        intermediate,
        email,
        error: 'Enter the 6-digit code from your authenticator.'
      });
    }

    let res: Response;
    try {
      res = await fetch(`${BACKEND_URL}/api/auth/2fa/verify`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ intermediate, code })
      });
    } catch {
      return fail(500, {
        requires_2fa: true,
        intermediate,
        email,
        error: 'Could not reach the server.'
      });
    }

    if (!res.ok) {
      // 401 here = bad code OR expired intermediate. Either way, the user
      // sees a single "invalid code" — same anti-enumeration principle.
      return fail(res.status === 401 ? 401 : res.status, {
        requires_2fa: true,
        intermediate,
        email,
        error: 'Invalid or expired code.'
      });
    }

    forwardSessionCookie(res, cookies);
    redirect(303, '/');
  }
};

function forwardSessionCookie(res: Response, cookies: import('@sveltejs/kit').Cookies) {
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
