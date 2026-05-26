/**
 * Server-side auth handle.
 *
 * Every HTTP request to SvelteKit passes through `handle`. We:
 *   1. Pull the session cookie out of the incoming request.
 *   2. If present, call the Rust backend's /api/auth/me to look up the user.
 *   3. Stash the result on event.locals.user (typed in src/app.d.ts).
 *   4. Resolve as usual.
 *
 * The `event.fetch` provided by SvelteKit is a special fetcher that:
 *   - Forwards the browser's cookies to upstream calls (so SSR fetches
 *     see the same auth state the browser does).
 *   - Routes to internal SvelteKit endpoints without going through the
 *     network (not relevant here — our backend is in a separate process).
 *
 * For OUR cross-origin call to the Rust backend, we manually attach the
 * session cookie via the `cookie` header. That's because `event.fetch`
 * only forwards cookies for SAME-ORIGIN destinations.
 */

import type { Handle } from '@sveltejs/kit';
import { authApi, ApiCallError } from '$lib/api';

const SESSION_COOKIE = 'contacts_session';

export const handle: Handle = async ({ event, resolve }) => {
  const raw = event.cookies.get(SESSION_COOKIE);
  event.locals.sessionCookie = raw ?? null;
  event.locals.user = null;

  if (raw) {
    try {
      // Build a fetch that forwards our session cookie to the cross-origin
      // backend. event.fetch won't do this automatically because the backend
      // lives on a different port.
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
        // Stale/invalid cookie — clear it so the browser stops sending it.
        event.cookies.delete(SESSION_COOKIE, { path: '/' });
      } else {
        console.error('hooks.server.ts: /me lookup failed', err);
      }
    }
  }

  return resolve(event);
};
