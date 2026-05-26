import { describe, it, expect, vi } from 'vitest';
import { booksApi, lookupApi, ApiCallError } from './api';
import type { Book, BookLookup } from './types';

function makeFetch(opts: { status: number; body: unknown }) {
  return vi.fn(async () => {
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, {
      status: opts.status,
      headers: new Headers({ 'content-type': 'application/json' })
    });
  });
}

const sample: Book = {
  id: 'b1',
  isbn: '9780134685991',
  title: 'Effective Java',
  author: 'Joshua Bloch',
  cover_url: null,
  pages: 412,
  status: 'reading',
  current_page: 100,
  started_at: '2026-01-01T00:00:00Z',
  finished_at: null,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z'
};

describe('booksApi', () => {
  it('list resolves to array', async () => {
    const fetcher = makeFetch({ status: 200, body: [sample] });
    await expect(booksApi.list(fetcher)).resolves.toEqual([sample]);
  });

  it('update PATCHes payload', async () => {
    const fetcher = makeFetch({ status: 200, body: sample });
    await booksApi.update(fetcher, 'b1', { current_page: 200 });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('PATCH');
    expect(init.body).toBe(JSON.stringify({ current_page: 200 }));
  });

  it('encodes id in path for get', async () => {
    const fetcher = makeFetch({ status: 200, body: sample });
    await booksApi.get(fetcher, 'a/b c');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/books/a%2Fb%20c');
  });
});

describe('lookupApi', () => {
  it('isbn resolves to BookLookup', async () => {
    const lookup: BookLookup = {
      isbn: '9780134685991',
      title: 'Effective Java',
      author: 'Joshua Bloch',
      cover_url: null,
      pages: 412
    };
    const fetcher = makeFetch({ status: 200, body: lookup });
    await expect(lookupApi.isbn(fetcher, '9780134685991')).resolves.toEqual(lookup);
  });

  it('throws ApiCallError on upstream error', async () => {
    const fetcher = makeFetch({
      status: 502,
      body: { error: { code: 'upstream_error', message: 'Open Library returned 503' } }
    });
    await expect(lookupApi.isbn(fetcher, '9780134685991')).rejects.toBeInstanceOf(ApiCallError);
  });
});
