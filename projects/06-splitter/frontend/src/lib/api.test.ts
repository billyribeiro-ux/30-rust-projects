import { describe, it, expect, vi } from 'vitest';
import { membersApi, expensesApi, ApiCallError } from './api';
import type { Member } from './types';

function makeFetch(opts: { status: number; body: unknown }) {
  return vi.fn(async () => {
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, {
      status: opts.status,
      headers: new Headers({ 'content-type': 'application/json' })
    });
  });
}

describe('membersApi', () => {
  it('list resolves to array', async () => {
    const sample: Member = { id: 'm1', name: 'A', color: '#4F46E5', created_at: '2026-01-01T00:00:00Z' };
    const fetcher = makeFetch({ status: 200, body: [sample] });
    await expect(membersApi.list(fetcher)).resolves.toEqual([sample]);
  });

  it('create POSTs payload', async () => {
    const sample: Member = { id: 'm1', name: 'A', color: '#4F46E5', created_at: '2026-01-01T00:00:00Z' };
    const fetcher = makeFetch({ status: 201, body: sample });
    await membersApi.create(fetcher, { name: 'A', color: '#4F46E5' });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ name: 'A', color: '#4F46E5' }));
  });

  it('remove returns undefined on 204', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await expect(membersApi.remove(fetcher, 'm1')).resolves.toBeUndefined();
  });
});

describe('expensesApi error surface', () => {
  it('throws ApiCallError with .fields on validation errors', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: {
        error: {
          code: 'validation_failed',
          message: 'validation (fields)',
          fields: [{ field: 'amount_cents', message: 'must be greater than 0' }]
        }
      }
    });
    try {
      await expensesApi.create(fetcher, {
        payer_id: 'm1',
        amount_cents: 0,
        description: '',
        split_kind: 'equal',
        shares: []
      });
      expect.unreachable('should throw');
    } catch (err) {
      expect(err).toBeInstanceOf(ApiCallError);
      const apiErr = err as ApiCallError;
      expect(apiErr.status).toBe(422);
      expect(apiErr.fields).toEqual([{ field: 'amount_cents', message: 'must be greater than 0' }]);
    }
  });
});
