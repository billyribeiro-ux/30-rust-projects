import type { Habit, ToggleResult, ApiError } from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3002';

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

export const habitsApi = {
  list: (fetcher: FetchLike) => request<Habit[]>(fetcher, '/api/habits'),

  create: (fetcher: FetchLike, name: string, color: string) =>
    request<Habit>(fetcher, '/api/habits', {
      method: 'POST',
      body: JSON.stringify({ name, color })
    }),

  update: (fetcher: FetchLike, id: string, patch: { name?: string; color?: string }) =>
    request<Habit>(fetcher, `/api/habits/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(patch)
    }),

  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/habits/${encodeURIComponent(id)}`, { method: 'DELETE' }),

  toggle: (fetcher: FetchLike, id: string, date: string) =>
    request<ToggleResult>(fetcher, `/api/habits/${encodeURIComponent(id)}/completions`, {
      method: 'POST',
      body: JSON.stringify({ date })
    })
};
