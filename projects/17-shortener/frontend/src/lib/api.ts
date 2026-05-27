/**
 * Typed HTTP client for the shortener backend.
 */

import type { Link, Stats, TwoFASetup, User, ApiError, FieldError } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3015';

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

/** Login can resolve to a full User OR a pending-2FA challenge. */
export type LoginResult =
  | User
  | { requires_2fa: true; intermediate: string };

export const authApi = {
  register: (f: FetchLike, email: string, password: string, name: string) =>
    request<User>(f, '/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, password, name })
    }),
  login: (f: FetchLike, email: string, password: string) =>
    request<LoginResult>(f, '/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password })
    }),
  twoFactorVerify: (f: FetchLike, intermediate: string, code: string) =>
    request<User>(f, '/api/auth/2fa/verify', {
      method: 'POST',
      body: JSON.stringify({ intermediate, code })
    }),
  logout: (f: FetchLike) => request<void>(f, '/api/auth/logout', { method: 'POST' }),
  me: (f: FetchLike) => request<User>(f, '/api/auth/me')
};

export const linksApi = {
  list: (f: FetchLike) => request<Link[]>(f, '/api/links'),
  create: (f: FetchLike, target_url: string, slug?: string) =>
    request<Link>(f, '/api/links', {
      method: 'POST',
      body: JSON.stringify({ target_url, slug: slug || null })
    }),
  stats: (f: FetchLike, slug: string) =>
    request<Stats>(f, `/api/links/${encodeURIComponent(slug)}/stats`)
};

export const twoFAApi = {
  setup: (f: FetchLike) => request<TwoFASetup>(f, '/api/2fa/setup', { method: 'POST' }),
  verify: (f: FetchLike, code: string) =>
    request<{ backup_codes: string[] }>(f, '/api/2fa/verify', {
      method: 'POST',
      body: JSON.stringify({ code })
    }),
  disable: (f: FetchLike, code: string) =>
    request<void>(f, '/api/2fa/disable', {
      method: 'POST',
      body: JSON.stringify({ code })
    })
};
