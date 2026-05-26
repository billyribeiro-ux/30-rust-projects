/**
 * Server-side fetch wrapper that forwards the session cookie to the
 * cross-origin Rust backend.
 *
 * Use from +page.server.ts / +layout.server.ts / actions where you need
 * authenticated calls to the backend. The hooks.server.ts populates
 * `locals.sessionCookie` with the raw cookie value.
 *
 * Usage:
 *   const data = await contactsApi.list(serverFetch(locals.sessionCookie));
 */

const SESSION_COOKIE = 'contacts_session';

export function serverFetch(cookie: string | null): typeof fetch {
  if (!cookie) {
    // Still need to return a fetch — the backend will return 401 for
    // authenticated routes, which the caller can handle.
    return fetch;
  }
  return ((input, init) =>
    fetch(input as RequestInfo, {
      ...init,
      headers: {
        ...(init?.headers ?? {}),
        cookie: `${SESSION_COOKIE}=${cookie}`
      }
    })) as typeof fetch;
}
