/**
 * Typed HTTP client for the kanban backend.
 *
 * Pattern reused verbatim from project 15 (vault). The fetcher is always
 * passed in so `+page.server.ts` can use SvelteKit's `event.fetch`
 * (which forwards cookies on SSR), while the browser uses native `fetch`.
 */

import type {
  ApiError,
  BoardFull,
  BoardSummary,
  CardRow,
  Comment,
  FieldError,
  ListRow,
  MemberRow,
  Role,
  User
} from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3018';

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
  me: (f: FetchLike) => request<User>(f, '/api/auth/me')
};

export const boardsApi = {
  list: (f: FetchLike) => request<BoardSummary[]>(f, '/api/boards'),
  create: (f: FetchLike, name: string, slug: string) =>
    request<BoardSummary>(f, '/api/boards', {
      method: 'POST',
      body: JSON.stringify({ name, slug })
    }),
  get: (f: FetchLike, slug: string) =>
    request<BoardFull>(f, `/api/boards/${encodeURIComponent(slug)}`),
  rename: (f: FetchLike, id: string, name: string) =>
    request<BoardSummary>(f, `/api/boards/${encodeURIComponent(id)}/rename`, {
      method: 'PATCH',
      body: JSON.stringify({ name })
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/boards/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  members: (f: FetchLike, board_id: string) =>
    request<MemberRow[]>(f, `/api/boards/${encodeURIComponent(board_id)}/memberships`),
  upsertMember: (f: FetchLike, board_id: string, email: string, role: Role) =>
    request<MemberRow>(f, `/api/boards/${encodeURIComponent(board_id)}/memberships`, {
      method: 'POST',
      body: JSON.stringify({ email, role })
    }),
  removeMember: (f: FetchLike, board_id: string, user_id: string) =>
    request<void>(
      f,
      `/api/boards/${encodeURIComponent(board_id)}/memberships/${encodeURIComponent(user_id)}`,
      { method: 'DELETE' }
    ),
  createList: (f: FetchLike, board_id: string, name: string, position: number) =>
    request<ListRow>(f, `/api/boards/${encodeURIComponent(board_id)}/lists`, {
      method: 'POST',
      body: JSON.stringify({ name, position })
    })
};

export const listsApi = {
  update: (
    f: FetchLike,
    id: string,
    patch: Partial<{ name: string; position: number }>
  ) =>
    request<ListRow>(f, `/api/lists/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(patch)
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/lists/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  createCard: (f: FetchLike, list_id: string, title: string, position: number) =>
    request<CardRow>(f, `/api/lists/${encodeURIComponent(list_id)}/cards`, {
      method: 'POST',
      body: JSON.stringify({ title, position })
    })
};

export const cardsApi = {
  update: (
    f: FetchLike,
    id: string,
    patch: Partial<{
      title: string;
      body: string;
      list_id: string;
      position: number;
      due_at: string | null;
    }>
  ) =>
    request<CardRow>(f, `/api/cards/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(patch)
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/cards/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  comments: (f: FetchLike, id: string) =>
    request<Comment[]>(f, `/api/cards/${encodeURIComponent(id)}/comments`),
  addComment: (f: FetchLike, id: string, body: string) =>
    request<Comment>(f, `/api/cards/${encodeURIComponent(id)}/comments`, {
      method: 'POST',
      body: JSON.stringify({ body })
    })
};
