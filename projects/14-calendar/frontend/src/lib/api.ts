/**
 * Typed HTTP client for the calendar backend.
 */

import type { Calendar, Occurrence, User, ApiError, FieldError } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3013';

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

export const calendarsApi = {
  list: (f: FetchLike) => request<Calendar[]>(f, '/api/calendars'),
  create: (f: FetchLike, name: string, color?: string, default_tz?: string) =>
    request<Calendar>(f, '/api/calendars', {
      method: 'POST',
      body: JSON.stringify({ name, color, default_tz })
    })
};

export type CreateEventInput = {
  calendar_id: string;
  title: string;
  description?: string;
  location?: string;
  start_at: string;
  end_at: string;
  tz?: string;
  rrule?: string | null;
  all_day?: boolean;
};

export const eventsApi = {
  range: (f: FetchLike, from: string, to: string, calendarId?: string) => {
    const u = new URLSearchParams({ from, to });
    if (calendarId) u.set('calendar_id', calendarId);
    return request<Occurrence[]>(f, `/api/events?${u.toString()}`);
  },
  create: (f: FetchLike, input: CreateEventInput) =>
    request<unknown>(f, '/api/events', {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/events/${encodeURIComponent(id)}`, { method: 'DELETE' })
};
