/**
 * Unit tests for the API client. We pass a mocked `fetch` to each call
 * and assert two things:
 *  1) the right URL + method + body went out
 *  2) errors are mapped to ApiCallError with the right status & fields
 *
 * The strict-types fetcher dance (see PATTERNS.md) requires a double cast
 * through `unknown` because Vitest's mock.calls typing collides with
 * `noUncheckedIndexedAccess` in our tsconfig.
 */
import { describe, expect, it, vi } from 'vitest';
import { productsApi, checkoutApi, ApiCallError } from './api';

function okResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

function errResponse(body: unknown, status: number): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('productsApi.list', () => {
  it('fetches /api/products and returns the array', async () => {
    const fetcher = vi.fn(async () => okResponse([{ id: 'p1', name: 'P1' }]));
    const result = await productsApi.list(fetcher as unknown as typeof fetch);
    expect(result).toEqual([{ id: 'p1', name: 'P1' }]);
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(url).toMatch(/\/api\/products$/);
  });

  it('maps 500 into ApiCallError with status and message', async () => {
    const fetcher = vi.fn(async () =>
      errResponse({ error: { code: 'internal_error', message: 'boom', fields: null } }, 500)
    );
    await expect(productsApi.list(fetcher as unknown as typeof fetch)).rejects.toMatchObject({
      status: 500,
      message: 'boom'
    });
  });
});

describe('checkoutApi.start', () => {
  it('POSTs JSON body and returns the URL', async () => {
    const fetcher = vi.fn(async () =>
      okResponse({ url: 'https://stripe', session_id: 'cs_1', order_id: 'o_1' })
    );
    const r = await checkoutApi.start(fetcher as unknown as typeof fetch, 'a@b.com', 'p_1');
    expect(r.session_id).toBe('cs_1');
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ email: 'a@b.com', product_id: 'p_1' }));
  });

  it('surfaces validation errors with their fields[]', async () => {
    const fetcher = vi.fn(async () =>
      errResponse(
        {
          error: {
            code: 'validation_failed',
            message: 'validation (fields)',
            fields: [{ field: 'email', message: 'required' }]
          }
        },
        422
      )
    );
    try {
      await checkoutApi.start(fetcher as unknown as typeof fetch, '', 'p');
      expect.unreachable('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(ApiCallError);
      const err = e as ApiCallError;
      expect(err.status).toBe(422);
      expect(err.fields?.[0]?.field).toBe('email');
    }
  });
});
