import { error } from '@sveltejs/kit';
import { recipesApi, ApiCallError } from '$lib/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, url, fetch }) => {
  const sig = url.searchParams.get('sig');
  const expRaw = url.searchParams.get('exp');
  if (!sig || !expRaw) {
    error(404, 'Share link is missing required parameters');
  }
  const exp = Number(expRaw);
  if (!Number.isFinite(exp)) {
    error(404, 'Share link is invalid');
  }
  try {
    const recipe = await recipesApi.getShared(fetch, params.slug, sig, exp);
    return { recipe };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) {
      // We don't know if it was bad sig or expired or missing — the
      // backend deliberately collapses both into 404 to avoid leaking
      // recipe existence.
      error(404, 'Share link is invalid or has expired');
    }
    throw err;
  }
};
