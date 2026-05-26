import { describe, it, expect, vi } from 'vitest';
import { sessionsApi, ApiCallError } from './api';
import type { Session, Stats } from './types';

function makeFetch(opts: { status: number; body: unknown; headers?: Record<string, string> }) {
  return vi.fn(async () => {
    const headers = new Headers({ 'content-type': 'application/json', ...(opts.headers ?? {}) });
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, { status: opts.status, headers });
  });
}

const sample: Session = {
  id: 's1',
  kind: 'work',
  label: 'Read paper',
  planned_seconds: 1500,
  actual_seconds: 1500,
  started_at: '2026-05-24T10:00:00.000Z',
  ended_at: '2026-05-24T10:25:00.000Z'
};

describe('sessionsApi', () => {
  it('list resolves to an array of sessions', async () => {
    const fetcher = makeFetch({ status: 200, body: [sample] });
    await expect(sessionsApi.list(fetcher)).resolves.toEqual([sample]);
  });

  it('list passes limit when provided', async () => {
    const fetcher = makeFetch({ status: 200, body: [] });
    await sessionsApi.list(fetcher, 10);
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/sessions?limit=10');
  });

  it('create POSTs the full payload', async () => {
    const fetcher = makeFetch({ status: 201, body: sample });
    const input = {
      kind: 'work' as const,
      label: 'Read paper',
      planned_seconds: 1500,
      actual_seconds: 1500,
      started_at: '2026-05-24T10:00:00.000Z',
      ended_at: '2026-05-24T10:25:00.000Z'
    };
    await expect(sessionsApi.create(fetcher, input)).resolves.toEqual(sample);
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify(input));
  });

  it('remove returns undefined on 204', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await expect(sessionsApi.remove(fetcher, 's1')).resolves.toBeUndefined();
  });

  it('stats returns the typed object', async () => {
    const stats: Stats = {
      focus_seconds_today: 3000,
      focus_seconds_week: 21000,
      sessions_today: 4,
      pomodoros_today: 2
    };
    const fetcher = makeFetch({ status: 200, body: stats });
    await expect(sessionsApi.stats(fetcher)).resolves.toEqual(stats);
  });

  it('throws ApiCallError on non-2xx', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: { error: { code: 'validation_failed', message: 'kind invalid' } }
    });
    const input = {
      kind: 'work' as const,
      label: null,
      planned_seconds: 1500,
      actual_seconds: 1500,
      started_at: '2026-01-01T00:00:00Z',
      ended_at: '2026-01-01T00:25:00Z'
    };
    await expect(sessionsApi.create(fetcher, input)).rejects.toBeInstanceOf(ApiCallError);
  });
});
