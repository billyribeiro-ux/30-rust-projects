import type { ApiError, ArticleFull, ArticleSummary, FieldError, SearchHit, User } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3023';

export function backendUrl(): string {
  return API_BASE;
}

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

export const articlesApi = {
  list: (f: FetchLike) => request<ArticleSummary[]>(f, '/api/articles'),
  get: (f: FetchLike, slug: string) =>
    request<ArticleFull>(f, `/api/articles/${encodeURIComponent(slug)}`)
};

export const searchApi = {
  query: (f: FetchLike, q: string) =>
    request<SearchHit[]>(f, `/api/search?q=${encodeURIComponent(q)}`)
};
