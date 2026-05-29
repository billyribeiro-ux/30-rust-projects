import { describe, it, expect, vi, beforeEach } from 'vitest';
import { postsApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('portfolio api', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('postsApi.list parses', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 'p',
          slug: 's',
          title: 'T',
          summary: '',
          hero_image_url: null,
          locale: 'en',
          published_at: ''
        }
      ])
    );
    const out = await postsApi.list(fetcher as unknown as typeof fetch);
    expect(out[0]?.slug).toBe('s');
  });

  it('postsApi.get URL-encodes slug', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        id: 'p',
        slug: 'a/b',
        title: 'T',
        summary: '',
        hero_image_url: null,
        body_html: '',
        locale: 'en',
        published_at: '',
        updated_at: ''
      })
    );
    await postsApi.get(fetcher as unknown as typeof fetch, 'a/b');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/posts/a%2Fb');
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ error: { code: 'not_found', message: 'no', fields: null } }, 404)
    );
    await expect(postsApi.get(fetcher as unknown as typeof fetch, 'x')).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
