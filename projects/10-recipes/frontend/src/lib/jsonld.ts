import type { Recipe } from './types';

/**
 * Build a Schema.org Recipe JSON-LD object for Google rich-results.
 *
 * Spec reference:
 *   https://developers.google.com/search/docs/appearance/structured-data/recipe
 *
 * The shape returned here is intentionally narrow — we only fill the
 * required + recommended fields. Optional fields like `aggregateRating`
 * or `nutrition` are easy to add later.
 *
 * Note on durations: Schema.org uses ISO-8601 durations ("PT15M" = 15
 * minutes). We compute these from the integer minute fields.
 */
export function recipeJsonLd(recipe: Recipe, pageUrl: string): Record<string, unknown> {
  const ld: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'Recipe',
    name: recipe.title,
    description: recipe.description || `A homemade recipe for ${recipe.title}.`,
    datePublished: recipe.created_at,
    dateModified: recipe.updated_at,
    author: {
      '@type': 'Person',
      // Single-user app — author is "you". Real product would resolve
      // this to a profile.
      name: 'Home Cook'
    },
    mainEntityOfPage: pageUrl,
    recipeIngredient: recipe.ingredients,
    recipeInstructions: recipe.instructions.map((step, i) => ({
      '@type': 'HowToStep',
      position: i + 1,
      text: step,
      name: `Step ${i + 1}`
    }))
  };

  if (recipe.prep_minutes != null) {
    ld.prepTime = `PT${recipe.prep_minutes}M`;
  }
  if (recipe.cook_minutes != null) {
    ld.cookTime = `PT${recipe.cook_minutes}M`;
  }
  if (recipe.prep_minutes != null && recipe.cook_minutes != null) {
    ld.totalTime = `PT${recipe.prep_minutes + recipe.cook_minutes}M`;
  }
  if (recipe.servings != null) {
    ld.recipeYield = String(recipe.servings);
  }

  // Image: an absolute URL is recommended. If the URL is relative, we
  // pass through unchanged — the consumer should ensure the deployment
  // includes BASE_URL. Schema.org also accepts an array of URLs.
  const images = recipe.images.map((img) => img.url);
  if (images.length > 0) {
    ld.image = images;
  }

  return ld;
}
