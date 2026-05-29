import type { PageServerLoad } from './$types';
import { postsApi, ApiCallError } from '$lib/api';

export const load: PageServerLoad = async ({ fetch }) => {
  try {
    return { posts: await postsApi.list(fetch) };
  } catch (err) {
    if (err instanceof ApiCallError) return { posts: [], error: err.message };
    return { posts: [], error: 'Could not load.' };
  }
};
