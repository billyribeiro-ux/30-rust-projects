import { error, redirect } from '@sveltejs/kit';
import { workoutsApi, ApiCallError } from '$lib/api';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, fetch }) => {
  try {
    const workout = await workoutsApi.get(fetch, params.id);
    return { workout };
  } catch (err) {
    if (err instanceof ApiCallError && err.status === 404) {
      error(404, 'Workout not found');
    }
    throw err;
  }
};

export const actions: Actions = {
  remove: async ({ params, fetch }) => {
    try {
      await workoutsApi.remove(fetch, params.id);
    } catch (err) {
      if (!(err instanceof ApiCallError) || err.status !== 404) throw err;
    }
    redirect(303, '/');
  }
};
