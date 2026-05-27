/**
 * Server-side fetch wrapper that forwards the session cookie to the
 * cross-origin Rust backend.
 */

const SESSION_COOKIE = 'app_session';

export function serverFetch(cookie: string | null): typeof fetch {
  if (!cookie) return fetch;
  return ((input, init) =>
    fetch(input as RequestInfo, {
      ...init,
      headers: {
        ...(init?.headers ?? {}),
        cookie: `${SESSION_COOKIE}=${cookie}`
      }
    })) as typeof fetch;
}
