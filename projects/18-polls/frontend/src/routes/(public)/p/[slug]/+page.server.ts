import { ApiCallError, API_BASE, pollsApi } from '$lib/api';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

/**
 * Public voter page — no auth. We fetch via the server's `fetch` (no cookie
 * forwarding needed; the endpoint is public).
 */
export const load: PageServerLoad = async ({ params, fetch }) => {
  try {
    const poll = await pollsApi.get(fetch, params.slug);
    return { poll, apiBase: API_BASE };
  } catch (e) {
    if (e instanceof ApiCallError && e.status === 404) error(404, 'Poll not found');
    throw e;
  }
};
