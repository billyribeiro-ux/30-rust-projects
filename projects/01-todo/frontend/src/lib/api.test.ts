import { describe, it, expect, vi } from 'vitest';
import { todosApi, ApiCallError } from './api';
import type { Todo } from './types';

function makeFetch(response: { status: number; body: unknown }) {
  return vi.fn(
    async () =>
      new Response(response.body == null ? null : JSON.stringify(response.body), {
        status: response.status,
        headers: { 'content-type': 'application/json' }
      })
  );
}

describe('todosApi', () => {
  it('list parses an array of todos', async () => {
    const sample: Todo[] = [
      {
        id: '1',
        title: 'buy milk',
        done: false,
        created_at: '2026-05-26T12:00:00Z',
        updated_at: '2026-05-26T12:00:00Z'
      }
    ];
    const fetcher = makeFetch({ status: 200, body: sample });
    await expect(todosApi.list(fetcher)).resolves.toEqual(sample);
  });

  it('throws ApiCallError with status on 422', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: { error: { code: 'validation_failed', message: 'title must not be empty' } }
    });
    await expect(todosApi.create(fetcher, '')).rejects.toBeInstanceOf(ApiCallError);
    await expect(todosApi.create(fetcher, '')).rejects.toMatchObject({
      status: 422,
      message: 'title must not be empty'
    });
  });

  it('returns void on 204 delete', async () => {
    const fetcher = vi.fn(async () => new Response(null, { status: 204 }));
    await expect(todosApi.remove(fetcher, 'x')).resolves.toBeUndefined();
  });

  it('encodes the id in the path', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await todosApi.remove(fetcher, 'a/b c');
    expect(fetcher).toHaveBeenCalledTimes(1);
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/todos/a%2Fb%20c');
  });
});
