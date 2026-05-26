import { fail, redirect } from '@sveltejs/kit';
import { recipesApi, ApiCallError } from '$lib/api';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request, fetch }) => {
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

    if (!title) {
      return fail(422, {
        error: 'Title is required.',
        title,
        description,
        ingredients: ingredients.join('\n'),
        instructions: instructions.join('\n')
      });
    }
    try {
      const created = await recipesApi.create(fetch, {
        title,
        description,
        ingredients,
        instructions,
        prep_minutes,
        cook_minutes,
        servings
      });
      redirect(303, `/recipes/${created.slug}/edit`);
    } catch (err) {
      if (err instanceof ApiCallError) {
        return fail(err.status, {
          error: err.message,
          title,
          description,
          ingredients: ingredients.join('\n'),
          instructions: instructions.join('\n')
        });
      }
      throw err;
    }
  }
};

function numOrNull(v: FormDataEntryValue | null): number | null {
  if (v === null) return null;
  const s = String(v).trim();
  if (!s) return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}
