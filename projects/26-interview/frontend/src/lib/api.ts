import type { ApiError, ExecutionOut, FieldError, Interview, User } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3025';

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

export const interviewsApi = {
  list: (f: FetchLike) => request<Interview[]>(f, '/api/interviews'),
  get: (f: FetchLike, id: string) =>
    request<Interview>(f, `/api/interviews/${encodeURIComponent(id)}`),
  create: (f: FetchLike, candidate_email: string, language: string = 'rust') =>
    request<{ id: string }>(f, '/api/interviews', {
      method: 'POST',
      body: JSON.stringify({ candidate_email, language })
    }),
  setState: (f: FetchLike, id: string, status: 'live' | 'ended') =>
    request<void>(f, `/api/interviews/${encodeURIComponent(id)}/state`, {
      method: 'POST',
      body: JSON.stringify({ status })
    })
};

export const execApi = {
  run: (f: FetchLike, interview_id: string, language: string, code: string) =>
    request<ExecutionOut>(f, `/api/exec/${encodeURIComponent(interview_id)}/execute`, {
      method: 'POST',
      body: JSON.stringify({ language, code })
    })
};

export const wsUrl = (interview_id: string) =>
  `${API_BASE.replace(/^http/, 'ws')}/ws/interview/${encodeURIComponent(interview_id)}`;
