import { ApiCallError, API_BASE, pollsApi } from '$lib/api';
import { serverFetch } from '$lib/server/api';
import { error, fail } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, locals }) => {
  try {
    const poll = await pollsApi.get(serverFetch(locals.sessionCookie), params.slug);
    return { poll, apiBase: API_BASE };
  } catch (e) {
    if (e instanceof ApiCallError && e.status === 404) error(404, 'Poll not found');
    throw e;
  }
};

export const actions: Actions = {
  close: async ({ params, locals }) => {
    try {
      await pollsApi.close(serverFetch(locals.sessionCookie), params.slug);
    } catch (e) {
      if (e instanceof ApiCallError) return fail(e.status, { error: e.message });
      throw e;
    }
    return { ok: true };
  }
};
