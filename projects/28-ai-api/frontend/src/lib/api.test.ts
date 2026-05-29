import { describe, it, expect, vi, beforeEach } from 'vitest';
import { keysApi, usageApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('ai-api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('keysApi.list parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 'k',
          name: 'CI',
          prefix: 'abcd1234',
          scopes: [],
          rpm_limit: 60,
          revoked: false,
          created_at: '',
          last_used_at: null
        }
      ])
    );
    const out = await keysApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.prefix).toBe('abcd1234');
  });

  it('keysApi.create posts body', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ id: 'x', secret: 'sk_live_y', prefix: 'y' }, 201));
    await keysApi.create(fetcher as unknown as typeof fetch, 'app', 100);
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.body).toBe(JSON.stringify({ name: 'app', rpm_limit: 100 }));
  });

  it('keysApi.revoke URL-encodes id', async () => {
    const fetcher = vi.fn(async () => new Response(null, { status: 204 }));
    await keysApi.revoke(fetcher as unknown as typeof fetch, 'k 1');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/keys/k%201/revoke');
  });

  it('usageApi.summary parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        calls_this_month: 12345,
        estimated_cents: 23.4,
        free_tier_calls: 10000,
        per_call_cents: 0.1,
        last_call_at: null
      })
    );
    const out = await usageApi.summary(fetcher as unknown as typeof fetch);
    expect(out.calls_this_month).toBe(12345);
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'forbidden', message: 'no', fields: null } }, 403)
    );
    await expect(keysApi.list(fetcher as unknown as typeof fetch)).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
