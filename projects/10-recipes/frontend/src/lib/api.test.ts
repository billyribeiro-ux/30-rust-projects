import { describe, it, expect, vi } from 'vitest';
import { recipesApi, ratingsApi, ApiCallError } from './api';
import type { Recipe, RecipeSummary } from './types';

function makeFetch(opts: { status: number; body: unknown }) {
  return vi.fn(async () => {
    const body = opts.status === 204 ? null : JSON.stringify(opts.body);
    return new Response(body, {
      status: opts.status,
      headers: new Headers({ 'content-type': 'application/json' })
    });
  });
}

const sample: Recipe = {
  id: 'r1',
  slug: 'apple-pie',
  title: 'Apple Pie',
  description: '',
  ingredients: ['2 apples'],
  instructions: ['Bake'],
  prep_minutes: 20,
  cook_minutes: 45,
  servings: 8,
  cover_image_id: null,
  images: [],
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z'
};

describe('recipesApi', () => {
  it('list resolves to array', async () => {
    const summary: RecipeSummary = {
      id: 'r1',
      slug: 'apple-pie',
      title: 'Apple Pie',
      description: '',
      prep_minutes: 20,
      cook_minutes: 45,
      servings: 8,
      cover_thumb_url: null,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z'
    };
    const fetcher = makeFetch({ status: 200, body: [summary] });
    await expect(recipesApi.list(fetcher)).resolves.toEqual([summary]);
  });

  it('update PATCHes payload', async () => {
    const fetcher = makeFetch({ status: 200, body: sample });
    await recipesApi.update(fetcher, 'r1', { title: 'Apple Pie 2' });
    const [, init] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? [
      '',
      {}
    ];
    expect(init.method).toBe('PATCH');
    expect(init.body).toBe(JSON.stringify({ title: 'Apple Pie 2' }));
  });

  it('encodes slug in path for getBySlug', async () => {
    const fetcher = makeFetch({ status: 200, body: sample });
    await recipesApi.getBySlug(fetcher, 'a b/c');
    const [url] = (fetcher.mock.calls as unknown as Array<[string, RequestInit]>)[0] ?? ['', {}];
    expect(url).toContain('/api/recipes/by-slug/a%20b%2Fc');
  });

  it('share returns signed URL', async () => {
    const fetcher = makeFetch({
      status: 200,
      body: { url: '/share/apple-pie?sig=abc&exp=999', expires_at: '2026-01-02T00:00:00Z' }
    });
    const link = await recipesApi.share(fetcher, 'r1');
    expect(link.url).toContain('sig=abc');
  });
});

describe('ratingsApi', () => {
  it('throws ApiCallError on 422', async () => {
    const fetcher = makeFetch({
      status: 422,
      body: { error: { code: 'validation_failed', message: 'bad stars', fields: null } }
    });
    await expect(
      ratingsApi.create(fetcher, 'r1', { stars: 10 })
    ).rejects.toBeInstanceOf(ApiCallError);
  });
});
