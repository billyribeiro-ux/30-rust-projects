import { error, fail, redirect } from '@sveltejs/kit';
import { recipesApi, ApiCallError } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, fetch }) => {
  try {
    const recipe = await recipesApi.getBySlug(fetch, params.slug);
    return { recipe };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) {
      error(404, 'Recipe not found');
    }
    throw err;
  }
};

export const actions: Actions = {
  save: async ({ request, fetch, params }) => {
    const data = await request.formData();
    const title = String(data.get('title') ?? '').trim();
    const description = String(data.get('description') ?? '').trim();
    const ingredients = String(data.get('ingredients') ?? '')
      .split('\n')
      .map((s) => s.trim())
      .filter(Boolean);
    const instructions = String(data.get('instructions') ?? '')
      .split('\n')
      .map((s) => s.trim())
      .filter(Boolean);
    const prep_minutes = numOrNull(data.get('prep_minutes'));
    const cook_minutes = numOrNull(data.get('cook_minutes'));
    const servings = numOrNull(data.get('servings'));

    if (!title) return fail(422, { error: 'Title is required.' });

    // Look up id by slug — we don't carry id through the form because
    // slug is in the URL and id is server-resolved.
    let recipe;
    try {
      recipe = await recipesApi.getBySlug(fetch, params.slug);
    } catch (err) {
      if (err instanceof ApiCallError && err.status === 404) error(404, 'Recipe not found');
      throw err;
    }

    try {
      await recipesApi.update(fetch, recipe.id, {
        title,
        description,
        ingredients,
        instructions,
        prep_minutes,
        cook_minutes,
        servings
      });
      return { success: true as const };
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, { error: err.message });
      }
      throw err;
    }
  },

  remove: async ({ params, fetch }) => {
    try {
      const recipe = await recipesApi.getBySlug(fetch, params.slug);
      await recipesApi.remove(fetch, recipe.id);
    } catch (err) {
      if (!(err instanceof ApiCallError) || err.status !== 404) throw err;
    }
    redirect(303, '/');
  }
};

function numOrNull(v: FormDataEntryValue | null): number | null {
  if (v === null) return null;
  const s = String(v).trim();
  if (!s) return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}
