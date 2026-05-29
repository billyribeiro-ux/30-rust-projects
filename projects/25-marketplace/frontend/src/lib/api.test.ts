import { describe, it, expect, vi, beforeEach } from 'vitest';
import { coursesApi, enrollApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('marketplace api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('coursesApi.list parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 'c',
          slug: 's',
          title: 'T',
          summary: '',
          price_cents: 4900,
          currency: 'usd',
          status: 'published',
          instructor_name: 'I',
          created_at: ''
        }
      ])
    );
    const out = await coursesApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.price_cents).toBe(4900);
  });

  it('coursesApi.create POSTs body', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ id: 'x' }, 201));
    await coursesApi.create(fetcher as unknown as typeof fetch, {
      slug: 'rust-101',
      title: 'Rust',
      price_cents: 4900
    });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toContain('rust-101');
  });

  it('enrollApi.checkout URL-encodes slug', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ checkout_url: 'https://stripe.example' }));
    await enrollApi.checkout(fetcher as unknown as typeof fetch, 'a b');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/enroll/checkout/a%20b');
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'not_found', message: 'x', fields: null } }, 404)
    );
    await expect(coursesApi.get(fetcher as unknown as typeof fetch, 'x')).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
