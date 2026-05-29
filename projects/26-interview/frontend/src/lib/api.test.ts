import { describe, it, expect, vi, beforeEach } from 'vitest';
import { interviewsApi, execApi, wsUrl, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('interview api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('interviewsApi.list parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 'i',
          interviewer_id: 'u',
          candidate_email: 'c@example.com',
          status: 'scheduled',
          code: '',
          language: 'rust',
          started_at: null,
          ended_at: null,
          created_at: ''
        }
      ])
    );
    const out = await interviewsApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.language).toBe('rust');
  });

  it('interviewsApi.create posts body', async () => {
    const fetcher = vi.fn(async () => jsonResponse({ id: 'new' }, 201));
    await interviewsApi.create(fetcher as unknown as typeof fetch, 'c@example.com', 'python');
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.body).toBe(
      JSON.stringify({ candidate_email: 'c@example.com', language: 'python' })
    );
  });

  it('execApi.run encodes interview id', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ id: 'e', stdout: '', stderr: '', exit_code: 0, duration_ms: 1 }, 201)
    );
    await execApi.run(fetcher as unknown as typeof fetch, 'i 1', 'rust', 'code');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/exec/i%201/execute');
  });

  it('wsUrl swaps http for ws', () => {
    expect(wsUrl('xyz')).toMatch(/^ws:\/\/.*\/ws\/interview\/xyz$/);
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'forbidden', message: 'x', fields: null } }, 403)
    );
    await expect(
      interviewsApi.list(fetcher as unknown as typeof fetch)
    ).rejects.toBeInstanceOf(ApiCallError);
  });
});
