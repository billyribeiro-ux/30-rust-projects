import { recipesApi } from '$lib/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const recipes = await recipesApi.list(fetch);
  return { recipes };
};
