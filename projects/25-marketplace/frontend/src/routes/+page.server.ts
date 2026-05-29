import type { PageServerLoad } from './$types';
import { ApiCallError, coursesApi } from '$lib/api';

export const load: PageServerLoad = async ({ fetch }) => {
  try {
    return { courses: await coursesApi.list(fetch) };
  } catch (err) {
    if (err instanceof ApiCallError) return { courses: [], error: err.message };
    return { courses: [], error: 'Could not load.' };
  }
};
