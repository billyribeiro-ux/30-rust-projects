import { describe, it, expect, vi, beforeEach } from 'vitest';
import { kpisApi, ingestApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('analytics api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('kpisApi.snapshot parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        events_last_minute: 1,
        events_last_hour: 2,
        unique_sessions_last_hour: 1,
        top_kinds: [{ kind: 'click', count: 1 }],
        ts: '2026-05-27T00:00:00Z'
      })
    );
    const out = await kpisApi.snapshot(fetcher as unknown as typeof fetch);
    expect(out.events_last_hour).toBe(2);
  });

  it('kpisApi.streamUrl points at /api/kpis/stream', () => {
    expect(kpisApi.streamUrl()).toMatch(/\/api\/kpis\/stream$/);
  });

  it('ingestApi.send posts kind + payload', async () => {
    const fetcher = vi.fn(async () => new Response(null, { status: 204 }));
    await ingestApi.send(fetcher as unknown as typeof fetch, 'click', { x: 1 }, 's1');
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.body).toBe(JSON.stringify({ kind: 'click', payload: { x: 1 }, session_id: 's1' }));
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'forbidden', message: 'x', fields: null } }, 403)
    );
    await expect(kpisApi.snapshot(fetcher as unknown as typeof fetch)).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
