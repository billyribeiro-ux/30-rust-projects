/**
 * Typed HTTP client for the chat backend.
 *
 * Server-side `load`/actions pass a cookie-forwarding `serverFetch`;
 * the browser uses `credentials: 'include'`.
 */

import type { Message, Room, User, ApiError, FieldError } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3012';

type FetchLike = typeof fetch;

async function request<T>(fetcher: FetchLike, path: string, init?: RequestInit): Promise<T> {
  const res = await fetcher(`${API_BASE}${path}`, {
    credentials: 'include',
    ...init,
    headers: {
      'content-type': 'application/json',
      accept: 'application/json',
      ...(init?.headers ?? {})
    }
  });
  if (res.status === 204) return undefined as T;
  const body = await res.json().catch(() => null);
  if (!res.ok) {
    const err = body as ApiError | null;
    throw new ApiCallError(
      err?.error?.message ?? `request failed: ${res.status}`,
      res.status,
      err?.error?.fields ?? null
    );
  }
  return body as T;
}

export class ApiCallError extends Error {
  status: number;
  fields: FieldError[] | null;
  constructor(message: string, status: number, fields: FieldError[] | null) {
    super(message);
    this.status = status;
    this.fields = fields;
  }
}

export const authApi = {
  register: (f: FetchLike, email: string, password: string, name: string) =>
    request<User>(f, '/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, password, name })
    }),
  login: (f: FetchLike, email: string, password: string) =>
    request<User>(f, '/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password })
    }),
  logout: (f: FetchLike) => request<void>(f, '/api/auth/logout', { method: 'POST' }),
  me: (f: FetchLike) => request<User>(f, '/api/auth/me')
};

export const roomsApi = {
  list: (f: FetchLike) => request<Room[]>(f, '/api/rooms'),
  get: (f: FetchLike, slug: string) => request<Room>(f, `/api/rooms/${encodeURIComponent(slug)}`),
  create: (f: FetchLike, slug: string, name: string) =>
    request<Room>(f, '/api/rooms', { method: 'POST', body: JSON.stringify({ slug, name }) }),
  join: (f: FetchLike, slug: string) =>
    request<Room>(f, `/api/rooms/${encodeURIComponent(slug)}/join`, { method: 'POST' }),
  leave: (f: FetchLike, slug: string) =>
    request<void>(f, `/api/rooms/${encodeURIComponent(slug)}/leave`, { method: 'DELETE' })
};

export const messagesApi = {
  list: (f: FetchLike, slug: string, opts: { before?: string; limit?: number } = {}) => {
    const u = new URLSearchParams();
    if (opts.before) u.set('before', opts.before);
    if (opts.limit) u.set('limit', String(opts.limit));
    const qs = u.toString();
    return request<Message[]>(
      f,
      `/api/rooms/${encodeURIComponent(slug)}/messages${qs ? `?${qs}` : ''}`
    );
  }
};

/** Construct the WebSocket URL from the HTTP API base (browser-only). */
export function wsUrl(slug: string): string {
  const base = API_BASE.replace(/^http/, 'ws');
  return `${base}/api/rooms/${encodeURIComponent(slug)}/ws`;
}
