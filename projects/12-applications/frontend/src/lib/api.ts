/**
 * Typed HTTP client for the applications backend.
 *
 * Same pattern as project 11: server-side `load`/actions pass a
 * cookie-forwarding `serverFetch`; the browser uses `credentials: 'include'`.
 */

import type {
  Application,
  ApplicationEvent,
  AppStatus,
  DashboardData,
  NextStep,
  User,
  ApiError,
  FieldError
} from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3011';

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

// ---- auth ----

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

// ---- applications ----

export type CreateApplicationInput = {
  company: string;
  role: string;
  location?: string;
  salary_min?: number | null;
  salary_max?: number | null;
  job_url?: string;
  notes?: string;
  status?: AppStatus;
};

export type UpdateApplicationInput = Partial<CreateApplicationInput>;

export const applicationsApi = {
  list: (f: FetchLike, opts: { status?: AppStatus } = {}) => {
    const u = new URLSearchParams();
    if (opts.status) u.set('status', opts.status);
    const qs = u.toString();
    return request<Application[]>(f, `/api/applications${qs ? `?${qs}` : ''}`);
  },
  get: (f: FetchLike, id: string) =>
    request<Application>(f, `/api/applications/${encodeURIComponent(id)}`),
  create: (f: FetchLike, input: CreateApplicationInput) =>
    request<Application>(f, '/api/applications', {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  update: (f: FetchLike, id: string, input: UpdateApplicationInput) =>
    request<Application>(f, `/api/applications/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      body: JSON.stringify(input)
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/applications/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const eventsApi = {
  list: (f: FetchLike, applicationId: string) =>
    request<ApplicationEvent[]>(
      f,
      `/api/applications/${encodeURIComponent(applicationId)}/events`
    ),
  createNote: (f: FetchLike, applicationId: string, body: string) =>
    request<ApplicationEvent>(
      f,
      `/api/applications/${encodeURIComponent(applicationId)}/events`,
      { method: 'POST', body: JSON.stringify({ body }) }
    )
};

export const nextStepsApi = {
  list: (f: FetchLike, applicationId: string) =>
    request<NextStep[]>(
      f,
      `/api/applications/${encodeURIComponent(applicationId)}/next-steps`
    ),
  create: (f: FetchLike, applicationId: string, body: string, due_at: string) =>
    request<NextStep>(
      f,
      `/api/applications/${encodeURIComponent(applicationId)}/next-steps`,
      { method: 'POST', body: JSON.stringify({ body, due_at }) }
    ),
  complete: (f: FetchLike, applicationId: string, stepId: string) =>
    request<void>(
      f,
      `/api/applications/${encodeURIComponent(applicationId)}/next-steps/${encodeURIComponent(stepId)}/complete`,
      { method: 'POST' }
    ),
  remove: (f: FetchLike, applicationId: string, stepId: string) =>
    request<void>(
      f,
      `/api/applications/${encodeURIComponent(applicationId)}/next-steps/${encodeURIComponent(stepId)}`,
      { method: 'DELETE' }
    )
};

export const dashboardApi = {
  get: (f: FetchLike) => request<DashboardData>(f, '/api/dashboard')
};
