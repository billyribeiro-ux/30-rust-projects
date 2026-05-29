import type { ApiError, FieldError, Message, Tenant, Ticket, User } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3022';

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

export const tenantsApi = {
  list: (f: FetchLike) => request<Tenant[]>(f, '/api/tenants'),
  create: (f: FetchLike, slug: string, name: string) =>
    request<Tenant>(f, '/api/tenants', {
      method: 'POST',
      body: JSON.stringify({ slug, name })
    })
};

export const ticketsApi = {
  list: (f: FetchLike, tenant_slug: string, status = 'open') =>
    request<Ticket[]>(
      f,
      `/api/t/${encodeURIComponent(tenant_slug)}/tickets?status=${encodeURIComponent(status)}`
    ),
  create: (f: FetchLike, tenant_slug: string, subject: string, body: string, priority = 'normal') =>
    request<{ id: string }>(f, `/api/t/${encodeURIComponent(tenant_slug)}/tickets`, {
      method: 'POST',
      body: JSON.stringify({ subject, body, priority })
    }),
  get: (f: FetchLike, tenant_slug: string, id: string) =>
    request<{ ticket: Ticket; messages: Message[] }>(
      f,
      `/api/t/${encodeURIComponent(tenant_slug)}/tickets/${encodeURIComponent(id)}`
    ),
  postMessage: (
    f: FetchLike,
    tenant_slug: string,
    id: string,
    body: string,
    internal = false
  ) =>
    request<{ id: string }>(
      f,
      `/api/t/${encodeURIComponent(tenant_slug)}/tickets/${encodeURIComponent(id)}/messages`,
      { method: 'POST', body: JSON.stringify({ body, internal }) }
    )
};

export const magicApi = {
  start: (f: FetchLike, tenant_slug: string, email: string) =>
    request<void>(f, '/api/magic/start', {
      method: 'POST',
      body: JSON.stringify({ tenant_slug, email })
    })
};
