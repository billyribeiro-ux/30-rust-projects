/**
 * Typed HTTP client for the finder backend.
 *
 * Same shape as project 15 — the fetcher is passed in so SvelteKit's
 * `event.fetch` is used on SSR (cookies forwarded), while browsers use
 * native `fetch`.
 */

import type { ApiError, FieldError, PlaceDetail, PlaceRow, ReviewRow, User } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3019';

export function backendUrl(): string {
  return API_BASE;
}

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
  me: (f: FetchLike) => request<User>(f, '/api/auth/me'),

  oauthStart: (provider: 'google' | 'github') =>
    `${API_BASE}/api/auth/oauth/${provider}/start`
};

export const placesApi = {
  list: (
    f: FetchLike,
    opts: { lat?: number; lng?: number; radius_m?: number; q?: string; cuisine?: string }
  ) => {
    const u = new URLSearchParams();
    if (opts.lat !== undefined) u.set('lat', String(opts.lat));
    if (opts.lng !== undefined) u.set('lng', String(opts.lng));
    if (opts.radius_m !== undefined) u.set('radius_m', String(opts.radius_m));
    if (opts.q) u.set('q', opts.q);
    if (opts.cuisine) u.set('cuisine', opts.cuisine);
    const qs = u.toString();
    return request<PlaceRow[]>(f, `/api/places${qs ? `?${qs}` : ''}`);
  },
  get: (f: FetchLike, id: string, opts?: { lat?: number; lng?: number }) => {
    const u = new URLSearchParams();
    if (opts?.lat !== undefined) u.set('lat', String(opts.lat));
    if (opts?.lng !== undefined) u.set('lng', String(opts.lng));
    const qs = u.toString();
    return request<PlaceDetail>(f, `/api/places/${encodeURIComponent(id)}${qs ? `?${qs}` : ''}`);
  }
};

export const reviewsApi = {
  upsert: (f: FetchLike, place_id: string, rating: number, body: string) =>
    request<ReviewRow>(f, `/api/reviews/${encodeURIComponent(place_id)}`, {
      method: 'POST',
      body: JSON.stringify({ rating, body })
    })
};
