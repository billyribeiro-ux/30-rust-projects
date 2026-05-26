import { describe, it, expect, vi } from 'vitest';
import { accountsApi, transactionsApi, ApiCallError } from './api';
import type { Account, Transaction } from './types';

function makeFetch(opts: { status: number; body: unknown }) {
  return vi.fn(async () => {
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, {
      status: opts.status,
      headers: new Headers({ 'content-type': 'application/json' })
    });
  });
}

describe('accountsApi', () => {
  it('list resolves', async () => {
    const a: Account = {
      id: 'a1',
      name: 'Checking',
      kind: 'asset',
      color: '#4F46E5',
      created_at: '2026-01-01T00:00:00Z',
      balance_minor: 0
    };
    const fetcher = makeFetch({ status: 200, body: [a] });
    await expect(accountsApi.list(fetcher)).resolves.toEqual([a]);
  });

  it('create POSTs payload', async () => {
    const a: Account = {
      id: 'a1',
      name: 'Checking',
      kind: 'asset',
      color: '#4F46E5',
      created_at: '2026-01-01T00:00:00Z',
      balance_minor: 0
    };
    const fetcher = makeFetch({ status: 201, body: a });
    await accountsApi.create(fetcher, { name: 'Checking', kind: 'asset', color: '#4F46E5' });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(
      JSON.stringify({ name: 'Checking', kind: 'asset', color: '#4F46E5' })
    );
  });
});

describe('transactionsApi error surface', () => {
  it('field-level errors land on ApiCallError.fields', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: {
        error: {
          code: 'validation_failed',
          message: 'validation (fields)',
          fields: [{ field: 'postings', message: 'debits must equal credits' }]
        }
      }
    });
    const sample: Transaction = {
      id: '',
      description: '',
      occurred_at: '2026-01-01T00:00:00Z',
      created_at: '2026-01-01T00:00:00Z',
      postings: []
    };
    try {
      await transactionsApi.create(fetcher, {
        description: sample.description,
        occurred_at: sample.occurred_at,
        postings: []
      });
      expect.unreachable();
    } catch (err) {
      expect(err).toBeInstanceOf(ApiCallError);
      const apiErr = err as ApiCallError;
      expect(apiErr.fields).toEqual([
        { field: 'postings', message: 'debits must equal credits' }
      ]);
    }
  });
});
