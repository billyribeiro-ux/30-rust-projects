/**
 * Server-side auth handle. Looks up the session cookie, calls the Rust
 * backend's /api/auth/me, and stashes the result on event.locals.
 */

import type { Handle } from '@sveltejs/kit';
import { authApi, ApiCallError } from '$lib/api';

const SESSION_COOKIE = 'app_session';

export const handle: Handle = async ({ event, resolve }) => {
  const raw = event.cookies.get(SESSION_COOKIE);
  event.locals.sessionCookie = raw ?? null;
  event.locals.user = null;

  if (raw) {
    try {
      const f: typeof fetch = (input, init) =>
        fetch(input as RequestInfo, {
          ...init,
          headers: {
            ...(init?.headers ?? {}),
            cookie: `${SESSION_COOKIE}=${raw}`
          }
        });
      event.locals.user = await authApi.me(f);
    } catch (err) {
      if (err instanceof ApiCallError && err.status === 401) {
        event.cookies.delete(SESSION_COOKIE, { path: '/' });
      } else {
        console.error('hooks.server.ts: /me lookup failed', err);
      }
    }
  }

  return resolve(event);
};
