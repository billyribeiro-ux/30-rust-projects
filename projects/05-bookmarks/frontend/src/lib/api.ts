import type { Bookmark, TagWithCount, ApiError } from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3004';

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

export type CreateBookmarkInput = {
  url: string;
  title: string;
  description: string;
  tags: string[];
};

export const bookmarksApi = {
  list: (fetcher: FetchLike, params: { q?: string; tag?: string } = {}) => {
    const usp = new URLSearchParams();
    if (params.q) usp.set('q', params.q);
    if (params.tag) usp.set('tag', params.tag);
    const qs = usp.toString();
    return request<Bookmark[]>(fetcher, `/api/bookmarks${qs ? `?${qs}` : ''}`);
  },

  create: (fetcher: FetchLike, input: CreateBookmarkInput) =>
    request<Bookmark>(fetcher, '/api/bookmarks', {
      method: 'POST',
      body: JSON.stringify(input)
    }),

  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/bookmarks/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const tagsApi = {
  list: (fetcher: FetchLike) => request<TagWithCount[]>(fetcher, '/api/tags')
};
