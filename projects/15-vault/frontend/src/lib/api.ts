/**
 * Typed HTTP client for the vault backend.
 */

import type { ApiError, FieldError, FileRow, Folder, UploadCreated, User } from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3014';

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

export const foldersApi = {
  list: (f: FetchLike) => request<Folder[]>(f, '/api/folders'),
  create: (f: FetchLike, name: string, parent_id: string | null) =>
    request<Folder>(f, '/api/folders', {
      method: 'POST',
      body: JSON.stringify({ name, parent_id })
    }),
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/folders/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const filesApi = {
  list: (f: FetchLike, folder_id: string | null) => {
    const u = new URLSearchParams();
    if (folder_id) u.set('folder_id', folder_id);
    const qs = u.toString();
    return request<FileRow[]>(f, `/api/files${qs ? `?${qs}` : ''}`);
  },
  remove: (f: FetchLike, id: string) =>
    request<void>(f, `/api/files/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  downloadUrl: (id: string) => `${API_BASE}/api/files/${encodeURIComponent(id)}/download`
};

export const uploadsApi = {
  create: (
    f: FetchLike,
    input: { filename: string; size: number; folder_id: string | null; content_type: string }
  ) =>
    request<UploadCreated>(f, '/api/uploads', {
      method: 'POST',
      body: JSON.stringify(input)
    })
};
