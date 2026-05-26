import type {
  Member,
  Expense,
  BalancesResponse,
  ApiError,
  SplitKind,
  ShareInput
} from './types';

const API_BASE = (import.meta.env.VITE_BACKEND_URL as string | undefined) ?? 'http://localhost:3005';

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

export type CreateMemberInput = { name: string; color: string };

export type CreateExpenseInput = {
  payer_id: string;
  amount_cents: number;
  description: string;
  split_kind: SplitKind;
  shares: ShareInput[];
};

export const membersApi = {
  list: (fetcher: FetchLike) => request<Member[]>(fetcher, '/api/members'),

  create: (fetcher: FetchLike, input: CreateMemberInput) =>
    request<Member>(fetcher, '/api/members', {
      method: 'POST',
      body: JSON.stringify(input)
    }),

  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/members/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const expensesApi = {
  list: (fetcher: FetchLike) => request<Expense[]>(fetcher, '/api/expenses'),

  create: (fetcher: FetchLike, input: CreateExpenseInput) =>
    request<Expense>(fetcher, '/api/expenses', {
      method: 'POST',
      body: JSON.stringify(input)
    }),

  remove: (fetcher: FetchLike, id: string) =>
    request<void>(fetcher, `/api/expenses/${encodeURIComponent(id)}`, { method: 'DELETE' })
};

export const balancesApi = {
  get: (fetcher: FetchLike) => request<BalancesResponse>(fetcher, '/api/balances')
};
