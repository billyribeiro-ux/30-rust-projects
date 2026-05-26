import type { Todo, ApiError } from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3000';

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

  if (res.status === 204) {
    return undefined as T;
  }

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

export const todosApi = {
  list: (fetcher: FetchLike) => request<Todo[]>(fetcher, '/api/todos'),

  create: (fetcher: FetchLike, title: string) =>
    request<Todo>(fetcher, '/api/todos', {
      method: 'POST',
      body: JSON.stringify({ title })
    }),

  update: (fetcher: FetchLike, id: string, patch: Partial<Pick<Todo, 'title' | 'done'>>) =>
    request<Todo>(fetcher, `/api/todos/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(patch)
    }),

  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/todos/${encodeURIComponent(id)}`, { method: 'DELETE' })
};
