/**
 * Typed HTTP client for the polls backend.
 */

import type { Poll, PollSummary, User, ApiError, FieldError } from './types';

export const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3016';

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

export type CreatePollInput = {
  slug: string;
  question: string;
  options: string[];
};

export const pollsApi = {
  list: (f: FetchLike) => request<PollSummary[]>(f, '/api/polls'),
  create: (f: FetchLike, input: CreatePollInput) =>
    request<Poll>(f, '/api/polls', {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  get: (f: FetchLike, slug: string) =>
    request<Poll>(f, `/api/polls/${encodeURIComponent(slug)}`),
  vote: (f: FetchLike, slug: string, optionId: string) =>
    request<unknown>(f, `/api/polls/${encodeURIComponent(slug)}/vote`, {
      method: 'POST',
      body: JSON.stringify({ option_id: optionId })
    }),
  close: (f: FetchLike, slug: string) =>
    request<void>(f, `/api/polls/${encodeURIComponent(slug)}/close`, { method: 'POST' })
};
