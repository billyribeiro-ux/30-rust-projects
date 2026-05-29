import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { postsApi, ApiCallError } from '$lib/api';

export const load: PageServerLoad = async ({ params, fetch }) => {
  try {
    return { post: await postsApi.get(fetch, params.slug) };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) error(404, 'Not found.');
    throw err;
  }
};
