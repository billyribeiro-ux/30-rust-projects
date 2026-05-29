import type { ApiError, FieldError, Project, Task, Tenant, User } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3029';

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

export const projectsApi = {
  list: (f: FetchLike, tenant_slug: string) =>
    request<Project[]>(f, `/api/t/${encodeURIComponent(tenant_slug)}/projects`),
  create: (f: FetchLike, tenant_slug: string, slug: string, name: string) =>
    request<{ id: string }>(f, `/api/t/${encodeURIComponent(tenant_slug)}/projects`, {
      method: 'POST',
      body: JSON.stringify({ slug, name })
    })
};

export const tasksApi = {
  list: (f: FetchLike, tenant_slug: string, project_id: string) =>
    request<Task[]>(
      f,
      `/api/t/${encodeURIComponent(tenant_slug)}/projects/${encodeURIComponent(project_id)}/tasks`
    ),
  create: (f: FetchLike, tenant_slug: string, project_id: string, title: string, body = '') =>
    request<{ id: string }>(
      f,
      `/api/t/${encodeURIComponent(tenant_slug)}/projects/${encodeURIComponent(project_id)}/tasks`,
      { method: 'POST', body: JSON.stringify({ title, body }) }
    )
};
