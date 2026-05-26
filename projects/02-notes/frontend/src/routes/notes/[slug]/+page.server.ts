import { error, redirect } from '@sveltejs/kit';
import { notesApi, ApiCallError } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, fetch }) => {
  try {
    const note = await notesApi.read(fetch, params.slug);
    return { note };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) {
      error(404, 'Note not found');
    }
    throw err;
  }
};

export const actions: Actions = {
  remove: async ({ params, fetch }) => {
    try {
      await notesApi.remove(fetch, params.slug);
    } catch (err) {
      if (err instanceof ApiCallError && err.status !== 404) {
        error(err.status, err.message);
      }
    }
    redirect(303, '/');
  }
};
