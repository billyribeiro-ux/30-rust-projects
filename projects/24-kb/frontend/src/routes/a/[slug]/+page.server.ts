import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { ApiCallError, articlesApi } from '$lib/api';

export const load: PageServerLoad = async ({ params, fetch }) => {
  try {
    const article = await articlesApi.get(fetch, params.slug);
    return { article };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) error(404, 'Article not found.');
    throw err;
  }
};
