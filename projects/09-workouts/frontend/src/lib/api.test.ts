import { describe, it, expect, vi } from 'vitest';
import { exercisesApi, workoutsApi, statsApi, ApiCallError } from './api';
import type { Exercise, Stats, Workout } from './types';

function makeFetch(opts: { status: number; body: unknown }) {
  return vi.fn(async () => {
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, {
      status: opts.status,
      headers: new Headers({ 'content-type': 'application/json' })
    });
  });
}

const ex: Exercise = {
  id: 'e1',
  name: 'Bench Press',
  muscle_group: 'chest',
  created_at: '2026-05-01T00:00:00Z'
};

const stats: Stats = {
  total_sets: 42,
  total_volume_minor: 1_000_000,
  workouts_last_7_days: 3,
  pr_count_total: 7
};

const workout: Workout = {
  id: 'w1',
  name: 'Push day',
  performed_at: '2026-05-20T18:00:00Z',
  created_at: '2026-05-20T18:30:00Z',
  sets: []
};

describe('exercisesApi', () => {
  it('list resolves to array', async () => {
    const fetcher = makeFetch({ status: 200, body: [ex] });
    await expect(exercisesApi.list(fetcher)).resolves.toEqual([ex]);
  });

  it('list with query encodes ?q=', async () => {
    const fetcher = makeFetch({ status: 200, body: [ex] });
    await exercisesApi.list(fetcher, 'bench');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(url).toContain('/api/exercises?q=bench');
  });

  it('list without query has no ?q=', async () => {
    const fetcher = makeFetch({ status: 200, body: [ex] });
    await exercisesApi.list(fetcher);
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(url.endsWith('/api/exercises')).toBe(true);
  });

  it('create posts payload', async () => {
    const fetcher = makeFetch({ status: 201, body: ex });
    await exercisesApi.create(fetcher, { name: 'Squat', muscle_group: 'legs' });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ name: 'Squat', muscle_group: 'legs' }));
  });
});

describe('workoutsApi', () => {
  it('get encodes id', async () => {
    const fetcher = makeFetch({ status: 200, body: workout });
    await workoutsApi.get(fetcher, 'a b/c');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(url).toContain('/api/workouts/a%20b%2Fc');
  });

  it('create posts JSON', async () => {
    const fetcher = makeFetch({ status: 201, body: workout });
    await workoutsApi.create(fetcher, {
      name: 'Push',
      sets: [{ exercise_id: 'e1', weight_minor: 80000, reps: 8, rir: 2 }]
    });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string).sets[0].exercise_id).toBe('e1');
  });
});

describe('statsApi', () => {
  it('read returns Stats', async () => {
    const fetcher = makeFetch({ status: 200, body: stats });
    await expect(statsApi.read(fetcher)).resolves.toEqual(stats);
  });

  it('throws ApiCallError on 5xx', async () => {
    const fetcher = makeFetch({
      status: 500,
      body: { error: { code: 'internal_error', message: 'oops' } }
    });
    await expect(statsApi.read(fetcher)).rejects.toBeInstanceOf(ApiCallError);
  });
});
