import { describe, it, expect, vi, beforeEach } from 'vitest';
import { articlesApi, searchApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('kb api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('articlesApi.list parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 'a',
          slug: 's',
          title: 'T',
          summary: '',
          category_slug: null,
          locale: 'en',
          published_at: ''
        }
      ])
    );
    const out = await articlesApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.slug).toBe('s');
  });

  it('articlesApi.get encodes slug', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        id: 'a',
        slug: 'x/y',
        title: 'T',
        summary: '',
        body_html: '',
        category_slug: null,
        locale: 'en',
        faqs: [],
        published_at: '',
        updated_at: ''
      })
    );
    await articlesApi.get(fetcher as unknown as typeof fetch, 'x/y');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/articles/x%2Fy');
  });

  it('searchApi.query encodes q', async () => {
    const fetcher = vi.fn(async () => jsonResponse([]));
    await searchApi.query(fetcher as unknown as typeof fetch, 'kafka basics');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('q=kafka%20basics');
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'not_found', message: 'no', fields: null } }, 404)
    );
    await expect(articlesApi.get(fetcher as unknown as typeof fetch, 'x')).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
