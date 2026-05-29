import { describe, it, expect, vi, beforeEach } from 'vitest';
import { jobsApi, queuesApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('jobs api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('queuesApi.list parses the array', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([{ queue: 'default', pending: 2, running: 1, succeeded: 5, failed: 0, dead: 0 }])
    );
    const out = await queuesApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.queue).toBe('default');
  });

  it('jobsApi.list serializes filters', async () => {
    const fetcher = vi.fn(async () => jsonResponse([]));
    await jobsApi.list(fetcher as unknown as typeof fetch, {
      queue: 'critical',
      status: 'dead',
      limit: 100
    });
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('queue=critical');
    expect(url).toContain('status=dead');
    expect(url).toContain('limit=100');
  });

  it('jobsApi.enqueue posts body', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ id: 'j1' }, 201));
    await jobsApi.enqueue(fetcher as unknown as typeof fetch, {
      kind: 'send_email',
      payload: { to: 'a@b.co' }
    });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ kind: 'send_email', payload: { to: 'a@b.co' } }));
  });

  it('jobsApi.retry POSTs to the right URL', async () => {
    const fetcher = vi.fn(async () => new Response(null, { status: 204 }));
    await jobsApi.retry(fetcher as unknown as typeof fetch, 'job 1');
    const [url, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(url).toContain('/api/admin/jobs/job%201/retry');
    expect(init.method).toBe('POST');
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'forbidden', message: 'no', fields: null } }, 403)
    );
    await expect(jobsApi.list(fetcher as unknown as typeof fetch)).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
