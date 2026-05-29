import { describe, it, expect, vi, beforeEach } from 'vitest';
import { postsApi, subscribeApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('newsletter api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('postsApi.list parses the array', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse([
        {
          id: 'p',
          slug: 's',
          title: 'T',
          summary: 'sum',
          is_gated: false,
          published_at: '2026-05-27T00:00:00Z'
        }
      ])
    );
    const posts = await postsApi.list(fetcher as unknown as typeof fetch);
    expect(posts[0]?.slug).toBe('s');
  });

  it('postsApi.get encodes the slug', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        id: 'p',
        slug: 'foo/bar',
        title: 'T',
        summary: '',
        is_gated: true,
        published_at: '',
        body_html: '<p>x</p>',
        paywalled: false
      })
    );
    await postsApi.get(fetcher as unknown as typeof fetch, 'foo/bar');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/posts/foo%2Fbar');
  });

  it('postsApi.get returns teaser when paywalled (HTTP 402)', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse(
        {
          id: 'p',
          slug: 'gated',
          title: 'T',
          summary: '',
          is_gated: true,
          published_at: '',
          body_html: 'teaser…',
          paywalled: true
        },
        402
      )
    );
    const post = await postsApi.get(fetcher as unknown as typeof fetch, 'gated');
    expect(post.paywalled).toBe(true);
  });

  it('subscribeApi.start posts plan correctly', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({ kind: 'pro', checkout_url: 'https://stripe.example/checkout' })
    );
    const out = await subscribeApi.start(fetcher as unknown as typeof fetch, 'a@b.co', 'pro');
    expect(out.kind).toBe('pro');
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.body).toBe(JSON.stringify({ email: 'a@b.co', plan: 'pro' }));
  });

  it('non-2xx (non-402) maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse(
        { error: { code: 'not_found', message: 'gone', fields: null } },
        404
      )
    );
    await expect(
      postsApi.get(fetcher as unknown as typeof fetch, 'x')
    ).rejects.toBeInstanceOf(ApiCallError);
  });
});
