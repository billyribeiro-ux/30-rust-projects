/**
 * Typed HTTP client for the contacts backend.
 *
 * Two callers:
 *  1. SvelteKit's server-side `load`/actions (running in Node) — they
 *     receive a special `fetch` from SvelteKit that ALREADY forwards the
 *     browser's cookies via `event.fetch`. We always pass that fetch in.
 *  2. The browser — same fetch shape, cookies attached automatically by
 *     the browser as long as `credentials: 'include'` is set AND the
 *     backend CORS allows credentials AND the cookie's domain matches.
 *
 * For server-only fetches (e.g., hooks.server.ts re-fetching `/api/auth/me`
 * to populate locals.user) we hand-thread the cookie header — see
 * `serverFetch` in `$lib/server/api.ts`.
 */

import type {
  Contact,
  DashboardData,
  TagWithCount,
  User,
  ApiError,
  FieldError
} from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3010';

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

// ---- auth ----

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
  me: (f: FetchLike) => request<User>(f, '/api/auth/me'),
  forgot: (f: FetchLike, email: string) =>
    request<void>(f, '/api/auth/forgot', {
      method: 'POST',
      body: JSON.stringify({ email })
    }),
  reset: (f: FetchLike, token: string, password: string) =>
    request<void>(f, `/api/auth/reset/${encodeURIComponent(token)}`, {
      method: 'POST',
      body: JSON.stringify({ password })
    }),
  verify: (f: FetchLike, token: string) =>
    request<void>(f, `/api/auth/verify/${encodeURIComponent(token)}`, { method: 'POST' })
};

// ---- contacts ----

export type CreateContactInput = {
  name: string;
  email?: string;
  phone?: string;
  company?: string;
  notes?: string;
  tags?: string[];
};

export type UpdateContactInput = Partial<CreateContactInput> & {
  touch_last_contacted?: boolean;
};

export const contactsApi = {
  list: (f: FetchLike, opts: { q?: string; tag?: string; include_deleted?: boolean } = {}) => {
    const u = new URLSearchParams();
    if (opts.q) u.set('q', opts.q);
    if (opts.tag) u.set('tag', opts.tag);
    if (opts.include_deleted) u.set('include_deleted', 'true');
    const qs = u.toString();
    return request<Contact[]>(f, `/api/contacts${qs ? `?${qs}` : ''}`);
  },
  get: (f: FetchLike, id: string) =>
    request<Contact>(f, `/api/contacts/${encodeURIComponent(id)}`),
  create: (f: FetchLike, input: CreateContactInput) =>
    request<Contact>(f, '/api/contacts', { method: 'POST', body: JSON.stringify(input) }),
  update: (f: FetchLike, id: string, input: UpdateContactInput) =>
    request<Contact>(f, `/api/contacts/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(input)
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/contacts/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  restore: (f: FetchLike, id: string) =>
    request<void>(f, `/api/contacts/${encodeURIComponent(id)}/restore`, { method: 'POST' }),
  touch: (f: FetchLike, id: string) =>
    request<void>(f, `/api/contacts/${encodeURIComponent(id)}/touch`, { method: 'POST' })
};

export const dashboardApi = {
  get: (f: FetchLike) => request<DashboardData>(f, '/api/dashboard')
};

export const tagsApi = {
  list: (f: FetchLike) => request<TagWithCount[]>(f, '/api/tags')
};
