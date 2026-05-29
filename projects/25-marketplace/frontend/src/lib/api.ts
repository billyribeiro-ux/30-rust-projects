import type { ApiError, CourseDetail, CourseSummary, FieldError, Instructor, User } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3024';

type FetchLike = typeof fetch;

async function request<T>(fetcher: FetchLike, path: string, init?: RequestInit): Promise<T> {
  const res = await fetcher(`${API_BASE}${path}`, {
    credentials: 'include',
    ...init,
    headers: { 'content-type': 'application/json', accept: 'application/json', ...(init?.headers ?? {}) }
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

export const coursesApi = {
  list: (f: FetchLike) => request<CourseSummary[]>(f, '/api/courses'),
  get: (f: FetchLike, slug: string) =>
    request<CourseDetail>(f, `/api/courses/${encodeURIComponent(slug)}`),
  create: (
    f: FetchLike,
    input: { slug: string; title: string; summary?: string; price_cents: number }
  ) =>
    request<{ id: string }>(f, '/api/courses', {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  publish: (f: FetchLike, slug: string) =>
    request<void>(f, `/api/courses/${encodeURIComponent(slug)}/publish`, { method: 'POST' })
};

export const instructorApi = {
  me: (f: FetchLike) => request<Instructor>(f, '/api/instructor/me'),
  onboard: (f: FetchLike) =>
    request<{ onboarding_url: string }>(f, '/api/instructor/me/onboard', { method: 'POST' })
};

export const enrollApi = {
  checkout: (f: FetchLike, slug: string) =>
    request<{ checkout_url: string }>(f, `/api/enroll/checkout/${encodeURIComponent(slug)}`, {
      method: 'POST'
    })
};
