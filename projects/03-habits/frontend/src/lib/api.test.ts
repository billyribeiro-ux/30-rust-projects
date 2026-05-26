import { describe, it, expect, vi } from 'vitest';
import { habitsApi, ApiCallError } from './api';
import type { Habit, ToggleResult } from './types';

function makeFetch(opts: { status: number; body: unknown; headers?: Record<string, string> }) {
  return vi.fn(async () => {
    const headers = new Headers({ 'content-type': 'application/json', ...(opts.headers ?? {}) });
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, { status: opts.status, headers });
  });
}

const sampleHabit: Habit = {
  id: 'h1',
  name: 'Read',
  color: '#4F46E5',
  created_at: '2026-05-01T00:00:00Z',
  streak: { current: 3, longest: 7, total: 12, last_completion: '2026-05-24' },
  completions: ['2026-05-22', '2026-05-23', '2026-05-24']
};

describe('habitsApi', () => {
  it('list resolves to an array of habits', async () => {
    const fetcher = makeFetch({ status: 200, body: [sampleHabit] });
    await expect(habitsApi.list(fetcher)).resolves.toEqual([sampleHabit]);
  });

  it('create POSTs name + color', async () => {
    const fetcher = makeFetch({ status: 201, body: sampleHabit });
    await expect(habitsApi.create(fetcher, 'Read', '#4F46E5')).resolves.toEqual(sampleHabit);
    expect(fetcher).toHaveBeenCalledTimes(1);
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ name: 'Read', color: '#4F46E5' }));
  });

  it('toggle POSTs the date and returns the new state', async () => {
    const toggled: ToggleResult = {
      completed: true,
      streak: sampleHabit.streak,
      completions: sampleHabit.completions
    };
    const fetcher = makeFetch({ status: 200, body: toggled });
    await expect(habitsApi.toggle(fetcher, 'h1', '2026-05-24')).resolves.toEqual(toggled);
    expect(fetcher).toHaveBeenCalledTimes(1);
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.body).toBe(JSON.stringify({ date: '2026-05-24' }));
  });

  it('remove returns undefined on 204', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await expect(habitsApi.remove(fetcher, 'h1')).resolves.toBeUndefined();
  });

  it('throws ApiCallError with server message on non-2xx', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: { error: { code: 'validation_failed', message: 'name must not be empty' } }
    });
    await expect(habitsApi.create(fetcher, '', '#4F46E5')).rejects.toBeInstanceOf(ApiCallError);
    await expect(habitsApi.create(fetcher, '', '#4F46E5')).rejects.toMatchObject({
      status: 422,
      message: 'name must not be empty'
    });
  });

  it('encodes id in path for toggle', async () => {
    const fetcher = makeFetch({
      status: 200,
      body: { completed: false, streak: sampleHabit.streak, completions: [] }
    });
    await habitsApi.toggle(fetcher, 'a/b c', '2026-01-01');
    expect(fetcher).toHaveBeenCalledTimes(1);
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/habits/a%2Fb%20c/completions');
  });
});
