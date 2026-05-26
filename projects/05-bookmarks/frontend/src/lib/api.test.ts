import { describe, it, expect, vi } from 'vitest';
import { bookmarksApi, ApiCallError } from './api';
import type { Bookmark } from './types';

function makeFetch(opts: { status: number; body: unknown }) {
  return vi.fn(async () => {
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, {
      status: opts.status,
      headers: new Headers({ 'content-type': 'application/json' })
    });
  });
}

const sample: Bookmark = {
  id: 'b1',
  url: 'https://example.com',
  title: 'Example',
  description: '',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
  tags: ['rust', 'axum']
};

describe('bookmarksApi', () => {
  it('list resolves to array', async () => {
    const fetcher = makeFetch({ status: 200, body: [sample] });
    await expect(bookmarksApi.list(fetcher)).resolves.toEqual([sample]);
  });

  it('list builds q + tag query string', async () => {
    const fetcher = makeFetch({ status: 200, body: [] });
    await bookmarksApi.list(fetcher, { q: 'foo bar', tag: 'rust' });
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('q=foo+bar');
    expect(url).toContain('tag=rust');
  });

  it('create POSTs the full payload', async () => {
    const fetcher = makeFetch({ status: 201, body: sample });
    const input = {
      url: 'https://example.com',
      title: 'Example',
      description: '',
      tags: ['rust']
    };
    await expect(bookmarksApi.create(fetcher, input)).resolves.toEqual(sample);
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify(input));
  });

  it('remove returns undefined on 204', async () => {
    const fetcher = makeFetch({ status: 204, body: null });
    await expect(bookmarksApi.remove(fetcher, 'b1')).resolves.toBeUndefined();
  });

  it('throws ApiCallError on non-2xx', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: { error: { code: 'validation_failed', message: 'url invalid' } }
    });
    const input = { url: 'bad', title: 't', description: '', tags: [] };
    await expect(bookmarksApi.create(fetcher, input)).rejects.toBeInstanceOf(ApiCallError);
  });
});
