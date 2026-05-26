import type { Account, Transaction, ImportResponse, ApiError, AccountKind } from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3006';
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

export type CreateAccountInput = { name: string; kind: AccountKind; color: string };
export type CreateTransactionInput = {
  description: string;
  occurred_at: string;
  postings: { account_id: string; amount_minor: number }[];
};

export const accountsApi = {
  list: (fetcher: FetchLike) => request<Account[]>(fetcher, '/api/accounts'),
  create: (fetcher: FetchLike, input: CreateAccountInput) =>
    request<Account>(fetcher, '/api/accounts', { method: 'POST', body: JSON.stringify(input) }),
  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/accounts/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const transactionsApi = {
  list: (fetcher: FetchLike) => request<Transaction[]>(fetcher, '/api/transactions'),
  create: (fetcher: FetchLike, input: CreateTransactionInput) =>
    request<Transaction>(fetcher, '/api/transactions', {
      method: 'POST',
      body: JSON.stringify(input)
    }),
  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/transactions/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const importsApi = {
  csv: (fetcher: FetchLike, csv: string, asset_account_name: string) =>
    request<ImportResponse>(fetcher, '/api/imports', {
      method: 'POST',
      body: JSON.stringify({ csv, asset_account_name })
    })
};
