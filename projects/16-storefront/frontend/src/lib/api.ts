/**
 * Typed HTTP client for the storefront backend.
 *
 * The pattern is the same as projects 11-14: a single `request<T>()` shim
 * that handles JSON encoding, error mapping, and the 204 short-circuit.
 * Specific resource clients (`productsApi`, `checkoutApi`, etc.) live
 * underneath.
 */

import type {
  AdminProduct,
  AdminUser,
  ApiError,
  CheckoutResponse,
  FieldError,
  Order,
  Product
} from './types';

const API_BASE =
  (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3015';

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

export const productsApi = {
  list: (f: FetchLike) => request<Product[]>(f, '/api/products'),
  get: (f: FetchLike, id: string) =>
    request<Product>(f, `/api/products/${encodeURIComponent(id)}`)
};

export const checkoutApi = {
  start: (f: FetchLike, email: string, product_id: string) =>
    request<CheckoutResponse>(f, '/api/checkout', {
      method: 'POST',
      body: JSON.stringify({ email, product_id })
    })
};

export const authApi = {
  login: (f: FetchLike, email: string, password: string) =>
    request<AdminUser>(f, '/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password })
    }),
  logout: (f: FetchLike) => request<void>(f, '/api/auth/logout', { method: 'POST' }),
  me: (f: FetchLike) => request<AdminUser>(f, '/api/auth/me'),
  register: (f: FetchLike, email: string, password: string, name: string) =>
    request<AdminUser>(f, '/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, password, name })
    })
};

export const adminApi = {
  listProducts: (f: FetchLike) => request<AdminProduct[]>(f, '/api/admin/products'),
  createProduct: (
    f: FetchLike,
    input: {
      sku: string;
      name: string;
      description: string;
      price_cents: number;
      currency: string;
    }
  ) =>
    request<AdminProduct>(f, '/api/admin/products', {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  uploadFile: (f: FetchLike, id: string, file_name: string, content_base64: string) =>
    request<AdminProduct>(f, `/api/admin/products/${encodeURIComponent(id)}/file`, {
      method: 'POST',
      body: JSON.stringify({ file_name, content_base64 })
    }),
  listOrders: (f: FetchLike) => request<Order[]>(f, '/api/admin/orders'),
  refundOrder: (f: FetchLike, id: string) =>
    request<Order>(f, `/api/admin/orders/${encodeURIComponent(id)}/refund`, {
      method: 'POST'
    })
};
