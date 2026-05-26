import type {
  Book,
  Session,
  Highlight,
  BookLookup,
  BookStatus,
  ApiError
} from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3007';
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
  fields: { field: string; message: string }[] | null;
  constructor(message: string, status: number, fields: { field: string; message: string }[] | null) {
    super(message);
    this.status = status;
    this.fields = fields;
  }
}

export type CreateBookInput = {
  isbn?: string | null;
  title: string;
  author?: string;
  cover_url?: string | null;
  pages?: number | null;
  status?: BookStatus;
};

export type UpdateBookInput = Partial<{
  status: BookStatus;
  current_page: number;
  title: string;
  author: string;
  pages: number;
}>;

export const booksApi = {
  list: (f: FetchLike) => request<Book[]>(f, '/api/books'),
  get: (f: FetchLike, id: string) => request<Book>(f, `/api/books/${encodeURIComponent(id)}`),
  create: (f: FetchLike, input: CreateBookInput) =>
    request<Book>(f, '/api/books', { method: 'POST', body: JSON.stringify(input) }),
  update: (f: FetchLike, id: string, input: UpdateBookInput) =>
    request<Book>(f, `/api/books/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(input)
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/books/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const sessionsApi = {
  list: (f: FetchLike, bookId: string) =>
    request<Session[]>(f, `/api/books/${encodeURIComponent(bookId)}/sessions`),
  create: (
    f: FetchLike,
    bookId: string,
    input: { pages_read: number; duration_minutes: number }
  ) =>
    request<Session>(f, `/api/books/${encodeURIComponent(bookId)}/sessions`, {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  remove: (f: FetchLike, bookId: string, id: string) =>
    request<void>(
      f,
      `/api/books/${encodeURIComponent(bookId)}/sessions/${encodeURIComponent(id)}`,
      { method: 'DELETE' }
    )
};

export const highlightsApi = {
  list: (f: FetchLike, bookId: string) =>
    request<Highlight[]>(f, `/api/books/${encodeURIComponent(bookId)}/highlights`),
  create: (
    f: FetchLike,
    bookId: string,
    input: { quote: string; note?: string; page?: number | null }
  ) =>
    request<Highlight>(f, `/api/books/${encodeURIComponent(bookId)}/highlights`, {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  remove: (f: FetchLike, bookId: string, id: string) =>
    request<void>(
      f,
      `/api/books/${encodeURIComponent(bookId)}/highlights/${encodeURIComponent(id)}`,
      { method: 'DELETE' }
    )
};

export const lookupApi = {
  isbn: (f: FetchLike, isbn: string) =>
    request<BookLookup>(f, `/api/lookup/isbn/${encodeURIComponent(isbn)}`)
};
