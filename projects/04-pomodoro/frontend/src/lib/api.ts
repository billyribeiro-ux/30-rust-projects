import type { Session, Stats, ApiError, SessionKind } from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3003';

type FetchLike = typeof fetch;

async function request<T>(fetcher: FetchLike, path: string, init?: RequestInit): Promise<T> {
  const res = await fetcher(`${API_BASE}${path}`, {
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
    throw new ApiCallError(err?.error?.message ?? `request failed: ${res.status}`, res.status);
  }

  return body as T;
}

export class ApiCallError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

export type CreateSessionInput = {
  kind: SessionKind;
  label: string | null;
  planned_seconds: number;
  actual_seconds: number;
  started_at: string;
  ended_at: string;
};

export const sessionsApi = {
  list: (fetcher: FetchLike, limit?: number) =>
    request<Session[]>(fetcher, `/api/sessions${limit ? `?limit=${limit}` : ''}`),

  create: (fetcher: FetchLike, input: CreateSessionInput) =>
    request<Session>(fetcher, '/api/sessions', {
      method: 'POST',
      body: JSON.stringify(input)
    }),

  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/sessions/${encodeURIComponent(id)}`, { method: 'DELETE' }),

  stats: (fetcher: FetchLike) => request<Stats>(fetcher, '/api/sessions/stats')
};
