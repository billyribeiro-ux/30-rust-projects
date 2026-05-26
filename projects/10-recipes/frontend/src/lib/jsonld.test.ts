import { describe, it, expect } from 'vitest';
import { recipeJsonLd } from './jsonld';
import type { Recipe } from './types';

function fixture(over: Partial<Recipe> = {}): Recipe {
  return {
    id: 'r1',
    slug: 'apple-pie',
    title: 'Apple Pie',
    description: 'A classic',
    ingredients: ['2 apples', '200g flour'],
    instructions: ['Peel apples', 'Bake'],
    prep_minutes: 20,
    cook_minutes: 45,
    servings: 8,
    cover_image_id: null,
    images: [],
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-02T00:00:00Z',
    ...over
  };
}

describe('recipeJsonLd', () => {
  it('produces a valid Recipe @type', () => {
    const ld = recipeJsonLd(fixture(), 'https://example.test/recipes/apple-pie');
    expect(ld['@context']).toBe('https://schema.org');
    expect(ld['@type']).toBe('Recipe');
    expect(ld.name).toBe('Apple Pie');
  });

  it('serializes durations as ISO-8601', () => {
    const ld = recipeJsonLd(fixture(), 'x');
    expect(ld.prepTime).toBe('PT20M');
    expect(ld.cookTime).toBe('PT45M');
    expect(ld.totalTime).toBe('PT65M');
  });

  it('emits HowToStep array with positions', () => {
    const ld = recipeJsonLd(fixture(), 'x');
    const steps = ld.recipeInstructions as Array<{ '@type': string; position: number; text: string }>;
    expect(steps).toHaveLength(2);
    expect(steps[0]).toMatchObject({ '@type': 'HowToStep', position: 1, text: 'Peel apples' });
    expect(steps[1]?.position).toBe(2);
  });

  it('skips totalTime if either prep or cook is missing', () => {
    const ld = recipeJsonLd(fixture({ prep_minutes: null }), 'x');
    expect(ld.totalTime).toBeUndefined();
    expect(ld.cookTime).toBe('PT45M');
  });

  it('serializes the result to valid JSON without circular refs', () => {
    const ld = recipeJsonLd(fixture(), 'x');
    const out = JSON.stringify(ld);
    expect(() => JSON.parse(out)).not.toThrow();
    expect(JSON.parse(out)['@type']).toBe('Recipe');
  });

  it('includes images when provided', () => {
    const ld = recipeJsonLd(
      fixture({
        images: [
          {
            id: 'i1',
            recipe_id: 'r1',
            mime_type: 'image/jpeg',
            width: 800,
            height: 600,
            bytes: 100_000,
            url: '/uploads/r1/i1.jpg',
            thumb_url: '/uploads/r1/i1-thumb.jpg'
          }
        ]
      }),
      'x'
    );
    expect(ld.image).toEqual(['/uploads/r1/i1.jpg']);
  });
});
