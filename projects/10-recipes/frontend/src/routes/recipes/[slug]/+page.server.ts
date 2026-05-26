import { error } from '@sveltejs/kit';
import { recipesApi, ApiCallError } from '$lib/api';
import type { PageServerLoad } from './$types';

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
