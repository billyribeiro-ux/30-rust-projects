import { describe, it, expect, vi, beforeEach } from 'vitest';
import { placesApi, reviewsApi, authApi, ApiCallError } from './api';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' }
  });
}

describe('finder api client', () => {
  beforeEach(() => vi.restoreAllMocks());

  it('placesApi.list serializes lat/lng/radius_m', async () => {
    const fetcher = vi.fn(async () => jsonResponse([]));
    await placesApi.list(fetcher as unknown as typeof fetch, {
      lat: 40.7,
      lng: -74,
      radius_m: 1500,
      q: 'thai'
    });
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('lat=40.7');
    expect(url).toContain('lng=-74');
    expect(url).toContain('radius_m=1500');
    expect(url).toContain('q=thai');
  });

  it('placesApi.list omits absent fields', async () => {
    const fetcher = vi.fn(async () => jsonResponse([]));
    await placesApi.list(fetcher as unknown as typeof fetch, {});
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toBe('http://localhost:3019/api/places');
  });

  it('placesApi.get encodes id', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        id: 'abc',
        name: 'X',
        cuisine: 'y',
        address: 'z',
        lat: 0,
        lng: 0,
        distance_m: null,
        avg_rating: null,
        review_count: 0,
        created_at: '',
        updated_at: '',
        reviews: []
      })
    );
    await placesApi.get(fetcher as unknown as typeof fetch, 'p/1');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/places/p%2F1');
  });

  it('reviewsApi.upsert posts rating + body', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse({
        id: 'r',
        user_id: 'u',
        user_name: 'n',
        rating: 5,
        body: 'great',
        created_at: ''
      })
    );
    await reviewsApi.upsert(fetcher as unknown as typeof fetch, 'p1', 5, 'great');
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('POST');
    expect(init.body).toBe(JSON.stringify({ rating: 5, body: 'great' }));
  });

  it('authApi.oauthStart returns absolute URL', () => {
    expect(authApi.oauthStart('google')).toBe(
      'http://localhost:3019/api/auth/oauth/google/start'
    );
  });

  it('non-2xx maps to ApiCallError', async () => {
    const fetcher = vi.fn(async () =>
      jsonResponse(
        { error: { code: 'not_found', message: 'no such place', fields: null } },
        404
      )
    );
    await expect(placesApi.get(fetcher as unknown as typeof fetch, 'x')).rejects.toBeInstanceOf(
      ApiCallError
    );
  });
});
