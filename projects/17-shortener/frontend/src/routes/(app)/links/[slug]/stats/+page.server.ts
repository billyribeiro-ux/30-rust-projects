import { ApiCallError, linksApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, params }) => {
  try {
    const stats = await linksApi.stats(serverFetch(locals.sessionCookie), params.slug);
    return { stats };
  } catch (e) {
    if (e instanceof ApiCallError && e.status === 404) {
      throw error(404, 'short link not found');
    }
    throw e;
  }
};
