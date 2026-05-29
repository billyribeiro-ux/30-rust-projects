import { describe, it, expect, vi, beforeEach } from 'vitest';
import { tenantsApi, ticketsApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('helpdesk api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('tenantsApi.list parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 't',
          slug: 'acme',
          name: 'Acme',
          role: 'admin',
          sla_first_response_minutes: 60,
          sla_resolve_minutes: 1440,
          created_at: ''
        }
      ])
    );
    const out = await tenantsApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.slug).toBe('acme');
  });

  it('ticketsApi.list URL-encodes slug + status', async () => {
    const fetcher = vi.fn(async () => jsonResponse([]));
    await ticketsApi.list(fetcher as unknown as typeof fetch, 'a b', 'open');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/t/a%20b/tickets?status=open');
  });

  it('ticketsApi.create posts body', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ id: 't1' }, 201));
    await ticketsApi.create(fetcher as unknown as typeof fetch, 'acme', 'Sub', 'Body', 'urgent');
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ subject: 'Sub', body: 'Body', priority: 'urgent' }));
  });

  it('ticketsApi.postMessage internal=true', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ id: 'm' }, 201));
    await ticketsApi.postMessage(fetcher as unknown as typeof fetch, 'acme', 't1', 'note', true);
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.body).toBe(JSON.stringify({ body: 'note', internal: true }));
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'forbidden', message: 'no', fields: null } }, 403)
    );
    await expect(tenantsApi.list(fetcher as unknown as typeof fetch)).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
