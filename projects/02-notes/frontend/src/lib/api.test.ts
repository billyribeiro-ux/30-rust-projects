import { describe, it, expect, vi } from 'vitest';
import { notesApi, ApiCallError } from './api';
import type { Note, NoteSummary } from './types';

function makeFetch(opts: { status: number; body: unknown; headers?: Record<string, string> }) {
  return vi.fn(async () => {
    const headers = new Headers({ 'content-type': 'application/json', ...(opts.headers ?? {}) });
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, { status: opts.status, headers });
  });
}

describe('notesApi', () => {
  it('list resolves to an array of summaries', async () => {
    const sample: NoteSummary[] = [
      {
        id: '1',
        slug: 'hello',
        title: 'Hello',
        excerpt: 'world',
        updated_at: '2026-01-01T00:00:00Z'
      }
    ];
    const fetcher = makeFetch({ status: 200, body: sample });
    await expect(notesApi.list(fetcher)).resolves.toEqual(sample);
  });

  it('read resolves to a single note', async () => {
    const sample: Note = {
      id: '1',
      slug: 'hi',
      title: 'Hi',
      body_md: '# hi',
      body_html: '<h1>hi</h1>',
      excerpt: 'hi',
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z'
    };
    const fetcher = makeFetch({ status: 200, body: sample });
    await expect(notesApi.read(fetcher, 'hi')).resolves.toEqual(sample);
  });

  it('create POSTs JSON body', async () => {
    const sample: Note = {
      id: '1',
      slug: 'note',
      title: 'Note',
      body_md: 'hi',
      body_html: '<p>hi</p>',
      excerpt: 'hi',
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z'
    };
    const fetcher = makeFetch({ status: 201, body: sample });
    await expect(notesApi.create(fetcher, 'Note', 'hi')).resolves.toEqual(sample);
    expect(fetcher).toHaveBeenCalledTimes(1);
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ title: 'Note', body_md: 'hi' }));
  });

  it('remove returns undefined on 204', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await expect(notesApi.remove(fetcher, 'note')).resolves.toBeUndefined();
  });

  it('throws ApiCallError with server message on non-2xx', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: { error: { code: 'validation_failed', message: 'title must not be empty' } }
    });
    await expect(notesApi.create(fetcher, '', '')).rejects.toBeInstanceOf(ApiCallError);
    await expect(notesApi.create(fetcher, '', '')).rejects.toMatchObject({
      status: 422,
      message: 'title must not be empty'
    });
  });

  it('encodes slug in the path', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await notesApi.remove(fetcher, 'a/b c');
    expect(fetcher).toHaveBeenCalledTimes(1);
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/notes/a%2Fb%20c');
  });
});
