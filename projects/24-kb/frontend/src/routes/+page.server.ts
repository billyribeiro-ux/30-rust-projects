import type { PageServerLoad } from './$types';
import { ApiCallError, articlesApi, searchApi } from '$lib/api';

export const load: PageServerLoad = async ({ url, fetch }) => {
  const q = url.searchParams.get('q') ?? '';
  try {
    const articles = await articlesApi.list(fetch);
    const hits = q ? await searchApi.query(fetch, q) : [];
    return { articles, hits, q };
  } catch (err) {
    if (err instanceof ApiCallError) {
      return { articles: [], hits: [], q, error: err.message };
    }
    return { articles: [], hits: [], q, error: 'Could not load.' };
  }
};
