import type {
  ApiError,
  FieldError,
  JobOut,
  QueueSummary,
  User
} from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3021';

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

export const queuesApi = {
  list: (f: FetchLike) => request<QueueSummary[]>(f, '/api/admin/queues')
};

export const jobsApi = {
  list: (f: FetchLike, opts: { queue?: string; status?: string; limit?: number } = {}) => {
    const u = new URLSearchParams();
    if (opts.queue) u.set('queue', opts.queue);
    if (opts.status) u.set('status', opts.status);
    if (opts.limit) u.set('limit', String(opts.limit));
    const qs = u.toString();
    return request<JobOut[]>(f, `/api/admin/jobs${qs ? `?${qs}` : ''}`);
  },
  get: (f: FetchLike, id: string) =>
    request<JobOut>(f, `/api/admin/jobs/${encodeURIComponent(id)}`),
  enqueue: (
    f: FetchLike,
    input: {
      queue?: string;
      kind: string;
      payload?: Record<string, unknown>;
      max_attempts?: number;
    }
  ) =>
    request<{ id: string }>(f, '/api/admin/jobs', {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  retry: (f: FetchLike, id: string) =>
    request<void>(f, `/api/admin/jobs/${encodeURIComponent(id)}/retry`, { method: 'POST' })
};

export const streamUrl = () => `${API_BASE}/api/stream/jobs`;
