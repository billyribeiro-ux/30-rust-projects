import { notesApi } from '$lib/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  const notes = await notesApi.list(fetch);
  return { notes };
};
